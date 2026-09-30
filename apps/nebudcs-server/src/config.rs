//! Configuration parameters for the NebuDCS realtime server.

use std::net::SocketAddr;
use std::path::PathBuf;

/// Server configuration parameters.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Network bind address for HTTP and WebSocket listeners.
    pub bind_addr: SocketAddr,
    /// Optional shared secret token for API and WebSocket authentication (maps to Admin).
    pub auth_token: Option<String>,
    /// Optional dedicated token for Admin role.
    pub admin_token: Option<String>,
    /// Optional dedicated token for Instructor role.
    pub instructor_token: Option<String>,
    /// Optional dedicated token for Operator role.
    pub operator_token: Option<String>,
    /// Optional dedicated token for Trainee role.
    pub trainee_token: Option<String>,
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
    pub static_dir: Option<PathBuf>,
    /// Base directory for persistent data (projects, scenarios, training records).
    pub data_dir: PathBuf,
    /// Whether rate limiting is enabled.
    pub rate_limit_enabled: bool,
    /// Burst capacity for rate limiting.
    pub rate_limit_burst: u32,
    /// Replenishment rate per second for rate limiting.
    pub rate_limit_rps: f64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:8080".parse().unwrap(),
            auth_token: None,
            admin_token: None,
            instructor_token: None,
            operator_token: None,
            trainee_token: None,
            cors_allowed_origins: vec!["*".to_string()],
            max_ws_message_size: 64 * 1024,          // 64 KB
            max_http_payload_size: 50 * 1024 * 1024, // 50 MB
            max_connections: 1024,
            simulation_dt: 0.05,
            speed_multiplier: 1.0,
            delta_deadband: 1e-4,
            broadcast_capacity: 1024,
            client_queue_capacity: 256,
            static_dir: Some(PathBuf::from("web/dist")),
            data_dir: PathBuf::from("data"),
            rate_limit_enabled: true,
            rate_limit_burst: 200,
            rate_limit_rps: 100.0,
        }
    }
}
