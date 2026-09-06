//! Control engine simulation system integrating instrumentation, PID, protection, and actuators.

use std::collections::BTreeMap;

use nebudcs_model::tag::TagQuality;
use nebudcs_sim::{SimError, SimulationSystem, StateReadView, StateWriteView, StepContext};
use nebudcs_types::{EquipmentId, TagId};

use crate::actuator::{
    ActuatorArbiter, ActuatorCommand, ActuatorConfig, ActuatorModel, CommandPriority,
};
use crate::instrument::{InstrumentConfig, InstrumentModel};
use crate::pid::{PidConfig, PidController};
use crate::protection::{ProtectionBlock, ProtectionConfig};

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
struct RegisteredActuator {
    var_name: String,
    arbiter: ActuatorArbiter,
    model: ActuatorModel,
    feedback_tag: Option<TagId>,
    power_air_ok: bool,
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

/// Comprehensive control and automation simulation system.
#[derive(Debug, Default)]
pub struct ControlSystem {
    instruments: BTreeMap<TagId, RegisteredInstrument>,
    pids: BTreeMap<TagId, RegisteredPid>,
    actuators: BTreeMap<EquipmentId, RegisteredActuator>,
    protections: BTreeMap<String, RegisteredProtection>,
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
