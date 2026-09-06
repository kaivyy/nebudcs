//! Data-driven custom equipment definition schemas.
//!
//! Supports Level 1 (declarative parameters/curves), Level 2 (composition of
//! sub-equipment subgraphs), and Level 3 (sandboxed dynamic Rhai scripts).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use nebudcs_model::equipment::{
    AlarmDefinition, EquipmentCategory, OperatingEnvelope, ParameterDefinition, PortDefinition,
};
use nebudcs_types::SchemaVersion;

use crate::curve::{Curve1D, Curve2D};

/// The customization tier of the equipment model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CustomizationTier {
    /// Level 1: Purely declarative parametric configuration, operating envelopes, and curves.
    #[default]
    Level1Declarative,
    /// Level 2: Composition of existing generic capabilities and internal subgraphs.
    Level2Composition,
    /// Level 3: Dynamic behavioral modeling using sandboxed Rhai scripts.
    Level3Scripted,
}

/// Comparison operator for trip and interlock triggers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TripOperator {
    #[default]
    GreaterThan,
    LessThan,
    GreaterOrEqual,
    LessOrEqual,
    Equal,
}

impl TripOperator {
    /// Evaluates the operator comparison.
    pub fn evaluate(&self, lhs: f64, rhs: f64) -> bool {
        match self {
            Self::GreaterThan => lhs > rhs,
            Self::LessThan => lhs < rhs,
            Self::GreaterOrEqual => lhs >= rhs,
            Self::LessOrEqual => lhs <= rhs,
            Self::Equal => (lhs - rhs).abs() < 1e-9,
        }
    }
}

/// A protective trip condition configured for the custom equipment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TripConditionDefinition {
    pub id: String,
    pub name: String,
    /// Monitored variable or tag name.
    pub condition_var: String,
    pub operator: TripOperator,
    pub threshold: f64,
    /// Desired state upon trip (e.g. "tripped", "stopped", "closed").
    #[serde(default = "default_tripped_state")]
    pub fail_safe_state: String,
    /// Whether the trip latches until an explicit reset command.
    #[serde(default = "default_true")]
    pub latched: bool,
}

fn default_tripped_state() -> String {
    "tripped".to_string()
}

fn default_true() -> bool {
    true
}

/// An automated interlock rule configured for the custom equipment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterlockRuleDefinition {
    pub id: String,
    pub description: String,
    pub cause_var: String,
    pub operator: TripOperator,
    pub cause_threshold: f64,
    pub effect_var: String,
    pub effect_value: f64,
}

/// Script configuration for Level 3 custom equipment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptDefinition {
    /// Rhai script source code.
    pub code: String,
    /// Main entry-point function name (defaults to "step").
    #[serde(default = "default_entry_point")]
    pub entry_point: String,
}

fn default_entry_point() -> String {
    "step".to_string()
}

/// Level 2 composition definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CompositionDefinition {
    /// Internal sub-components that comprise this compound package.
    #[serde(default)]
    pub sub_components: Vec<SubComponentDefinition>,
    /// Connections between internal sub-components.
    #[serde(default)]
    pub internal_connections: Vec<InternalConnectionDefinition>,
    /// Mapping of external ports to internal sub-component ports.
    #[serde(default)]
    pub port_mappings: Vec<PortMappingDefinition>,
}

/// Sub-component within a Level 2 compound equipment definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubComponentDefinition {
    pub id: String,
    pub template_id: String,
    #[serde(default)]
    pub parameters: IndexMap<String, f64>,
}

/// Connection between two sub-components within a compound package.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InternalConnectionDefinition {
    pub from_component: String,
    pub from_port: String,
    pub to_component: String,
    pub to_port: String,
}

/// Mapping from an external package port to an internal sub-component port.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortMappingDefinition {
    pub external_port_name: String,
    pub internal_component: String,
    pub internal_port: String,
}

/// Complete, versioned custom equipment definition.
///
/// Can be loaded dynamically from TOML or JSON project data without recompiling Rust.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomEquipmentDefinition {
    pub id: String,
    pub name: String,
    pub version: SchemaVersion,
    pub category: EquipmentCategory,
    #[serde(default)]
    pub tier: CustomizationTier,
    #[serde(default)]
    pub description: String,
    /// Configurable parameters with physical quantity kinds and bounds.
    #[serde(default)]
    pub parameters: IndexMap<String, ParameterDefinition>,
    /// Semantic ports.
    #[serde(default)]
    pub ports: Vec<PortDefinition>,
    /// 1-D performance curves.
    #[serde(default)]
    pub curves: IndexMap<String, Curve1D>,
    /// 2-D performance curves (surfaces).
    #[serde(default)]
    pub curves_2d: IndexMap<String, Curve2D>,
    /// Multi-dimensional operating envelope.
    #[serde(default)]
    pub operating_envelope: OperatingEnvelope,
    /// Alarm thresholds.
    #[serde(default)]
    pub alarm_definitions: Vec<AlarmDefinition>,
    /// Protective trip conditions.
    #[serde(default)]
    pub trips: Vec<TripConditionDefinition>,
    /// Cause-and-effect interlocks.
    #[serde(default)]
    pub interlocks: Vec<InterlockRuleDefinition>,
    /// Sandboxed Rhai script (required for Level 3).
    #[serde(default)]
    pub script: Option<ScriptDefinition>,
    /// Package composition structure (required for Level 2).
    #[serde(default)]
    pub composition: Option<CompositionDefinition>,
    /// Arbitrary metadata tags.
    #[serde(default)]
    pub metadata: IndexMap<String, String>,
}
