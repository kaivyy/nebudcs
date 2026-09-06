//! Configuration parameters for the NebuDCS realtime server.

use std::net::SocketAddr;

/// Server configuration parameters.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Network bind address for HTTP and WebSocket listeners.
    pub bind_addr: SocketAddr,
    /// Optional shared secret token for API and WebSocket authentication.
    pub auth_token: Option<String>,
    /// Allowed CORS origins (e.g. `["http://localhost:3000"]`, or `["*"]`).
    pub cors_allowed_origins: Vec<String>,
    /// Maximum allowed WebSocket inbound frame size in bytes.
    pub max_ws_message_size: usize,
    /// Maximum allowed HTTP JSON payload size in bytes.
    pub max_http_payload_size: usize,
    /// Maximum concurrent WebSocket client connections.
    pub max_connections: usize,
    /// Simulation timestep $\Delta t$ in seconds.
    pub simulation_dt: f64,
    /// Simulation execution pacing multiplier.
    pub speed_multiplier: f64,
    /// Deadband threshold for delta updates (fractional or absolute).
    pub delta_deadband: f64,
    /// Capacity of the internal broadcast channel.
    pub broadcast_capacity: usize,
    /// Per-client WebSocket outbound buffer capacity.
    pub client_queue_capacity: usize,
    /// Optional directory to serve static frontend assets from (e.g. `web/dist`).
    pub static_dir: Option<std::path::PathBuf>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:8080".parse().unwrap(),
            auth_token: None,
            cors_allowed_origins: vec!["*".to_string()],
            max_ws_message_size: 64 * 1024,     // 64 KB
            max_http_payload_size: 1024 * 1024, // 1 MB
            max_connections: 1024,
            simulation_dt: 0.05,
            speed_multiplier: 1.0,
            delta_deadband: 1e-4,
            broadcast_capacity: 1024,
            client_queue_capacity: 256,
            static_dir: Some(std::path::PathBuf::from("web/dist")),
        }
    }
}
