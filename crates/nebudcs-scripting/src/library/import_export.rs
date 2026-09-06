//! Data-driven Import and Export for Equipment Templates and Packages.
//!
//! Provides deterministic serialization in JSON and TOML formats with
//! rigorous pre-admission validation, resource limits, and sanitization.

use serde::{Deserialize, Serialize};

use crate::error::{ScriptingError, ScriptingResult};
use crate::library::package::EquipmentPackage;
use crate::library::template::EquipmentTemplate;
use crate::validation::validate_custom_equipment;

/// Supported data serialization formats for import and export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SerializationFormat {
    Json,
    Toml,
}

/// Maximum payload size permitted for imported templates and packages (2 MB).
pub const MAX_IMPORT_SIZE_BYTES: usize = 2 * 1024 * 1024;

/// Maximum number of sub-components allowed within an imported package.
pub const MAX_PACKAGE_SUB_COMPONENTS: usize = 64;

/// Maximum number of internal streams allowed within an imported package.
pub const MAX_PACKAGE_INTERNAL_STREAMS: usize = 128;

/// Validates identifier characters (alphanumeric, underscores, hyphens, and periods).
pub fn validate_identifier(id: &str, context: &str) -> ScriptingResult<()> {
    let trimmed = id.trim();
    if trimmed.is_empty() {
        return Err(ScriptingError::ImportExport(format!(
            "{context} identifier cannot be empty"
        )));
    }
    for ch in trimmed.chars() {
        if !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-' && ch != '.' {
            return Err(ScriptingError::ImportExport(format!(
                "{context} identifier '{id}' contains illegal character '{ch}' (allowed: [a-zA-Z0-9_.-])"
            )));
        }
    }
    Ok(())
}

/// Serializes an equipment template deterministically.
pub fn export_template(
    template: &EquipmentTemplate,
    format: SerializationFormat,
) -> ScriptingResult<String> {
    match format {
        SerializationFormat::Json => serde_json::to_string_pretty(template)
            .map_err(|e| ScriptingError::Serialization(e.to_string())),
        SerializationFormat::Toml => toml::to_string_pretty(template)
            .map_err(|e| ScriptingError::Serialization(e.to_string())),
    }
}

/// Parses, validates, and admits an equipment template from a data payload.
pub fn import_template(
    content: &str,
    format: SerializationFormat,
) -> ScriptingResult<EquipmentTemplate> {
    if content.len() > MAX_IMPORT_SIZE_BYTES {
        return Err(ScriptingError::ImportExport(format!(
            "Payload size ({} bytes) exceeds maximum limit of {} bytes",
            content.len(),
            MAX_IMPORT_SIZE_BYTES
        )));
    }

    let template: EquipmentTemplate = match format {
        SerializationFormat::Json => serde_json::from_str(content)
            .map_err(|e| ScriptingError::ImportExport(format!("Malformed JSON document: {e}")))?,
        SerializationFormat::Toml => toml::from_str(content)
            .map_err(|e| ScriptingError::ImportExport(format!("Malformed TOML document: {e}")))?,
    };

    validate_identifier(&template.id, "Template")?;
    validate_identifier(&template.definition.id, "Definition")?;

    // Perform deep pre-execution validation
    validate_custom_equipment(&template.definition)?;

    Ok(template)
}

/// Serializes an equipment package deterministically.
pub fn export_package(
    package: &EquipmentPackage,
    format: SerializationFormat,
) -> ScriptingResult<String> {
    match format {
        SerializationFormat::Json => serde_json::to_string_pretty(package)
            .map_err(|e| ScriptingError::Serialization(e.to_string())),
        SerializationFormat::Toml => toml::to_string_pretty(package)
            .map_err(|e| ScriptingError::Serialization(e.to_string())),
    }
}

/// Parses, validates, and admits an equipment package from a data payload.
pub fn import_package(
    content: &str,
    format: SerializationFormat,
) -> ScriptingResult<EquipmentPackage> {
    if content.len() > MAX_IMPORT_SIZE_BYTES {
        return Err(ScriptingError::ImportExport(format!(
            "Payload size ({} bytes) exceeds maximum limit of {} bytes",
            content.len(),
            MAX_IMPORT_SIZE_BYTES
        )));
    }

    let package: EquipmentPackage = match format {
        SerializationFormat::Json => serde_json::from_str(content)
            .map_err(|e| ScriptingError::ImportExport(format!("Malformed JSON document: {e}")))?,
        SerializationFormat::Toml => toml::from_str(content)
            .map_err(|e| ScriptingError::ImportExport(format!("Malformed TOML document: {e}")))?,
    };

    validate_identifier(&package.id, "Package")?;

    if package.sub_components.len() > MAX_PACKAGE_SUB_COMPONENTS {
        return Err(ScriptingError::ImportExport(format!(
            "Package '{}' exceeds maximum sub-component limit of {} (contains {})",
            package.id,
            MAX_PACKAGE_SUB_COMPONENTS,
            package.sub_components.len()
        )));
    }

    if package.internal_streams.len() > MAX_PACKAGE_INTERNAL_STREAMS {
        return Err(ScriptingError::ImportExport(format!(
            "Package '{}' exceeds maximum internal stream limit of {} (contains {})",
            package.id,
            MAX_PACKAGE_INTERNAL_STREAMS,
            package.internal_streams.len()
        )));
    }

    for sub in &package.sub_components {
        validate_identifier(&sub.id, "Sub-component")?;
        validate_identifier(&sub.template_id, "Referenced template")?;
        for (k, v) in &sub.parameter_overrides {
            if !v.is_finite() {
                return Err(ScriptingError::ImportExport(format!(
                    "Sub-component '{}' parameter override '{k}' is non-finite: {v}",
                    sub.id
                )));
            }
        }
    }

    for stream in &package.internal_streams {
        validate_identifier(&stream.id, "Internal stream")?;
    }

    for port_map in &package.external_ports {
        validate_identifier(&port_map.external_port_name, "External port")?;
    }

    for (k, param) in &package.parameters {
        validate_identifier(k, "Package parameter")?;
        if !param.default_value.is_finite() {
            return Err(ScriptingError::ImportExport(format!(
                "Package parameter '{k}' default value is non-finite: {}",
                param.default_value
            )));
        }
    }

    Ok(package)
}
