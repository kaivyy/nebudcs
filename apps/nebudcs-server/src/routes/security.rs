//! REST API endpoints for IEC 62443 Industrial Cybersecurity & Zones.

use axum::extract::{Query, State};
use axum::Json;
use nebudcs_protocol::{
    AuditVerificationResultDto, PurdueConduitDto, PurdueZoneDto, SecurityIncidentDto,
    SecurityStatusDto, UpdateConduitPolicyRequestDto,
};
use serde::Deserialize;

use crate::error::{ServerError, ServerResult};
use crate::host::HostHandle;
use crate::security::UserRole;

#[derive(Debug, Deserialize)]
pub struct IncidentsQueryParams {
    pub limit: Option<usize>,
}

/// GET /api/v1/security/status
/// Retrieves high-level IEC 62443 cybersecurity posture and active threat level.
pub async fn get_security_status(
    State(host): State<HostHandle>,
    _role: UserRole,
) -> ServerResult<Json<SecurityStatusDto>> {
    let audit_guard = host
        .audit_manager()
        .read()
        .map_err(|e| ServerError::Internal(e.to_string()))?;
    let sec_guard = host
        .security_manager()
        .read()
        .map_err(|e| ServerError::Internal(e.to_string()))?;
    let status = sec_guard.get_security_status(&audit_guard);
    Ok(Json(status))
}

/// GET /api/v1/security/zones
/// Retrieves all Purdue Model zones (Level 0 to Level 5).
pub async fn get_zones(
    State(host): State<HostHandle>,
    _role: UserRole,
) -> ServerResult<Json<Vec<PurdueZoneDto>>> {
    let sec_guard = host
        .security_manager()
        .read()
        .map_err(|e| ServerError::Internal(e.to_string()))?;
    Ok(Json(sec_guard.get_zones()))
}

/// GET /api/v1/security/conduits
/// Retrieves all industrial conduits connecting Purdue zones.
pub async fn get_conduits(
    State(host): State<HostHandle>,
    _role: UserRole,
) -> ServerResult<Json<Vec<PurdueConduitDto>>> {
    let sec_guard = host
        .security_manager()
        .read()
        .map_err(|e| ServerError::Internal(e.to_string()))?;
    Ok(Json(sec_guard.get_conduits()))
}

/// POST /api/v1/security/conduits/policy
/// Updates conduit security policy and protocol whitelisting (Admin/Engineer only).
pub async fn update_conduit_policy(
    State(host): State<HostHandle>,
    role: UserRole,
    Json(req): Json<UpdateConduitPolicyRequestDto>,
) -> ServerResult<Json<PurdueConduitDto>> {
    if !role.can_modify_topology() {
        if let Ok(mut sec_guard) = host.security_manager().write() {
            sec_guard.ids.detect_privilege_escalation(
                "operator_or_trainee",
                &role.to_string(),
                "UPDATE_CONDUIT_POLICY",
            );
        }
        return Err(ServerError::Forbidden(
            "Only Admin or Engineer roles may modify industrial conduit policies".to_string(),
        ));
    }

    let conduit_id = req.conduit_id.clone();
    {
        let mut sec_guard = host
            .security_manager()
            .write()
            .map_err(|e| ServerError::Internal(e.to_string()))?;
        let updated = sec_guard.update_conduit_policy(req);
        if !updated {
            return Err(ServerError::NotFound(format!("Conduit '{conduit_id}' not found")));
        }
    }

    let sec_guard = host
        .security_manager()
        .read()
        .map_err(|e| ServerError::Internal(e.to_string()))?;
    let conduits = sec_guard.get_conduits();
    let conduit = conduits
        .into_iter()
        .find(|c| c.conduit_id == conduit_id)
        .ok_or_else(|| ServerError::NotFound(format!("Conduit '{conduit_id}' not found")))?;

    // Audit log this security reconfiguration
    if let Ok(mut audit) = host.audit_manager().write() {
        audit.record(
            "SEC-POL",
            "SecurityAdmin",
            &role.to_string(),
            "SecurityPanel",
            &conduit_id,
            "UPDATE_CONDUIT_POLICY",
            None,
            Some(conduit.status.clone()),
            "SUCCESS",
            Some("Conduit protocol whitelist policy updated".to_string()),
            None,
            0.0,
        );
    }

    Ok(Json(conduit))
}

/// GET /api/v1/security/incidents
/// Retrieves intrusion detection incidents and anomaly alerts.
pub async fn get_incidents(
    State(host): State<HostHandle>,
    _role: UserRole,
    Query(params): Query<IncidentsQueryParams>,
) -> ServerResult<Json<Vec<SecurityIncidentDto>>> {
    let sec_guard = host
        .security_manager()
        .read()
        .map_err(|e| ServerError::Internal(e.to_string()))?;
    let limit = params.limit.unwrap_or(50);
    Ok(Json(sec_guard.get_incidents(limit)))
}

/// POST /api/v1/security/audit/verify
/// Performs mathematical cryptographic verification of the SHA-256 audit hash chain.
pub async fn verify_audit_integrity(
    State(host): State<HostHandle>,
    _role: UserRole,
) -> ServerResult<Json<AuditVerificationResultDto>> {
    let audit_guard = host
        .audit_manager()
        .read()
        .map_err(|e| ServerError::Internal(e.to_string()))?;
    let mut sec_guard = host
        .security_manager()
        .write()
        .map_err(|e| ServerError::Internal(e.to_string()))?;
    let result = sec_guard.verify_audit_integrity(&audit_guard);
    Ok(Json(result))
}
