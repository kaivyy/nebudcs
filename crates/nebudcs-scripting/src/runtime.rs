//! Runtime execution engine for custom equipment instances.
//!
//! Integrates data-driven custom equipment with the double-buffered simulation
//! state, control system, alarm engine, and tag store without recompiling Rust.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_model::tag::{TagQuality, TagValue};
use nebudcs_sim::{SimError, SimulationSystem, StateReadView, StateWriteView, StepContext};
use nebudcs_types::{EquipmentId, SchemaVersion, TagId};
use rhai::AST;

use crate::error::{ScriptingError, ScriptingResult};
use crate::library::{
    EquipmentLibrary, EquipmentTemplate, PackageInstantiationResult, TemplateMetadata,
};
use crate::model::{CustomEquipmentDefinition, CustomizationTier};
use crate::sandbox::{RhaiSandbox, ScriptExecutionContext};
use crate::validation::validate_custom_equipment;

/// Pre-compiled and validated custom equipment definition.
#[derive(Clone)]
pub struct CompiledDefinition {
    pub def: CustomEquipmentDefinition,
    pub compiled_ast: Option<Arc<AST>>,
}

/// An active runtime instance of a custom equipment definition.
#[derive(Debug, Clone)]
pub struct CustomEquipmentInstance {
    pub id: EquipmentId,
    pub definition_id: String,
    pub name: String,
    /// Configured parameters (defaults overridden by instance configs).
    pub parameters: BTreeMap<String, f64>,
    /// Continuous process variables (e.g. "flow", "delta_p", "speed", "vibration", "temperature").
    pub variables: BTreeMap<String, f64>,
    /// Discrete status flags (e.g. "is_running", "is_tripped", "permissive_ok").
    pub flags: BTreeMap<String, bool>,
    /// Active state machine state.
    pub active_state: String,
    /// Currently latched trip condition IDs.
    pub latched_trips: Vec<String>,
}

impl CustomEquipmentInstance {
    /// Retrieves a continuous process variable.
    pub fn get_var(&self, name: &str) -> Option<f64> {
        self.variables.get(name).copied()
    }

    /// Sets a continuous process variable.
    pub fn set_var(&mut self, name: impl Into<String>, val: f64) {
        self.variables.insert(name.into(), val);
    }

    /// Retrieves a discrete boolean flag.
    pub fn get_flag(&self, name: &str) -> Option<bool> {
        self.flags.get(name).copied()
    }

    /// Sets a discrete boolean flag.
    pub fn set_flag(&mut self, name: impl Into<String>, val: bool) {
        self.flags.insert(name.into(), val);
    }

    /// Checks whether the equipment is currently running.
    pub fn is_running(&self) -> bool {
        self.active_state == "running" && self.latched_trips.is_empty()
    }

    /// Checks whether the equipment is currently tripped.
    pub fn is_tripped(&self) -> bool {
        !self.latched_trips.is_empty() || self.active_state == "tripped"
    }
}

/// Central manager orchestrating custom equipment definitions and instances.
pub struct CustomEquipmentManager {
    definitions: HashMap<String, CompiledDefinition>,
    instances: HashMap<EquipmentId, CustomEquipmentInstance>,
    sandbox: RhaiSandbox,
    /// Mapping of (EquipmentId, variable_name) -> TagId for tag synchronization.
    tag_bindings: HashMap<(EquipmentId, String), TagId>,
    /// Equipment library storing standard and user templates and packages.
    library: EquipmentLibrary,
}

impl Default for CustomEquipmentManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CustomEquipmentManager {
    /// Creates a new custom equipment manager.
    pub fn new() -> Self {
        let mut library = EquipmentLibrary::new();
        let _ = library.load_standard_catalog();
        Self {
            definitions: HashMap::new(),
            instances: HashMap::new(),
            sandbox: RhaiSandbox::new(),
            tag_bindings: HashMap::new(),
            library,
        }
    }

    /// Registers and compiles a new custom equipment definition.
    pub fn register_definition(&mut self, def: CustomEquipmentDefinition) -> ScriptingResult<()> {
        validate_custom_equipment(&def)?;

        let compiled_ast = if def.tier == CustomizationTier::Level3Scripted {
            if let Some(script) = &def.script {
                let ast = self.sandbox.compile_script(&script.code)?;
                Some(Arc::new(ast))
            } else {
                None
            }
        } else {
            None
        };

        let id = def.id.clone();
        self.definitions
            .insert(id, CompiledDefinition { def, compiled_ast });
        Ok(())
    }

    /// Retrieves a registered definition.
    pub fn get_definition(&self, id: &str) -> Option<&CustomEquipmentDefinition> {
        self.definitions.get(id).map(|c| &c.def)
    }

    /// Lists all registered definition IDs.
    pub fn list_definitions(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.definitions.keys().cloned().collect();
        keys.sort();
        keys
    }

    /// Lists all registered definition objects.
    pub fn list_definition_objects(&self) -> Vec<&CustomEquipmentDefinition> {
        self.definitions.values().map(|c| &c.def).collect()
    }

    /// Instantiates custom equipment in the runtime model.
    pub fn instantiate(
        &mut self,
        instance_id: EquipmentId,
        definition_id: &str,
        name: impl Into<String>,
        param_overrides: &BTreeMap<String, f64>,
    ) -> ScriptingResult<&CustomEquipmentInstance> {
        let compiled = self
            .definitions
            .get(definition_id)
            .ok_or_else(|| ScriptingError::NotFound(definition_id.to_string()))?;

        let mut parameters = BTreeMap::new();
        for (k, param_def) in &compiled.def.parameters {
            parameters.insert(k.clone(), param_def.default_value);
        }
        for (k, v) in param_overrides {
            if !v.is_finite() {
                return Err(ScriptingError::Instance(format!(
                    "Parameter override for '{k}' is non-finite: {v}"
                )));
            }
            if let Some(param_def) = compiled.def.parameters.get(k) {
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

        let mut variables = BTreeMap::new();
        variables.insert("speed".to_string(), 0.0);
        variables.insert("flow".to_string(), 0.0);
        variables.insert("delta_p".to_string(), 0.0);
        variables.insert("power".to_string(), 0.0);
        variables.insert("efficiency".to_string(), 1.0);
        variables.insert("vibration".to_string(), 0.0);

        let mut flags = BTreeMap::new();
        flags.insert("is_running".to_string(), false);
        flags.insert("is_tripped".to_string(), false);
        flags.insert("permissive_ok".to_string(), true);

        let instance = CustomEquipmentInstance {
            id: instance_id,
            definition_id: definition_id.to_string(),
            name: name.into(),
            parameters,
            variables,
            flags,
            active_state: "stopped".to_string(),
            latched_trips: Vec::new(),
        };

        self.instances.insert(instance_id, instance);
        Ok(self.instances.get(&instance_id).unwrap())
    }

    /// Access reference to the equipment library.
    pub fn library(&self) -> &EquipmentLibrary {
        &self.library
    }

    /// Mutable reference to the equipment library.
    pub fn library_mut(&mut self) -> &mut EquipmentLibrary {
        &mut self.library
    }

    /// Instantiates an equipment template from the library into runtime simulation state.
    pub fn instantiate_template(
        &mut self,
        template_id: &str,
        version: Option<&SchemaVersion>,
        instance_id: EquipmentId,
        name: impl Into<String>,
        param_overrides: &BTreeMap<String, f64>,
    ) -> ScriptingResult<&CustomEquipmentInstance> {
        let tmpl = self
            .library
            .get_template(template_id, version)
            .cloned()
            .ok_or_else(|| {
                ScriptingError::NotFound(format!("Template '{template_id}' not found"))
            })?;

        let instance = tmpl.instantiate(instance_id, name, param_overrides)?;

        // Ensure definition is registered
        if !self.definitions.contains_key(&tmpl.definition.id) {
            self.register_definition(tmpl.definition.clone())?;
        }

        self.instances.insert(instance_id, instance);
        Ok(self.instances.get(&instance_id).unwrap())
    }

    /// Instantiates a composite package into runtime simulation state.
    pub fn instantiate_package(
        &mut self,
        package_id: &str,
        version: Option<&SchemaVersion>,
        prefix: &str,
        param_overrides: &BTreeMap<String, f64>,
    ) -> ScriptingResult<PackageInstantiationResult> {
        let result =
            self.library
                .instantiate_package(package_id, version, prefix, param_overrides)?;

        for inst in &result.instances {
            // Ensure child definitions are registered
            if !self.definitions.contains_key(&inst.definition_id) {
                if let Some(tmpl) = self.library.get_template(&inst.definition_id, None) {
                    let def = tmpl.definition.clone();
                    self.register_definition(def)?;
                }
            }
            self.instances.insert(inst.id, inst.clone());
        }

        Ok(result)
    }

    /// Promotes a registered custom definition to a reusable user template in the library.
    pub fn promote_to_user_template(
        &mut self,
        definition_id: &str,
        metadata: Option<TemplateMetadata>,
    ) -> ScriptingResult<EquipmentTemplate> {
        let def = self.get_definition(definition_id).cloned().ok_or_else(|| {
            ScriptingError::NotFound(format!("Definition '{definition_id}' not found"))
        })?;
        self.library.register_user_template(def, metadata)
    }

    /// Binds an equipment variable to a runtime tag for automated synchronization.
    pub fn bind_tag(
        &mut self,
        eq_id: EquipmentId,
        variable_name: impl Into<String>,
        tag_id: TagId,
    ) {
        self.tag_bindings
            .insert((eq_id, variable_name.into()), tag_id);
    }

    /// Retrieves an instance reference.
    pub fn get_instance(&self, id: EquipmentId) -> Option<&CustomEquipmentInstance> {
        self.instances.get(&id)
    }

    /// Retrieves a mutable instance reference.
    pub fn get_instance_mut(&mut self, id: EquipmentId) -> Option<&mut CustomEquipmentInstance> {
        self.instances.get_mut(&id)
    }

    /// Lists all active equipment instances.
    pub fn list_instances(&self) -> Vec<&CustomEquipmentInstance> {
        self.instances.values().collect()
    }

    /// Commands equipment start (permissives and trips checked).
    pub fn start(&mut self, id: EquipmentId) -> ScriptingResult<()> {
        let inst = self
            .instances
            .get_mut(&id)
            .ok_or_else(|| ScriptingError::NotFound(id.to_string()))?;

        if !inst.latched_trips.is_empty() {
            return Err(ScriptingError::Instance(format!(
                "Cannot start equipment '{}': protective trip active ({:?})",
                inst.name, inst.latched_trips
            )));
        }

        inst.active_state = "running".to_string();
        inst.flags.insert("is_running".to_string(), true);
        inst.flags.insert("is_tripped".to_string(), false);
        Ok(())
    }

    /// Commands equipment stop.
    pub fn stop(&mut self, id: EquipmentId) -> ScriptingResult<()> {
        let inst = self
            .instances
            .get_mut(&id)
            .ok_or_else(|| ScriptingError::NotFound(id.to_string()))?;

        inst.active_state = "stopped".to_string();
        inst.flags.insert("is_running".to_string(), false);
        inst.variables.insert("speed".to_string(), 0.0);
        inst.variables.insert("delta_p".to_string(), 0.0);
        inst.variables.insert("flow".to_string(), 0.0);
        Ok(())
    }

    /// Attempts to reset latched trips on an equipment instance.
    pub fn reset_trip(&mut self, id: EquipmentId) -> ScriptingResult<bool> {
        let inst = self
            .instances
            .get_mut(&id)
            .ok_or_else(|| ScriptingError::NotFound(id.to_string()))?;

        let compiled = self
            .definitions
            .get(&inst.definition_id)
            .ok_or_else(|| ScriptingError::NotFound(inst.definition_id.clone()))?;

        // Re-check trip conditions: cannot reset if trip condition is still actively violated
        let mut still_tripped = false;
        for trip_id in &inst.latched_trips {
            if let Some(trip_def) = compiled.def.trips.iter().find(|t| &t.id == trip_id) {
                let curr_val = inst
                    .variables
                    .get(&trip_def.condition_var)
                    .copied()
                    .unwrap_or(0.0);
                if trip_def.operator.evaluate(curr_val, trip_def.threshold) {
                    still_tripped = true;
                    break;
                }
            }
        }

        if still_tripped {
            Ok(false)
        } else {
            inst.latched_trips.clear();
            inst.flags.insert("is_tripped".to_string(), false);
            if inst.active_state == "tripped" {
                inst.active_state = "stopped".to_string();
            }
            Ok(true)
        }
    }

    /// Steps all custom equipment instances for duration `dt`.
    pub fn step(
        &mut self,
        dt: f64,
        inputs_by_eq: &HashMap<EquipmentId, BTreeMap<String, f64>>,
        tag_store: &mut RuntimeTagStore,
    ) -> ScriptingResult<()> {
        for (eq_id, inst) in &mut self.instances {
            let compiled = match self.definitions.get(&inst.definition_id) {
                Some(c) => c,
                None => continue,
            };

            let default_inputs = BTreeMap::new();
            let inputs = inputs_by_eq.get(eq_id).unwrap_or(&default_inputs);

            // Read tag store values as additional inputs if bound
            let mut full_inputs = inputs.clone();
            for tag_ref in tag_store.tags() {
                if let Some(f) = tag_store.get_value_float(tag_ref.id) {
                    full_inputs.insert(tag_ref.name.clone(), f);
                }
            }

            for (k, v) in &inst.variables {
                full_inputs.entry(k.clone()).or_insert(*v);
            }

            // 1. Evaluate according to tier
            match compiled.def.tier {
                CustomizationTier::Level1Declarative => {
                    if inst.is_running() {
                        // Evaluate 1D curves
                        for (_c_name, curve) in &compiled.def.curves {
                            let in_val = full_inputs
                                .get(&curve.input_var)
                                .or_else(|| inst.variables.get(&curve.input_var))
                                .or_else(|| inst.parameters.get(&curve.input_var))
                                .copied()
                                .unwrap_or(0.0);

                            if let Ok(out_val) = curve.evaluate(in_val) {
                                inst.variables.insert(curve.output_var.clone(), out_val);
                            }
                        }

                        // Evaluate 2D curves
                        for (_c_name, curve2d) in &compiled.def.curves_2d {
                            let in_x = full_inputs
                                .get(&curve2d.input_x_var)
                                .or_else(|| inst.variables.get(&curve2d.input_x_var))
                                .copied()
                                .unwrap_or(0.0);
                            let in_y = full_inputs
                                .get(&curve2d.input_y_var)
                                .or_else(|| inst.variables.get(&curve2d.input_y_var))
                                .copied()
                                .unwrap_or(0.0);

                            if let Ok(out_z) = curve2d.evaluate(in_x, in_y) {
                                inst.variables.insert(curve2d.output_var.clone(), out_z);
                            }
                        }
                    } else {
                        // Stopped / Tripped: zero head and flow
                        inst.variables.insert("delta_p".to_string(), 0.0);
                        inst.variables.insert("flow".to_string(), 0.0);
                        inst.variables.insert("power".to_string(), 0.0);
                    }
                }
                CustomizationTier::Level2Composition => {
                    // Level 2 evaluates sub-components and curves
                    if inst.is_running() {
                        for (_c_name, curve) in &compiled.def.curves {
                            let in_val = full_inputs
                                .get(&curve.input_var)
                                .or_else(|| inst.variables.get(&curve.input_var))
                                .copied()
                                .unwrap_or(0.0);
                            if let Ok(out_val) = curve.evaluate(in_val) {
                                inst.variables.insert(curve.output_var.clone(), out_val);
                            }
                        }
                    }
                }
                CustomizationTier::Level3Scripted => {
                    if let (Some(script_def), Some(ast)) =
                        (&compiled.def.script, &compiled.compiled_ast)
                    {
                        let ctx = ScriptExecutionContext {
                            dt,
                            inputs: full_inputs,
                            parameters: inst.parameters.clone(),
                            state_variables: inst.variables.clone(),
                            state_flags: inst.flags.clone(),
                        };

                        let output =
                            self.sandbox
                                .execute_step(ast, &script_def.entry_point, &ctx)?;

                        // Merge updated variables and flags
                        for (k, v) in output.variables {
                            inst.variables.insert(k, v);
                        }
                        for (k, b) in output.flags {
                            inst.flags.insert(k, b);
                        }

                        if output.trip_commanded {
                            inst.latched_trips.push("SCRIPT_TRIP".to_string());
                            inst.active_state = "tripped".to_string();
                            inst.flags.insert("is_tripped".to_string(), true);
                            inst.flags.insert("is_running".to_string(), false);
                        }
                    }
                }
            }

            // 2. Evaluate Operating Limits & Trips
            for trip in &compiled.def.trips {
                let curr_val = inst
                    .variables
                    .get(&trip.condition_var)
                    .copied()
                    .unwrap_or(0.0);
                if trip.operator.evaluate(curr_val, trip.threshold) {
                    if !inst.latched_trips.contains(&trip.id) {
                        inst.latched_trips.push(trip.id.clone());
                    }
                    inst.active_state = trip.fail_safe_state.clone();
                    inst.flags.insert("is_tripped".to_string(), true);
                    inst.flags.insert("is_running".to_string(), false);
                    inst.variables.insert("speed".to_string(), 0.0);
                    inst.variables.insert("delta_p".to_string(), 0.0);
                    inst.variables.insert("flow".to_string(), 0.0);
                }
            }

            // 3. Evaluate Interlocks
            for interlock in &compiled.def.interlocks {
                let cause_val = inst
                    .variables
                    .get(&interlock.cause_var)
                    .copied()
                    .unwrap_or(0.0);
                if interlock
                    .operator
                    .evaluate(cause_val, interlock.cause_threshold)
                {
                    inst.variables
                        .insert(interlock.effect_var.clone(), interlock.effect_value);
                }
            }

            // 4. Synchronize state with tag store
            for ((b_eq_id, var_name), tag_id) in &self.tag_bindings {
                if b_eq_id == eq_id {
                    if let Some(&val) = inst.variables.get(var_name) {
                        tag_store.set_tag_float(*tag_id, val, TagQuality::Good);
                    }
                }
            }

            // Also auto-sync standard status tags if present: "NAME.STATUS"
            let status_tag_name = format!("{}.STATUS", inst.name);
            if let Some(tag_ref) = tag_store.tags().iter().find(|t| t.name == status_tag_name) {
                let status_val = if inst.is_running() { 1 } else { 0 };
                tag_store.set_tag(tag_ref.id, TagValue::Integer(status_val), TagQuality::Good);
            }
        }

        Ok(())
    }
}

/// SimulationSystem wrapper for CustomEquipmentManager to integrate directly into SimulationEngine.
pub struct CustomEquipmentSystem {
    manager: Arc<std::sync::Mutex<CustomEquipmentManager>>,
}

impl CustomEquipmentSystem {
    pub fn new(manager: Arc<std::sync::Mutex<CustomEquipmentManager>>) -> Self {
        Self { manager }
    }
}

impl SimulationSystem for CustomEquipmentSystem {
    fn name(&self) -> &str {
        "CustomEquipmentSystem"
    }

    fn step(
        &mut self,
        ctx: StepContext,
        read_state: &StateReadView<'_>,
        write_state: &mut StateWriteView<'_>,
    ) -> Result<(), SimError> {
        let mut mgr = self.manager.lock().unwrap();
        let mut inputs_by_eq = HashMap::new();

        // Extract inputs from StateReadView
        for eq_id in mgr.instances.keys() {
            let mut inputs = BTreeMap::new();
            if let Ok(eq_state) = read_state.equipment(*eq_id) {
                for (k, &v) in &eq_state.variables {
                    inputs.insert(k.clone(), v);
                }
            }
            inputs_by_eq.insert(*eq_id, inputs);
        }

        let mut dummy_store = RuntimeTagStore::new();
        let _ = mgr.step(ctx.dt, &inputs_by_eq, &mut dummy_store);

        // Commit outputs into write_state
        for (eq_id, inst) in &mgr.instances {
            let _ = write_state.mutate_equipment(*eq_id, |eq| {
                eq.active_state = inst.active_state.clone();
                for (k, &v) in &inst.variables {
                    eq.set_var(k, v);
                }
                for (k, &b) in &inst.flags {
                    eq.set_flag(k, b);
                }
            });
        }

        Ok(())
    }
}
