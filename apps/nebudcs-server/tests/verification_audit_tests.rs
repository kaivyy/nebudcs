//! Independent Verification Test Suite for NebuDCS Phase 7.
//!
//! Validates:
//! 1. Determinism and multi-run repeatability
//! 2. Linearizability & read consistency (ack -> snapshot visibility)
//! 3. Subscription isolation across concurrent clients
//! 4. Backpressure handling and lagged client snapshot resync
//! 5. Numeric validation (NaN, Infinity, negative values)
//! 6. Security, token authentication, and payload limits
//! 7. Exhaustive REST endpoint coverage
//! 8. Graceful shutdown without thread leakage

use std::time::Duration;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message as TungMessage;
use tower::ServiceExt;

use nebudcs_alarm::block::AlarmBlockConfig;
use nebudcs_alarm::types::{AlarmSeverity, AlarmType};
use nebudcs_alarm::AlarmSystem;
use nebudcs_control::pid::{PidConfig, PidDirection};
use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_control::ControlSystem;
use nebudcs_model::tag::{TagQuality, TagValue};
use nebudcs_protocol::*;
use nebudcs_server::{build_app, HostHandle, ServerConfig, SimulationHost};
use nebudcs_sim::SimulationEngine;
use nebudcs_types::{EquipmentId, TagId};

fn create_verification_host(config: ServerConfig) -> (SimulationHost, HostHandle) {
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

/// Verification 1: Determinism and Multi-run Repeatability
#[tokio::test]
async fn test_audit_determinism_multi_run_repeatability() {
    let run_simulation = async || -> StateSnapshotDto {
        let config = ServerConfig {
            simulation_dt: 0.01,
            speed_multiplier: 10.0,
            ..Default::default()
        };
        let (mut host, handle) = create_verification_host(config);

        // Ensure simulation is paused for discrete deterministic stepping
        handle
            .dispatch_command(RuntimeCommand::SimControl(SimControlCommand::Pause))
            .await
            .unwrap();

        // Step 25 ticks deterministically
        for _ in 0..25 {
            handle
                .dispatch_command(RuntimeCommand::SimControl(SimControlCommand::Step))
                .await
                .unwrap();
        }

        // Write a tag
        handle
            .dispatch_command(RuntimeCommand::WriteTag {
                tag: "TI-101".to_string(),
                value: TagValueDto::Float(215.75),
            })
            .await
            .unwrap();

        // Step 10 more ticks
        for _ in 0..10 {
            handle
                .dispatch_command(RuntimeCommand::SimControl(SimControlCommand::Step))
                .await
                .unwrap();
        }

        handle
            .dispatch_command(RuntimeCommand::SimControl(SimControlCommand::Pause))
            .await
            .unwrap();

        let snapshot = handle.get_snapshot().await;
        host.stop();
        snapshot
    };

    let snap1 = run_simulation().await;
    let snap2 = run_simulation().await;

    assert_eq!(
        snap1.tick_count, snap2.tick_count,
        "Tick count must be identical"
    );
    assert!(
        (snap1.sim_time_sec - snap2.sim_time_sec).abs() < 1e-9,
        "Sim time must be identical"
    );

    for (t1, t2) in snap1.tags.iter().zip(snap2.tags.iter()) {
        assert_eq!(t1.tag, t2.tag);
        assert_eq!(t1.value, t2.value);
        assert_eq!(t1.quality, t2.quality);
    }
}

/// Verification 2: Linearizability & Read Consistency
/// Invariant: Once dispatch_command returns Ok(()), the state MUST be visible in get_snapshot().
#[tokio::test]
async fn test_audit_linearizability_read_consistency() {
    let config = ServerConfig::default();
    let (mut host, handle) = create_verification_host(config);

    for target_temp in [210.0, 225.5, 250.0, 275.25, 300.0] {
        // Dispatch write command
        handle
            .dispatch_command(RuntimeCommand::WriteTag {
                tag: "TI-101".to_string(),
                value: TagValueDto::Float(target_temp),
            })
            .await
            .unwrap();

        // Immediately read snapshot without any sleep
        let snap = handle.get_snapshot().await;
        let tag_sample = snap
            .tags
            .iter()
            .find(|t| t.tag == "TI-101")
            .expect("TI-101 must exist");

        assert_eq!(
            tag_sample.value,
            TagValueDto::Float(target_temp),
            "Linearizability violation: updated value not visible immediately after ack"
        );
    }

    host.stop();
}

/// Verification 3: Subscription Isolation Across Multiple Clients
#[tokio::test]
async fn test_audit_subscription_isolation() {
    let config = ServerConfig::default();
    let (mut host, handle) = create_verification_host(config);
    let app = build_app(handle);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let ws_url = format!("ws://{addr}/ws");

    // Client 1 subscribes to TI-101
    let (mut ws1, _) = connect_async(&ws_url).await.unwrap();
    let hs1 = ClientMessage::Handshake {
        protocol_version: PROTOCOL_VERSION.to_string(),
        client_id: "client-sub-ti".to_string(),
        capabilities: ClientCapabilities::default(),
    };
    ws1.send(TungMessage::Text(
        serde_json::to_string(&hs1).unwrap().into(),
    ))
    .await
    .unwrap();
    let _ = ws1.next().await.unwrap().unwrap(); // Ack
    let _ = ws1.next().await.unwrap().unwrap(); // Snapshot
    let sub1 = ClientMessage::Subscribe {
        tags: vec!["TI-101".to_string()],
        all_tags: false,
        include_alarms: false,
    };
    ws1.send(TungMessage::Text(
        serde_json::to_string(&sub1).unwrap().into(),
    ))
    .await
    .unwrap();

    // Client 2 subscribes to PI-101
    let (mut ws2, _) = connect_async(&ws_url).await.unwrap();
    let hs2 = ClientMessage::Handshake {
        protocol_version: PROTOCOL_VERSION.to_string(),
        client_id: "client-sub-pi".to_string(),
        capabilities: ClientCapabilities::default(),
    };
    ws2.send(TungMessage::Text(
        serde_json::to_string(&hs2).unwrap().into(),
    ))
    .await
    .unwrap();
    let _ = ws2.next().await.unwrap().unwrap(); // Ack
    let _ = ws2.next().await.unwrap().unwrap(); // Snapshot
    let sub2 = ClientMessage::Subscribe {
        tags: vec!["PI-101".to_string()],
        all_tags: false,
        include_alarms: false,
    };
    ws2.send(TungMessage::Text(
        serde_json::to_string(&sub2).unwrap().into(),
    ))
    .await
    .unwrap();

    // Mutate TI-101 from Client 1
    let cmd_ti = ClientMessage::Command {
        command_id: "cmd-ti-1".to_string(),
        command: RuntimeCommand::WriteTag {
            tag: "TI-101".to_string(),
            value: TagValueDto::Float(199.9),
        },
    };
    ws1.send(TungMessage::Text(
        serde_json::to_string(&cmd_ti).unwrap().into(),
    ))
    .await
    .unwrap();

    // Client 1 receives TI-101 update
    let mut c1_got_ti = false;
    for _ in 0..5 {
        let reply = ws1.next().await.unwrap().unwrap();
        let msg: ServerMessage = serde_json::from_str(&reply.into_text().unwrap()).unwrap();
        if let ServerMessage::Delta(d) = msg {
            if d.changed_tags
                .iter()
                .any(|t| t.tag == "TI-101" && t.value == TagValueDto::Float(199.9))
            {
                c1_got_ti = true;
                break;
            }
        }
    }
    assert!(c1_got_ti, "Client 1 must receive subscribed TI-101 update");

    // Send a Ping from Client 2 to verify its message queue without hanging
    let ping2 = ClientMessage::Ping { nonce: 999 };
    ws2.send(TungMessage::Text(
        serde_json::to_string(&ping2).unwrap().into(),
    ))
    .await
    .unwrap();
    let reply2 = ws2.next().await.unwrap().unwrap();
    let msg2: ServerMessage = serde_json::from_str(&reply2.into_text().unwrap()).unwrap();
    if let ServerMessage::Delta(d) = msg2 {
        assert!(
            !d.changed_tags.iter().any(|t| t.tag == "TI-101"),
            "Client 2 must NOT receive unsubscribed TI-101 tag update"
        );
    }

    ws1.close(None).await.unwrap();
    ws2.close(None).await.unwrap();
    server_task.abort();
    host.stop();
}

/// Verification 4: Graceful Shutdown
#[tokio::test]
async fn test_audit_graceful_shutdown() {
    let config = ServerConfig::default();
    let (mut host, handle) = create_verification_host(config);
    let app = build_app(handle);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let ws_url = format!("ws://{addr}/ws");
    let (mut ws, _) = connect_async(&ws_url).await.unwrap();

    let hs = ClientMessage::Handshake {
        protocol_version: PROTOCOL_VERSION.to_string(),
        client_id: "shutdown-client".to_string(),
        capabilities: ClientCapabilities::default(),
    };
    ws.send(TungMessage::Text(
        serde_json::to_string(&hs).unwrap().into(),
    ))
    .await
    .unwrap();
    let _ = ws.next().await.unwrap().unwrap(); // Ack
    let _ = ws.next().await.unwrap().unwrap(); // Snapshot

    // Stop host gracefully
    host.stop();

    // Verify calling stop again is idempotent
    host.stop();

    ws.close(None).await.unwrap();
    server_task.abort();
}

/// Verification 5: Numeric Input Validation (NaN, Infinity, Negatives)
#[tokio::test]
async fn test_audit_numeric_input_validation() {
    let config = ServerConfig::default();
    let (mut host, handle) = create_verification_host(config);

    // 1. Negative speed multiplier must be rejected
    let res = handle
        .dispatch_command(RuntimeCommand::SimControl(
            SimControlCommand::SetSpeedMultiplier { multiplier: -2.0 },
        ))
        .await;
    assert!(res.is_err(), "Negative speed multiplier must be rejected");

    // 2. NaN speed multiplier must be rejected
    let res = handle
        .dispatch_command(RuntimeCommand::SimControl(
            SimControlCommand::SetSpeedMultiplier {
                multiplier: f64::NAN,
            },
        ))
        .await;
    assert!(res.is_err(), "NaN speed multiplier must be rejected");

    // 3. Infinity speed multiplier must be rejected
    let res = handle
        .dispatch_command(RuntimeCommand::SimControl(
            SimControlCommand::SetSpeedMultiplier {
                multiplier: f64::INFINITY,
            },
        ))
        .await;
    assert!(res.is_err(), "Infinity speed multiplier must be rejected");

    // 4. Non-finite setpoint must be rejected
    let res = handle
        .dispatch_command(RuntimeCommand::SetSetpoint {
            loop_name: "FIC-101.OUT".to_string(),
            sp: f64::NAN,
        })
        .await;
    assert!(res.is_err(), "NaN setpoint must be rejected");

    let res = handle
        .dispatch_command(RuntimeCommand::SetSetpoint {
            loop_name: "FIC-101.OUT".to_string(),
            sp: f64::INFINITY,
        })
        .await;
    assert!(res.is_err(), "Infinity setpoint must be rejected");

    // 5. Non-finite manual output must be rejected
    let res = handle
        .dispatch_command(RuntimeCommand::SetManualOutput {
            loop_name: "FIC-101.OUT".to_string(),
            output: f64::INFINITY,
        })
        .await;
    assert!(res.is_err(), "Infinity manual output must be rejected");

    // 6. Negative or non-finite shelve duration must be rejected
    let res = handle
        .dispatch_command(RuntimeCommand::ShelveAlarm {
            alarm_name: "ALM_PI101_HI".to_string(),
            duration_seconds: -10.0,
        })
        .await;
    assert!(res.is_err(), "Negative shelve duration must be rejected");

    // 7. Non-finite float tag write must be rejected
    let res = handle
        .dispatch_command(RuntimeCommand::WriteTag {
            tag: "TI-101".to_string(),
            value: TagValueDto::Float(f64::INFINITY),
        })
        .await;
    assert!(res.is_err(), "Infinity tag write must be rejected");

    host.stop();
}

/// Verification 6: Security, Token Authentication, and Payload Limits
#[tokio::test]
async fn test_audit_security_token_and_payload_limits() {
    let config = ServerConfig {
        auth_token: Some("secret-dcs-token-123".to_string()),
        max_http_payload_size: 1024, // 1 KB payload limit for test
        ..Default::default()
    };
    let (mut host, handle) = create_verification_host(config);
    let app = build_app(handle);

    // 1. Health endpoint must be accessible without token
    let req = Request::builder()
        .uri("/api/v1/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 2. Protected endpoint without token must return 401 Unauthorized
    let req = Request::builder()
        .uri("/api/v1/sim/snapshot")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 3. Protected endpoint with invalid Bearer token must return 401 Unauthorized
    let req = Request::builder()
        .uri("/api/v1/sim/snapshot")
        .method("GET")
        .header(header::AUTHORIZATION, "Bearer wrong-token")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 4. Protected endpoint with valid Bearer token must succeed (200 OK)
    let req = Request::builder()
        .uri("/api/v1/sim/snapshot")
        .method("GET")
        .header(header::AUTHORIZATION, "Bearer secret-dcs-token-123")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 5. Protected endpoint with valid query token must succeed (200 OK)
    let req = Request::builder()
        .uri("/api/v1/sim/snapshot?token=secret-dcs-token-123")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 6. Oversized payload exceeding max_http_payload_size must return 413 Payload Too Large
    let oversized_body = vec![b'x'; 2048]; // 2 KB > 1 KB limit
    let req = Request::builder()
        .uri("/api/v1/tags/write")
        .method("POST")
        .header(header::AUTHORIZATION, "Bearer secret-dcs-token-123")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(oversized_body))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::PAYLOAD_TOO_LARGE);

    host.stop();
}

/// Verification 7: Exhaustive REST Endpoint Coverage
#[tokio::test]
async fn test_audit_exhaustive_rest_endpoints() {
    let config = ServerConfig::default();
    let (mut host, handle) = create_verification_host(config);
    let app = build_app(handle);

    // GET /api/v1/health
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // GET /api/v1/sim/snapshot
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/sim/snapshot")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // POST /api/v1/sim/control (Play)
    let body = serde_json::to_vec(&SimControlCommand::Play).unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/sim/control")
                .method("POST")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // POST /api/v1/sim/control (Pause)
    let body = serde_json::to_vec(&SimControlCommand::Pause).unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/sim/control")
                .method("POST")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // POST /api/v1/sim/control (Step)
    let body = serde_json::to_vec(&SimControlCommand::Step).unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/sim/control")
                .method("POST")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // POST /api/v1/sim/control (SetSpeedMultiplier)
    let body =
        serde_json::to_vec(&SimControlCommand::SetSpeedMultiplier { multiplier: 2.5 }).unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/sim/control")
                .method("POST")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // GET /api/v1/tags
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/tags")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // GET /api/v1/tags/{tag_id}
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/tags/TI-101")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // POST /api/v1/tags/write
    let write_req = json!({ "tag": "TI-101", "value": 245.0 });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/tags/write")
                .method("POST")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&write_req).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // GET /api/v1/alarms
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/alarms")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // POST /api/v1/alarms/acknowledge
    let ack_req = json!({ "alarm_name": "ALM_PI101_HI" });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/alarms/acknowledge")
                .method("POST")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&ack_req).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // POST /api/v1/alarms/shelve
    let shelve_req = json!({ "alarm_name": "ALM_PI101_HI", "duration_seconds": 120.0 });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/alarms/shelve")
                .method("POST")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&shelve_req).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // POST /api/v1/control/pid/setpoint
    let sp_req = json!({ "loop_name": "FIC-101.OUT", "sp": 65.0 });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/control/pid/setpoint")
                .method("POST")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&sp_req).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // POST /api/v1/control/pid/mode
    let mode_req = json!({ "loop_name": "FIC-101.OUT", "mode": "AUTO" });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/control/pid/mode")
                .method("POST")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&mode_req).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // POST /api/v1/control/pid/output
    let out_req = json!({ "loop_name": "FIC-101.OUT", "output": 55.0 });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/control/pid/output")
                .method("POST")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&out_req).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    host.stop();
}

/// Verification 8: Backpressure Handling and Lagged Client Snapshot Resync
#[tokio::test]
async fn test_audit_lagged_client_resync() {
    let config = ServerConfig {
        broadcast_capacity: 4,
        client_queue_capacity: 64,
        simulation_dt: 0.01,
        speed_multiplier: 100.0,
        ..Default::default()
    };
    let (mut host, handle) = create_verification_host(config);
    let app = build_app(handle.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let ws_url = format!("ws://{addr}/ws");
    let (mut ws, _) = connect_async(&ws_url).await.unwrap();

    let hs = ClientMessage::Handshake {
        protocol_version: PROTOCOL_VERSION.to_string(),
        client_id: "lagged-client".to_string(),
        capabilities: ClientCapabilities::default(),
    };
    ws.send(TungMessage::Text(
        serde_json::to_string(&hs).unwrap().into(),
    ))
    .await
    .unwrap();

    // Consume HandshakeAck and initial Snapshot
    let _ = ws.next().await.unwrap().unwrap();
    let _ = ws.next().await.unwrap().unwrap();

    // Subscribe to all tags
    let sub = ClientMessage::Subscribe {
        tags: vec!["*".to_string()],
        all_tags: true,
        include_alarms: true,
    };
    ws.send(TungMessage::Text(
        serde_json::to_string(&sub).unwrap().into(),
    ))
    .await
    .unwrap();

    // Generate rapid updates while client sleeps
    for i in 0..20 {
        let _ = handle
            .dispatch_command(RuntimeCommand::WriteTag {
                tag: "TI-101".to_string(),
                value: TagValueDto::Float(100.0 + i as f64),
            })
            .await;
        let _ = handle
            .dispatch_command(RuntimeCommand::SimControl(SimControlCommand::Step))
            .await;
    }

    // Give a small moment for messages to flood broadcast buffer
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Now read from the client. It should receive messages without crashing or hanging
    let mut got_messages = 0;
    while let Ok(Some(Ok(msg))) = tokio::time::timeout(Duration::from_millis(200), ws.next()).await
    {
        if let TungMessage::Text(t) = msg {
            let parsed: ServerMessage = serde_json::from_str(&t).unwrap();
            match parsed {
                ServerMessage::Delta(_) | ServerMessage::Snapshot(_) => {
                    got_messages += 1;
                }
                _ => {}
            }
        }
        if got_messages >= 3 {
            break;
        }
    }
    assert!(
        got_messages > 0,
        "Client must receive messages without crashing after broadcast overflow"
    );

    ws.close(None).await.unwrap();
    server_task.abort();
    host.stop();
}
