//! Virtual industrial controller management and diagnostic REST endpoints.

use axum::extract::{Path, State};
use axum::Json;
use serde_json::{json, Value};

use nebudcs_protocol::{
    ControllerDetailDto, ControllerSummaryDto, UpdateControllerStateRequestDto,
};

use crate::error::{ServerError, ServerResult};
use crate::host::HostHandle;
use crate::security::{audit_log, UserRole};

/// Lists all configured virtual controllers.
pub async fn list_controllers(
    State(host): State<HostHandle>,
) -> ServerResult<Json<Vec<ControllerSummaryDto>>> {
    Ok(Json(host.get_controllers().await))
}

/// Retrieves detailed status, task execution statistics, and diagnostics for a controller.
pub async fn get_controller(
    State(host): State<HostHandle>,
    Path(id): Path<String>,
) -> ServerResult<Json<ControllerDetailDto>> {
    if let Some(detail) = host.get_controller(&id).await {
        Ok(Json(detail))
    } else {
        Err(ServerError::NotFound(format!(
            "Controller '{id}' not found"
        )))
    }
}

/// Changes the operational lifecycle state of a virtual controller (Start, Stop, Pause, Resume, Reset).
///
/// Requires Engineer or Admin privileges per ISA-62443 / RBAC.
pub async fn update_controller_state(
    State(host): State<HostHandle>,
    role: UserRole,
    Path(id): Path<String>,
    Json(req): Json<UpdateControllerStateRequestDto>,
) -> ServerResult<Json<Value>> {
    if !role.can_modify_topology() {
        audit_log(
            "CONTROLLER_STATE_MUTATION",
            &format!("{role}"),
            &format!("/api/v1/controllers/{id}/state"),
            "Unauthorized attempt to mutate controller lifecycle state",
            false,
        );
        return Err(ServerError::Forbidden(
            "Engineer or Admin role required to mutate controller lifecycle state".to_string(),
        ));
    }

    let cmd = nebudcs_protocol::RuntimeCommand::ControllerAction {
        controller_id: id.clone(),
        action: req.action.clone(),
    };

    host.dispatch_command(cmd).await?;

    audit_log(
        "CONTROLLER_STATE_MUTATION",
        &format!("{role}"),
        &format!("/api/v1/controllers/{id}/state"),
        &format!("Action '{}' applied successfully", req.action),
        true,
    );

    Ok(Json(json!({
        "status": "ok",
        "controller_id": id,
        "action": req.action,
    })))
}

/// Compiles and validates IEC 61131-3 logic (ST, LD, FBD, SFC) and returns structured diagnostics.
pub async fn compile_iec(
    Json(req): Json<nebudcs_protocol::IecCompileRequestDto>,
) -> ServerResult<Json<nebudcs_protocol::IecCompileResponseDto>> {
    let result = match req.language.to_ascii_uppercase().as_str() {
        "ST" => nebudcs_control::iec61131::compile_st(&req.source_text),
        "LD" => {
            let diag: Result<nebudcs_control::iec61131::LdDiagram, _> =
                serde_json::from_str(&req.source_text);
            match diag {
                Ok(d) => nebudcs_control::iec61131::compile_ladder(&d),
                Err(e) => Err(nebudcs_control::error::ControlError::InvalidParameter(
                    format!("Invalid LD JSON: {e}"),
                )),
            }
        }
        "FBD" => {
            let graph: Result<nebudcs_control::iec61131::FbdGraph, _> =
                serde_json::from_str(&req.source_text);
            match graph {
                Ok(g) => nebudcs_control::iec61131::compile_fbd(&g),
                Err(e) => Err(nebudcs_control::error::ControlError::InvalidParameter(
                    format!("Invalid FBD JSON: {e}"),
                )),
            }
        }
        "SFC" => {
            let chart: Result<nebudcs_control::iec61131::SfcChart, _> =
                serde_json::from_str(&req.source_text);
            match chart {
                Ok(c) => nebudcs_control::iec61131::compile_sfc(&c),
                Err(e) => Err(nebudcs_control::error::ControlError::InvalidParameter(
                    format!("Invalid SFC JSON: {e}"),
                )),
            }
        }
        other => Err(nebudcs_control::error::ControlError::InvalidParameter(
            format!("Unsupported IEC language '{other}'"),
        )),
    };

    match result {
        Ok((symbols, stmts)) => {
            let variables = symbols
                .declarations()
                .values()
                .map(|decl| nebudcs_protocol::IecVariableDto {
                    name: decl.name.clone(),
                    data_type: decl.data_type.keyword(),
                    value: format!("{:?}", decl.initial_value),
                    scope: format!("{:?}", decl.scope),
                    address_binding: decl.address_binding.clone(),
                })
                .collect();

            Ok(Json(nebudcs_protocol::IecCompileResponseDto {
                success: true,
                error_message: None,
                symbol_count: symbols.declarations().len(),
                statement_count: stmts.len(),
                variables,
            }))
        }
        Err(e) => Ok(Json(nebudcs_protocol::IecCompileResponseDto {
            success: false,
            error_message: Some(e.to_string()),
            symbol_count: 0,
            statement_count: 0,
            variables: Vec::new(),
        })),
    }
}

/// Updates a DCS Control Module's mode, SP, or manual output.
pub async fn update_control_module(
    State(host): State<HostHandle>,
    role: UserRole,
    Path((ctrl_id, mod_id)): Path<(String, String)>,
    Json(req): Json<nebudcs_protocol::ControlModuleActionRequestDto>,
) -> ServerResult<Json<Value>> {
    if !role.can_operate_dcs() {
        return Err(ServerError::Forbidden(
            "Operator, Engineer, or Admin role required to command control module".to_string(),
        ));
    }

    let cmd = nebudcs_protocol::RuntimeCommand::ControlModuleAction {
        controller_id: ctrl_id.clone(),
        module_id: mod_id.clone(),
        mode: req.mode.clone(),
        sp: req.sp,
        manual_out: req.manual_out,
    };

    host.dispatch_command(cmd).await?;

    audit_log(
        "CONTROL_MODULE_COMMAND",
        &format!("{role}"),
        &format!("/api/v1/controllers/{ctrl_id}/modules/{mod_id}"),
        &format!(
            "Module command applied: mode={:?}, sp={:?}, out={:?}",
            req.mode, req.sp, req.manual_out
        ),
        true,
    );

    Ok(Json(json!({
        "status": "ok",
        "controller_id": ctrl_id,
        "module_id": mod_id,
        "mode": req.mode,
        "sp": req.sp,
        "manual_out": req.manual_out,
    })))
}

/// Injects or clears a controlled fault on a virtual controller.
pub async fn inject_controller_fault(
    State(host): State<HostHandle>,
    role: UserRole,
    Path(id): Path<String>,
    Json(req): Json<nebudcs_protocol::ControllerFaultInjectionRequestDto>,
) -> ServerResult<Json<Value>> {
    if !role.can_modify_topology() {
        return Err(ServerError::Forbidden(
            "Engineer or Admin role required to inject controller faults".to_string(),
        ));
    }

    let cmd = nebudcs_protocol::RuntimeCommand::ControllerFaultAction {
        controller_id: id.clone(),
        fault_type: req.fault_type.clone(),
        description: req.description.clone(),
        recoverable: req.recoverable,
    };

    host.dispatch_command(cmd).await?;

    audit_log(
        "CONTROLLER_FAULT_INJECTION",
        &format!("{role}"),
        &format!("/api/v1/controllers/{id}/fault"),
        &format!(
            "Fault injected: type={}, recoverable={}",
            req.fault_type, req.recoverable
        ),
        true,
    );

    Ok(Json(json!({
        "status": "ok",
        "controller_id": id,
        "fault_type": req.fault_type,
        "recoverable": req.recoverable,
    })))
}

/// Triggers high-availability redundancy switchover from Primary to Standby controller.
pub async fn trigger_redundancy_switchover(
    State(host): State<HostHandle>,
    role: UserRole,
    Path(id): Path<String>,
) -> ServerResult<Json<Value>> {
    if !role.can_modify_topology() {
        return Err(ServerError::Forbidden(
            "Engineer or Admin role required to initiate redundancy switchover".to_string(),
        ));
    }

    let cmd = nebudcs_protocol::RuntimeCommand::ControllerAction {
        controller_id: id.clone(),
        action: "switchover".to_string(),
    };

    host.dispatch_command(cmd).await?;

    audit_log(
        "CONTROLLER_REDUNDANCY_SWITCHOVER",
        &format!("{role}"),
        &format!("/api/v1/controllers/{id}/redundancy/switchover"),
        "Redundancy switchover triggered successfully",
        true,
    );

    Ok(Json(json!({
        "status": "ok",
        "controller_id": id,
        "action": "switchover",
    })))
}
