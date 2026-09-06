//! Fault injection and active fault lifecycle manager.

use std::collections::HashMap;

use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_model::tag::{TagQuality, TagValue};

use crate::error::ScenarioResult;
use crate::model::{FaultDefinition, FaultType};
use crate::random::DeterministicPrng;

/// Represents an active fault currently influencing simulation state.
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveFault {
    pub definition: FaultDefinition,
    pub activated_at_sim_sec: f64,
    pub elapsed_active_sec: f64,
    pub initial_baseline: Option<f64>,
    pub frozen_baseline: Option<f64>,
}

impl ActiveFault {
    pub fn new(
        definition: FaultDefinition,
        current_sim_time: f64,
        initial_val: Option<f64>,
    ) -> Self {
        let frozen_baseline = match &definition.fault_type {
            FaultType::SensorFrozen { value } => value.or(initial_val),
            _ => None,
        };
        Self {
            definition,
            activated_at_sim_sec: current_sim_time,
            elapsed_active_sec: 0.0,
            initial_baseline: initial_val,
            frozen_baseline,
        }
    }

    /// Checks whether this fault has exceeded its configured duration.
    pub fn is_expired(&self) -> bool {
        if let Some(dur) = self.definition.duration_sec {
            self.elapsed_active_sec >= dur
        } else {
            false
        }
    }
}

/// Summary DTO representing an active fault for API and UI status reports.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ActiveFaultSummary {
    pub id: String,
    pub target: String,
    pub category: String,
    pub elapsed_sec: f64,
    pub duration_sec: Option<f64>,
}

/// Manages all active faults, evaluating durations and modifying runtime state deterministically.
#[derive(Debug, Clone)]
pub struct FaultManager {
    active_faults: HashMap<String, ActiveFault>,
    prng: DeterministicPrng,
}

impl FaultManager {
    /// Creates a new fault manager with a deterministic seed for noise degradation.
    pub fn new(seed: u64) -> Self {
        Self {
            active_faults: HashMap::new(),
            prng: DeterministicPrng::new(seed),
        }
    }

    /// Injects a new fault into the active fault table.
    pub fn inject_fault(
        &mut self,
        fault: FaultDefinition,
        current_sim_time: f64,
        tag_store: &RuntimeTagStore,
    ) -> ScenarioResult<()> {
        let baseline = if let Some(tag_id) = tag_store.resolve_name(&fault.target) {
            let existing_baseline = self
                .active_faults
                .values()
                .find(|f| f.definition.target == fault.target)
                .and_then(|f| f.initial_baseline);

            existing_baseline.or_else(|| {
                tag_store.get_value(tag_id).and_then(|v| match v {
                    TagValue::Float(f) => Some(*f),
                    TagValue::Integer(i) => Some(*i as f64),
                    _ => None,
                })
            })
        } else {
            None
        };

        let active = ActiveFault::new(fault.clone(), current_sim_time, baseline);
        self.active_faults.insert(fault.id, active);
        Ok(())
    }

    /// Manually clears an active fault by ID.
    pub fn clear_fault(&mut self, fault_id: &str, tag_store: &mut RuntimeTagStore) -> bool {
        if let Some(removed) = self.active_faults.remove(fault_id) {
            let target = removed.definition.target.clone();
            let has_other = self
                .active_faults
                .values()
                .any(|f| f.definition.target == target);

            if !has_other {
                self.restore_clean_state(&removed, tag_store);
            } else {
                self.apply_active_faults_for_target(&target, tag_store);
            }
            true
        } else {
            false
        }
    }

    /// Advances simulation time for all active faults and applies their physical effects.
    ///
    /// Returns a list of IDs of faults that expired and were automatically cleared during this step.
    pub fn step(&mut self, dt: f64, tag_store: &mut RuntimeTagStore) -> Vec<String> {
        let mut expired = Vec::new();

        // 1. Advance elapsed durations and check expiry
        for (id, fault) in &mut self.active_faults {
            fault.elapsed_active_sec += dt;
            if fault.is_expired() {
                expired.push(id.clone());
            }
        }

        // 2. Remove expired faults and restore clean state if no remaining faults for target
        for id in &expired {
            if let Some(f) = self.active_faults.remove(id) {
                let target = f.definition.target.clone();
                let has_other = self
                    .active_faults
                    .values()
                    .any(|other| other.definition.target == target);
                if !has_other {
                    self.restore_clean_state(&f, tag_store);
                }
            }
        }

        // 3. Collect distinct targets of all active faults deterministically
        let mut distinct_targets: Vec<String> = self
            .active_faults
            .values()
            .map(|f| f.definition.target.clone())
            .collect();
        distinct_targets.sort();
        distinct_targets.dedup();

        // 4. Apply combined active fault perturbations per target
        for target in distinct_targets {
            self.apply_active_faults_for_target(&target, tag_store);
        }

        expired
    }

    fn apply_active_faults_for_target(&mut self, target: &str, tag_store: &mut RuntimeTagStore) {
        let Some(tag_id) = tag_store.resolve_name(target) else {
            return;
        };

        let mut matching_faults: Vec<ActiveFault> = self
            .active_faults
            .values()
            .filter(|f| f.definition.target == target)
            .cloned()
            .collect();

        if matching_faults.is_empty() {
            return;
        }

        matching_faults.sort_by(|a, b| a.definition.id.cmp(&b.definition.id));

        let baseline = matching_faults
            .first()
            .and_then(|f| f.initial_baseline)
            .or_else(|| {
                tag_store.get_value(tag_id).and_then(|v| match v {
                    TagValue::Float(f) => Some(*f),
                    TagValue::Integer(i) => Some(*i as f64),
                    _ => None,
                })
            });

        let mut val = baseline.unwrap_or(0.0);
        let mut quality = TagQuality::Good;
        let mut forced_override: Option<f64> = None;
        let mut forced_bad_quality = false;
        let mut trip_status: Option<i64> = None;

        for fault in &matching_faults {
            match &fault.definition.fault_type {
                FaultType::SensorBias { offset } => {
                    val += *offset;
                }
                FaultType::SensorDrift { rate_per_sec } => {
                    val += rate_per_sec * fault.elapsed_active_sec;
                }
                FaultType::SensorFrozen { value } => {
                    let frozen_val = value
                        .or(fault.frozen_baseline)
                        .or(fault.initial_baseline)
                        .unwrap_or(val);
                    forced_override = Some(frozen_val);
                }
                FaultType::SensorLossOfSignal => {
                    forced_bad_quality = true;
                }
                FaultType::SensorDegradation { noise_amplitude } => {
                    let noise = self.prng.next_bipolar() * noise_amplitude;
                    val += noise;
                    if quality == TagQuality::Good {
                        quality = TagQuality::Uncertain;
                    }
                }
                FaultType::ActuatorStuck { position } => {
                    forced_override = Some(*position);
                }
                FaultType::ActuatorSluggish { .. } => {}
                FaultType::ActuatorFailSafeTrip => {
                    forced_override = Some(0.0);
                }
                FaultType::EquipmentMechanicalTrip => {
                    forced_override = Some(0.0);
                    trip_status = Some(0);
                }
                FaultType::EquipmentEfficiencyLoss { factor } => {
                    val *= 1.0 - factor;
                }
                FaultType::ProcessBlockage {
                    resistance_multiplier,
                } => {
                    val /= resistance_multiplier;
                }
                FaultType::ProcessLeak { leak_rate_kg_s } => {
                    val = (val - leak_rate_kg_s).max(0.0);
                }
            }
        }

        if forced_bad_quality {
            tag_store.set_tag(tag_id, TagValue::BadQuality, TagQuality::Bad);
        } else {
            let final_val = forced_override.unwrap_or(val);
            tag_store.set_tag_float(tag_id, final_val, quality);
        }

        if let Some(st) = trip_status {
            let status_tag_name = format!("{target}.STATUS");
            if let Some(st_id) = tag_store.resolve_name(&status_tag_name) {
                tag_store.set_tag(st_id, TagValue::Integer(st), TagQuality::Good);
            }
        }
    }

    fn restore_clean_state(&self, fault: &ActiveFault, tag_store: &mut RuntimeTagStore) {
        if let Some(tag_id) = tag_store.resolve_name(&fault.definition.target) {
            if let Some(base) = fault.initial_baseline {
                tag_store.set_tag_float(tag_id, base, TagQuality::Good);
            } else if let Some(curr) = tag_store.get_value(tag_id).cloned() {
                let clean_val = match curr {
                    TagValue::BadQuality => TagValue::Float(0.0),
                    other => other,
                };
                tag_store.set_tag(tag_id, clean_val, TagQuality::Good);
            }

            // Restore companion status tag for equipment trips
            if matches!(
                &fault.definition.fault_type,
                FaultType::EquipmentMechanicalTrip
            ) {
                let status_tag_name = format!("{}.STATUS", fault.definition.target);
                if let Some(st_id) = tag_store.resolve_name(&status_tag_name) {
                    tag_store.set_tag(st_id, TagValue::Integer(1), TagQuality::Good);
                }
            }
        }
    }

    /// Resets all active faults and reinitializes PRNG seed.
    pub fn reset(&mut self, seed: u64, tag_store: &mut RuntimeTagStore) {
        let mut distinct_targets: Vec<String> = self
            .active_faults
            .values()
            .map(|f| f.definition.target.clone())
            .collect();
        distinct_targets.sort();
        distinct_targets.dedup();

        for target in distinct_targets {
            if let Some(fault) = self
                .active_faults
                .values()
                .find(|f| f.definition.target == target)
            {
                self.restore_clean_state(fault, tag_store);
            }
        }
        self.active_faults.clear();
        self.prng.reset(seed);
    }

    /// Returns a list of summaries of all currently active faults.
    pub fn summaries(&self) -> Vec<ActiveFaultSummary> {
        let mut summaries: Vec<_> = self
            .active_faults
            .values()
            .map(|f| {
                let category = match &f.definition.fault_type {
                    FaultType::SensorBias { .. } => "sensor_bias",
                    FaultType::SensorDrift { .. } => "sensor_drift",
                    FaultType::SensorFrozen { .. } => "sensor_frozen",
                    FaultType::SensorLossOfSignal => "sensor_loss_of_signal",
                    FaultType::SensorDegradation { .. } => "sensor_degradation",
                    FaultType::ActuatorStuck { .. } => "actuator_stuck",
                    FaultType::ActuatorSluggish { .. } => "actuator_sluggish",
                    FaultType::ActuatorFailSafeTrip => "actuator_trip",
                    FaultType::EquipmentMechanicalTrip => "equipment_trip",
                    FaultType::EquipmentEfficiencyLoss { .. } => "efficiency_loss",
                    FaultType::ProcessBlockage { .. } => "process_blockage",
                    FaultType::ProcessLeak { .. } => "process_leak",
                }
                .to_string();

                ActiveFaultSummary {
                    id: f.definition.id.clone(),
                    target: f.definition.target.clone(),
                    category,
                    elapsed_sec: f.elapsed_active_sec,
                    duration_sec: f.definition.duration_sec,
                }
            })
            .collect();

        summaries.sort_by(|a, b| a.id.cmp(&b.id));
        summaries
    }

    pub fn len(&self) -> usize {
        self.active_faults.len()
    }

    pub fn is_empty(&self) -> bool {
        self.active_faults.is_empty()
    }
}
