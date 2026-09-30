//! Integration tests for Phase 15: Training Studio server-side subsystems.
//! Tests cover session lifecycle, scenario CRUD, trainee action recording,
//! deterministic evaluation, and replay frame reconstruction.

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use tower::ServiceExt;

use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_protocol::{
    CreateTrainingSessionRequest, ScenarioAuthoringDto, TraineeActionDto, TrainingSessionDetailDto,
};
use nebudcs_server::{
    build_app, scenario_manager::ScenarioManager, training_manager::TrainingManager, ServerConfig,
    SimulationHost,
};
use nebudcs_sim::SimulationEngine;

// ── Helper factory ────────────────────────────────────────────────────────

fn generic_authoring_dto(id: &str, name: &str) -> ScenarioAuthoringDto {
    ScenarioAuthoringDto {
        id: id.to_string(),
        name: name.to_string(),
        description: "Integration test scenario".to_string(),
        author: "test-author".to_string(),
        version: "1.0.0".to_string(),
        compatible_project_id: None,
        duration_limit_sec: Some(600.0),
        random_seed: 0xDEAD_BEEF,
        initial_conditions_json: "{}".to_string(),
        timeline_json: "[]".to_string(),
        objectives_json: "[]".to_string(),
        criteria_json: "[]".to_string(),
    }
}

fn create_session_req(name: &str) -> CreateTrainingSessionRequest {
    CreateTrainingSessionRequest {
        name: name.to_string(),
        description: Some("test session".to_string()),
        project_id: Some("proj-test-001".to_string()),
        scenario_id: Some("scn-generic-feed-loss".to_string()),
        instructor_id: "INSTRUCTOR-A".to_string(),
        trainee_id: "TRAINEE-B".to_string(),
    }
}

// ── ScenarioManager tests ────────────────────────────────────────────────

#[test]
fn test_scenario_manager_seeded_with_generic_scenarios() {
    let mgr = ScenarioManager::new();
    let scenarios = mgr.list_scenarios();
    assert!(
        scenarios.len() >= 2,
        "ScenarioManager must be seeded with at least 2 generic scenarios"
    );

    let ids: Vec<_> = scenarios.iter().map(|s| s.id.as_str()).collect();
    assert!(
        ids.contains(&"scn-generic-feed-loss"),
        "Feed-loss scenario must be pre-seeded"
    );
    assert!(
        ids.contains(&"scn-reactor-cooling-upset"),
        "Cooling-upset scenario must be pre-seeded"
    );
}

#[test]
fn test_scenario_crud_and_persistence() {
    let mut mgr = ScenarioManager::new();
    let dto = generic_authoring_dto("scn-test-crud", "CRUD Test Scenario");

    // Save
    let saved = mgr.save_scenario(dto.clone()).expect("save must succeed");
    assert_eq!(saved.id, "scn-test-crud");

    // Get
    let got = mgr
        .get_scenario("scn-test-crud")
        .expect("get must return saved scenario");
    assert_eq!(got.name, "CRUD Test Scenario");

    // Update
    let mut updated = got.clone();
    updated.description = "Updated description".to_string();
    mgr.save_scenario(updated).expect("update must succeed");

    let got2 = mgr.get_scenario("scn-test-crud").unwrap();
    assert_eq!(got2.description, "Updated description");

    // Delete
    mgr.delete_scenario("scn-test-crud")
        .expect("delete must succeed");
    assert!(
        mgr.get_scenario("scn-test-crud").is_none(),
        "Deleted scenario must not be retrievable"
    );
}

#[test]
fn test_scenario_duplicate() {
    let mut mgr = ScenarioManager::new();
    let dup = mgr
        .duplicate_scenario(
            "scn-generic-feed-loss",
            "scn-feed-loss-copy",
            "Feed Loss Copy",
        )
        .expect("duplicate must succeed");

    assert_eq!(dup.id, "scn-feed-loss-copy");
    assert_eq!(dup.name, "Feed Loss Copy");

    // Both exist
    assert!(mgr.get_scenario("scn-generic-feed-loss").is_some());
    assert!(mgr.get_scenario("scn-feed-loss-copy").is_some());
}

#[test]
fn test_scenario_save_rejects_invalid_json_in_criteria() {
    let mut mgr = ScenarioManager::new();
    let mut dto = generic_authoring_dto("scn-bad-criteria", "Bad Criteria");
    dto.criteria_json = "NOT VALID JSON".to_string();

    let result = mgr.save_scenario(dto);
    assert!(result.is_err(), "save must reject malformed criteria_json");
}

#[test]
fn test_scenario_to_runtime_definition() {
    let mgr = ScenarioManager::new();
    let def = mgr
        .to_runtime_definition("scn-generic-feed-loss")
        .expect("runtime definition must be produced for seeded scenario");

    assert_eq!(def.id, "scn-generic-feed-loss");
    assert_eq!(
        def.schema_version,
        nebudcs_scenario::model::SCENARIO_SCHEMA_VERSION
    );
}

// ── TrainingManager session lifecycle tests ──────────────────────────────

#[test]
fn test_create_and_retrieve_session() {
    let mut tm = TrainingManager::new();
    let dto = tm
        .create_session(create_session_req("My Test Session"))
        .expect("create session must succeed");

    assert_eq!(dto.state, "CREATED");
    assert!(dto.id.starts_with("tsess-"));

    let retrieved = tm
        .get_session(&dto.id)
        .expect("must retrieve created session");
    assert_eq!(retrieved.id, dto.id);
    assert_eq!(retrieved.name, "My Test Session");
}

#[test]
fn test_session_lifecycle_load_start_pause_resume_complete() {
    let mut tm = TrainingManager::new();
    let created = tm
        .create_session(create_session_req("Lifecycle Session"))
        .unwrap();
    let sess_id = created.id.clone();

    // CREATED → LOADED
    let loaded = tm
        .load_session_project(&sess_id, "proj-test-x", "scn-generic-feed-loss", None)
        .expect("load must succeed");
    assert_eq!(loaded.state, "LOADED");

    // LOADED → RUNNING
    let started = tm
        .start_session(&sess_id, 10.0)
        .expect("start must succeed");
    assert_eq!(started.state, "RUNNING");
    assert!(started.started_at_ms.is_some());

    // RUNNING → PAUSED
    let paused = tm
        .pause_session(&sess_id, 20.0)
        .expect("pause must succeed");
    assert_eq!(paused.state, "PAUSED");

    // PAUSED → RUNNING
    let resumed = tm
        .resume_session(&sess_id, 25.0)
        .expect("resume must succeed");
    assert_eq!(resumed.state, "RUNNING");

    // RUNNING → COMPLETED
    let report = tm
        .complete_session(&sess_id, 120.0)
        .expect("complete must succeed");
    assert_eq!(report.session_id, sess_id);
    assert!(report.max_possible_score == 0); // no criteria loaded
}

#[test]
fn test_session_abort_from_running_state() {
    let mut tm = TrainingManager::new();
    let created = tm
        .create_session(create_session_req("Abort Session"))
        .unwrap();
    let sess_id = created.id.clone();

    tm.load_session_project(&sess_id, "proj-test-x", "scn-generic-feed-loss", None)
        .unwrap();
    tm.start_session(&sess_id, 0.0).unwrap();

    let aborted = tm
        .abort_session(&sess_id, "Instructor terminated early", 55.0)
        .expect("abort must succeed");

    assert_eq!(aborted.state, "ABORTED");
    assert_eq!(
        aborted.abort_reason.as_deref(),
        Some("Instructor terminated early")
    );
}

#[test]
fn test_session_reset_clears_recorded_data() {
    let mut tm = TrainingManager::new();
    let created = tm
        .create_session(create_session_req("Reset Session"))
        .unwrap();
    let sess_id = created.id.clone();

    tm.load_session_project(&sess_id, "proj-test-x", "scn-generic-feed-loss", None)
        .unwrap();
    tm.start_session(&sess_id, 0.0).unwrap();

    // Ingest some data
    tm.ingest_tick(5.0, &[("TAG-A".to_string(), 42.0)]);

    // Reset
    let reset = tm.reset_session(&sess_id).expect("reset must succeed");
    assert_eq!(reset.state, "LOADED");
    assert!(reset.started_at_ms.is_none());
    assert_eq!(reset.current_simulation_time_sec, 0.0);
}

#[test]
fn test_trainee_action_recording_accepted() {
    use nebudcs_scenario::training::action::{ActionStatus, TraineeActionType};

    let mut tm = TrainingManager::new();
    let created = tm
        .create_session(create_session_req("Action Session"))
        .unwrap();
    let sess_id = created.id.clone();

    tm.load_session_project(&sess_id, "proj-test-x", "scn-generic-feed-loss", None)
        .unwrap();
    tm.start_session(&sess_id, 0.0).unwrap();

    let action = TraineeActionType::SetSetpoint {
        loop_name: "LIC-101".to_string(),
        sp: 55.0,
    };

    let dto = tm
        .record_trainee_action(
            &sess_id,
            "TRAINEE-B".to_string(),
            action,
            ActionStatus::Accepted,
            10.0,
        )
        .expect("action recording must succeed");

    assert_eq!(dto.status, "Accepted");
    assert_eq!(dto.requested_value, Some(55.0));
}

#[test]
fn test_action_recording_rejected_in_non_running_session() {
    use nebudcs_scenario::training::action::{ActionStatus, TraineeActionType};

    let mut tm = TrainingManager::new();
    let created = tm
        .create_session(create_session_req("Reject Test"))
        .unwrap();
    let sess_id = created.id.clone();

    // Session is in CREATED state — not RUNNING
    let result = tm.record_trainee_action(
        &sess_id,
        "T".to_string(),
        TraineeActionType::EmergencyStop,
        ActionStatus::Accepted,
        0.0,
    );

    assert!(
        result.is_err(),
        "Action recording in non-RUNNING session must fail"
    );
}

#[test]
fn test_replay_data_available_after_complete() {
    let mut tm = TrainingManager::new();
    let created = tm
        .create_session(create_session_req("Replay Session"))
        .unwrap();
    let sess_id = created.id.clone();

    tm.load_session_project(&sess_id, "proj-test-x", "scn-generic-feed-loss", None)
        .unwrap();
    tm.start_session(&sess_id, 0.0).unwrap();

    tm.ingest_tick(
        5.0,
        &[("TAG-A".to_string(), 100.0), ("TAG-B".to_string(), 50.0)],
    );
    tm.ingest_tick(
        10.0,
        &[("TAG-A".to_string(), 95.0), ("TAG-B".to_string(), 52.0)],
    );

    let _ = tm.complete_session(&sess_id, 10.0).unwrap();

    let summary = tm
        .get_replay_summary(&sess_id)
        .expect("replay summary must be available after completion");

    assert_eq!(summary.session_id, sess_id);
    assert!(summary.end_sim_time_sec >= 10.0);

    let frame = tm
        .get_replay_frame(&sess_id, 7.0)
        .expect("replay frame must be interpolatable");

    // TAG-A at sim_time=7 should be interpolated between 100 (t=5) and 95 (t=10)
    let tag_a = frame.tags.get("TAG-A").copied().unwrap_or(-1.0);
    assert!(
        tag_a > 94.0 && tag_a <= 100.0,
        "TAG-A interpolated value out of expected range: {tag_a}"
    );
}

#[test]
fn test_session_isolation_two_sessions_same_manager() {
    let mut tm = TrainingManager::new();

    let sess_a = tm.create_session(create_session_req("Session-A")).unwrap();
    let sess_b = tm.create_session(create_session_req("Session-B")).unwrap();

    tm.load_session_project(&sess_a.id, "proj-alpha", "scn-alpha", None)
        .unwrap();
    tm.load_session_project(&sess_b.id, "proj-beta", "scn-beta", None)
        .unwrap();

    let a = tm.get_session(&sess_a.id).unwrap();
    let b = tm.get_session(&sess_b.id).unwrap();

    assert_eq!(a.project_id, "proj-alpha");
    assert_eq!(b.project_id, "proj-beta");
    assert_ne!(a.id, b.id, "Session IDs must be distinct");
}

#[test]
fn test_tick_ingestion_updates_sim_time_across_running_sessions() {
    let mut tm = TrainingManager::new();

    let created = tm
        .create_session(create_session_req("Tick Session"))
        .unwrap();
    let id = created.id.clone();

    tm.load_session_project(&id, "proj-x", "scn-x", None)
        .unwrap();
    tm.start_session(&id, 0.0).unwrap();

    tm.ingest_tick(100.0, &[("PRESSURE".to_string(), 5.5)]);

    let detail = tm.get_session(&id).unwrap();
    assert!(
        detail.current_simulation_time_sec >= 100.0,
        "Sim time must be updated after ingest_tick"
    );
}

#[tokio::test]
async fn test_training_studio_rest_api_full_flow() {
    let config = ServerConfig {
        simulation_dt: 0.05,
        ..Default::default()
    };
    let mut engine = SimulationEngine::new(config.simulation_dt).expect("Failed to create engine");
    engine.clock_mut().play();
    let tag_store = RuntimeTagStore::new();

    let host = SimulationHost::start(config, engine, tag_store, None, None, None)
        .expect("Failed to start simulation host");
    let handle = host.handle();
    let app = build_app(handle);

    // 1. List scenarios
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/training/scenarios")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 2. Validate scenario against default project
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/training/scenarios/scn-generic-feed-loss/validate?project_id=proj-vertical-slice-01")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. Create session
    let create_payload = serde_json::json!({
        "name": "Surge Control Test Drill",
        "description": "Operator assessment",
        "instructor_id": "INST-01",
        "trainee_id": "TRAINEE-01",
        "project_id": "proj-vertical-slice-01",
        "scenario_id": "scn-generic-feed-loss"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/training/sessions")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&create_payload).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let session_dto: TrainingSessionDetailDto = serde_json::from_slice(&body_bytes).unwrap();
    let session_id = session_dto.id;

    // 4. Load project & scenario into session
    let load_payload = serde_json::json!({
        "project_id": "proj-vertical-slice-01",
        "scenario_id": "scn-generic-feed-loss"
    });
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/training/sessions/{}/load", session_id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&load_payload).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 5. Start session
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/training/sessions/{}/start", session_id))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 6. Trainee action: Permitted (Set Setpoint)
    let action_payload = serde_json::json!({
        "session_id": session_id,
        "actor": "TRAINEE-01",
        "target_entity": "LIC-101",
        "action_type": "SET_SP",
        "requested_value": 55.0
    });
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/training/sessions/{}/action", session_id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&action_payload).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 7. Trainee action: Forbidden administrative command (INJECT_FAULT) -> Rejected
    let forbidden_payload = serde_json::json!({
        "session_id": session_id,
        "actor": "TRAINEE-01",
        "target_entity": "K-101",
        "action_type": "INJECT_FAULT"
    });
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/training/sessions/{}/action", session_id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&forbidden_payload).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let action_res: TraineeActionDto = serde_json::from_slice(&body_bytes).unwrap();
    assert!(action_res.status.contains("Rejected"));

    // 8. Complete session & obtain evaluation
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/training/sessions/{}/complete", session_id))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 9. Query evaluation report
    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/v1/training/sessions/{}/evaluation",
            session_id
        ))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 10. Query replay summary
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/training/sessions/{}/replay", session_id))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 11. Query replay frame
    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/v1/training/sessions/{}/replay/frame?sim_time=0.0",
            session_id
        ))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
