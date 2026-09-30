pub mod admin;
pub mod alarms;
pub mod audit;
pub mod cluster;
pub mod communications;
pub mod control;
pub mod controllers;
pub mod diagnostics;
pub mod equipment;
pub mod hal;
pub mod health;
pub mod history;
pub mod library;
pub mod network;
pub mod observability;
pub mod projects;
pub mod safety;
pub mod scenario;
pub mod security;
pub mod sim;
pub mod tags;
pub mod training;
pub mod validation;
pub mod ws;

use axum::routing::{get, post};
use axum::Router;

use crate::host::HostHandle;

/// Builds the API and WebSocket routes configured with state.
pub fn build_routes() -> Router<HostHandle> {
    Router::new()
        // Health, Liveness, Readiness, and Telemetry Metrics
        .route("/api/v1/health", get(health::get_health))
        .route("/api/v1/health/live", get(health::get_live))
        .route("/api/v1/health/ready", get(health::get_ready))
        .route("/api/v1/metrics", get(health::get_metrics))
        // Hydraulic Network Diagnostics (Stage 6)
        .route(
            "/api/v1/network/diagnostics",
            get(network::get_network_diagnostics),
        )
        // Simulation State and Lifecycle
        .route("/api/v1/sim/snapshot", get(sim::get_snapshot))
        .route("/api/v1/sim/control", post(sim::post_sim_control))
        // Tag Inspection and Mutation
        .route("/api/v1/tags", get(tags::list_tags))
        .route("/api/v1/tags/{tag_id}", get(tags::get_tag))
        .route("/api/v1/tags/write", post(tags::write_tag))
        // Alarm and Protection Systems
        .route("/api/v1/alarms", get(alarms::list_alarms))
        .route("/api/v1/alarms/summary", get(alarms::get_alarm_summary))
        .route(
            "/api/v1/alarms/acknowledge",
            post(alarms::acknowledge_alarm),
        )
        .route("/api/v1/alarms/shelve", post(alarms::shelve_alarm))
        .route("/api/v1/protection/reset", post(alarms::reset_protection))
        // Model-Driven Equipment Diagnostics & Validity (Stage 10)
        .route(
            "/api/v1/diagnostics",
            get(diagnostics::list_equipment_diagnostics),
        )
        .route(
            "/api/v1/diagnostics/{id}",
            get(diagnostics::get_equipment_diagnostics),
        )
        .route(
            "/api/v1/equipment/{id}/diagnostics",
            get(diagnostics::get_equipment_diagnostics),
        )
        // Control Loops
        .route(
            "/api/v1/control/pid/setpoint",
            post(control::set_pid_setpoint),
        )
        .route("/api/v1/control/pid/mode", post(control::set_pid_mode))
        .route(
            "/api/v1/control/pid/output",
            post(control::set_manual_output),
        )
        // Virtual Controller Runtime (Stage 1 & Stage 8)
        .route("/api/v1/controllers", get(controllers::list_controllers))
        .route("/api/v1/controllers/{id}", get(controllers::get_controller))
        .route(
            "/api/v1/controllers/{id}/state",
            post(controllers::update_controller_state),
        )
        .route(
            "/api/v1/controllers/{ctrl_id}/modules/{mod_id}",
            post(controllers::update_control_module),
        )
        .route(
            "/api/v1/controllers/{id}/fault",
            post(controllers::inject_controller_fault),
        )
        .route(
            "/api/v1/controllers/{id}/redundancy/switchover",
            post(controllers::trigger_redundancy_switchover),
        )
        // IEC 61131-3 Logic Runtime (Stage 2)
        .route(
            "/api/v1/controllers/iec/compile",
            post(controllers::compile_iec),
        )
        // I/O Hardware Abstraction Layer (HAL - Stage 3)
        .route("/api/v1/hal/systems", get(hal::list_hal_systems))
        .route("/api/v1/hal/channels/{address}", get(hal::get_hal_channel))
        .route(
            "/api/v1/hal/channels/{address}/fault",
            post(hal::set_channel_fault),
        )
        .route("/api/v1/hal/mappings", get(hal::list_hal_mappings))
        // Industrial Protocols & Communications (Stage 7)
        .route(
            "/api/v1/communications/connections",
            get(communications::list_connections).post(communications::create_connection),
        )
        .route(
            "/api/v1/communications/connections/{id}",
            get(communications::get_connection)
                .put(communications::update_connection)
                .delete(communications::delete_connection),
        )
        .route(
            "/api/v1/communications/connections/{id}/test",
            post(communications::test_connection),
        )
        .route(
            "/api/v1/communications/mappings",
            get(communications::list_mappings).post(communications::create_mapping),
        )
        .route(
            "/api/v1/communications/mappings/{id}",
            get(communications::get_mapping)
                .put(communications::update_mapping)
                .delete(communications::delete_mapping),
        )
        .route(
            "/api/v1/communications/diagnostics",
            get(communications::get_diagnostics),
        )
        .route(
            "/api/v1/communications/architecture",
            get(communications::get_architecture),
        )
        // Safety Instrumented System (SIS - Stage 9)
        .route("/api/v1/sis", get(safety::get_sis_summary))
        .route("/api/v1/sis/summary", get(safety::get_sis_summary))
        .route("/api/v1/sis/sifs", get(safety::list_sifs))
        .route("/api/v1/sis/sifs/{id}", get(safety::get_sif))
        .route("/api/v1/sis/sifs/{id}/reset", post(safety::reset_sif))
        .route("/api/v1/sis/sifs/{id}/bypass", post(safety::set_sif_bypass))
        .route(
            "/api/v1/sis/sifs/{id}/proof-test",
            post(safety::execute_proof_test),
        )
        .route(
            "/api/v1/sis/cause-effect",
            get(safety::get_cause_effect_matrix),
        )
        // Historian Queries
        .route("/api/v1/history/query", get(history::query_history))
        .route("/api/v1/history/events", get(history::query_events))
        .route(
            "/api/v1/history/events/first-out",
            get(history::query_first_out_event),
        )
        // Scenario & Fault Engine
        .route(
            "/api/v1/scenario/status",
            get(scenario::get_scenario_status),
        )
        .route("/api/v1/scenario/load", post(scenario::load_scenario))
        .route("/api/v1/scenario/start", post(scenario::start_scenario))
        .route("/api/v1/scenario/pause", post(scenario::pause_scenario))
        .route("/api/v1/scenario/resume", post(scenario::resume_scenario))
        .route("/api/v1/scenario/abort", post(scenario::abort_scenario))
        .route("/api/v1/scenario/reset", post(scenario::reset_scenario))
        .route(
            "/api/v1/scenario/fault/inject",
            post(scenario::inject_fault),
        )
        .route("/api/v1/scenario/fault/clear", post(scenario::clear_fault))
        // Custom Equipment
        .route(
            "/api/v1/equipment/validate",
            post(equipment::validate_equipment),
        )
        .route(
            "/api/v1/equipment/define",
            post(equipment::define_equipment),
        )
        .route(
            "/api/v1/equipment/definitions",
            get(equipment::list_definitions),
        )
        .route(
            "/api/v1/equipment/definitions/{id}",
            get(equipment::get_definition),
        )
        .route(
            "/api/v1/equipment/instantiate",
            post(equipment::instantiate_equipment),
        )
        .route(
            "/api/v1/equipment/instances",
            get(equipment::list_instances),
        )
        .route(
            "/api/v1/equipment/instances/{id}",
            get(equipment::get_instance),
        )
        .route(
            "/api/v1/equipment/control",
            post(equipment::control_equipment),
        )
        // Equipment Library & Template Packages
        .route(
            "/api/v1/library/templates",
            get(library::list_templates).post(library::register_template),
        )
        .route(
            "/api/v1/library/templates/import",
            post(library::import_template_endpoint),
        )
        .route("/api/v1/library/templates/{id}", get(library::get_template))
        .route(
            "/api/v1/library/templates/{id}/export",
            get(library::export_template_endpoint),
        )
        .route(
            "/api/v1/library/templates/{id}/instantiate",
            post(library::instantiate_template_endpoint),
        )
        .route(
            "/api/v1/library/packages",
            get(library::list_packages).post(library::register_package),
        )
        .route(
            "/api/v1/library/packages/validate",
            post(library::validate_package_endpoint),
        )
        .route(
            "/api/v1/library/packages/import",
            post(library::import_package_endpoint),
        )
        .route("/api/v1/library/packages/{id}", get(library::get_package))
        .route(
            "/api/v1/library/packages/{id}/export",
            get(library::export_package_endpoint),
        )
        .route(
            "/api/v1/library/packages/{id}/instantiate",
            post(library::instantiate_package_endpoint),
        )
        // Industrial Domain Packages (Refinery, Power, Gas, Chemical)
        .route("/api/v1/library/domains", get(library::list_domains))
        .route("/api/v1/library/domains/{id}", get(library::get_domain))
        // Engineering Studio & Project Management
        .route(
            "/api/v1/projects",
            get(projects::list_projects).post(projects::create_project),
        )
        .route(
            "/api/v1/projects/{id}",
            get(projects::get_project)
                .put(projects::save_project)
                .delete(projects::delete_project),
        )
        .route(
            "/api/v1/projects/{id}/validate",
            post(projects::validate_project_endpoint),
        )
        .route(
            "/api/v1/projects/{id}/load-sim",
            post(projects::load_project_sim_endpoint),
        )
        .route(
            "/api/v1/projects/{id}/cross-reference",
            get(projects::get_cross_reference_endpoint),
        )
        .route(
            "/api/v1/projects/{id}/advanced-control",
            get(projects::list_advanced_control_endpoint)
                .put(projects::save_advanced_control_endpoint)
                .post(projects::save_advanced_control_endpoint),
        )
        .route(
            "/api/v1/projects/{id}/advanced-control/{strategy_id}",
            axum::routing::delete(projects::delete_advanced_control_endpoint),
        )
        .route(
            "/api/v1/projects/{id}/communications",
            get(communications::get_project_communications)
                .put(communications::save_project_communications)
                .post(communications::save_project_communications),
        )
        // Training Studio — Scenario Catalog & Authoring
        .route(
            "/api/v1/training/scenarios",
            get(training::list_scenarios).post(training::save_scenario),
        )
        .route(
            "/api/v1/training/scenarios/{id}",
            get(training::get_scenario)
                .put(training::update_scenario)
                .delete(training::delete_scenario),
        )
        .route(
            "/api/v1/training/scenarios/{id}/duplicate",
            post(training::duplicate_scenario),
        )
        .route(
            "/api/v1/training/scenarios/{id}/validate",
            post(training::validate_scenario_endpoint),
        )
        // Training Studio — Session Lifecycle
        .route(
            "/api/v1/training/sessions",
            get(training::list_sessions).post(training::create_session),
        )
        .route("/api/v1/training/sessions/{id}", get(training::get_session))
        .route(
            "/api/v1/training/sessions/{id}/load",
            post(training::load_session_project),
        )
        .route(
            "/api/v1/training/sessions/{id}/prepare",
            post(training::prepare_session),
        )
        .route(
            "/api/v1/training/sessions/{id}/start",
            post(training::start_session),
        )
        .route(
            "/api/v1/training/sessions/{id}/pause",
            post(training::pause_session),
        )
        .route(
            "/api/v1/training/sessions/{id}/resume",
            post(training::resume_session),
        )
        .route(
            "/api/v1/training/sessions/{id}/reset",
            post(training::reset_session),
        )
        .route(
            "/api/v1/training/sessions/{id}/complete",
            post(training::complete_session),
        )
        .route(
            "/api/v1/training/sessions/{id}/abort",
            post(training::abort_session),
        )
        .route(
            "/api/v1/training/sessions/{id}/debrief",
            get(training::get_debrief),
        )
        .route(
            "/api/v1/training/sessions/{id}/branches",
            get(training::get_branches),
        )
        .route(
            "/api/v1/training/sessions/{id}/branches/{branch_id}/trigger",
            post(training::trigger_branch),
        )
        // Training Studio — Trainee Action Recording
        .route(
            "/api/v1/training/sessions/{id}/action",
            post(training::record_trainee_action),
        )
        // Training Studio — Evaluation & Replay
        .route(
            "/api/v1/training/sessions/{id}/evaluation",
            get(training::get_evaluation),
        )
        .route(
            "/api/v1/training/sessions/{id}/replay",
            get(training::get_replay_summary),
        )
        .route(
            "/api/v1/training/sessions/{id}/replay/frame",
            get(training::get_replay_frame),
        )
        // Scientific Validation, Benchmark & Certification (Stage 13)
        .route("/api/v1/validation/summary", get(validation::get_validation_summary))
        .route("/api/v1/validation/cases", get(validation::list_validation_cases))
        .route("/api/v1/validation/cases/{id}", get(validation::get_validation_case))
        .route("/api/v1/validation/run", post(validation::run_validation_benchmarks))
        .route("/api/v1/validation/references", get(validation::list_validation_references))
        .route("/api/v1/validation/models", get(validation::list_validation_models))
        .route("/api/v1/validation/manifest", get(validation::get_validation_manifest))
        .route("/api/v1/validation/custom", post(validation::register_custom_benchmark))
        // Administrative Operations (Guarded by UserRole::Admin)
        .route("/api/v1/admin/backup", post(admin::post_backup))
        .route("/api/v1/admin/backup/validate", post(admin::post_validate_backup))
        .route("/api/v1/admin/restore", post(admin::post_restore))
        .route("/api/v1/admin/migrate", post(admin::post_migrate))
        .route("/api/v1/admin/reload", post(admin::post_reload))
        // Enterprise High Availability & Distributed Cluster (Stage 14)
        .route("/api/v1/cluster/topology", get(cluster::get_topology))
        .route("/api/v1/cluster/heartbeat", post(cluster::post_heartbeat))
        .route("/api/v1/cluster/failover", post(cluster::post_failover))
        .route("/api/v1/cluster/inject-failure", post(cluster::post_inject_failure))
        .route("/api/v1/cluster/clear-failure", post(cluster::post_clear_failure))
        .route("/api/v1/cluster/failure-status", get(cluster::get_failure_status))
        .route("/api/v1/cluster/replication/snapshot", post(cluster::post_replication_snapshot))
        .route("/api/v1/cluster/replication/apply", post(cluster::post_apply_replication))
        // Unified Audit Trail & Security Logs (Stage 14)
        .route("/api/v1/audit/events", get(audit::get_events))
        .route("/api/v1/audit/summary", get(audit::get_summary))
        .route("/api/v1/audit/record", post(audit::post_record))
        // Subsystem Observability & Global Diagnostics (Stage 14)
        .route("/api/v1/observability/telemetry", get(observability::get_telemetry))
        .route("/api/v1/observability/diagnostics", get(observability::get_diagnostics))
        .route("/api/v1/observability/diagnostics/summary", get(observability::get_diagnostics_summary))
        // Industrial Cyber Security & IEC 62443 Alignment (Stage 15)
        .route("/api/v1/security/status", get(security::get_security_status))
        .route("/api/v1/security/zones", get(security::get_zones))
        .route("/api/v1/security/conduits", get(security::get_conduits))
        .route(
            "/api/v1/security/conduits/policy",
            post(security::update_conduit_policy),
        )
        .route("/api/v1/security/incidents", get(security::get_incidents))
        .route(
            "/api/v1/security/audit/verify",
            post(security::verify_audit_integrity),
        )
        // Realtime Streaming WebSocket
        .route("/ws", get(ws::ws_upgrade_handler))
}
