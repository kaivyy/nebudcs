//! PID control loop configuration and setpoint routes.

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use nebudcs_protocol::{PidModeDto, RuntimeCommand};

use crate::error::ServerResult;
use crate::host::HostHandle;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetPidSetpointRequest {
    pub loop_name: String,
    pub sp: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetPidModeRequest {
    pub loop_name: String,
    pub mode: PidModeDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetManualOutputRequest {
    pub loop_name: String,
    pub output: f64,
}

/// Sets the setpoint for a PID loop.
pub async fn set_pid_setpoint(
    State(host): State<HostHandle>,
    Json(req): Json<SetPidSetpointRequest>,
) -> ServerResult<Json<Value>> {
    let cmd = RuntimeCommand::SetSetpoint {
        loop_name: req.loop_name,
        sp: req.sp,
    };
    host.dispatch_command(cmd).await?;
    Ok(Json(json!({ "status": "ok" })))
}

/// Changes the operating mode of a PID loop (Auto, Manual, Cascade, etc.).
pub async fn set_pid_mode(
    State(host): State<HostHandle>,
    Json(req): Json<SetPidModeRequest>,
) -> ServerResult<Json<Value>> {
    let cmd = RuntimeCommand::SetMode {
        loop_name: req.loop_name,
        mode: req.mode,
    };
    host.dispatch_command(cmd).await?;
    Ok(Json(json!({ "status": "ok" })))
}

/// Sets the manual output demand when in Manual mode.
pub async fn set_manual_output(
    State(host): State<HostHandle>,
    Json(req): Json<SetManualOutputRequest>,
) -> ServerResult<Json<Value>> {
    let cmd = RuntimeCommand::SetManualOutput {
        loop_name: req.loop_name,
        output: req.output,
    };
    host.dispatch_command(cmd).await?;
    Ok(Json(json!({ "status": "ok" })))
}
