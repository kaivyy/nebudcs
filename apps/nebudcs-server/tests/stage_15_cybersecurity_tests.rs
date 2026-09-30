//! Stage 15 Acceptance Test Suite: Industrial Cyber Security & IEC 62443 Alignment.
//!
//! Standards Reference:
//! - IEC 62443-3-2: Security Risk Assessment and System Design (Zones & Conduits).
//! - IEC 62443-3-3: System Security Requirements and Security Levels (SL-1 to SL-4).
//! - FDA 21 CFR Part 11: Electronic Records & Cryptographic Audit Trails.

use nebudcs_protocol::*;
use nebudcs_server::audit_manager::AuditManager;
use nebudcs_server::security::ids::IntrusionDetectionSystem;
use nebudcs_server::security::manager::CyberSecurityManager;
use nebudcs_server::security::purdue::PurdueSecurityEngine;

#[test]
fn test_case_a_purdue_model_zone_segregation() {
    let engine = PurdueSecurityEngine::new();
    let zones = engine.get_zones();

    assert_eq!(zones.len(), 6, "Expected 6 Purdue Model zones");

    // Level 0: Physical Process
    let l0 = zones.iter().find(|z| z.purdue_level == 0).expect("Level 0 zone missing");
    assert_eq!(l0.zone_id, "zone-l0-process");
    assert_eq!(l0.target_security_level, "SL-1");
    assert_eq!(l0.status, "SECURE");

    // Level 1: Field Sensing & Manipulation
    let l1 = zones.iter().find(|z| z.purdue_level == 1).expect("Level 1 zone missing");
    assert_eq!(l1.zone_id, "zone-l1-io");
    assert_eq!(l1.target_security_level, "SL-2");

    // Level 2: Realtime Control & SIS
    let l2 = zones.iter().find(|z| z.purdue_level == 2).expect("Level 2 zone missing");
    assert_eq!(l2.zone_id, "zone-l2-control");
    assert_eq!(l2.target_security_level, "SL-3");

    // Level 3: Operations & SCADA
    let l3 = zones.iter().find(|z| z.purdue_level == 3).expect("Level 3 zone missing");
    assert_eq!(l3.zone_id, "zone-l3-ops");
    assert_eq!(l3.target_security_level, "SL-3");

    // Level 3.5: Industrial DMZ
    let l35 = zones.iter().find(|z| z.purdue_level == 4).expect("Level 3.5 zone missing");
    assert_eq!(l35.zone_id, "zone-l35-idmz");
    assert!(l35.trust_boundary);

    // Level 4/5: Enterprise & Cloud
    let l4 = zones.iter().find(|z| z.purdue_level == 5).expect("Level 4/5 zone missing");
    assert_eq!(l4.zone_id, "zone-l4-enterprise");
}

#[test]
fn test_case_b_conduit_whitelist_and_inspection() {
    let mut engine = PurdueSecurityEngine::new();
    let conduits = engine.get_conduits();

    assert!(conduits.len() >= 5, "Expected at least 5 industrial conduits");

    // Test valid HTTPS traversal across DMZ conduit
    let res = engine.inspect_traffic("cnd-l35-l3", "HTTPS", false);
    assert!(res.is_ok(), "HTTPS must be permitted on DMZ to Operations conduit");

    // Test prohibited protocol on DMZ conduit
    let bad_proto = engine.inspect_traffic("cnd-l35-l3", "TELNET", false);
    assert!(bad_proto.is_err(), "TELNET must be blocked on industrial conduit");

    // Verify conduit violation counter incremented
    let updated_cnds = engine.get_conduits();
    let dmz_cnd = updated_cnds.iter().find(|c| c.conduit_id == "cnd-l35-l3").unwrap();
    assert!(dmz_cnd.violations_blocked >= 1);
    assert!(dmz_cnd.packets_inspected >= 1);
}

#[test]
fn test_case_c_unidirectional_data_diode_enforcement() {
    let mut engine = PurdueSecurityEngine::new();

    // Read telemetry egress to enterprise is allowed
    let read_res = engine.inspect_traffic("cnd-l3-l4", "MQTT_SPARKPLUG_B", false);
    assert!(read_res.is_ok(), "Telemetry read egress to Enterprise must be permitted");

    // Inbound write commands from Enterprise to Operations/Control must be blocked
    let write_res = engine.inspect_traffic("cnd-l3-l4", "MQTT_SPARKPLUG_B", true);
    assert!(write_res.is_err(), "Write commands from Enterprise must be strictly blocked (Data Diode)");
}

#[test]
fn test_case_d_audit_hash_chain_cryptographic_verification() {
    let mut audit = AuditManager::new();

    audit.record(
        "SETPOINT",
        "operator_01",
        "Operator",
        "DCS_Faceplate",
        "FIC-101",
        "SETPOINT_CHANGE",
        Some("50.0".to_string()),
        Some("65.0".to_string()),
        "SUCCESS",
        Some("Process ramp-up".to_string()),
        None,
        10.5,
    );

    audit.record(
        "MODE",
        "operator_01",
        "Operator",
        "DCS_Faceplate",
        "FIC-101",
        "MODE_SWITCH",
        Some("MANUAL".to_string()),
        Some("AUTO".to_string()),
        "SUCCESS",
        Some("Closed loop engagement".to_string()),
        None,
        15.2,
    );

    audit.record(
        "SIS-RESET",
        "supervisor_01",
        "Engineer",
        "SafetyWorkbench",
        "SIF-01",
        "RESET_PROTECTIVE_TRIP",
        Some("TRIPPED".to_string()),
        Some("ARMED".to_string()),
        "SUCCESS",
        Some("Field permissives verified normal".to_string()),
        None,
        25.0,
    );

    let verify_result = audit.verify_integrity();
    assert!(verify_result.is_valid, "Cryptographic audit hash chain must be 100% valid");
    assert!(!verify_result.tampering_detected, "No tampering must be detected");
    assert_eq!(verify_result.total_events_checked, 4); // 1 startup + 3 records
    assert!(!verify_result.root_hash.is_empty());
    assert!(!verify_result.latest_hash.is_empty());
    assert!(verify_result.first_invalid_event_id.is_none());
}

#[test]
fn test_case_e_audit_tamper_detection() {
    let mut audit = AuditManager::new();

    audit.record(
        "CMD-01",
        "operator_01",
        "Operator",
        "DCS_Faceplate",
        "P-101",
        "START_PUMP",
        Some("STOPPED".to_string()),
        Some("RUNNING".to_string()),
        "SUCCESS",
        None,
        None,
        1.0,
    );

    audit.record(
        "CMD-02",
        "operator_01",
        "Operator",
        "DCS_Faceplate",
        "V-101",
        "OPEN_VALVE",
        Some("CLOSED".to_string()),
        Some("OPEN".to_string()),
        "SUCCESS",
        None,
        None,
        2.0,
    );

    // Baseline should verify cleanly
    assert!(audit.verify_integrity().is_valid);

    // Record an event legitimately first
    audit.record(
        "TEST",
        "operator1",
        "Operator",
        "HMI",
        "FIC-101",
        "SETPOINT_CHANGE",
        Some("50.0".to_string()),
        Some("55.0".to_string()),
        "SUCCESS",
        None,
        None,
        3.0,
    );
    assert!(audit.verify_integrity().is_valid);

    // Tamper with the last recorded event directly in the audit log (simulating database alteration)
    audit.tamper_last_event_for_test("MALICIOUS_TAMPERED_ACTION");

    let tampered_result = audit.verify_integrity();
    // Tampering must be immediately caught
    assert!(!tampered_result.is_valid, "Tampered chain must fail cryptographic validation");
    assert!(tampered_result.tampering_detected, "Tampering detected flag must be true");
    assert!(tampered_result.first_invalid_event_id.is_some());

    // Additionally test injecting an event with a broken previous_hash
    let mut audit2 = AuditManager::new();
    let broken_chain_event = AuditEventDto {
        event_id: "INJECTED-000099".to_string(),
        timestamp_iso: "2026-09-08T12:00:00Z".to_string(),
        sim_time_secs: 99.0,
        actor: "attacker".to_string(),
        role: "Operator".to_string(),
        source: "InjectedPayload".to_string(),
        target: "ESD_VALVE".to_string(),
        action: "BYPASS_SAFETY".to_string(),
        prev_value: None,
        new_value: None,
        status: "SUCCESS".to_string(),
        reason: None,
        correlation_id: None,
        previous_hash: Some("FORGED_PREVIOUS_HASH".to_string()),
        event_hash: Some("FORGED_CURRENT_HASH".to_string()),
    };
    audit2.inject_raw_event_unverified(broken_chain_event);
    let tampered_res2 = audit2.verify_integrity();
    assert!(!tampered_res2.is_valid);
    assert!(tampered_res2.tampering_detected);
}

#[test]
fn test_case_f_intrusion_detection_system_incidents() {
    let mut ids = IntrusionDetectionSystem::new();

    assert_eq!(ids.compute_threat_level(), "LOW");

    // Inject high severity incident (role escalation)
    ids.detect_privilege_escalation("trainee_bob", "Trainee", "EXPORT_PROJECT_DATABASE");
    assert_eq!(ids.compute_threat_level(), "LOW"); // 1 high incident is still below threshold 2

    // Inject second high severity incident (conduit violation)
    ids.detect_conduit_violation("cnd-l3-l2", "Unauthorized write from untrusted subnet");
    assert_eq!(ids.compute_threat_level(), "HIGH"); // 2 high incidents triggers HIGH threat level

    // Inject critical path traversal incident
    ids.detect_path_traversal("trainee_bob", "../../../etc/shadow");
    assert_eq!(ids.compute_threat_level(), "CRITICAL"); // Any critical incident triggers CRITICAL

    let incidents = ids.get_incidents(10);
    assert_eq!(incidents.len(), 4); // 1 baseline + 3 injected
    assert_eq!(incidents[0].category, "PATH_TRAVERSAL");
    assert_eq!(incidents[0].severity, "CRITICAL");
}

#[test]
fn test_case_g_cybersecurity_manager_orchestration() {
    let mut sec_mgr = CyberSecurityManager::new();
    let audit_mgr = AuditManager::new();

    let status = sec_mgr.get_security_status(&audit_mgr);
    assert_eq!(status.system_security_level, "SL-3 INDUSTRIAL SECURE (IEC 62443)");
    assert!(status.audit_chain_valid);
    assert!(status.rbac_enforced);
    assert_eq!(status.zones_count, 6);
    assert_eq!(status.conduits_count, 5);

    // Test updating conduit policy
    let update_req = UpdateConduitPolicyRequestDto {
        conduit_id: "cnd-l35-l3".to_string(),
        allowed_protocols: vec!["HTTPS".to_string(), "WSS".to_string(), "GRPC_TLS".to_string()],
        is_encrypted: true,
        status: "ACTIVE".to_string(),
    };

    let ok = sec_mgr.update_conduit_policy(update_req);
    assert!(ok, "Updating conduit policy must succeed");

    let conduits = sec_mgr.get_conduits();
    let dmz_cnd = conduits.iter().find(|c| c.conduit_id == "cnd-l35-l3").unwrap();
    assert!(dmz_cnd.allowed_protocols.contains(&"GRPC_TLS".to_string()));

    // Verify audit log verification integration
    let verify_res = sec_mgr.verify_audit_integrity(&audit_mgr);
    assert!(verify_res.is_valid);
    assert_eq!(verify_res.total_events_checked, 1);
}
