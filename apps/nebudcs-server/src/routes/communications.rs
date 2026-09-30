//! Industrial Protocol & Communications REST API endpoints (Stage 7).
//!
//! Provides CRUD operations for connections, tag mappings, diagnostics,
//! ISA-95 zone architecture, and SSRF-guarded connection testing.

use std::collections::HashMap;

use axum::extract::{Path, Query, State};
use axum::Json;
use serde_json::{json, Value};

use nebudcs_protocol::dto::{
    CommunicationsConfigDto, ConnectionTestResultDto, IndustrialZoneDto, ProtocolConnectionDto,
    ProtocolDiagnosticsDto, TagMappingDto,
};

use crate::error::{ServerError, ServerResult};
use crate::host::HostHandle;
use crate::security::{audit_log, UserRole};

/// Lists all configured industrial protocol connections.
pub async fn list_connections(
    State(host): State<HostHandle>,
) -> ServerResult<Json<Vec<ProtocolConnectionDto>>> {
    let mgr = host.communications_manager().read().map_err(|e| {
        ServerError::Internal(format!(
            "Failed to acquire communications manager lock: {e}"
        ))
    })?;
    Ok(Json(mgr.list_connections()))
}

/// Retrieves a specific protocol connection by ID.
pub async fn get_connection(
    State(host): State<HostHandle>,
    Path(id): Path<String>,
) -> ServerResult<Json<ProtocolConnectionDto>> {
    let mgr = host.communications_manager().read().map_err(|e| {
        ServerError::Internal(format!(
            "Failed to acquire communications manager lock: {e}"
        ))
    })?;
    if let Some(conn) = mgr.get_connection(&id) {
        Ok(Json(conn))
    } else {
        Err(ServerError::NotFound(format!(
            "Connection '{id}' not found"
        )))
    }
}

/// Creates a new protocol connection with SSRF guard validation.
pub async fn create_connection(
    State(host): State<HostHandle>,
    role: UserRole,
    Json(conn): Json<ProtocolConnectionDto>,
) -> ServerResult<Json<ProtocolConnectionDto>> {
    if !role.can_modify_topology() {
        audit_log(
            "COMM_CONNECTION_CREATE",
            &format!("{role}"),
            "/api/v1/communications/connections",
            "Permission denied to create protocol connection",
            false,
        );
        return Err(ServerError::Forbidden(
            "Engineer or Admin role required to create protocol connections".to_string(),
        ));
    }

    let mut mgr = host.communications_manager().write().map_err(|e| {
        ServerError::Internal(format!(
            "Failed to acquire communications manager lock: {e}"
        ))
    })?;

    let created = mgr
        .add_connection(conn.clone())
        .map_err(ServerError::InvalidRequest)?;

    audit_log(
        "COMM_CONNECTION_CREATE",
        &format!("{role}"),
        &format!("/api/v1/communications/connections/{}", created.id),
        &format!(
            "Created connection {} ({}) endpoint {}",
            created.id, created.protocol, created.endpoint
        ),
        true,
    );

    Ok(Json(created))
}

/// Updates an existing protocol connection.
pub async fn update_connection(
    State(host): State<HostHandle>,
    role: UserRole,
    Path(id): Path<String>,
    Json(conn): Json<ProtocolConnectionDto>,
) -> ServerResult<Json<ProtocolConnectionDto>> {
    if !role.can_modify_topology() {
        return Err(ServerError::Forbidden(
            "Engineer or Admin role required to update protocol connections".to_string(),
        ));
    }

    let mut mgr = host.communications_manager().write().map_err(|e| {
        ServerError::Internal(format!(
            "Failed to acquire communications manager lock: {e}"
        ))
    })?;

    let updated = mgr
        .update_connection(&id, conn)
        .map_err(ServerError::InvalidRequest)?;

    Ok(Json(updated))
}

/// Deletes a protocol connection and its associated mappings.
pub async fn delete_connection(
    State(host): State<HostHandle>,
    role: UserRole,
    Path(id): Path<String>,
) -> ServerResult<Json<Value>> {
    if !role.can_modify_topology() {
        return Err(ServerError::Forbidden(
            "Engineer or Admin role required to delete protocol connections".to_string(),
        ));
    }

    let mut mgr = host.communications_manager().write().map_err(|e| {
        ServerError::Internal(format!(
            "Failed to acquire communications manager lock: {e}"
        ))
    })?;

    mgr.delete_connection(&id).map_err(ServerError::NotFound)?;

    audit_log(
        "COMM_CONNECTION_DELETE",
        &format!("{role}"),
        &format!("/api/v1/communications/connections/{id}"),
        &format!("Deleted connection {id}"),
        true,
    );

    Ok(Json(json!({
        "status": "ok",
        "deleted_id": id
    })))
}

/// Tests connection reachability with SSRF guard validation.
pub async fn test_connection(
    State(host): State<HostHandle>,
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> ServerResult<Json<ConnectionTestResultDto>> {
    let mgr = host.communications_manager().read().map_err(|e| {
        ServerError::Internal(format!(
            "Failed to acquire communications manager lock: {e}"
        ))
    })?;

    let protocol = params
        .get("protocol")
        .cloned()
        .unwrap_or_else(|| "ModbusTcp".to_string());

    let result = mgr.test_connection(&id, &protocol);
    Ok(Json(result))
}

/// Lists all configured tag mappings.
pub async fn list_mappings(
    State(host): State<HostHandle>,
) -> ServerResult<Json<Vec<TagMappingDto>>> {
    let mgr = host.communications_manager().read().map_err(|e| {
        ServerError::Internal(format!(
            "Failed to acquire communications manager lock: {e}"
        ))
    })?;
    Ok(Json(mgr.list_mappings()))
}

/// Retrieves a specific tag mapping by ID.
pub async fn get_mapping(
    State(host): State<HostHandle>,
    Path(id): Path<String>,
) -> ServerResult<Json<TagMappingDto>> {
    let mgr = host.communications_manager().read().map_err(|e| {
        ServerError::Internal(format!(
            "Failed to acquire communications manager lock: {e}"
        ))
    })?;
    if let Some(m) = mgr.get_mapping(&id) {
        Ok(Json(m))
    } else {
        Err(ServerError::NotFound(format!("Mapping '{id}' not found")))
    }
}

/// Creates a new tag mapping.
pub async fn create_mapping(
    State(host): State<HostHandle>,
    role: UserRole,
    Json(mapping): Json<TagMappingDto>,
) -> ServerResult<Json<TagMappingDto>> {
    if !role.can_modify_topology() {
        return Err(ServerError::Forbidden(
            "Engineer or Admin role required to create tag mappings".to_string(),
        ));
    }

    let mut mgr = host.communications_manager().write().map_err(|e| {
        ServerError::Internal(format!(
            "Failed to acquire communications manager lock: {e}"
        ))
    })?;

    let created = mgr
        .add_mapping(mapping)
        .map_err(ServerError::InvalidRequest)?;

    Ok(Json(created))
}

/// Updates an existing tag mapping.
pub async fn update_mapping(
    State(host): State<HostHandle>,
    role: UserRole,
    Path(id): Path<String>,
    Json(mapping): Json<TagMappingDto>,
) -> ServerResult<Json<TagMappingDto>> {
    if !role.can_modify_topology() {
        return Err(ServerError::Forbidden(
            "Engineer or Admin role required to update tag mappings".to_string(),
        ));
    }

    let mut mgr = host.communications_manager().write().map_err(|e| {
        ServerError::Internal(format!(
            "Failed to acquire communications manager lock: {e}"
        ))
    })?;

    let updated = mgr
        .update_mapping(&id, mapping)
        .map_err(ServerError::InvalidRequest)?;

    Ok(Json(updated))
}

/// Deletes a tag mapping.
pub async fn delete_mapping(
    State(host): State<HostHandle>,
    role: UserRole,
    Path(id): Path<String>,
) -> ServerResult<Json<Value>> {
    if !role.can_modify_topology() {
        return Err(ServerError::Forbidden(
            "Engineer or Admin role required to delete tag mappings".to_string(),
        ));
    }

    let mut mgr = host.communications_manager().write().map_err(|e| {
        ServerError::Internal(format!(
            "Failed to acquire communications manager lock: {e}"
        ))
    })?;

    mgr.delete_mapping(&id).map_err(ServerError::NotFound)?;

    Ok(Json(json!({
        "status": "ok",
        "deleted_id": id
    })))
}

/// Obtains diagnostic metrics across all active protocol connections.
pub async fn get_diagnostics(
    State(host): State<HostHandle>,
) -> ServerResult<Json<Vec<ProtocolDiagnosticsDto>>> {
    let mgr = host.communications_manager().read().map_err(|e| {
        ServerError::Internal(format!(
            "Failed to acquire communications manager lock: {e}"
        ))
    })?;
    Ok(Json(mgr.get_diagnostics()))
}

/// Obtains ISA-95 zones and security architecture definition.
pub async fn get_architecture(
    State(host): State<HostHandle>,
) -> ServerResult<Json<Vec<IndustrialZoneDto>>> {
    let mgr = host.communications_manager().read().map_err(|e| {
        ServerError::Internal(format!(
            "Failed to acquire communications manager lock: {e}"
        ))
    })?;
    Ok(Json(mgr.get_zones()))
}

/// Obtains project-level communications configuration.
pub async fn get_project_communications(
    State(host): State<HostHandle>,
    Path(id): Path<String>,
) -> ServerResult<Json<CommunicationsConfigDto>> {
    let pm = host.project_manager().read().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire project manager lock: {e}"))
    })?;

    let proj = pm
        .get_project(&id)
        .ok_or_else(|| ServerError::NotFound(format!("Project '{id}' not found")))?;

    if let Some(ref comms) = proj.communications {
        Ok(Json(comms.clone()))
    } else {
        // Return defaults from communications manager
        let cm = host.communications_manager().read().map_err(|e| {
            ServerError::Internal(format!(
                "Failed to acquire communications manager lock: {e}"
            ))
        })?;
        Ok(Json(CommunicationsConfigDto {
            connections: cm.list_connections(),
            tag_mappings: cm.list_mappings(),
            zones: cm.get_zones(),
        }))
    }
}

/// Saves project-level communications configuration.
pub async fn save_project_communications(
    State(host): State<HostHandle>,
    role: UserRole,
    Path(id): Path<String>,
    Json(req): Json<CommunicationsConfigDto>,
) -> ServerResult<Json<CommunicationsConfigDto>> {
    if !role.can_modify_topology() {
        return Err(ServerError::Forbidden(
            "Engineer or Admin role required to configure project communications".to_string(),
        ));
    }

    let mut pm = host.project_manager().write().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire project manager lock: {e}"))
    })?;

    let mut proj = pm
        .get_project(&id)
        .ok_or_else(|| ServerError::NotFound(format!("Project '{id}' not found")))?;

    proj.communications = Some(req.clone());

    // Update in-memory communications manager as well
    if let Ok(mut cm) = host.communications_manager().write() {
        cm.load_project_communications(&Some(req.clone()));
    }

    let _ = pm
        .save_project(nebudcs_protocol::SaveProjectRequestDto { project: proj })
        .map_err(ServerError::InvalidRequest)?;

    Ok(Json(req))
}
