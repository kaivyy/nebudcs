//! Structured DCS event journal representation.

use nebudcs_types::SimulationTime;
use serde::{Deserialize, Serialize};

/// Discrete operational, alarm, protective, or operator intervention event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoricalEvent {
    /// Simulation run identifier.
    pub run_id: String,
    /// Exact simulation timestamp of event occurrence.
    pub sim_time: SimulationTime,
    /// Monotonic sequence discriminator for intra-tick ordering.
    pub seq: u64,
    /// Event category (`"ALARM"`, `"TRIP"`, `"RESET"`, `"MODE"`, `"SETPOINT"`, `"OPERATOR"`, `"LIFECYCLE"`).
    pub event_type: String,
    /// Target or source entity (e.g. tag path, protection block name, controller tag).
    pub source_id: String,
    /// Previous state before transition, if applicable.
    pub old_state: Option<String>,
    /// New active state following transition.
    pub new_state: String,
    /// Associated numeric value at event instant (e.g. measured PV that tripped alarm).
    pub value: Option<f64>,
    /// Criticality severity level (1=Diagnostic, 2=Advisory, 3=Warning, 4=Critical).
    pub severity: Option<u8>,
    /// Optional structured JSON payload with extended event diagnostic parameters.
    pub details_json: Option<String>,
    /// Administrative UTC wall-clock epoch timestamp (milliseconds).
    pub persisted_wall_clock_ms: i64,
}

impl HistoricalEvent {
    /// Creates an alarm state transition event record conforming to ANSI/ISA-18.2.
    #[allow(clippy::too_many_arguments)]
    pub fn alarm(
        run_id: impl Into<String>,
        sim_time: SimulationTime,
        seq: u64,
        alarm_name: impl Into<String>,
        old_state: Option<String>,
        new_state: impl Into<String>,
        pv: f64,
        severity: u8,
        details_json: Option<String>,
        persisted_wall_clock_ms: i64,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            sim_time,
            seq,
            event_type: "ALARM".to_string(),
            source_id: alarm_name.into(),
            old_state,
            new_state: new_state.into(),
            value: Some(pv),
            severity: Some(severity),
            details_json,
            persisted_wall_clock_ms,
        }
    }

    /// Creates a protective interlock trip or ESD event.
    #[allow(clippy::too_many_arguments)]
    pub fn protection_trip(
        run_id: impl Into<String>,
        sim_time: SimulationTime,
        seq: u64,
        block_name: impl Into<String>,
        target_actuator: impl Into<String>,
        safe_position: f64,
        details_json: Option<String>,
        persisted_wall_clock_ms: i64,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            sim_time,
            seq,
            event_type: "TRIP".to_string(),
            source_id: block_name.into(),
            old_state: Some("Normal".to_string()),
            new_state: "Tripped".to_string(),
            value: Some(safe_position),
            severity: Some(4), // Critical
            details_json: details_json.or_else(|| {
                Some(format!(
                    "{{\"target_actuator\":\"{}\"}}",
                    target_actuator.into()
                ))
            }),
            persisted_wall_clock_ms,
        }
    }

    /// Creates a controller mode change event.
    pub fn control_mode(
        run_id: impl Into<String>,
        sim_time: SimulationTime,
        seq: u64,
        loop_name: impl Into<String>,
        old_mode: impl Into<String>,
        new_mode: impl Into<String>,
        persisted_wall_clock_ms: i64,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            sim_time,
            seq,
            event_type: "MODE".to_string(),
            source_id: loop_name.into(),
            old_state: Some(old_mode.into()),
            new_state: new_mode.into(),
            value: None,
            severity: Some(2), // Advisory
            details_json: None,
            persisted_wall_clock_ms,
        }
    }

    /// Creates an operator setpoint intervention event.
    pub fn operator_setpoint(
        run_id: impl Into<String>,
        sim_time: SimulationTime,
        seq: u64,
        loop_name: impl Into<String>,
        old_sp: f64,
        new_sp: f64,
        persisted_wall_clock_ms: i64,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            sim_time,
            seq,
            event_type: "SETPOINT".to_string(),
            source_id: loop_name.into(),
            old_state: Some(format!("{old_sp:.4}")),
            new_state: format!("{new_sp:.4}"),
            value: Some(new_sp),
            severity: Some(2), // Advisory
            details_json: None,
            persisted_wall_clock_ms,
        }
    }
}
