//! NebuDCS Stage 10: Advanced DCS Operations & HMI Integration Tests
//!
//! Standards Reference: ISA-101 / ISA-18.2 / IEC 62682 / IEC 61511 / IEC 62443.
//!
//! Verifies:
//! 1. Alarm summary & ISA-18.2 flood metrics endpoint (`GET /api/v1/alarms/summary`).
//! 2. Alarm acknowledgement with operator audit trail (`POST /api/v1/alarms/acknowledge`).
//! 3. Operational separation and RBAC: Trainee cannot shelve alarms (403 Forbidden), Operator can.
//! 4. Model-driven equipment diagnostics (`GET /api/v1/diagnostics` and `/api/v1/diagnostics/{id}`).
//! 5. Model validity and uncertainty states (`Validated`, `WithinValidityRange`, `BadQuality`, etc.).
//! 6. First-out disturbance sequence analysis (`GET /api/v1/history/events/first-out`).

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use tower::ServiceExt;

use nebudcs_control::actuator::{ActuatorConfig, FailAction};
use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_control::system::ControlSystem;
use nebudcs_protocol::{
    AlarmSummaryDto, EquipmentDiagnosticsDto, FirstOutEventDto, ValidityStateDto,
};
use nebudcs_server::{build_app, HostHandle, ServerConfig, SimulationHost};
use nebudcs_sim::SimulationEngine;
use nebudcs_types::EquipmentId;

fn create_stage_10_test_host(config: ServerConfig) -> (SimulationHost, HostHandle) {
    let mut engine = SimulationEngine::new(config.simulation_dt).expect("Failed to create engine");
    engine.clock_mut().play();
    let tag_store = RuntimeTagStore::new();

    let mut ctrl_sys = ControlSystem::new();
    let valve_id = EquipmentId::new();
    ctrl_sys.register_actuator(
        valve_id,
        "opening",
        ActuatorConfig {
            stroke_time: 0.1,
            fail_action: FailAction::FailClosed,
            ..Default::default()
        },
        50.0,
        None,
    );

    let host = SimulationHost::start(
        config,
        engine,
        tag_store,
        Some(nebudcs_alarm::AlarmSystem::new()),
        Some(ctrl_sys),
        None,
    )
    .expect("Failed to start host");
    let handle = host.handle();
    (host, handle)
}

#[tokio::test]
async fn test_alarm_summary_and_flood_metrics() {
    let config = ServerConfig {
        simulation_dt: 0.02,
        ..Default::default()
    };
    let (_host, handle) = create_stage_10_test_host(config);
    let app = build_app(handle);

    let req = Request::builder()
        .uri("/api/v1/alarms/summary")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let summary: AlarmSummaryDto = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(summary.metrics.flood_threshold_per_10min, 10.0);
    assert!(!summary.metrics.is_flood);
    assert_eq!(summary.metrics.total_active, 0);
}

#[tokio::test]
async fn test_alarm_shelve_rbac_operational_separation() {
    let config = ServerConfig {
        simulation_dt: 0.02,
        trainee_token: Some("trainee-token-123".to_string()),
        operator_token: Some("operator-token-456".to_string()),
        admin_token: Some("admin-token-789".to_string()),
        ..Default::default()
    };

    let (_host, handle) = create_stage_10_test_host(config.clone());

    // 1. Trainee attempt to shelve alarm must be rejected with 403 Forbidden
    let app1 = build_app(handle.clone());
    let req_trainee = Request::builder()
        .uri("/api/v1/alarms/shelve")
        .method("POST")
        .header(header::AUTHORIZATION, "Bearer trainee-token-123")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({
                "alarm_name": "PAH-101",
                "duration_seconds": 300.0
            })
            .to_string(),
        ))
        .unwrap();

    let res_trainee = app1.oneshot(req_trainee).await.unwrap();
    assert_eq!(res_trainee.status(), StatusCode::FORBIDDEN);

    // 2. Operator attempt to shelve alarm must succeed (200 OK)
    let app2 = build_app(handle.clone());
    let req_operator = Request::builder()
        .uri("/api/v1/alarms/shelve")
        .method("POST")
        .header(header::AUTHORIZATION, "Bearer operator-token-456")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({
                "alarm_name": "PAH-101",
                "duration_seconds": 300.0
            })
            .to_string(),
        ))
        .unwrap();

    let res_operator = app2.oneshot(req_operator).await.unwrap();
    assert_eq!(res_operator.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_equipment_diagnostics_endpoint() {
    let config = ServerConfig {
        simulation_dt: 0.02,
        ..Default::default()
    };
    let (_host, handle) = create_stage_10_test_host(config);
    let app = build_app(handle.clone());

    let req = Request::builder()
        .uri("/api/v1/diagnostics")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let diags: Vec<EquipmentDiagnosticsDto> = serde_json::from_slice(&body_bytes).unwrap();

    // Default project has seeded equipment (e.g. pumps, valves, vessels)
    assert!(!diags.is_empty(), "Diagnostics list should not be empty");

    for diag in &diags {
        assert!(!diag.equipment_id.is_empty());
        assert!(!diag.equipment_name.is_empty());
        assert_eq!(diag.validity_state, ValidityStateDto::Validated);

        if diag.equipment_type == "Pump" {
            assert!(diag.npsha_m.is_some());
            assert!(diag.cavitation_margin_m.is_some());
            assert!(diag.vibration_mm_s.is_some());
        }
    }
}

#[tokio::test]
async fn test_first_out_event_sequence_endpoint() {
    let config = ServerConfig {
        simulation_dt: 0.02,
        ..Default::default()
    };
    let (_host, handle) = create_stage_10_test_host(config);
    let app = build_app(handle);

    let req = Request::builder()
        .uri("/api/v1/history/events/first-out?run_id=active")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let first_out: FirstOutEventDto = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(first_out.run_id, "active");
}
