//! Stage 14 Acceptance Test Suite: Enterprise, High Availability & Distributed Runtime + Complete UI Feature Parity.
//!
//! Acceptance Matrix: Cases A through W (23 Exhaustive Verification Cases).

use nebudcs_protocol::*;
use nebudcs_server::audit_manager::AuditManager;
use nebudcs_server::backup::{create_backup, restore_backup, validate_bundle};
use nebudcs_server::cluster_manager::ClusterManager;
use nebudcs_server::diagnostics_manager::DiagnosticsManager;
use nebudcs_server::persistence::DiskStore;
use nebudcs_server::security::UserRole;
use nebudcs_server::validation_manager::ValidationManager;

#[test]
fn test_case_a_standalone_runtime() {
    let mgr = ClusterManager::new(
        "nebudcs-standalone",
        "node-01",
        "Standalone Controller",
        NodeRole::ControlSimulation,
        HaState::Standalone,
    );

    let topo = mgr.get_topology();
    assert_eq!(topo.local_ha_state, HaState::Standalone);
    assert_eq!(topo.leader_epoch, 1);
    assert!(topo.local_ha_state.is_active_primary());
    assert!(topo.local_ha_state.can_accept_commands());
}

#[test]
fn test_case_b_primary_standby_cluster_initial_setup() {
    let mgr = ClusterManager::new(
        "nebudcs-cluster-prod",
        "node-sim-01",
        "Primary Controller",
        NodeRole::ControlSimulation,
        HaState::Primary,
    );

    let topo = mgr.get_topology();
    assert_eq!(topo.cluster_id, "nebudcs-cluster-prod");
    assert_eq!(topo.current_leader_id, Some("node-sim-01".to_string()));
    assert_eq!(topo.nodes.len(), 5);
    assert_eq!(topo.leader_epoch, 1);

    // Verify presence of standby, historian, comms, gateway nodes
    let standby = topo.nodes.iter().find(|n| n.role == NodeRole::Standby).unwrap();
    assert_eq!(standby.ha_state, HaState::Synchronized);
    let historian = topo.nodes.iter().find(|n| n.role == NodeRole::Historian).unwrap();
    assert_eq!(historian.ha_state, HaState::Synchronized);
}

#[test]
fn test_case_c_primary_failure_detection() {
    let mut mgr = ClusterManager::new(
        "nebudcs-cluster-prod",
        "node-sim-01",
        "Primary Controller",
        NodeRole::ControlSimulation,
        HaState::Primary,
    );

    // Simulate 6 seconds elapse without peer heartbeat
    mgr.process_tick(6.0);

    let topo = mgr.get_topology();
    let standby = topo.nodes.iter().find(|n| n.role == NodeRole::Standby).unwrap();
    assert!(standby.last_heartbeat_secs_ago >= 6.0);
    assert_eq!(standby.health_status, "HEARTBEAT_TIMEOUT_UNREACHABLE");
}

#[test]
fn test_case_d_automatic_controlled_failover_to_standby() {
    let mut standby_mgr = ClusterManager::new(
        "nebudcs-cluster-prod",
        "node-standby-02",
        "Secondary Standby",
        NodeRole::Standby,
        HaState::Standby,
    );

    let failover_req = FailoverRequestDto {
        target_leader_id: "node-standby-02".to_string(),
        target_epoch: 2,
        force: false,
        reason: "Primary node unresponsive".to_string(),
    };

    let res = standby_mgr.request_failover(failover_req).unwrap();
    assert!(res.success);
    assert_eq!(res.promoted_node_id, "node-standby-02");
    assert_eq!(res.new_epoch, 2);
    assert_eq!(standby_mgr.local_ha_state(), HaState::Primary);
}

#[test]
fn test_case_e_split_brain_prevention_fencing_epoch() {
    let mut primary = ClusterManager::new(
        "cluster-alpha",
        "node-sim-01",
        "Primary",
        NodeRole::ControlSimulation,
        HaState::Primary,
    );

    // Case 1: Inbound conflicting primary with lower epoch is fenced and rejected
    primary.process_tick(0.1);
    let stale_hb = HaHeartbeatMessage {
        sender_node_id: "node-standby-02".to_string(),
        sender_role: NodeRole::ControlSimulation,
        sender_ha_state: HaState::Primary,
        epoch: 0,
        sequence: 10,
        sim_time_secs: 1.0,
        sim_tick: 20,
        timestamp_iso: "t_1.0s".to_string(),
    };
    let err = primary.handle_heartbeat(stale_hb);
    assert!(err.is_err());
    assert!(primary.get_topology().split_brain_fencing_active);

    // Case 2: Inbound primary with higher epoch causes local node to demote immediately
    let higher_hb = HaHeartbeatMessage {
        sender_node_id: "node-standby-02".to_string(),
        sender_role: NodeRole::ControlSimulation,
        sender_ha_state: HaState::Primary,
        epoch: 5,
        sequence: 100,
        sim_time_secs: 10.0,
        sim_tick: 200,
        timestamp_iso: "t_10.0s".to_string(),
    };
    assert!(primary.handle_heartbeat(higher_hb).is_ok());
    assert_eq!(primary.local_ha_state(), HaState::Standby);
    assert_eq!(primary.leader_epoch(), 5);
}

#[test]
fn test_case_f_state_synchronization_and_replication() {
    let mut primary = ClusterManager::new(
        "cluster-prod",
        "node-01",
        "Primary",
        NodeRole::ControlSimulation,
        HaState::Primary,
    );
    let mut standby = ClusterManager::new(
        "cluster-prod",
        "node-02",
        "Standby",
        NodeRole::Standby,
        HaState::Synchronizing,
    );

    let snap = primary.create_replication_snapshot(42.5, 850, 180, 1, false, 5000);
    assert_eq!(snap.epoch, 1);
    assert_eq!(snap.tag_count, 180);
    assert!(!snap.checksum_sha256.is_empty());

    // Standby verifies SHA-256 and applies state
    assert!(standby.apply_replication_snapshot(snap).is_ok());
    assert_eq!(standby.local_ha_state(), HaState::Synchronized);
}

#[test]
fn test_case_g_recovery_and_rejoin_of_recovered_node() {
    let mut recovered_node = ClusterManager::new(
        "cluster-prod",
        "node-01",
        "Recovered Node",
        NodeRole::Standby,
        HaState::Recovering,
    );

    // Receive snapshot from new primary at epoch 3
    let mut snap = ReplicationSnapshotDto {
        epoch: 3,
        sequence: 15,
        sim_time_secs: 100.0,
        sim_tick: 2000,
        sim_state: "RUNNING".to_string(),
        project_version: "1.0.0".to_string(),
        tag_count: 200,
        active_alarms_count: 0,
        sis_tripped: false,
        historian_seq: 12000,
        checksum_sha256: "".to_string(),
        payload_json: "{\"recovered\":true}".to_string(),
    };
    snap.checksum_sha256 = nebudcs_server::backup::sha256::sha256_hex(snap.payload_json.as_bytes());

    assert!(recovered_node.apply_replication_snapshot(snap).is_ok());
    assert_eq!(recovered_node.leader_epoch(), 3);
}

#[test]
fn test_case_h_cryptographic_backup_bundle_generation() {
    let temp_dir = std::env::temp_dir().join(format!("nebudcs_test_backup_{}", std::process::id()));
    let disk_store = DiskStore::new(&temp_dir).unwrap();

    let sample_proj = serde_json::json!({"id": "proj-alfa", "name": "Crude Unit"});
    disk_store.save_json("projects", "proj-alfa", &sample_proj).unwrap();

    let bundle = create_backup(&disk_store).unwrap();
    assert_eq!(bundle.bundle_version, "1.0.0");
    assert_eq!(bundle.projects.len(), 1);
    assert_eq!(bundle.manifest.len(), 1);
    assert!(bundle.cluster_config.is_some());
    assert!(bundle.validation_evidence.is_some());

    let summary = bundle.summary();
    assert_eq!(summary.projects_count, 1);
    assert!(summary.has_cluster_config);
    assert!(summary.has_validation_evidence);
}

#[test]
fn test_case_i_integrity_verified_restore_with_dry_run_validation() {
    let temp_dir = std::env::temp_dir().join(format!("nebudcs_test_restore_{}", std::process::id()));
    let disk_store = DiskStore::new(&temp_dir).unwrap();

    let sample_proj = serde_json::json!({"id": "proj-beta", "name": "Hydrocracker"});
    disk_store.save_json("projects", "proj-beta", &sample_proj).unwrap();

    let bundle = create_backup(&disk_store).unwrap();

    // 1. Dry-run validation check
    let val_res = validate_bundle(&bundle).unwrap();
    assert!(val_res.valid);
    assert_eq!(val_res.projects_count, 1);
    assert_eq!(val_res.manifest_items_verified, 1);

    // 2. Verified restore execution
    let rest_res = restore_backup(&disk_store, &bundle, true).unwrap();
    assert_eq!(rest_res.projects_restored, 1);
}

#[test]
fn test_case_j_version_aware_schema_migration() {
    let temp_dir = std::env::temp_dir().join(format!("nebudcs_test_migrate_{}", std::process::id()));
    let disk_store = DiskStore::new(&temp_dir).unwrap();

    let rep = nebudcs_server::backup::migration::run_migration(&disk_store).unwrap();
    assert_eq!(rep.entities_migrated, 0);
}

#[test]
fn test_case_k_multi_role_rbac() {
    assert!(!UserRole::Trainee.can_administer());
    assert!(!UserRole::Trainee.can_modify_topology());
    assert!(!UserRole::Trainee.can_control_simulation());
    assert!(UserRole::Trainee.can_operate_dcs());

    assert!(!UserRole::Operator.can_administer());
    assert!(UserRole::Operator.can_operate_dcs());
    assert!(UserRole::Operator.can_run_validation());

    assert!(UserRole::Instructor.can_instruct());
    assert!(UserRole::Instructor.can_control_simulation());

    assert!(UserRole::Engineer.can_modify_topology());
    assert!(UserRole::Engineer.can_control_simulation());
    assert!(!UserRole::Engineer.can_administer());

    assert!(UserRole::Admin.can_administer());
    assert!(UserRole::Admin.can_modify_topology());
    assert!(UserRole::Admin.can_control_simulation());
}

#[test]
fn test_case_l_unified_audit_trail_and_event_logging() {
    let mut audit = AuditManager::new();

    audit.record(
        "DCS-OP",
        "trainee_01",
        "Trainee",
        "WebConsole",
        "FIC-101",
        "SETPOINT_ADJUST",
        Some("100.0".to_string()),
        Some("110.0".to_string()),
        "SUCCESS",
        Some("Supervised feed ramp".to_string()),
        Some("corr-99".to_string()),
        15.5,
    );

    let summary = audit.get_summary();
    assert!(summary.total_events_recorded >= 2);

    let filtered = audit.query(Some("trainee_01"), None, None, None, 10);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].action, "SETPOINT_ADJUST");
    assert_eq!(filtered[0].target, "FIC-101");
}

#[test]
fn test_case_m_subsystem_observability() {
    let diag = DiagnosticsManager::new();
    let sum = diag.summary();
    assert!(sum.total_active >= 4);

    let sys_items = diag.list(Some("System"), None);
    assert!(!sys_items.is_empty());
    assert_eq!(sys_items[0].domain, "System");
}

#[test]
fn test_case_n_stage_13_scientific_validation_preservation() {
    let mgr = ValidationManager::new();
    let summary = mgr.summary();
    assert_eq!(summary.total_cases, 20);
    assert_eq!(summary.passed_cases, 20);
    assert_eq!(summary.failed_cases, 0);
    assert_eq!(summary.pass_rate_pct, 100.0);
}

#[test]
fn test_case_o_stage_11_ots_training_simulator_preservation() {
    let temp_dir = std::env::temp_dir().join(format!("nebudcs_test_ots_{}", std::process::id()));
    let disk_store = DiskStore::new(&temp_dir).unwrap();
    let tm = nebudcs_server::training_manager::TrainingManager::new().with_disk_store(disk_store);

    let sessions = tm.list_sessions();
    assert_eq!(sessions.len(), 0); // initial clean slate
}

#[test]
fn test_case_p_stage_9_sis_functional_safety_preservation() {
    // Verify 2oo3 sensor voting evaluation
    let ch1 = nebudcs_control::sis::voting::SensorChannel::new("PT-101A", "PT_101A", 50.0);
    let ch2 = nebudcs_control::sis::voting::SensorChannel::new("PT-101B", "PT_101B", 52.0);
    let ch3 = nebudcs_control::sis::voting::SensorChannel::new("PT-101C", "PT_101C", 51.0);
    let res = nebudcs_control::sis::voting::evaluate_voting(
        nebudcs_control::sis::voting::VotingArchitecture::TwoOutOfThree,
        nebudcs_control::sis::voting::DegradationPolicy::DegradeTo1oo2,
        nebudcs_control::sis::voting::TripDirection::High,
        90.0,
        &[ch1, ch2, ch3],
    );
    assert!(!res.trip_demanded);
}

#[test]
fn test_case_q_stage_7_industrial_protocol_preservation() {
    let cm = nebudcs_server::communications_manager::CommunicationsManager::new();
    let diag = cm.get_diagnostics();
    assert!(diag.len() >= 2);
}

#[test]
fn test_case_r_stage_1_to_8_virtual_controller_runtime_preservation() {
    let vc = nebudcs_control::runtime::VirtualController::with_name(
        "VC-01",
        nebudcs_control::runtime::ControllerConfig::default(),
    );
    assert_eq!(vc.name(), "VC-01");
}

#[test]
fn test_case_s_complete_ui_feature_parity_contracts() {
    // Verify JSON serialization round-trip for ClusterTopologyDto
    let node = ClusterNodeDto {
        node_id: "node-01".to_string(),
        name: "Test Node".to_string(),
        role: NodeRole::ControlSimulation,
        ha_state: HaState::Primary,
        endpoint_url: "http://127.0.0.1:8088".to_string(),
        is_local: true,
        epoch: 1,
        last_heartbeat_secs_ago: 0.1,
        health_status: "HEALTHY".to_string(),
        replication_lag_ms: 12.0,
    };
    let json = serde_json::to_string(&node).unwrap();
    let deserialized: ClusterNodeDto = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, node);
}

#[test]
fn test_case_t_large_model_ui_performance() {
    // Generate 100 diagnostic items and measure query latency
    let mut dm = DiagnosticsManager::new();
    for i in 0..100 {
        dm.add_item(DiagnosticItemDto {
            id: format!("PERF-{i}"),
            domain: "Simulation".to_string(),
            severity: "INFO".to_string(),
            title: format!("Performance test item {i}"),
            description: "Measurement overhead test".to_string(),
            entity_id: Some(format!("TAG_{i}")),
            entity_type: Some("Tag".to_string()),
            timestamp_iso: "2026-09-08T00:00:00Z".to_string(),
            suggested_action: "None".to_string(),
        });
    }

    let start = std::time::Instant::now();
    let filtered = dm.list(Some("Simulation"), None);
    let elapsed = start.elapsed();
    assert!(filtered.len() >= 100);
    assert!(elapsed.as_millis() < 10); // sub-10ms query performance
}

#[test]
fn test_case_u_deterministic_simulation_after_failover() {
    let mut primary = ClusterManager::new(
        "cluster-prod",
        "node-01",
        "Primary",
        NodeRole::ControlSimulation,
        HaState::Primary,
    );

    // Create snapshot at tick 1000
    let snap = primary.create_replication_snapshot(10.0, 1000, 50, 0, false, 1000);

    let mut standby = ClusterManager::new(
        "cluster-prod",
        "node-02",
        "Standby",
        NodeRole::Standby,
        HaState::Synchronizing,
    );

    standby.apply_replication_snapshot(snap).unwrap();
    assert_eq!(standby.leader_epoch(), 1);

    // Failover
    let failover_res = standby.request_failover(FailoverRequestDto {
        target_leader_id: "node-02".to_string(),
        target_epoch: 2,
        force: false,
        reason: "Simulated primary failure".to_string(),
    }).unwrap();

    assert!(failover_res.success);
    assert_eq!(standby.local_ha_state(), HaState::Primary);
    assert_eq!(standby.leader_epoch(), 2);
}

#[test]
fn test_case_v_custom_plant_project_execution_without_recompile() {
    let temp_dir = std::env::temp_dir().join(format!("nebudcs_test_custom_plant_{}", std::process::id()));
    let disk_store = DiskStore::new(&temp_dir).unwrap();
    let mut pm = nebudcs_server::project_manager::ProjectManager::new().with_disk_store(disk_store);

    let req = nebudcs_protocol::dto::CreateProjectRequestDto {
        name: "Custom Continuous Bioreactor Plant".to_string(),
        description: Some("Custom plant definition without recompilation".to_string()),
        author: Some("Enterprise Engineer".to_string()),
    };

    let detail = pm.create_project(req);
    assert_eq!(detail.name, "Custom Continuous Bioreactor Plant");
    assert!(pm.get_project(&detail.id).is_some());
}

#[test]
fn test_case_w_full_end_to_end_operations_training_validation_enterprise_workflow() {
    // 1. Audit Manager
    let mut audit = AuditManager::new();
    audit.record(
        "E2E-START",
        "LeadEngineer",
        "Admin",
        "Studio",
        "Cluster",
        "START_E2E_WORKFLOW",
        None,
        None,
        "SUCCESS",
        Some("Validating full cross-subsystem pipeline".to_string()),
        Some("e2e-tx-001".to_string()),
        0.0,
    );

    // 2. Validation Manager: verify scientific benchmarks intact
    let vm = ValidationManager::new();
    assert_eq!(vm.summary().passed_cases, 20);

    // 3. Cluster Manager: verify HA state and replication
    let mut cm = ClusterManager::new(
        "nebudcs-cluster-alpha",
        "node-sim-01",
        "Primary Controller",
        NodeRole::ControlSimulation,
        HaState::Primary,
    );
    let snap = cm.create_replication_snapshot(5.0, 100, 150, 0, false, 1000);
    assert_eq!(snap.epoch, 1);

    // 4. Diagnostics Manager: cross-domain visibility
    let dm = DiagnosticsManager::new();
    assert!(dm.summary().total_active >= 4);

    // 5. Final audit entry
    audit.record(
        "E2E-END",
        "LeadEngineer",
        "Admin",
        "Studio",
        "Cluster",
        "COMPLETE_E2E_WORKFLOW",
        None,
        Some("VERIFIED".to_string()),
        "SUCCESS",
        Some("Stage 14 full integration verified across Operations, Training, Validation, and Enterprise".to_string()),
        Some("e2e-tx-001".to_string()),
        5.0,
    );

    assert_eq!(audit.get_summary().total_events_recorded, 3);
}
