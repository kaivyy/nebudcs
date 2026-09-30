//! Binary entrypoint for the NebuDCS realtime server.
//!
//! Runtime state (tags, alarms, PID controllers) is project-driven: tags are
//! seeded by the simulation engine when a project is loaded via
//! `POST /api/v1/projects/:id/load-sim`.  The server starts with an empty
//! tag store so that no project-specific identifiers are hard-wired here.

use std::net::SocketAddr;
use tokio::signal;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use nebudcs_alarm::AlarmSystem;
use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_control::ControlSystem;
use nebudcs_server::{build_app, ServerConfig, SimulationHost};
use nebudcs_sim::SimulationEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,nebudcs_server=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting NebuDCS Realtime Server (Phase 14 — generic model-driven runtime)...");

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let bind_addr: SocketAddr = format!("0.0.0.0:{port}").parse()?;

    let mut config = ServerConfig {
        bind_addr,
        ..Default::default()
    };

    if let Ok(token) = std::env::var("AUTH_TOKEN") {
        if !token.is_empty() {
            tracing::info!("Server authentication enabled (legacy shared token)");
            config.auth_token = Some(token);
        }
    }
    if let Ok(token) = std::env::var("ADMIN_TOKEN") {
        if !token.is_empty() {
            config.admin_token = Some(token);
        }
    }
    if let Ok(token) = std::env::var("INSTRUCTOR_TOKEN") {
        if !token.is_empty() {
            config.instructor_token = Some(token);
        }
    }
    if let Ok(token) = std::env::var("OPERATOR_TOKEN") {
        if !token.is_empty() {
            config.operator_token = Some(token);
        }
    }
    if let Ok(token) = std::env::var("TRAINEE_TOKEN") {
        if !token.is_empty() {
            config.trainee_token = Some(token);
        }
    }
    if let Ok(dir) = std::env::var("DATA_DIR") {
        if !dir.is_empty() {
            config.data_dir = std::path::PathBuf::from(dir);
        }
    }
    if let Ok(dir) = std::env::var("STATIC_DIR") {
        if !dir.is_empty() {
            config.static_dir = Some(std::path::PathBuf::from(dir));
        }
    }
    if let Ok(burst) = std::env::var("RATE_LIMIT_BURST") {
        if let Ok(b) = burst.parse::<u32>() {
            config.rate_limit_burst = b;
        }
    }
    if let Ok(rps) = std::env::var("RATE_LIMIT_RPS") {
        if let Ok(r) = rps.parse::<f64>() {
            config.rate_limit_rps = r;
        }
    }

    // Initialize simulation engine with an empty runtime store.
    // Tags, alarms, and PID controllers are registered when the operator loads
    // a project via the Engineering Studio (POST /api/v1/projects/:id/load-sim).
    // The baseline demonstration project (K-101 / FV-101 / V-101) exists only
    // as project model data in ProjectManager — it is not auto-activated here.
    let engine = SimulationEngine::new(config.simulation_dt)?;
    let tag_store = RuntimeTagStore::new();
    let alarm_sys = AlarmSystem::new();
    let ctrl_sys = ControlSystem::new();

    // Start simulation host with clean runtime state
    let mut host = SimulationHost::start(
        config.clone(),
        engine,
        tag_store,
        Some(alarm_sys),
        Some(ctrl_sys),
        None,
    )?;

    let app = build_app(host.handle().clone());
    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    tracing::info!("Server listening on http://{}", listener.local_addr()?);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Server shutting down simulation host...");
    host.stop();

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
