//! Binary entrypoint for the NebuDCS realtime server.

use std::net::SocketAddr;
use tokio::signal;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use nebudcs_alarm::block::AlarmBlockConfig;
use nebudcs_alarm::types::{AlarmSeverity, AlarmType};
use nebudcs_alarm::AlarmSystem;
use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_control::ControlSystem;
use nebudcs_model::tag::{TagQuality, TagValue};
use nebudcs_server::{build_app, ServerConfig, SimulationHost};
use nebudcs_sim::SimulationEngine;
use nebudcs_types::{EquipmentId, TagId};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,nebudcs_server=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting NebuDCS Realtime Server (Phase 7)...");

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
            tracing::info!("Server authentication enabled");
            config.auth_token = Some(token);
        }
    }

    // Initialize simulation engine baseline
    let engine = SimulationEngine::new(config.simulation_dt)?;

    // Initialize baseline demonstration tags
    let mut tag_store = RuntimeTagStore::new();
    let ti101_id = TagId::new();
    let pi101_id = TagId::new();
    let ft101_flow_id = TagId::new();
    let fic101_pv_id = TagId::new();
    let fic101_sp_id = TagId::new();
    let fic101_out_id = TagId::new();
    let lic101_pv_id = TagId::new();
    let lic101_sp_id = TagId::new();
    let lic101_out_id = TagId::new();
    let v101_level_id = TagId::new();
    let fv101_pos_id = TagId::new();
    let k101_speed_id = TagId::new();
    let k101_status_id = TagId::new();

    let fv101_eq_id = EquipmentId::new();

    tag_store.register_tag("TI-101", ti101_id, TagValue::Float(185.4), TagQuality::Good);
    tag_store.register_tag("PI-101", pi101_id, TagValue::Float(32.5), TagQuality::Good);
    tag_store.register_tag(
        "FT-101.FLOW",
        ft101_flow_id,
        TagValue::Float(4500.0),
        TagQuality::Good,
    );
    tag_store.register_tag(
        "FIC-101.PV",
        fic101_pv_id,
        TagValue::Float(4500.0),
        TagQuality::Good,
    );
    tag_store.register_tag(
        "FIC-101.SP",
        fic101_sp_id,
        TagValue::Float(4500.0),
        TagQuality::Good,
    );
    tag_store.register_tag(
        "FIC-101.OUT",
        fic101_out_id,
        TagValue::Float(55.2),
        TagQuality::Good,
    );
    tag_store.register_tag(
        "LIC-101.PV",
        lic101_pv_id,
        TagValue::Float(50.0),
        TagQuality::Good,
    );
    tag_store.register_tag(
        "LIC-101.SP",
        lic101_sp_id,
        TagValue::Float(50.0),
        TagQuality::Good,
    );
    tag_store.register_tag(
        "LIC-101.OUT",
        lic101_out_id,
        TagValue::Float(55.2),
        TagQuality::Good,
    );
    tag_store.register_tag(
        "V-101.LEVEL",
        v101_level_id,
        TagValue::Float(50.0),
        TagQuality::Good,
    );
    tag_store.register_tag(
        "FV-101.POS",
        fv101_pos_id,
        TagValue::Float(55.2),
        TagQuality::Good,
    );
    tag_store.register_tag(
        "K-101.SPEED",
        k101_speed_id,
        TagValue::Float(2950.0),
        TagQuality::Good,
    );
    tag_store.register_tag(
        "K-101.STATUS",
        k101_status_id,
        TagValue::Integer(1),
        TagQuality::Good,
    );

    // Initialize alarm and control systems
    let mut alarm_sys = AlarmSystem::new();
    alarm_sys.add_alarm(AlarmBlockConfig {
        name: "ALM_PI101_HI".to_string(),
        tag_id: pi101_id,
        alarm_type: AlarmType::Hi,
        severity: AlarmSeverity::Warning,
        limit: 35.0,
        deadband: 0.5,
        default_shelve_duration: 60.0,
    });
    alarm_sys.add_alarm(AlarmBlockConfig {
        name: "ALM_LIC101_HI".to_string(),
        tag_id: lic101_pv_id,
        alarm_type: AlarmType::Hi,
        severity: AlarmSeverity::Warning,
        limit: 75.0,
        deadband: 1.0,
        default_shelve_duration: 120.0,
    });
    alarm_sys.add_alarm(AlarmBlockConfig {
        name: "ALM_LIC101_HIHI".to_string(),
        tag_id: lic101_pv_id,
        alarm_type: AlarmType::Hi,
        severity: AlarmSeverity::Critical,
        limit: 85.0,
        deadband: 1.0,
        default_shelve_duration: 120.0,
    });

    let mut ctrl_sys = ControlSystem::new();
    ctrl_sys.register_pid(
        lic101_pv_id,
        lic101_out_id,
        fv101_eq_id,
        nebudcs_control::pid::PidConfig {
            kp: 1.5,
            ti: 20.0,
            td: 0.5,
            ..Default::default()
        },
    );
    ctrl_sys.register_pid(
        fic101_pv_id,
        fic101_out_id,
        fv101_eq_id,
        nebudcs_control::pid::PidConfig {
            kp: 0.8,
            ti: 10.0,
            td: 0.0,
            ..Default::default()
        },
    );

    // Start simulation host
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
