//! Data Transfer Objects (DTOs) representing externalized simulation state.

use serde::{Deserialize, Serialize};

use crate::command::TagValueDto;

/// Transport representation of signal quality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TagQualityDto {
    Good,
    Uncertain,
    Bad,
    NotConnected,
}

/// A single tag value and quality sample.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TagValueSampleDto {
    pub tag: String,
    pub value: TagValueDto,
    pub quality: TagQualityDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

/// ANSI/ISA-18.2 alarm lifecycle state for transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AlarmStateDto {
    Normal,
    ActiveUnack,
    ActiveAck,
    Cleared,
    Shelved,
}

/// Alarm criticality level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AlarmSeverityDto {
    Diagnostic,
    Advisory,
    Warning,
    Critical,
}

/// Summary snapshot of an individual alarm.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlarmSnapshotDto {
    pub alarm_name: String,
    pub state: AlarmStateDto,
    pub severity: AlarmSeverityDto,
    pub pv: f64,
    pub trip_limit: f64,
}

/// Full point-in-time state snapshot delivered upon client connection or resynchronization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateSnapshotDto {
    /// Unique identifier for this snapshot.
    pub snapshot_id: String,
    /// Simulation time at snapshot creation.
    pub sim_time_sec: f64,
    /// Monotonic tick count at snapshot creation.
    pub tick_count: u64,
    /// Complete array of process and control tags.
    pub tags: Vec<TagValueSampleDto>,
    /// All currently active or unacknowledged alarms.
    pub active_alarms: Vec<AlarmSnapshotDto>,
}

/// Incremental delta update containing only tags that changed during the tick.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeltaUpdateDto {
    /// Monotonic update sequence counter for gap detection.
    pub seq: u64,
    /// Current simulation timestamp.
    pub sim_time_sec: f64,
    /// Simulation tick counter.
    pub tick_count: u64,
    /// Subset of tags that changed value or quality.
    pub changed_tags: Vec<TagValueSampleDto>,
}

/// Historical discrete event representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoricalEventDto {
    pub sim_time_sec: f64,
    pub seq: u64,
    pub event_type: String,
    pub source_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_state: Option<String>,
    pub new_state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details_json: Option<String>,
    pub wall_clock_ms: i64,
}

/// Diagnostic health snapshot of the NebuDCS server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerHealthDto {
    pub status: String,
    pub simulation_running: bool,
    pub sim_time_sec: f64,
    pub tick_count: u64,
    pub simulation_dt: f64,
    pub speed_multiplier: f64,
    pub connected_clients: usize,
    pub protocol_version: String,
    pub historian_healthy: bool,
}

/// Active fault state in transport format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveFaultDto {
    pub id: String,
    pub target: String,
    pub category: String,
    pub elapsed_sec: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_sec: Option<f64>,
}

/// Scenario state in transport format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioStatusDto {
    pub scenario_id: Option<String>,
    pub scenario_name: Option<String>,
    pub state: String,
    pub elapsed_sec: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_limit_sec: Option<f64>,
    pub active_faults: Vec<ActiveFaultDto>,
    pub active_disturbances_count: usize,
    pub events_total: usize,
    pub events_fired: usize,
    pub score: f64,
    pub mandatory_completed: bool,
}

/// High-level summary of a registered custom equipment definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquipmentDefinitionSummaryDto {
    pub id: String,
    pub name: String,
    pub version: String,
    pub category: String,
    pub tier: String,
    pub description: String,
    pub parameter_count: usize,
    pub port_count: usize,
    pub curve_count: usize,
    pub trip_count: usize,
    pub interlock_count: usize,
    pub has_script: bool,
    pub has_composition: bool,
}

/// State summary of an active custom equipment instance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquipmentInstanceSummaryDto {
    pub id: String,
    pub definition_id: String,
    pub name: String,
    pub active_state: String,
    pub is_running: bool,
    pub is_tripped: bool,
    pub latched_trips: Vec<String>,
    pub parameters: std::collections::BTreeMap<String, f64>,
    pub variables: std::collections::BTreeMap<String, f64>,
    pub flags: std::collections::BTreeMap<String, bool>,
}

/// Pre-execution validation response for custom equipment definitions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidateEquipmentResponseDto {
    pub valid: bool,
    pub definition_id: String,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Request payload to instantiate a custom equipment definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstantiateEquipmentRequestDto {
    #[serde(default)]
    pub instance_id: Option<String>,
    pub definition_id: String,
    pub name: String,
    #[serde(default)]
    pub parameter_overrides: std::collections::BTreeMap<String, f64>,
}

/// Request payload to command custom equipment state or variable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquipmentControlRequestDto {
    pub instance_id: String,
    pub action: String,
    #[serde(default)]
    pub variable_name: Option<String>,
    #[serde(default)]
    pub value: Option<f64>,
}
