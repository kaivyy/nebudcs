//! Historian timeseries and event journal query endpoints.

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use nebudcs_historian::point::HistorianPointId;
use nebudcs_historian::{AggregatedBucket, AggregatedQueryRequest, RawQueryRequest, RawSample};
use nebudcs_protocol::{FirstOutEventDto, HistoricalEventDto};
use nebudcs_types::SimulationTime;

use crate::error::{ServerError, ServerResult};
use crate::host::HostHandle;

#[derive(Debug, Clone, Deserialize)]
pub struct HistoryQueryParams {
    pub run_id: String,
    pub point_id: String,
    pub start_time: Option<f64>,
    pub end_time: Option<f64>,
    pub limit: Option<usize>,
    pub bucket_count: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EventQueryParams {
    pub run_id: String,
    pub start_time: Option<f64>,
    pub end_time: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum HistoryQueryResponse {
    Raw(Vec<RawSample>),
    Aggregated(Vec<AggregatedBucket>),
}

/// Queries historical timeseries data (either raw samples or aggregated downsampled buckets).
pub async fn query_history(
    State(host): State<HostHandle>,
    Query(params): Query<HistoryQueryParams>,
) -> ServerResult<Json<HistoryQueryResponse>> {
    let reader = host
        .historian_reader()
        .ok_or_else(|| ServerError::NotFound("Historian reader not configured".to_string()))?;

    let start = SimulationTime::try_new(params.start_time.unwrap_or(0.0).max(0.0))
        .unwrap_or(SimulationTime::ZERO);
    let end = SimulationTime::try_new(params.end_time.unwrap_or(1e9).max(0.0))
        .unwrap_or(SimulationTime::ZERO);

    let point_id = if let Ok(id) = params.point_id.parse::<u32>() {
        HistorianPointId::new(id)
    } else {
        let points = reader
            .list_points(&params.run_id)
            .map_err(|e| ServerError::Internal(e.to_string()))?;
        let found = points
            .iter()
            .find(|p| p.name == params.point_id || p.tag_id.to_string() == params.point_id);
        match found {
            Some(p) => p.point_id,
            None => {
                return Err(ServerError::NotFound(format!(
                    "Point '{}' not found in run '{}'",
                    params.point_id, params.run_id
                )))
            }
        }
    };

    if let Some(bucket_count) = params.bucket_count {
        let req = AggregatedQueryRequest {
            run_id: params.run_id,
            point_id,
            start_time: start,
            end_time: end,
            bucket_count,
        };
        let buckets = reader
            .query_aggregated(req)
            .map_err(|e| ServerError::Internal(e.to_string()))?;
        Ok(Json(HistoryQueryResponse::Aggregated(buckets)))
    } else {
        let req = RawQueryRequest {
            run_id: params.run_id,
            point_id,
            start_time: start,
            end_time: end,
            limit: params.limit,
        };
        let samples = reader
            .query_raw(req)
            .map_err(|e| ServerError::Internal(e.to_string()))?;
        Ok(Json(HistoryQueryResponse::Raw(samples)))
    }
}

/// Queries historical discrete operational and alarm events.
pub async fn query_events(
    State(host): State<HostHandle>,
    Query(params): Query<EventQueryParams>,
) -> ServerResult<Json<Vec<HistoricalEventDto>>> {
    let reader = host
        .historian_reader()
        .ok_or_else(|| ServerError::NotFound("Historian reader not configured".to_string()))?;

    let start = SimulationTime::try_new(params.start_time.unwrap_or(0.0).max(0.0))
        .unwrap_or(SimulationTime::ZERO);
    let end = SimulationTime::try_new(params.end_time.unwrap_or(1e9).max(0.0))
        .unwrap_or(SimulationTime::ZERO);

    let events = reader
        .query_events(&params.run_id, start, end)
        .map_err(|e| ServerError::Internal(e.to_string()))?;

    let dtos = events
        .into_iter()
        .map(|ev| HistoricalEventDto {
            sim_time_sec: ev.sim_time.seconds(),
            seq: ev.seq,
            event_type: ev.event_type,
            source_id: ev.source_id,
            old_state: ev.old_state,
            new_state: ev.new_state,
            value: ev.value,
            severity: ev.severity,
            details_json: ev.details_json,
            wall_clock_ms: ev.persisted_wall_clock_ms,
        })
        .collect();

    Ok(Json(dtos))
}

#[derive(Debug, Clone, Deserialize)]
pub struct FirstOutQueryParams {
    pub run_id: Option<String>,
}

/// Chronological first-out disturbance sequence analysis (Stage 10 / ISA-18.2).
pub async fn query_first_out_event(
    State(host): State<HostHandle>,
    Query(params): Query<FirstOutQueryParams>,
) -> ServerResult<Json<FirstOutEventDto>> {
    let run_id = params.run_id.unwrap_or_else(|| "active".to_string());
    let reader = match host.historian_reader() {
        Some(r) => r,
        None => {
            return Ok(Json(FirstOutEventDto {
                run_id,
                first_out_alarm: None,
                initiating_event_type: None,
                timestamp_sim: None,
                disturbance_chain: Vec::new(),
            }));
        }
    };
    let snapshot = host.get_snapshot().await;
    let end =
        SimulationTime::try_new(snapshot.sim_time_sec.max(0.0)).unwrap_or(SimulationTime::ZERO);

    let events = reader
        .query_events(&run_id, SimulationTime::ZERO, end)
        .map_err(|e| ServerError::Internal(e.to_string()))?;

    let dtos: Vec<HistoricalEventDto> = events
        .into_iter()
        .map(|ev| HistoricalEventDto {
            sim_time_sec: ev.sim_time.seconds(),
            seq: ev.seq,
            event_type: ev.event_type,
            source_id: ev.source_id,
            old_state: ev.old_state,
            new_state: ev.new_state,
            value: ev.value,
            severity: ev.severity,
            details_json: ev.details_json,
            wall_clock_ms: ev.persisted_wall_clock_ms,
        })
        .collect();

    // Identify initiating disturbance (first trip, safety event, or alarm activation)
    let first_disturbance = dtos.iter().find(|e| {
        e.event_type.contains("trip")
            || e.event_type.contains("alarm")
            || e.event_type.contains("sif")
            || e.event_type.contains("fault")
    });

    let (first_out_alarm, initiating_event_type, timestamp_sim, disturbance_chain) =
        if let Some(first) = first_disturbance {
            let t0 = first.sim_time_sec;
            let chain = dtos
                .iter()
                .filter(|e| e.sim_time_sec >= t0 && e.sim_time_sec <= t0 + 120.0)
                .cloned()
                .collect();
            (
                Some(first.source_id.clone()),
                Some(first.event_type.clone()),
                Some(t0),
                chain,
            )
        } else {
            (None, None, None, dtos)
        };

    Ok(Json(FirstOutEventDto {
        run_id,
        first_out_alarm,
        initiating_event_type,
        timestamp_sim,
        disturbance_chain,
    }))
}
