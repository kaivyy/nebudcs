//! End-to-end integration tests for NebuDCS Phase 10 Custom Equipment API endpoints.

use std::collections::BTreeMap;
use std::time::Duration;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_model::{
    EquipmentCategory, OperatingEnvelope, ParameterDefinition, PortDefinition, PortDirection,
    PortDomain,
};
use nebudcs_protocol::{
    EquipmentDefinitionSummaryDto, EquipmentInstanceSummaryDto, ValidateEquipmentResponseDto,
};
use nebudcs_scripting::{
    Curve1D, CustomEquipmentDefinition, CustomizationTier, IndexMap, ScriptDefinition,
    TripConditionDefinition, TripOperator,
};
use nebudcs_server::{build_app, HostHandle, ServerConfig, SimulationHost};
use nebudcs_sim::SimulationEngine;
use nebudcs_types::SchemaVersion;

fn create_test_host(config: ServerConfig) -> (SimulationHost, HostHandle) {
    let mut engine = SimulationEngine::new(config.simulation_dt).expect("Failed to create engine");
    engine.clock_mut().play();
    let tag_store = RuntimeTagStore::new();

    let host = SimulationHost::start(config, engine, tag_store, None, None, None)
        .expect("Failed to start simulation host");

    let handle = host.handle();
    (host, handle)
}

fn make_port(name: &str, dir: PortDirection, domain: PortDomain) -> PortDefinition {
    PortDefinition {
        id: nebudcs_types::PortId::new(),
        name: name.to_string(),
        domain,
        direction: dir,
        phase_compatibility: vec![nebudcs_model::Phase::Liquid],
        required: true,
        cardinality: nebudcs_model::equipment::Cardinality::ExactlyOne,
        description: format!("Port {name}"),
    }
}

fn make_param(name: &str, default: f64, min: f64, max: f64) -> ParameterDefinition {
    ParameterDefinition {
        name: name.to_string(),
        description: format!("Parameter {name}"),
        quantity_kind: nebudcs_units::display::QuantityKind::Pressure,
        default_value: default,
        min_value: Some(min),
        max_value: Some(max),
        required: true,
    }
}

fn sample_pump_definition(id: &str, name: &str) -> CustomEquipmentDefinition {
    let mut params = IndexMap::new();
    params.insert(
        "rated_speed".to_string(),
        make_param("rated_speed", 1800.0, 0.0, 3600.0),
    );

    let mut curves = IndexMap::new();
    curves.insert(
        "head_curve".to_string(),
        Curve1D::new(
            "HeadCurve",
            "flow",
            "delta_p",
            vec![(0.0, 100.0), (50.0, 85.0), (100.0, 60.0), (150.0, 20.0)],
        ),
    );

    CustomEquipmentDefinition {
        id: id.into(),
        name: name.into(),
        version: SchemaVersion::new(1, 0, 0),
        category: EquipmentCategory::Rotating,
        tier: CustomizationTier::Level1Declarative,
        description: "Standard centrifugal pump".into(),
        parameters: params,
        ports: vec![
            make_port("suction", PortDirection::Inlet, PortDomain::Material),
            make_port("discharge", PortDirection::Outlet, PortDomain::Material),
        ],
        curves,
        curves_2d: IndexMap::new(),
        operating_envelope: OperatingEnvelope::default(),
        alarm_definitions: vec![],
        trips: vec![TripConditionDefinition {
            id: "TRIP-OVERPRESSURE".into(),
            name: "High Discharge Pressure Trip".into(),
            condition_var: "discharge_p".into(),
            operator: TripOperator::GreaterThan,
            threshold: 120.0,
            fail_safe_state: "tripped".into(),
            latched: true,
        }],
        interlocks: vec![],
        script: None,
        composition: None,
        metadata: IndexMap::new(),
    }
}

#[tokio::test]
async fn test_equipment_validation_api() {
    let config = ServerConfig {
        simulation_dt: 0.05,
        auth_token: Some("secret123".to_string()),
        ..Default::default()
    };
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle);

    // 1. Valid definition
    let valid_def = sample_pump_definition("DEF-PUMP-VALID", "Valid Pump");
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/equipment/validate")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::from(serde_json::to_vec(&valid_def).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let val_resp: ValidateEquipmentResponseDto = serde_json::from_slice(&body_bytes).unwrap();
    assert!(val_resp.valid);
    assert!(val_resp.errors.is_empty());

    // 2. Invalid definition (empty name, min > max)
    let mut invalid_def = valid_def.clone();
    invalid_def.name = "   ".into();
    invalid_def
        .parameters
        .get_mut("rated_speed")
        .unwrap()
        .min_value = Some(5000.0);
    invalid_def
        .parameters
        .get_mut("rated_speed")
        .unwrap()
        .max_value = Some(1000.0);

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/equipment/validate")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::from(serde_json::to_vec(&invalid_def).unwrap()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let val_resp: ValidateEquipmentResponseDto = serde_json::from_slice(&body_bytes).unwrap();
    assert!(!val_resp.valid);
    assert!(!val_resp.errors.is_empty());
}

#[tokio::test]
async fn test_equipment_definition_and_query_api() {
    let config = ServerConfig {
        simulation_dt: 0.05,
        auth_token: Some("secret123".to_string()),
        ..Default::default()
    };
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle);

    let def = sample_pump_definition("DEF-PUMP-QUERY", "Centrifugal Feed Pump");

    // 1. Register definition
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/equipment/define")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::from(serde_json::to_vec(&def).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Give simulation runner a tick to apply command
    tokio::time::sleep(Duration::from_millis(100)).await;

    // 2. Query definitions list
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/equipment/definitions")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let defs: Vec<EquipmentDefinitionSummaryDto> = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].id, "DEF-PUMP-QUERY");
    assert_eq!(defs[0].name, "Centrifugal Feed Pump");

    // 3. Query single definition by ID
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/equipment/definitions/DEF-PUMP-QUERY")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let fetched: CustomEquipmentDefinition = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(fetched.id, "DEF-PUMP-QUERY");
    assert_eq!(fetched.category, EquipmentCategory::Rotating);
}

#[tokio::test]
async fn test_equipment_instantiation_and_control_lifecycle() {
    let config = ServerConfig {
        simulation_dt: 0.05,
        auth_token: Some("secret123".to_string()),
        ..Default::default()
    };
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle);

    // 1. Define equipment
    let def = sample_pump_definition("DEF-PUMP-LIFECYCLE", "Lifecycle Pump");
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/equipment/define")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::from(serde_json::to_vec(&def).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    tokio::time::sleep(Duration::from_millis(80)).await;

    // 2. Instantiate equipment
    let mut overrides = BTreeMap::new();
    overrides.insert("rated_speed".to_string(), 3000.0);

    let inst_req = json!({
        "definition_id": "DEF-PUMP-LIFECYCLE",
        "name": "P-801",
        "parameter_overrides": overrides
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/equipment/instantiate")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::from(serde_json::to_vec(&inst_req).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let inst_res: Value = serde_json::from_slice(&body_bytes).unwrap();
    let inst_id = inst_res["instance_id"].as_str().unwrap().to_string();

    tokio::time::sleep(Duration::from_millis(80)).await;

    // 3. Inspect instance: initially stopped
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/equipment/instances/{inst_id}"))
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let inst: EquipmentInstanceSummaryDto = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(inst.active_state, "stopped");
    assert!(!inst.is_running);
    assert_eq!(inst.parameters.get("rated_speed").copied().unwrap(), 3000.0);

    // 4. Command Start
    let ctrl_req = json!({
        "instance_id": inst_id,
        "action": "start"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/equipment/control")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::from(serde_json::to_vec(&ctrl_req).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    tokio::time::sleep(Duration::from_millis(80)).await;

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/equipment/instances/{inst_id}"))
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let inst: EquipmentInstanceSummaryDto = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(inst.active_state, "running");
    assert!(inst.is_running);

    // 5. Trigger trip: set discharge_p > 120.0
    let trip_req = json!({
        "instance_id": inst_id,
        "action": "set_var",
        "variable_name": "discharge_p",
        "value": 135.0
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/equipment/control")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::from(serde_json::to_vec(&trip_req).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Give simulation runner ticks to evaluate trip condition
    tokio::time::sleep(Duration::from_millis(150)).await;

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/equipment/instances/{inst_id}"))
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let inst: EquipmentInstanceSummaryDto = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(inst.active_state, "tripped");
    assert!(inst.is_tripped);
    assert!(!inst.is_running);

    // 6. Attempt reset while condition still violated -> rejected
    let reset_req = json!({
        "instance_id": inst_id,
        "action": "reset_trip"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/equipment/control")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::from(serde_json::to_vec(&reset_req).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // 7. Clear condition and reset -> succeeds
    let clear_req = json!({
        "instance_id": inst_id,
        "action": "set_var",
        "variable_name": "discharge_p",
        "value": 90.0
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/equipment/control")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::from(serde_json::to_vec(&clear_req).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    tokio::time::sleep(Duration::from_millis(80)).await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/equipment/control")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::from(serde_json::to_vec(&reset_req).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    tokio::time::sleep(Duration::from_millis(80)).await;

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/equipment/instances/{inst_id}"))
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let inst: EquipmentInstanceSummaryDto = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(inst.active_state, "stopped");
    assert!(!inst.is_tripped);
}

#[tokio::test]
async fn test_level3_scripted_equipment_api() {
    let config = ServerConfig {
        simulation_dt: 0.05,
        auth_token: Some("secret123".to_string()),
        ..Default::default()
    };
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle);

    // Create Level 3 scripted heater
    let mut params = IndexMap::new();
    params.insert(
        "duty_kw".to_string(),
        make_param("duty_kw", 150.0, 0.0, 500.0),
    );

    let script = r#"
        fn step(inputs, params, state, dt) {
            let flow = inputs["flow"];
            let duty = params["duty_kw"];
            let cp = 2.1;
            
            // dT = Q / (m_dot * Cp)
            let dt_temp = if flow > 0.01 {
                duty / (flow * cp)
            } else {
                0.0
            };
            
            state["temp_rise"] = clamp(dt_temp, 0.0, 150.0);
            state["outlet_temp"] = 300.0 + state["temp_rise"];
            state
        }
    "#;

    let def = CustomEquipmentDefinition {
        id: "DEF-HEATER-01".into(),
        name: "Electric Inline Heater".into(),
        version: SchemaVersion::new(1, 0, 0),
        category: EquipmentCategory::HeatTransfer,
        tier: CustomizationTier::Level3Scripted,
        description: "Rhai-scripted electric process heater".into(),
        parameters: params,
        ports: vec![
            make_port("inlet", PortDirection::Inlet, PortDomain::Material),
            make_port("outlet", PortDirection::Outlet, PortDomain::Material),
        ],
        curves: IndexMap::new(),
        curves_2d: IndexMap::new(),
        operating_envelope: OperatingEnvelope::default(),
        alarm_definitions: vec![],
        trips: vec![],
        interlocks: vec![],
        script: Some(ScriptDefinition {
            code: script.into(),
            entry_point: "step".into(),
        }),
        composition: None,
        metadata: IndexMap::new(),
    };

    // 1. Define
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/equipment/define")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::from(serde_json::to_vec(&def).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    if status != StatusCode::OK {
        let err_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        panic!(
            "define failed with status {}: {}",
            status,
            String::from_utf8_lossy(&err_bytes)
        );
    }
    tokio::time::sleep(Duration::from_millis(80)).await;

    // 2. Instantiate
    let inst_req = json!({
        "definition_id": "DEF-HEATER-01",
        "name": "E-101"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/equipment/instantiate")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::from(serde_json::to_vec(&inst_req).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let inst_res: Value = serde_json::from_slice(&body_bytes).unwrap();
    let inst_id = inst_res["instance_id"].as_str().unwrap().to_string();

    tokio::time::sleep(Duration::from_millis(80)).await;

    // 3. Start
    let ctrl_req = json!({
        "instance_id": inst_id,
        "action": "start"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/equipment/control")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::from(serde_json::to_vec(&ctrl_req).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 4. Set flow = 10.0 kg/s
    let flow_req = json!({
        "instance_id": inst_id,
        "action": "set_var",
        "variable_name": "flow",
        "value": 10.0
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/equipment/control")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::from(serde_json::to_vec(&flow_req).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Allow runner to execute script step
    tokio::time::sleep(Duration::from_millis(150)).await;

    // 5. Verify computed temperature rise
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/equipment/instances/{inst_id}"))
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let inst: EquipmentInstanceSummaryDto = serde_json::from_slice(&body_bytes).unwrap();
    assert!(inst.is_running);
    let temp_rise = inst.variables.get("temp_rise").copied().unwrap();
    // dT = 150 / (10 * 2.1) = 7.1428 K
    assert!((temp_rise - (150.0 / 21.0)).abs() < 1e-3);
    let outlet_temp = inst.variables.get("outlet_temp").copied().unwrap();
    assert!((outlet_temp - (300.0 + 150.0 / 21.0)).abs() < 1e-3);
}

#[tokio::test]
async fn test_equipment_api_security_and_adversarial_rejections() {
    let config = ServerConfig {
        simulation_dt: 0.05,
        auth_token: Some("secret123".to_string()),
        ..Default::default()
    };
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle);

    // 1. Unauthenticated request rejected with 401 Unauthorized
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/equipment/definitions")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // 2. Wrong token rejected with 401 Unauthorized
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/equipment/definitions")
        .header(header::AUTHORIZATION, "Bearer WRONG_TOKEN")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // 3. Define equipment with forbidden keyword in script (eval) -> rejected with 400 Bad Request
    let mut bad_def = sample_pump_definition("DEF-BAD-SCRIPT", "Hostile Script Pump");
    bad_def.tier = CustomizationTier::Level3Scripted;
    bad_def.script = Some(ScriptDefinition {
        code: "fn step(inputs, params, state, dt) { eval(\"1+1\"); return state; }".into(),
        entry_point: "step".into(),
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/equipment/define")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::from(serde_json::to_vec(&bad_def).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // 4. Instantiate nonexistent definition -> command rejected with 400 Bad Request
    let inst_bad = json!({
        "definition_id": "NON_EXISTENT_DEF_ID",
        "name": "GhostEquipment"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/equipment/instantiate")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::from(serde_json::to_vec(&inst_bad).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // 5. Control command on nonexistent instance -> command rejected with 400 Bad Request
    let ctrl_bad_inst = json!({
        "instance_id": "01J00000000000000000000099",
        "action": "start"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/equipment/control")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::from(serde_json::to_vec(&ctrl_bad_inst).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // 6. Control command with unsupported action -> rejected with 400 Bad Request
    let ctrl_bad_action = json!({
        "instance_id": "01J00000000000000000000099",
        "action": "unsupported_exploit"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/equipment/control")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::from(serde_json::to_vec(&ctrl_bad_action).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // 7. Malformed JSON payload -> rejected with 400 Bad Request or 422 Unprocessable Entity
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/equipment/define")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::from("{ malformed json :::"))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert!(
        resp.status() == StatusCode::BAD_REQUEST
            || resp.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}
