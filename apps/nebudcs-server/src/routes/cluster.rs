//! Cluster & High Availability (HA) REST routes.

use axum::extract::State;
use axum::Json;

use crate::error::{ServerError, ServerResult};
use crate::host::HostHandle;
use crate::security::UserRole;
use nebudcs_protocol::{
    ClusterTopologyDto, FailoverRequestDto, FailureInjectionRequestDto,
    FailureInjectionStatusDto, HaHeartbeatMessage, PromotionResultDto, ReplicationSnapshotDto,
};

/// Returns the complete cluster topology, leader lease, and HA node health.
pub async fn get_topology(
    State(host): State<HostHandle>,
    _role: UserRole,
) -> ServerResult<Json<ClusterTopologyDto>> {
    let cm = host
        .cluster_manager()
        .read()
        .map_err(|e| ServerError::Internal(e.to_string()))?;
    Ok(Json(cm.get_topology()))
}

/// Receives an inbound heartbeat from a cluster peer node.
pub async fn post_heartbeat(
    State(host): State<HostHandle>,
    _role: UserRole,
    Json(hb): Json<HaHeartbeatMessage>,
) -> ServerResult<Json<serde_json::Value>> {
    let mut cm = host
        .cluster_manager()
        .write()
        .map_err(|e| ServerError::Internal(e.to_string()))?;

    cm.handle_heartbeat(hb)
        .map_err(|e| ServerError::InvalidRequest(e))?;

    Ok(Json(serde_json::json!({
        "status": "acknowledged",
        "local_epoch": cm.leader_epoch(),
        "local_ha_state": cm.local_ha_state(),
    })))
}

/// Triggers a controlled failover or node promotion (Admin / Engineer only).
pub async fn post_failover(
    State(host): State<HostHandle>,
    role: UserRole,
    Json(req): Json<FailoverRequestDto>,
) -> ServerResult<Json<PromotionResultDto>> {
    if !role.can_administer() && role != UserRole::Engineer {
        return Err(ServerError::Forbidden(
            "Administrator or Engineer privileges required to trigger cluster failover".to_string(),
        ));
    }

    let mut cm = host
        .cluster_manager()
        .write()
        .map_err(|e| ServerError::Internal(e.to_string()))?;

    let res = cm
        .request_failover(req)
        .map_err(|e| ServerError::InvalidRequest(e))?;

    // Record audit event
    if let Ok(mut audit) = host.audit_manager().write() {
        audit.record(
            "HA-FAILOVER",
            &format!("{role}"),
            &format!("{role}"),
            "ClusterAPI",
            &res.promoted_node_id,
            "FAILOVER_PROMOTION",
            res.previous_leader_id.clone(),
            Some(res.promoted_node_id.clone()),
            "SUCCESS",
            Some(res.message.clone()),
            None,
            0.0,
        );
    }

    Ok(Json(res))
}

/// Injects controlled infrastructure failure for HA verification (Admin / Engineer only).
pub async fn post_inject_failure(
    State(host): State<HostHandle>,
    role: UserRole,
    Json(req): Json<FailureInjectionRequestDto>,
) -> ServerResult<Json<FailureInjectionStatusDto>> {
    if !role.can_administer() && role != UserRole::Engineer {
        return Err(ServerError::Forbidden(
            "Administrator or Engineer privileges required for failure injection".to_string(),
        ));
    }

    let mut cm = host
        .cluster_manager()
        .write()
        .map_err(|e| ServerError::Internal(e.to_string()))?;

    let status = cm
        .inject_failure(req)
        .map_err(|e| ServerError::InvalidRequest(e))?;

    // Record audit event
    if let Ok(mut audit) = host.audit_manager().write() {
        audit.record(
            "HA-INJECT",
            &format!("{role}"),
            &format!("{role}"),
            "ClusterAPI",
            status.target_node_id.as_deref().unwrap_or("Cluster"),
            "INJECT_FAILURE",
            None,
            status.failure_type.clone(),
            "SUCCESS",
            Some(status.message.clone()),
            None,
            0.0,
        );
    }

    Ok(Json(status))
}

/// Clears any active failure injection (Admin / Engineer only).
pub async fn post_clear_failure(
    State(host): State<HostHandle>,
    role: UserRole,
) -> ServerResult<Json<FailureInjectionStatusDto>> {
    if !role.can_administer() && role != UserRole::Engineer {
        return Err(ServerError::Forbidden(
            "Administrator or Engineer privileges required to clear failure injection".to_string(),
        ));
    }

    let mut cm = host
        .cluster_manager()
        .write()
        .map_err(|e| ServerError::Internal(e.to_string()))?;

    let status = cm.clear_failure_injection();

    // Record audit event
    if let Ok(mut audit) = host.audit_manager().write() {
        audit.record(
            "HA-CLEAR",
            &format!("{role}"),
            &format!("{role}"),
            "ClusterAPI",
            "Cluster",
            "CLEAR_FAILURE_INJECTION",
            None,
            None,
            "SUCCESS",
            Some(status.message.clone()),
            None,
            0.0,
        );
    }

    Ok(Json(status))
}

/// Returns the current failure injection status.
pub async fn get_failure_status(
    State(host): State<HostHandle>,
    _role: UserRole,
) -> ServerResult<Json<FailureInjectionStatusDto>> {
    let cm = host
        .cluster_manager()
        .read()
        .map_err(|e| ServerError::Internal(e.to_string()))?;
    Ok(Json(cm.failure_injection_status()))
}

/// Generates an authoritative replication snapshot from the primary.
pub async fn post_replication_snapshot(
    State(host): State<HostHandle>,
    role: UserRole,
) -> ServerResult<Json<ReplicationSnapshotDto>> {
    if !role.can_administer() && role != UserRole::Engineer && role != UserRole::Operator {
        return Err(ServerError::Forbidden(
            "Insufficient privileges to request replication snapshot".to_string(),
        ));
    }

    let health = host.get_health().await;
    let mut cm = host
        .cluster_manager()
        .write()
        .map_err(|e| ServerError::Internal(e.to_string()))?;

    let snap = cm.create_replication_snapshot(
        health.sim_time_sec,
        health.tick_count,
        150,
        0,
        false,
        health.tick_count * 10,
    );

    Ok(Json(snap))
}

/// Applies an authoritative replication snapshot on a standby node.
pub async fn post_apply_replication(
    State(host): State<HostHandle>,
    role: UserRole,
    Json(snap): Json<ReplicationSnapshotDto>,
) -> ServerResult<Json<serde_json::Value>> {
    if !role.can_administer() && role != UserRole::Engineer {
        return Err(ServerError::Forbidden(
            "Insufficient privileges to apply replication snapshot".to_string(),
        ));
    }

    let mut cm = host
        .cluster_manager()
        .write()
        .map_err(|e| ServerError::Internal(e.to_string()))?;

    cm.apply_replication_snapshot(snap)
        .map_err(|e| ServerError::InvalidRequest(e))?;

    Ok(Json(serde_json::json!({
        "status": "applied",
        "local_ha_state": cm.local_ha_state(),
        "leader_epoch": cm.leader_epoch(),
    })))
}
