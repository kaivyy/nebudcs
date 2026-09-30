//! Integration tests for Stage 3 I/O HAL REST APIs and RBAC enforcement.

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_control::system::ControlSystem;
use nebudcs_protocol::{HalChannelDto, HalSystemSummaryDto};
use nebudcs_server::{build_app, HostHandle, ServerConfig, SimulationHost};
use nebudcs_sim::SimulationEngine;

fn create_test_host_with_hal(config: ServerConfig) -> (SimulationHost, HostHandle) {
    let mut engine = SimulationEngine::new(config.simulation_dt).expect("Failed to create engine");
    engine.clock_mut().play();
    let tag_store = RuntimeTagStore::new();

    let mut ctrl_sys = ControlSystem::new();
    ctrl_sys.set_io_system(nebudcs_server::build_default_hal_system());

    let host = SimulationHost::start(config, engine, tag_store, None, Some(ctrl_sys), None)
        .expect("Failed to start simulation host");

    let handle = host.handle();
    (host, handle)
}

#[tokio::test]
async fn test_hal_api_endpoints_and_rbac() {
    let config = ServerConfig {
        simulation_dt: 0.05,
        admin_token: Some("secret_admin".to_string()),
        operator_token: Some("secret_operator".to_string()),
        trainee_token: Some("secret_trainee".to_string()),
        ..Default::default()
    };

    let (_host, handle) = create_test_host_with_hal(config);
    let app = build_app(handle);

    // Wait a brief moment for runner thread to step and populate hal_cache
    tokio::time::sleep(std::time::Duration::from_millis(60)).await;

    // 1. GET /api/v1/hal/systems (accessible with operator token)
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/hal/systems")
        .header(header::AUTHORIZATION, "Bearer secret_operator")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let systems: Vec<HalSystemSummaryDto> = serde_json::from_slice(&body).unwrap();
    assert_eq!(systems.len(), 1);
    let sys = &systems[0];
    assert_eq!(sys.name, "HAL-MAIN");
    assert_eq!(sys.rack_count, 1);
    assert!(sys.module_count >= 4);
    assert!(sys.channel_count >= 32);
    assert_eq!(sys.racks.len(), 1);
    assert_eq!(sys.racks[0].rack_number, 1);
    assert!(sys.racks[0].power_ok);

    // 2. GET /api/v1/hal/channels/R01-C01-AI01 and extended R01-C01-M01-AI01
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/hal/channels/R01-C01-AI01")
        .header(header::AUTHORIZATION, "Bearer secret_trainee")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let ch: HalChannelDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(ch.address, "R01-C01-AI01");
    assert_eq!(ch.kind, "AI");
    assert_eq!(ch.engineering_unit, "%");
    assert_eq!(ch.lrv, 0.0);
    assert_eq!(ch.urv, 100.0);

    // Also verify extended address format query resolves same channel
    let req_ext = Request::builder()
        .method("GET")
        .uri("/api/v1/hal/channels/R01-C01-M01-AI01")
        .header(header::AUTHORIZATION, "Bearer secret_trainee")
        .body(Body::empty())
        .unwrap();
    let resp_ext = app.clone().oneshot(req_ext).await.unwrap();
    assert_eq!(resp_ext.status(), StatusCode::OK);

    // 3. RBAC: Trainee attempts POST /api/v1/hal/channels/R01-C01-AI01/fault -> 403 Forbidden
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/hal/channels/R01-C01-AI01/fault")
        .header(header::AUTHORIZATION, "Bearer secret_trainee")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "fault": "wirebreak"
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "Trainee must be forbidden from injecting hardware faults"
    );

    // 4. RBAC: Operator attempts POST /api/v1/hal/channels/R01-C01-AI01/fault -> 403 Forbidden
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/hal/channels/R01-C01-AI01/fault")
        .header(header::AUTHORIZATION, "Bearer secret_operator")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "fault": "wirebreak"
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "Operator must be forbidden from injecting hardware faults"
    );

    // 5. RBAC: Admin executes POST /api/v1/hal/channels/R01-C01-AI01/fault -> 200 OK
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/hal/channels/R01-C01-AI01/fault")
        .header(header::AUTHORIZATION, "Bearer secret_admin")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "fault": "wirebreak",
                "forced_value": 0.0
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Admin must be permitted to inject hardware faults"
    );
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let res: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(res["status"], "ok");

    // Allow runner tick to process the command and refresh cache
    tokio::time::sleep(std::time::Duration::from_millis(60)).await;

    // 6. Verify channel state reflects fault
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/hal/channels/R01-C01-M01-AI01")
        .header(header::AUTHORIZATION, "Bearer secret_admin")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let ch_faulted: HalChannelDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(ch_faulted.fault_state, "WireBreak");

    // 7. Clear fault with Admin
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/hal/channels/R01-C01-AI01/fault")
        .header(header::AUTHORIZATION, "Bearer secret_admin")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "fault": "clear"
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 8. GET /api/v1/hal/mappings
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/hal/mappings")
        .header(header::AUTHORIZATION, "Bearer secret_operator")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 9. Nonexistent channel returns 404
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/hal/channels/R99-C99-M99-AI99")
        .header(header::AUTHORIZATION, "Bearer secret_admin")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
