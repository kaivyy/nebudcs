use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;

use nebudcs_control::iec61131::*;
use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_protocol::IecCompileResponseDto;
use nebudcs_server::{build_app, HostHandle, ServerConfig, SimulationHost};
use nebudcs_sim::SimulationEngine;

fn create_test_host() -> (SimulationHost, HostHandle) {
    let config = ServerConfig {
        simulation_dt: 0.05,
        admin_token: Some("test_admin".to_string()),
        operator_token: Some("test_operator".to_string()),
        trainee_token: Some("test_trainee".to_string()),
        ..Default::default()
    };
    let engine = SimulationEngine::new(config.simulation_dt).unwrap();
    let tag_store = RuntimeTagStore::new();
    let host = SimulationHost::start(config, engine, tag_store, None, None, None).unwrap();
    let handle = host.handle();
    (host, handle)
}

#[tokio::test]
async fn test_iec_compile_st_api() {
    let (_host, handle) = create_test_host();
    let app = build_app(handle);

    // 1. Valid ST compilation
    let st_source = r#"
        VAR
            SETPOINT : REAL := 50.0;
            CURRENT : REAL := 45.0;
            OUT_DEMAND : REAL := 0.0;
        END_VAR

        IF CURRENT < SETPOINT THEN
            OUT_DEMAND := (SETPOINT - CURRENT) * 2.0;
        END_IF;
    "#;

    let payload = json!({
        "language": "ST",
        "source_text": st_source,
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/controllers/iec/compile")
        .header(header::AUTHORIZATION, "Bearer test_operator")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let dto: IecCompileResponseDto = serde_json::from_slice(&body).unwrap();
    assert!(dto.success, "Valid ST compilation should succeed");
    assert!(dto.error_message.is_none());
    assert_eq!(dto.symbol_count, 3);
    assert!(dto.statement_count >= 1);
    assert_eq!(dto.variables.len(), 3);

    // 2. Syntax error ST compilation
    let bad_st = "VAR X : REAL; END_VAR IF X THEN Y := ; END_IF;";
    let bad_payload = json!({
        "language": "ST",
        "source_text": bad_st,
    });

    let req2 = Request::builder()
        .method("POST")
        .uri("/api/v1/controllers/iec/compile")
        .header(header::AUTHORIZATION, "Bearer test_operator")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&bad_payload).unwrap()))
        .unwrap();

    let resp2 = app.clone().oneshot(req2).await.unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);

    let body2 = axum::body::to_bytes(resp2.into_body(), usize::MAX)
        .await
        .unwrap();
    let dto2: IecCompileResponseDto = serde_json::from_slice(&body2).unwrap();
    assert!(!dto2.success, "Invalid ST should report compilation error");
    assert!(dto2.error_message.is_some());
}

#[tokio::test]
async fn test_iec_compile_ld_api() {
    let (_host, handle) = create_test_host();
    let app = build_app(handle);

    let mut diag = LdDiagram::new();
    diag.add_variable(VariableDeclaration::new("START", IecType::Bool));
    let mut rung = LdRung::new(0);
    rung.add_element(LdElement::Contact(LdContact::no("START")));
    rung.add_coil(LdCoil::output("%QX0.0"));
    diag.add_rung(rung);

    let ld_json = serde_json::to_string(&diag).unwrap();
    let payload = json!({
        "language": "LD",
        "source_text": ld_json,
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/controllers/iec/compile")
        .header(header::AUTHORIZATION, "Bearer test_operator")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let dto: IecCompileResponseDto = serde_json::from_slice(&body).unwrap();
    assert!(dto.success);
    assert_eq!(dto.statement_count, 1);
}

#[tokio::test]
async fn test_iec_compile_fbd_and_sfc_api() {
    let (_host, handle) = create_test_host();
    let app = build_app(handle);

    // FBD API test
    let mut graph = FbdGraph::new();
    graph.add_node(FbdNode::new(
        "N1",
        FbdNodeKind::InputVar {
            name: "IN_A".to_string(),
        },
    ));
    graph.add_node(FbdNode::new(
        "N2",
        FbdNodeKind::OutputVar {
            name: "OUT_B".to_string(),
        },
    ));
    graph.add_connection(FbdConnection::new("N1", "OUT", "N2", "IN"));

    let fbd_json = serde_json::to_string(&graph).unwrap();
    let fbd_payload = json!({
        "language": "FBD",
        "source_text": fbd_json,
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/controllers/iec/compile")
        .header(header::AUTHORIZATION, "Bearer test_operator")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&fbd_payload).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let dto: IecCompileResponseDto = serde_json::from_slice(&body).unwrap();
    assert!(dto.success);

    // SFC API test
    let mut chart = SfcChart::new();
    chart.add_step(SfcStep::new("S1", true));
    chart.add_step(SfcStep::new("S2", false));
    chart.add_transition(SfcTransition::new("T1", "S1", "S2", "TRUE"));
    chart.add_action(SfcAction::new("S1", SfcActionQualifier::N, "PUMP_ON"));

    let sfc_json = serde_json::to_string(&chart).unwrap();
    let sfc_payload = json!({
        "language": "SFC",
        "source_text": sfc_json,
    });

    let req2 = Request::builder()
        .method("POST")
        .uri("/api/v1/controllers/iec/compile")
        .header(header::AUTHORIZATION, "Bearer test_operator")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&sfc_payload).unwrap()))
        .unwrap();

    let resp2 = app.oneshot(req2).await.unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);
    let body2 = axum::body::to_bytes(resp2.into_body(), usize::MAX)
        .await
        .unwrap();
    let dto2: IecCompileResponseDto = serde_json::from_slice(&body2).unwrap();
    assert!(dto2.success);
}
