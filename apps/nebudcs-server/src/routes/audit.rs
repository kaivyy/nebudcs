//! Audit Trail & Governance REST routes.

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

use crate::audit_manager::AuditSummaryDto;
use crate::error::{ServerError, ServerResult};
use crate::host::HostHandle;
use crate::security::UserRole;
use nebudcs_protocol::AuditEventDto;

#[derive(Debug, Deserialize)]
pub struct AuditQueryParams {
    pub actor: Option<String>,
    pub role: Option<String>,
    pub action: Option<String>,
    pub source: Option<String>,
    pub limit: Option<usize>,
}

/// Queries audit trail events with optional filtering.
pub async fn get_events(
    State(host): State<HostHandle>,
    _role: UserRole,
    Query(params): Query<AuditQueryParams>,
) -> ServerResult<Json<Vec<AuditEventDto>>> {
    let audit = host
        .audit_manager()
        .read()
        .map_err(|e| ServerError::Internal(e.to_string()))?;

    let events = audit.query(
        params.actor.as_deref(),
        params.role.as_deref(),
        params.action.as_deref(),
        params.source.as_deref(),
        params.limit.unwrap_or(50),
    );

    Ok(Json(events))
}

/// Returns summary statistics of the platform audit trail.
pub async fn get_summary(
    State(host): State<HostHandle>,
    _role: UserRole,
) -> ServerResult<Json<AuditSummaryDto>> {
    let audit = host
        .audit_manager()
        .read()
        .map_err(|e| ServerError::Internal(e.to_string()))?;

    Ok(Json(audit.get_summary()))
}

/// Appends a new audit event to the tamper-evident audit buffer.
pub async fn post_record(
    State(host): State<HostHandle>,
    role: UserRole,
    Json(event): Json<AuditEventDto>,
) -> ServerResult<Json<serde_json::Value>> {
    let mut audit = host
        .audit_manager()
        .write()
        .map_err(|e| ServerError::Internal(e.to_string()))?;

    audit.record(
        "UI-ACTION",
        &event.actor,
        &format!("{role}"),
        &event.source,
        &event.target,
        &event.action,
        event.prev_value,
        event.new_value,
        &event.status,
        event.reason,
        event.correlation_id,
        event.sim_time_secs,
    );

    Ok(Json(serde_json::json!({
        "status": "recorded"
    })))
}
