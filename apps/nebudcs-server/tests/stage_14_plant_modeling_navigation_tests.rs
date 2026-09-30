//! Stage 14 Plant Modeling, Operational Mapping & Navigation Integration Tests.
//!
//! Verifies:
//! 1. Full plant structure persistence (Plant -> Area -> System -> Subsystem -> Unit)
//! 2. First-class display persistence (DisplayDto with multi-equipment membership)
//! 3. Cross-display navigation link persistence (NavigationLinkDto)
//! 4. Non-destructive physical vs operational mapping roundtrip
//! 5. Design-time display and navigation validation via REST API

use std::collections::BTreeMap;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use tower::ServiceExt;

use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_protocol::{
    BoundaryConditionDto, DisplayDto, DisplayElementPlacementDto, NavigationLinkDto, PlantAreaDto,
    PlantStructureDto, PlantSubsystemDto, PlantSystemDto, PlantUnitDto, Position2dDto,
    ProjectDetailDto, ProjectValidationResponseDto, SaveProjectRequestDto, StudioEquipmentDto,
    StudioStreamDto, ViewportConfigDto,
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
async fn test_plant_modeling_and_navigation_persistence_roundtrip() {
    let config = ServerConfig {
        simulation_dt: 0.05,
        auth_token: Some("plant_nav_token".to_string()),
        ..Default::default()
    };
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle);

    // 1. Construct Plant Structure
    let plant_structure = PlantStructureDto {
        plant_name: "Refinery & Petrochemical Complex Alpha".to_string(),
        areas: vec![PlantAreaDto {
            id: "area-100".to_string(),
            name: "Hydroprocessing Complex".to_string(),
            description: Some("High Pressure Reaction & Treatment".to_string()),
            systems: vec![PlantSystemDto {
                id: "sys-101".to_string(),
                name: "Reactor Loop System".to_string(),
                area_id: Some("area-100".to_string()),
                default_display_id: Some("disp-sys-101".to_string()),
                description: Some("Reaction loop and separation".to_string()),
                subsystems: vec![PlantSubsystemDto {
                    id: "subsys-101a".to_string(),
                    name: "Recycle Gas Subsystem".to_string(),
                    system_id: Some("sys-101".to_string()),
                    description: None,
                    units: vec![PlantUnitDto {
                        id: "unit-101a".to_string(),
                        name: "Compression Unit".to_string(),
                        subsystem_id: Some("subsys-101a".to_string()),
                        system_id: Some("sys-101".to_string()),
                        area_id: Some("area-100".to_string()),
                        equipment_ids: vec!["eq-c101".to_string()],
                        display_ids: vec!["disp-c101".to_string()],
                        description: Some("Recycle compressor unit".to_string()),
                    }],
                }],
            }],
        }],
        displays: vec![],
    };

    // 2. Construct Displays
    let displays = vec![
        DisplayDto {
            id: "disp-plant".to_string(),
            name: "Plant Overview".to_string(),
            display_type: "plant_overview".to_string(),
            area_id: None,
            system_id: None,
            subsystem_id: None,
            unit_id: None,
            equipment_ids: vec!["eq-c101".to_string(), "eq-v101".to_string()],
            instrument_ids: vec![],
            stream_ids: vec!["st-01".to_string()],
            viewport: Some(ViewportConfigDto {
                x: 0.0,
                y: 0.0,
                width: 1920.0,
                height: 1080.0,
                zoom: 0.85,
            }),
            is_default: true,
            order: Some(1),
            description: Some("Global plant overview".to_string()),
            placements: vec![],
        },
        DisplayDto {
            id: "disp-sys-101".to_string(),
            name: "Reactor Loop Overview".to_string(),
            display_type: "system_overview".to_string(),
            area_id: Some("area-100".to_string()),
            system_id: Some("sys-101".to_string()),
            subsystem_id: None,
            unit_id: None,
            equipment_ids: vec!["eq-c101".to_string(), "eq-v101".to_string()],
            instrument_ids: vec![],
            stream_ids: vec!["st-01".to_string()],
            viewport: Some(ViewportConfigDto {
                x: 100.0,
                y: 50.0,
                width: 1920.0,
                height: 1080.0,
                zoom: 1.0,
            }),
            is_default: false,
            order: Some(2),
            description: Some("System-level graphic".to_string()),
            placements: vec![],
        },
        DisplayDto {
            id: "disp-c101".to_string(),
            name: "C-101 Compressor Detail".to_string(),
            display_type: "unit_process_graphic".to_string(),
            area_id: Some("area-100".to_string()),
            system_id: Some("sys-101".to_string()),
            subsystem_id: Some("subsys-101a".to_string()),
            unit_id: Some("unit-101a".to_string()),
            equipment_ids: vec!["eq-c101".to_string()],
            instrument_ids: vec![],
            stream_ids: vec!["st-01".to_string()],
            viewport: Some(ViewportConfigDto {
                x: 50.0,
                y: 50.0,
                width: 1920.0,
                height: 1080.0,
                zoom: 1.25,
            }),
            is_default: false,
            order: Some(3),
            description: Some("Recycle compressor unit graphic".to_string()),
            placements: vec![DisplayElementPlacementDto {
                equipment_id: "eq-c101".to_string(),
                x: 250.0,
                y: 350.0,
                width: Some(120.0),
                height: Some(90.0),
                rotation: Some(0.0),
            }],
        },
    ];

    // 3. Construct Navigation Links
    let navigation_links = vec![
        NavigationLinkDto {
            id: "nav-c101-to-sys".to_string(),
            source_display_id: "disp-c101".to_string(),
            source_element_id: Some("eq-c101".to_string()),
            target_display_id: "disp-sys-101".to_string(),
            target_element_id: Some("eq-v101".to_string()),
            label: "To HP Separator V-101".to_string(),
            behavior: "focus_and_center".to_string(),
            description: Some("Downstream process continuation link".to_string()),
        },
    ];

    // 4. Construct Physical Equipment & Streams
    let equipment = vec![
        StudioEquipmentDto {
            id: "eq-c101".to_string(),
            definition_id: "standard.compressor.centrifugal".to_string(),
            definition_version: "1.0.0".to_string(),
            name: "C-101".to_string(),
            parameters: BTreeMap::new(),
            position: Some(Position2dDto { x: 100.0, y: 150.0 }),
            display_width: None,
            display_height: None,
            rotation: None,
        },
        StudioEquipmentDto {
            id: "eq-v101".to_string(),
            definition_id: "standard.vessel.vertical_separator".to_string(),
            definition_version: "1.0.0".to_string(),
            name: "V-101".to_string(),
            parameters: BTreeMap::new(),
            position: Some(Position2dDto { x: 400.0, y: 150.0 }),
            display_width: None,
            display_height: None,
            rotation: None,
        },
    ];

    let streams = vec![
        StudioStreamDto {
            id: "st-01".to_string(),
            name: "Recycle Discharge".to_string(),
            stream_type: "material".to_string(),
            source_equipment_id: "eq-c101".to_string(),
            source_port_id: "discharge".to_string(),
            sink_equipment_id: "eq-v101".to_string(),
            sink_port_id: "inlet".to_string(),
            composition: BTreeMap::new(),
            route_points: vec![],
            diameter: None,
            length: None,
            roughness: None,
            elevation_change: None,
            loss_k: None,
        },
    ];

    // 5. Create Project via REST API
    let create_req = nebudcs_protocol::CreateProjectRequestDto {
        name: "Alpha Hydroprocessing Model".to_string(),
        description: Some("Stage 14 Full Plant Model acceptance fixture".to_string()),
        author: Some("Automated Verification Test".to_string()),
    };

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/projects")
        .header(header::AUTHORIZATION, "Bearer plant_nav_token")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&create_req).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let created: ProjectDetailDto = serde_json::from_slice(&body).unwrap();
    let proj_id = created.id.clone();

    // 6. Save Project with Plant Structure, Displays, and Navigation Links
    let save_payload = SaveProjectRequestDto {
        project: ProjectDetailDto {
            id: proj_id.clone(),
            name: "Alpha Hydroprocessing Model".to_string(),
            schema_version: "0.1.0".to_string(),
            description: "Stage 14 Full Plant Model acceptance fixture".to_string(),
            author: "Automated Verification Test".to_string(),
            created: created.created.clone(),
            modified: created.modified.clone(),
            equipment,
            streams,
            instruments: vec![],
            control_loops: vec![],
            alarms: vec![],
            interlocks: vec![],
            controllers: vec![],
            io_systems: vec![],
            io_mappings: vec![],
            tags: vec![],
            advanced_control: vec![],
            communications: None,
            sis: None,
            boundary_conditions: vec![BoundaryConditionDto::feed(
                "bc-alpha-feed-01",
                "Feed Stream",
                "eq-feed-surge-v101",
                50.0,
                "m3/h",
            )],
            initial_conditions: vec![],
            plant_structure: Some(plant_structure.clone()),
            areas: vec![],
            displays: displays.clone(),
            navigation_links: navigation_links.clone(),
        },
    };

    let req = Request::builder()
        .method("PUT")
        .uri(format!("/api/v1/projects/{}", proj_id))
        .header(header::AUTHORIZATION, "Bearer plant_nav_token")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&save_payload).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 6. Fetch Project and Verify Persistence Integrity
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/projects/{}", proj_id))
        .header(header::AUTHORIZATION, "Bearer plant_nav_token")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let loaded: ProjectDetailDto = serde_json::from_slice(&body).unwrap();

    // Verify plant structure
    let loaded_plant = loaded.plant_structure.expect("plant_structure must exist");
    assert_eq!(loaded_plant.plant_name, "Refinery & Petrochemical Complex Alpha");
    assert_eq!(loaded_plant.areas.len(), 1);
    assert_eq!(loaded_plant.areas[0].systems.len(), 1);
    assert_eq!(loaded_plant.areas[0].systems[0].subsystems[0].units.len(), 1);
    let unit = &loaded_plant.areas[0].systems[0].subsystems[0].units[0];
    assert_eq!(unit.equipment_ids, vec!["eq-c101".to_string()]);

    // Verify displays
    let loaded_displays = &loaded.displays;
    assert_eq!(loaded_displays.len(), 3);
    assert_eq!(loaded_displays[0].id, "disp-plant");
    assert!(loaded_displays[0].is_default);
    assert_eq!(loaded_displays[2].id, "disp-c101");
    assert_eq!(loaded_displays[2].unit_id, Some("unit-101a".to_string()));
    assert_eq!(loaded_displays[2].placements.len(), 1);
    assert_eq!(loaded_displays[2].placements[0].equipment_id, "eq-c101");
    assert_eq!(loaded_displays[2].placements[0].x, 250.0);
    assert_eq!(loaded_displays[2].placements[0].y, 350.0);

    // Verify navigation links
    let loaded_nav = &loaded.navigation_links;
    assert_eq!(loaded_nav.len(), 1);
    assert_eq!(loaded_nav[0].id, "nav-c101-to-sys");
    assert_eq!(loaded_nav[0].source_display_id, "disp-c101");
    assert_eq!(loaded_nav[0].target_display_id, "disp-sys-101");
    assert_eq!(loaded_nav[0].target_element_id, Some("eq-v101".to_string()));

    // 7. Validate Project via REST API
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/projects/{}/validate", proj_id))
        .header(header::AUTHORIZATION, "Bearer plant_nav_token")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let validation: ProjectValidationResponseDto = serde_json::from_slice(&body).unwrap();
    
    // There should be no VLD-DSP or VLD-NAV errors for our properly wired project
    let nav_errors: Vec<_> = validation
        .results
        .iter()
        .filter(|r| r.code.starts_with("VLD-DSP") || r.code.starts_with("VLD-NAV"))
        .collect();
    assert!(
        nav_errors.is_empty(),
        "Expected no navigation or display validation errors, got: {:?}",
        nav_errors
    );
}
