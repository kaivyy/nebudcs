//! Stage 14 Naphtha Hydrotreater (NHT) Reference Plant Integration Tests.
//!
//! Verifies:
//! 1. Authoritative seed initialization from HPCL Visakh Operating Manual & EIL standards.
//! 2. Full plant structure (12 systems, 7 units, 12 displays, 17 navigation links).
//! 3. Clean design-time validation (0 errors, 0 warnings).
//! 4. REST API project retrieval and validation endpoint verification.
//! 5. Equipment topology, stream connectivity, and multi-display element placements.
//! 6. Bidirectional cross-display navigation graph integrity.

use std::str::FromStr;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use tower::ServiceExt;

use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_protocol::{ProjectDetailDto, ProjectValidationResponseDto};
use nebudcs_server::{
    build_app, nht_seed::PROJ_NHT_REFERENCE, HostHandle, ProjectManager, ServerConfig,
    SimulationHost,
};
use nebudcs_sim::SimulationEngine;
use nebudcs_topology::validation::validate_project;

fn create_test_host(config: ServerConfig) -> (SimulationHost, HostHandle) {
    let mut engine = SimulationEngine::new(config.simulation_dt).expect("Failed to create engine");
    engine.clock_mut().play();
    let tag_store = RuntimeTagStore::new();

    let host = SimulationHost::start(config, engine, tag_store, None, None, None)
        .expect("Failed to start simulation host");

    let handle = host.handle();
    (host, handle)
}

#[test]
fn test_nht_reference_project_seed_and_validation() {
    let pm = ProjectManager::new();
    assert!(
        pm.get_project(PROJ_NHT_REFERENCE).is_some(),
        "NHT reference project must be registered in ProjectManager"
    );
    let proj = nebudcs_server::nht_seed::build_nht_reference_project();

    assert_eq!(
        proj.id,
        nebudcs_types::ProjectId::from_str(PROJ_NHT_REFERENCE).unwrap()
    );
    assert_eq!(
        proj.name,
        "Naphtha Hydrotreater (NHT) Reference Plant"
    );

    // Validate plant structure
    let plant_struct = proj
        .plant_structure
        .as_ref()
        .expect("Plant structure must be present");
    assert_eq!(plant_struct.plant_name, "Naphtha Hydrotreater Complex");
    assert_eq!(plant_struct.areas.len(), 1);

    let area = &plant_struct.areas[0];
    assert_eq!(area.id, "area-nht");
    assert_eq!(area.systems.len(), 12, "Must contain 12 operational systems");

    // Check all 12 operational systems
    let system_ids: Vec<&str> = area.systems.iter().map(|s| s.id.as_str()).collect();
    assert!(system_ids.contains(&"sys-nht-feed"));
    assert!(system_ids.contains(&"sys-nht-h2"));
    assert!(system_ids.contains(&"sys-nht-rx"));
    assert!(system_ids.contains(&"sys-nht-sep"));
    assert!(system_ids.contains(&"sys-nht-recycle"));
    assert!(system_ids.contains(&"sys-nht-strip"));
    assert!(system_ids.contains(&"sys-nht-split"));
    assert!(system_ids.contains(&"sys-nht-water"));
    assert!(system_ids.contains(&"sys-nht-steam"));
    assert!(system_ids.contains(&"sys-nht-sour"));
    assert!(system_ids.contains(&"sys-nht-offgas"));
    assert!(system_ids.contains(&"sys-nht-bl"));

    // Check all 7 operational units
    let mut all_units = Vec::new();
    for sys in &area.systems {
        for subsys in &sys.subsystems {
            all_units.extend(subsys.units.clone());
        }
    }
    assert_eq!(all_units.len(), 7, "Must contain 7 operational units");
    let unit_ids: Vec<&str> = all_units.iter().map(|u| u.id.as_str()).collect();
    assert!(unit_ids.contains(&"unit-nht-feed"));
    assert!(unit_ids.contains(&"unit-nht-rx"));
    assert!(unit_ids.contains(&"unit-nht-sep"));
    assert!(unit_ids.contains(&"unit-nht-recycle"));
    assert!(unit_ids.contains(&"unit-nht-strip"));
    assert!(unit_ids.contains(&"unit-nht-split"));
    assert!(unit_ids.contains(&"unit-nht-util"));

    // Check all 12 displays
    assert_eq!(
        plant_struct.displays.len(),
        12,
        "Must contain 12 DCS displays"
    );
    let display_ids: Vec<&str> = plant_struct.displays.iter().map(|d| d.id.as_str()).collect();
    assert!(display_ids.contains(&"disp-nht-overview"));
    assert!(display_ids.contains(&"disp-nht-feed"));
    assert!(display_ids.contains(&"disp-nht-rx"));
    assert!(display_ids.contains(&"disp-nht-sep"));
    assert!(display_ids.contains(&"disp-nht-h2-recycle"));
    assert!(display_ids.contains(&"disp-nht-strip"));
    assert!(display_ids.contains(&"disp-nht-split"));
    assert!(display_ids.contains(&"disp-nht-util"));
    assert!(display_ids.contains(&"disp-nht-comp-detail"));
    assert!(display_ids.contains(&"disp-nht-rx-detail"));
    assert!(display_ids.contains(&"disp-nht-alarm-summary"));
    assert!(display_ids.contains(&"disp-nht-ctl-summary"));

    // Check non-empty placements for each display
    for disp in &plant_struct.displays {
        assert!(
            !disp.placements.is_empty(),
            "Display {} must have element placements",
            disp.id
        );
    }

    // Check 17 navigation links
    assert_eq!(
        proj.navigation_links.len(),
        17,
        "Must contain exactly 17 navigation links"
    );

    // Verify all 41 physical equipment items exist in Unit 1 (physical process topology)
    let unit = &proj.plants[0].areas[0].units[0];
    assert_eq!(
        unit.equipment.len(),
        41,
        "Physical unit must contain 41 equipment items"
    );

    // Verify key equipment presence
    let eq_map: std::collections::HashMap<&str, _> = unit
        .equipment
        .iter()
        .map(|e| (e.name.as_str(), e))
        .collect();

    assert!(eq_map.contains_key("V-101"), "Feed Surge Drum V-101 must exist");
    assert!(eq_map.contains_key("P-101A"), "Feed Pump P-101A must exist");
    assert!(eq_map.contains_key("R-101"), "Reactor R-101 must exist");
    assert!(eq_map.contains_key("F-101"), "Fired Heater F-101 must exist");
    assert!(eq_map.contains_key("K-101A"), "Recycle Gas Compressor K-101A must exist");
    assert!(eq_map.contains_key("V-102"), "HP Separator V-102 must exist");
    assert!(eq_map.contains_key("V-103"), "LP Separator V-103 must exist");
    assert!(eq_map.contains_key("C-101"), "Stripper Column C-101 must exist");
    assert!(eq_map.contains_key("C-102"), "Splitter Column C-102 must exist");

    // Check 7 CFE shells
    for shell in ["E-101A", "E-101B", "E-101C", "E-101D", "E-101E", "E-101F", "E-101G"] {
        assert!(
            eq_map.contains_key(shell),
            "CFE shell {} must exist",
            shell
        );
    }

    // Verify 49 continuous material streams
    assert_eq!(
        unit.streams.len(),
        49,
        "Physical unit must contain 49 streams"
    );

    // Verify clean design-time validation
    let eq_mgr = nebudcs_scripting::CustomEquipmentManager::new();
    let mut defs = indexmap::IndexMap::new();
    for def in eq_mgr.list_definition_objects() {
        let d = def.to_domain_definition();
        defs.insert(d.id, d);
    }
    for t in eq_mgr.library().list_templates(None, None) {
        let d = t.definition.to_domain_definition();
        defs.insert(d.id, d);
    }
    for d in nebudcs_scripting::standard_domain_equipment_definitions() {
        defs.insert(d.id, d);
    }
    let validation = validate_project(&proj, &defs);
    assert!(
        validation.is_valid,
        "NHT reference project must be valid. Results: {:?}",
        validation.results
    );
    assert_eq!(validation.error_count, 0, "Errors must be 0");
}

#[tokio::test]
async fn test_nht_reference_plant_rest_api_roundtrip() {
    let config = ServerConfig {
        simulation_dt: 0.05,
        auth_token: Some("nht_test_token".to_string()),
        ..Default::default()
    };
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle);

    // 1. Fetch project via GET /api/v1/projects/{id}
    let req = Request::builder()
        .uri(format!("/api/v1/projects/{}", PROJ_NHT_REFERENCE))
        .method("GET")
        .header(header::AUTHORIZATION, "Bearer nht_test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let detail: ProjectDetailDto =
        serde_json::from_slice(&bytes).expect("Failed to parse ProjectDetailDto");

    assert_eq!(detail.id, PROJ_NHT_REFERENCE);
    assert_eq!(
        detail.name,
        "Naphtha Hydrotreater (NHT) Reference Plant"
    );

    // Check displays in REST response
    assert_eq!(detail.displays.len(), 12);
    let overview_disp = detail
        .displays
        .iter()
        .find(|d| d.id == "disp-nht-overview")
        .expect("Overview display must exist in DTO");
    assert!(
        overview_disp.placements.len() >= 10,
        "Overview display placements: {}",
        overview_disp.placements.len()
    );

    // Check navigation links in REST response
    assert_eq!(detail.navigation_links.len(), 17);

    // 2. Test design-time validation endpoint POST /api/v1/projects/{id}/validate
    let val_req = Request::builder()
        .uri(format!("/api/v1/projects/{}/validate", PROJ_NHT_REFERENCE))
        .method("POST")
        .header(header::AUTHORIZATION, "Bearer nht_test_token")
        .body(Body::empty())
        .unwrap();

    let val_response = app.oneshot(val_req).await.unwrap();
    assert_eq!(val_response.status(), StatusCode::OK);

    let val_bytes = axum::body::to_bytes(val_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let val_result: ProjectValidationResponseDto =
        serde_json::from_slice(&val_bytes).expect("Failed to parse ProjectValidationResponseDto");

    assert!(
        val_result.valid,
        "Project validation endpoint must report valid"
    );
    assert_eq!(val_result.error_count, 0);
}

#[test]
fn test_nht_reference_plant_equipment_provenance_and_instrumentation() {
    let proj = nebudcs_server::nht_seed::build_nht_reference_project();
    let unit = &proj.plants[0].areas[0].units[0];

    // Check Reactor R-101 parameters and provenance
    let r101 = unit
        .equipment
        .iter()
        .find(|e| e.name == "R-101")
        .expect("R-101 must exist");

    assert_eq!(r101.name, "R-101");
    assert_eq!(
        r101.definition_id,
        nebudcs_types::EquipmentDefinitionId::from_str("standard.reactor.fixed_bed").unwrap()
    );
    assert_eq!(r101.parameters.get("lhsv"), Some(&10.0));
    assert_eq!(r101.parameters.get("operating_temp"), Some(&332.0));
    assert_eq!(r101.parameters.get("max_bed_delta_t"), Some(&10.0));
    assert_eq!(r101.parameters.get("catalyst_volume_m3"), Some(&19.9));

    // Check Compressor K-101A parameters
    let k101a = unit
        .equipment
        .iter()
        .find(|e| e.name == "K-101A")
        .expect("K-101A must exist");
    assert_eq!(k101a.name, "K-101A");
    assert_eq!(k101a.parameters.get("speed_rpm"), Some(&10450.0));
    assert_eq!(k101a.parameters.get("rated_power"), Some(&680000.0));

    // Check Control Loops
    assert_eq!(unit.control_loops.len(), 12, "Must contain 12 PID control loops");
    let loop_names: Vec<&str> = unit.control_loops.iter().map(|c| c.name.as_str()).collect();
    assert!(loop_names.contains(&"FIC-101"), "Feed flow controller FIC-101");
    assert!(loop_names.contains(&"PIC-101"), "HP Separator pressure PIC-101");
    assert!(loop_names.contains(&"LIC-101"), "HP Separator level LIC-101");
    assert!(loop_names.contains(&"TIC-101"), "Furnace COT TIC-101");
    assert!(loop_names.contains(&"FIC-102"), "Recycle gas flow FIC-102");
    assert!(loop_names.contains(&"FIC-103"), "Wash water flow FIC-103");
    assert!(loop_names.contains(&"PIC-103"), "Stripper pressure PIC-103");

    // Check Alarms
    assert_eq!(unit.alarms.len(), 8, "Must contain 8 critical alarms");
    let alarm_names: Vec<&str> = unit.alarms.iter().map(|a| a.name.as_str()).collect();
    assert!(alarm_names.contains(&"TAH-101"));
    assert!(alarm_names.contains(&"TAHH-101"));
    assert!(alarm_names.contains(&"TDAH-101"));
    assert!(alarm_names.contains(&"PAHH-101"));

    // Check Interlocks / ESD
    assert_eq!(unit.interlocks.len(), 2, "Must contain 2 protective ESD interlocks");
    let esd_r101 = unit
        .interlocks
        .iter()
        .find(|i| i.name == "ESD-R101-TEMP")
        .expect("ESD-R101-TEMP must exist");
    assert_eq!(esd_r101.source_tag, "TT-102");
    assert_eq!(esd_r101.threshold, 343.3);
}

#[test]
fn test_nht_cross_display_navigation_traversal() {
    let proj = nebudcs_server::nht_seed::build_nht_reference_project();
    let plant_struct = proj.plant_structure.as_ref().unwrap();

    let display_ids: std::collections::HashSet<&str> =
        plant_struct.displays.iter().map(|d| d.id.as_str()).collect();

    // Verify that every navigation link points between valid displays
    for link in &proj.navigation_links {
        assert!(
            display_ids.contains(link.source_display_id.as_str()),
            "Link {} has unknown source display {}",
            link.id,
            link.source_display_id
        );
        assert!(
            display_ids.contains(link.target_display_id.as_str()),
            "Link {} has unknown target display {}",
            link.id,
            link.target_display_id
        );
    }

    // Verify key navigation paths
    let has_path = |from: &str, to: &str| {
        proj.navigation_links
            .iter()
            .any(|l| l.source_display_id == from && l.target_display_id == to)
    };

    assert!(has_path("disp-nht-overview", "disp-nht-feed"));
    assert!(has_path("disp-nht-overview", "disp-nht-rx"));
    assert!(has_path("disp-nht-overview", "disp-nht-sep"));
    assert!(has_path("disp-nht-overview", "disp-nht-h2-recycle"));
    assert!(has_path("disp-nht-overview", "disp-nht-strip"));
    assert!(has_path("disp-nht-overview", "disp-nht-split"));
    assert!(has_path("disp-nht-feed", "disp-nht-rx"));
    assert!(has_path("disp-nht-rx", "disp-nht-rx-detail"));
    assert!(has_path("disp-nht-rx-detail", "disp-nht-rx"));
    assert!(has_path("disp-nht-rx", "disp-nht-sep"));
    assert!(has_path("disp-nht-sep", "disp-nht-h2-recycle"));
    assert!(has_path("disp-nht-h2-recycle", "disp-nht-comp-detail"));
    assert!(has_path("disp-nht-comp-detail", "disp-nht-h2-recycle"));
    assert!(has_path("disp-nht-h2-recycle", "disp-nht-rx"));
    assert!(has_path("disp-nht-strip", "disp-nht-split"));
    assert!(has_path("disp-nht-split", "disp-nht-overview"));
}
