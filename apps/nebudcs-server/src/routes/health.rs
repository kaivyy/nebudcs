//! Health and status inspection routes.

use axum::extract::State;
use axum::Json;
use nebudcs_protocol::ServerHealthDto;

use crate::host::HostHandle;

/// Returns the current server and simulation engine health.
pub async fn get_health(State(host): State<HostHandle>) -> Json<ServerHealthDto> {
    let health = host.get_health().await;
    Json(health)
}
