//! Simulation lifecycle and snapshot routes.

use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

use nebudcs_protocol::{RuntimeCommand, SimControlCommand, StateSnapshotDto};

use crate::error::ServerResult;
use crate::host::HostHandle;

/// Retrieves a coherent full point-in-time state snapshot.
pub async fn get_snapshot(State(host): State<HostHandle>) -> Json<StateSnapshotDto> {
    let snapshot = host.get_snapshot().await;
    Json(snapshot)
}

/// Executes a simulation control lifecycle command (play, pause, step, speed, reset).
pub async fn post_sim_control(
    State(host): State<HostHandle>,
    Json(control_cmd): Json<SimControlCommand>,
) -> ServerResult<Json<Value>> {
    let cmd = RuntimeCommand::SimControl(control_cmd);
    host.dispatch_command(cmd).await?;
    Ok(Json(json!({ "status": "ok" })))
}
