//! Project management, validation, and simulation synchronization REST API.

use axum::extract::{Path, State};
use axum::Json;
use serde_json::{json, Value};

use nebudcs_protocol::{
    AdvancedControlStrategyDto, CreateProjectRequestDto, CrossReferenceQueryDto,
    CrossReferenceResultDto, ProjectDetailDto, ProjectSummaryDto, ProjectValidationResponseDto,
    SaveProjectRequestDto,
};

use crate::error::{ServerError, ServerResult};
use crate::host::HostHandle;
use crate::security::{audit_log, sanitize_identifier, UserRole};

/// Lists all available projects.
pub async fn list_projects(
    State(host): State<HostHandle>,
) -> ServerResult<Json<Vec<ProjectSummaryDto>>> {
    let mgr = host.project_manager().read().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire project manager lock: {e}"))
    })?;
    Ok(Json(mgr.list_projects()))
}

/// Creates a new empty project.
pub async fn create_project(
    State(host): State<HostHandle>,
    role: UserRole,
    Json(req): Json<CreateProjectRequestDto>,
) -> ServerResult<Json<ProjectDetailDto>> {
    if !role.can_modify_topology() {
        audit_log(
            "PROJECT_CREATE",
            &format!("{role}"),
            "/api/v1/projects",
            "Non-engineer attempted project creation",
            false,
        );
        return Err(ServerError::Forbidden(
            "Engineer or Admin role required to create projects".to_string(),
        ));
    }

    let mut mgr = host.project_manager().write().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire project manager lock: {e}"))
    })?;
    let dto = mgr.create_project(req);
    audit_log(
        "PROJECT_CREATE",
        &format!("{role}"),
        &format!("/api/v1/projects/{}", dto.id),
        "Project created",
        true,
    );
    Ok(Json(dto))
}

/// Retrieves complete project details for the Engineering Studio.
pub async fn get_project(
    State(host): State<HostHandle>,
    Path(id): Path<String>,
) -> ServerResult<Json<ProjectDetailDto>> {
    let safe_id = sanitize_identifier(&id)?;
    let mgr = host.project_manager().read().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire project manager lock: {e}"))
    })?;
    let project = mgr
        .get_project(&safe_id)
        .ok_or_else(|| ServerError::NotFound(format!("Project '{safe_id}' not found")))?;
    Ok(Json(project))
}

/// Saves an updated project from the Engineering Studio.
pub async fn save_project(
    State(host): State<HostHandle>,
    role: UserRole,
    Path(id): Path<String>,
    Json(mut req): Json<SaveProjectRequestDto>,
) -> ServerResult<Json<ProjectDetailDto>> {
    if !role.can_modify_topology() {
        audit_log(
            "PROJECT_SAVE",
            &format!("{role}"),
            &format!("/api/v1/projects/{id}"),
            "Non-engineer attempted project save",
            false,
        );
        return Err(ServerError::Forbidden(
            "Engineer or Admin role required to modify project topology".to_string(),
        ));
    }

    let safe_id = sanitize_identifier(&id)?;
    req.project.id = safe_id.clone();
    let mut mgr = host.project_manager().write().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire project manager lock: {e}"))
    })?;
    let saved = mgr.save_project(req).map_err(ServerError::InvalidRequest)?;
    audit_log(
        "PROJECT_SAVE",
        &format!("{role}"),
        &format!("/api/v1/projects/{safe_id}"),
        "Project saved",
        true,
    );
    Ok(Json(saved))
}

/// Deletes a project from the server.
pub async fn delete_project(
    State(host): State<HostHandle>,
    role: UserRole,
    Path(id): Path<String>,
) -> ServerResult<Json<Value>> {
    if !role.can_modify_topology() {
        audit_log(
            "PROJECT_DELETE",
            &format!("{role}"),
            &format!("/api/v1/projects/{id}"),
            "Non-engineer attempted project deletion",
            false,
        );
        return Err(ServerError::Forbidden(
            "Engineer or Admin role required to delete projects".to_string(),
        ));
    }

    let safe_id = sanitize_identifier(&id)?;
    let mut mgr = host.project_manager().write().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire project manager lock: {e}"))
    })?;
    let deleted = mgr
        .delete_project(&safe_id)
        .map_err(ServerError::Internal)?;
    if deleted {
        audit_log(
            "PROJECT_DELETE",
            &format!("{role}"),
            &format!("/api/v1/projects/{safe_id}"),
            "Project deleted",
            true,
        );
        Ok(Json(json!({ "status": "deleted", "id": safe_id })))
    } else {
        Err(ServerError::NotFound(format!(
            "Project '{safe_id}' not found"
        )))
    }
}

/// Validates a project against registered equipment definitions.
pub async fn validate_project_endpoint(
    State(host): State<HostHandle>,
    Path(id): Path<String>,
) -> ServerResult<Json<ProjectValidationResponseDto>> {
    let safe_id = sanitize_identifier(&id)?;
    let mgr = host.project_manager().read().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire project manager lock: {e}"))
    })?;

    let eq_mgr = host.equipment_manager().read().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire equipment manager lock: {e}"))
    })?;

    // Gather definitions from the equipment manager and library
    let mut defs = indexmap::IndexMap::new();
    for def in eq_mgr.list_definition_objects() {
        let d = def.to_domain_definition();
        defs.insert(d.id, d);
    }
    for t in eq_mgr.library().list_templates(None, None) {
        let d = t.definition.to_domain_definition();
        defs.insert(d.id, d);
    }
    for d in nebudcs_scripting::standard_domain_equipment_definitions() {
        defs.insert(d.id, d);
    }

    let res = mgr
        .validate_project(&safe_id, &defs)
        .map_err(ServerError::NotFound)?;
    Ok(Json(res))
}

/// Loads and synchronizes the project with the active simulation host.
pub async fn load_project_sim_endpoint(
    State(host): State<HostHandle>,
    role: UserRole,
    Path(id): Path<String>,
) -> ServerResult<Json<Value>> {
    if !role.can_control_simulation() {
        audit_log(
            "SIM_LOAD_PROJECT",
            &format!("{role}"),
            &format!("/api/v1/projects/{id}/load-sim"),
            "Permission denied to load project into sim",
            false,
        );
        return Err(ServerError::Forbidden(
            "Instructor, Engineer, or Admin role required to load project into simulation"
                .to_string(),
        ));
    }

    let safe_id = sanitize_identifier(&id)?;
    host.load_project_into_simulation(&safe_id).await?;
    audit_log(
        "SIM_LOAD_PROJECT",
        &format!("{role}"),
        &format!("/api/v1/projects/{safe_id}/load-sim"),
        "Project synchronized into sim",
        true,
    );
    Ok(Json(json!({
        "status": "ok",
        "message": format!("Project '{safe_id}' successfully synchronized into simulation host"),
        "project_id": safe_id
    })))
}

/// Inspects cross-references for a tag or entity within a project.
pub async fn get_cross_reference_endpoint(
    State(host): State<HostHandle>,
    Path(id): Path<String>,
    axum::extract::Query(query): axum::extract::Query<CrossReferenceQueryDto>,
) -> ServerResult<Json<CrossReferenceResultDto>> {
    let safe_id = sanitize_identifier(&id)?;
    let mgr = host.project_manager().read().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire project manager lock: {e}"))
    })?;
    let res = mgr
        .get_cross_reference(&safe_id, query)
        .map_err(ServerError::NotFound)?;
    Ok(Json(res))
}

/// Lists all advanced control strategies for a project.
pub async fn list_advanced_control_endpoint(
    State(host): State<HostHandle>,
    Path(id): Path<String>,
) -> ServerResult<Json<Vec<AdvancedControlStrategyDto>>> {
    let safe_id = sanitize_identifier(&id)?;
    let mgr = host.project_manager().read().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire project manager lock: {e}"))
    })?;
    let strategies = mgr
        .get_advanced_control(&safe_id)
        .ok_or_else(|| ServerError::NotFound(format!("Project '{safe_id}' not found")))?;
    Ok(Json(strategies))
}

/// Saves an advanced control strategy to a project.
pub async fn save_advanced_control_endpoint(
    State(host): State<HostHandle>,
    role: UserRole,
    Path(id): Path<String>,
    Json(strategy): Json<AdvancedControlStrategyDto>,
) -> ServerResult<Json<AdvancedControlStrategyDto>> {
    if !role.can_modify_topology() {
        audit_log(
            "STRATEGY_SAVE",
            &format!("{role}"),
            &format!("/api/v1/projects/{id}/advanced-control"),
            "Non-engineer attempted strategy save",
            false,
        );
        return Err(ServerError::Forbidden(
            "Engineer or Admin role required to modify advanced control strategies".to_string(),
        ));
    }

    let safe_id = sanitize_identifier(&id)?;
    let mut mgr = host.project_manager().write().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire project manager lock: {e}"))
    })?;
    let saved = mgr
        .save_advanced_control(&safe_id, strategy)
        .map_err(ServerError::InvalidRequest)?;
    audit_log(
        "STRATEGY_SAVE",
        &format!("{role}"),
        &format!("/api/v1/projects/{safe_id}/advanced-control/{}", saved.id),
        "Advanced control strategy saved",
        true,
    );
    Ok(Json(saved))
}

/// Deletes an advanced control strategy from a project.
pub async fn delete_advanced_control_endpoint(
    State(host): State<HostHandle>,
    role: UserRole,
    Path((id, strategy_id)): Path<(String, String)>,
) -> ServerResult<Json<Value>> {
    if !role.can_modify_topology() {
        audit_log(
            "STRATEGY_DELETE",
            &format!("{role}"),
            &format!("/api/v1/projects/{id}/advanced-control/{strategy_id}"),
            "Non-engineer attempted strategy deletion",
            false,
        );
        return Err(ServerError::Forbidden(
            "Engineer or Admin role required to delete advanced control strategies".to_string(),
        ));
    }

    let safe_id = sanitize_identifier(&id)?;
    let safe_strat_id = sanitize_identifier(&strategy_id)?;
    let mut mgr = host.project_manager().write().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire project manager lock: {e}"))
    })?;
    let deleted = mgr
        .delete_advanced_control(&safe_id, &safe_strat_id)
        .map_err(ServerError::InvalidRequest)?;
    if deleted {
        audit_log(
            "STRATEGY_DELETE",
            &format!("{role}"),
            &format!("/api/v1/projects/{safe_id}/advanced-control/{safe_strat_id}"),
            "Advanced control strategy deleted",
            true,
        );
        Ok(Json(json!({ "status": "deleted", "id": safe_strat_id })))
    } else {
        Err(ServerError::NotFound(format!(
            "Strategy '{safe_strat_id}' not found in project '{safe_id}'"
        )))
    }
}
