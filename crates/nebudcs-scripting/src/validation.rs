//! Comprehensive validation engine for custom equipment definitions.
//!
//! Validates schema integrity, numeric boundaries, operating envelopes,
//! curves, composition subgraphs, and compiles/dry-runs Rhai scripts before
//! admitting definitions into the runtime simulation.

use std::collections::HashSet;

use crate::error::{ScriptingError, ScriptingResult};
use crate::model::{CustomEquipmentDefinition, CustomizationTier};
use crate::sandbox::{RhaiSandbox, ScriptExecutionContext};

/// Alias for validate_custom_equipment.
pub fn validate_definition(def: &CustomEquipmentDefinition) -> ScriptingResult<()> {
    validate_custom_equipment(def)
}

/// Validates a custom equipment definition against all architectural invariants.
pub fn validate_custom_equipment(def: &CustomEquipmentDefinition) -> ScriptingResult<()> {
    let mut errors = Vec::new();

    // 1. Identification validation
    if def.id.trim().is_empty() {
        errors.push("Equipment definition ID cannot be empty".to_string());
    } else if !def
        .id
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        errors.push(format!(
            "Equipment ID '{}' contains invalid characters (allowed: a-z, 0-9, -, _, .)",
            def.id
        ));
    }

    if def.name.trim().is_empty() {
        errors.push("Equipment definition name cannot be empty".to_string());
    }

    // 2. Parameters validation
    for (param_name, param) in &def.parameters {
        if param_name.trim().is_empty() {
            errors.push("Parameter name cannot be empty".to_string());
        }
        if !param.default_value.is_finite() {
            errors.push(format!(
                "Parameter '{param_name}' default_value is non-finite"
            ));
        }

        if let (Some(min), Some(max)) = (param.min_value, param.max_value) {
            if !min.is_finite() || !max.is_finite() {
                errors.push(format!("Parameter '{param_name}' limits must be finite"));
            } else if min > max {
                errors.push(format!(
                    "Parameter '{param_name}' min_value ({min}) > max_value ({max})"
                ));
            } else if param.default_value < min || param.default_value > max {
                errors.push(format!(
                    "Parameter '{param_name}' default_value ({}) out of range [{min}, {max}]",
                    param.default_value
                ));
            }
        }
    }

    // 3. Ports validation
    if def.ports.is_empty() {
        errors.push("Equipment must declare at least one port".to_string());
    }
    let mut port_names = HashSet::new();
    for port in &def.ports {
        if port.name.trim().is_empty() {
            errors.push("Port name cannot be empty".to_string());
        } else if !port_names.insert(&port.name) {
            errors.push(format!("Duplicate port name '{}'", port.name));
        }
    }

    // 4. Performance curves validation
    for (curve_name, curve) in &def.curves {
        if let Err(e) = curve.validate() {
            errors.push(format!("Curve '{curve_name}' invalid: {e}"));
        }
    }
    for (curve_name, curve_2d) in &def.curves_2d {
        if let Err(e) = curve_2d.validate() {
            errors.push(format!("Curve2D '{curve_name}' invalid: {e}"));
        }
    }

    // 5. Operating envelope validation
    for dim in &def.operating_envelope.dimensions {
        if !dim.normal_min.is_finite() || !dim.normal_max.is_finite() {
            errors.push(format!(
                "Envelope dimension '{}' normal range must be finite",
                dim.description
            ));
        } else if dim.normal_min > dim.normal_max {
            errors.push(format!(
                "Envelope dimension '{}' normal_min ({}) > normal_max ({})",
                dim.description, dim.normal_min, dim.normal_max
            ));
        }
    }

    // 6. Protective trips & interlocks validation
    let mut trip_ids = HashSet::new();
    for trip in &def.trips {
        if trip.id.trim().is_empty() {
            errors.push("Trip ID cannot be empty".to_string());
        } else if !trip_ids.insert(&trip.id) {
            errors.push(format!("Duplicate trip ID '{}'", trip.id));
        }
        if trip.condition_var.trim().is_empty() {
            errors.push(format!("Trip '{}' condition_var cannot be empty", trip.id));
        }
        if !trip.threshold.is_finite() {
            errors.push(format!("Trip '{}' threshold is non-finite", trip.id));
        }
    }

    for interlock in &def.interlocks {
        if interlock.id.trim().is_empty() {
            errors.push("Interlock ID cannot be empty".to_string());
        }
        if interlock.cause_var.trim().is_empty() || interlock.effect_var.trim().is_empty() {
            errors.push(format!(
                "Interlock '{}' cause and effect variables cannot be empty",
                interlock.id
            ));
        }
        if !interlock.cause_threshold.is_finite() || !interlock.effect_value.is_finite() {
            errors.push(format!(
                "Interlock '{}' thresholds must be finite",
                interlock.id
            ));
        }
    }

    // 7. Tier-specific validation
    match def.tier {
        CustomizationTier::Level1Declarative => {
            // Level 1 does not require script or composition
        }
        CustomizationTier::Level2Composition => {
            if let Some(comp) = &def.composition {
                if comp.sub_components.is_empty() {
                    errors.push(
                        "Level 2 composition must define at least one sub-component".to_string(),
                    );
                }
                let mut comp_ids = HashSet::new();
                for sub in &comp.sub_components {
                    if sub.id.trim().is_empty() {
                        errors.push("Sub-component ID cannot be empty".to_string());
                    } else if !comp_ids.insert(&sub.id) {
                        errors.push(format!("Duplicate sub-component ID '{}'", sub.id));
                    }
                }
                for conn in &comp.internal_connections {
                    if !comp_ids.contains(&conn.from_component) {
                        errors.push(format!(
                            "Internal connection references unknown component '{}'",
                            conn.from_component
                        ));
                    }
                    if !comp_ids.contains(&conn.to_component) {
                        errors.push(format!(
                            "Internal connection references unknown component '{}'",
                            conn.to_component
                        ));
                    }
                }

                let port_names: HashSet<&str> = def.ports.iter().map(|p| p.name.as_str()).collect();
                for mapping in &comp.port_mappings {
                    if !port_names.contains(mapping.external_port_name.as_str()) {
                        errors.push(format!(
                            "Port mapping references unknown external package port '{}'",
                            mapping.external_port_name
                        ));
                    }
                    if !comp_ids.contains(&mapping.internal_component) {
                        errors.push(format!(
                            "Port mapping references unknown sub-component '{}'",
                            mapping.internal_component
                        ));
                    }
                }
            } else {
                errors.push("Level 2 equipment requires composition definition".to_string());
            }
        }
        CustomizationTier::Level3Scripted => {
            if let Some(script_def) = &def.script {
                if script_def.code.trim().is_empty() {
                    errors.push("Level 3 equipment requires non-empty script code".to_string());
                } else {
                    let sandbox = RhaiSandbox::new();
                    match sandbox.compile_script(&script_def.code) {
                        Ok(ast) => {
                            // Perform dry-run evaluation with dummy inputs to verify safety & instruction limits
                            let mut test_ctx = ScriptExecutionContext {
                                dt: 0.05,
                                ..Default::default()
                            };
                            for (p_name, p) in &def.parameters {
                                test_ctx.parameters.insert(p_name.clone(), p.default_value);
                            }
                            for port in &def.ports {
                                test_ctx.inputs.insert(port.name.clone(), 10.0);
                                test_ctx.inputs.insert(format!("{}_flow", port.name), 10.0);
                                test_ctx
                                    .inputs
                                    .insert(format!("{}_pressure", port.name), 101.3);
                                test_ctx
                                    .inputs
                                    .insert(format!("{}_temperature", port.name), 300.0);
                            }
                            // Common physical process variable names with safe non-zero defaults
                            test_ctx.inputs.insert("flow".into(), 10.0);
                            test_ctx.inputs.insert("temperature".into(), 300.0);
                            test_ctx.inputs.insert("pressure".into(), 100.0);
                            test_ctx.inputs.insert("suction_pressure".into(), 100.0);
                            test_ctx.inputs.insert("feed_flow".into(), 10.0);
                            test_ctx.inputs.insert("in".into(), 1.0);
                            if let Err(e) =
                                sandbox.execute_step(&ast, &script_def.entry_point, &test_ctx)
                            {
                                errors.push(format!(
                                    "Script dry-run test failed: {e} (ensure entry point '{}' exists and is callable)",
                                    script_def.entry_point
                                ));
                            }
                        }
                        Err(e) => {
                            errors.push(format!("Script compilation failed: {e}"));
                        }
                    }
                }
            } else {
                errors.push("Level 3 equipment requires script definition".to_string());
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(ScriptingError::Validation(errors.join("; ")))
    }
}
