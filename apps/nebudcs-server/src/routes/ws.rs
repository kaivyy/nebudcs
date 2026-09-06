//! WebSocket endpoint entry point.

use axum::extract::ws::WebSocketUpgrade;
use axum::extract::State;
use axum::response::Response;

use crate::host::HostHandle;
use crate::session::handle_ws_session;

/// Upgrades HTTP GET /ws request to full-duplex WebSocket connection.
pub async fn ws_upgrade_handler(ws: WebSocketUpgrade, State(host): State<HostHandle>) -> Response {
    ws.on_upgrade(move |socket| handle_ws_session(socket, host))
}
