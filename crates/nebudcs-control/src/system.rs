//! Control engine simulation system integrating instrumentation, PID, protection, and actuators.

use std::collections::BTreeMap;

use nebudcs_model::tag::TagQuality;
use nebudcs_sim::{SimError, SimulationSystem, StateReadView, StateWriteView, StepContext};
use nebudcs_types::{ControllerId, EquipmentId, TagId};
use serde::{Deserialize, Serialize};

use crate::actuator::{
    ActuatorArbiter, ActuatorCommand, ActuatorConfig, ActuatorModel, CommandPriority,
};
use crate::hal::IoSystem;
use crate::instrument::{InstrumentConfig, InstrumentModel};
use crate::pid::{PidConfig, PidController};
use crate::protection::{ProtectionBlock, ProtectionConfig};
use crate::runtime::VirtualController;

/// Registration for a process transmitter bound to an equipment physical variable.
#[derive(Debug, Clone)]
struct RegisteredInstrument {
    equipment_id: EquipmentId,
    var_name: String,
    model: InstrumentModel,
}

/// Registration for a PID control loop.
#[derive(Debug, Clone)]
struct RegisteredPid {
    pv_tag: TagId,
    target_actuator: EquipmentId,
    controller: PidController,
}

/// Registration for a physical actuator positioner.
#[derive(Debug, Clone)]
pub(crate) struct RegisteredActuator {
    pub(crate) var_name: String,
    pub(crate) arbiter: ActuatorArbiter,
    pub(crate) model: ActuatorModel,
    pub(crate) feedback_tag: Option<TagId>,
    pub(crate) power_air_ok: bool,
}

/// Registration for a protective trip function.
#[derive(Debug, Clone)]
struct RegisteredProtection {
    monitored_tag: TagId,
    target_actuator: EquipmentId,
    override_priority: CommandPriority,
    safe_position: f64,
    block: ProtectionBlock,
    reset_requested: bool,
}

/// Binding mapping a simulation state tag to a controller input image point (%I).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControllerInputBinding {
    /// Source tag in simulation state.
    pub tag_id: TagId,
    /// Destination virtual controller ID.
    pub controller_id: ControllerId,
    /// Key in the controller's Input Image Table (%I).
    pub signal_key: String,
}

/// Binding mapping a controller output image point (%Q) to an actuator or tag.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControllerOutputBinding {
    /// Source virtual controller ID.
    pub controller_id: ControllerId,
    /// Key in the controller's Output Image Table (%Q).
    pub signal_key: String,
    /// Target physical/virtual actuator equipment ID (if driving an actuator).
    pub target_actuator: Option<EquipmentId>,
    /// Target tag in simulation state (if publishing to a tag).
    pub target_tag: Option<TagId>,
    /// Command arbitration priority.
    pub priority: CommandPriority,
}

/// Comprehensive control and automation simulation system.
#[derive(Debug, Default)]
pub struct ControlSystem {
    instruments: BTreeMap<TagId, RegisteredInstrument>,
    pids: BTreeMap<TagId, RegisteredPid>,
    pub(crate) actuators: BTreeMap<EquipmentId, RegisteredActuator>,
    protections: BTreeMap<String, RegisteredProtection>,
    controllers: BTreeMap<ControllerId, VirtualController>,
    strategies: BTreeMap<String, crate::advanced::AdvancedControlStrategy>,
    input_bindings: Vec<ControllerInputBinding>,
    output_bindings: Vec<ControllerOutputBinding>,
    pub io_system: IoSystem,
    pub sis: crate::sis::SafetyInstrumentedSystem,
}

impl ControlSystem {
    /// Creates an empty control system.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a transmitter measuring an equipment physical variable and publishing to a tag.
    pub fn register_instrument(
        &mut self,
        equipment_id: EquipmentId,
        var_name: impl Into<String>,
        pv_tag: TagId,
        config: InstrumentConfig,
        initial_val: f64,
    ) {
        let model = InstrumentModel::new(config, initial_val);
        self.instruments.insert(
            pv_tag,
            RegisteredInstrument {
                equipment_id,
                var_name: var_name.into(),
                model,
            },
        );
    }

    /// Registers an actuator driving an equipment variable (e.g. "opening").
    pub fn register_actuator(
        &mut self,
        equipment_id: EquipmentId,
        var_name: impl Into<String>,
        config: ActuatorConfig,
        initial_position: f64,
        feedback_tag: Option<TagId>,
    ) {
        let fail_action = config.fail_action;
        let arbiter = ActuatorArbiter::new(fail_action);
        let model = ActuatorModel::new(config, initial_position);
        self.actuators.insert(
            equipment_id,
            RegisteredActuator {
                var_name: var_name.into(),
                arbiter,
                model,
                feedback_tag,
                power_air_ok: true,
            },
        );
    }

    /// Registers a PID controller loop.
    pub fn register_pid(
        &mut self,
        pv_tag: TagId,
        out_tag: TagId,
        target_actuator: EquipmentId,
        config: PidConfig,
    ) {
        let controller = PidController::new(config);
        self.pids.insert(
            out_tag,
            RegisteredPid {
                pv_tag,
                target_actuator,
                controller,
            },
        );
    }

    /// Registers a protective trip function.
    pub fn register_protection(
        &mut self,
        name: impl Into<String>,
        monitored_tag: TagId,
        target_actuator: EquipmentId,
        override_priority: CommandPriority,
        safe_position: f64,
        config: ProtectionConfig,
    ) {
        let name_str = name.into();
        let block = ProtectionBlock::new(config);
        self.protections.insert(
            name_str,
            RegisteredProtection {
                monitored_tag,
                target_actuator,
                override_priority,
                safe_position,
                block,
                reset_requested: false,
            },
        );
    }

    /// Registers a protective trip function with an explicit trip direction (High or Low).
    #[allow(clippy::too_many_arguments)]
    pub fn register_protection_with_direction(
        &mut self,
        name: impl Into<String>,
        monitored_tag: TagId,
        target_actuator: EquipmentId,
        override_priority: CommandPriority,
        safe_position: f64,
        threshold: f64,
        reset_deadband: f64,
        direction: crate::protection::TripDirection,
    ) {
        let name_str = name.into();
        let config = ProtectionConfig {
            name: name_str.clone(),
            threshold,
            reset_deadband,
            direction,
        };
        let block = ProtectionBlock::new(config);
        self.protections.insert(
            name_str,
            RegisteredProtection {
                monitored_tag,
                target_actuator,
                override_priority,
                safe_position,
                block,
                reset_requested: false,
            },
        );
    }

    /// Clears all registered instruments, PIDs, actuators, protection blocks, controllers, and strategies.
    pub fn clear(&mut self) {
        self.instruments.clear();
        self.pids.clear();
        self.actuators.clear();
        self.protections.clear();
        self.controllers.clear();
        self.strategies.clear();
        self.input_bindings.clear();
        self.output_bindings.clear();
    }

    /// Registers an advanced control strategy into the control system.
    pub fn register_strategy(&mut self, strategy: crate::advanced::AdvancedControlStrategy) {
        self.strategies.insert(strategy.id.clone(), strategy);
    }

    /// Removes an advanced control strategy by ID.
    pub fn remove_strategy(
        &mut self,
        id: &str,
    ) -> Option<crate::advanced::AdvancedControlStrategy> {
        self.strategies.remove(id)
    }

    /// Access to an advanced control strategy by ID.
    pub fn strategy(&self, id: &str) -> Option<&crate::advanced::AdvancedControlStrategy> {
        self.strategies.get(id)
    }

    /// Access to a mutable advanced control strategy by ID.
    pub fn strategy_mut(
        &mut self,
        id: &str,
    ) -> Option<&mut crate::advanced::AdvancedControlStrategy> {
        self.strategies.get_mut(id)
    }

    /// Returns a map of all registered advanced control strategies.
    pub fn strategies(&self) -> &BTreeMap<String, crate::advanced::AdvancedControlStrategy> {
        &self.strategies
    }

    /// Returns a mutable map of all registered advanced control strategies.
    pub fn strategies_mut(
        &mut self,
    ) -> &mut BTreeMap<String, crate::advanced::AdvancedControlStrategy> {
        &mut self.strategies
    }

    /// Registers a virtual controller into the control system.
    pub fn register_controller(&mut self, controller: VirtualController) {
        self.controllers.insert(controller.id(), controller);
    }

    /// Removes a virtual controller by ID.
    pub fn remove_controller(&mut self, id: ControllerId) -> Option<VirtualController> {
        self.input_bindings.retain(|b| b.controller_id != id);
        self.output_bindings.retain(|b| b.controller_id != id);
        self.controllers.remove(&id)
    }

    /// Access to an immutable reference of a virtual controller.
    pub fn controller(&self, id: ControllerId) -> Option<&VirtualController> {
        self.controllers.get(&id)
    }

    /// Access to a mutable reference of a virtual controller.
    pub fn controller_mut(&mut self, id: ControllerId) -> Option<&mut VirtualController> {
        self.controllers.get_mut(&id)
    }

    /// Finds a virtual controller by its human-readable name.
    pub fn controller_by_name(&self, name: &str) -> Option<&VirtualController> {
        self.controllers.values().find(|c| c.name() == name)
    }

    /// Finds a mutable virtual controller by its human-readable name.
    pub fn controller_by_name_mut(&mut self, name: &str) -> Option<&mut VirtualController> {
        self.controllers.values_mut().find(|c| c.name() == name)
    }

    /// Returns a map of all registered virtual controllers.
    pub fn controllers(&self) -> &BTreeMap<ControllerId, VirtualController> {
        &self.controllers
    }

    /// Returns a mutable map of all registered virtual controllers.
    pub fn controllers_mut(&mut self) -> &mut BTreeMap<ControllerId, VirtualController> {
        &mut self.controllers
    }

    /// Binds an input tag from the simulation state to a controller's Input Image Table (%I).
    pub fn bind_input(
        &mut self,
        tag_id: TagId,
        controller_id: ControllerId,
        signal_key: impl Into<String>,
    ) {
        self.input_bindings.push(ControllerInputBinding {
            tag_id,
            controller_id,
            signal_key: signal_key.into(),
        });
    }

    /// Binds a controller output point (%Q) to drive a physical or virtual actuator.
    pub fn bind_output_actuator(
        &mut self,
        controller_id: ControllerId,
        signal_key: impl Into<String>,
        target_actuator: EquipmentId,
        priority: CommandPriority,
    ) {
        self.output_bindings.push(ControllerOutputBinding {
            controller_id,
            signal_key: signal_key.into(),
            target_actuator: Some(target_actuator),
            target_tag: None,
            priority,
        });
    }

    /// Binds a controller output point (%Q) to publish directly to a simulation state tag.
    pub fn bind_output_tag(
        &mut self,
        controller_id: ControllerId,
        signal_key: impl Into<String>,
        target_tag: TagId,
    ) {
        self.output_bindings.push(ControllerOutputBinding {
            controller_id,
            signal_key: signal_key.into(),
            target_actuator: None,
            target_tag: Some(target_tag),
            priority: CommandPriority::Auto,
        });
    }

    /// Access to registered controller input bindings.
    pub fn input_bindings(&self) -> &[ControllerInputBinding] {
        &self.input_bindings
    }

    /// Access to registered controller output bindings.
    pub fn output_bindings(&self) -> &[ControllerOutputBinding] {
        &self.output_bindings
    }

    /// Immutable access to the I/O Hardware Abstraction Layer (HAL).
    pub fn io_system(&self) -> &IoSystem {
        &self.io_system
    }

    /// Mutable access to the I/O Hardware Abstraction Layer (HAL).
    pub fn io_system_mut(&mut self) -> &mut IoSystem {
        &mut self.io_system
    }

    /// Replaces the I/O Hardware Abstraction Layer configuration.
    pub fn set_io_system(&mut self, io_system: IoSystem) {
        self.io_system = io_system;
    }

    /// Access to the Safety Instrumented System (SIS) engine.
    pub fn sis(&self) -> &crate::sis::SafetyInstrumentedSystem {
        &self.sis
    }

    /// Mutable access to the Safety Instrumented System (SIS) engine.
    pub fn sis_mut(&mut self) -> &mut crate::sis::SafetyInstrumentedSystem {
        &mut self.sis
    }

    /// Resets all registered virtual controllers deterministically.
    pub fn reset_controllers(&mut self) {
        for ctrl in self.controllers.values_mut() {
            let _ = ctrl.reset();
        }
    }

    /// Requests a manual operator reset on a named protection block.
    pub fn request_protection_reset(&mut self, name: &str) -> bool {
        if let Some(prot) = self.protections.get_mut(name) {
            prot.reset_requested = true;
            true
        } else {
            false
        }
    }

    /// Access to a mutable PID controller by its output tag ID or PV tag ID.
    pub fn pid_mut(&mut self, tag: TagId) -> Option<&mut PidController> {
        if self.pids.contains_key(&tag) {
            return self.pids.get_mut(&tag).map(|r| &mut r.controller);
        }
        self.pids
            .values_mut()
            .find(|r| r.pv_tag == tag)
            .map(|r| &mut r.controller)
    }

    /// Access to a PID controller by its output tag ID or PV tag ID.
    pub fn pid(&self, tag: TagId) -> Option<&PidController> {
        if let Some(r) = self.pids.get(&tag) {
            return Some(&r.controller);
        }
        self.pids
            .values()
            .find(|r| r.pv_tag == tag)
            .map(|r| &r.controller)
    }

    /// Access to a mutable protection block by name.
    pub fn protection_mut(&mut self, name: &str) -> Option<&mut ProtectionBlock> {
        self.protections.get_mut(name).map(|r| &mut r.block)
    }

    /// Access to an immutable protection block by name.
    pub fn protection(&self, name: &str) -> Option<&ProtectionBlock> {
        self.protections.get(name).map(|r| &r.block)
    }

    /// Returns an iterator over all registered protection block names and blocks.
    pub fn protections(&self) -> impl Iterator<Item = (&str, &ProtectionBlock)> {
        self.protections.iter().map(|(k, v)| (k.as_str(), &v.block))
    }

    /// Checks if an actuator model exists for the given equipment ID.
    pub fn has_actuator(&self, equipment_id: EquipmentId) -> bool {
        self.actuators.contains_key(&equipment_id)
    }

    /// Access to an actuator model by equipment ID.
    pub fn actuator(&self, equipment_id: EquipmentId) -> Option<&ActuatorModel> {
        self.actuators.get(&equipment_id).map(|r| &r.model)
    }

    /// Simulates supply air / electrical power loss to an actuator.
    pub fn set_actuator_power_air(&mut self, equipment_id: EquipmentId, ok: bool) {
        if let Some(act) = self.actuators.get_mut(&equipment_id) {
            act.power_air_ok = ok;
        }
    }

    /// Simulates supply air / electrical power loss to all registered actuators.
    pub fn set_all_actuators_power_air(&mut self, ok: bool) {
        for act in self.actuators.values_mut() {
            act.power_air_ok = ok;
        }
    }

    /// Returns the number of actuators currently held in an ESD safety override state.
    pub fn active_safety_overrides_count(&self) -> usize {
        self.actuators
            .values()
            .filter(|r| r.arbiter.arbitrate().map(|c| c.priority) == Some(CommandPriority::Esd))
            .count()
    }

    /// Access to the primary process controller I/O image table.
    pub fn io_image(&self) -> crate::runtime::IoImageTable {
        if let Some(ctrl) = self.controllers.values().next() {
            ctrl.io_image().clone()
        } else {
            crate::runtime::IoImageTable::new()
        }
    }
}

impl SimulationSystem for ControlSystem {
    fn name(&self) -> &str {
        "ControlSystem"
    }

    fn step(
        &mut self,
        ctx: StepContext,
        read_state: &StateReadView<'_>,
        write_state: &mut StateWriteView<'_>,
    ) -> Result<(), SimError> {
        let dt = ctx.dt;

        // -------------------------------------------------------------------
        // Stage 1: Instruments & Transmitters
        // -------------------------------------------------------------------
        // Evaluated in deterministic BTreeMap key order (TagId binary bytes)
        for (pv_tag, inst) in &mut self.instruments {
            // Read physical variable from write_state (produced by physics in current tick)
            // or fallback to committed read_state
            let raw_phys = if let Some(eq) = write_state.get_equipment(inst.equipment_id) {
                eq.get_var(&inst.var_name).unwrap_or(0.0)
            } else if let Ok(eq) = read_state.equipment(inst.equipment_id) {
                eq.get_var(&inst.var_name).unwrap_or(0.0)
            } else {
                0.0
            };

            let measured = inst.model.step(dt, raw_phys, dt);
            let quality = inst.model.quality();

            write_state.set_tag(
                *pv_tag,
                nebudcs_model::tag::TagValue::Float(measured),
                quality,
            );
        }

        // -------------------------------------------------------------------
        // Stage 1.4: I/O Hardware Abstraction Layer (HAL) Input Scanning
        // -------------------------------------------------------------------
        let sim_time_sec = ctx.tick_count as f64 * dt;
        self.io_system
            .sample_inputs(read_state, write_state, &mut self.controllers, sim_time_sec);

        // -------------------------------------------------------------------
        // Stage 1.5: Virtual Controller Input Image (%I) Direct Ingestion (Legacy Fallback)
        // -------------------------------------------------------------------
        for binding in &self.input_bindings {
            let (val, quality) = match write_state.get_tag(binding.tag_id) {
                Some(nebudcs_model::tag::TagValue::Float(v)) => {
                    let q = write_state
                        .get_quality(binding.tag_id)
                        .unwrap_or(TagQuality::Good);
                    (*v, q)
                }
                _ => match read_state.tag_value(binding.tag_id) {
                    Ok(nebudcs_model::tag::TagValue::Float(v)) => {
                        let q = read_state
                            .tag_quality(binding.tag_id)
                            .unwrap_or(TagQuality::Good);
                        (*v, q)
                    }
                    _ => (0.0, TagQuality::Bad),
                },
            };

            if let Some(ctrl) = self.controllers.get_mut(&binding.controller_id) {
                ctrl.io_image_mut()
                    .write_input(&binding.signal_key, val, quality, sim_time_sec);
            }
        }

        // -------------------------------------------------------------------
        // Stage 1.6: Virtual Controller Deterministic Task & Scan Execution
        // -------------------------------------------------------------------
        for ctrl in self.controllers.values_mut() {
            let _ = ctrl.step(dt, sim_time_sec);
        }

        // -------------------------------------------------------------------
        // Stage 1.7: I/O Hardware Abstraction Layer (HAL) Output Dispatch
        // -------------------------------------------------------------------
        self.io_system.dispatch_outputs(
            &self.controllers,
            |act_id, cmd| {
                if let Some(act) = self.actuators.get_mut(&act_id) {
                    act.arbiter.submit(cmd);
                }
            },
            write_state,
            sim_time_sec,
        );

        // -------------------------------------------------------------------
        // Stage 1.75: Virtual Controller Output Image (%Q) Direct Dispatch (Legacy Fallback)
        // -------------------------------------------------------------------
        for binding in &self.output_bindings {
            if let Some(ctrl) = self.controllers.get(&binding.controller_id) {
                if let Some(out_sig) = ctrl.io_image().read_output(&binding.signal_key) {
                    if let Some(target_act) = binding.target_actuator {
                        if let Some(act) = self.actuators.get_mut(&target_act) {
                            act.arbiter.submit(
                                ActuatorCommand::new(binding.priority, out_sig.value)
                                    .with_source(ctrl.name()),
                            );
                        }
                    }
                    if let Some(target_tag) = binding.target_tag {
                        write_state.set_tag(
                            target_tag,
                            nebudcs_model::tag::TagValue::Float(out_sig.value),
                            out_sig.quality,
                        );
                    }
                }
            }
        }

        // -------------------------------------------------------------------
        // Stage 2: Protective Interlocks & ESD Evaluation
        // -------------------------------------------------------------------
        for (name, prot) in &mut self.protections {
            let monitored_val = match write_state.get_tag(prot.monitored_tag) {
                Some(nebudcs_model::tag::TagValue::Float(v)) => *v,
                _ => match read_state.tag_value(prot.monitored_tag) {
                    Ok(nebudcs_model::tag::TagValue::Float(v)) => *v,
                    _ => 0.0,
                },
            };

            let reset_cmd = prot.reset_requested;
            prot.reset_requested = false;

            let is_tripped = prot.block.step(monitored_val, reset_cmd);
            if is_tripped {
                // Assert high priority override on target actuator with source identifier
                if let Some(act) = self.actuators.get_mut(&prot.target_actuator) {
                    act.arbiter.submit(
                        ActuatorCommand::new(prot.override_priority, prot.safe_position)
                            .with_source(name.as_str()),
                    );
                }
            } else {
                // When trip condition has cleared and block is reset, clear this override command
                if let Some(act) = self.actuators.get_mut(&prot.target_actuator) {
                    act.arbiter.clear_source(name.as_str());
                }
            }
        }

        // -------------------------------------------------------------------
        // Stage 2.5: Safety Instrumented System (SIS) & SIF Evaluation
        // -------------------------------------------------------------------
        if self.sis.enabled {
            let sis_io = if let Some(ctrl) = self.controllers.values().next() {
                ctrl.io_image().clone()
            } else {
                crate::runtime::IoImageTable::new()
            };

            let mut sif_commands = Vec::new();
            self.sis.step(dt, sim_time_sec, &sis_io, &mut sif_commands);

            for (target_eq, cmd) in sif_commands {
                if let Ok(eq_id) = target_eq.parse::<EquipmentId>() {
                    if let Some(act) = self.actuators.get_mut(&eq_id) {
                        act.arbiter.submit(cmd);
                    }
                } else {
                    for (eq_id, act) in &mut self.actuators {
                        if eq_id.to_string() == target_eq {
                            act.arbiter.submit(cmd.clone());
                            break;
                        }
                    }
                }
            }

            for sif in self.sis.sifs().values() {
                if !sif.is_tripped() {
                    let source_id = format!("SIS:{}", sif.id());
                    for fe in &sif.config().final_elements {
                        if let Ok(eq_id) = fe.target_equipment.parse::<EquipmentId>() {
                            if let Some(act) = self.actuators.get_mut(&eq_id) {
                                act.arbiter.clear_source(&source_id);
                            }
                        } else {
                            for (eq_id, act) in &mut self.actuators {
                                if eq_id.to_string() == fe.target_equipment {
                                    act.arbiter.clear_source(&source_id);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        // -------------------------------------------------------------------
        // Stage 3: Regulatory Control (PID Loops)
        // -------------------------------------------------------------------
        // Evaluated in deterministic TagId sort order
        for (out_tag, reg_pid) in &mut self.pids {
            // Check if target actuator has an active safety override (ESD or Interlock)
            let override_active = if let Some(act) = self.actuators.get(&reg_pid.target_actuator) {
                if let Some(cmd) = act.arbiter.arbitrate() {
                    cmd.priority == CommandPriority::Esd
                        || cmd.priority == CommandPriority::Interlock
                } else {
                    false
                }
            } else {
                false
            };

            if override_active {
                // Controller enters tracking mode tracking actual valve position
                if let Some(act) = self.actuators.get(&reg_pid.target_actuator) {
                    reg_pid
                        .controller
                        .enter_tracking(act.model.actual_position());
                }
            } else if reg_pid.controller.mode() != crate::pid::PidMode::Manual {
                reg_pid.controller.exit_tracking();
            }

            // Read PV tag and quality from write_state (from Stage 1) or committed State[N]
            let (pv_val, pv_qual) = match write_state.get_tag(reg_pid.pv_tag) {
                Some(nebudcs_model::tag::TagValue::Float(v)) => {
                    let q = write_state
                        .get_quality(reg_pid.pv_tag)
                        .unwrap_or(TagQuality::Good);
                    (*v, q)
                }
                _ => match read_state.tag_value(reg_pid.pv_tag) {
                    Ok(nebudcs_model::tag::TagValue::Float(v)) => {
                        let q = read_state
                            .tag_quality(reg_pid.pv_tag)
                            .unwrap_or(TagQuality::Good);
                        (*v, q)
                    }
                    _ => (0.0, TagQuality::Bad),
                },
            };

            reg_pid.controller.set_quality(pv_qual);
            let out_val = reg_pid.controller.step(dt, pv_val);

            // Write output tag to State[N+1]
            write_state.set_tag_float(*out_tag, out_val);

            // Submit Priority 5 (Auto) command to target actuator arbiter
            if let Some(act) = self.actuators.get_mut(&reg_pid.target_actuator) {
                act.arbiter
                    .submit(ActuatorCommand::new(CommandPriority::Auto, out_val));
            }
        }

        // -------------------------------------------------------------------
        // Stage 3.5: Advanced Process Control Strategies
        // -------------------------------------------------------------------
        for strategy in self.strategies.values_mut() {
            strategy.step(dt, read_state, write_state, &mut self.actuators);
        }

        // -------------------------------------------------------------------
        // Stage 4: Actuator Arbitration, Dynamics & Physical State Update
        // -------------------------------------------------------------------
        for (eq_id, reg_act) in &mut self.actuators {
            // If commands are active, arbitrate winning command.
            // If all signals are lost, fallback to mechanical fail action position.
            let target_pos = match reg_act.arbiter.arbitrate() {
                Some(winning_cmd) => winning_cmd.target_position,
                None => match reg_act.model.config().fail_action {
                    crate::actuator::FailAction::FailClosed => 0.0,
                    crate::actuator::FailAction::FailOpen => 100.0,
                    crate::actuator::FailAction::FailInPlace => reg_act.model.actual_position(),
                },
            };

            let actual_pos = reg_act.model.step(dt, target_pos, reg_act.power_air_ok);

            // Update physical state in State[N+1]
            let _ = write_state.mutate_equipment(*eq_id, |eq| {
                if let Some(v) = eq.variables.get_mut(&reg_act.var_name) {
                    *v = actual_pos;
                } else {
                    eq.set_var(&reg_act.var_name, actual_pos);
                }
            });

            // Update feedback tag if configured
            if let Some(fb_tag) = reg_act.feedback_tag {
                write_state.set_tag(
                    fb_tag,
                    nebudcs_model::tag::TagValue::Float(actual_pos),
                    TagQuality::Good,
                );
            }
        }

        Ok(())
    }
}
