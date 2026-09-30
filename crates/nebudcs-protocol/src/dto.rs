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

// ---------------------------------------------------------------------------
// Phase 11 Equipment Library & Template Package DTOs
// ---------------------------------------------------------------------------

/// Port summary metadata for studio canvas and routing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplatePortDto {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub direction: String,
}

/// High-level summary of an equipment template in the library catalog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateSummaryDto {
    pub id: String,
    pub name: String,
    pub version: String,
    pub category: String,
    pub source: String,
    pub description: String,
    pub parameter_count: usize,
    pub port_count: usize,
    pub curve_count: usize,
    pub trip_count: usize,
    pub author: String,
    #[serde(default)]
    pub ports: Vec<TemplatePortDto>,
}

/// Detailed information about an equipment template.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateDetailDto {
    pub id: String,
    pub name: String,
    pub version: String,
    pub category: String,
    pub source: String,
    pub description: String,
    pub author: String,
    pub tags: Vec<String>,
    pub icon_hint: Option<String>,
    pub definition_json: String,
}

/// High-level summary of an equipment package in the library.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackageSummaryDto {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub sub_component_count: usize,
    pub internal_stream_count: usize,
    pub external_port_count: usize,
    pub dependency_count: usize,
}

/// Detailed information about an equipment package.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackageDetailDto {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub package_json: String,
}

/// Pre-registration validation response for equipment packages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidatePackageResponseDto {
    pub valid: bool,
    pub package_id: String,
    pub resolved_dependencies: Vec<String>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Request payload to import a template or package from data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportLibraryItemRequestDto {
    pub content: String,
    #[serde(default = "default_format_json")]
    pub format: String,
}

fn default_format_json() -> String {
    "json".to_string()
}

/// Request payload to instantiate an equipment template.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstantiateTemplateRequestDto {
    pub template_id: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub instance_id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub parameter_overrides: std::collections::BTreeMap<String, f64>,
}

/// Request payload to instantiate an equipment package into the simulation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstantiatePackageRequestDto {
    pub package_id: String,
    #[serde(default)]
    pub version: Option<String>,
    pub instance_prefix: String,
    #[serde(default)]
    pub parameter_overrides: std::collections::BTreeMap<String, f64>,
}

/// Export response payload containing serialized data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportLibraryItemResponseDto {
    pub id: String,
    pub version: String,
    pub format: String,
    pub content: String,
}

// ---------------------------------------------------------------------------
// Phase 13 Engineering Studio & Project DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Position2dDto {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectSummaryDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub schema_version: String,
    pub plant_count: usize,
    pub equipment_count: usize,
    pub stream_count: usize,
    pub created: String,
    pub modified: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StudioEquipmentDto {
    pub id: String,
    pub definition_id: String,
    pub definition_version: String,
    pub name: String,
    #[serde(default)]
    pub parameters: std::collections::BTreeMap<String, f64>,
    #[serde(default)]
    pub position: Option<Position2dDto>,
    #[serde(default)]
    pub display_width: Option<f64>,
    #[serde(default)]
    pub display_height: Option<f64>,
    #[serde(default)]
    pub rotation: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StudioStreamDto {
    pub id: String,
    pub name: String,
    pub stream_type: String,
    pub source_equipment_id: String,
    pub source_port_id: String,
    pub sink_equipment_id: String,
    pub sink_port_id: String,
    #[serde(default)]
    pub composition: std::collections::BTreeMap<String, f64>,
    #[serde(default)]
    pub route_points: Vec<Position2dDto>,
    #[serde(default)]
    pub diameter: Option<f64>,
    #[serde(default)]
    pub length: Option<f64>,
    #[serde(default)]
    pub roughness: Option<f64>,
    #[serde(default)]
    pub elevation_change: Option<f64>,
    #[serde(default)]
    pub loss_k: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StudioInstrumentDto {
    pub id: String,
    pub tag: String,
    #[serde(default)]
    pub equipment_id: Option<String>,
    #[serde(default)]
    pub port_id: Option<String>,
    pub measurement_kind: String,
    pub range_min: f64,
    pub range_max: f64,
    pub unit: String,
    #[serde(default)]
    pub position: Option<Position2dDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StudioControlLoopDto {
    pub id: String,
    pub name: String,
    pub pv_tag: String,
    pub sp: f64,
    pub out_tag: String,
    pub kp: f64,
    pub ti: f64,
    pub td: f64,
    #[serde(default)]
    pub reverse_acting: bool,
    #[serde(default = "default_auto_mode")]
    pub mode: String,
    #[serde(default = "default_zero")]
    pub output_min: f64,
    #[serde(default = "default_hundred")]
    pub output_max: f64,
    #[serde(default)]
    pub position: Option<Position2dDto>,
}

fn default_auto_mode() -> String {
    "AUTO".to_string()
}
fn default_zero() -> f64 {
    0.0
}
fn default_hundred() -> f64 {
    100.0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StudioAlarmDto {
    pub id: String,
    pub name: String,
    pub pv_tag: String,
    pub alarm_type: String,
    pub severity: String,
    pub trip_limit: f64,
    #[serde(default)]
    pub deadband: f64,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StudioInterlockDto {
    pub id: String,
    pub name: String,
    pub source_tag: String,
    pub condition: String,
    pub threshold: f64,
    pub target_action: String,
    #[serde(default)]
    pub target_equipment: Option<String>,
    #[serde(default = "default_true_val")]
    pub latched: bool,
    #[serde(default)]
    pub description: String,
}

fn default_true_val() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectDetailDto {
    pub id: String,
    pub name: String,
    pub schema_version: String,
    pub description: String,
    pub author: String,
    pub created: String,
    pub modified: String,
    pub equipment: Vec<StudioEquipmentDto>,
    pub streams: Vec<StudioStreamDto>,
    pub instruments: Vec<StudioInstrumentDto>,
    pub control_loops: Vec<StudioControlLoopDto>,
    pub alarms: Vec<StudioAlarmDto>,
    pub interlocks: Vec<StudioInterlockDto>,
    #[serde(default)]
    pub controllers: Vec<ControllerConfigDto>,
    #[serde(default)]
    pub io_systems: Vec<HalSystemSummaryDto>,
    #[serde(default)]
    pub io_mappings: Vec<HalMappingDto>,
    #[serde(default)]
    pub tags: Vec<ProjectTagDto>,
    #[serde(default)]
    pub advanced_control: Vec<AdvancedControlStrategyDto>,
    #[serde(default)]
    pub communications: Option<CommunicationsConfigDto>,
    #[serde(default)]
    pub sis: Option<SisConfigDto>,
    #[serde(default)]
    pub plant_structure: Option<PlantStructureDto>,
    #[serde(default)]
    pub areas: Vec<PlantAreaDto>,
    #[serde(default)]
    pub displays: Vec<DisplayDto>,
    #[serde(default)]
    pub navigation_links: Vec<NavigationLinkDto>,
    #[serde(default)]
    pub boundary_conditions: Vec<BoundaryConditionDto>,
    #[serde(default)]
    pub initial_conditions: Vec<InitialConditionDto>,
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

/// Boundary condition DTO representing an external process feed, utility supply, or sink.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundaryConditionDto {
    pub id: String,
    pub name: String,
    #[serde(default = "default_bc_type")]
    pub condition_type: String,
    #[serde(default)]
    pub target_equipment_id: Option<String>,
    #[serde(default)]
    pub target_port: Option<String>,
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
    pub composition: std::collections::BTreeMap<String, f64>,
    #[serde(default)]
    pub validity_min_flow: Option<f64>,
    #[serde(default)]
    pub validity_max_flow: Option<f64>,
    #[serde(default = "default_bc_provenance")]
    pub source_provenance: String,
    #[serde(default = "default_true_val")]
    pub enabled: bool,
}

impl Default for BoundaryConditionDto {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            condition_type: default_bc_type(),
            target_equipment_id: None,
            target_port: None,
            target_stream_id: None,
            flow_value: 0.0,
            flow_unit: "m3/h".to_string(),
            pressure_value: default_bc_pressure(),
            pressure_unit: default_bc_pressure_unit(),
            temperature_value: default_bc_temperature(),
            temperature_unit: default_bc_temperature_unit(),
            phase: default_bc_phase(),
            density: None,
            molecular_weight: None,
            enthalpy: None,
            quality: None,
            composition: std::collections::BTreeMap::new(),
            validity_min_flow: None,
            validity_max_flow: None,
            source_provenance: default_bc_provenance(),
            enabled: true,
        }
    }
}

impl BoundaryConditionDto {
    /// Creates a feed boundary condition targeting equipment.
    pub fn feed(
        id: impl Into<String>,
        name: impl Into<String>,
        target_equipment_id: impl Into<String>,
        flow_value: f64,
        flow_unit: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            condition_type: "feed".to_string(),
            target_equipment_id: Some(target_equipment_id.into()),
            flow_value,
            flow_unit: flow_unit.into(),
            ..Default::default()
        }
    }
}

/// Explicit initial internal condition for equipment or process state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InitialConditionDto {
    pub id: String,
    pub entity_id: String,
    pub variable: String,
    pub value: f64,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Viewport configuration for displays.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewportConfigDto {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub zoom: f64,
}

/// First-class display definition DTO for DCS graphics and P&ID screens.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisplayDto {
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
    pub viewport: Option<ViewportConfigDto>,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub order: Option<i32>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub placements: Vec<DisplayElementPlacementDto>,
}

/// Display-specific graphical placement reference DTO.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisplayElementPlacementDto {
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

fn default_nav_link_behavior() -> String {
    "focus_and_center".to_string()
}

/// Model-driven cross-display navigation link DTO.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NavigationLinkDto {
    pub id: String,
    pub source_display_id: String,
    #[serde(default)]
    pub source_element_id: Option<String>,
    pub target_display_id: String,
    #[serde(default)]
    pub target_element_id: Option<String>,
    pub label: String,
    #[serde(default = "default_nav_link_behavior")]
    pub behavior: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Plant Unit reference mapping DTO.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlantUnitDto {
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

/// Plant Subsystem reference mapping DTO.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlantSubsystemDto {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub system_id: Option<String>,
    #[serde(default)]
    pub units: Vec<PlantUnitDto>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Plant System reference mapping DTO.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlantSystemDto {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub area_id: Option<String>,
    #[serde(default)]
    pub subsystems: Vec<PlantSubsystemDto>,
    #[serde(default)]
    pub default_display_id: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Plant Area reference mapping DTO.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlantAreaDto {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub systems: Vec<PlantSystemDto>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Full operational plant structure DTO.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlantStructureDto {
    pub plant_name: String,
    #[serde(default)]
    pub areas: Vec<PlantAreaDto>,
    #[serde(default)]
    pub displays: Vec<DisplayDto>,
}

/// Canonical communications configuration within a project DTO.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CommunicationsConfigDto {
    #[serde(default)]
    pub connections: Vec<ProtocolConnectionDto>,
    #[serde(default)]
    pub tag_mappings: Vec<TagMappingDto>,
    #[serde(default)]
    pub zones: Vec<IndustrialZoneDto>,
}

/// Advanced process control strategy transport representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdvancedControlStrategyDto {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_true_val")]
    pub enabled: bool,
    /// Strategy kind: "Cascade", "Ratio", "FeedForward", "Override", "SplitRange", "AntiSurge", "ValveCharacterization"
    pub strategy_type: String,
    /// Strategy parameters as key-value pairs (e.g. primary_pv, secondary_pv, wild_pv, ratio, lead_time, lag_time, etc.)
    #[serde(default)]
    pub parameters: std::collections::BTreeMap<String, String>,
}

/// Schedulable task configuration within a virtual controller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskConfigDto {
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
    #[serde(default = "default_true_val")]
    pub enabled: bool,
    #[serde(default)]
    pub programs: Vec<ProgramConfigDto>,
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
fn default_watchdog_timeout() -> f64 {
    1.0
}

/// Control program configuration containing IEC or regulatory routines.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgramConfigDto {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_program_type")]
    pub program_type: String, // "IEC", "PID", "INTERLOCK"
    #[serde(default)]
    pub routines: Vec<IecRoutineDto>,
}

fn default_program_type() -> String {
    "IEC".to_string()
}

/// Virtual CPU Hardware Profile DTO.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CpuProfileDto {
    pub execution_budget_us: u64,
    pub max_scan_frequency_hz: f64,
    pub utilization_threshold_pct: f64,
    pub architecture: String,
}

/// Virtual Memory Profile DTO.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryProfileDto {
    pub program_memory_bytes: usize,
    pub data_memory_bytes: usize,
    pub retentive_memory_bytes: usize,
    #[serde(default)]
    pub program_memory_used_bytes: usize,
    #[serde(default)]
    pub data_memory_used_bytes: usize,
    #[serde(default)]
    pub retentive_memory_used_bytes: usize,
}

/// Virtual Controller Firmware Profile DTO.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FirmwareProfileDto {
    pub version: String,
    pub build: String,
    pub iec_standard: String,
}

/// Redundancy configuration DTO.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedundancyConfigDto {
    pub role: String,
    pub partner_controller_id: Option<String>,
    pub sync_frequency_hz: f64,
    pub switchover_timeout_seconds: f64,
}

/// Redundancy runtime status DTO.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedundancyStatusDto {
    pub role: String,
    pub status: String,
    pub partner_id: Option<String>,
    pub sync_frequency_hz: f64,
    pub switchover_timeout_seconds: f64,
    pub last_sync_sim_time: f64,
    pub sync_cycle_count: u64,
}

fn default_module_strategy() -> String {
    "PID".to_string()
}
fn default_control_mode() -> String {
    "AUTO".to_string()
}

/// DCS Control Module DTO.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DcsControlModuleDto {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_control_mode")]
    pub mode: String,
    pub execution_rate_seconds: f64,
    pub pv_signal: String,
    pub sp: f64,
    pub out_signal: String,
    #[serde(default = "default_module_strategy")]
    pub strategy_type: String,
    pub kp: f64,
    pub ti: f64,
    #[serde(default)]
    pub td: f64,
    pub output_min: f64,
    pub output_max: f64,
    #[serde(default)]
    pub permissives: Vec<String>,
    #[serde(default)]
    pub interlocks: Vec<String>,
    #[serde(default)]
    pub safe_position: f64,
    #[serde(default)]
    pub current_pv: f64,
    #[serde(default)]
    pub current_out: f64,
    #[serde(default)]
    pub is_interlocked: bool,
    #[serde(default = "default_true_val")]
    pub permissives_ok: bool,
    #[serde(default)]
    pub tracking_active: bool,
}

/// Control module action request DTO.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlModuleActionRequestDto {
    pub mode: Option<String>,
    pub sp: Option<f64>,
    pub manual_out: Option<f64>,
}

/// Controlled fault injection request DTO.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControllerFaultInjectionRequestDto {
    pub fault_type: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_true_val")]
    pub recoverable: bool,
}

/// Virtual controller engineering configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControllerConfigDto {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_watchdog_timeout")]
    pub watchdog_timeout_seconds: f64,
    #[serde(default = "default_true_val")]
    pub enabled: bool,
    #[serde(default)]
    pub tasks: Vec<TaskConfigDto>,
    #[serde(default)]
    pub cpu_profile: Option<CpuProfileDto>,
    #[serde(default)]
    pub memory_profile: Option<MemoryProfileDto>,
    #[serde(default)]
    pub firmware_profile: Option<FirmwareProfileDto>,
    #[serde(default)]
    pub redundancy: Option<RedundancyConfigDto>,
    #[serde(default)]
    pub control_modules: Vec<DcsControlModuleDto>,
}

/// Unified project tag definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectTagDto {
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

/// Query for inspecting cross-references of a tag or entity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrossReferenceQueryDto {
    pub tag_or_entity: String,
}

/// Single cross-reference link.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrossReferenceEntryDto {
    pub tag: String,
    pub entity_type: String, // "Instrument", "Tag", "Controller", "Task", "Program", "Routine", "Channel", "Alarm", "ControlLoop"
    pub role: String,        // "Defined", "Read", "Written", "Mapped", "Monitored"
    pub location: String, // e.g. "P&ID (FT-101)", "R01-C01-AI01", "C-101 / FastLogic / PRV_ROUTINE"
    pub details: String,
}

/// Result of a cross-reference query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrossReferenceResultDto {
    pub target: String,
    pub entries: Vec<CrossReferenceEntryDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationResultDto {
    pub severity: String,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectValidationResponseDto {
    pub valid: bool,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub review_count: usize,
    pub results: Vec<ValidationResultDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateProjectRequestDto {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveProjectRequestDto {
    pub project: ProjectDetailDto,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatingRangeDto {
    pub quantity: String,
    pub min_value: f64,
    pub max_value: f64,
    pub unit: String,
    pub normal_operating_value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainPackageSummaryDto {
    pub id: String,
    pub name: String,
    pub version: String,
    pub domain_kind: String,
    pub description: String,
    pub author: String,
    pub plant_template_ids: Vec<String>,
    pub control_strategy_count: usize,
    pub scenario_preset_count: usize,
    pub equipment_template_count: usize,
    #[serde(default)]
    pub scientific_references: Vec<String>,
    #[serde(default)]
    pub model_classifications: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainPackageDetailDto {
    pub id: String,
    pub name: String,
    pub version: String,
    pub domain_kind: String,
    pub description: String,
    pub author: String,
    pub thermodynamic_requirements: Vec<String>,
    pub utility_requirements: Vec<String>,
    pub composition_defaults: std::collections::BTreeMap<String, f64>,
    pub operating_ranges: std::collections::BTreeMap<String, OperatingRangeDto>,
    pub control_strategies: Vec<String>,
    pub scenario_presets: Vec<String>,
    pub equipment_templates: Vec<String>,
    pub plant_template_ids: Vec<String>,
    pub validation_rules: Vec<String>,
    #[serde(default)]
    pub scientific_references: Vec<String>,
    #[serde(default)]
    pub model_classifications: Vec<String>,
}

/// Summary DTO representing a virtual controller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControllerSummaryDto {
    pub id: String,
    pub name: String,
    pub lifecycle: String,
    pub task_count: usize,
    pub scan_count: u64,
    pub avg_scan_duration_us: f64,
    pub max_scan_duration_us: u64,
    pub watchdog_trips: u64,
    pub overrun_count: u64,
    pub is_faulted: bool,
}

/// DTO representing an individual controller task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControllerTaskDto {
    pub id: String,
    pub name: String,
    pub task_type: String,
    pub period_seconds: Option<f64>,
    pub priority: u8,
    pub enabled: bool,
    pub execution_count: u64,
    pub overrun_count: u64,
    pub last_duration_us: u64,
    pub max_duration_us: u64,
    pub program_count: usize,
}

/// DTO representing a controller fault condition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControllerFaultDto {
    pub code: String,
    pub message: String,
    pub timestamp_sim: f64,
    pub recoverable: bool,
}

/// Detailed DTO representing a virtual controller with tasks, diagnostics, and faults.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControllerDetailDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub lifecycle: String,
    pub watchdog_timeout_seconds: f64,
    pub tasks: Vec<ControllerTaskDto>,
    pub input_points: Vec<String>,
    pub output_points: Vec<String>,
    pub scan_count: u64,
    pub task_execution_count: u64,
    pub avg_scan_duration_us: f64,
    pub max_scan_duration_us: u64,
    pub last_scan_duration_us: u64,
    pub watchdog_trips: u64,
    pub overrun_count: u64,
    pub fault: Option<ControllerFaultDto>,
    #[serde(default)]
    pub cpu_profile: Option<CpuProfileDto>,
    #[serde(default)]
    pub memory_profile: Option<MemoryProfileDto>,
    #[serde(default)]
    pub firmware_profile: Option<FirmwareProfileDto>,
    #[serde(default)]
    pub redundancy: Option<RedundancyStatusDto>,
    #[serde(default)]
    pub control_modules: Vec<DcsControlModuleDto>,
}

/// Request DTO for changing a controller's lifecycle state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateControllerStateRequestDto {
    pub action: String, // "start", "stop", "pause", "resume", "reset"
}

/// DTO representing an IEC 61131-3 logic routine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IecRoutineDto {
    pub name: String,
    pub language: String, // "ST", "LD", "FBD", "SFC"
    pub source_text: String,
    pub enabled: bool,
    pub statement_count: usize,
    pub symbol_count: usize,
}

/// Request DTO for compiling and validating an IEC 61131-3 source routine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IecCompileRequestDto {
    pub language: String, // "ST", "LD", "FBD", "SFC"
    pub source_text: String,
}

/// Response DTO containing compilation diagnostics for IEC logic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IecCompileResponseDto {
    pub success: bool,
    pub error_message: Option<String>,
    pub symbol_count: usize,
    pub statement_count: usize,
    pub variables: Vec<IecVariableDto>,
}

/// DTO representing a declared or live IEC variable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IecVariableDto {
    pub name: String,
    pub data_type: String,
    pub value: String,
    pub scope: String,
    pub address_binding: Option<String>,
}

/// DTO for saving or updating an IEC routine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveIecRoutineRequestDto {
    pub name: String,
    pub language: String,
    pub source_text: String,
    pub enabled: bool,
}

/// Diagnostic summary DTO for the I/O Hardware Abstraction Layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HalDiagnosticsDto {
    pub channels_scanned: u64,
    pub faulted_channels: usize,
    pub last_scan_duration_us: u64,
    pub max_scan_duration_us: u64,
    pub avg_scan_duration_us: f64,
}

/// DTO representing an I/O Channel in the HAL hierarchy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HalChannelDto {
    pub id: String,
    pub channel_number: u8,
    pub address: String,
    pub name: String,
    pub kind: String,
    pub value: f64,
    pub raw_counts: i32,
    pub quality: String,
    pub fault_state: String,
    pub status: String,
    pub engineering_unit: String,
    pub lrv: f64,
    pub urv: f64,
    pub fallback_behavior: Option<String>,
    pub forced_value: Option<f64>,
}

/// DTO representing an I/O Module card.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HalModuleDto {
    pub slot: u8,
    pub name: String,
    pub module_type: String,
    pub status: String,
    pub channels: Vec<HalChannelDto>,
}

/// DTO representing an I/O Chassis/Backplane.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HalChassisDto {
    pub chassis_number: u8,
    pub name: String,
    pub bus_ok: bool,
    pub modules: Vec<HalModuleDto>,
}

/// DTO representing an I/O Rack with dual power supplies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HalRackDto {
    pub rack_number: u8,
    pub name: String,
    pub power_ok: bool,
    pub chassis: Vec<HalChassisDto>,
}

/// Summary DTO representing the entire I/O Hardware Abstraction Layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HalSystemSummaryDto {
    pub id: String,
    pub name: String,
    pub rack_count: usize,
    pub module_count: usize,
    pub channel_count: usize,
    pub mapping_count: usize,
    pub diagnostics: HalDiagnosticsDto,
    pub racks: Vec<HalRackDto>,
    pub mappings: Vec<HalMappingDto>,
}

/// DTO representing a Controller-to-I/O Channel mapping.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HalMappingDto {
    pub controller_id: String,
    pub signal_key: String,
    pub channel_address: String,
    pub direction: String,
    pub target_tag: Option<String>,
    pub target_actuator: Option<String>,
    pub priority: Option<u8>,
}

/// Request DTO for injecting or clearing an I/O channel fault or forced value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HalFaultRequestDto {
    pub fault: Option<String>,
    pub forced_value: Option<f64>,
}

/// DTO representing an external industrial protocol connection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolConnectionDto {
    pub id: String,
    pub name: String,
    pub protocol: String,
    pub endpoint: String,
    pub enabled: bool,
    pub state: String,
    pub zone_id: String,
    pub timeout_ms: u64,
    pub reconnect_delay_ms: u64,
    pub use_tls: bool,
    pub active_tags: usize,
    pub last_comm_ms: u64,
}

/// DTO representing a tag mapping between external protocol address and canonical tag.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TagMappingDto {
    pub id: String,
    pub connection_id: String,
    pub external_address: String,
    pub canonical_tag: String,
    pub direction: String,
    pub datatype: String,
    pub scale: f64,
    pub offset: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engineering_unit: Option<String>,
    pub deadband: f64,
    pub update_rate_ms: u32,
    pub stale_timeout_ms: u64,
    pub access: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_value: Option<String>,
    pub quality: String,
}

/// DTO for diagnostic metrics across connections.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolDiagnosticsDto {
    pub connection_id: String,
    pub protocol: String,
    pub state: String,
    pub is_healthy: bool,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub read_operations: u64,
    pub write_operations: u64,
    pub error_count: u64,
    pub reconnect_count: u32,
    pub active_subscriptions: usize,
    pub stale_tags_count: usize,
    pub queue_depth: usize,
    pub last_latency_ms: f64,
    pub last_comm_ms: u64,
}

/// DTO representing an ISA-95 zone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndustrialZoneDto {
    pub id: String,
    pub name: String,
    pub level: String,
    pub permitted_protocols: Vec<String>,
    pub trust_boundary: bool,
    pub security_policy: String,
}

/// DTO for connection test endpoint response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectionTestResultDto {
    pub connection_id: String,
    pub endpoint: String,
    pub protocol: String,
    pub success: bool,
    pub latency_ms: f64,
    pub message: String,
}

// ---------------------------------------------------------------------------
// Stage 9: Safety Instrumented System (SIS) & Functional Safety DTOs
// ---------------------------------------------------------------------------

/// Configuration for the Safety Instrumented System within a project DTO.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SisConfigDto {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub sifs: Vec<SifConfigDto>,
}

/// Static SIF configuration DTO within a project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SifConfigDto {
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
    #[serde(default)]
    pub direction: String,
    pub threshold: f64,
    #[serde(default)]
    pub reset_deadband: f64,
    #[serde(default)]
    pub trip_delay_seconds: f64,
    #[serde(default)]
    pub architecture: String,
    #[serde(default)]
    pub degradation_policy: String,
    #[serde(default)]
    pub sensor_tags: Vec<String>,
    #[serde(default)]
    pub final_elements: Vec<SifFinalElementConfigDto>,
    #[serde(default)]
    pub latched: bool,
    #[serde(default)]
    pub reset_permissives: Vec<String>,
    #[serde(default)]
    pub pfd_avg_estimate: Option<f64>,
}

/// Final element configuration within an SIF DTO.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SifFinalElementConfigDto {
    pub target_equipment: String,
    #[serde(default)]
    pub actuator_var: String,
    #[serde(default)]
    pub safe_state: String,
    #[serde(default)]
    pub de_energize_to_trip: bool,
    #[serde(default)]
    pub action_type: String,
    #[serde(default)]
    pub feedback_tag: Option<String>,
}

/// High-level SIS status summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SisStatusSummaryDto {
    pub enabled: bool,
    pub sif_count: usize,
    pub armed_count: usize,
    pub tripped_count: usize,
    pub bypassed_count: usize,
    pub degraded_count: usize,
    pub proof_tests_count: usize,
    pub active_overrides_count: usize,
}

/// SIF detail DTO for REST queries and Studio workbench visualization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SifDetailDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub hazard_id: Option<String>,
    pub hazard_scenario: Option<String>,
    pub target_sil: Option<String>,
    pub direction: String,
    pub threshold: f64,
    pub reset_deadband: f64,
    pub trip_delay_seconds: f64,
    pub architecture: String,
    pub degradation_policy: String,
    pub state: String,
    pub trip_latched: bool,
    pub sensor_channels: Vec<SifSensorChannelDto>,
    pub final_elements: Vec<SifFinalElementDto>,
    pub diagnostics: SifDiagnosticsDto,
    pub bypass: Option<SifBypassDto>,
    pub pfd_avg_estimate: Option<f64>,
}

/// Sensor channel state inside an SIF.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SifSensorChannelDto {
    pub id: String,
    pub tag: String,
    pub value: f64,
    pub quality: String,
    pub unit: Option<String>,
    pub faulted: bool,
}

/// Final element configuration and status inside an SIF.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SifFinalElementDto {
    pub target_equipment: String,
    pub actuator_var: String,
    pub safe_state: String,
    pub de_energize_to_trip: bool,
    pub action_type: String,
    pub feedback_tag: Option<String>,
}

/// Realtime diagnostics metrics for an SIF.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SifDiagnosticsDto {
    pub total_sensors: usize,
    pub healthy_sensors: usize,
    pub faulted_sensors: usize,
    pub final_elements_count: usize,
    pub trip_count: u64,
    pub last_trip_sim_time: Option<f64>,
    pub last_trip_cause: Option<String>,
    pub last_reset_sim_time: Option<f64>,
    pub voting_scheme: String,
    pub delay_elapsed_seconds: f64,
}

/// SIF administrative bypass info.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SifBypassDto {
    pub bypass_type: String,
    pub authorized_by: String,
    pub reason: String,
    pub start_sim_time: f64,
    pub duration_seconds: Option<f64>,
    pub active: bool,
}

/// Request payload to apply or clear a bypass on an SIF.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SifBypassRequestDto {
    pub bypass_type: String,
    pub authorized_by: String,
    pub reason: String,
    pub duration_seconds: Option<f64>,
    pub active: bool,
}

/// Request payload to reset a tripped SIF.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SifResetRequestDto {
    pub actor: String,
}

/// Request payload to execute a simulated proof test.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProofTestExecutionRequestDto {
    pub technician: String,
}

/// Cause and Effect matrix transport representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CauseEffectMatrixDto {
    pub causes: Vec<CauseEntryDto>,
    pub effects: Vec<EffectEntryDto>,
    pub cells: Vec<CauseEffectCellDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CauseEntryDto {
    pub sif_id: String,
    pub description: String,
    pub hazard_id: Option<String>,
    pub threshold: f64,
    pub voting: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectEntryDto {
    pub equipment_id: String,
    pub variable: String,
    pub safe_state: String,
    pub action_type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CauseEffectCellDto {
    pub cause_index: usize,
    pub effect_index: usize,
    pub active_link: bool,
    pub commanded_state: Option<String>,
    pub is_asserted: bool,
}

/// Proof test record report DTO.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProofTestRecordDto {
    pub test_id: String,
    pub sif_id: String,
    pub timestamp_sim: f64,
    pub passed: bool,
    pub step_results: Vec<ProofTestStepResultDto>,
    pub technician: String,
    pub comments: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProofTestStepResultDto {
    pub component: String,
    pub passed: bool,
    pub observed_value: String,
    pub notes: String,
}

/// ISA-18.2 / IEC 62682 alarm flood and operational health metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlarmFloodMetricsDto {
    /// 10-minute rolling alarm rate (alarms per 10 minutes).
    pub alarm_rate_per_10min: f64,
    /// Whether an alarm flood condition is active (> 10.0 alarms / 10 min per EEMUA 191 / ISA-18.2).
    pub is_flood: bool,
    /// Configured flood threshold (default: 10.0).
    pub flood_threshold_per_10min: f64,
    /// Total currently active alarms.
    pub total_active: usize,
    /// Total active unacknowledged alarms.
    pub unacknowledged: usize,
    /// Total active acknowledged alarms.
    pub acknowledged: usize,
    /// Total shelved alarms.
    pub shelved: usize,
    /// Total suppressed / out-of-service alarms.
    pub suppressed: usize,
    /// Total standing alarms (active continuous).
    pub standing_count: usize,
    /// Total stale alarms (active unacknowledged > 24h or prolonged).
    pub stale_count: usize,
    /// Identifier of first-out initiating alarm if known.
    pub first_out_alarm: Option<String>,
    /// Simulation timestamp of first-out initiating alarm.
    pub first_out_timestamp: Option<f64>,
}

/// Comprehensive alarm summary response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlarmSummaryDto {
    pub metrics: AlarmFloodMetricsDto,
    pub active_alarms: Vec<AlarmSnapshotDto>,
    pub shelved_alarms: Vec<String>,
}

/// Model validity and uncertainty classification for process and equipment data (ISA-101 / Stage 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidityStateDto {
    Validated,
    WithinValidityRange,
    Extrapolated,
    EmpiricalCorrelation,
    Theoretical,
    Approximation,
    PropertyUnavailable,
    BadQuality,
    Stale,
    CommunicationLost,
}

/// Model-driven equipment diagnostics snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquipmentDiagnosticsDto {
    pub equipment_id: String,
    pub equipment_name: String,
    pub equipment_type: String,
    pub validity_state: ValidityStateDto,
    pub state_label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_rpm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow_m3_h: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suction_pressure_kpa: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discharge_pressure_kpa: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head_m: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power_kw: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub efficiency_pct: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub npsha_m: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub npshr_m: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cavitation_margin_m: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cavitation_regime: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vibration_mm_s: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surge_margin_pct: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pressure_ratio: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compressor_operating_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inlet_temp_c: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outlet_temp_c: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thermal_duty_kw: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta_t_c: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lmtd_c: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valve_command_pct: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valve_actual_pct: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valve_fail_action: Option<String>,
    pub is_tripped: bool,
    pub active_alarms: Vec<String>,
    pub active_interlocks: Vec<String>,
    pub sis_override_active: bool,
}

/// Chronological first-out disturbance sequence analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FirstOutEventDto {
    pub run_id: String,
    pub first_out_alarm: Option<String>,
    pub initiating_event_type: Option<String>,
    pub timestamp_sim: Option<f64>,
    pub disturbance_chain: Vec<HistoricalEventDto>,
}

/// Operator action audit log record for IEC 62443 compliance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatorActionAuditDto {
    pub timestamp_sim: f64,
    pub wall_clock_ms: i64,
    pub actor: String,
    pub role: String,
    pub action: String,
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_state: Option<String>,
    pub requested_state: String,
    pub authorized: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
}
