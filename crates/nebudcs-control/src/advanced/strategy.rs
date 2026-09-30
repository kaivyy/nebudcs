//! Composable advanced process control strategies.
//!
//! Provides [`AdvancedControlStrategy`] unifying:
//! - Cascade control
//! - Ratio control
//! - Feedback + Feed-forward disturbance rejection
//! - Override / Selective constraint control
//! - Split-range multi-valve control
//! - Dynamic compressor anti-surge control
//! - Valve characterization

use nebudcs_model::tag::{TagQuality, TagValue};
use nebudcs_types::{EquipmentId, TagId};
use serde::{Deserialize, Serialize};

use crate::actuator::{ActuatorCommand, CommandPriority};
use crate::advanced::anti_surge::AntiSurgeController;
use crate::advanced::filter::LeadLagCompensator;
use crate::advanced::ratio::RatioController;
use crate::advanced::selector::OverrideSelector;
use crate::advanced::split_range::SplitRangeController;
use crate::advanced::valve_char::ValveCharacterizer;
use crate::pid::PidController;

/// The concrete strategy configuration and dynamic runtime blocks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StrategyKind {
    /// Cascade control loop: primary master controller drives secondary slave setpoint.
    Cascade {
        /// Primary process variable (e.g. Vessel Liquid Level).
        primary_pv: TagId,
        /// Primary working setpoint.
        primary_sp: f64,
        /// Primary master PID controller.
        primary_pid: PidController,
        /// Secondary process variable (e.g. Discharge Flow Rate).
        secondary_pv: TagId,
        /// Secondary slave PID controller.
        secondary_pid: PidController,
        /// Output tag publishing the secondary output command.
        output_tag: Option<TagId>,
        /// Target actuator driven by the secondary output command.
        target_actuator: Option<EquipmentId>,
    },

    /// Ratio control loop: regulates a controlled stream in fixed proportion to a wild stream.
    Ratio {
        /// Wild (uncontrolled) stream process variable tag.
        wild_pv: TagId,
        /// Controlled stream process variable tag.
        controlled_pv: TagId,
        /// Ratio calculation block.
        ratio_controller: RatioController,
        /// Regulatory PID controller driving the controlled stream.
        pid: PidController,
        /// Output tag publishing the valve command.
        output_tag: Option<TagId>,
        /// Target actuator driven by the valve command.
        target_actuator: Option<EquipmentId>,
    },

    /// Feedback + feed-forward disturbance rejection control.
    FeedForward {
        /// Regulated process variable tag (e.g. Heater Outlet Temperature).
        pv_tag: TagId,
        /// Target temperature setpoint.
        sp: f64,
        /// Feedback PID controller.
        pid: PidController,
        /// Measured disturbance tag (e.g. Feed Inlet Flow Rate).
        disturbance_tag: TagId,
        /// Lead-lag dynamic compensator.
        lead_lag: LeadLagCompensator,
        /// Output tag publishing the fuel/duty demand.
        output_tag: Option<TagId>,
        /// Target actuator driven by the command.
        target_actuator: Option<EquipmentId>,
    },

    /// Override / Selective control: arbitrates between primary control and constraint safety limit.
    Override {
        /// Primary process variable tag (e.g. Delivery Flow).
        primary_pv: TagId,
        /// Primary setpoint.
        primary_sp: f64,
        /// Primary PID controller.
        primary_pid: PidController,
        /// Constraint process variable tag (e.g. Suction Pressure).
        constraint_pv: TagId,
        /// Constraint threshold limit.
        constraint_limit: f64,
        /// Constraint PID controller.
        constraint_pid: PidController,
        /// Override selector (High or Low selection) with external reset feedback.
        selector: OverrideSelector,
        /// Output tag publishing the arbitrated command.
        output_tag: Option<TagId>,
        /// Target actuator driven by the arbitrated command.
        target_actuator: Option<EquipmentId>,
    },

    /// Split-range control: splits a master demand across two complementary actuators.
    SplitRange {
        /// Process variable tag.
        pv_tag: TagId,
        /// Working setpoint.
        sp: f64,
        /// Master PID controller.
        master_pid: PidController,
        /// Split range characterizer.
        split_range: SplitRangeController,
        /// Output tag for Split A (e.g. Heating valve).
        split_a_output_tag: Option<TagId>,
        /// Target actuator for Split A.
        split_a_target_actuator: Option<EquipmentId>,
        /// Output tag for Split B (e.g. Cooling valve).
        split_b_output_tag: Option<TagId>,
        /// Target actuator for Split B.
        split_b_target_actuator: Option<EquipmentId>,
    },

    /// Compressor anti-surge control loop.
    AntiSurge {
        /// Measured compressor flow tag.
        flow_tag: TagId,
        /// Suction pressure tag.
        p_suction_tag: TagId,
        /// Discharge pressure tag.
        p_discharge_tag: TagId,
        /// Anti-surge controller block.
        controller: AntiSurgeController,
        /// Output tag for recycle valve opening demand.
        recycle_valve_tag: Option<TagId>,
        /// Target actuator for recycle valve.
        target_actuator: Option<EquipmentId>,
    },

    /// Control valve trim characterization block.
    ValveCharacterization {
        /// Nominal stem demand input tag ($0\text{--}100\%$).
        input_tag: TagId,
        /// Valve characterizer (Linear, Equal-Percentage, Quick-Opening).
        characterizer: ValveCharacterizer,
        /// Effective characterized output tag ($0\text{--}100\%$).
        output_tag: Option<TagId>,
        /// Target actuator driven by characterized demand.
        target_actuator: Option<EquipmentId>,
    },
}

/// An active advanced control strategy deployed in the control system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdvancedControlStrategy {
    /// Strategy unique identifier.
    pub id: String,
    /// Human-readable strategy name.
    pub name: String,
    /// Description of control objective.
    pub description: String,
    /// Whether this strategy is currently enabled.
    pub enabled: bool,
    /// Strategy configuration and execution engine.
    pub kind: StrategyKind,
}

impl AdvancedControlStrategy {
    /// Creates a new advanced control strategy.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        kind: StrategyKind,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            enabled: true,
            kind,
        }
    }

    /// Read float helper from current state.
    fn read_tag_val(
        tag_id: TagId,
        read_state: &nebudcs_sim::StateReadView<'_>,
        write_state: &nebudcs_sim::StateWriteView<'_>,
    ) -> (f64, TagQuality) {
        match write_state.get_tag(tag_id) {
            Some(TagValue::Float(v)) => {
                let q = write_state.get_quality(tag_id).unwrap_or(TagQuality::Good);
                (*v, q)
            }
            _ => match read_state.tag_value(tag_id) {
                Ok(TagValue::Float(v)) => {
                    let q = read_state.tag_quality(tag_id).unwrap_or(TagQuality::Good);
                    (*v, q)
                }
                _ => (0.0, TagQuality::Bad),
            },
        }
    }

    /// Executes one deterministic calculation step for this advanced strategy.
    pub(crate) fn step(
        &mut self,
        dt: f64,
        read_state: &nebudcs_sim::StateReadView<'_>,
        write_state: &mut nebudcs_sim::StateWriteView<'_>,
        actuators: &mut std::collections::BTreeMap<EquipmentId, crate::system::RegisteredActuator>,
    ) {
        if !self.enabled || dt <= 0.0 {
            return;
        }

        let strategy_name = self.name.clone();

        match &mut self.kind {
            StrategyKind::Cascade {
                primary_pv,
                primary_sp,
                primary_pid,
                secondary_pv,
                secondary_pid,
                output_tag,
                target_actuator,
            } => {
                let (pv1, q1) = Self::read_tag_val(*primary_pv, read_state, write_state);
                let (pv2, q2) = Self::read_tag_val(*secondary_pv, read_state, write_state);

                primary_pid.set_quality(q1);
                primary_pid.set_sp(*primary_sp);
                let primary_out = primary_pid.step(dt, pv1);

                secondary_pid.set_quality(q2);
                secondary_pid.set_cascade_sp(primary_out);
                let sec_out = secondary_pid.step(dt, pv2);

                if let Some(out_t) = output_tag {
                    write_state.set_tag(*out_t, TagValue::Float(sec_out), TagQuality::Good);
                }
                if let Some(act_id) = target_actuator {
                    if let Some(act) = actuators.get_mut(act_id) {
                        act.arbiter.submit(
                            ActuatorCommand::new(CommandPriority::Auto, sec_out)
                                .with_source(strategy_name.as_str()),
                        );
                    }
                }
            }

            StrategyKind::Ratio {
                wild_pv,
                controlled_pv,
                ratio_controller,
                pid,
                output_tag,
                target_actuator,
            } => {
                let (pv_wild, _) = Self::read_tag_val(*wild_pv, read_state, write_state);
                let (pv_ctrl, q_ctrl) = Self::read_tag_val(*controlled_pv, read_state, write_state);

                let target_sp = ratio_controller.calculate(pv_wild);
                pid.set_quality(q_ctrl);
                pid.set_sp(target_sp);
                let out = pid.step(dt, pv_ctrl);

                if let Some(out_t) = output_tag {
                    write_state.set_tag(*out_t, TagValue::Float(out), TagQuality::Good);
                }
                if let Some(act_id) = target_actuator {
                    if let Some(act) = actuators.get_mut(act_id) {
                        act.arbiter.submit(
                            ActuatorCommand::new(CommandPriority::Auto, out)
                                .with_source(strategy_name.as_str()),
                        );
                    }
                }
            }

            StrategyKind::FeedForward {
                pv_tag,
                sp,
                pid,
                disturbance_tag,
                lead_lag,
                output_tag,
                target_actuator,
            } => {
                let (pv, q_pv) = Self::read_tag_val(*pv_tag, read_state, write_state);
                let (dist, _) = Self::read_tag_val(*disturbance_tag, read_state, write_state);

                let ff_trim = lead_lag.step(dt, dist);
                pid.set_quality(q_pv);
                pid.set_sp(*sp);
                let out = pid.step_with_feedforward(dt, pv, ff_trim);

                if let Some(out_t) = output_tag {
                    write_state.set_tag(*out_t, TagValue::Float(out), TagQuality::Good);
                }
                if let Some(act_id) = target_actuator {
                    if let Some(act) = actuators.get_mut(act_id) {
                        act.arbiter.submit(
                            ActuatorCommand::new(CommandPriority::Auto, out)
                                .with_source(strategy_name.as_str()),
                        );
                    }
                }
            }

            StrategyKind::Override {
                primary_pv,
                primary_sp,
                primary_pid,
                constraint_pv,
                constraint_limit,
                constraint_pid,
                selector,
                output_tag,
                target_actuator,
            } => {
                let (pv_p, q_p) = Self::read_tag_val(*primary_pv, read_state, write_state);
                let (pv_c, q_c) = Self::read_tag_val(*constraint_pv, read_state, write_state);

                primary_pid.set_quality(q_p);
                primary_pid.set_sp(*primary_sp);
                let u_prim = primary_pid.step(dt, pv_p);

                constraint_pid.set_quality(q_c);
                constraint_pid.set_sp(*constraint_limit);
                let u_ovr = constraint_pid.step(dt, pv_c);

                let winning_out = selector.select(u_prim, u_ovr);

                // Anti-windup external reset feedback
                let fb = selector.external_reset_feedback();
                primary_pid.apply_external_reset_feedback(fb);
                constraint_pid.apply_external_reset_feedback(fb);

                if let Some(out_t) = output_tag {
                    write_state.set_tag(*out_t, TagValue::Float(winning_out), TagQuality::Good);
                }
                if let Some(act_id) = target_actuator {
                    if let Some(act) = actuators.get_mut(act_id) {
                        act.arbiter.submit(
                            ActuatorCommand::new(CommandPriority::Auto, winning_out)
                                .with_source(strategy_name.as_str()),
                        );
                    }
                }
            }

            StrategyKind::SplitRange {
                pv_tag,
                sp,
                master_pid,
                split_range,
                split_a_output_tag,
                split_a_target_actuator,
                split_b_output_tag,
                split_b_target_actuator,
            } => {
                let (pv, q_pv) = Self::read_tag_val(*pv_tag, read_state, write_state);

                master_pid.set_quality(q_pv);
                master_pid.set_sp(*sp);
                let master_demand = master_pid.step(dt, pv);

                let splits = split_range.evaluate_all(master_demand);
                let val_a = splits.first().map(|(_, v)| *v).unwrap_or(0.0);
                let val_b = splits.get(1).map(|(_, v)| *v).unwrap_or(0.0);

                if let Some(out_a) = split_a_output_tag {
                    write_state.set_tag(*out_a, TagValue::Float(val_a), TagQuality::Good);
                }
                if let Some(act_a) = split_a_target_actuator {
                    if let Some(act) = actuators.get_mut(act_a) {
                        act.arbiter.submit(
                            ActuatorCommand::new(CommandPriority::Auto, val_a)
                                .with_source(strategy_name.as_str()),
                        );
                    }
                }

                if let Some(out_b) = split_b_output_tag {
                    write_state.set_tag(*out_b, TagValue::Float(val_b), TagQuality::Good);
                }
                if let Some(act_b) = split_b_target_actuator {
                    if let Some(act) = actuators.get_mut(act_b) {
                        act.arbiter.submit(
                            ActuatorCommand::new(CommandPriority::Auto, val_b)
                                .with_source(strategy_name.as_str()),
                        );
                    }
                }
            }

            StrategyKind::AntiSurge {
                flow_tag,
                p_suction_tag,
                p_discharge_tag,
                controller,
                recycle_valve_tag,
                target_actuator,
            } => {
                let (flow, _) = Self::read_tag_val(*flow_tag, read_state, write_state);
                let (p_suc, _) = Self::read_tag_val(*p_suction_tag, read_state, write_state);
                let (p_dis, _) = Self::read_tag_val(*p_discharge_tag, read_state, write_state);

                let valve_demand = controller.step(dt, flow, p_suc, p_dis);

                if let Some(out_t) = recycle_valve_tag {
                    write_state.set_tag(*out_t, TagValue::Float(valve_demand), TagQuality::Good);
                }
                if let Some(act_id) = target_actuator {
                    if let Some(act) = actuators.get_mut(act_id) {
                        act.arbiter.submit(
                            ActuatorCommand::new(CommandPriority::Auto, valve_demand)
                                .with_source(strategy_name.as_str()),
                        );
                    }
                }
            }

            StrategyKind::ValveCharacterization {
                input_tag,
                characterizer,
                output_tag,
                target_actuator,
            } => {
                let (stem_lift, _) = Self::read_tag_val(*input_tag, read_state, write_state);
                // Convert 0-100% to fraction 0-1, evaluate, convert back to 0-100%
                let fraction = (stem_lift / 100.0).clamp(0.0, 1.0);
                let cv_fraction = characterizer.evaluate(fraction);
                let characterized_demand = cv_fraction * 100.0;

                if let Some(out_t) = output_tag {
                    write_state.set_tag(
                        *out_t,
                        TagValue::Float(characterized_demand),
                        TagQuality::Good,
                    );
                }
                if let Some(act_id) = target_actuator {
                    if let Some(act) = actuators.get_mut(act_id) {
                        act.arbiter.submit(
                            ActuatorCommand::new(CommandPriority::Auto, characterized_demand)
                                .with_source(strategy_name.as_str()),
                        );
                    }
                }
            }
        }
    }
}
