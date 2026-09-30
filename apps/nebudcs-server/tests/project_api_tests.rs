//! End-to-end integration tests for NebuDCS Phase 13 Project & Engineering Studio REST APIs.

use std::collections::BTreeMap;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use tower::ServiceExt;

use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_protocol::{
    CreateProjectRequestDto, Position2dDto, ProjectDetailDto, ProjectSummaryDto,
    ProjectValidationResponseDto, SaveProjectRequestDto, StudioEquipmentDto, StudioStreamDto,
};
use nebudcs_server::{build_app, HostHandle, ServerConfig, SimulationHost};
use nebudcs_sim::SimulationEngine;

fn create_test_host(config: ServerConfig) -> (SimulationHost, HostHandle) {
    let mut engine = SimulationEngine::new(config.simulation_dt).expect("Failed to create engine");
    engine.clock_mut().play();
    let tag_store = RuntimeTagStore::new();

    let host = SimulationHost::start(config, engine, tag_store, None, None, None)
        .expect("Failed to start simulation host");

    let handle = host.handle();
    (host, handle)
}

#[tokio::test]
async fn test_project_list_and_get_default() {
    let config = ServerConfig {
        simulation_dt: 0.05,
        auth_token: Some("studio_secret".to_string()),
        ..Default::default()
    };
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle);

    // 1. List projects
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/projects")
        .header(header::AUTHORIZATION, "Bearer studio_secret")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let projects: Vec<ProjectSummaryDto> = serde_json::from_slice(&body).unwrap();
    assert!(
        !projects.is_empty(),
        "Should contain at least default project"
    );
    let default_proj = projects
        .iter()
        .find(|p| p.id == "proj-vertical-slice-01")
        .expect("Default project should exist in project list");
    assert_eq!(default_proj.id, "proj-vertical-slice-01");
    assert_eq!(default_proj.equipment_count, 3);
    assert_eq!(default_proj.stream_count, 2);
    assert_eq!(default_proj.plant_count, 1);

    // 2. Get default project details
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/projects/proj-vertical-slice-01")
        .header(header::AUTHORIZATION, "Bearer studio_secret")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let detail: ProjectDetailDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(detail.id, "proj-vertical-slice-01");
    assert_eq!(detail.equipment.len(), 3);
    assert_eq!(detail.streams.len(), 2);
    assert_eq!(detail.instruments.len(), 4);
    assert_eq!(detail.control_loops.len(), 1);
    assert_eq!(detail.alarms.len(), 2);
    assert_eq!(detail.interlocks.len(), 1);

    // Verify stream route points exist for P&ID orthogonal rendering
    assert_eq!(detail.streams[0].route_points.len(), 3);
}

#[tokio::test]
async fn test_project_create_save_and_retrieve() {
    let config = ServerConfig {
        simulation_dt: 0.05,
        ..Default::default()
    };
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle);

    // 1. Create a new project
    let create_req = CreateProjectRequestDto {
        name: "Ammonia Synthesis Loop".to_string(),
        description: Some("High-pressure Haber-Bosch reaction and recycle loop".to_string()),
        author: Some("Chemical Process Engineer".to_string()),
    };
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/projects")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&create_req).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let mut created: ProjectDetailDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(created.name, "Ammonia Synthesis Loop");
    let proj_id = created.id.clone();

    // 2. Add equipment and a stream to project and save
    let mut eq_params = BTreeMap::new();
    eq_params.insert("power_kw".to_string(), 250.0);

    let eq_id = "00000000-0000-0000-0000-000000000001".to_string();
    let comp_id = "00000000-0000-0000-0000-000000000002".to_string();

    created.equipment.push(StudioEquipmentDto {
        id: eq_id.clone(),
        definition_id: "standard.pump.centrifugal".to_string(),
        definition_version: "1.0.0".to_string(),
        name: "P-201".to_string(),
        parameters: eq_params,
        position: Some(Position2dDto { x: 150.0, y: 300.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    created.equipment.push(StudioEquipmentDto {
        id: comp_id.clone(),
        definition_id: "standard.compressor.centrifugal".to_string(),
        definition_version: "1.0.0".to_string(),
        name: "C-201".to_string(),
        parameters: BTreeMap::new(),
        position: Some(Position2dDto { x: 450.0, y: 300.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    created.streams.push(StudioStreamDto {
        id: "00000000-0000-0000-0000-000000000003".to_string(),
        name: "S-201".to_string(),
        stream_type: "Material".to_string(),
        source_equipment_id: eq_id,
        source_port_id: "discharge".to_string(),
        sink_equipment_id: comp_id,
        sink_port_id: "suction".to_string(),
        composition: BTreeMap::new(),
        route_points: vec![
            Position2dDto { x: 150.0, y: 300.0 },
            Position2dDto { x: 450.0, y: 300.0 },
        ],
        ..Default::default()
    });

    let save_req = SaveProjectRequestDto {
        project: created.clone(),
    };

    let req = Request::builder()
        .method("PUT")
        .uri(format!("/api/v1/projects/{proj_id}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&save_req).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. Re-fetch project and confirm persistence
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/projects/{proj_id}"))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let re_fetched: ProjectDetailDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(re_fetched.equipment.len(), 2);
    assert_eq!(re_fetched.streams.len(), 1);
    assert_eq!(re_fetched.streams[0].name, "S-201");
}

#[tokio::test]
async fn test_project_validation_endpoint() {
    let config = ServerConfig {
        simulation_dt: 0.05,
        ..Default::default()
    };
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle);

    // Validate default project
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/projects/proj-vertical-slice-01/validate")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let val_resp: ProjectValidationResponseDto = serde_json::from_slice(&body).unwrap();
    assert!(
        val_resp.valid,
        "Baseline project should be structurally valid"
    );
    assert_eq!(val_resp.error_count, 0);
}

#[tokio::test]
async fn test_project_load_simulation_endpoint() {
    let config = ServerConfig {
        simulation_dt: 0.05,
        ..Default::default()
    };
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle);

    // Synchronize default project into simulation host
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/projects/proj-vertical-slice-01/load-sim")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["status"], "ok");
    assert_eq!(v["project_id"], "proj-vertical-slice-01");
}

#[tokio::test]
async fn test_project_security_and_not_found() {
    let config = ServerConfig {
        simulation_dt: 0.05,
        auth_token: Some("required_token".to_string()),
        ..Default::default()
    };
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle);

    // Unauthorized request without token
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/projects")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Request non-existent project
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/projects/non-existent-proj-id")
        .header(header::AUTHORIZATION, "Bearer required_token")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_end_to_end_studio_lifecycle_workflow() {
    // 17-Step Vertical Slice Proof:
    // UI Create -> Configure -> Canvas Place -> Wire Streams -> Instruments -> Control -> Protection ->
    // Validate -> Save -> Reload -> Re-Validate -> Load Simulation -> Advance Sim -> Verify Separation
    let config = ServerConfig {
        simulation_dt: 0.05,
        ..Default::default()
    };
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle);

    // Step 1: Create project
    let create_req = CreateProjectRequestDto {
        name: "Alpha Cracking Train".to_string(),
        description: Some("Full end-to-end vertical slice test flowsheet".to_string()),
        author: Some("Lead Verification Engineer".to_string()),
    };
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/projects")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&create_req).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let mut project: ProjectDetailDto = serde_json::from_slice(&body).unwrap();
    let proj_id = project.id.clone();
    assert_eq!(project.name, "Alpha Cracking Train");

    // Steps 2-4: Add equipment, configure parameters, and place on canvas
    let eq_pump_id = "00000000-0000-0000-0001-000000000001".to_string();
    let eq_valve_id = "00000000-0000-0000-0001-000000000002".to_string();
    let eq_vessel_id = "00000000-0000-0000-0001-000000000003".to_string();

    let mut pump_params = BTreeMap::new();
    pump_params.insert("design_flow".to_string(), 4500.0);
    pump_params.insert("rated_head".to_string(), 110.0);
    project.equipment.push(StudioEquipmentDto {
        id: eq_pump_id.clone(),
        definition_id: "standard.pump.centrifugal".to_string(),
        definition_version: "1.0.0".to_string(),
        name: "P-101".to_string(),
        parameters: pump_params,
        position: Some(Position2dDto { x: 120.0, y: 200.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    let mut valve_params = BTreeMap::new();
    valve_params.insert("cv".to_string(), 65.0);
    valve_params.insert("position".to_string(), 50.0);
    project.equipment.push(StudioEquipmentDto {
        id: eq_valve_id.clone(),
        definition_id: "standard.valve.control_globe".to_string(),
        definition_version: "1.0.0".to_string(),
        name: "FV-101".to_string(),
        parameters: valve_params,
        position: Some(Position2dDto { x: 300.0, y: 200.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    let mut vessel_params = BTreeMap::new();
    vessel_params.insert("volume".to_string(), 18.0);
    project.equipment.push(StudioEquipmentDto {
        id: eq_vessel_id.clone(),
        definition_id: "standard.vessel.vertical_separator".to_string(),
        definition_version: "1.0.0".to_string(),
        name: "V-101".to_string(),
        parameters: vessel_params,
        position: Some(Position2dDto { x: 500.0, y: 200.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Steps 5-6: Create streams, connect ports, and define orthogonal Manhattan route points
    project.streams.push(StudioStreamDto {
        id: "00000000-0000-0000-0002-000000000001".to_string(),
        name: "S-101".to_string(),
        stream_type: "Material".to_string(),
        source_equipment_id: eq_pump_id.clone(),
        source_port_id: "discharge".to_string(),
        sink_equipment_id: eq_valve_id.clone(),
        sink_port_id: "inlet".to_string(),
        composition: BTreeMap::new(),
        route_points: vec![
            Position2dDto { x: 120.0, y: 200.0 },
            Position2dDto { x: 300.0, y: 200.0 },
        ],
        ..Default::default()
    });

    project.streams.push(StudioStreamDto {
        id: "00000000-0000-0000-0002-000000000002".to_string(),
        name: "S-102".to_string(),
        stream_type: "Material".to_string(),
        source_equipment_id: eq_valve_id.clone(),
        source_port_id: "outlet".to_string(),
        sink_equipment_id: eq_vessel_id.clone(),
        sink_port_id: "feed".to_string(),
        composition: BTreeMap::new(),
        route_points: vec![
            Position2dDto { x: 300.0, y: 200.0 },
            Position2dDto { x: 500.0, y: 200.0 },
        ],
        ..Default::default()
    });

    // Step 7: Configure field instruments
    project
        .instruments
        .push(nebudcs_protocol::StudioInstrumentDto {
            id: "00000000-0000-0000-0003-000000000001".to_string(),
            tag: "PT-101".to_string(),
            equipment_id: Some(eq_pump_id.clone()),
            port_id: Some("discharge".to_string()),
            measurement_kind: "Pressure".to_string(),
            range_min: 0.0,
            range_max: 30.0,
            unit: "bar".to_string(),
            position: Some(Position2dDto { x: 180.0, y: 120.0 }),
        });
    project
        .instruments
        .push(nebudcs_protocol::StudioInstrumentDto {
            id: "00000000-0000-0000-0003-000000000002".to_string(),
            tag: "LT-101".to_string(),
            equipment_id: Some(eq_vessel_id.clone()),
            port_id: None,
            measurement_kind: "Level".to_string(),
            range_min: 0.0,
            range_max: 100.0,
            unit: "%".to_string(),
            position: Some(Position2dDto { x: 500.0, y: 120.0 }),
        });

    // Step 8: Configure control loop
    project
        .control_loops
        .push(nebudcs_protocol::StudioControlLoopDto {
            id: "00000000-0000-0000-0004-000000000001".to_string(),
            name: "LIC-101".to_string(),
            pv_tag: "V-101.LEVEL".to_string(),
            sp: 55.0,
            out_tag: "FV-101.POS".to_string(),
            kp: 1.8,
            ti: 25.0,
            td: 1.0,
            reverse_acting: false,
            mode: "AUTO".to_string(),
            output_min: 0.0,
            output_max: 100.0,
            position: Some(Position2dDto { x: 400.0, y: 80.0 }),
        });

    // Step 9: Configure alarms and safety interlocks
    project.alarms.push(nebudcs_protocol::StudioAlarmDto {
        id: "00000000-0000-0000-0005-000000000001".to_string(),
        name: "LAH-101".to_string(),
        pv_tag: "V-101.LEVEL".to_string(),
        alarm_type: "Hi".to_string(),
        severity: "Critical".to_string(),
        trip_limit: 85.0,
        deadband: 1.0,
        description: "Vessel high level alarm".to_string(),
    });

    project
        .interlocks
        .push(nebudcs_protocol::StudioInterlockDto {
            id: "00000000-0000-0000-0006-000000000001".to_string(),
            name: "TRIP-K101".to_string(),
            source_tag: "V-101.LEVEL".to_string(),
            condition: "<".to_string(),
            threshold: 8.0,
            target_action: "TRIP".to_string(),
            target_equipment: Some(eq_pump_id.clone()),
            latched: true,
            description: "Low level pump dry-run trip".to_string(),
        });

    // Step 10 & 11: Save project to backend
    let save_req = SaveProjectRequestDto {
        project: project.clone(),
    };
    let req = Request::builder()
        .method("PUT")
        .uri(format!("/api/v1/projects/{proj_id}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&save_req).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Step 12: Validate project via backend engine
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/projects/{proj_id}/validate"))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let val_resp: ProjectValidationResponseDto = serde_json::from_slice(&body).unwrap();
    assert!(
        val_resp.valid,
        "Flowsheet must pass validation: {:?}",
        val_resp.results
    );
    assert_eq!(val_resp.error_count, 0);

    // Step 13: Reload project from persistence
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/projects/{proj_id}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let reloaded: ProjectDetailDto = serde_json::from_slice(&body).unwrap();

    // Verify complete fidelity across reload
    assert_eq!(reloaded.id, proj_id);
    assert_eq!(reloaded.equipment.len(), 3);
    assert_eq!(reloaded.streams.len(), 2);
    assert_eq!(reloaded.instruments.len(), 2);
    assert_eq!(reloaded.control_loops.len(), 1);
    assert_eq!(reloaded.alarms.len(), 1);
    assert_eq!(reloaded.interlocks.len(), 1);

    // Verify graphical layout coordinates survived
    assert_eq!(
        reloaded.equipment[0].position,
        Some(Position2dDto { x: 120.0, y: 200.0 })
    );
    assert_eq!(reloaded.streams[0].route_points.len(), 2);

    // Step 14: Re-validate reloaded model
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/projects/{proj_id}/validate"))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Step 15: Load simulation from authored project
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/projects/{proj_id}/load-sim"))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let sim_resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(sim_resp["status"], "ok");

    // Step 16 & 17: Verify engineering configuration remains completely intact
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/projects/{proj_id}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let final_check: ProjectDetailDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(final_check.control_loops[0].sp, 55.0);
    assert_eq!(final_check.control_loops[0].kp, 1.8);
    assert_eq!(
        final_check.equipment[0].parameters.get("design_flow"),
        Some(&4500.0)
    );
}
