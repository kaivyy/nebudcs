//! Hydraulic network diagnostics endpoint.

use axum::extract::State;
use axum::Json;
use nebudcs_physics::network::NetworkDiagnostics;

use crate::host::HostHandle;

/// Returns current hydraulic network diagnostics.
pub async fn get_network_diagnostics(State(host): State<HostHandle>) -> Json<NetworkDiagnostics> {
    Json(host.get_network_diagnostics().await)
}
