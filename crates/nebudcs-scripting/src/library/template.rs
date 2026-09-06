//! Equipment Template definitions and immutable instantiation.
//!
//! Separates reusable template definitions from runtime instances.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use nebudcs_model::equipment::EquipmentCategory;
use nebudcs_types::{EquipmentId, SchemaVersion};

use crate::error::{ScriptingError, ScriptingResult};
use crate::model::CustomEquipmentDefinition;
use crate::runtime::CustomEquipmentInstance;
use crate::validation::validate_custom_equipment;

/// Provenance source of an equipment template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateSource {
    /// Standard verified library template shipped with NebuDCS.
    Standard,
    /// User-created custom template authored at runtime.
    User,
}

/// Rich metadata describing a template for discovery and display.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateMetadata {
    pub author: String,
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub icon_hint: Option<String>,
    #[serde(default)]
    pub documentation: Option<String>,
    #[serde(default = "default_created_at")]
    pub created_at: String,
    #[serde(default = "default_schema_version")]
    pub schema_version: SchemaVersion,
    #[serde(default)]
    pub custom_attributes: IndexMap<String, String>,
}

fn default_created_at() -> String {
    "2026-09-06T00:00:00Z".to_string()
}

fn default_schema_version() -> SchemaVersion {
    SchemaVersion::new(1, 0, 0)
}

impl Default for TemplateMetadata {
    fn default() -> Self {
        Self {
            author: "NebuDCS Architecture Team".to_string(),
            description: "Standard industrial equipment template".to_string(),
            tags: Vec::new(),
            icon_hint: None,
            documentation: None,
            created_at: default_created_at(),
            schema_version: default_schema_version(),
            custom_attributes: IndexMap::new(),
        }
    }
}

/// A reusable, immutable equipment template.
///
/// Templates define the engineering characteristics, parameters, ports, curves,
/// operating envelopes, alarms, trips, and behavior for an equipment type.
/// Instantiating a template creates an independent `CustomEquipmentInstance`
/// without mutating or sharing state with the source template.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquipmentTemplate {
    pub id: String,
    pub name: String,
    pub version: SchemaVersion,
    pub category: EquipmentCategory,
    pub source: TemplateSource,
    pub metadata: TemplateMetadata,
    pub definition: CustomEquipmentDefinition,
}

impl EquipmentTemplate {
    /// Constructs and validates a new equipment template.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        version: SchemaVersion,
        category: EquipmentCategory,
        source: TemplateSource,
        metadata: TemplateMetadata,
        definition: CustomEquipmentDefinition,
    ) -> ScriptingResult<Self> {
        let id_str = id.into();
        if id_str.trim().is_empty() {
            return Err(ScriptingError::Template(
                "Template ID cannot be empty".to_string(),
            ));
        }

        // Validate underlying custom equipment definition
        validate_custom_equipment(&definition)?;

        Ok(Self {
            id: id_str,
            name: name.into(),
            version,
            category,
            source,
            metadata,
            definition,
        })
    }

    /// Promotes a validated Phase 10 CustomEquipmentDefinition into a reusable user template.
    pub fn from_user_definition(
        definition: CustomEquipmentDefinition,
        metadata: Option<TemplateMetadata>,
    ) -> ScriptingResult<Self> {
        validate_custom_equipment(&definition)?;

        let meta = metadata.unwrap_or_else(|| TemplateMetadata {
            author: "User".to_string(),
            description: definition.description.clone(),
            tags: vec!["user-defined".to_string()],
            icon_hint: None,
            documentation: None,
            created_at: default_created_at(),
            schema_version: definition.version,
            custom_attributes: IndexMap::new(),
        });

        Ok(Self {
            id: definition.id.clone(),
            name: definition.name.clone(),
            version: definition.version,
            category: definition.category,
            source: TemplateSource::User,
            metadata: meta,
            definition,
        })
    }

    /// Safely instantiates this template into an independent runtime instance.
    ///
    /// This method enforces:
    /// - Template immutability (self is borrowed immutably).
    /// - Parameter validation and non-finite float rejection.
    /// - Unique instance identity.
    /// - Complete runtime state isolation.
    pub fn instantiate(
        &self,
        instance_id: EquipmentId,
        name: impl Into<String>,
        param_overrides: &BTreeMap<String, f64>,
    ) -> ScriptingResult<CustomEquipmentInstance> {
        // Collect default parameters from definition
        let mut parameters = BTreeMap::new();
        for (k, param_def) in &self.definition.parameters {
            parameters.insert(k.clone(), param_def.default_value);
        }

        // Apply parameter overrides with bounds & finiteness checking
        for (k, v) in param_overrides {
            if !v.is_finite() {
                return Err(ScriptingError::Instance(format!(
                    "Parameter override for '{k}' is non-finite: {v}"
                )));
            }
            if let Some(param_def) = self.definition.parameters.get(k) {
                if let Some(min) = param_def.min_value {
                    if *v < min {
                        return Err(ScriptingError::Instance(format!(
                            "Parameter override '{k}' = {v} is below minimum {min}"
                        )));
                    }
                }
                if let Some(max) = param_def.max_value {
                    if *v > max {
                        return Err(ScriptingError::Instance(format!(
                            "Parameter override '{k}' = {v} exceeds maximum {max}"
                        )));
                    }
                }
            }
            parameters.insert(k.clone(), *v);
        }

        // Initialize standard process variables
        let mut variables = BTreeMap::new();
        variables.insert("speed".to_string(), 0.0);
        variables.insert("flow".to_string(), 0.0);
        variables.insert("delta_p".to_string(), 0.0);
        variables.insert("power".to_string(), 0.0);
        variables.insert("efficiency".to_string(), 1.0);
        variables.insert("vibration".to_string(), 0.0);

        // Initialize standard discrete status flags
        let mut flags = BTreeMap::new();
        flags.insert("is_running".to_string(), false);
        flags.insert("is_tripped".to_string(), false);
        flags.insert("permissive_ok".to_string(), true);

        Ok(CustomEquipmentInstance {
            id: instance_id,
            definition_id: self.definition.id.clone(),
            name: name.into(),
            parameters,
            variables,
            flags,
            active_state: "stopped".to_string(),
            latched_trips: Vec::new(),
        })
    }
}
