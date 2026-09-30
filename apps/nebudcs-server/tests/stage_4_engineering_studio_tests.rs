//! Stage 4 PLC/DCS Engineering Studio integration tests.

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use tower::ServiceExt;

use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_protocol::*;
use nebudcs_server::{build_app, HostHandle, ServerConfig, SimulationHost};
use nebudcs_sim::SimulationEngine;

fn create_test_host() -> (SimulationHost, HostHandle) {
    let temp_dir = std::env::temp_dir().join(format!(
        "nebudcs_stage4_test_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let config = ServerConfig {
        simulation_dt: 0.05,
        admin_token: Some("test_admin".to_string()),
        operator_token: Some("test_operator".to_string()),
        trainee_token: Some("test_trainee".to_string()),
        data_dir: temp_dir,
        ..Default::default()
    };
    let mut engine = SimulationEngine::new(config.simulation_dt).expect("Failed to create engine");
    engine.clock_mut().play();
    let tag_store = RuntimeTagStore::new();

    let host = SimulationHost::start(config, engine, tag_store, None, None, None)
        .expect("Failed to start simulation host");

    let handle = host.handle();
    (host, handle)
}

#[tokio::test]
async fn test_default_project_has_stage_4_hierarchy() {
    let (_host, handle) = create_test_host();
    let app = build_app(handle);

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/projects/proj-vertical-slice-01")
        .header(header::AUTHORIZATION, "Bearer test_operator")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let proj: ProjectDetailDto = serde_json::from_slice(&body).unwrap();

    assert_eq!(proj.id, "proj-vertical-slice-01");
    // Verify controllers
    assert_eq!(
        proj.controllers.len(),
        1,
        "Should have 1 default controller"
    );
    let ctrl = &proj.controllers[0];
    assert_eq!(ctrl.id, "C-101");
    assert_eq!(ctrl.tasks.len(), 1);
    assert_eq!(ctrl.tasks[0].id, "FastLogic");
    assert_eq!(ctrl.tasks[0].programs.len(), 1);
    assert_eq!(ctrl.tasks[0].programs[0].routines.len(), 1);
    assert_eq!(ctrl.tasks[0].programs[0].routines[0].language, "ST");

    // Verify HAL systems
    assert_eq!(proj.io_systems.len(), 1, "Should have 1 default HAL system");
    let hal = &proj.io_systems[0];
    assert_eq!(hal.id, "HAL-MAIN");
    assert_eq!(hal.racks.len(), 1);
    assert_eq!(hal.racks[0].chassis.len(), 1);
    assert_eq!(hal.racks[0].chassis[0].modules.len(), 4);

    // Verify mappings
    assert!(
        proj.io_mappings.len() >= 2,
        "Should have input and output mappings"
    );

    // Verify tags
    assert!(proj.tags.len() >= 4, "Should have canonical project tags");
}

#[tokio::test]
async fn test_cross_reference_endpoint() {
    let (_host, handle) = create_test_host();
    let app = build_app(handle);

    // 1. Search for specific tag
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/projects/proj-vertical-slice-01/cross-reference?tag_or_entity=V-101.LEVEL")
        .header(header::AUTHORIZATION, "Bearer test_operator")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let xref: CrossReferenceResultDto = serde_json::from_slice(&body).unwrap();

    assert_eq!(xref.target, "V-101.LEVEL");
    assert!(
        !xref.entries.is_empty(),
        "Should find cross-references for V-101.LEVEL"
    );
    let has_tag = xref.entries.iter().any(|e| e.entity_type == "Tag");
    let has_loop = xref.entries.iter().any(|e| e.entity_type == "ControlLoop");
    let has_alarm = xref.entries.iter().any(|e| e.entity_type == "Alarm");
    let has_mapping = xref
        .entries
        .iter()
        .any(|e| e.entity_type == "ChannelMapping");
    assert!(has_tag, "Should contain Tag reference");
    assert!(has_loop, "Should contain ControlLoop reference");
    assert!(has_alarm, "Should contain Alarm reference");
    assert!(has_mapping, "Should contain ChannelMapping reference");

    // 2. Wildcard search returns all entities
    let req2 = Request::builder()
        .method("GET")
        .uri("/api/v1/projects/proj-vertical-slice-01/cross-reference?tag_or_entity=*")
        .header(header::AUTHORIZATION, "Bearer test_operator")
        .body(Body::empty())
        .unwrap();

    let resp2 = app.oneshot(req2).await.unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);
    let body2 = axum::body::to_bytes(resp2.into_body(), usize::MAX)
        .await
        .unwrap();
    let xref_all: CrossReferenceResultDto = serde_json::from_slice(&body2).unwrap();
    assert!(
        xref_all.entries.len() >= 10,
        "Wildcard should return comprehensive entries"
    );
}

#[tokio::test]
async fn test_project_validation_catches_invalid_task_priority() {
    let (_host, handle) = create_test_host();
    let app = build_app(handle);

    // Get default project
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/projects/proj-vertical-slice-01")
        .header(header::AUTHORIZATION, "Bearer test_operator")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let mut proj: ProjectDetailDto = serde_json::from_slice(
        &axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();

    // Mutate task priority to invalid value (e.g. 0 or 20)
    proj.controllers[0].tasks[0].priority = 20;

    // Save project
    let save_req = Request::builder()
        .method("PUT")
        .uri("/api/v1/projects/proj-vertical-slice-01")
        .header(header::AUTHORIZATION, "Bearer test_admin")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_vec(&SaveProjectRequestDto { project: proj }).unwrap(),
        ))
        .unwrap();
    let save_resp = app.clone().oneshot(save_req).await.unwrap();
    assert_eq!(save_resp.status(), StatusCode::OK);

    // Validate project
    let val_req = Request::builder()
        .method("POST")
        .uri("/api/v1/projects/proj-vertical-slice-01/validate")
        .header(header::AUTHORIZATION, "Bearer test_operator")
        .body(Body::empty())
        .unwrap();
    let val_resp = app.oneshot(val_req).await.unwrap();
    assert_eq!(val_resp.status(), StatusCode::OK);
    let val_res: ProjectValidationResponseDto = serde_json::from_slice(
        &axum::body::to_bytes(val_resp.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();

    assert!(
        !val_res.valid,
        "Project validation should fail for invalid priority"
    );
    assert!(val_res
        .results
        .iter()
        .any(|r| r.code == "VAL_TASK_PRIORITY_RANGE"));
}

#[tokio::test]
async fn test_project_validation_catches_iec_syntax_error() {
    let (_host, handle) = create_test_host();
    let app = build_app(handle);

    // Get default project
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/projects/proj-vertical-slice-01")
        .header(header::AUTHORIZATION, "Bearer test_operator")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let mut proj: ProjectDetailDto = serde_json::from_slice(
        &axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();

    // Mutate ST routine with invalid syntax
    proj.controllers[0].tasks[0].programs[0].routines[0].source_text =
        "VAR X: REAL; END_VAR IF THEN".to_string();

    // Save project
    let save_req = Request::builder()
        .method("PUT")
        .uri("/api/v1/projects/proj-vertical-slice-01")
        .header(header::AUTHORIZATION, "Bearer test_admin")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_vec(&SaveProjectRequestDto { project: proj }).unwrap(),
        ))
        .unwrap();
    let _ = app.clone().oneshot(save_req).await.unwrap();

    // Validate project
    let val_req = Request::builder()
        .method("POST")
        .uri("/api/v1/projects/proj-vertical-slice-01/validate")
        .header(header::AUTHORIZATION, "Bearer test_operator")
        .body(Body::empty())
        .unwrap();
    let val_resp = app.oneshot(val_req).await.unwrap();
    assert_eq!(val_resp.status(), StatusCode::OK);
    let val_res: ProjectValidationResponseDto = serde_json::from_slice(
        &axum::body::to_bytes(val_resp.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();

    assert!(
        !val_res.valid,
        "Project validation should fail for invalid IEC routine syntax"
    );
    assert!(val_res
        .results
        .iter()
        .any(|r| r.code == "VAL_IEC_COMPILE_ERROR"));
}

#[tokio::test]
async fn test_project_validation_catches_hal_direction_mismatch() {
    let (_host, handle) = create_test_host();
    let app = build_app(handle);

    // Get default project
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/projects/proj-vertical-slice-01")
        .header(header::AUTHORIZATION, "Bearer test_operator")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let mut proj: ProjectDetailDto = serde_json::from_slice(
        &axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();

    // Add incompatible mapping: direction input targeting AO channel
    proj.io_mappings.push(HalMappingDto {
        controller_id: "C-101".to_string(),
        signal_key: "BAD_SIG".to_string(),
        channel_address: "R01-C01-M02-AO02".to_string(), // AO channel
        direction: "input".to_string(),                  // mismatch!
        target_tag: None,
        target_actuator: None,
        priority: Some(5),
    });

    // Save project
    let save_req = Request::builder()
        .method("PUT")
        .uri("/api/v1/projects/proj-vertical-slice-01")
        .header(header::AUTHORIZATION, "Bearer test_admin")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_vec(&SaveProjectRequestDto { project: proj }).unwrap(),
        ))
        .unwrap();
    let _ = app.clone().oneshot(save_req).await.unwrap();

    // Validate project
    let val_req = Request::builder()
        .method("POST")
        .uri("/api/v1/projects/proj-vertical-slice-01/validate")
        .header(header::AUTHORIZATION, "Bearer test_operator")
        .body(Body::empty())
        .unwrap();
    let val_resp = app.oneshot(val_req).await.unwrap();
    assert_eq!(val_resp.status(), StatusCode::OK);
    let val_res: ProjectValidationResponseDto = serde_json::from_slice(
        &axum::body::to_bytes(val_resp.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();

    assert!(
        !val_res.valid,
        "Project validation should fail for direction mismatch"
    );
    assert!(val_res
        .results
        .iter()
        .any(|r| r.code == "VAL_HAL_DIRECTION_MISMATCH"));
}

#[tokio::test]
async fn test_load_sim_deploys_controllers_and_hal() {
    let (_host, handle) = create_test_host();
    let app = build_app(handle);

    // 1. Trigger load-sim for default project
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/projects/proj-vertical-slice-01/load-sim")
        .header(header::AUTHORIZATION, "Bearer test_admin")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 2. Query virtual controllers
    let ctrl_req = Request::builder()
        .method("GET")
        .uri("/api/v1/controllers")
        .header(header::AUTHORIZATION, "Bearer test_operator")
        .body(Body::empty())
        .unwrap();
    let ctrl_resp = app.clone().oneshot(ctrl_req).await.unwrap();
    assert_eq!(ctrl_resp.status(), StatusCode::OK);
    let ctrl_body = axum::body::to_bytes(ctrl_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let ctrls: Vec<ControllerSummaryDto> = serde_json::from_slice(&ctrl_body).unwrap();
    assert!(
        !ctrls.is_empty(),
        "Virtual controllers should be instantiated from project"
    );
    let c101 = ctrls.iter().find(|c| c.name == "Primary DCS Controller");
    assert!(c101.is_some(), "C-101 should be deployed");

    // 3. Query HAL systems
    let hal_req = Request::builder()
        .method("GET")
        .uri("/api/v1/hal/systems")
        .header(header::AUTHORIZATION, "Bearer test_operator")
        .body(Body::empty())
        .unwrap();
    let hal_resp = app.oneshot(hal_req).await.unwrap();
    assert_eq!(hal_resp.status(), StatusCode::OK);
    let hal_body = axum::body::to_bytes(hal_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let hal_systems: Vec<HalSystemSummaryDto> = serde_json::from_slice(&hal_body).unwrap();
    assert!(
        !hal_systems.is_empty(),
        "HAL systems should be instantiated from project"
    );
    assert_eq!(hal_systems[0].name, "Main Process I/O Subsystem");
}
