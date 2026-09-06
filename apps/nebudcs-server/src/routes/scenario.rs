//! Scenario and fault injection routes.

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use nebudcs_protocol::{RuntimeCommand, ScenarioCommand, ScenarioStatusDto};
use nebudcs_scenario::{FaultDefinition, ScenarioDefinition};

use crate::error::ServerResult;
use crate::host::HostHandle;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AbortScenarioRequest {
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearFaultRequest {
    pub fault_id: String,
}

/// Retrieves current scenario and fault injection status.
pub async fn get_scenario_status(State(host): State<HostHandle>) -> Json<ScenarioStatusDto> {
    let status = host.get_scenario_status().await;
    Json(status)
}

/// Loads a declarative scenario definition.
pub async fn load_scenario(
    State(host): State<HostHandle>,
    Json(scenario): Json<ScenarioDefinition>,
) -> ServerResult<Json<Value>> {
    let scenario_json = serde_json::to_string(&scenario)
        .map_err(|e| crate::error::ServerError::InvalidRequest(e.to_string()))?;
    let cmd = RuntimeCommand::Scenario(ScenarioCommand::Load { scenario_json });
    host.dispatch_command(cmd).await?;
    Ok(Json(json!({ "status": "ok", "scenario_id": scenario.id })))
}

/// Arms initial conditions and starts scenario execution.
pub async fn start_scenario(State(host): State<HostHandle>) -> ServerResult<Json<Value>> {
    let cmd = RuntimeCommand::Scenario(ScenarioCommand::Start);
    host.dispatch_command(cmd).await?;
    Ok(Json(json!({ "status": "ok" })))
}

/// Pauses scenario execution.
pub async fn pause_scenario(State(host): State<HostHandle>) -> ServerResult<Json<Value>> {
    let cmd = RuntimeCommand::Scenario(ScenarioCommand::Pause);
    host.dispatch_command(cmd).await?;
    Ok(Json(json!({ "status": "ok" })))
}

/// Resumes scenario execution.
pub async fn resume_scenario(State(host): State<HostHandle>) -> ServerResult<Json<Value>> {
    let cmd = RuntimeCommand::Scenario(ScenarioCommand::Resume);
    host.dispatch_command(cmd).await?;
    Ok(Json(json!({ "status": "ok" })))
}

/// Aborts scenario execution.
pub async fn abort_scenario(
    State(host): State<HostHandle>,
    Json(req): Json<AbortScenarioRequest>,
) -> ServerResult<Json<Value>> {
    let cmd = RuntimeCommand::Scenario(ScenarioCommand::Abort { reason: req.reason });
    host.dispatch_command(cmd).await?;
    Ok(Json(json!({ "status": "ok" })))
}

/// Resets scenario to loaded baseline.
pub async fn reset_scenario(State(host): State<HostHandle>) -> ServerResult<Json<Value>> {
    let cmd = RuntimeCommand::Scenario(ScenarioCommand::Reset);
    host.dispatch_command(cmd).await?;
    Ok(Json(json!({ "status": "ok" })))
}

/// Injects an ad-hoc fault into the simulation.
pub async fn inject_fault(
    State(host): State<HostHandle>,
    Json(fault): Json<FaultDefinition>,
) -> ServerResult<Json<Value>> {
    let fault_json = serde_json::to_string(&fault)
        .map_err(|e| crate::error::ServerError::InvalidRequest(e.to_string()))?;
    let cmd = RuntimeCommand::Scenario(ScenarioCommand::InjectFault { fault_json });
    host.dispatch_command(cmd).await?;
    Ok(Json(json!({ "status": "ok", "fault_id": fault.id })))
}

/// Manually clears an active fault.
pub async fn clear_fault(
    State(host): State<HostHandle>,
    Json(req): Json<ClearFaultRequest>,
) -> ServerResult<Json<Value>> {
    let cmd = RuntimeCommand::Scenario(ScenarioCommand::ClearFault {
        fault_id: req.fault_id,
    });
    host.dispatch_command(cmd).await?;
    Ok(Json(json!({ "status": "ok" })))
}
