//! Production Hardening Comprehensive Test Suite for NebuDCS Phase 16.
//!
//! Validates:
//! 1. Constant-Time Authentication & Server-Side Role-Based Access Control (RBAC).
//! 2. Path Traversal & Injection Attack Defense.
//! 3. Token-Bucket Rate Limiter & Resource Exhaustion Defense.
//! 4. Atomic Disk Storage & Server Restart Crash Recovery.
//! 5. Cryptographic SHA-256 Manifest Integrity & Backup/Restore Engine.
//! 6. Forward-Compatible Schema Migration Engine.
//! 7. System Health, Liveness, Readiness, and Metrics Endpoints.

use std::time::Duration;
use tempfile::TempDir;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use nebudcs_alarm::AlarmSystem;
use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_control::ControlSystem;
use nebudcs_server::auth::resolve_token_role;
use nebudcs_server::backup::migration::migrate_project_json;
use nebudcs_server::backup::sha256::sha256_hex;
use nebudcs_server::backup::{create_backup, restore_backup};
use nebudcs_server::config::ServerConfig;
use nebudcs_server::host::SimulationHost;
use nebudcs_server::persistence::DiskStore;
use nebudcs_server::project_manager::ProjectManager;
use nebudcs_server::security::roles::{constant_time_eq, sanitize_identifier, UserRole};
use nebudcs_server::security::RateLimiter;
use nebudcs_server::{build_app, ServerError};
use nebudcs_sim::SimulationEngine;

// ============================================================================
// 1. Security & RBAC Tests
// ============================================================================

#[test]
fn test_constant_time_token_comparison() {
    let secret = b"super-secure-production-bearer-token-12345";
    let identical = b"super-secure-production-bearer-token-12345";
    let forged = b"super-secure-production-bearer-token-99999";
    let short = b"super-secure";

    assert!(constant_time_eq(secret, identical));
    assert!(!constant_time_eq(secret, forged));
    assert!(!constant_time_eq(secret, short));
    assert!(!constant_time_eq(secret, b""));
}

#[test]
fn test_role_resolution_from_tokens() {
    let config = ServerConfig {
        admin_token: Some("tok-admin-001".to_string()),
        instructor_token: Some("tok-inst-002".to_string()),
        operator_token: Some("tok-oper-003".to_string()),
        trainee_token: Some("tok-train-004".to_string()),
        ..Default::default()
    };

    // Valid tokens
    assert_eq!(
        resolve_token_role(&config, Some("tok-admin-001")).unwrap(),
        UserRole::Admin
    );
    assert_eq!(
        resolve_token_role(&config, Some("tok-inst-002")).unwrap(),
        UserRole::Instructor
    );
    assert_eq!(
        resolve_token_role(&config, Some("tok-oper-003")).unwrap(),
        UserRole::Operator
    );
    assert_eq!(
        resolve_token_role(&config, Some("tok-train-004")).unwrap(),
        UserRole::Trainee
    );

    // Invalid & missing tokens
    assert!(matches!(
        resolve_token_role(&config, Some("forged-token")),
        Err(ServerError::InvalidCredentials)
    ));
    assert!(matches!(
        resolve_token_role(&config, None),
        Err(ServerError::AuthenticationRequired)
    ));
}

#[test]
fn test_role_hierarchy_and_permissions() {
    // Admin permissions
    assert!(UserRole::Admin.can_administer());
    assert!(UserRole::Admin.can_modify_topology());
    assert!(UserRole::Admin.can_instruct());
    assert!(UserRole::Admin.can_control_simulation());
    assert!(UserRole::Admin.can_operate_dcs());

    // Engineer permissions
    assert!(!UserRole::Engineer.can_administer());
    assert!(UserRole::Engineer.can_modify_topology());
    assert!(!UserRole::Engineer.can_instruct());
    assert!(UserRole::Engineer.can_control_simulation());

    // Instructor permissions
    assert!(!UserRole::Instructor.can_administer());
    assert!(!UserRole::Instructor.can_modify_topology());
    assert!(UserRole::Instructor.can_instruct());
    assert!(UserRole::Instructor.can_control_simulation());

    // Operator permissions
    assert!(!UserRole::Operator.can_administer());
    assert!(!UserRole::Operator.can_modify_topology());
    assert!(!UserRole::Operator.can_instruct());
    assert!(!UserRole::Operator.can_control_simulation());
    assert!(UserRole::Operator.can_operate_dcs());

    // Trainee permissions (strictly confined to operating DCS controls)
    assert!(!UserRole::Trainee.can_administer());
    assert!(!UserRole::Trainee.can_modify_topology());
    assert!(!UserRole::Trainee.can_instruct());
    assert!(!UserRole::Trainee.can_control_simulation());
    assert!(UserRole::Trainee.can_operate_dcs());
}

// ============================================================================
// 2. Path Traversal & Injection Attack Defense Tests
// ============================================================================

#[test]
fn test_path_traversal_sanitizer() {
    // Malicious traversal attempts
    assert!(sanitize_identifier("../../etc/passwd").is_err());
    assert!(sanitize_identifier("..\\..\\windows\\system32").is_err());
    assert!(sanitize_identifier("/bin/sh").is_err());
    assert!(sanitize_identifier("project\0nullbyte").is_err());
    assert!(sanitize_identifier("foo/bar").is_err());
    assert!(sanitize_identifier("foo\\bar").is_err());
    assert!(sanitize_identifier("").is_err());
    assert!(sanitize_identifier("   ").is_err());

    // Valid identifiers
    assert_eq!(
        sanitize_identifier("proj-standard-01").unwrap(),
        "proj-standard-01"
    );
    assert_eq!(
        sanitize_identifier("scenario_feed_loss.v2").unwrap(),
        "scenario_feed_loss.v2"
    );
    assert_eq!(
        sanitize_identifier("tact-01jbq47z8k9m0n1p2q3r4s5t6u").unwrap(),
        "tact-01jbq47z8k9m0n1p2q3r4s5t6u"
    );
}

// ============================================================================
// 3. Rate Limiter Tests
// ============================================================================

#[test]
fn test_token_bucket_rate_limiter() {
    // Bucket with 5 token capacity and 10 tokens/sec replenishment
    let limiter = RateLimiter::new(5, 10.0);
    let client = "192.168.1.100";

    // First 5 requests must succeed immediately (burst capacity)
    for _ in 0..5 {
        assert!(limiter.check_allowed(client));
    }

    // 6th request immediately after burst must be rejected (exhausted)
    assert!(!limiter.check_allowed(client));

    // Wait 250ms -> at 10 tokens/sec, ~2.5 tokens replenished
    std::thread::sleep(Duration::from_millis(250));

    // Request should now be permitted
    assert!(limiter.check_allowed(client));
}

// ============================================================================
// 4. Persistence & Crash Recovery Tests
// ============================================================================

#[test]
fn test_disk_store_atomic_writes_and_crash_recovery() {
    let tmp = TempDir::new().unwrap();
    let store = DiskStore::new(tmp.path()).unwrap();

    // 1. Create a project through ProjectManager with DiskStore attached
    let mut pm = ProjectManager::new().with_disk_store(store.clone());
    let created = pm.create_project(nebudcs_protocol::CreateProjectRequestDto {
        name: "Atomic Persistence Verification Plant".to_string(),
        description: Some("Testing atomic crash recovery".to_string()),
        author: Some("QA Engineer".to_string()),
    });
    let proj_id = created.id.clone();

    // Verify file exists on disk
    let path = store.projects_dir().join(format!("{proj_id}.json"));
    assert!(path.exists());
    assert!(!path.with_extension("json.tmp").exists());

    // 2. Crash recovery simulation: drop `pm`, instantiate brand new ProjectManager on same store
    drop(pm);
    let recovered_pm = ProjectManager::new().with_disk_store(store.clone());
    let projects = recovered_pm.list_projects();
    assert!(
        projects.iter().any(|p| p.id == proj_id),
        "Persisted project must survive manager restart and be reloaded on boot"
    );

    // 3. Delete operation
    assert!(store.delete_json("projects", &proj_id).unwrap());
    let after_delete: Option<serde_json::Value> = store.load_json("projects", &proj_id).unwrap();
    assert_eq!(after_delete, None);
}

// ============================================================================
// 5. SHA-256 Manifest Integrity & Backup/Restore Tests
// ============================================================================

#[test]
fn test_sha256_cryptographic_primitives() {
    // NIST FIPS 180-4 Standard Test Vectors
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn test_backup_generation_integrity_and_restore() {
    let tmp = TempDir::new().unwrap();
    let store = DiskStore::new(tmp.path()).unwrap();

    // Seed test project & scenario
    let project_json = serde_json::json!({
        "id": "proj-integrity-test",
        "name": "Integrity Test Plant",
        "schema_version": "1.0.0"
    });
    store
        .save_json("projects", "proj-integrity-test", &project_json)
        .unwrap();

    let scenario_json = serde_json::json!({
        "id": "scn-integrity-test",
        "name": "Integrity Test Scenario",
        "version": "1.0.0"
    });
    store
        .save_json("scenarios", "scn-integrity-test", &scenario_json)
        .unwrap();

    // 1. Create backup bundle
    let bundle = create_backup(&store).unwrap();
    assert_eq!(bundle.bundle_version, "1.0.0");
    assert_eq!(bundle.manifest.len(), 2);

    // 2. Wipe directory to simulate clean disaster recovery restore
    let restore_tmp = TempDir::new().unwrap();
    let restore_store = DiskStore::new(restore_tmp.path()).unwrap();

    // 3. Tamper detection: modify a bit in the project payload
    let mut tampered_bundle = bundle.clone();
    tampered_bundle.projects.insert(
        "proj-integrity-test".to_string(),
        serde_json::json!({
            "id": "proj-integrity-test",
            "name": "TAMPERED Malicious Payload",
            "schema_version": "1.0.0"
        }),
    );

    let tamper_result = restore_backup(&restore_store, &tampered_bundle, true);
    assert!(
        tamper_result.is_err(),
        "Tampered payload must be detected and rejected by SHA-256 verification"
    );

    // 4. Valid restore
    let restore_result = restore_backup(&restore_store, &bundle, true).unwrap();
    assert_eq!(restore_result.projects_restored, 1);
    assert_eq!(restore_result.scenarios_restored, 1);

    // Verify restored project on disk
    let restored_p: Option<serde_json::Value> = restore_store
        .load_json("projects", "proj-integrity-test")
        .unwrap();
    assert_eq!(restored_p, Some(project_json));
}

// ============================================================================
// 6. Schema Migration Engine Tests
// ============================================================================

#[test]
fn test_schema_migration_engine() {
    let mut legacy_project = serde_json::json!({
        "id": "proj-legacy",
        "name": "Legacy Plant",
        "schema_version": "0.1.0",
        "plants": [
            {
                "name": "Plant A",
                "areas": [
                    {
                        "name": "Area A",
                        "units": [
                            {
                                "name": "Unit A",
                                "equipment": [
                                    {
                                        "id": "eq-1",
                                        "name": "Pump 1"
                                    }
                                ]
                            }
                        ]
                    }
                ]
            }
        ]
    });

    assert!(migrate_project_json(&mut legacy_project));

    // Check migrated fields
    assert_eq!(legacy_project["schema_version"], "1.0.0");
    assert_eq!(
        legacy_project["plants"][0]["areas"][0]["units"][0]["equipment"][0]["definition_version"]
            ["major"],
        1
    );

    // Running migration on already-current schema should return false (no-op)
    assert!(!migrate_project_json(&mut legacy_project));
}

// ============================================================================
// 7. HTTP API Integration & Role Guard Endpoints
// ============================================================================

#[tokio::test]
async fn test_admin_and_health_routes_integration() {
    let tmp = TempDir::new().unwrap();

    let config = ServerConfig {
        admin_token: Some("adm-secret-token".to_string()),
        trainee_token: Some("trn-secret-token".to_string()),
        data_dir: tmp.path().to_path_buf(),
        ..Default::default()
    };

    let engine = SimulationEngine::new(config.simulation_dt).unwrap();
    let tag_store = RuntimeTagStore::new();
    let alarm_sys = AlarmSystem::new();
    let ctrl_sys = ControlSystem::new();

    let mut host = SimulationHost::start(
        config.clone(),
        engine,
        tag_store,
        Some(alarm_sys),
        Some(ctrl_sys),
        None,
    )
    .unwrap();

    let app = build_app(host.handle().clone());

    // 1. Health Liveness Probe (no auth required)
    let req = Request::builder()
        .uri("/api/v1/health/live")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 2. Health Readiness Probe (no auth required)
    let req = Request::builder()
        .uri("/api/v1/health/ready")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. Telemetry Metrics Probe (no auth required)
    let req = Request::builder()
        .uri("/api/v1/metrics")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 4. Admin Backup with Trainee Token -> Expect HTTP 403 Forbidden
    let req = Request::builder()
        .uri("/api/v1/admin/backup")
        .method("POST")
        .header("Authorization", "Bearer trn-secret-token")
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "Trainee must be denied access to /api/v1/admin/backup"
    );

    // 5. Admin Backup with Admin Token -> Expect HTTP 200 OK
    let req = Request::builder()
        .uri("/api/v1/admin/backup")
        .method("POST")
        .header("Authorization", "Bearer adm-secret-token")
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Admin must have access to /api/v1/admin/backup"
    );

    // Stop host thread
    host.stop();
}
