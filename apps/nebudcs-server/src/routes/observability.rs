//! Subsystem Observability & Global Diagnostics REST routes.

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

use crate::diagnostics_manager::DiagnosticsSummaryDto;
use crate::error::{ServerError, ServerResult};
use crate::host::HostHandle;
use crate::observability::metrics::collect_subsystem_observability;
use crate::security::UserRole;
use nebudcs_protocol::{DiagnosticItemDto, SubsystemObservabilityDto};

#[derive(Debug, Deserialize)]
pub struct DiagnosticsQueryParams {
    pub domain: Option<String>,
    pub severity: Option<String>,
}

/// Returns comprehensive telemetry across all NebuDCS subsystems.
pub async fn get_telemetry(
    State(host): State<HostHandle>,
    _role: UserRole,
) -> ServerResult<Json<SubsystemObservabilityDto>> {
    let obs = collect_subsystem_observability(&host).await;
    Ok(Json(obs))
}

/// Returns cross-domain diagnostic items correlating faults across subsystems.
pub async fn get_diagnostics(
    State(host): State<HostHandle>,
    _role: UserRole,
    Query(params): Query<DiagnosticsQueryParams>,
) -> ServerResult<Json<Vec<DiagnosticItemDto>>> {
    let diag = host
        .diagnostics_manager()
        .read()
        .map_err(|e| ServerError::Internal(e.to_string()))?;

    let items = diag.list(params.domain.as_deref(), params.severity.as_deref());
    Ok(Json(items))
}

/// Returns diagnostics summary counts by severity level.
pub async fn get_diagnostics_summary(
    State(host): State<HostHandle>,
    _role: UserRole,
) -> ServerResult<Json<DiagnosticsSummaryDto>> {
    let diag = host
        .diagnostics_manager()
        .read()
        .map_err(|e| ServerError::Internal(e.to_string()))?;

    Ok(Json(diag.summary()))
}
