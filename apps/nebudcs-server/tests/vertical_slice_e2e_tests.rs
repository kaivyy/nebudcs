//! End-to-End Vertical Slice Integration Tests for NebuDCS Phase 8.
//!
//! Validates the complete architectural chain:
//! Web UI Assets / REST / WebSocket <-> Simulation Host <-> Runtime Engine <-> State Streaming

use axum::body::Body;
use axum::http::{Request, StatusCode};
use futures_util::{SinkExt, StreamExt};
use std::path::PathBuf;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as TungMessage;
use tower::ServiceExt;

use nebudcs_alarm::block::AlarmBlockConfig;
use nebudcs_alarm::types::{AlarmSeverity, AlarmType};
use nebudcs_alarm::AlarmSystem;
use nebudcs_control::pid::{PidConfig, PidDirection};
use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_control::ControlSystem;
use nebudcs_model::tag::{TagQuality, TagValue};
use nebudcs_protocol::{
    ClientCapabilities, ClientMessage, RuntimeCommand, ServerMessage, SimControlCommand,
    TagValueDto, PROTOCOL_VERSION,
};
use nebudcs_server::{build_app, HostHandle, ServerConfig, SimulationHost};
use nebudcs_sim::SimulationEngine;
use nebudcs_types::{EquipmentId, TagId};

/// Creates a fully initialized vertical slice simulation host
fn create_vertical_slice_host(config: ServerConfig) -> (SimulationHost, HostHandle) {
    let engine = SimulationEngine::new(config.simulation_dt).expect("Failed to create engine");

    let mut tag_store = RuntimeTagStore::new();
    let ti101_id = TagId::new();
    let pi101_id = TagId::new();
    let ft101_id = TagId::new();
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
        ft101_id,
        TagValue::Float(4500.0),
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

    let mut ctrl_sys = ControlSystem::new();
    ctrl_sys.register_pid(
        lic101_pv_id,
        lic101_out_id,
        fv101_eq_id,
        PidConfig {
            kp: 1.5,
            ti: 20.0,
            td: 0.5,
            direction: PidDirection::Direct,
            out_min: 0.0,
            out_max: 100.0,
            ..Default::default()
        },
    );

    let host = SimulationHost::start(
        config,
        engine,
        tag_store,
        Some(alarm_sys),
        Some(ctrl_sys),
        None,
    )
    .expect("Failed to start host");

    let handle = host.handle().clone();
    (host, handle)
}

#[tokio::test]
async fn test_vertical_slice_static_web_delivery_and_auth_bypass() {
    let mut config = ServerConfig {
        auth_token: Some("super-secret-dcs-token".to_string()),
        ..Default::default()
    };
    // Point to web/dist
    config.static_dir = Some(PathBuf::from("../../web/dist"));

    let (_host, handle) = create_vertical_slice_host(config);
    let app = build_app(handle);

    // 1. GET / serves index.html without needing Authorization header
    let req = Request::builder()
        .method("GET")
        .uri("/")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let html = String::from_utf8_lossy(&body);
    assert!(
        html.contains("nebu-app"),
        "index.html must contain root <nebu-app> component"
    );

    // 2. GET /api/v1/sim/snapshot requires authentication
    let unauth_req = Request::builder()
        .method("GET")
        .uri("/api/v1/sim/snapshot")
        .body(Body::empty())
        .unwrap();
    let unauth_resp = app.clone().oneshot(unauth_req).await.unwrap();
    assert_eq!(unauth_resp.status(), StatusCode::UNAUTHORIZED);

    // 3. GET /api/v1/sim/snapshot with Bearer token succeeds
    let auth_req = Request::builder()
        .method("GET")
        .uri("/api/v1/sim/snapshot")
        .header("Authorization", "Bearer super-secret-dcs-token")
        .body(Body::empty())
        .unwrap();
    let auth_resp = app.oneshot(auth_req).await.unwrap();
    assert_eq!(auth_resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_vertical_slice_websocket_full_lifecycle() {
    let config = ServerConfig {
        simulation_dt: 0.05,
        speed_multiplier: 1.0,
        ..Default::default()
    };
    let (mut host, handle) = create_vertical_slice_host(config);
    let app = build_app(handle);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let ws_url = format!("ws://{addr}/ws");
    let (mut ws, _) = connect_async(&ws_url)
        .await
        .expect("WebSocket connection failed");

    // 1. Handshake
    let handshake = ClientMessage::Handshake {
        protocol_version: PROTOCOL_VERSION.to_string(),
        client_id: "vertical-slice-web-client".to_string(),
        capabilities: ClientCapabilities::default(),
    };
    ws.send(TungMessage::Text(
        serde_json::to_string(&handshake).unwrap().into(),
    ))
    .await
    .unwrap();

    // 2. Expect HandshakeAck
    let ack_msg = ws.next().await.unwrap().unwrap();
    let ack_server: ServerMessage = serde_json::from_str(&ack_msg.into_text().unwrap()).unwrap();
    match ack_server {
        ServerMessage::HandshakeAck {
            protocol_version,
            server_version,
            ..
        } => {
            assert_eq!(protocol_version, PROTOCOL_VERSION);
            assert_eq!(server_version, "0.1.0");
        }
        other => panic!("Expected HandshakeAck, got {other:?}"),
    }

    // 3. Expect Initial Snapshot with all vertical slice tags
    let snap_msg = ws.next().await.unwrap().unwrap();
    let snap_server: ServerMessage = serde_json::from_str(&snap_msg.into_text().unwrap()).unwrap();
    match snap_server {
        ServerMessage::Snapshot(snap) => {
            let tags: Vec<String> = snap.tags.iter().map(|t| t.tag.clone()).collect();
            assert!(tags.contains(&"LIC-101.PV".to_string()));
            assert!(tags.contains(&"LIC-101.SP".to_string()));
            assert!(tags.contains(&"LIC-101.OUT".to_string()));
            assert!(tags.contains(&"V-101.LEVEL".to_string()));
            assert!(tags.contains(&"FV-101.POS".to_string()));
            assert!(tags.contains(&"K-101.SPEED".to_string()));
            assert!(tags.contains(&"K-101.STATUS".to_string()));
            assert!(tags.contains(&"FT-101.FLOW".to_string()));
        }
        other => panic!("Expected Snapshot, got {other:?}"),
    }

    let mut collected = Vec::new();

    // 4. Send Play Command
    let play_cmd = ClientMessage::Command {
        command_id: "cmd-play-1".to_string(),
        command: RuntimeCommand::SimControl(SimControlCommand::Play),
    };
    ws.send(TungMessage::Text(
        serde_json::to_string(&play_cmd).unwrap().into(),
    ))
    .await
    .unwrap();
    assert!(wait_for_command_ack(&mut ws, "cmd-play-1", &mut collected).await);

    // 5. Setpoint modification via WebSocket
    let sp_cmd = ClientMessage::Command {
        command_id: "cmd-sp-1".to_string(),
        command: RuntimeCommand::SetSetpoint {
            loop_name: "LIC-101.OUT".to_string(),
            sp: 62.5,
        },
    };
    ws.send(TungMessage::Text(
        serde_json::to_string(&sp_cmd).unwrap().into(),
    ))
    .await
    .unwrap();
    assert!(wait_for_command_ack(&mut ws, "cmd-sp-1", &mut collected).await);

    // 6. Write high PV tag to trigger ALM_LIC101_HI (limit 75.0)
    let write_cmd = ClientMessage::Command {
        command_id: "cmd-write-1".to_string(),
        command: RuntimeCommand::WriteTag {
            tag: "LIC-101.PV".to_string(),
            value: TagValueDto::Float(82.0),
        },
    };
    ws.send(TungMessage::Text(
        serde_json::to_string(&write_cmd).unwrap().into(),
    ))
    .await
    .unwrap();
    assert!(wait_for_command_ack(&mut ws, "cmd-write-1", &mut collected).await);

    // 7. Step simulation and verify streaming delta receives updated PV
    let step_cmd = ClientMessage::Command {
        command_id: "cmd-step-1".to_string(),
        command: RuntimeCommand::SimControl(SimControlCommand::Step),
    };
    ws.send(TungMessage::Text(
        serde_json::to_string(&step_cmd).unwrap().into(),
    ))
    .await
    .unwrap();
    assert!(wait_for_command_ack(&mut ws, "cmd-step-1", &mut collected).await);

    // Verify high PV was captured in streaming deltas
    let mut found_high_pv = false;
    for msg in &collected {
        if let ServerMessage::Delta(delta) = msg {
            for tag_sample in &delta.changed_tags {
                if tag_sample.tag == "LIC-101.PV" {
                    if let TagValueDto::Float(val) = tag_sample.value {
                        if (val - 82.0).abs() < 1e-4 {
                            found_high_pv = true;
                            break;
                        }
                    }
                }
            }
        }
    }
    assert!(
        found_high_pv,
        "Client must receive streaming delta for updated tag LIC-101.PV"
    );

    // 8. Acknowledge alarm
    let ack_alm_cmd = ClientMessage::Command {
        command_id: "cmd-ack-alm-1".to_string(),
        command: RuntimeCommand::AcknowledgeAlarm {
            alarm_name: "ALM_LIC101_HI".to_string(),
        },
    };
    ws.send(TungMessage::Text(
        serde_json::to_string(&ack_alm_cmd).unwrap().into(),
    ))
    .await
    .unwrap();
    assert!(wait_for_command_ack(&mut ws, "cmd-ack-alm-1", &mut collected).await);

    // 9. Pause and clean shutdown
    let pause_cmd = ClientMessage::Command {
        command_id: "cmd-pause-1".to_string(),
        command: RuntimeCommand::SimControl(SimControlCommand::Pause),
    };
    ws.send(TungMessage::Text(
        serde_json::to_string(&pause_cmd).unwrap().into(),
    ))
    .await
    .unwrap();
    assert!(wait_for_command_ack(&mut ws, "cmd-pause-1", &mut collected).await);

    server_task.abort();
    host.stop();
}

async fn wait_for_command_ack(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    expected_cmd_id: &str,
    collected: &mut Vec<ServerMessage>,
) -> bool {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(3);
    while tokio::time::Instant::now() < deadline {
        if let Ok(Some(Ok(msg))) =
            tokio::time::timeout(std::time::Duration::from_millis(300), ws.next()).await
        {
            if let Ok(text) = msg.into_text() {
                if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                    let is_target = match &server_msg {
                        ServerMessage::CommandAck {
                            command_id,
                            success,
                            ..
                        } => {
                            if command_id == expected_cmd_id {
                                Some(*success)
                            } else {
                                None
                            }
                        }
                        _ => None,
                    };
                    collected.push(server_msg);
                    if let Some(success) = is_target {
                        return success;
                    }
                }
            }
        }
    }
    panic!("Timed out waiting for CommandAck for {expected_cmd_id}");
}

#[tokio::test]
async fn test_vertical_slice_rest_api_consistency() {
    use nebudcs_protocol::dto::{StateSnapshotDto, TagValueSampleDto};

    let config = ServerConfig::default();
    let (mut host, handle) = create_vertical_slice_host(config);
    let app = build_app(handle);

    // 1. GET /api/v1/sim/snapshot
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/sim/snapshot")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let snap: StateSnapshotDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(snap.tags.len(), 10);

    // 2. GET /api/v1/alarms
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/alarms")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let alarm_list: nebudcs_server::routes::alarms::AlarmListResponse =
        serde_json::from_slice(&body).unwrap();
    assert_eq!(alarm_list.alarms.len(), 2);

    // 3. POST /api/v1/control/pid/setpoint
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/control/pid/setpoint")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"loop_name":"LIC-101.OUT","sp":58.5}"#))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 4. POST /api/v1/tags/write
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/tags/write")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"tag":"LIC-101.PV","value":52.0}"#))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 5. GET /api/v1/tags/LIC-101.PV
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/tags/LIC-101.PV")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let sample: TagValueSampleDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(sample.tag, "LIC-101.PV");
    assert_eq!(sample.value, TagValueDto::Float(52.0));

    host.stop();
}
