//! Safety Instrumented System (SIS) and Functional Safety REST endpoints.
//!
//! Standards Reference: IEC 61511 / ISA-84 / IEC 62443.
//! Note: Virtual simulation and engineering validation runtime.

use axum::extract::{Path, State};
use axum::Json;
use serde_json::{json, Value};

use nebudcs_protocol::{
    CauseEffectMatrixDto, ProofTestExecutionRequestDto, SifBypassRequestDto, SifDetailDto,
    SifResetRequestDto, SisStatusSummaryDto,
};

use crate::error::{ServerError, ServerResult};
use crate::host::HostHandle;
use crate::security::{audit_log, UserRole};

/// Returns executive high-level status summary for the Safety Instrumented System.
pub async fn get_sis_summary(
    State(host): State<HostHandle>,
) -> ServerResult<Json<SisStatusSummaryDto>> {
    Ok(Json(host.get_sis_summary().await))
}

/// Lists all configured Safety Instrumented Functions (SIFs) with live voting and diagnostic telemetry.
pub async fn list_sifs(State(host): State<HostHandle>) -> ServerResult<Json<Vec<SifDetailDto>>> {
    Ok(Json(host.get_sifs().await))
}

/// Retrieves complete telemetry, diagnostics, and channel state for a designated SIF.
pub async fn get_sif(
    State(host): State<HostHandle>,
    Path(id): Path<String>,
) -> ServerResult<Json<SifDetailDto>> {
    if let Some(sif) = host.get_sif(&id).await {
        Ok(Json(sif))
    } else {
        Err(ServerError::NotFound(format!(
            "SIF with ID '{id}' not found"
        )))
    }
}

/// Requests manual reset of a latched or tripped SIF.
///
/// Requires operator permission per ISA-84 / IEC 62443 RBAC.
pub async fn reset_sif(
    State(host): State<HostHandle>,
    role: UserRole,
    Path(id): Path<String>,
    Json(req): Json<SifResetRequestDto>,
) -> ServerResult<Json<Value>> {
    if !role.can_operate_dcs() {
        audit_log(
            "SIF_MANUAL_RESET",
            &format!("{role}"),
            &format!("/api/v1/sis/sifs/{id}/reset"),
            "Unauthorized attempt to perform safety reset",
            false,
        );
        return Err(ServerError::Forbidden(
            "Operator, Engineer, or Admin role required to execute safety reset".to_string(),
        ));
    }

    let cmd = nebudcs_protocol::RuntimeCommand::SifResetAction {
        sif_id: id.clone(),
        actor: req.actor.clone(),
    };

    host.dispatch_command(cmd).await?;

    audit_log(
        "SIF_MANUAL_RESET",
        &req.actor,
        &format!("/api/v1/sis/sifs/{id}/reset"),
        &format!("Manual safety reset commanded for SIF '{id}'"),
        true,
    );

    Ok(Json(json!({
        "status": "ok",
        "sif_id": id,
        "actor": req.actor,
        "message": "Reset request evaluated and applied"
    })))
}

/// Sets or clears an administrative bypass (maintenance inhibit) on an SIF.
///
/// Requires Engineer or Admin privileges per ISA-84 / IEC 62443 RBAC.
pub async fn set_sif_bypass(
    State(host): State<HostHandle>,
    role: UserRole,
    Path(id): Path<String>,
    Json(req): Json<SifBypassRequestDto>,
) -> ServerResult<Json<Value>> {
    if !role.can_modify_topology() {
        audit_log(
            "SIF_ADMINISTRATIVE_BYPASS",
            &format!("{role}"),
            &format!("/api/v1/sis/sifs/{id}/bypass"),
            "Unauthorized attempt to modify SIF bypass state",
            false,
        );
        return Err(ServerError::Forbidden(
            "Engineer or Admin role required to set or clear SIF bypasses".to_string(),
        ));
    }

    let cmd = nebudcs_protocol::RuntimeCommand::SifBypassAction {
        sif_id: id.clone(),
        bypass_type: req.bypass_type.clone(),
        authorized_by: req.authorized_by.clone(),
        reason: req.reason.clone(),
        duration_seconds: req.duration_seconds,
        active: req.active,
    };

    host.dispatch_command(cmd).await?;

    audit_log(
        "SIF_ADMINISTRATIVE_BYPASS",
        &req.authorized_by,
        &format!("/api/v1/sis/sifs/{id}/bypass"),
        &format!(
            "Bypass {} for SIF '{}' (type: {}, reason: '{}')",
            if req.active { "ENABLED" } else { "DISABLED" },
            id,
            req.bypass_type,
            req.reason
        ),
        true,
    );

    Ok(Json(json!({
        "status": "ok",
        "sif_id": id,
        "active": req.active,
        "bypass_type": req.bypass_type,
        "authorized_by": req.authorized_by,
    })))
}

/// Executes a simulated proof test on a designated SIF.
///
/// Requires Engineer or Admin privileges per IEC 61511 proof-test verification.
pub async fn execute_proof_test(
    State(host): State<HostHandle>,
    role: UserRole,
    Path(id): Path<String>,
    Json(req): Json<ProofTestExecutionRequestDto>,
) -> ServerResult<Json<Value>> {
    if !role.can_modify_topology() {
        audit_log(
            "SIF_PROOF_TEST_EXECUTION",
            &format!("{role}"),
            &format!("/api/v1/sis/sifs/{id}/proof-test"),
            "Unauthorized attempt to execute SIF proof test",
            false,
        );
        return Err(ServerError::Forbidden(
            "Engineer or Admin role required to execute SIF proof tests".to_string(),
        ));
    }

    let cmd = nebudcs_protocol::RuntimeCommand::SifProofTestAction {
        sif_id: id.clone(),
        technician: req.technician.clone(),
    };

    host.dispatch_command(cmd).await?;

    audit_log(
        "SIF_PROOF_TEST_EXECUTION",
        &req.technician,
        &format!("/api/v1/sis/sifs/{id}/proof-test"),
        &format!(
            "Proof test initiated for SIF '{id}' by '{}'",
            req.technician
        ),
        true,
    );

    Ok(Json(json!({
        "status": "ok",
        "sif_id": id,
        "technician": req.technician,
        "message": "Simulated proof test executed successfully"
    })))
}

/// Retrieves the live Cause and Effect Matrix dynamically extracted from all configured SIFs.
pub async fn get_cause_effect_matrix(
    State(host): State<HostHandle>,
) -> ServerResult<Json<CauseEffectMatrixDto>> {
    Ok(Json(host.get_cause_effect_matrix().await))
}
