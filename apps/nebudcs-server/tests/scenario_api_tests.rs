//! End-to-end integration tests for NebuDCS Phase 9 Scenario & Fault subsystem.

use std::time::Duration;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message as TungMessage;
use tower::ServiceExt;

use nebudcs_control::pid::{PidConfig, PidDirection};
use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_control::ControlSystem;
use nebudcs_model::tag::{TagQuality, TagValue};
use nebudcs_protocol::*;
use nebudcs_scenario::SCENARIO_SCHEMA_VERSION;
use nebudcs_server::{build_app, HostHandle, ServerConfig, SimulationHost};
use nebudcs_sim::SimulationEngine;
use nebudcs_types::{EquipmentId, TagId};

fn create_test_host(config: ServerConfig) -> (SimulationHost, HostHandle) {
    let engine = SimulationEngine::new(config.simulation_dt).expect("Failed to create engine");

    let mut tag_store = RuntimeTagStore::new();
    let p1 = TagId::new();
    let lic_pv = TagId::new();
    let lic_out = TagId::new();
    let eq_id = EquipmentId::new();

    tag_store.register_tag("PI-101", p1, TagValue::Float(30.0), TagQuality::Good);
    tag_store.register_tag(
        "LIC-101.PV",
        lic_pv,
        TagValue::Float(50.0),
        TagQuality::Good,
    );
    tag_store.register_tag(
        "LIC-101.OUT",
        lic_out,
        TagValue::Float(50.0),
        TagQuality::Good,
    );

    let mut ctrl_sys = ControlSystem::new();
    ctrl_sys.register_pid(
        lic_pv,
        lic_out,
        eq_id,
        PidConfig {
            kp: 1.0,
            ti: 10.0,
            direction: PidDirection::Direct,
            ..Default::default()
        },
    );

    let host = SimulationHost::start(config, engine, tag_store, None, Some(ctrl_sys), None)
        .expect("Failed to start simulation host");

    let handle = host.handle();
    (host, handle)
}

#[tokio::test]
async fn test_scenario_rest_lifecycle() {
    let config = ServerConfig {
        simulation_dt: 0.05,
        ..Default::default()
    };
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle.clone());

    // 1. Initial scenario status should be IDLE
    let req = Request::builder()
        .uri("/api/v1/scenario/status")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let status_dto: ScenarioStatusDto = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(status_dto.state, "IDLE");
    assert_eq!(status_dto.active_faults.len(), 0);

    // 2. Load declarative scenario definition
    let scn_def = json!({
        "id": "SCN-TEST-1",
        "name": "Integration Test Scenario",
        "description": "Verifying REST API integration",
        "author": "Tester",
        "schema_version": SCENARIO_SCHEMA_VERSION,
        "duration_limit_sec": 120.0,
        "random_seed": 12345,
        "initial_conditions": {
            "tags": [
                { "tag": "PI-101", "value": 45.0 }
            ],
            "controllers": [
                { "loop_name": "LIC-101.PV", "mode": "MANUAL", "sp": 60.0, "manual_output": 75.0 }
            ],
            "equipment": []
        },
        "timeline": [
            {
                "id": "EVT-1",
                "priority": 1,
                "trigger": { "type": "time", "elapsed_sec": 10.0 },
                "action": {
                    "type": "apply_disturbance",
                    "id": "DST-STEP",
                    "target_tag": "PI-101",
                    "disturbance_type": { "waveform": "step", "value": 5.0, "relative": true },
                    "duration_sec": 20.0
                }
            }
        ],
        "objectives": []
    });

    let req = Request::builder()
        .uri("/api/v1/scenario/load")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(scn_def.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Give the runner thread a brief instant to apply the command
    tokio::time::sleep(Duration::from_millis(50)).await;

    let status = handle.get_scenario_status().await;
    assert_eq!(status.state, "LOADED");
    assert_eq!(status.scenario_id, Some("SCN-TEST-1".to_string()));

    // 3. Start scenario (arms initial conditions and begins ticking)
    let req = Request::builder()
        .uri("/api/v1/scenario/start")
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    tokio::time::sleep(Duration::from_millis(50)).await;
    let status = handle.get_scenario_status().await;
    assert_eq!(status.state, "RUNNING");

    // Verify initial tag value was applied (PI-101 set to 45.0)
    let snap = handle.get_snapshot().await;
    let pi101 = snap.tags.iter().find(|t| t.tag == "PI-101").unwrap();
    assert_eq!(pi101.value, TagValueDto::Float(45.0));

    // 4. Pause scenario
    let req = Request::builder()
        .uri("/api/v1/scenario/pause")
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(handle.get_scenario_status().await.state, "PAUSED");

    // 5. Resume scenario
    let req = Request::builder()
        .uri("/api/v1/scenario/resume")
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(handle.get_scenario_status().await.state, "RUNNING");

    // 6. Abort scenario
    let req = Request::builder()
        .uri("/api/v1/scenario/abort")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "reason": "Operator command" }).to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(handle.get_scenario_status().await.state, "ABORTED");

    // 7. Reset scenario
    let req = Request::builder()
        .uri("/api/v1/scenario/reset")
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(handle.get_scenario_status().await.state, "LOADED");
}

#[tokio::test]
async fn test_fault_injection_and_clearing_api() {
    let config = ServerConfig {
        simulation_dt: 0.05,
        ..Default::default()
    };
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle.clone());

    // 1. Check baseline tag value
    let snap = handle.get_snapshot().await;
    let pi101 = snap.tags.iter().find(|t| t.tag == "PI-101").unwrap();
    let baseline_val = pi101.value.as_f64().unwrap();

    // 2. Inject sensor bias fault (+15.0 bar)
    let fault_json = json!({
        "id": "FLT-TEST-BIAS",
        "target": "PI-101",
        "fault_type": {
            "category": "sensor_bias",
            "offset": 15.0
        },
        "duration_sec": 60.0
    });

    let req = Request::builder()
        .uri("/api/v1/scenario/fault/inject")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(fault_json.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    tokio::time::sleep(Duration::from_millis(60)).await;

    // Verify fault is active
    let status = handle.get_scenario_status().await;
    assert_eq!(status.active_faults.len(), 1);
    assert_eq!(status.active_faults[0].id, "FLT-TEST-BIAS");
    assert_eq!(status.active_faults[0].category, "sensor_bias");

    // Verify physical tag is offset by +15.0
    let snap = handle.get_snapshot().await;
    let pi101_faulted = snap.tags.iter().find(|t| t.tag == "PI-101").unwrap();
    let val_faulted = pi101_faulted.value.as_f64().unwrap();
    assert!((val_faulted - (baseline_val + 15.0)).abs() < 1e-4);

    // 3. Clear active fault
    let clear_json = json!({
        "fault_id": "FLT-TEST-BIAS"
    });

    let req = Request::builder()
        .uri("/api/v1/scenario/fault/clear")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(clear_json.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    tokio::time::sleep(Duration::from_millis(60)).await;

    // Verify fault cleared and tag value restored
    let status = handle.get_scenario_status().await;
    assert_eq!(status.active_faults.len(), 0);

    let snap = handle.get_snapshot().await;
    let pi101_clean = snap.tags.iter().find(|t| t.tag == "PI-101").unwrap();
    let val_clean = pi101_clean.value.as_f64().unwrap();
    assert!((val_clean - baseline_val).abs() < 1e-4);
}

#[tokio::test]
async fn test_scenario_security_token_enforcement() {
    let config = ServerConfig {
        simulation_dt: 0.05,
        auth_token: Some("secret-token-42".to_string()),
        ..Default::default()
    };
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle);

    // Unauthorized request without token should be rejected with 401 Unauthorized
    let req = Request::builder()
        .uri("/api/v1/scenario/reset")
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // Authorized request with Bearer token succeeds
    let req = Request::builder()
        .uri("/api/v1/scenario/reset")
        .method("POST")
        .header(header::AUTHORIZATION, "Bearer secret-token-42")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_websocket_scenario_status_streaming() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind ephemeral test port");
    let local_addr = listener.local_addr().unwrap();

    let config = ServerConfig {
        bind_addr: local_addr,
        simulation_dt: 0.02,
        ..Default::default()
    };
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle.clone());

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let ws_url = format!("ws://127.0.0.1:{}/ws", local_addr.port());
    let (mut ws_stream, _) = connect_async(&ws_url)
        .await
        .expect("Failed to connect websocket");

    // Perform handshake
    let handshake = ClientMessage::Handshake {
        protocol_version: PROTOCOL_VERSION.to_string(),
        client_id: "test-scenario-client".to_string(),
        capabilities: ClientCapabilities::default(),
    };
    ws_stream
        .send(TungMessage::Text(
            serde_json::to_string(&handshake).unwrap().into(),
        ))
        .await
        .unwrap();

    let ack_msg = ws_stream.next().await.unwrap().unwrap();
    let ack: ServerMessage = serde_json::from_str(ack_msg.to_text().unwrap()).unwrap();
    assert!(matches!(ack, ServerMessage::HandshakeAck { .. }));

    // Inject fault via REST and verify WebSocket receives ScenarioUpdate or delta
    let fault_json = json!({
        "id": "FLT-WS-01",
        "target": "PI-101",
        "fault_type": {
            "category": "sensor_bias",
            "offset": 20.0
        },
        "duration_sec": 30.0
    });

    handle
        .dispatch_command(RuntimeCommand::Scenario(ScenarioCommand::InjectFault {
            fault_json: fault_json.to_string(),
        }))
        .await
        .unwrap();

    // Check incoming websocket messages for ScenarioUpdate
    let mut found_update = false;
    for _ in 0..10 {
        if let Ok(Some(Ok(msg))) =
            tokio::time::timeout(Duration::from_millis(200), ws_stream.next()).await
        {
            if let Ok(ServerMessage::ScenarioUpdate(dto)) =
                serde_json::from_str::<ServerMessage>(msg.to_text().unwrap())
            {
                if dto.active_faults.iter().any(|f| f.id == "FLT-WS-01") {
                    found_update = true;
                    break;
                }
            }
        }
    }
    assert!(
        found_update,
        "Expected WebSocket to receive ScenarioUpdate broadcast with active fault"
    );
}

#[tokio::test]
async fn test_adversarial_rest_api_rejection() {
    let config = ServerConfig {
        simulation_dt: 0.05,
        ..Default::default()
    };
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle.clone());

    // 1. Malformed JSON payload on /load
    let req = Request::builder()
        .uri("/api/v1/scenario/load")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"not_a_valid_field": true}"#))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(
        res.status().is_client_error(),
        "Malformed JSON must return 4xx error"
    );

    // 2. Malformed JSON payload on /fault/inject
    let req = Request::builder()
        .uri("/api/v1/scenario/fault/inject")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"category": "unknown_cat"}"#))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(
        res.status().is_client_error(),
        "Malformed fault injection must return 4xx error"
    );

    // 3. Scenario validation error (e.g. negative duration)
    let invalid_scn = json!({
        "id": "SCN-BAD",
        "name": "Bad Scenario",
        "description": "Validation failure",
        "author": "Tester",
        "schema_version": SCENARIO_SCHEMA_VERSION,
        "duration_limit_sec": -10.0,
        "random_seed": 1,
        "initial_conditions": {
            "tags": [],
            "controllers": [],
            "equipment": []
        },
        "timeline": [],
        "objectives": []
    });
    let req = Request::builder()
        .uri("/api/v1/scenario/load")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(invalid_scn.to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(
        res.status().is_client_error(),
        "Scenario with negative duration must return 4xx error"
    );
}
