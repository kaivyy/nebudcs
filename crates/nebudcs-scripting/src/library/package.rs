//! Reusable Equipment Package abstraction and composition validation.
//!
//! Encapsulates composite machinery, nested subgraphs, internal streams,
//! external port mappings, parameter forwarding, and dependency declarations.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

use nebudcs_model::equipment::{
    EquipmentCategory, ParameterDefinition, PortDefinition, PortDirection, PortDomain,
};
use nebudcs_types::{EquipmentId, SchemaVersion};

use crate::error::{ScriptingError, ScriptingResult};
use crate::library::template::EquipmentTemplate;
use crate::model::{
    CompositionDefinition, CustomEquipmentDefinition, CustomizationTier,
    InternalConnectionDefinition, PortMappingDefinition, SubComponentDefinition,
};
use crate::runtime::CustomEquipmentInstance;

/// Dependency on another template or package.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PackageDependency {
    pub id: String,
    pub version_req: String,
}

/// A sub-component definition within an equipment package.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackageSubComponent {
    pub id: String,
    pub template_id: String,
    #[serde(default)]
    pub template_version: Option<SchemaVersion>,
    #[serde(default)]
    pub parameter_overrides: BTreeMap<String, f64>,
}

/// An internal process or signal stream connecting two child sub-components.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackageInternalStream {
    pub id: String,
    pub from_component: String,
    pub from_port: String,
    pub to_component: String,
    pub to_port: String,
}

/// Mapping of an external package port to an internal sub-component port.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackagePortMapping {
    pub external_port_name: String,
    pub direction: PortDirection,
    pub domain: PortDomain,
    pub internal_component: String,
    pub internal_port: String,
}

/// Forwarding of a package-level parameter to an internal child component.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackageParameterMapping {
    pub package_param: String,
    pub target_component: String,
    pub target_param: String,
}

/// An equipment package definition.
///
/// A package represents a composite piece of process machinery (e.g. 2-stage compressor skid,
/// pump with bypass loop, furnace with draft fan) containing multiple child equipment
/// templates, internal stream routing, and external port forwarding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquipmentPackage {
    pub id: String,
    pub name: String,
    pub version: SchemaVersion,
    pub description: String,
    #[serde(default = "default_author")]
    pub author: String,
    #[serde(default)]
    pub dependencies: Vec<PackageDependency>,
    #[serde(default)]
    pub sub_components: Vec<PackageSubComponent>,
    #[serde(default)]
    pub internal_streams: Vec<PackageInternalStream>,
    #[serde(default)]
    pub external_ports: Vec<PackagePortMapping>,
    #[serde(default)]
    pub parameters: IndexMap<String, ParameterDefinition>,
    #[serde(default)]
    pub parameter_mappings: Vec<PackageParameterMapping>,
    #[serde(default)]
    pub metadata: IndexMap<String, String>,
}

fn default_author() -> String {
    "NebuDCS Engineering Team".to_string()
}

/// Result of instantiating an equipment package.
#[derive(Debug, Clone)]
pub struct PackageInstantiationResult {
    pub package_id: String,
    pub instances: Vec<CustomEquipmentInstance>,
    pub port_mappings: Vec<PackagePortMapping>,
    pub internal_streams: Vec<PackageInternalStream>,
}

impl EquipmentPackage {
    /// Validates the package against a template lookup provider.
    ///
    /// Checks:
    /// - Non-empty valid identifier and version.
    /// - No duplicate sub-component IDs.
    /// - Existence of referenced child templates.
    /// - Non-finite float check on parameter overrides.
    /// - Valid internal connections (ports exist, directions & domains compatible, no self-loops).
    /// - Valid external port forwarding (internal ports exist and match).
    /// - Valid parameter mappings.
    pub fn validate<'a>(
        &self,
        lookup_template: impl Fn(&str, Option<&SchemaVersion>) -> Option<&'a EquipmentTemplate>,
    ) -> ScriptingResult<()> {
        if self.id.trim().is_empty() {
            return Err(ScriptingError::Package(
                "Package ID cannot be empty".to_string(),
            ));
        }

        // Check for duplicate sub-component IDs
        let mut sub_comp_ids = HashSet::new();
        for sub in &self.sub_components {
            if sub.id.trim().is_empty() {
                return Err(ScriptingError::Package(
                    "Sub-component ID cannot be empty".to_string(),
                ));
            }
            if !sub_comp_ids.insert(&sub.id) {
                return Err(ScriptingError::Package(format!(
                    "Duplicate sub-component ID '{}' in package '{}'",
                    sub.id, self.id
                )));
            }
        }

        // Validate child templates exist and have valid parameter overrides
        let mut child_templates = BTreeMap::new();
        for sub in &self.sub_components {
            let tmpl = lookup_template(&sub.template_id, sub.template_version.as_ref())
                .ok_or_else(|| {
                    ScriptingError::Package(format!(
                        "Package '{}' references unknown template '{}' (version: {:?})",
                        self.id, sub.template_id, sub.template_version
                    ))
                })?;

            for (k, v) in &sub.parameter_overrides {
                if !v.is_finite() {
                    return Err(ScriptingError::Package(format!(
                        "Sub-component '{}' parameter '{k}' is non-finite: {v}",
                        sub.id
                    )));
                }
            }

            child_templates.insert(&sub.id, tmpl);
        }

        // Check for duplicate internal stream IDs
        let mut stream_ids = HashSet::new();
        for stream in &self.internal_streams {
            if !stream_ids.insert(&stream.id) {
                return Err(ScriptingError::Package(format!(
                    "Duplicate internal stream ID '{}' in package '{}'",
                    stream.id, self.id
                )));
            }

            if stream.from_component == stream.to_component {
                return Err(ScriptingError::Package(format!(
                    "Internal stream '{}' creates an invalid self-connection on component '{}'",
                    stream.id, stream.from_component
                )));
            }

            let from_tmpl = child_templates.get(&stream.from_component).ok_or_else(|| {
                ScriptingError::Package(format!(
                    "Internal stream '{}' source component '{}' does not exist",
                    stream.id, stream.from_component
                ))
            })?;

            let to_tmpl = child_templates.get(&stream.to_component).ok_or_else(|| {
                ScriptingError::Package(format!(
                    "Internal stream '{}' destination component '{}' does not exist",
                    stream.id, stream.to_component
                ))
            })?;

            let from_port = from_tmpl
                .definition
                .ports
                .iter()
                .find(|p| p.name == stream.from_port)
                .ok_or_else(|| {
                    ScriptingError::Package(format!(
                        "Internal stream '{}': port '{}' not found on component '{}'",
                        stream.id, stream.from_port, stream.from_component
                    ))
                })?;

            let to_port = to_tmpl
                .definition
                .ports
                .iter()
                .find(|p| p.name == stream.to_port)
                .ok_or_else(|| {
                    ScriptingError::Package(format!(
                        "Internal stream '{}': port '{}' not found on component '{}'",
                        stream.id, stream.to_port, stream.to_component
                    ))
                })?;

            if from_port.direction != PortDirection::Outlet
                && from_port.direction != PortDirection::Bidirectional
            {
                return Err(ScriptingError::Package(format!(
                    "Internal stream '{}' source port '{}.{}' must be Outlet or Bidirectional (found {:?})",
                    stream.id, stream.from_component, stream.from_port, from_port.direction
                )));
            }

            if to_port.direction != PortDirection::Inlet
                && to_port.direction != PortDirection::Bidirectional
            {
                return Err(ScriptingError::Package(format!(
                    "Internal stream '{}' destination port '{}.{}' must be Inlet or Bidirectional (found {:?})",
                    stream.id, stream.to_component, stream.to_port, to_port.direction
                )));
            }

            if from_port.domain != to_port.domain {
                return Err(ScriptingError::Package(format!(
                    "Internal stream '{}' connects incompatible port domains: {:?} -> {:?}",
                    stream.id, from_port.domain, to_port.domain
                )));
            }
        }

        // Check for duplicate external port names and validate mappings
        let mut ext_port_names = HashSet::new();
        for port_map in &self.external_ports {
            if !ext_port_names.insert(&port_map.external_port_name) {
                return Err(ScriptingError::Package(format!(
                    "Duplicate external port name '{}' in package '{}'",
                    port_map.external_port_name, self.id
                )));
            }

            let comp_tmpl = child_templates
                .get(&port_map.internal_component)
                .ok_or_else(|| {
                    ScriptingError::Package(format!(
                        "External port '{}' references nonexistent internal component '{}'",
                        port_map.external_port_name, port_map.internal_component
                    ))
                })?;

            let internal_port = comp_tmpl
                .definition
                .ports
                .iter()
                .find(|p| p.name == port_map.internal_port)
                .ok_or_else(|| {
                    ScriptingError::Package(format!(
                        "External port '{}': internal port '{}.{}' does not exist",
                        port_map.external_port_name,
                        port_map.internal_component,
                        port_map.internal_port
                    ))
                })?;

            if internal_port.domain != port_map.domain {
                return Err(ScriptingError::Package(format!(
                    "External port '{}' domain {:?} does not match internal port domain {:?}",
                    port_map.external_port_name, port_map.domain, internal_port.domain
                )));
            }
        }

        // Validate parameter mappings
        for mapping in &self.parameter_mappings {
            if !self.parameters.contains_key(&mapping.package_param) {
                return Err(ScriptingError::Package(format!(
                    "Parameter mapping references unknown package parameter '{}'",
                    mapping.package_param
                )));
            }

            let comp_tmpl = child_templates
                .get(&mapping.target_component)
                .ok_or_else(|| {
                    ScriptingError::Package(format!(
                        "Parameter mapping references nonexistent target component '{}'",
                        mapping.target_component
                    ))
                })?;

            if !comp_tmpl
                .definition
                .parameters
                .contains_key(&mapping.target_param)
            {
                return Err(ScriptingError::Package(format!(
                    "Parameter mapping: target parameter '{}.{}' does not exist",
                    mapping.target_component, mapping.target_param
                )));
            }
        }

        Ok(())
    }

    /// Converts this package into a Level 2 `CompositionDefinition`.
    pub fn to_composition_definition(&self) -> CompositionDefinition {
        let sub_components = self
            .sub_components
            .iter()
            .map(|s| {
                let mut params = IndexMap::new();
                for (k, v) in &s.parameter_overrides {
                    params.insert(k.clone(), *v);
                }
                SubComponentDefinition {
                    id: s.id.clone(),
                    template_id: s.template_id.clone(),
                    parameters: params,
                }
            })
            .collect();

        let internal_connections = self
            .internal_streams
            .iter()
            .map(|s| InternalConnectionDefinition {
                from_component: s.from_component.clone(),
                from_port: s.from_port.clone(),
                to_component: s.to_component.clone(),
                to_port: s.to_port.clone(),
            })
            .collect();

        let port_mappings = self
            .external_ports
            .iter()
            .map(|p| PortMappingDefinition {
                external_port_name: p.external_port_name.clone(),
                internal_component: p.internal_component.clone(),
                internal_port: p.internal_port.clone(),
            })
            .collect();

        CompositionDefinition {
            sub_components,
            internal_connections,
            port_mappings,
        }
    }

    /// Converts this package into a validated `CustomEquipmentDefinition` of tier Level 2.
    pub fn to_custom_equipment_definition(
        &self,
        category: EquipmentCategory,
    ) -> CustomEquipmentDefinition {
        let ports = self
            .external_ports
            .iter()
            .map(|p| PortDefinition {
                id: nebudcs_types::PortId::new(),
                name: p.external_port_name.clone(),
                direction: p.direction,
                domain: p.domain.clone(),
                phase_compatibility: Vec::new(),
                required: true,
                cardinality: nebudcs_model::equipment::Cardinality::ZeroOrMore,
                description: format!("External port {}", p.external_port_name),
            })
            .collect();

        CustomEquipmentDefinition {
            id: self.id.clone(),
            name: self.name.clone(),
            version: self.version,
            category,
            tier: CustomizationTier::Level2Composition,
            description: self.description.clone(),
            parameters: self.parameters.clone(),
            ports,
            curves: IndexMap::new(),
            curves_2d: IndexMap::new(),
            operating_envelope: Default::default(),
            alarm_definitions: Vec::new(),
            trips: Vec::new(),
            interlocks: Vec::new(),
            script: None,
            composition: Some(self.to_composition_definition()),
            metadata: self.metadata.clone(),
        }
    }

    /// Instantiates all sub-components of this package into concrete `CustomEquipmentInstance`s.
    pub fn instantiate_subgraph<'a>(
        &self,
        instance_prefix: &str,
        param_overrides: &BTreeMap<String, f64>,
        lookup_template: impl Fn(&str, Option<&SchemaVersion>) -> Option<&'a EquipmentTemplate>,
    ) -> ScriptingResult<PackageInstantiationResult> {
        self.validate(&lookup_template)?;

        // Resolve top-level package parameters
        let mut resolved_package_params = BTreeMap::new();
        for (k, p) in &self.parameters {
            resolved_package_params.insert(k.clone(), p.default_value);
        }
        for (k, v) in param_overrides {
            if !v.is_finite() {
                return Err(ScriptingError::Package(format!(
                    "Parameter override '{k}' is non-finite: {v}"
                )));
            }
            resolved_package_params.insert(k.clone(), *v);
        }

        // Build parameter forwardings per child
        let mut forwarded_params: BTreeMap<String, BTreeMap<String, f64>> = BTreeMap::new();
        for mapping in &self.parameter_mappings {
            if let Some(val) = resolved_package_params.get(&mapping.package_param) {
                forwarded_params
                    .entry(mapping.target_component.clone())
                    .or_default()
                    .insert(mapping.target_param.clone(), *val);
            }
        }

        let mut instances = Vec::new();
        for sub in &self.sub_components {
            let tmpl = lookup_template(&sub.template_id, sub.template_version.as_ref())
                .ok_or_else(|| {
                    ScriptingError::Package(format!("Template '{}' not found", sub.template_id))
                })?;

            // Combine sub-component defaults, package forwarded params, and explicit sub overrides
            let mut combined_overrides = sub.parameter_overrides.clone();
            if let Some(forwarded) = forwarded_params.get(&sub.id) {
                for (k, v) in forwarded {
                    combined_overrides.insert(k.clone(), *v);
                }
            }

            let sub_inst_id = EquipmentId::new();
            let sub_name = format!("{}_{}", instance_prefix, sub.id);
            let instance = tmpl.instantiate(sub_inst_id, sub_name, &combined_overrides)?;
            instances.push(instance);
        }

        Ok(PackageInstantiationResult {
            package_id: self.id.clone(),
            instances,
            port_mappings: self.external_ports.clone(),
            internal_streams: self.internal_streams.clone(),
        })
    }
}
