//! HTTP and WebSocket route configuration and builder.

pub mod alarms;
pub mod control;
pub mod equipment;
pub mod health;
pub mod history;
pub mod scenario;
pub mod sim;
pub mod tags;
pub mod ws;

use axum::routing::{get, post};
use axum::Router;

use crate::host::HostHandle;

/// Builds the API and WebSocket routes configured with state.
pub fn build_routes() -> Router<HostHandle> {
    Router::new()
        // Health and Diagnostics
        .route("/api/v1/health", get(health::get_health))
        // Simulation State and Lifecycle
        .route("/api/v1/sim/snapshot", get(sim::get_snapshot))
        .route("/api/v1/sim/control", post(sim::post_sim_control))
        // Tag Inspection and Mutation
        .route("/api/v1/tags", get(tags::list_tags))
        .route("/api/v1/tags/{tag_id}", get(tags::get_tag))
        .route("/api/v1/tags/write", post(tags::write_tag))
        // Alarm and Protection Systems
        .route("/api/v1/alarms", get(alarms::list_alarms))
        .route(
            "/api/v1/alarms/acknowledge",
            post(alarms::acknowledge_alarm),
        )
        .route("/api/v1/alarms/shelve", post(alarms::shelve_alarm))
        .route("/api/v1/protection/reset", post(alarms::reset_protection))
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
        // Historian Queries
        .route("/api/v1/history/query", get(history::query_history))
        .route("/api/v1/history/events", get(history::query_events))
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
        // Realtime Streaming WebSocket
        .route("/ws", get(ws::ws_upgrade_handler))
}
