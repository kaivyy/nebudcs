//! Alarm and protection system routes.

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use nebudcs_protocol::{
    AlarmFloodMetricsDto, AlarmSnapshotDto, AlarmStateDto, AlarmSummaryDto, RuntimeCommand,
};

use crate::error::{ServerError, ServerResult};
use crate::host::HostHandle;
use crate::security::{audit_log, UserRole};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlarmListResponse {
    pub count: usize,
    pub alarms: Vec<AlarmSnapshotDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcknowledgeAlarmRequest {
    pub alarm_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShelveAlarmRequest {
    pub alarm_name: String,
    pub duration_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetProtectionRequest {
    pub block_name: String,
}

/// Retrieves all active and unacknowledged alarms.
pub async fn list_alarms(State(host): State<HostHandle>) -> Json<AlarmListResponse> {
    let snapshot = host.get_snapshot().await;
    let count = snapshot.active_alarms.len();
    Json(AlarmListResponse {
        count,
        alarms: snapshot.active_alarms,
    })
}

/// Retrieves comprehensive alarm summary with ISA-18.2 / IEC 62682 flood metrics.
pub async fn get_alarm_summary(State(host): State<HostHandle>) -> Json<AlarmSummaryDto> {
    let snapshot = host.get_snapshot().await;
    let active = snapshot.active_alarms;
    let total_active = active.len();
    let unacknowledged = active
        .iter()
        .filter(|a| a.state == AlarmStateDto::ActiveUnack)
        .count();
    let acknowledged = active
        .iter()
        .filter(|a| a.state == AlarmStateDto::ActiveAck)
        .count();
    let shelved = active
        .iter()
        .filter(|a| a.state == AlarmStateDto::Shelved)
        .count();

    let current_sim_time = snapshot.sim_time_sec;
    let window_start = (current_sim_time - 600.0).max(0.0);
    let mut alarms_last_10min = total_active;
    let mut first_out = None;
    let mut first_out_time = None;

    if let Some(reader) = host.historian_reader() {
        if let Ok(events) = reader.query_events(
            "active",
            nebudcs_types::SimulationTime::try_new(window_start)
                .unwrap_or(nebudcs_types::SimulationTime::ZERO),
            nebudcs_types::SimulationTime::try_new(current_sim_time)
                .unwrap_or(nebudcs_types::SimulationTime::ZERO),
        ) {
            let alarm_events: Vec<_> = events
                .into_iter()
                .filter(|e| e.event_type.contains("alarm"))
                .collect();
            if !alarm_events.is_empty() {
                alarms_last_10min = alarm_events.len();
                if let Some(first) = alarm_events.first() {
                    first_out = Some(first.source_id.clone());
                    first_out_time = Some(first.sim_time.seconds());
                }
            }
        }
    }

    if first_out.is_none() && !active.is_empty() {
        first_out = Some(active[0].alarm_name.clone());
        first_out_time = Some(current_sim_time);
    }

    let alarm_rate_per_10min = alarms_last_10min as f64;
    let flood_threshold_per_10min = 10.0;
    let is_flood = alarm_rate_per_10min > flood_threshold_per_10min;

    let metrics = AlarmFloodMetricsDto {
        alarm_rate_per_10min,
        is_flood,
        flood_threshold_per_10min,
        total_active,
        unacknowledged,
        acknowledged,
        shelved,
        suppressed: 0,
        standing_count: total_active,
        stale_count: unacknowledged,
        first_out_alarm: first_out,
        first_out_timestamp: first_out_time,
    };

    Json(AlarmSummaryDto {
        metrics,
        active_alarms: active,
        shelved_alarms: Vec::new(),
    })
}

/// Acknowledges an active alarm with operator audit logging.
pub async fn acknowledge_alarm(
    State(host): State<HostHandle>,
    role: UserRole,
    Json(req): Json<AcknowledgeAlarmRequest>,
) -> ServerResult<Json<Value>> {
    if !role.can_operate_dcs() {
        audit_log(
            "ALARM_ACKNOWLEDGE",
            &format!("{role}"),
            &format!("/api/v1/alarms/acknowledge: {}", req.alarm_name),
            "Unauthorized attempt to acknowledge alarm",
            false,
        );
        return Err(ServerError::Forbidden(
            "Current role is not authorized to acknowledge alarms".to_string(),
        ));
    }

    let cmd = RuntimeCommand::AcknowledgeAlarm {
        alarm_name: req.alarm_name.clone(),
    };
    host.dispatch_command(cmd).await?;

    audit_log(
        "ALARM_ACKNOWLEDGE",
        &format!("{role}"),
        &format!("/api/v1/alarms/acknowledge: {}", req.alarm_name),
        "Alarm successfully acknowledged by operator",
        true,
    );

    Ok(Json(json!({ "status": "ok" })))
}

/// Shelves an alarm for a specified duration in seconds.
pub async fn shelve_alarm(
    State(host): State<HostHandle>,
    role: UserRole,
    Json(req): Json<ShelveAlarmRequest>,
) -> ServerResult<Json<Value>> {
    if matches!(role, UserRole::Trainee) {
        audit_log(
            "ALARM_SHELVE",
            &format!("{role}"),
            &format!("/api/v1/alarms/shelve: {}", req.alarm_name),
            "Trainees are prohibited from shelving alarms",
            false,
        );
        return Err(ServerError::Forbidden(
            "Trainees are not permitted to shelve alarms".to_string(),
        ));
    }

    let cmd = RuntimeCommand::ShelveAlarm {
        alarm_name: req.alarm_name.clone(),
        duration_seconds: req.duration_seconds,
    };
    host.dispatch_command(cmd).await?;

    audit_log(
        "ALARM_SHELVE",
        &format!("{role}"),
        &format!(
            "/api/v1/alarms/shelve: {} (duration: {}s)",
            req.alarm_name, req.duration_seconds
        ),
        "Alarm shelved",
        true,
    );

    Ok(Json(json!({ "status": "ok" })))
}

/// Resets a latched BPCS protection trip.
/// Note: This resets BPCS interlocks only; it NEVER resets SIS trips.
pub async fn reset_protection(
    State(host): State<HostHandle>,
    role: UserRole,
    Json(req): Json<ResetProtectionRequest>,
) -> ServerResult<Json<Value>> {
    if !role.can_operate_dcs() {
        return Err(ServerError::Forbidden(
            "Current role is not authorized to reset protection blocks".to_string(),
        ));
    }

    let cmd = RuntimeCommand::ResetProtection {
        block_name: req.block_name.clone(),
    };
    host.dispatch_command(cmd).await?;

    audit_log(
        "PROTECTION_RESET",
        &format!("{role}"),
        &format!("/api/v1/protection/reset: {}", req.block_name),
        "Protection block trip reset",
        true,
    );

    Ok(Json(json!({ "status": "ok" })))
}
