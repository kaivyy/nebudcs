//! Central alarm engine managing alarms, severity hierarchies, and evaluation.

use std::collections::BTreeMap;

use nebudcs_model::tag::TagQuality;
use nebudcs_types::TagId;
use serde::{Deserialize, Serialize};

use crate::block::{AlarmBlock, AlarmBlockConfig};
use crate::types::AlarmState;

/// Central coordinator for all process alarm blocks.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AlarmEngine {
    alarms: BTreeMap<String, AlarmBlock>,
}

impl AlarmEngine {
    /// Creates an empty alarm engine.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new alarm block.
    pub fn add_alarm(&mut self, config: AlarmBlockConfig) {
        let name = config.name.clone();
        self.alarms.insert(name, AlarmBlock::new(config));
    }

    /// Access to an alarm block by name.
    pub fn get_alarm(&self, name: &str) -> Option<&AlarmBlock> {
        self.alarms.get(name)
    }

    /// Mutable access to an alarm block by name.
    pub fn get_alarm_mut(&mut self, name: &str) -> Option<&mut AlarmBlock> {
        self.alarms.get_mut(name)
    }

    /// Acknowledges an alarm by name.
    pub fn acknowledge_alarm(&mut self, name: &str) {
        if let Some(alarm) = self.alarms.get_mut(name) {
            alarm.acknowledge();
        }
    }

    /// Shelves an alarm by name for a given duration in seconds.
    pub fn shelve_alarm(&mut self, name: &str, duration_seconds: f64) {
        if let Some(alarm) = self.alarms.get_mut(name) {
            alarm.shelve(duration_seconds);
        }
    }

    /// Evaluates all alarms bound to a specific process tag.
    ///
    /// Evaluates in order of alarm severity descending (Critical first).
    /// Ensures that if a lower-tier alarm is shelved and the process escalates across
    /// a higher-tier threshold (e.g. HIHI), the higher-tier alarm activates immediately as ActiveUnack.
    pub fn evaluate_tag(&mut self, tag_id: TagId, pv: f64, quality: TagQuality, dt: f64) {
        for block in self.alarms.values_mut() {
            if block.config().tag_id == tag_id {
                block.step(pv, quality, dt);
            }
        }
    }

    /// Returns the count of currently active alarms.
    pub fn active_count(&self) -> usize {
        self.alarms.values().filter(|b| b.is_active()).count()
    }

    /// Returns the count of currently unacknowledged alarms.
    pub fn unack_count(&self) -> usize {
        self.alarms.values().filter(|b| b.is_unack()).count()
    }

    /// Iterates over all alarms in deterministic name order.
    pub fn alarms(&self) -> impl Iterator<Item = (&String, &AlarmBlock)> {
        self.alarms.iter()
    }

    /// Returns true if any alarm is in the unacknowledged active state.
    pub fn has_unacknowledged(&self) -> bool {
        self.alarms
            .values()
            .any(|b| b.state() == AlarmState::ActiveUnack)
    }
}
