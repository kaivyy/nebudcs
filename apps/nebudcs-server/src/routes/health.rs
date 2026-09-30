//! Health, liveness, readiness, and metrics inspection routes.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use nebudcs_protocol::ServerHealthDto;

use crate::host::HostHandle;
use crate::observability::{collect_metrics, ServerMetricsDto};

/// Returns the operational health report.
pub async fn get_health(State(host): State<HostHandle>) -> Json<ServerHealthDto> {
    let health = host.get_health().await;
    Json(health)
}

/// Lightweight liveness probe for Kubernetes, systemd, or monitoring agents.
/// Always returns HTTP 200 OK as long as the HTTP event loop is responding.
pub async fn get_live() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({ "status": "alive" })),
    )
}

/// Readiness probe confirming that the simulation engine and persistence are ready for traffic.
pub async fn get_ready(State(host): State<HostHandle>) -> impl IntoResponse {
    let health = host.get_health().await;
    // Check if simulation host is initialized
    if health.status == "healthy" || health.status == "running" || health.status == "idle" {
        (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "ready",
                "simulation": health.status,
                "tick_count": health.tick_count
            })),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "status": "degraded",
                "simulation": health.status
            })),
        )
    }
}

/// Returns detailed runtime performance and telemetry metrics.
pub async fn get_metrics(State(host): State<HostHandle>) -> Json<ServerMetricsDto> {
    let metrics = collect_metrics(&host).await;
    Json(metrics)
}
