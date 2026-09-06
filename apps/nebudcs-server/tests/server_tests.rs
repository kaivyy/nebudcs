//! End-to-end integration tests for NebuDCS Realtime API and WebSocket Server (Phase 7).

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message as TungMessage;
use tower::ServiceExt;

use nebudcs_alarm::block::AlarmBlockConfig;
use nebudcs_alarm::types::{AlarmSeverity, AlarmType};
use nebudcs_alarm::AlarmSystem;
use nebudcs_control::pid::{PidConfig, PidDirection};
use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_control::ControlSystem;
use nebudcs_historian::point::{HistorianPointConfig, HistorianPointId, SamplingMode};
use nebudcs_historian::run::HistorianRunMeta;
use nebudcs_historian::sqlite::SqliteStorage;
use nebudcs_historian::{
    HistorianRecord, HistoricalEvent, HistoricalSample, SqliteHistorianReader,
};
use nebudcs_model::tag::{TagQuality, TagValue};
use nebudcs_protocol::*;
use nebudcs_server::{build_app, HostHandle, ServerConfig, SimulationHost};
use nebudcs_sim::SimulationEngine;
use nebudcs_types::{EquipmentId, ProjectId, SimulationTime, TagId};

/// Helper to create a test simulation host and handle.
fn create_test_host(config: ServerConfig) -> (SimulationHost, HostHandle) {
    let engine = SimulationEngine::new(config.simulation_dt).expect("Failed to create engine");

    let mut tag_store = RuntimeTagStore::new();
    let ti_id = TagId::new();
    let pi_id = TagId::new();
    let fic_pv_id = TagId::new();
    let fic_sp_id = TagId::new();
    let fic_out_id = TagId::new();

    tag_store.register_tag("TI-101", ti_id, TagValue::Float(185.0), TagQuality::Good);
    tag_store.register_tag("PI-101", pi_id, TagValue::Float(30.0), TagQuality::Good);
    tag_store.register_tag(
        "FIC-101.PV",
        fic_pv_id,
        TagValue::Float(4500.0),
        TagQuality::Good,
    );
    tag_store.register_tag(
        "FIC-101.SP",
        fic_sp_id,
        TagValue::Float(4500.0),
        TagQuality::Good,
    );
    tag_store.register_tag(
        "FIC-101.OUT",
        fic_out_id,
        TagValue::Float(50.0),
        TagQuality::Good,
    );

    let mut alarm_sys = AlarmSystem::new();
    alarm_sys.add_alarm(AlarmBlockConfig {
        name: "ALM_PI101_HI".to_string(),
        tag_id: pi_id,
        alarm_type: AlarmType::Hi,
        severity: AlarmSeverity::Warning,
        limit: 35.0,
        deadband: 0.5,
        default_shelve_duration: 60.0,
    });

    let mut ctrl_sys = ControlSystem::new();
    let pid_cfg = PidConfig {
        kp: 1.2,
        ti: 20.0,
        td: 0.0,
        derivative_filter_n: 10.0,
        out_min: 0.0,
        out_max: 100.0,
        direction: PidDirection::Direct,
        sp_tracking_in_manual: true,
        scan_divisor: 1,
    };
    let eq_id = EquipmentId::new();
    ctrl_sys.register_pid(fic_pv_id, fic_out_id, eq_id, pid_cfg);

    let host = SimulationHost::start(
        config.clone(),
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
async fn test_server_health_endpoint() {
    let config = ServerConfig::default();
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle);

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/health")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let health: ServerHealthDto = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(health.status, "running");
    assert_eq!(health.protocol_version, PROTOCOL_VERSION);
    assert!(!health.simulation_running);
    assert_eq!(health.connected_clients, 0);
}

#[tokio::test]
async fn test_sim_snapshot_and_tag_inspection() {
    let config = ServerConfig::default();
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle);

    // 1. Snapshot
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/sim/snapshot")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let snap: StateSnapshotDto = serde_json::from_slice(&body_bytes).unwrap();
    assert!(snap.tags.iter().any(|t| t.tag == "TI-101"));
    assert!(snap.tags.iter().any(|t| t.tag == "PI-101"));

    // 2. List Tags
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/tags")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. Single Tag
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/tags/TI-101")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let tag_sample: TagValueSampleDto = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(tag_sample.tag, "TI-101");
    assert_eq!(tag_sample.value, TagValueDto::Float(185.0));

    // 4. Unknown Tag -> 404
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/tags/NON_EXISTENT")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_deterministic_simulation_control_and_tag_mutation() {
    let config = ServerConfig::default();
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle);

    // 1. Play
    let play_cmd = SimControlCommand::Play;
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/sim/control")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&play_cmd).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify health running = true
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/health")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let health: ServerHealthDto = serde_json::from_slice(&body_bytes).unwrap();
    assert!(health.simulation_running);

    // 2. Pause
    let pause_cmd = SimControlCommand::Pause;
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/sim/control")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&pause_cmd).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. Step
    let step_cmd = SimControlCommand::Step;
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/sim/control")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&step_cmd).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 4. Tag Write Mutation
    let write_payload = json!({
        "tag": "TI-101",
        "value": 240.5
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/tags/write")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&write_payload).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify mutated value reflects
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/tags/TI-101")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let tag_sample: TagValueSampleDto = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(tag_sample.value, TagValueDto::Float(240.5));
}

#[tokio::test]
async fn test_authentication_enforcement() {
    let config = ServerConfig {
        auth_token: Some("secure-token-xyz".to_string()),
        ..Default::default()
    };
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle);

    // 1. Health is open without token
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/health")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 2. Protected endpoint without token -> 401
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/tags")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // 3. Protected endpoint with invalid Bearer token -> 401
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/tags")
        .header(header::AUTHORIZATION, "Bearer wrong-token")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // 4. Protected endpoint with valid Bearer token -> 200 OK
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/tags")
        .header(header::AUTHORIZATION, "Bearer secure-token-xyz")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 5. Protected endpoint with valid query token -> 200 OK
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/tags?token=secure-token-xyz")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_alarm_and_pid_endpoints() {
    let config = ServerConfig::default();
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle);

    // 1. Acknowledge alarm
    let ack_req = json!({
        "alarm_name": "ALM_PI101_HI"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/alarms/acknowledge")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&ack_req).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 2. Shelve alarm
    let shelve_req = json!({
        "alarm_name": "ALM_PI101_HI",
        "duration_seconds": 45.0
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/alarms/shelve")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&shelve_req).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. PID Setpoint
    let sp_req = json!({
        "loop_name": "FIC-101.OUT",
        "sp": 4800.0
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/control/pid/setpoint")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&sp_req).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 4. PID Mode
    let mode_req = json!({
        "loop_name": "FIC-101.OUT",
        "mode": "MANUAL"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/control/pid/mode")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&mode_req).unwrap()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_historian_reader_query_endpoints() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("historian_api_test.sqlite");

    let tag_id = TagId::new();
    let pt_id = HistorianPointId::new(0);

    let run_meta = HistorianRunMeta::new(ProjectId::new(), "1.0.0", 0.05, 1.0);
    let run_id = run_meta.run_id.clone();

    // Write historical data directly into SQLite storage
    {
        let mut storage = SqliteStorage::open(&db_path).unwrap();
        storage.insert_run(&run_meta).unwrap();

        let pt_cfg = HistorianPointConfig::new(
            pt_id,
            tag_id,
            "TI-101",
            None,
            "degC",
            0.0,
            300.0,
            SamplingMode::Periodic {
                interval_sim_seconds: 1.0,
            },
        );
        storage.insert_points(&run_meta.run_id, &[pt_cfg]).unwrap();

        let rec1 = HistorianRecord::Sample(HistoricalSample::new_float(
            pt_id,
            SimulationTime::try_new(0.0).unwrap(),
            1,
            180.0,
            TagQuality::Good,
            1700000000000,
        ));
        let rec2 = HistorianRecord::Sample(HistoricalSample::new_float(
            pt_id,
            SimulationTime::try_new(1.0).unwrap(),
            2,
            182.5,
            TagQuality::Good,
            1700000001000,
        ));
        let rec3 = HistorianRecord::Event(HistoricalEvent::alarm(
            &run_meta.run_id,
            SimulationTime::try_new(1.0).unwrap(),
            1,
            "ALM_PI101_HI",
            None,
            "ActiveUnack",
            36.2,
            3,
            None,
            1700000000000,
        ));

        storage
            .write_batch(&run_meta.run_id, &[rec1, rec2, rec3])
            .unwrap();
    }

    // Attach SqliteHistorianReader to host
    let reader = Arc::new(SqliteHistorianReader::open(&db_path).unwrap());

    let config = ServerConfig::default();
    let engine = SimulationEngine::new(config.simulation_dt).unwrap();
    let mut tag_store = RuntimeTagStore::new();
    tag_store.register_tag("TI-101", tag_id, TagValue::Float(185.0), TagQuality::Good);

    let host = SimulationHost::start(
        config,
        engine,
        tag_store,
        None,
        None,
        Some(reader as Arc<dyn nebudcs_historian::HistorianReader>),
    )
    .unwrap();

    let app = build_app(host.handle().clone());

    // Query Timeseries Raw
    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/v1/history/query?run_id={run_id}&point_id=TI-101&start_time=0.0&end_time=2.0"
        ))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    if status != StatusCode::OK {
        panic!(
            "Query failed with status {status}: {}",
            String::from_utf8_lossy(&body_bytes)
        );
    }
    let raw_resp: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(raw_resp.is_array());
    assert_eq!(raw_resp.as_array().unwrap().len(), 2);

    // Query Events
    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/v1/history/events?run_id={run_id}&start_time=0.0&end_time=2.0"
        ))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let events_resp: Vec<HistoricalEventDto> = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(events_resp.len(), 1);
    assert_eq!(events_resp[0].event_type, "ALARM");
    assert_eq!(events_resp[0].source_id, "ALM_PI101_HI");
}

#[tokio::test]
async fn test_websocket_lifecycle_handshake_and_command_stream() {
    let config = ServerConfig::default();
    let (mut host, handle) = create_test_host(config.clone());
    let app = build_app(handle);

    // Bind TCP listener on ephemeral port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let ws_url = format!("ws://{addr}/ws");
    let (mut ws_stream, _) = connect_async(&ws_url)
        .await
        .expect("WebSocket connection failed");

    // 1. Handshake
    let handshake = ClientMessage::Handshake {
        protocol_version: PROTOCOL_VERSION.to_string(),
        client_id: "test-client-1".to_string(),
        capabilities: ClientCapabilities::default(),
    };
    let hs_json = serde_json::to_string(&handshake).unwrap();
    ws_stream
        .send(TungMessage::Text(hs_json.into()))
        .await
        .unwrap();

    // 2. Expect HandshakeAck
    let first_reply = ws_stream.next().await.unwrap().unwrap();
    let first_text = first_reply.into_text().unwrap();
    let server_ack: ServerMessage = serde_json::from_str(&first_text).unwrap();
    match server_ack {
        ServerMessage::HandshakeAck {
            protocol_version, ..
        } => {
            assert_eq!(protocol_version, PROTOCOL_VERSION);
        }
        other => panic!("Expected HandshakeAck, got {other:?}"),
    }

    // 3. Expect initial Snapshot
    let second_reply = ws_stream.next().await.unwrap().unwrap();
    let second_text = second_reply.into_text().unwrap();
    let server_snap: ServerMessage = serde_json::from_str(&second_text).unwrap();
    match server_snap {
        ServerMessage::Snapshot(s) => {
            assert!(s.tags.iter().any(|t| t.tag == "TI-101"));
        }
        other => panic!("Expected Snapshot, got {other:?}"),
    }

    // 4. Ping / Pong
    let ping = ClientMessage::Ping { nonce: 4242 };
    ws_stream
        .send(TungMessage::Text(
            serde_json::to_string(&ping).unwrap().into(),
        ))
        .await
        .unwrap();

    let pong_reply = ws_stream.next().await.unwrap().unwrap();
    let pong_text = pong_reply.into_text().unwrap();
    let server_pong: ServerMessage = serde_json::from_str(&pong_text).unwrap();
    match server_pong {
        ServerMessage::Pong { nonce } => assert_eq!(nonce, 4242),
        other => panic!("Expected Pong, got {other:?}"),
    }

    // 5. Command over WebSocket: WriteTag
    let cmd = ClientMessage::Command {
        command_id: "cmd-ws-1".to_string(),
        command: RuntimeCommand::WriteTag {
            tag: "TI-101".to_string(),
            value: TagValueDto::Float(205.0),
        },
    };
    ws_stream
        .send(TungMessage::Text(
            serde_json::to_string(&cmd).unwrap().into(),
        ))
        .await
        .unwrap();

    let mut got_command_ack = false;
    let mut got_delta_update = false;

    for _ in 0..5 {
        let reply = ws_stream.next().await.unwrap().unwrap();
        let reply_text = reply.into_text().unwrap();
        let msg: ServerMessage = serde_json::from_str(&reply_text).unwrap();
        match msg {
            ServerMessage::CommandAck {
                command_id,
                success,
                ..
            } => {
                assert_eq!(command_id, "cmd-ws-1");
                assert!(success);
                got_command_ack = true;
            }
            ServerMessage::Delta(d)
                if d.changed_tags
                    .iter()
                    .any(|t| t.tag == "TI-101" && t.value == TagValueDto::Float(205.0)) =>
            {
                got_delta_update = true;
            }
            _ => {}
        }
        if got_command_ack && got_delta_update {
            break;
        }
    }

    assert!(got_command_ack, "Expected CommandAck to be received");
    assert!(got_delta_update, "Expected Delta update to be received");

    // Clean shutdown
    ws_stream.close(None).await.unwrap();
    server_task.abort();
    host.stop();
}

#[tokio::test]
async fn test_websocket_incompatible_version_rejected() {
    let config = ServerConfig::default();
    let (mut host, handle) = create_test_host(config);
    let app = build_app(handle);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let ws_url = format!("ws://{addr}/ws");
    let (mut ws_stream, _) = connect_async(&ws_url)
        .await
        .expect("WebSocket connection failed");

    // Incompatible major version
    let handshake = ClientMessage::Handshake {
        protocol_version: "99.0.0".to_string(),
        client_id: "incompatible-client".to_string(),
        capabilities: ClientCapabilities::default(),
    };
    ws_stream
        .send(TungMessage::Text(
            serde_json::to_string(&handshake).unwrap().into(),
        ))
        .await
        .unwrap();

    // Server must respond with Error: ProtocolVersionMismatch
    let reply = ws_stream.next().await.unwrap().unwrap();
    let reply_text = reply.into_text().unwrap();
    let server_err: ServerMessage = serde_json::from_str(&reply_text).unwrap();
    match server_err {
        ServerMessage::Error(err) => {
            assert_eq!(err.code, ProtocolErrorCode::ProtocolVersionMismatch);
        }
        other => panic!("Expected Error message, got {other:?}"),
    }

    server_task.abort();
    host.stop();
}

#[tokio::test]
async fn test_websocket_subscription_filtering() {
    let config = ServerConfig::default();
    let (mut host, handle) = create_test_host(config);
    let app = build_app(handle);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let ws_url = format!("ws://{addr}/ws");

    // Client A connects and subscribes only to TI-101
    let (mut ws_a, _) = connect_async(&ws_url).await.unwrap();
    let hs_a = ClientMessage::Handshake {
        protocol_version: PROTOCOL_VERSION.to_string(),
        client_id: "client-a".to_string(),
        capabilities: ClientCapabilities::default(),
    };
    ws_a.send(TungMessage::Text(
        serde_json::to_string(&hs_a).unwrap().into(),
    ))
    .await
    .unwrap();
    let _ = ws_a.next().await.unwrap().unwrap(); // HandshakeAck
    let _ = ws_a.next().await.unwrap().unwrap(); // Snapshot
    let sub_a = ClientMessage::Subscribe {
        tags: vec!["TI-101".to_string()],
        all_tags: false,
        include_alarms: false,
    };
    ws_a.send(TungMessage::Text(
        serde_json::to_string(&sub_a).unwrap().into(),
    ))
    .await
    .unwrap();

    // Client B connects and subscribes only to PI-101
    let (mut ws_b, _) = connect_async(&ws_url).await.unwrap();
    let hs_b = ClientMessage::Handshake {
        protocol_version: PROTOCOL_VERSION.to_string(),
        client_id: "client-b".to_string(),
        capabilities: ClientCapabilities::default(),
    };
    ws_b.send(TungMessage::Text(
        serde_json::to_string(&hs_b).unwrap().into(),
    ))
    .await
    .unwrap();
    let _ = ws_b.next().await.unwrap().unwrap(); // HandshakeAck
    let _ = ws_b.next().await.unwrap().unwrap(); // Snapshot
    let sub_b = ClientMessage::Subscribe {
        tags: vec!["PI-101".to_string()],
        all_tags: false,
        include_alarms: false,
    };
    ws_b.send(TungMessage::Text(
        serde_json::to_string(&sub_b).unwrap().into(),
    ))
    .await
    .unwrap();

    // Mutate TI-101 via Command from Client A
    let cmd = ClientMessage::Command {
        command_id: "cmd-sub-1".to_string(),
        command: RuntimeCommand::WriteTag {
            tag: "TI-101".to_string(),
            value: TagValueDto::Float(222.0),
        },
    };
    ws_a.send(TungMessage::Text(
        serde_json::to_string(&cmd).unwrap().into(),
    ))
    .await
    .unwrap();

    // Client A must receive Delta containing TI-101
    let mut client_a_got_delta = false;
    for _ in 0..5 {
        let reply = ws_a.next().await.unwrap().unwrap();
        let msg: ServerMessage = serde_json::from_str(&reply.into_text().unwrap()).unwrap();
        if let ServerMessage::Delta(d) = msg {
            let matched = d
                .changed_tags
                .iter()
                .any(|t| t.tag == "TI-101" && t.value == TagValueDto::Float(222.0));
            if matched {
                client_a_got_delta = true;
                break;
            }
        }
    }
    assert!(client_a_got_delta, "Client A should receive TI-101 update");

    // Client B must NOT receive any Delta for TI-101 (timeout check via Ping/Pong)
    let ping_b = ClientMessage::Ping { nonce: 7777 };
    ws_b.send(TungMessage::Text(
        serde_json::to_string(&ping_b).unwrap().into(),
    ))
    .await
    .unwrap();
    let reply_b = ws_b.next().await.unwrap().unwrap();
    let msg_b: ServerMessage = serde_json::from_str(&reply_b.into_text().unwrap()).unwrap();
    match msg_b {
        ServerMessage::Pong { nonce } => assert_eq!(nonce, 7777),
        ServerMessage::Delta(d) => {
            assert!(
                !d.changed_tags.iter().any(|t| t.tag == "TI-101"),
                "Client B should not receive TI-101"
            );
        }
        _ => {}
    }

    ws_a.close(None).await.unwrap();
    ws_b.close(None).await.unwrap();
    server_task.abort();
    host.stop();
}

#[tokio::test]
async fn test_concurrent_clients_and_connection_tracking() {
    let config = ServerConfig::default();
    let (mut host, handle) = create_test_host(config);
    let app = build_app(handle.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let ws_url = format!("ws://{addr}/ws");
    let mut clients = Vec::new();

    for i in 0..5 {
        let (mut ws, _) = connect_async(&ws_url).await.unwrap();
        let hs = ClientMessage::Handshake {
            protocol_version: PROTOCOL_VERSION.to_string(),
            client_id: format!("concurrent-client-{i}"),
            capabilities: ClientCapabilities::default(),
        };
        ws.send(TungMessage::Text(
            serde_json::to_string(&hs).unwrap().into(),
        ))
        .await
        .unwrap();
        let _ = ws.next().await.unwrap().unwrap(); // Ack
        let _ = ws.next().await.unwrap().unwrap(); // Snapshot
        clients.push(ws);
    }

    // Check health endpoint reports 5 connected clients
    let health = handle.get_health().await;
    assert_eq!(health.connected_clients, 5);

    // Disconnect 3 clients
    for mut ws in clients.drain(..3) {
        let _ = ws.close(None).await;
    }
    tokio::time::sleep(Duration::from_millis(50)).await;

    let health = handle.get_health().await;
    assert_eq!(health.connected_clients, 2);

    // Disconnect remaining 2
    for mut ws in clients.drain(..) {
        let _ = ws.close(None).await;
    }
    tokio::time::sleep(Duration::from_millis(50)).await;

    let health = handle.get_health().await;
    assert_eq!(health.connected_clients, 0);

    server_task.abort();
    host.stop();
}
