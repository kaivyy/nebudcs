//! Administrative endpoints for backup, restore, schema migration, and operational controls.

use axum::extract::State;
use axum::Json;

use crate::backup::migration::{run_migration, MigrationReportDto};
use crate::backup::{create_backup, restore_backup, BackupBundle, RestoreResultDto};
use crate::error::{ServerError, ServerResult};
use crate::host::HostHandle;
use crate::security::{audit_log, UserRole};

/// Creates a cryptographically signed backup bundle of all projects, scenarios, and training records.
pub async fn post_backup(
    State(host): State<HostHandle>,
    role: UserRole,
) -> ServerResult<Json<BackupBundle>> {
    if !role.can_administer() {
        audit_log(
            "ADMIN_BACKUP",
            &format!("{role}"),
            "/api/v1/admin/backup",
            "Non-admin attempted backup",
            false,
        );
        return Err(ServerError::Forbidden(
            "Administrator privileges required for backup".to_string(),
        ));
    }

    audit_log(
        "ADMIN_BACKUP",
        &format!("{role}"),
        "/api/v1/admin/backup",
        "Initiating platform backup",
        true,
    );

    let bundle = create_backup(host.disk_store())
        .map_err(|e| ServerError::Internal(format!("Backup generation failed: {e}")))?;

    Ok(Json(bundle))
}

/// Restores a backup bundle after validating cryptographic checksums and reloads in-memory managers.
pub async fn post_restore(
    State(host): State<HostHandle>,
    role: UserRole,
    Json(bundle): Json<BackupBundle>,
) -> ServerResult<Json<RestoreResultDto>> {
    if !role.can_administer() {
        audit_log(
            "ADMIN_RESTORE",
            &format!("{role}"),
            "/api/v1/admin/restore",
            "Non-admin attempted restore",
            false,
        );
        return Err(ServerError::Forbidden(
            "Administrator privileges required for restore".to_string(),
        ));
    }

    audit_log(
        "ADMIN_RESTORE",
        &format!("{role}"),
        "/api/v1/admin/restore",
        "Initiating platform restore",
        true,
    );

    let result = restore_backup(host.disk_store(), &bundle, true)
        .map_err(|e| ServerError::InvalidRequest(format!("Restore rejected: {e}")))?;

    // Reload state in running managers
    if let Ok(mut mgr) = host.project_manager().write() {
        let _ = mgr.reload_from_disk();
    }
    if let Ok(mut mgr) = host.scenario_manager().write() {
        let _ = mgr.reload_from_disk();
    }
    if let Ok(mut mgr) = host.training_manager().write() {
        let _ = mgr.reload_from_disk();
    }

    Ok(Json(result))
}

/// Validates a backup bundle's cryptographic signatures and schemas without applying it to disk (dry-run).
pub async fn post_validate_backup(
    State(_host): State<HostHandle>,
    role: UserRole,
    Json(bundle): Json<BackupBundle>,
) -> ServerResult<Json<crate::backup::RestoreValidationDto>> {
    if !role.can_administer() && role != UserRole::Engineer {
        return Err(ServerError::Forbidden(
            "Administrator or Engineer privileges required to validate backup bundle".to_string(),
        ));
    }

    let report = crate::backup::validate_bundle(&bundle)
        .map_err(|e| ServerError::InvalidRequest(format!("Backup validation failed: {e}")))?;

    Ok(Json(report))
}

/// Runs schema migration on disk and synchronizes active in-memory models.
pub async fn post_migrate(
    State(host): State<HostHandle>,
    role: UserRole,
) -> ServerResult<Json<MigrationReportDto>> {
    if !role.can_administer() {
        audit_log(
            "ADMIN_MIGRATE",
            &format!("{role}"),
            "/api/v1/admin/migrate",
            "Non-admin attempted schema migration",
            false,
        );
        return Err(ServerError::Forbidden(
            "Administrator privileges required for schema migration".to_string(),
        ));
    }

    audit_log(
        "ADMIN_MIGRATE",
        &format!("{role}"),
        "/api/v1/admin/migrate",
        "Initiating schema migration",
        true,
    );

    let report = run_migration(host.disk_store())
        .map_err(|e| ServerError::Internal(format!("Migration failed: {e}")))?;

    // Synchronize managers
    if let Ok(mut mgr) = host.project_manager().write() {
        let _ = mgr.reload_from_disk();
    }
    if let Ok(mut mgr) = host.scenario_manager().write() {
        let _ = mgr.reload_from_disk();
    }
    if let Ok(mut mgr) = host.training_manager().write() {
        let _ = mgr.reload_from_disk();
    }

    Ok(Json(report))
}

/// Reloads all managers directly from persistent storage.
pub async fn post_reload(
    State(host): State<HostHandle>,
    role: UserRole,
) -> ServerResult<Json<serde_json::Value>> {
    if !role.can_administer() {
        return Err(ServerError::Forbidden(
            "Administrator privileges required to reload models".to_string(),
        ));
    }

    let projects_count = host
        .project_manager()
        .write()
        .map_err(|e| ServerError::Internal(e.to_string()))?
        .reload_from_disk()
        .map_err(ServerError::Internal)?;

    let scenarios_count = host
        .scenario_manager()
        .write()
        .map_err(|e| ServerError::Internal(e.to_string()))?
        .reload_from_disk()
        .map_err(ServerError::Internal)?;

    let training_count = host
        .training_manager()
        .write()
        .map_err(|e| ServerError::Internal(e.to_string()))?
        .reload_from_disk()
        .map_err(ServerError::Internal)?;

    Ok(Json(serde_json::json!({
        "status": "reloaded",
        "projects_count": projects_count,
        "scenarios_count": scenarios_count,
        "training_count": training_count,
    })))
}
