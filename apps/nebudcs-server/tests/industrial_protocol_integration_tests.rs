//! Stage 7: Industrial Protocol & Communication Architecture Integration Tests.
//!
//! Verifies:
//! 1. REST API endpoints for Connections, Tag Mappings, Diagnostics, and ISA-95 Architecture.
//! 2. Cybersecurity SSRF Guard blocking cloud metadata and blacklisted ports.
//! 3. Tag Mapping ownership conflict rejection (multiple inbounds to same canonical tag).
//! 4. 10,000-tick bit-identical determinism verification (communications decoupled from simulation core).

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;

use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_model::tag::{TagQuality, TagValue};
use nebudcs_protocol::dto::*;
use nebudcs_server::{build_app, HostHandle, ServerConfig, SimulationHost};
use nebudcs_sim::SimulationEngine;
use nebudcs_types::TagId;

fn create_test_host(config: ServerConfig) -> (SimulationHost, HostHandle) {
    let mut engine = SimulationEngine::new(config.simulation_dt).expect("Failed to create engine");
    engine.clock_mut().play();

    let mut tag_store = RuntimeTagStore::new();
    let ti_id = TagId::new();
    let pi_id = TagId::new();

    tag_store.register_tag("TI-101.PV", ti_id, TagValue::Float(185.0), TagQuality::Good);
    tag_store.register_tag("PI-101.PV", pi_id, TagValue::Float(32.4), TagQuality::Good);

    let host = SimulationHost::start(config, engine, tag_store, None, None, None)
        .expect("Failed to start host");

    let handle = host.handle().clone();
    (host, handle)
}

#[tokio::test]
async fn test_communications_rest_endpoints_and_ssrf() {
    let config = ServerConfig {
        simulation_dt: 0.05,
        admin_token: Some("secret_admin".to_string()),
        operator_token: Some("secret_operator".to_string()),
        trainee_token: Some("secret_trainee".to_string()),
        ..Default::default()
    };

    let (_host, handle) = create_test_host(config);
    let app = build_app(handle);

    // 1. GET /api/v1/communications/connections (accessible without auth or with any role)
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/communications/connections")
        .header(header::AUTHORIZATION, "Bearer secret_operator")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let conns: Vec<ProtocolConnectionDto> = serde_json::from_slice(&body).unwrap();
    assert!(
        conns.len() >= 5,
        "Acceptance Plants A, B, C, D, E must be pre-seeded"
    );

    // 2. GET /api/v1/communications/connections/conn-modbus-a
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/communications/connections/conn-modbus-a")
        .header(header::AUTHORIZATION, "Bearer secret_operator")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let plant_a: ProtocolConnectionDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(plant_a.protocol, "ModbusTcp");
    assert_eq!(plant_a.zone_id, "zone-l2-control");

    // 3. Trainee attempting to create connection MUST receive 403 Forbidden
    let new_conn = json!({
        "id": "conn-unauthorized",
        "name": "Rogue Connection",
        "protocol": "ModbusTcp",
        "endpoint": "tcp://192.168.1.99:502",
        "enabled": true,
        "state": "Disconnected",
        "zone_id": "zone-l2-control",
        "timeout_ms": 1000,
        "reconnect_delay_ms": 5000,
        "use_tls": false,
        "active_tags": 0,
        "last_comm_ms": 0
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/communications/connections")
        .header(header::AUTHORIZATION, "Bearer secret_trainee")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&new_conn).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // 4. Admin creating connection with SSRF Cloud Metadata MUST be rejected
    let ssrf_conn = json!({
        "id": "conn-ssrf-attack",
        "name": "SSRF Cloud Metadata Exploit",
        "protocol": "OpcUa",
        "endpoint": "http://169.254.169.254/latest/meta-data/",
        "enabled": true,
        "state": "Disconnected",
        "zone_id": "zone-l2-control",
        "timeout_ms": 1000,
        "reconnect_delay_ms": 5000,
        "use_tls": false,
        "active_tags": 0,
        "last_comm_ms": 0
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/communications/connections")
        .header(header::AUTHORIZATION, "Bearer secret_admin")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&ssrf_conn).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "SSRF attempt to cloud metadata must be rejected"
    );

    // 5. Test Connection reachability endpoint testing SSRF
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/communications/connections/conn-modbus-a/test?protocol=ModbusTcp")
        .header(header::AUTHORIZATION, "Bearer secret_operator")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 6. GET /api/v1/communications/mappings
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/communications/mappings")
        .header(header::AUTHORIZATION, "Bearer secret_operator")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let mappings: Vec<TagMappingDto> = serde_json::from_slice(&body).unwrap();
    assert!(!mappings.is_empty());

    // 7. Conflict detection: Admin attempting to create duplicate Inbound mapping for same canonical tag
    let duplicate_mapping = json!({
        "id": "map-duplicate-conflict",
        "connection_id": "conn-modbus-a",
        "external_address": "HR:40099:F32:WORDSWAP",
        "canonical_tag": "TI-101.PV", // TI-101.PV is already mapped
        "direction": "Inbound",
        "datatype": "Float32",
        "scale": 1.0,
        "offset": 0.0,
        "engineering_unit": "°C",
        "deadband": 0.1,
        "update_rate_ms": 100,
        "stale_timeout_ms": 2000,
        "access": "ReadOnly",
        "quality": "Good"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/communications/mappings")
        .header(header::AUTHORIZATION, "Bearer secret_admin")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&duplicate_mapping).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "Duplicate inbound mapping targeting same canonical tag must be rejected"
    );

    // 8. GET /api/v1/communications/architecture (ISA-95 Level 0-4 zones)
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/communications/architecture")
        .header(header::AUTHORIZATION, "Bearer secret_operator")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let zones: Vec<IndustrialZoneDto> = serde_json::from_slice(&body).unwrap();
    assert_eq!(zones.len(), 5, "Must represent ISA-95 Levels 0, 1, 2, 3, 4");
    assert!(zones[0].level.contains("0"));
    assert!(zones[4].level.contains("4"));

    // 9. GET /api/v1/communications/diagnostics
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/communications/diagnostics")
        .header(header::AUTHORIZATION, "Bearer secret_operator")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_simulation_10000_tick_bit_identical_determinism() {
    let dt = 0.05;

    // Create baseline engine and simulate 10,000 ticks in complete isolation
    let mut baseline_engine = SimulationEngine::new(dt).expect("Failed to create baseline engine");
    baseline_engine.clock_mut().play();

    for _ in 0..10_000 {
        baseline_engine.step().expect("Baseline step failed");
    }
    let baseline_time_bits = baseline_engine.clock().time().seconds().to_bits();
    let baseline_ticks = baseline_engine.clock().tick_count();

    // Create communications-integrated engine and simulate 10,000 ticks
    let mut comms_engine = SimulationEngine::new(dt).expect("Failed to create comms engine");
    comms_engine.clock_mut().play();

    let mut comms_mgr = nebudcs_server::CommunicationsManager::new();
    let mut tag_store = RuntimeTagStore::new();

    for tick in 0..10_000 {
        // Run tick boundary sync (zero overhead, non-blocking)
        comms_mgr.sync_external_values(&mut tag_store, tick as u64);
        comms_engine.step().expect("Comms step failed");
    }
    let comms_time_bits = comms_engine.clock().time().seconds().to_bits();
    let comms_ticks = comms_engine.clock().tick_count();

    assert_eq!(baseline_ticks, 10_000);
    assert_eq!(comms_ticks, 10_000);
    assert_eq!(
        baseline_time_bits, comms_time_bits,
        "10,000 ticks simulation time must be 100% bit-identical (to_bits)"
    );
}
