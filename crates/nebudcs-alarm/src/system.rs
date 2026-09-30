//! Alarm simulation system implementing [`nebudcs_sim::SimulationSystem`].
//!
//! # Non-Interference Invariant
//!
//! Alarm evaluation is strictly observational. Alarm state transitions never mutate
//! physical equipment variables or valve positions.

use std::collections::BTreeSet;

use nebudcs_model::tag::TagQuality;
use nebudcs_sim::{SimError, SimulationSystem, StateReadView, StateWriteView, StepContext};
use nebudcs_types::TagId;

use crate::block::AlarmBlockConfig;
use crate::engine::AlarmEngine;

/// Simulation system executing alarm detection and lifecycle updates per tick.
#[derive(Debug, Default)]
pub struct AlarmSystem {
    engine: AlarmEngine,
    monitored_tags: BTreeSet<TagId>,
}

impl AlarmSystem {
    /// Creates an empty alarm simulation system.
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears all registered alarms and monitored tags.
    pub fn clear(&mut self) {
        self.engine.clear();
        self.monitored_tags.clear();
    }

    /// Registers an alarm block and tracks its monitored tag.
    pub fn add_alarm(&mut self, config: AlarmBlockConfig) {
        self.monitored_tags.insert(config.tag_id);
        self.engine.add_alarm(config);
    }

    /// Access to internal alarm engine.
    pub fn engine(&self) -> &AlarmEngine {
        &self.engine
    }

    /// Mutable access to internal alarm engine (e.g. for operator acknowledgement or shelving).
    pub fn engine_mut(&mut self) -> &mut AlarmEngine {
        &mut self.engine
    }
}

impl SimulationSystem for AlarmSystem {
    fn name(&self) -> &str {
        "AlarmSystem"
    }

    fn step(
        &mut self,
        ctx: StepContext,
        read_state: &StateReadView<'_>,
        write_state: &mut StateWriteView<'_>,
    ) -> Result<(), SimError> {
        let dt = ctx.dt;

        // Iterate through all monitored tags in deterministic binary TagId order
        for tag_id in &self.monitored_tags {
            // Read tag from write_state (if updated earlier in the tick by instruments)
            // or fallback to committed read_state
            let (pv, quality) = if let Some(val) = write_state.get_tag(*tag_id) {
                let qual = write_state.get_quality(*tag_id).unwrap_or(TagQuality::Good);
                match val {
                    nebudcs_model::tag::TagValue::Float(v) => (*v, qual),
                    _ => (0.0, qual),
                }
            } else if let Ok(val) = read_state.tag_value(*tag_id) {
                let qual = read_state.tag_quality(*tag_id).unwrap_or(TagQuality::Good);
                match val {
                    nebudcs_model::tag::TagValue::Float(v) => (*v, qual),
                    _ => (0.0, qual),
                }
            } else {
                continue;
            };

            self.engine.evaluate_tag(*tag_id, pv, quality, dt);
        }

        Ok(())
    }
}
