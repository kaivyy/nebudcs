//! Equipment definition and instance types.
//!
//! # Key separation
//!
//! | Type | Mutability | Purpose |
//! |---|---|---|
//! | [`EquipmentDefinition`] | Immutable once in use | Versioned template |
//! | [`EquipmentInstance`] | Mutable by the engineer | Project-specific instance |
//! | `EquipmentRuntimeState` | Mutable each tick | Simulation state (Phase 3+) |
//!
//! `EquipmentRuntimeState` does not exist in this phase — it belongs in
//! `nebudcs-sim` (Phase 3).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use nebudcs_types::{
    AlarmDefinitionId, EquipmentDefinitionId, EquipmentId, FailureModeId, PortId, SchemaVersion,
    TagId,
};
use nebudcs_units::display::QuantityKind;

// ---------------------------------------------------------------------------
// Equipment category
// ---------------------------------------------------------------------------

/// Broad category that governs which generic physics model applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EquipmentCategory {
    Rotating,
    HeatTransfer,
    Process,
    Valve,
    Instrumentation,
    Utility,
}

// ---------------------------------------------------------------------------
// Capability
// ---------------------------------------------------------------------------

/// A capability describes a role that equipment can play in the simulation.
///
/// Multiple capabilities may be assigned to one definition (e.g., a
/// turbine-driven pump is both `RotatingEquipment` and `EnergyConsumer`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    FlowSource,
    FlowSink,
    RotatingEquipment,
    HeatTransfer,
    PressureBoundary,
    ControlElement,
    MeasurementSource,
    EnergyConsumer,
    EnergyProducer,
    Reaction,
    Separation,
    UtilityConsumer,
    UtilityProvider,
    ProtectiveDevice,
}

// ---------------------------------------------------------------------------
// Port types
// ---------------------------------------------------------------------------

/// The physical domain a port belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortDomain {
    Material,
    Utility(UtilityKind),
    Electrical,
    Mechanical,
    Signal,
    Catalyst,
    Energy,
}

/// Specific utility network the port connects to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UtilityKind {
    Hydrogen,
    SteamHp,
    SteamMp,
    SteamLp,
    CoolingWater,
    InstrumentAir,
    PlantAir,
    Nitrogen,
    FuelGas,
    Electricity,
    Flare,
}

/// Flow direction of a port relative to the equipment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortDirection {
    Inlet,
    Outlet,
    Bidirectional,
}

/// The phase(s) of matter the port accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Liquid,
    Gas,
    TwoPhase,
    Solid,
}

/// How many connections a port may have.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cardinality {
    /// Exactly one connection required.
    ExactlyOne,
    /// Zero or one connection.
    ZeroOrOne,
    /// One or more connections.
    OneOrMore,
    /// Any number of connections (including zero).
    ZeroOrMore,
}

/// A typed port on an equipment definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortDefinition {
    pub id: PortId,
    pub name: String,
    pub domain: PortDomain,
    pub direction: PortDirection,
    /// Phases of matter accepted by this port.
    #[serde(default)]
    pub phase_compatibility: Vec<Phase>,
    /// Whether a connection is required for the model to be valid.
    #[serde(default = "default_true")]
    pub required: bool,
    pub cardinality: Cardinality,
    pub description: String,
}

fn default_true() -> bool {
    true
}

// ---------------------------------------------------------------------------
// Parameter definition
// ---------------------------------------------------------------------------

/// A configurable engineering parameter on an equipment definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParameterDefinition {
    /// Unique key within this definition (e.g., `"rated_flow"`).
    pub name: String,
    pub description: String,
    /// The physical quantity this parameter represents.
    pub quantity_kind: QuantityKind,
    /// Default value in canonical SI.
    pub default_value: f64,
    /// Minimum allowed value in canonical SI.
    pub min_value: Option<f64>,
    /// Maximum allowed value in canonical SI.
    pub max_value: Option<f64>,
    /// Whether the project must supply this value.
    #[serde(default)]
    pub required: bool,
}

// ---------------------------------------------------------------------------
// Tag definition
// ---------------------------------------------------------------------------

/// Source of a tag's value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TagSource {
    /// Computed by the simulation engine each tick.
    Simulated,
    /// Written by an operator or external system.
    Operator,
    /// Derived from other tags by a formula.
    Derived,
}

/// A process variable tag on an equipment definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TagDefinition {
    pub id: TagId,
    /// The tag address suffix (e.g., `"FLOW"`, `"SPEED"`, `"PRESS"`).
    pub name: String,
    pub description: String,
    pub quantity_kind: QuantityKind,
    #[serde(default)]
    pub writable: bool,
    pub source: TagSource,
    /// IDs of alarm definitions that monitor this tag.
    #[serde(default)]
    pub alarm_refs: Vec<AlarmDefinitionId>,
}

// ---------------------------------------------------------------------------
// Operating envelope
// ---------------------------------------------------------------------------

/// One dimension of the equipment's operating envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvelopeDimension {
    /// The tag this dimension applies to.
    pub tag_id: TagId,
    pub description: String,
    /// Normal operating range (min, max) in canonical SI.
    pub normal_min: f64,
    pub normal_max: f64,
    /// Warning range.
    pub warning_min: Option<f64>,
    pub warning_max: Option<f64>,
    /// Critical range.
    pub critical_min: Option<f64>,
    pub critical_max: Option<f64>,
    /// Trip condition.
    pub trip_min: Option<f64>,
    pub trip_max: Option<f64>,
}

/// Multi-dimensional operating envelope for an equipment definition.
///
/// This is intentionally not a flat list of scalar thresholds — see the
/// architecture requirement for multidimensional envelopes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct OperatingEnvelope {
    pub dimensions: Vec<EnvelopeDimension>,
}

// ---------------------------------------------------------------------------
// Alarm definition
// ---------------------------------------------------------------------------

/// The type of alarm condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AlarmType {
    Hi,
    HiHi,
    Lo,
    LoLo,
    Deviation,
    RateOfChange,
    BadQuality,
    EquipmentFault,
    Trip,
    Advisory,
}

/// Priority level of an alarm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlarmPriority {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
    Emergency = 5,
}

/// An alarm condition associated with an equipment definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlarmDefinition {
    pub id: AlarmDefinitionId,
    pub name: String,
    pub alarm_type: AlarmType,
    pub priority: AlarmPriority,
    /// The tag whose value triggers this alarm.
    pub limit_tag: TagId,
    /// The threshold value in canonical SI units.
    pub limit_value: f64,
    pub description: String,
}

// ---------------------------------------------------------------------------
// Failure mode definition
// ---------------------------------------------------------------------------

/// How a failure mode affects the simulation.
///
/// In Phase 1 this is stored as a string key — the actual behaviour is
/// interpreted by the simulation engine (Phase 3+).  This avoids premature
/// commitment to a behaviour DSL.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FailureBehavior {
    /// Machine-readable key (e.g., `"freeze_output"`, `"set_flow_zero"`).
    pub kind: String,
    /// Optional parameters for the behaviour (interpreted by the engine).
    #[serde(default)]
    pub parameters: IndexMap<String, serde_json::Value>,
}

/// A configurable failure mode for an equipment definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FailureModeDefinition {
    pub id: FailureModeId,
    pub name: String,
    pub description: String,
    /// Tags whose values are affected by this failure.
    #[serde(default)]
    pub affected_tags: Vec<TagId>,
    pub behavior: FailureBehavior,
}

// ---------------------------------------------------------------------------
// State machine definition
// ---------------------------------------------------------------------------

/// A single state in the equipment lifecycle state machine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateDef {
    pub name: String,
    pub description: String,
}

/// A transition between two states.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransitionDef {
    pub from_state: String,
    pub to_state: String,
    /// Boolean expression evaluated against tags (interpreted by engine).
    pub guard: Option<String>,
    /// Actions executed on transition (interpreted by engine).
    #[serde(default)]
    pub actions: Vec<String>,
}

/// State machine definition for equipment lifecycle (stopped, starting,
/// running, stopping, tripped, faulted, maintenance, unavailable).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StateMachineDefinition {
    pub states: Vec<StateDef>,
    pub transitions: Vec<TransitionDef>,
    pub initial_state: String,
}

// ---------------------------------------------------------------------------
// Equipment metadata
// ---------------------------------------------------------------------------

/// Informational metadata attached to an equipment definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct EquipmentMetadata {
    pub description: String,
    pub manufacturer: Option<String>,
    pub model_number: Option<String>,
    pub tags: Vec<String>,
}

// ---------------------------------------------------------------------------
// EquipmentDefinition — the versioned, immutable template
// ---------------------------------------------------------------------------

/// A versioned, data-driven equipment template.
///
/// # Invariant
///
/// An `EquipmentDefinition` is **immutable** once it is referenced by a
/// project.  The project stores the `version` it was authored against.
/// Updating the global template library must not silently change existing
/// projects.
///
/// New equipment types are created by authoring a new definition — not by
/// adding Rust code to the simulation core.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquipmentDefinition {
    pub id: EquipmentDefinitionId,
    pub name: String,
    pub version: SchemaVersion,
    pub category: EquipmentCategory,
    /// Roles this equipment plays in the simulation.
    #[serde(default)]
    pub capabilities: Vec<Capability>,
    /// Configurable engineering parameters (ordered for deterministic serialization).
    #[serde(default)]
    pub parameters: IndexMap<String, ParameterDefinition>,
    /// Typed ports through which process streams connect.
    #[serde(default)]
    pub ports: Vec<PortDefinition>,
    /// Process variable tags exposed by this equipment.
    #[serde(default)]
    pub tags: Vec<TagDefinition>,
    /// Multidimensional operating envelope.
    #[serde(default)]
    pub operating_envelope: OperatingEnvelope,
    /// Alarm conditions.
    #[serde(default)]
    pub alarm_definitions: Vec<AlarmDefinition>,
    /// Configurable failure modes.
    #[serde(default)]
    pub failure_modes: Vec<FailureModeDefinition>,
    /// Lifecycle state machine.
    #[serde(default)]
    pub state_machine: StateMachineDefinition,
    #[serde(default)]
    pub metadata: EquipmentMetadata,
}

// ---------------------------------------------------------------------------
// EquipmentInstance — a project-specific instance of a definition
// ---------------------------------------------------------------------------

/// An instance of an equipment definition within a [`Unit`](super::project::Unit).
///
/// The instance overrides specific parameters for this particular piece of
/// equipment while referencing the definition by ID and version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquipmentInstance {
    pub id: EquipmentId,
    /// Reference to the definition template.
    pub definition_id: EquipmentDefinitionId,
    /// The exact version of the definition this instance was authored against.
    pub definition_version: SchemaVersion,
    /// Human-readable tag prefix (e.g., `"K-101"`, `"FCV-201"`).
    pub name: String,
    /// Parameter overrides in canonical SI.  Missing keys use the definition
    /// default.
    #[serde(default)]
    pub parameters: IndexMap<String, f64>,
    /// Optional 2-D position on a P&ID canvas.  Stored separately from the
    /// model — layout is not topology.
    pub position: Option<Position2D>,
}

/// A 2-D point in the P&ID canvas coordinate system.
///
/// This is **layout metadata only** — it has no effect on simulation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Position2D {
    pub x: f64,
    pub y: f64,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use nebudcs_types::SchemaVersion;
    use nebudcs_units::display::QuantityKind;

    /// Build a representative pump EquipmentDefinition for testing.
    fn make_pump_definition() -> EquipmentDefinition {
        let flow_tag_id = TagId::new();
        let speed_tag_id = TagId::new();
        let power_tag_id = TagId::new();

        let hi_alarm_id = AlarmDefinitionId::new();
        let trip_alarm_id = AlarmDefinitionId::new();

        let mut params = IndexMap::new();
        params.insert(
            "rated_flow".to_owned(),
            ParameterDefinition {
                name: "rated_flow".to_owned(),
                description: "Rated volumetric flow at BEP".to_owned(),
                quantity_kind: QuantityKind::VolumetricFlow,
                default_value: 0.05, // 50 L/s in m³/s
                min_value: Some(0.0),
                max_value: Some(1.0),
                required: true,
            },
        );
        params.insert(
            "rated_head".to_owned(),
            ParameterDefinition {
                name: "rated_head".to_owned(),
                description: "Rated differential head in Pa".to_owned(),
                quantity_kind: QuantityKind::DifferentialPressure,
                default_value: 500_000.0,
                min_value: Some(0.0),
                max_value: None,
                required: true,
            },
        );
        params.insert(
            "max_power".to_owned(),
            ParameterDefinition {
                name: "max_power".to_owned(),
                description: "Maximum shaft power in W".to_owned(),
                quantity_kind: QuantityKind::Power,
                default_value: 100_000.0, // 100 kW
                min_value: Some(0.0),
                max_value: None,
                required: false,
            },
        );

        let suction_port = PortDefinition {
            id: PortId::new(),
            name: "suction".to_owned(),
            domain: PortDomain::Material,
            direction: PortDirection::Inlet,
            phase_compatibility: vec![Phase::Liquid],
            required: true,
            cardinality: Cardinality::ExactlyOne,
            description: "Pump suction inlet".to_owned(),
        };
        let discharge_port = PortDefinition {
            id: PortId::new(),
            name: "discharge".to_owned(),
            domain: PortDomain::Material,
            direction: PortDirection::Outlet,
            phase_compatibility: vec![Phase::Liquid],
            required: true,
            cardinality: Cardinality::ExactlyOne,
            description: "Pump discharge outlet".to_owned(),
        };

        let flow_tag = TagDefinition {
            id: flow_tag_id,
            name: "FLOW".to_owned(),
            description: "Actual volumetric flow".to_owned(),
            quantity_kind: QuantityKind::VolumetricFlow,
            writable: false,
            source: TagSource::Simulated,
            alarm_refs: vec![],
        };
        let speed_tag = TagDefinition {
            id: speed_tag_id,
            name: "SPEED".to_owned(),
            description: "Shaft speed".to_owned(),
            quantity_kind: QuantityKind::Speed,
            writable: true,
            source: TagSource::Operator,
            alarm_refs: vec![],
        };
        let power_tag = TagDefinition {
            id: power_tag_id,
            name: "POWER".to_owned(),
            description: "Shaft power".to_owned(),
            quantity_kind: QuantityKind::Power,
            writable: false,
            source: TagSource::Simulated,
            alarm_refs: vec![hi_alarm_id, trip_alarm_id],
        };

        let hi_alarm = AlarmDefinition {
            id: hi_alarm_id,
            name: "POWER_HI".to_owned(),
            alarm_type: AlarmType::Hi,
            priority: AlarmPriority::High,
            limit_tag: power_tag_id,
            limit_value: 90_000.0, // 90 kW
            description: "Pump power high".to_owned(),
        };
        let trip_alarm = AlarmDefinition {
            id: trip_alarm_id,
            name: "POWER_HIHI".to_owned(),
            alarm_type: AlarmType::HiHi,
            priority: AlarmPriority::Critical,
            limit_tag: power_tag_id,
            limit_value: 100_000.0, // 100 kW trip
            description: "Pump power trip".to_owned(),
        };

        let failure_mode = FailureModeDefinition {
            id: FailureModeId::new(),
            name: "MECHANICAL_SEIZURE".to_owned(),
            description: "Pump bearing seizes, flow drops to zero".to_owned(),
            affected_tags: vec![flow_tag_id, power_tag_id],
            behavior: FailureBehavior {
                kind: "set_flow_zero".to_owned(),
                parameters: IndexMap::new(),
            },
        };

        let state_machine = StateMachineDefinition {
            initial_state: "stopped".to_owned(),
            states: vec![
                StateDef {
                    name: "stopped".to_owned(),
                    description: "Pump at rest".to_owned(),
                },
                StateDef {
                    name: "starting".to_owned(),
                    description: "Ramp-up".to_owned(),
                },
                StateDef {
                    name: "running".to_owned(),
                    description: "Normal operation".to_owned(),
                },
                StateDef {
                    name: "tripped".to_owned(),
                    description: "Protection trip".to_owned(),
                },
            ],
            transitions: vec![
                TransitionDef {
                    from_state: "stopped".to_owned(),
                    to_state: "starting".to_owned(),
                    guard: Some("START_CMD == true".to_owned()),
                    actions: vec!["ramp_speed".to_owned()],
                },
                TransitionDef {
                    from_state: "starting".to_owned(),
                    to_state: "running".to_owned(),
                    guard: Some("SPEED > MIN_SPEED".to_owned()),
                    actions: vec![],
                },
            ],
        };

        EquipmentDefinition {
            id: EquipmentDefinitionId::new(),
            name: "Generic Centrifugal Pump".to_owned(),
            version: SchemaVersion::new(0, 1, 0),
            category: EquipmentCategory::Rotating,
            capabilities: vec![
                Capability::RotatingEquipment,
                Capability::FlowSource,
                Capability::EnergyConsumer,
            ],
            parameters: params,
            ports: vec![suction_port, discharge_port],
            tags: vec![flow_tag, speed_tag, power_tag],
            operating_envelope: OperatingEnvelope { dimensions: vec![] },
            alarm_definitions: vec![hi_alarm, trip_alarm],
            failure_modes: vec![failure_mode],
            state_machine,
            metadata: EquipmentMetadata {
                description: "Generic centrifugal pump for process service".to_owned(),
                ..Default::default()
            },
        }
    }

    #[test]
    fn test_pump_definition_fixture() {
        let pump = make_pump_definition();
        assert_eq!(pump.name, "Generic Centrifugal Pump");
        assert_eq!(pump.category, EquipmentCategory::Rotating);
        assert!(pump.capabilities.contains(&Capability::RotatingEquipment));
        assert_eq!(pump.ports.len(), 2);
        assert_eq!(pump.tags.len(), 3);
        assert_eq!(pump.alarm_definitions.len(), 2);
        assert_eq!(pump.failure_modes.len(), 1);
    }

    #[test]
    fn test_equipment_definition_serde_json() {
        let pump = make_pump_definition();
        let json = serde_json::to_string_pretty(&pump).expect("serialize");
        let decoded: EquipmentDefinition = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(pump.name, decoded.name);
        assert_eq!(pump.version, decoded.version);
        assert_eq!(pump.ports.len(), decoded.ports.len());
        assert_eq!(pump.tags.len(), decoded.tags.len());
        assert_eq!(
            pump.alarm_definitions.len(),
            decoded.alarm_definitions.len()
        );
    }

    #[test]
    fn test_equipment_definition_serde_toml() {
        let pump = make_pump_definition();
        let toml_str = toml::to_string_pretty(&pump).expect("serialize to TOML");
        let decoded: EquipmentDefinition =
            toml::from_str(&toml_str).expect("deserialize from TOML");
        assert_eq!(pump.name, decoded.name);
        assert_eq!(pump.category, decoded.category);
    }

    #[test]
    fn test_equipment_instance_references_definition() {
        let pump_def = make_pump_definition();
        let instance = EquipmentInstance {
            id: EquipmentId::new(),
            definition_id: pump_def.id,
            definition_version: pump_def.version,
            name: "K-101".to_owned(),
            parameters: {
                let mut m = IndexMap::new();
                m.insert("rated_flow".to_owned(), 0.08_f64);
                m
            },
            position: Some(Position2D { x: 100.0, y: 200.0 }),
        };
        assert_eq!(instance.definition_id, pump_def.id);
        assert_eq!(instance.definition_version, pump_def.version);
    }

    #[test]
    fn test_port_domain_variants_serde() {
        let domains = vec![
            PortDomain::Material,
            PortDomain::Utility(UtilityKind::Hydrogen),
            PortDomain::Electrical,
            PortDomain::Mechanical,
            PortDomain::Signal,
            PortDomain::Catalyst,
            PortDomain::Energy,
        ];
        for domain in &domains {
            let json = serde_json::to_string(domain).expect("serialize");
            let decoded: PortDomain = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(domain, &decoded);
        }
    }

    /// Build a vessel EquipmentDefinition.
    fn make_vessel_definition() -> EquipmentDefinition {
        let level_tag_id = TagId::new();
        let press_tag_id = TagId::new();

        let mut params = IndexMap::new();
        params.insert(
            "volume".to_owned(),
            ParameterDefinition {
                name: "volume".to_owned(),
                description: "Internal vessel volume in m³".to_owned(),
                quantity_kind: QuantityKind::Volume,
                default_value: 10.0,
                min_value: Some(0.01),
                max_value: None,
                required: true,
            },
        );
        params.insert(
            "design_pressure".to_owned(),
            ParameterDefinition {
                name: "design_pressure".to_owned(),
                description: "Maximum allowable working pressure in Pa".to_owned(),
                quantity_kind: QuantityKind::Pressure,
                default_value: 1_500_000.0, // 15 barg ≈ 1.5 MPa abs
                min_value: Some(0.0),
                max_value: None,
                required: true,
            },
        );

        EquipmentDefinition {
            id: EquipmentDefinitionId::new(),
            name: "Generic Vertical Vessel".to_owned(),
            version: SchemaVersion::new(0, 1, 0),
            category: EquipmentCategory::Process,
            capabilities: vec![Capability::PressureBoundary, Capability::Separation],
            parameters: params,
            ports: vec![
                PortDefinition {
                    id: PortId::new(),
                    name: "inlet".to_owned(),
                    domain: PortDomain::Material,
                    direction: PortDirection::Inlet,
                    phase_compatibility: vec![Phase::Liquid, Phase::Gas, Phase::TwoPhase],
                    required: true,
                    cardinality: Cardinality::OneOrMore,
                    description: "Process inlet".to_owned(),
                },
                PortDefinition {
                    id: PortId::new(),
                    name: "vapor_outlet".to_owned(),
                    domain: PortDomain::Material,
                    direction: PortDirection::Outlet,
                    phase_compatibility: vec![Phase::Gas],
                    required: false,
                    cardinality: Cardinality::ZeroOrOne,
                    description: "Overhead vapor outlet".to_owned(),
                },
                PortDefinition {
                    id: PortId::new(),
                    name: "liquid_outlet".to_owned(),
                    domain: PortDomain::Material,
                    direction: PortDirection::Outlet,
                    phase_compatibility: vec![Phase::Liquid],
                    required: true,
                    cardinality: Cardinality::ExactlyOne,
                    description: "Bottoms liquid outlet".to_owned(),
                },
            ],
            tags: vec![
                TagDefinition {
                    id: level_tag_id,
                    name: "LEVEL".to_owned(),
                    description: "Vessel liquid level fraction".to_owned(),
                    quantity_kind: QuantityKind::Level,
                    writable: false,
                    source: TagSource::Simulated,
                    alarm_refs: vec![],
                },
                TagDefinition {
                    id: press_tag_id,
                    name: "PRESS".to_owned(),
                    description: "Vessel pressure".to_owned(),
                    quantity_kind: QuantityKind::Pressure,
                    writable: false,
                    source: TagSource::Simulated,
                    alarm_refs: vec![],
                },
            ],
            operating_envelope: OperatingEnvelope { dimensions: vec![] },
            alarm_definitions: vec![],
            failure_modes: vec![],
            state_machine: StateMachineDefinition::default(),
            metadata: EquipmentMetadata {
                description: "Generic vertical pressure vessel".to_owned(),
                ..Default::default()
            },
        }
    }

    #[test]
    fn test_vessel_definition_fixture() {
        let vessel = make_vessel_definition();
        assert_eq!(vessel.category, EquipmentCategory::Process);
        assert_eq!(vessel.ports.len(), 3);
        assert_eq!(vessel.tags.len(), 2);
        assert!(vessel.capabilities.contains(&Capability::PressureBoundary));
    }
}
