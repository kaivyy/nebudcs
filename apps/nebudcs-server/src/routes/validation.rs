//! Scientific Validation, Benchmark & Certification REST endpoints.
//!
//! Standards Reference: ASME V&V 10 / 20 / ISO 9001 / IEC 61511 / ISA-84 / API 610 / API 617.
//! Note: Virtual simulation and internal engineering validation runtime.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use nebudcs_protocol::{
    ModelValidationSummaryDto, ReferenceDatasetDto, ValidationCaseDetailDto,
    ValidationCaseSummaryDto, ValidationExecutiveSummaryDto, ValidationManifestDto,
    ValidationRunRequestDto, ValidationRunResponseDto,
};

use crate::error::{ServerError, ServerResult};
use crate::host::HostHandle;
use crate::security::{audit_log, UserRole};

#[derive(Debug, Deserialize)]
pub struct CaseListQuery {
    pub domain: Option<String>,
}

/// Returns executive high-level summary of scientific validation and benchmark coverage.
pub async fn get_validation_summary(
    State(host): State<HostHandle>,
) -> ServerResult<Json<ValidationExecutiveSummaryDto>> {
    let mgr = host.validation_manager().read().map_err(|e| {
        ServerError::Internal(format!("Validation manager lock poisoned: {e}"))
    })?;
    Ok(Json(mgr.summary()))
}

/// Lists summary records for all validation benchmark cases.
pub async fn list_validation_cases(
    State(host): State<HostHandle>,
    Query(query): Query<CaseListQuery>,
) -> ServerResult<Json<Vec<ValidationCaseSummaryDto>>> {
    let mgr = host.validation_manager().read().map_err(|e| {
        ServerError::Internal(format!("Validation manager lock poisoned: {e}"))
    })?;
    Ok(Json(mgr.list_cases(query.domain.as_deref())))
}

/// Retrieves complete specification, metrics, and citations for a designated benchmark case.
pub async fn get_validation_case(
    State(host): State<HostHandle>,
    Path(id): Path<String>,
) -> ServerResult<Json<ValidationCaseDetailDto>> {
    let mgr = host.validation_manager().read().map_err(|e| {
        ServerError::Internal(format!("Validation manager lock poisoned: {e}"))
    })?;
    if let Some(case) = mgr.get_case(&id) {
        Ok(Json(case))
    } else {
        Err(ServerError::NotFound(format!("Validation case '{id}' not found")))
    }
}

/// Executes scientific validation benchmark suite on demand.
///
/// Restricted to Operator, Instructor, Engineer, or Admin roles.
/// Trainee requests are rejected with 403 Forbidden.
pub async fn run_validation_benchmarks(
    State(host): State<HostHandle>,
    role: UserRole,
    Json(req): Json<ValidationRunRequestDto>,
) -> ServerResult<Json<ValidationRunResponseDto>> {
    if !role.can_run_validation() {
        audit_log(
            "VALIDATION_BENCHMARK_EXECUTE",
            &format!("{role}"),
            "/api/v1/validation/run",
            "Unauthorized attempt by trainee to execute benchmark suite",
            false,
        );
        return Err(ServerError::Forbidden(
            "Trainees are not authorized to run validation benchmarks. Requires Operator, Engineer, or Admin.".to_string(),
        ));
    }

    let mut mgr = host.validation_manager().write().map_err(|e| {
        ServerError::Internal(format!("Validation manager lock poisoned: {e}"))
    })?;

    let res = mgr.run_benchmarks(req.domain_filter.as_deref(), req.case_ids.as_deref());

    audit_log(
        "VALIDATION_BENCHMARK_EXECUTE",
        &format!("{role}"),
        "/api/v1/validation/run",
        &format!("Executed {} benchmark cases (all passed: {})", res.executed_cases, res.failed_cases == 0),
        true,
    );

    Ok(Json(res))
}

/// Lists all authoritative reference datasets.
pub async fn list_validation_references(
    State(host): State<HostHandle>,
) -> ServerResult<Json<Vec<ReferenceDatasetDto>>> {
    let mgr = host.validation_manager().read().map_err(|e| {
        ServerError::Internal(format!("Validation manager lock poisoned: {e}"))
    })?;
    Ok(Json(mgr.list_references()))
}

/// Lists all model validation summaries.
pub async fn list_validation_models(
    State(host): State<HostHandle>,
) -> ServerResult<Json<Vec<ModelValidationSummaryDto>>> {
    let mgr = host.validation_manager().read().map_err(|e| {
        ServerError::Internal(format!("Validation manager lock poisoned: {e}"))
    })?;
    Ok(Json(mgr.list_models()))
}

/// Generates and exports the complete validation certification manifest.
pub async fn get_validation_manifest(
    State(host): State<HostHandle>,
) -> ServerResult<Json<ValidationManifestDto>> {
    let mgr = host.validation_manager().read().map_err(|e| {
        ServerError::Internal(format!("Validation manager lock poisoned: {e}"))
    })?;
    Ok(Json(mgr.generate_manifest()))
}

/// Registers a custom or project-specific benchmark case.
pub async fn register_custom_benchmark(
    State(host): State<HostHandle>,
    role: UserRole,
    Json(req): Json<ValidationCaseDetailDto>,
) -> ServerResult<Json<Value>> {
    if !role.can_register_benchmarks() {
        return Err(ServerError::Forbidden(
            "Registration of custom benchmarks requires Engineer or Admin permission.".to_string(),
        ));
    }

    let mut mgr = host.validation_manager().write().map_err(|e| {
        ServerError::Internal(format!("Validation manager lock poisoned: {e}"))
    })?;

    let case_id = req.id.clone();
    mgr.register_custom_case(req).map_err(ServerError::InvalidRequest)?;

    audit_log(
        "REGISTER_CUSTOM_BENCHMARK",
        &format!("{role}"),
        &format!("/api/v1/validation/cases/{case_id}"),
        "Registered custom benchmark case",
        true,
    );

    Ok(Json(json!({
        "status": "REGISTERED",
        "case_id": case_id,
    })))
}
