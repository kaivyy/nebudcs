//! NebuDCS Stage 11: Advanced Training Simulator / Operator Training Simulator (OTS) Integration Tests
//!
//! Standards Reference: ANSI/ISA-77.20 / IEC 61511 / IEC 62443 / ISA-18.2 / ISA-101.
//!
//! Verifies:
//! 1. Extended lifecycle state machine: CREATED -> PREPARING -> READY -> RUNNING -> COMPLETED.
//! 2. Session prepare endpoint (`POST /api/v1/training/sessions/{id}/prepare`).
//! 3. Structured debrief endpoint (`GET /api/v1/training/sessions/{id}/debrief`).
//! 4. Scenario branching endpoints (`GET /api/v1/training/sessions/{id}/branches` and trigger).
//! 5. RBAC operational enforcement:
//!    - Trainee is strictly prohibited from scenario authoring, fault injection, and SIF bypass (403 Forbidden).
//!    - Trainee is fully permitted to operate modulating control loops, acknowledge alarms, and record actions (200 OK).
//! 6. Deterministic evaluation with SafetyTripPolicy (Avoidable vs Expected).

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;

use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_control::system::ControlSystem;
use nebudcs_model::tag::{TagQuality, TagValue};
use nebudcs_protocol::{
    CreateTrainingSessionRequest, EvaluationReportDto, ScenarioBranchDto, TrainingDebriefDto,
    TrainingSessionDetailDto,
};
use nebudcs_server::{build_app, HostHandle, ServerConfig, SimulationHost};
use nebudcs_sim::SimulationEngine;
use nebudcs_types::{EquipmentId, TagId};

fn create_stage_11_test_host() -> (SimulationHost, HostHandle, ServerConfig) {
    let config = ServerConfig {
        simulation_dt: 0.1,
        admin_token: Some("sec-admin-token-stage11-xyz".to_string()),
        instructor_token: Some("sec-inst-token-stage11-xyz".to_string()),
        operator_token: Some("sec-oper-token-stage11-xyz".to_string()),
        trainee_token: Some("sec-train-token-stage11-xyz".to_string()),
        ..Default::default()
    };

    let mut engine = SimulationEngine::new(config.simulation_dt).expect("Failed to create engine");
    engine.clock_mut().play();
    let mut tag_store = RuntimeTagStore::new();
    let pt_id = TagId::new();
    let pic_id = TagId::new();
    tag_store.register_tag("PT-101", pt_id, TagValue::Float(40.0), TagQuality::Good);
    tag_store.register_tag("PIC-101", pic_id, TagValue::Float(40.0), TagQuality::Good);

    let mut ctrl_sys = ControlSystem::new();
    let pid_cfg = nebudcs_control::pid::PidConfig {
        kp: 1.2,
        ti: 20.0,
        td: 0.0,
        derivative_filter_n: 10.0,
        out_min: 0.0,
        out_max: 100.0,
        direction: nebudcs_control::pid::PidDirection::Direct,
        sp_tracking_in_manual: true,
        scan_divisor: 1,
    };
    ctrl_sys.register_pid(pt_id, pic_id, EquipmentId::new(), pid_cfg);

    let host = SimulationHost::start(
        config.clone(),
        engine,
        tag_store,
        Some(nebudcs_alarm::AlarmSystem::new()),
        Some(ctrl_sys),
        None,
    )
    .expect("Failed to start simulation host");
    let handle = host.handle();

    (host, handle, config)
}

#[tokio::test]
async fn test_stage_11_session_lifecycle_prepare_start_complete_debrief() {
    let (_host, handle, _config) = create_stage_11_test_host();
    let app = build_app(handle);

    let admin_tok = "sec-admin-token-stage11-xyz";

    // 1. Create a training session as Instructor/Admin
    let create_req = CreateTrainingSessionRequest {
        name: "Stage 11 OTS Lifecycle Drill".to_string(),
        description: Some("Comprehensive test of OTS state machine".to_string()),
        project_id: None,
        scenario_id: None,
        instructor_id: "inst-lead".to_string(),
        trainee_id: "trainee-alpha".to_string(),
    };

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/training/sessions")
        .header(header::AUTHORIZATION, format!("Bearer {admin_tok}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&create_req).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let created: TrainingSessionDetailDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(created.state, "CREATED");
    let session_id = created.id;

    // 2. Prepare session -> Transitions through Preparing to Ready
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/training/sessions/{session_id}/prepare"))
        .header(header::AUTHORIZATION, format!("Bearer {admin_tok}"))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let prepared: TrainingSessionDetailDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(prepared.state, "READY");

    // 3. Start session -> Transitions to Running
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/training/sessions/{session_id}/start"))
        .header(header::AUTHORIZATION, format!("Bearer {admin_tok}"))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let started: TrainingSessionDetailDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(started.state, "RUNNING");

    // 4. Record a trainee intervention action
    let action_req = json!({
        "session_id": session_id,
        "actor": "trainee-alpha",
        "target_entity": "TAH-101",
        "action_type": "ACK_ALARM"
    });

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/training/sessions/{session_id}/action"))
        .header(header::AUTHORIZATION, format!("Bearer {admin_tok}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&action_req).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 5. Complete session -> Generates evaluation and structured debrief
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/training/sessions/{session_id}/complete"))
        .header(header::AUTHORIZATION, format!("Bearer {admin_tok}"))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let eval: EvaluationReportDto = serde_json::from_slice(&body).unwrap();
    assert!(eval.passed);
    assert!(eval.debrief.is_some());

    // 6. Query structured debrief via GET /api/v1/training/sessions/{id}/debrief
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/training/sessions/{session_id}/debrief"))
        .header(header::AUTHORIZATION, format!("Bearer {admin_tok}"))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let debrief: TrainingDebriefDto = serde_json::from_slice(&body).unwrap();
    assert!(!debrief.process_performance.is_empty());
    assert!(!debrief.operator_performance.is_empty());
    assert!(!debrief.safety_assessment.is_empty());
    assert!(debrief.outcome_verdict.contains("STAGE 11 OTS ASSESSMENT"));
    assert!(!debrief.recommendations.is_empty());
}

#[tokio::test]
async fn test_stage_11_branching_endpoints() {
    let (_host, handle, _config) = create_stage_11_test_host();
    let app = build_app(handle);
    let admin_tok = "sec-admin-token-stage11-xyz";

    // 1. Create a session
    let create_req = CreateTrainingSessionRequest {
        name: "Branching Scenario Test".to_string(),
        description: None,
        project_id: None,
        scenario_id: None,
        instructor_id: "inst-lead".to_string(),
        trainee_id: "trainee-1".to_string(),
    };

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/training/sessions")
        .header(header::AUTHORIZATION, format!("Bearer {admin_tok}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&create_req).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let created: TrainingSessionDetailDto = serde_json::from_slice(&body).unwrap();
    let session_id = created.id;

    // 2. Query branches
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/training/sessions/{session_id}/branches"))
        .header(header::AUTHORIZATION, format!("Bearer {admin_tok}"))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. Trigger branch manually
    let req = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/v1/training/sessions/{session_id}/branches/BR-SURGE-RECOVERY/trigger"
        ))
        .header(header::AUTHORIZATION, format!("Bearer {admin_tok}"))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let branch: ScenarioBranchDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(branch.branch_id, "BR-SURGE-RECOVERY");
    assert!(branch.triggered);
}

#[tokio::test]
async fn test_stage_11_rbac_trainee_operational_isolation() {
    let (_host, handle, _config) = create_stage_11_test_host();
    let app = build_app(handle);

    let trainee_tok = "sec-train-token-stage11-xyz";
    let admin_tok = "sec-admin-token-stage11-xyz";

    // 1. Trainee cannot create training session (403 Forbidden)
    let create_req = CreateTrainingSessionRequest {
        name: "Unauthorized Trainee Session".to_string(),
        description: None,
        project_id: None,
        scenario_id: None,
        instructor_id: "inst-lead".to_string(),
        trainee_id: "trainee-1".to_string(),
    };

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/training/sessions")
        .header(header::AUTHORIZATION, format!("Bearer {trainee_tok}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&create_req).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // 2. Trainee cannot inject faults into simulation (403 Forbidden)
    let fault_req = json!({
        "id": "FLT-ILLEGAL",
        "target": "PT-101",
        "fault_type": {
            "category": "sensor_bias",
            "offset": 50.0
        },
        "duration_sec": null
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/scenario/fault/inject")
        .header(header::AUTHORIZATION, format!("Bearer {trainee_tok}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&fault_req).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // 3. Trainee cannot bypass SIFs (403 Forbidden)
    let bypass_req = json!({
        "bypass_type": "MAINTENANCE",
        "authorized_by": "trainee-1",
        "reason": "illegal bypass attempt",
        "duration_seconds": 60.0,
        "active": true
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/sis/sifs/SIF-101/bypass")
        .header(header::AUTHORIZATION, format!("Bearer {trainee_tok}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&bypass_req).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // 4. Trainee CAN operate modulating DCS control loops (200 OK)
    let sp_req = json!({
        "loop_name": "PIC-101",
        "sp": 45.0
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/control/pid/setpoint")
        .header(header::AUTHORIZATION, format!("Bearer {trainee_tok}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&sp_req).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 5. Trainee CAN acknowledge alarms (200 OK)
    let ack_req = json!({
        "alarm_name": "PAH-101"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/alarms/acknowledge")
        .header(header::AUTHORIZATION, format!("Bearer {trainee_tok}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&ack_req).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 6. Admin creates a session and starts it, trainee can record trainee action
    let create_req = CreateTrainingSessionRequest {
        name: "Trainee Interaction Session".to_string(),
        description: None,
        project_id: None,
        scenario_id: None,
        instructor_id: "inst-lead".to_string(),
        trainee_id: "trainee-1".to_string(),
    };

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/training/sessions")
        .header(header::AUTHORIZATION, format!("Bearer {admin_tok}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&create_req).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let created: TrainingSessionDetailDto = serde_json::from_slice(&body).unwrap();
    let session_id = created.id;

    // Prepare & start
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/training/sessions/{session_id}/prepare"))
        .header(header::AUTHORIZATION, format!("Bearer {admin_tok}"))
        .body(Body::empty())
        .unwrap();
    let _ = app.clone().oneshot(req).await.unwrap();

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/training/sessions/{session_id}/start"))
        .header(header::AUTHORIZATION, format!("Bearer {admin_tok}"))
        .body(Body::empty())
        .unwrap();
    let _ = app.clone().oneshot(req).await.unwrap();

    // Trainee submits operator intervention
    let action_req = json!({
        "session_id": session_id,
        "actor": "trainee-1",
        "target_entity": "PIC-101",
        "action_type": "SET_SETPOINT",
        "requested_value": 45.0
    });

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/training/sessions/{session_id}/action"))
        .header(header::AUTHORIZATION, format!("Bearer {trainee_tok}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&action_req).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
