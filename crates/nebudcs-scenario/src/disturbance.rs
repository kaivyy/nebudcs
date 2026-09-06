//! Dynamic process disturbances manager.

use std::collections::HashMap;

use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_model::tag::{TagQuality, TagValue};

use crate::model::{DisturbanceDefinition, DisturbanceType};
use crate::random::DeterministicPrng;

/// Active disturbance in progress.
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveDisturbance {
    pub definition: DisturbanceDefinition,
    pub activated_at_sim_sec: f64,
    pub elapsed_sec: f64,
    pub baseline_value: f64,
}

impl ActiveDisturbance {
    pub fn is_expired(&self) -> bool {
        if let Some(dur) = self.definition.duration_sec {
            self.elapsed_sec >= dur
        } else {
            match &self.definition.disturbance_type {
                DisturbanceType::Ramp { duration_sec, .. } => self.elapsed_sec >= *duration_sec,
                DisturbanceType::Pulse {
                    pulse_width_sec, ..
                } => self.elapsed_sec >= *pulse_width_sec,
                _ => false,
            }
        }
    }
}

/// Manages and executes deterministic process disturbances.
#[derive(Debug, Clone)]
pub struct DisturbanceManager {
    active_disturbances: HashMap<String, ActiveDisturbance>,
    prng: DeterministicPrng,
}

impl DisturbanceManager {
    pub fn new(seed: u64) -> Self {
        Self {
            active_disturbances: HashMap::new(),
            prng: DeterministicPrng::new(seed),
        }
    }

    /// Registers a new active disturbance.
    pub fn apply_disturbance(
        &mut self,
        dist: DisturbanceDefinition,
        current_sim_time: f64,
        tag_store: &RuntimeTagStore,
    ) {
        let baseline = if let Some(tag_id) = tag_store.resolve_name(&dist.target_tag) {
            tag_store
                .get_value(tag_id)
                .and_then(|v| match v {
                    TagValue::Float(f) => Some(*f),
                    TagValue::Integer(i) => Some(*i as f64),
                    _ => None,
                })
                .unwrap_or(0.0)
        } else {
            0.0
        };

        let active = ActiveDisturbance {
            definition: dist.clone(),
            activated_at_sim_sec: current_sim_time,
            elapsed_sec: 0.0,
            baseline_value: baseline,
        };

        self.active_disturbances.insert(dist.id, active);
    }

    /// Advances disturbances and applies value modifications to target tags.
    pub fn step(&mut self, dt: f64, tag_store: &mut RuntimeTagStore) -> Vec<String> {
        // 1. Advance elapsed times
        for dist in self.active_disturbances.values_mut() {
            dist.elapsed_sec += dt;
        }

        // 2. Apply active waveforms (including final boundary at expiration)
        for dist in self.active_disturbances.values() {
            let Some(tag_id) = tag_store.resolve_name(&dist.definition.target_tag) else {
                continue;
            };

            let new_val = match &dist.definition.disturbance_type {
                DisturbanceType::Step { value, relative } => {
                    if *relative {
                        dist.baseline_value + *value
                    } else {
                        *value
                    }
                }
                DisturbanceType::Ramp {
                    target_value,
                    duration_sec,
                } => {
                    let progress = (dist.elapsed_sec / duration_sec).clamp(0.0, 1.0);
                    dist.baseline_value + (*target_value - dist.baseline_value) * progress
                }
                DisturbanceType::Pulse {
                    value,
                    pulse_width_sec,
                } => {
                    if dist.elapsed_sec <= *pulse_width_sec {
                        dist.baseline_value + *value
                    } else {
                        dist.baseline_value
                    }
                }
                DisturbanceType::Sinusoidal {
                    amplitude,
                    frequency_hz,
                    offset,
                } => {
                    let t = dist.elapsed_sec;
                    let angle = 2.0 * std::f64::consts::PI * frequency_hz * t;
                    offset + amplitude * angle.sin()
                }
                DisturbanceType::BoundedNoise { max_amplitude } => {
                    let noise = self.prng.next_bipolar() * max_amplitude;
                    dist.baseline_value + noise
                }
            };

            tag_store.set_tag_float(tag_id, new_val, TagQuality::Good);
        }

        // 3. Collect and remove expired disturbances
        let mut expired = Vec::new();
        for (id, dist) in &self.active_disturbances {
            if dist.is_expired() {
                expired.push(id.clone());
            }
        }
        for id in &expired {
            self.active_disturbances.remove(id);
        }

        expired
    }

    /// Resets all active disturbances.
    pub fn reset(&mut self, seed: u64) {
        self.active_disturbances.clear();
        self.prng.reset(seed);
    }

    pub fn len(&self) -> usize {
        self.active_disturbances.len()
    }

    pub fn is_empty(&self) -> bool {
        self.active_disturbances.is_empty()
    }
}
