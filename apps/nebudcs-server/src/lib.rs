//! NebuDCS Realtime API and WebSocket Server library.

pub mod audit_manager;
pub mod auth;
pub mod backup;
pub mod cluster_manager;
pub mod communications_manager;
pub mod config;
pub mod diagnostics_manager;
pub mod error;
pub mod host;
pub mod nht_seed;
pub mod observability;
pub mod persistence;
pub mod project_manager;
pub mod routes;
pub mod scenario_manager;
pub mod security;
pub mod session;
pub mod training_manager;
pub mod validation_manager;

use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub use audit_manager::AuditManager;
pub use cluster_manager::ClusterManager;
pub use communications_manager::CommunicationsManager;
pub use config::ServerConfig;
pub use diagnostics_manager::DiagnosticsManager;
pub use error::{ServerError, ServerResult};
pub use host::{build_default_hal_system, HostHandle, SimulationHost};
pub use project_manager::ProjectManager;
pub use validation_manager::ValidationManager;

/// Builds the production-grade Axum router configured with middleware, CORS, and routes.
pub fn build_app(host: HostHandle) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    let max_body = host.config().max_http_payload_size;

    let app = routes::build_routes()
        .layer(axum::middleware::from_fn_with_state(
            host.clone(),
            auth::auth_middleware,
        ))
        .layer(axum::extract::DefaultBodyLimit::max(max_body))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(host.clone());

    if let Some(ref static_dir) = host.config().static_dir {
        let effective_path = if static_dir.exists() {
            Some(static_dir.clone())
        } else {
            let alt_path = std::path::PathBuf::from("../../").join(static_dir);
            if alt_path.exists() {
                Some(alt_path)
            } else {
                None
            }
        };

        if let Some(path) = effective_path {
            return app.fallback_service(tower_http::services::ServeDir::new(path));
        }
    }

    app
}
