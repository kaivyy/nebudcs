//! Project hierarchy data transfer objects.
//!
//! `Project` is the root of the NebuDCS plant model.  Everything in a
//! simulation lives under a project.
//!
//! # Hierarchy
//!
//! ```text
//! Project
//! └── Plant (1..n)
//!     └── Area (1..n)
//!         └── Unit (1..n)
//!             ├── EquipmentInstance (0..n)
//!             └── StreamDefinition (0..n)
//! ```

use serde::{Deserialize, Serialize};

use nebudcs_types::{
    AreaId, EntityId, EquipmentId, PlantId, PortId, ProjectId, SchemaVersion, UnitId,
};

use crate::equipment::{AlarmPriority, AlarmType, EquipmentInstance, Position2D};
use crate::stream::StreamDefinition;

/// Top-level project metadata.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ProjectMetadata {
    /// Free-text description.
    pub description: String,
    /// Author / organisation.
    pub author: String,
    /// Date created (ISO 8601 string — not parsed for Phase 1).
    pub created: String,
    /// Date last modified.
    pub modified: String,
}

/// An instrument or sensor installed in a unit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstrumentInstance {
    pub id: EntityId,
    /// Measured tag address, e.g. "TI-101", "PI-101", "FT-101", "LT-101".
    pub tag: String,
    /// Associated equipment, if measuring equipment internals/casing.
    #[serde(default)]
    pub equipment_id: Option<EquipmentId>,
    /// Associated port, if measuring at a specific connection nozzle.
    #[serde(default)]
    pub port_id: Option<PortId>,
    /// Measurement category: "Temperature", "Pressure", "Flow", "Level", "Speed", etc.
    pub measurement_kind: String,
    pub range_min: f64,
    pub range_max: f64,
    pub unit: String,
    #[serde(default)]
    pub position: Option<Position2D>,
}

/// A PID or automated control loop configured in the unit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlLoopInstance {
    pub id: EntityId,
    /// Control loop tag identifier, e.g. "LIC-101", "FIC-101", "PIC-101".
    pub name: String,
    /// Process variable tag input.
    pub pv_tag: String,
    /// Initial or target setpoint.
    pub sp: f64,
    /// Manipulated variable tag output (e.g. "FV-101.POS").
    pub out_tag: String,
    /// Proportional gain.
    pub kp: f64,
    /// Integral time constant in seconds.
    pub ti: f64,
    /// Derivative time constant in seconds.
    pub td: f64,
    /// Action direction: true for reverse-acting (cooling/venting), false for direct-acting.
    #[serde(default)]
    pub reverse_acting: bool,
    /// Operating mode: "MANUAL", "AUTO", "CASCADE".
    #[serde(default = "default_control_mode")]
    pub mode: String,
    #[serde(default = "default_output_min")]
    pub output_min: f64,
    #[serde(default = "default_output_max")]
    pub output_max: f64,
    #[serde(default)]
    pub position: Option<Position2D>,
}

fn default_control_mode() -> String {
    "AUTO".to_string()
}
fn default_output_min() -> f64 {
    0.0
}
fn default_output_max() -> f64 {
    100.0
}

/// An alarm limit monitor configured in the unit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlarmInstance {
    pub id: EntityId,
    pub name: String,
    pub pv_tag: String,
    pub alarm_type: AlarmType,
    pub severity: AlarmPriority,
    pub trip_limit: f64,
    #[serde(default)]
    pub deadband: f64,
    #[serde(default)]
    pub description: String,
}

/// An automated safety or process interlock.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterlockInstance {
    pub id: EntityId,
    pub name: String,
    pub source_tag: String,
    /// Condition operator: ">", "<", ">=", "<=", "==".
    pub condition: String,
    pub threshold: f64,
    /// Actuator / target command, e.g. "TRIP", "CLOSE", "STOP".
    pub target_action: String,
    #[serde(default)]
    pub target_equipment: Option<EquipmentId>,
    #[serde(default = "default_true")]
    pub latched: bool,
    #[serde(default)]
    pub description: String,
}

fn default_true() -> bool {
    true
}

/// A process unit within an area — the finest subdivision of the plant
/// hierarchy before individual equipment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Unit {
    pub id: UnitId,
    pub name: String,
    /// Equipment instances that belong to this unit.
    #[serde(default)]
    pub equipment: Vec<EquipmentInstance>,
    /// Stream connections within (and entering/leaving) this unit.
    #[serde(default)]
    pub streams: Vec<StreamDefinition>,
    /// Field instruments and sensors installed in this unit.
    #[serde(default)]
    pub instruments: Vec<InstrumentInstance>,
    /// Closed-loop and cascade controllers configured in this unit.
    #[serde(default)]
    pub control_loops: Vec<ControlLoopInstance>,
    /// Process alarm monitors configured in this unit.
    #[serde(default)]
    pub alarms: Vec<AlarmInstance>,
    /// Safety and equipment interlocks configured in this unit.
    #[serde(default)]
    pub interlocks: Vec<InterlockInstance>,
    /// Process boundary conditions (feed inputs, utilities, product boundaries).
    #[serde(default)]
    pub boundary_conditions: Vec<BoundaryConditionInstance>,
    /// Initial internal conditions (levels, initial pressures/temperatures).
    #[serde(default)]
    pub initial_conditions: Vec<InitialConditionInstance>,
}

/// Boundary condition definition representing an external process source or sink.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundaryConditionInstance {
    pub id: EntityId,
    pub name: String,
    #[serde(default = "default_bc_type")]
    pub condition_type: String, // "feed", "utility", "product_export", "environmental"
    #[serde(default)]
    pub target_equipment_id: Option<EquipmentId>,
    #[serde(default)]
    pub target_port_id: Option<PortId>,
    #[serde(default)]
    pub target_stream_id: Option<String>,
    pub flow_value: f64,
    pub flow_unit: String,
    #[serde(default = "default_bc_pressure")]
    pub pressure_value: f64,
    #[serde(default = "default_bc_pressure_unit")]
    pub pressure_unit: String,
    #[serde(default = "default_bc_temperature")]
    pub temperature_value: f64,
    #[serde(default = "default_bc_temperature_unit")]
    pub temperature_unit: String,
    #[serde(default = "default_bc_phase")]
    pub phase: String,
    #[serde(default)]
    pub density: Option<f64>,
    #[serde(default)]
    pub molecular_weight: Option<f64>,
    #[serde(default)]
    pub enthalpy: Option<f64>,
    #[serde(default)]
    pub quality: Option<f64>,
    #[serde(default)]
    pub composition: indexmap::IndexMap<String, f64>,
    #[serde(default)]
    pub validity_min_flow: Option<f64>,
    #[serde(default)]
    pub validity_max_flow: Option<f64>,
    #[serde(default = "default_bc_provenance")]
    pub source_provenance: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_bc_type() -> String {
    "feed".to_string()
}
fn default_bc_pressure() -> f64 {
    101_325.0
}
fn default_bc_pressure_unit() -> String {
    "Pa".to_string()
}
fn default_bc_temperature() -> f64 {
    25.0
}
fn default_bc_temperature_unit() -> String {
    "°C".to_string()
}
fn default_bc_phase() -> String {
    "Liquid".to_string()
}
fn default_bc_provenance() -> String {
    "User-defined".to_string()
}

/// Explicit initial internal condition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InitialConditionInstance {
    pub id: EntityId,
    pub entity_id: String,
    pub variable: String,
    pub value: f64,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

impl Unit {
    /// Create an empty unit with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: UnitId::new(),
            name: name.into(),
            equipment: Vec::new(),
            streams: Vec::new(),
            instruments: Vec::new(),
            control_loops: Vec::new(),
            alarms: Vec::new(),
            interlocks: Vec::new(),
            boundary_conditions: Vec::new(),
            initial_conditions: Vec::new(),
        }
    }
}

/// A plant area — groups related process units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Area {
    pub id: AreaId,
    pub name: String,
    #[serde(default)]
    pub units: Vec<Unit>,
}

impl Area {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: AreaId::new(),
            name: name.into(),
            units: Vec::new(),
        }
    }
}

/// A plant — a collection of areas sharing a common site or purpose.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Plant {
    pub id: PlantId,
    pub name: String,
    #[serde(default)]
    pub areas: Vec<Area>,
}

impl Plant {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: PlantId::new(),
            name: name.into(),
            areas: Vec::new(),
        }
    }
}

/// Canonical controller configuration within a project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectController {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_watchdog_timeout")]
    pub watchdog_timeout_seconds: f64,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub tasks: Vec<ProjectTask>,
    #[serde(default)]
    pub cpu_profile: Option<ProjectCpuProfile>,
    #[serde(default)]
    pub memory_profile: Option<ProjectMemoryProfile>,
    #[serde(default)]
    pub firmware_profile: Option<ProjectFirmwareProfile>,
    #[serde(default)]
    pub redundancy: Option<ProjectRedundancyConfig>,
    #[serde(default)]
    pub control_modules: Vec<ProjectControlModule>,
}

fn default_watchdog_timeout() -> f64 {
    1.0
}

/// Virtual CPU Hardware Profile configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectCpuProfile {
    /// Nominal execution time budget per scan in microseconds.
    #[serde(default = "default_cpu_budget_us")]
    pub execution_budget_us: u64,
    /// Maximum scan capacity frequency in Hz.
    #[serde(default = "default_scan_capacity_hz")]
    pub max_scan_frequency_hz: f64,
    /// CPU utilization warning threshold percentage (0.0 to 100.0).
    #[serde(default = "default_cpu_utilization_threshold")]
    pub utilization_threshold_pct: f64,
    /// Core architecture identifier (e.g. "VendorNeutral-VirtualVCPU-64").
    #[serde(default = "default_cpu_arch")]
    pub architecture: String,
}

fn default_cpu_budget_us() -> u64 {
    50_000
}
fn default_scan_capacity_hz() -> f64 {
    50.0
}
fn default_cpu_utilization_threshold() -> f64 {
    80.0
}
fn default_cpu_arch() -> String {
    "VendorNeutral-VirtualVCPU-64".to_string()
}

/// Virtual Memory Profile configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectMemoryProfile {
    /// Maximum program memory capacity in bytes.
    #[serde(default = "default_program_memory_bytes")]
    pub program_memory_bytes: usize,
    /// Maximum volatile data memory capacity in bytes.
    #[serde(default = "default_data_memory_bytes")]
    pub data_memory_bytes: usize,
    /// Maximum non-volatile / retentive memory capacity in bytes.
    #[serde(default = "default_retentive_memory_bytes")]
    pub retentive_memory_bytes: usize,
}

fn default_program_memory_bytes() -> usize {
    1024 * 1024 * 16 // 16 MB
}
fn default_data_memory_bytes() -> usize {
    1024 * 1024 * 32 // 32 MB
}
fn default_retentive_memory_bytes() -> usize {
    1024 * 1024 * 4 // 4 MB
}

/// Virtual Controller Firmware Profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFirmwareProfile {
    #[serde(default = "default_firmware_version")]
    pub version: String,
    #[serde(default = "default_build_number")]
    pub build: String,
    #[serde(default = "default_iec_standard")]
    pub iec_standard: String,
}

fn default_firmware_version() -> String {
    "v8.1.0".to_string()
}
fn default_build_number() -> String {
    "2026.09".to_string()
}
fn default_iec_standard() -> String {
    "IEC 61131-3:2013 Ed.3".to_string()
}

/// Redundancy configuration for high-availability controllers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectRedundancyConfig {
    /// Redundancy role: "Standalone", "Primary", "Standby".
    #[serde(default = "default_redundancy_role")]
    pub role: String,
    /// Identifier of the partnered redundant controller.
    #[serde(default)]
    pub partner_controller_id: Option<String>,
    /// State synchronization frequency in Hz.
    #[serde(default = "default_sync_frequency_hz")]
    pub sync_frequency_hz: f64,
    /// Maximum heartbeat loss timeout in seconds before initiating failover switchover.
    #[serde(default = "default_switchover_timeout_seconds")]
    pub switchover_timeout_seconds: f64,
}

fn default_redundancy_role() -> String {
    "Standalone".to_string()
}
fn default_sync_frequency_hz() -> f64 {
    20.0
}
fn default_switchover_timeout_seconds() -> f64 {
    0.1
}

/// DCS Control Module configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectControlModule {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Operating mode: "MAN", "AUTO", "CAS", "RCAS".
    #[serde(default = "default_control_mode")]
    pub mode: String,
    /// Execution period in seconds.
    #[serde(default = "default_period")]
    pub execution_rate_seconds: f64,
    /// PV (Process Variable) signal binding key.
    pub pv_signal: String,
    /// SP (Setpoint) initial value.
    #[serde(default)]
    pub sp: f64,
    /// OUT (Manipulated Variable / Demand) signal binding key.
    pub out_signal: String,
    /// Control strategy: "PID", "Cascade", "Ratio", "Override", "Manual".
    #[serde(default = "default_module_strategy")]
    pub strategy_type: String,
    /// Proportional gain Kp.
    #[serde(default = "default_kp")]
    pub kp: f64,
    /// Integral time Ti (seconds).
    #[serde(default = "default_ti")]
    pub ti: f64,
    /// Derivative time Td (seconds).
    #[serde(default)]
    pub td: f64,
    /// Minimum output limit.
    #[serde(default = "default_output_min")]
    pub output_min: f64,
    /// Maximum output limit.
    #[serde(default = "default_output_max")]
    pub output_max: f64,
    /// Permissive signal keys (must all be true for AUTO/CAS engagement).
    #[serde(default)]
    pub permissives: Vec<String>,
    /// Interlock signal keys (if any true, drives output to safe_position).
    #[serde(default)]
    pub interlocks: Vec<String>,
    /// Safe output position when interlocked.
    #[serde(default)]
    pub safe_position: f64,
}

fn default_module_strategy() -> String {
    "PID".to_string()
}
fn default_kp() -> f64 {
    1.0
}
fn default_ti() -> f64 {
    20.0
}

/// Schedulable task configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectTask {
    pub id: String,
    pub name: String,
    #[serde(default = "default_task_type")]
    pub task_type: String, // "periodic", "event", "continuous"
    #[serde(default = "default_period")]
    pub period_seconds: f64,
    #[serde(default = "default_priority")]
    pub priority: u8,
    #[serde(default = "default_watchdog_timeout")]
    pub watchdog_seconds: f64,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub programs: Vec<ProjectProgram>,
}

fn default_task_type() -> String {
    "periodic".to_string()
}
fn default_period() -> f64 {
    0.05
}
fn default_priority() -> u8 {
    5
}

/// Control program container.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectProgram {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_program_type")]
    pub program_type: String, // "IEC", "PID", "INTERLOCK"
    #[serde(default)]
    pub routines: Vec<ProjectRoutine>,
}

fn default_program_type() -> String {
    "IEC".to_string()
}

/// IEC 61131-3 logic routine definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectRoutine {
    pub id: String,
    pub name: String,
    pub language: String, // "ST", "LD", "FBD", "SFC"
    pub source_text: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// I/O Channel definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectIoChannel {
    pub channel_number: u8,
    pub address: String,
    pub name: String,
    pub kind: String, // "AI", "AO", "DI", "DO"
    #[serde(default)]
    pub engineering_unit: String,
    #[serde(default)]
    pub lrv: f64,
    #[serde(default = "default_urv")]
    pub urv: f64,
    #[serde(default)]
    pub fallback_behavior: Option<String>,
    #[serde(default)]
    pub forced_value: Option<f64>,
}

fn default_urv() -> f64 {
    100.0
}

/// I/O Module card.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectIoModule {
    pub slot: u8,
    pub name: String,
    pub module_type: String,
    #[serde(default)]
    pub channels: Vec<ProjectIoChannel>,
}

/// I/O Chassis / Backplane.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectIoChassis {
    pub chassis_number: u8,
    pub name: String,
    #[serde(default = "default_true")]
    pub bus_ok: bool,
    #[serde(default)]
    pub modules: Vec<ProjectIoModule>,
}

/// I/O Rack cabinet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectIoRack {
    pub rack_number: u8,
    pub name: String,
    #[serde(default = "default_true")]
    pub power_ok: bool,
    #[serde(default)]
    pub chassis: Vec<ProjectIoChassis>,
}

/// Hardware Abstraction Layer I/O system definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectIoSystem {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub racks: Vec<ProjectIoRack>,
}

/// Controller-to-I/O mapping.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectIoMapping {
    pub controller_id: String,
    pub signal_key: String,
    pub channel_address: String,
    pub direction: String, // "input", "output"
    #[serde(default)]
    pub target_tag: Option<String>,
    #[serde(default)]
    pub target_actuator: Option<String>,
    #[serde(default)]
    pub priority: Option<u8>,
}

/// Unified project tag definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectTag {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_tag_datatype")]
    pub data_type: String,
    #[serde(default)]
    pub engineering_unit: String,
    #[serde(default)]
    pub source: String, // "field", "controller", "internal", "sim"
    #[serde(default)]
    pub direction: String, // "input", "output", "internal"
    #[serde(default)]
    pub controller_id: Option<String>,
    #[serde(default)]
    pub channel_address: Option<String>,
    #[serde(default)]
    pub process_entity: Option<String>,
    #[serde(default)]
    pub alarm_id: Option<String>,
}

fn default_tag_datatype() -> String {
    "REAL".to_string()
}

/// Canonical advanced process control strategy within a project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectAdvancedControlStrategy {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub strategy_type: String,
    #[serde(default)]
    pub parameters: std::collections::BTreeMap<String, String>,
}

/// Canonical communications configuration within a project.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ProjectCommunications {
    #[serde(default)]
    pub connections: Vec<ProjectProtocolConnection>,
    #[serde(default)]
    pub tag_mappings: Vec<ProjectTagMapping>,
    #[serde(default)]
    pub zones: Vec<ProjectIndustrialZone>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectProtocolConnection {
    pub id: String,
    pub name: String,
    pub protocol: String,
    pub endpoint: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_zone_id")]
    pub zone_id: String,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_reconnect_ms")]
    pub reconnect_delay_ms: u64,
    #[serde(default)]
    pub use_tls: bool,
}

fn default_zone_id() -> String {
    "zone-l2-control".to_string()
}
fn default_timeout_ms() -> u64 {
    2000
}
fn default_reconnect_ms() -> u64 {
    5000
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectTagMapping {
    pub id: String,
    pub connection_id: String,
    pub external_address: String,
    pub canonical_tag: String,
    #[serde(default = "default_mapping_dir")]
    pub direction: String,
    #[serde(default = "default_tag_datatype")]
    pub datatype: String,
    #[serde(default = "default_one_f64")]
    pub scale: f64,
    #[serde(default)]
    pub offset: f64,
    #[serde(default)]
    pub deadband: f64,
    #[serde(default = "default_rate_ms")]
    pub update_rate_ms: u32,
    #[serde(default = "default_stale_ms")]
    pub stale_timeout_ms: u64,
    #[serde(default = "default_access")]
    pub access: String,
}

fn default_mapping_dir() -> String {
    "inbound".to_string()
}
fn default_one_f64() -> f64 {
    1.0
}
fn default_rate_ms() -> u32 {
    100
}
fn default_stale_ms() -> u64 {
    5000
}
fn default_access() -> String {
    "read_write".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectIndustrialZone {
    pub id: String,
    pub name: String,
    pub level: String,
    #[serde(default)]
    pub permitted_protocols: Vec<String>,
    #[serde(default)]
    pub trust_boundary: bool,
    #[serde(default)]
    pub security_policy: String,
}

/// The root of the NebuDCS domain model.
///
/// A `Project` is a complete, self-contained simulation project.  It carries
/// its schema version so that migrations can be applied on load.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    /// Schema version of the persisted file format.
    pub schema_version: SchemaVersion,
    #[serde(default)]
    pub plants: Vec<Plant>,
    #[serde(default)]
    pub metadata: ProjectMetadata,
    #[serde(default)]
    pub controllers: Vec<ProjectController>,
    #[serde(default)]
    pub io_systems: Vec<ProjectIoSystem>,
    #[serde(default)]
    pub io_mappings: Vec<ProjectIoMapping>,
    #[serde(default)]
    pub tags: Vec<ProjectTag>,
    #[serde(default)]
    pub advanced_control: Vec<ProjectAdvancedControlStrategy>,
    #[serde(default)]
    pub communications: Option<ProjectCommunications>,
    #[serde(default)]
    pub sis: Option<ProjectSisConfig>,
    #[serde(default)]
    pub displays: Vec<ProjectDisplay>,
    #[serde(default)]
    pub navigation_links: Vec<ProjectNavigationLink>,
    #[serde(default)]
    pub plant_structure: Option<ProjectPlantStructure>,
}

impl Project {
    /// Create a new empty project.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: ProjectId::new(),
            name: name.into(),
            schema_version: SchemaVersion::CURRENT,
            plants: Vec::new(),
            metadata: ProjectMetadata::default(),
            controllers: Vec::new(),
            io_systems: Vec::new(),
            io_mappings: Vec::new(),
            tags: Vec::new(),
            advanced_control: Vec::new(),
            communications: None,
            sis: None,
            displays: Vec::new(),
            navigation_links: Vec::new(),
            plant_structure: None,
        }
    }
}

/// Project-level display definition for DCS operator graphics and P&ID overviews.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectDisplay {
    pub id: String,
    pub name: String,
    pub display_type: String,
    #[serde(default)]
    pub area_id: Option<String>,
    #[serde(default)]
    pub system_id: Option<String>,
    #[serde(default)]
    pub subsystem_id: Option<String>,
    #[serde(default)]
    pub unit_id: Option<String>,
    #[serde(default)]
    pub equipment_ids: Vec<String>,
    #[serde(default)]
    pub instrument_ids: Vec<String>,
    #[serde(default)]
    pub stream_ids: Vec<String>,
    #[serde(default)]
    pub viewport_x: Option<f64>,
    #[serde(default)]
    pub viewport_y: Option<f64>,
    #[serde(default)]
    pub viewport_width: Option<f64>,
    #[serde(default)]
    pub viewport_height: Option<f64>,
    #[serde(default)]
    pub viewport_zoom: Option<f64>,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub order: Option<i32>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub placements: Vec<DisplayElementPlacement>,
}

/// Display-specific graphical placement reference for an equipment instance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisplayElementPlacement {
    pub equipment_id: String,
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub width: Option<f64>,
    #[serde(default)]
    pub height: Option<f64>,
    #[serde(default)]
    pub rotation: Option<f64>,
}

fn default_nav_behavior() -> String {
    "focus_and_center".to_string()
}

/// Project-level cross-display navigation link.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectNavigationLink {
    pub id: String,
    pub source_display_id: String,
    #[serde(default)]
    pub source_element_id: Option<String>,
    pub target_display_id: String,
    #[serde(default)]
    pub target_element_id: Option<String>,
    pub label: String,
    #[serde(default = "default_nav_behavior")]
    pub behavior: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Project-level plant hierarchy structure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectPlantStructure {
    pub plant_name: String,
    #[serde(default)]
    pub areas: Vec<ProjectPlantArea>,
    #[serde(default)]
    pub displays: Vec<ProjectDisplay>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectPlantArea {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub systems: Vec<ProjectPlantSystem>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectPlantSystem {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub area_id: Option<String>,
    #[serde(default)]
    pub subsystems: Vec<ProjectPlantSubsystem>,
    #[serde(default)]
    pub default_display_id: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectPlantSubsystem {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub system_id: Option<String>,
    #[serde(default)]
    pub units: Vec<ProjectPlantUnit>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectPlantUnit {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub subsystem_id: Option<String>,
    #[serde(default)]
    pub system_id: Option<String>,
    #[serde(default)]
    pub area_id: Option<String>,
    #[serde(default)]
    pub equipment_ids: Vec<String>,
    #[serde(default)]
    pub display_ids: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Project-level Safety Instrumented System configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectSisConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub sifs: Vec<ProjectSifDefinition>,
}

impl Default for ProjectSisConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sifs: Vec::new(),
        }
    }
}

/// Project SIF definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectSifDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub hazard_id: Option<String>,
    #[serde(default)]
    pub hazard_scenario: Option<String>,
    #[serde(default)]
    pub target_sil: Option<String>,
    #[serde(default = "default_sif_direction")]
    pub direction: String,
    pub threshold: f64,
    #[serde(default)]
    pub reset_deadband: f64,
    #[serde(default)]
    pub trip_delay_seconds: f64,
    #[serde(default = "default_voting_arch")]
    pub architecture: String,
    #[serde(default = "default_degrade_policy")]
    pub degradation_policy: String,
    #[serde(default)]
    pub sensor_tags: Vec<String>,
    #[serde(default)]
    pub final_elements: Vec<ProjectSifFinalElement>,
    #[serde(default = "default_true")]
    pub latched: bool,
    #[serde(default)]
    pub reset_permissives: Vec<String>,
    #[serde(default)]
    pub pfd_avg_estimate: Option<f64>,
}

fn default_sif_direction() -> String {
    "High".to_string()
}
fn default_voting_arch() -> String {
    "2oo3".to_string()
}
fn default_degrade_policy() -> String {
    "DegradeTo1oo2".to_string()
}

/// Project Final Element definition within an SIF.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectSifFinalElement {
    pub target_equipment: String,
    #[serde(default = "default_actuator_var_str")]
    pub actuator_var: String,
    #[serde(default = "default_safe_state_str")]
    pub safe_state: String,
    #[serde(default = "default_true")]
    pub de_energize_to_trip: bool,
    #[serde(default = "default_action_type_str")]
    pub action_type: String,
    #[serde(default)]
    pub feedback_tag: Option<String>,
}

fn default_actuator_var_str() -> String {
    "opening".to_string()
}
fn default_safe_state_str() -> String {
    "ValveClosed".to_string()
}
fn default_action_type_str() -> String {
    "GeneralSafetyTrip".to_string()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    fn build_project() -> Project {
        let mut project = Project::new("Test Refinery");
        let mut plant = Plant::new("CDU Plant");
        let mut area = Area::new("Distillation Area");
        let unit = Unit::new("NHT Unit");
        area.units.push(unit);
        plant.areas.push(area);
        project.plants.push(plant);
        project
    }

    #[test]
    fn test_project_hierarchy() {
        let project = build_project();
        assert_eq!(project.name, "Test Refinery");
        assert_eq!(project.plants.len(), 1);
        assert_eq!(project.plants[0].areas.len(), 1);
        assert_eq!(project.plants[0].areas[0].units.len(), 1);
    }

    #[test]
    fn test_project_serde_json() {
        let project = build_project();
        let json = serde_json::to_string_pretty(&project).expect("serialize");
        let decoded: Project = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(project.name, decoded.name);
        assert_eq!(project.schema_version, decoded.schema_version);
        assert_eq!(project.plants.len(), decoded.plants.len());
    }

    #[test]
    fn test_project_schema_version_is_current() {
        let project = Project::new("test");
        assert_eq!(project.schema_version, SchemaVersion::CURRENT);
    }

    #[test]
    fn test_backward_compatibility_legacy_project() {
        // Legacy JSON without controllers, io_systems, io_mappings, or tags
        let pid = ProjectId::new();
        let legacy_json = format!(
            r#"{{
            "id": "{pid}",
            "name": "Legacy Plant",
            "schema_version": "0.1.0",
            "plants": [],
            "metadata": {{
                "description": "Old project",
                "author": "Tester",
                "created": "2026-09-01T00:00:00Z",
                "modified": "2026-09-01T00:00:00Z"
            }}
        }}"#
        );

        let decoded: Project =
            serde_json::from_str(&legacy_json).expect("Legacy project must deserialize cleanly");
        assert_eq!(decoded.name, "Legacy Plant");
        assert!(decoded.controllers.is_empty());
        assert!(decoded.io_systems.is_empty());
        assert!(decoded.io_mappings.is_empty());
        assert!(decoded.tags.is_empty());
    }

    #[test]
    fn test_unified_project_full_serde() {
        let mut project = Project::new("Unified Engineering Plant");

        // Add controller with task and ST routine
        let mut ctrl = ProjectController {
            id: "C-101".to_string(),
            name: "MainPLC".to_string(),
            description: "Main Plant Controller".to_string(),
            watchdog_timeout_seconds: 0.5,
            enabled: true,
            tasks: Vec::new(),
            cpu_profile: None,
            memory_profile: None,
            firmware_profile: None,
            redundancy: None,
            control_modules: Vec::new(),
        };
        let mut task = ProjectTask {
            id: "T-FAST".to_string(),
            name: "FastLogic".to_string(),
            task_type: "periodic".to_string(),
            period_seconds: 0.02,
            priority: 1,
            watchdog_seconds: 0.05,
            enabled: true,
            programs: Vec::new(),
        };
        let prog = ProjectProgram {
            id: "PROG-PRV".to_string(),
            name: "PressureRelief".to_string(),
            description: "Overpressure control".to_string(),
            program_type: "IEC".to_string(),
            routines: vec![ProjectRoutine {
                id: "ROUT-01".to_string(),
                name: "ReliefST".to_string(),
                language: "ST".to_string(),
                source_text: "IF %IW100 > 10.0 THEN %QW100 := 100.0; ELSE %QW100 := 0.0; END_IF;"
                    .to_string(),
                enabled: true,
            }],
        };
        task.programs.push(prog);
        ctrl.tasks.push(task);
        project.controllers.push(ctrl);

        // Add I/O mapping
        project.io_mappings.push(ProjectIoMapping {
            controller_id: "C-101".to_string(),
            signal_key: "%IW100".to_string(),
            channel_address: "R01-C01-AI01".to_string(),
            direction: "input".to_string(),
            target_tag: Some("PT-101".to_string()),
            target_actuator: None,
            priority: None,
        });

        // Add Tag
        project.tags.push(ProjectTag {
            id: "TAG-01".to_string(),
            name: "PT-101".to_string(),
            description: "Main Reactor Pressure".to_string(),
            data_type: "REAL".to_string(),
            engineering_unit: "bar".to_string(),
            source: "field".to_string(),
            direction: "input".to_string(),
            controller_id: Some("C-101".to_string()),
            channel_address: Some("R01-C01-AI01".to_string()),
            process_entity: Some("V-101".to_string()),
            alarm_id: Some("PAH-101".to_string()),
        });

        let json = serde_json::to_string_pretty(&project).expect("serialize unified project");
        let decoded: Project = serde_json::from_str(&json).expect("deserialize unified project");

        assert_eq!(decoded.controllers.len(), 1);
        assert_eq!(
            decoded.controllers[0].tasks[0].programs[0].routines[0].language,
            "ST"
        );
        assert_eq!(decoded.io_mappings.len(), 1);
        assert_eq!(decoded.tags.len(), 1);
        assert_eq!(decoded.tags[0].name, "PT-101");
    }
}
