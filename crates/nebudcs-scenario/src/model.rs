//! Declarative data model for scenarios, faults, disturbances, and assessment criteria.

use nebudcs_types::SchemaVersion;
use serde::{Deserialize, Serialize};

/// Current schema version for NebuDCS scenario files.
pub const SCENARIO_SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(1, 0, 0);

/// Primary declarative scenario definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioDefinition {
    /// Unique scenario identifier (ULID or alphanumeric identifier).
    pub id: String,
    /// Human-readable title of the scenario.
    pub name: String,
    /// Detailed description of the operational upset or training objective.
    pub description: String,
    /// Author / instructor identifier.
    pub author: String,
    /// Scenario file format version for schema migrations.
    pub schema_version: SchemaVersion,
    /// Optional scenario time limit in simulation seconds.
    pub duration_limit_sec: Option<f64>,
    /// Deterministic pseudo-random seed for disturbances and noise.
    #[serde(default = "default_seed")]
    pub random_seed: u64,
    /// Initial condition overrides applied prior to tick execution.
    #[serde(default)]
    pub initial_conditions: InitialConditions,
    /// Chronological / condition-triggered sequence of events.
    #[serde(default)]
    pub timeline: Vec<ScenarioEvent>,
    /// Evaluation objectives and grading hooks.
    #[serde(default)]
    pub objectives: Vec<ScenarioObjective>,
}

fn default_seed() -> u64 {
    0x42_1337_cafe
}

/// Initial condition snapshot applied deterministically before scenario launch.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InitialConditions {
    /// Initial tag value overrides.
    #[serde(default)]
    pub tags: Vec<TagInitialValue>,
    /// Controller setpoints and operational modes.
    #[serde(default)]
    pub controllers: Vec<ControllerInitialState>,
    /// Equipment operational state overrides.
    #[serde(default)]
    pub equipment: Vec<EquipmentInitialState>,
}

/// Initial value assigned to a specific runtime tag.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TagInitialValue {
    pub tag: String,
    pub value: f64,
}

/// Initial state assigned to a DCS/PLC control loop.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControllerInitialState {
    pub loop_name: String,
    pub mode: Option<String>, // e.g. "MAN", "AUTO", "CAS"
    pub sp: Option<f64>,
    pub manual_output: Option<f64>,
}

/// Initial state assigned to an equipment unit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquipmentInitialState {
    pub equipment_name: String,
    pub speed_rpm: Option<f64>,
    pub is_running: Option<bool>,
}

/// A timed or condition-triggered scenario event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioEvent {
    pub id: String,
    /// Priority discriminator for deterministic tie-breaking of simultaneous events.
    #[serde(default)]
    pub priority: u32,
    /// Trigger condition that fires this event.
    pub trigger: EventTrigger,
    /// Action executed upon firing.
    pub action: ScenarioAction,
}

/// Condition determining when a scenario event fires.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventTrigger {
    /// Triggers at or after a specific elapsed scenario time.
    Time { elapsed_sec: f64 },
    /// Triggers dynamically when a runtime tag value satisfies a condition.
    TagCondition {
        tag: String,
        operator: ComparisonOp,
        value: f64,
    },
}

/// Comparison operators for condition-based event triggers and evaluation criteria.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOp {
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Equal,
    NotEqual,
}

impl ComparisonOp {
    /// Evaluates `actual` compared to `target` using this operator with epsilon tolerance.
    pub fn evaluate(self, actual: f64, target: f64) -> bool {
        const EPSILON: f64 = 1e-7;
        match self {
            ComparisonOp::GreaterThan => actual > target,
            ComparisonOp::GreaterThanOrEqual => actual >= (target - EPSILON),
            ComparisonOp::LessThan => actual < target,
            ComparisonOp::LessThanOrEqual => actual <= (target + EPSILON),
            ComparisonOp::Equal => (actual - target).abs() <= EPSILON,
            ComparisonOp::NotEqual => (actual - target).abs() > EPSILON,
        }
    }
}

/// Action performed when a scenario event triggers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScenarioAction {
    /// Injects a fault into equipment, instrument, or process.
    InjectFault(FaultDefinition),
    /// Explicitly clears an active fault by ID.
    ClearFault { fault_id: String },
    /// Applies a continuous or discrete disturbance.
    ApplyDisturbance(DisturbanceDefinition),
    /// Simulates a pre-scripted operator action.
    ExecuteOperatorAction(OperatorAction),
    /// Emits a persistent scenario marker to the historian.
    EmitMarker { label: String, message: String },
}

/// Generic fault definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FaultDefinition {
    /// Unique fault identifier.
    pub id: String,
    /// Target equipment or tag name.
    pub target: String,
    /// Fault type and parameters.
    pub fault_type: FaultType,
    /// Optional duration in seconds; None indicates permanent until cleared.
    pub duration_sec: Option<f64>,
}

/// Categorized failure modes supported by the generic simulation engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "category", rename_all = "snake_case")]
pub enum FaultType {
    /// Constant measurement offset added to sensor reading.
    SensorBias { offset: f64 },
    /// Constant drift rate added to sensor reading per elapsed second.
    SensorDrift { rate_per_sec: f64 },
    /// Sensor signal freezes at its current value or a specified value.
    SensorFrozen { value: Option<f64> },
    /// Sensor loses signal completely, forcing TagQuality::Bad.
    SensorLossOfSignal,
    /// High-frequency pseudo-random measurement noise degradation.
    SensorDegradation { noise_amplitude: f64 },
    /// Actuator position locks at a fixed percentage (0-100%).
    ActuatorStuck { position: f64 },
    /// Actuator travel speed is severely constrained.
    ActuatorSluggish { max_rate_per_sec: f64 },
    /// Actuator trips immediately to its fail-safe position.
    ActuatorFailSafeTrip,
    /// Rotating equipment motor or turbine trips, speed collapses to zero.
    EquipmentMechanicalTrip,
    /// Heat exchanger or pump efficiency degrades by factor in [0.0, 1.0].
    EquipmentEfficiencyLoss { factor: f64 },
    /// Pipe segment hydraulic resistance multiplies, simulating fouling or blockage.
    ProcessBlockage { resistance_multiplier: f64 },
    /// Pipe or vessel leaks mass to atmosphere at specified rate (kg/s).
    ProcessLeak { leak_rate_kg_s: f64 },
}

/// Dynamic process disturbance definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisturbanceDefinition {
    pub id: String,
    pub target_tag: String,
    pub disturbance_type: DisturbanceType,
    pub duration_sec: Option<f64>,
}

/// Supported continuous and discrete disturbance waveforms.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "waveform", rename_all = "snake_case")]
pub enum DisturbanceType {
    /// Step change applied to the target tag.
    Step { value: f64, relative: bool },
    /// Linear ramp from current value to target value over specified duration.
    Ramp {
        target_value: f64,
        duration_sec: f64,
    },
    /// Rectangular pulse applied for pulse_width_sec.
    Pulse { value: f64, pulse_width_sec: f64 },
    /// Sinusoidal variation around nominal value.
    Sinusoidal {
        amplitude: f64,
        frequency_hz: f64,
        offset: f64,
    },
    /// Bounded deterministic pseudo-random variation.
    BoundedNoise { max_amplitude: f64 },
}

/// Simulated or scripted operator action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum OperatorAction {
    SetSetpoint {
        loop_name: String,
        sp: f64,
    },
    SetMode {
        loop_name: String,
        mode: String,
    },
    SetManualOutput {
        loop_name: String,
        output: f64,
    },
    WriteTag {
        tag: String,
        value: f64,
    },
    AcknowledgeAlarm {
        alarm_name: String,
    },
    ShelveAlarm {
        alarm_name: String,
        duration_sec: f64,
    },
    ResetProtection,
}

/// Scenario learning/training objective evaluated against simulation state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioObjective {
    pub id: String,
    pub description: String,
    pub criteria: ObjectiveCriteria,
    pub time_limit_sec: Option<f64>,
    #[serde(default = "default_weight")]
    pub weight: u32,
    #[serde(default = "default_true")]
    pub mandatory: bool,
}

fn default_weight() -> u32 {
    100
}

fn default_true() -> bool {
    true
}

/// Specific condition required to satisfy an objective.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ObjectiveCriteria {
    /// Tag must remain within `[min_val, max_val]` continuously for `hold_duration_sec`.
    TagWithinRange {
        tag: String,
        min_val: f64,
        max_val: f64,
        hold_duration_sec: f64,
    },
    /// Tag must reach target value evaluated via comparison operator.
    TagReachesValue {
        tag: String,
        operator: ComparisonOp,
        target_val: f64,
    },
    /// Specified alarm must be acknowledged within `within_sec` after activating.
    AlarmAcknowledged { alarm_name: String, within_sec: f64 },
    /// Entire scenario finishes without any equipment protective trip.
    NoTripOccurred,
}

/// Current status of an individual scenario objective.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ObjectiveStatus {
    Pending { progress: f64 },
    Satisfied { at_sim_time: f64 },
    Failed { reason: String },
    TimedOut,
}

/// Lifecycle state of the scenario engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioState {
    /// No scenario loaded.
    Idle,
    /// Scenario definition loaded and validated.
    Loaded,
    /// Scenario armed; initial conditions applied.
    Armed,
    /// Active execution; timeline and faults progressing.
    Running,
    /// Execution temporarily suspended.
    Paused,
    /// Successfully finished (all mandatory objectives met or duration limit reached).
    Completed,
    /// Manually or automatically aborted due to fatal condition.
    Aborted,
}
