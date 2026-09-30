//! Integration tests for Stage 1 Virtual Controller REST APIs and RBAC enforcement.

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

use nebudcs_control::runtime::{ControllerConfig, ControllerTask, VirtualController};
use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_control::system::ControlSystem;
use nebudcs_protocol::{ControllerDetailDto, ControllerSummaryDto};
use nebudcs_server::{build_app, HostHandle, ServerConfig, SimulationHost};
use nebudcs_sim::SimulationEngine;
use nebudcs_types::{ControllerId, TaskId};

fn create_test_host_with_controller(
    config: ServerConfig,
) -> (SimulationHost, HostHandle, ControllerId) {
    let mut engine = SimulationEngine::new(config.simulation_dt).expect("Failed to create engine");
    engine.clock_mut().play();
    let tag_store = RuntimeTagStore::new();

    let mut ctrl_sys = ControlSystem::new();
    let ctrl_id = ControllerId::new();
    let mut ctrl = VirtualController::new(ctrl_id, "CTRL-UNIT-01", ControllerConfig::default());
    let task = ControllerTask::new_periodic(TaskId::new(), "Fast50ms", 0.05, 5);
    ctrl.add_task(task);
    ctrl_sys.register_controller(ctrl);

    let host = SimulationHost::start(config, engine, tag_store, None, Some(ctrl_sys), None)
        .expect("Failed to start simulation host");

    let handle = host.handle();
    (host, handle, ctrl_id)
}

#[tokio::test]
async fn test_controller_api_list_get_and_rbac() {
    let config = ServerConfig {
        simulation_dt: 0.05,
        admin_token: Some("secret_admin".to_string()),
        operator_token: Some("secret_operator".to_string()),
        trainee_token: Some("secret_trainee".to_string()),
        ..Default::default()
    };

    let (_host, handle, ctrl_id) = create_test_host_with_controller(config);
    let app = build_app(handle);

    // Wait a brief moment for runner thread to populate controllers_cache
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // 1. GET /api/v1/controllers (accessible with operator token)
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/controllers")
        .header(header::AUTHORIZATION, "Bearer secret_operator")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let controllers: Vec<ControllerSummaryDto> = serde_json::from_slice(&body).unwrap();
    assert_eq!(controllers.len(), 1);
    assert_eq!(controllers[0].name, "CTRL-UNIT-01");
    assert_eq!(controllers[0].id, ctrl_id.to_string());
    assert_eq!(controllers[0].task_count, 1);

    // 2. GET /api/v1/controllers/{id} by ControllerId
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/controllers/{ctrl_id}"))
        .header(header::AUTHORIZATION, "Bearer secret_trainee")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let detail: ControllerDetailDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(detail.name, "CTRL-UNIT-01");
    assert_eq!(detail.lifecycle, "STOPPED");
    assert_eq!(detail.tasks.len(), 1);
    assert_eq!(detail.tasks[0].name, "Fast50ms");

    // 3. GET /api/v1/controllers/{name} by name
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/controllers/CTRL-UNIT-01")
        .header(header::AUTHORIZATION, "Bearer secret_trainee")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 4. RBAC: Trainee attempts POST /api/v1/controllers/{id}/state -> 403 Forbidden
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/controllers/{ctrl_id}/state"))
        .header(header::AUTHORIZATION, "Bearer secret_trainee")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "action": "start" }).to_string()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "Trainee must be forbidden from mutating controller state"
    );

    // 5. RBAC: Operator attempts POST /api/v1/controllers/{id}/state -> 403 Forbidden
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/controllers/{ctrl_id}/state"))
        .header(header::AUTHORIZATION, "Bearer secret_operator")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "action": "start" }).to_string()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "Operator must be forbidden from mutating controller state"
    );

    // 6. RBAC: Admin executes POST /api/v1/controllers/{id}/state -> 200 OK
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/controllers/{ctrl_id}/state"))
        .header(header::AUTHORIZATION, "Bearer secret_admin")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "action": "start" }).to_string()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Admin must be permitted to start controller"
    );
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let res: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(res["status"], "ok");

    // 7. Nonexistent controller returns 404
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/controllers/NONEXISTENT/state")
        .header(header::AUTHORIZATION, "Bearer secret_admin")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "action": "start" }).to_string()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // 8. Invalid action returns 400 Bad Request
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/controllers/{ctrl_id}/state"))
        .header(header::AUTHORIZATION, "Bearer secret_admin")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "action": "destroy" }).to_string()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
