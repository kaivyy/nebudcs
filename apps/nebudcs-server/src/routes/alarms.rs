//! Alarm and protection system routes.

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use nebudcs_protocol::{AlarmSnapshotDto, RuntimeCommand};

use crate::error::ServerResult;
use crate::host::HostHandle;

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

/// Acknowledges an active alarm.
pub async fn acknowledge_alarm(
    State(host): State<HostHandle>,
    Json(req): Json<AcknowledgeAlarmRequest>,
) -> ServerResult<Json<Value>> {
    let cmd = RuntimeCommand::AcknowledgeAlarm {
        alarm_name: req.alarm_name,
    };
    host.dispatch_command(cmd).await?;
    Ok(Json(json!({ "status": "ok" })))
}

/// Shelves an alarm for a specified duration in seconds.
pub async fn shelve_alarm(
    State(host): State<HostHandle>,
    Json(req): Json<ShelveAlarmRequest>,
) -> ServerResult<Json<Value>> {
    let cmd = RuntimeCommand::ShelveAlarm {
        alarm_name: req.alarm_name,
        duration_seconds: req.duration_seconds,
    };
    host.dispatch_command(cmd).await?;
    Ok(Json(json!({ "status": "ok" })))
}

/// Resets a latched protection trip.
pub async fn reset_protection(
    State(host): State<HostHandle>,
    Json(req): Json<ResetProtectionRequest>,
) -> ServerResult<Json<Value>> {
    let cmd = RuntimeCommand::ResetProtection {
        block_name: req.block_name,
    };
    host.dispatch_command(cmd).await?;
    Ok(Json(json!({ "status": "ok" })))
}
