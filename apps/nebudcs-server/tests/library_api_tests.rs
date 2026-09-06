//! End-to-end integration tests for NebuDCS Phase 11 Equipment Library API endpoints.

use std::collections::BTreeMap;
use std::time::Duration;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use tower::ServiceExt;

use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_protocol::{
    ExportLibraryItemResponseDto, ImportLibraryItemRequestDto, InstantiatePackageRequestDto,
    InstantiateTemplateRequestDto, PackageDetailDto, PackageSummaryDto, TemplateDetailDto,
    TemplateSummaryDto, ValidatePackageResponseDto,
};
use nebudcs_scripting::EquipmentPackage;
use nebudcs_server::{build_app, HostHandle, ServerConfig, SimulationHost};
use nebudcs_sim::SimulationEngine;
use nebudcs_types::EquipmentId;

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
async fn test_library_templates_list_and_filter() {
    let config = ServerConfig {
        simulation_dt: 0.05,
        auth_token: Some("secret123".to_string()),
        ..Default::default()
    };
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle);

    // 1. List all templates (12 standard templates)
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/library/templates")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let templates: Vec<TemplateSummaryDto> = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(
        templates.len(),
        12,
        "Should have 12 standard equipment templates"
    );

    // 2. Filter by rotating category
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/library/templates?category=rotating")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let rotating_templates: Vec<TemplateSummaryDto> = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(rotating_templates.len(), 3, "Pump, Compressor, Fan");
    for t in &rotating_templates {
        assert_eq!(t.category, "Rotating");
        assert_eq!(t.source, "standard");
    }

    // 3. Filter by valve category
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/library/templates?category=valve")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let valve_templates: Vec<TemplateSummaryDto> = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(valve_templates.len(), 1, "Globe control valve");
    assert_eq!(valve_templates[0].id, "standard.valve.control_globe");
}

#[tokio::test]
async fn test_library_template_detail_and_not_found() {
    let config = ServerConfig {
        simulation_dt: 0.05,
        auth_token: Some("secret123".to_string()),
        ..Default::default()
    };
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle);

    // 1. Get existing template detail
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/library/templates/standard.pump.centrifugal")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let detail: TemplateDetailDto = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(detail.id, "standard.pump.centrifugal");
    assert_eq!(detail.category, "Rotating");
    assert!(detail.definition_json.contains("head_flow"));

    // 2. 404 on nonexistent template
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/library/templates/nonexistent.template.id")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_library_template_export_import_roundtrip() {
    let config = ServerConfig {
        simulation_dt: 0.05,
        auth_token: Some("secret123".to_string()),
        ..Default::default()
    };
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle);

    // 1. Export standard compressor template to TOML
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/library/templates/standard.compressor.centrifugal/export?format=toml")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let export_resp: ExportLibraryItemResponseDto = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(export_resp.id, "standard.compressor.centrifugal");
    assert_eq!(export_resp.format, "toml");
    assert!(export_resp
        .content
        .contains("Standard Centrifugal Compressor"));

    // 2. Modify exported TOML to create a new user template and import it
    let user_toml = export_resp
        .content
        .replace(
            "standard.compressor.centrifugal",
            "user.compressor.high_flow",
        )
        .replace(
            "Standard Centrifugal Compressor",
            "High Flow Custom Compressor",
        );

    let import_req = ImportLibraryItemRequestDto {
        content: user_toml,
        format: "toml".to_string(),
    };

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/library/templates/import")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::from(serde_json::to_vec(&import_req).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let imported_summary: TemplateSummaryDto = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(imported_summary.id, "user.compressor.high_flow");

    // Allow background loop to process registration
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Verify imported template is queryable via GET
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/library/templates/user.compressor.high_flow")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. Reject invalid import payload
    let bad_import = ImportLibraryItemRequestDto {
        content: "corrupted toml content {{{".to_string(),
        format: "toml".to_string(),
    };
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/library/templates/import")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::from(serde_json::to_vec(&bad_import).unwrap()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_library_template_instantiate_and_tags() {
    let config = ServerConfig {
        simulation_dt: 0.05,
        auth_token: Some("secret123".to_string()),
        ..Default::default()
    };
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle);

    let mut overrides = BTreeMap::new();
    overrides.insert("rated_flow".to_string(), 150.0);

    let inst_id = EquipmentId::new();
    let inst_req = InstantiateTemplateRequestDto {
        template_id: "standard.pump.centrifugal".to_string(),
        version: None,
        instance_id: Some(inst_id.to_string()),
        name: "PUMP_101".to_string(),
        parameter_overrides: overrides,
    };

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/library/templates/standard.pump.centrifugal/instantiate")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::from(serde_json::to_vec(&inst_req).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Wait for simulation tick loop to dispatch and register tags
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Verify status tag exists in tag store
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/tags/PUMP_101.STATUS")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify speed tag exists
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/tags/PUMP_101.SPEED")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_library_packages_endpoints_and_instantiate() {
    let config = ServerConfig {
        simulation_dt: 0.05,
        auth_token: Some("secret123".to_string()),
        ..Default::default()
    };
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle);

    // 1. List packages (should have 2 standard packages)
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/library/packages")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let pkgs: Vec<PackageSummaryDto> = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(pkgs.len(), 2);

    // 2. Get pump skid package detail
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/library/packages/standard.package.pump_bypass_skid")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let detail: PackageDetailDto = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(detail.id, "standard.package.pump_bypass_skid");
    assert!(detail.package_json.contains("standard.pump.centrifugal"));

    // 3. Export package to JSON
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/library/packages/standard.package.pump_bypass_skid/export?format=json")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let export_resp: ExportLibraryItemResponseDto = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(export_resp.format, "json");

    // 4. Validate package endpoint
    let package_obj: EquipmentPackage = serde_json::from_str(&export_resp.content).unwrap();
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/library/packages/validate")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::from(serde_json::to_vec(&package_obj).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let val_resp: ValidatePackageResponseDto = serde_json::from_slice(&body_bytes).unwrap();
    assert!(val_resp.valid);
    assert_eq!(val_resp.package_id, "standard.package.pump_bypass_skid");
    assert!(val_resp
        .resolved_dependencies
        .contains(&"standard.pump.centrifugal".to_string()));

    // 5. Instantiate package
    let inst_pkg_req = InstantiatePackageRequestDto {
        package_id: "standard.package.pump_bypass_skid".to_string(),
        version: None,
        instance_prefix: "SKID_101".to_string(),
        parameter_overrides: BTreeMap::new(),
    };

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/library/packages/standard.package.pump_bypass_skid/instantiate")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::from(serde_json::to_vec(&inst_pkg_req).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Wait for simulation tick loop to dispatch sub-components
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Verify sub-component tags exist in tag store
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/tags/SKID_101_MAIN_PUMP.STATUS")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/tags/SKID_101_BYPASS_VALVE.STATUS")
        .header(header::AUTHORIZATION, "Bearer secret123")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
