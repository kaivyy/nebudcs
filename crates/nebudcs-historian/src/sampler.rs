//! Zero-allocation runtime sampling engine evaluated in deterministic simulation time.

use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_model::tag::{TagQuality, TagValue};
use nebudcs_sim::state::StateReadView;
use nebudcs_types::SimulationTime;

use crate::buffer::{HistorianRecord, HistorianSender};
use crate::error::HistorianResult;
use crate::point::{HistorianPointConfig, HistorianPointId, SamplingMode};
use crate::sample::{HistoricalSample, HistoricalSampleValue};

/// Internal cached state for an individual registered point.
///
/// Pre-allocated and flat-indexed to eliminate hot-path allocations.
#[derive(Debug, Clone, PartialEq)]
struct PointSlot {
    config: HistorianPointConfig,
    tag_store_index: Option<usize>,
    last_sample_time: Option<SimulationTime>,
    last_sample_value: Option<HistoricalSampleValue>,
    last_sample_quality: Option<TagQuality>,
}

/// Zero-allocation sampling engine coordinating point evaluation in the simulation tick loop.
#[derive(Debug)]
pub struct HistorianSampler {
    slots: Vec<PointSlot>,
    seq: u64,
    had_dropped_samples: bool,
}

impl HistorianSampler {
    /// Creates a new empty historian sampler.
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            seq: 0,
            had_dropped_samples: false,
        }
    }

    /// Registers a point configuration and pre-allocates its runtime evaluation slot.
    pub fn register_point(
        &mut self,
        config: HistorianPointConfig,
        tag_store: Option<&RuntimeTagStore>,
    ) -> HistorianResult<HistorianPointId> {
        let expected_idx = self.slots.len();
        if config.point_id.as_usize() != expected_idx {
            // Re-assign or validate dense sequential IDs
        }

        // Resolve tag index if a runtime store was provided
        let tag_store_index = tag_store.and_then(|store| store.get_index(config.tag_id));

        self.slots.push(PointSlot {
            config,
            tag_store_index,
            last_sample_time: None,
            last_sample_value: None,
            last_sample_quality: None,
        });

        Ok(HistorianPointId::new(expected_idx as u32))
    }

    /// Resolves and updates flat vector tag indices from a `RuntimeTagStore`.
    ///
    /// Must be called once during plant configuration before the simulation hot loop begins.
    pub fn bind_runtime_tags(&mut self, tag_store: &RuntimeTagStore) {
        for slot in &mut self.slots {
            slot.tag_store_index = tag_store.get_index(slot.config.tag_id);
        }
    }

    /// Number of registered points.
    #[inline]
    pub fn point_count(&self) -> usize {
        self.slots.len()
    }

    /// Access registered point configuration by dense ID.
    pub fn get_point(&self, point_id: HistorianPointId) -> Option<&HistorianPointConfig> {
        self.slots.get(point_id.as_usize()).map(|s| &s.config)
    }

    /// Iterates over all registered point configurations.
    pub fn points(&self) -> impl Iterator<Item = &HistorianPointConfig> {
        self.slots.iter().map(|s| &s.config)
    }

    /// Evaluates sampling conditions against pre-allocated `RuntimeTagStore` in $O(1)$ per tag.
    ///
    /// # Performance Guarantee
    /// Executes with **zero heap allocations** per tick during steady-state simulation.
    pub fn sample_tags(
        &mut self,
        current_time: SimulationTime,
        tag_store: &RuntimeTagStore,
        sender: &HistorianSender,
    ) -> usize {
        let mut emitted = 0;
        let now_sec = current_time.seconds();

        // Check if we need to emit a data-loss recovery gap marker
        let health = sender.health();
        if self.had_dropped_samples && health.is_healthy {
            self.seq += 1;
            let gap_rec = HistorianRecord::DataLossGap {
                sim_time: current_time,
                seq: self.seq,
                dropped_count: health.dropped_samples_total,
            };
            let _ = sender.try_send(gap_rec);
            self.had_dropped_samples = false;
        }

        // Iterate flat slice of pre-allocated slots
        for slot in &mut self.slots {
            // Direct index read from tag store
            let (tag_val, tag_qual) = if let Some(idx) = slot.tag_store_index {
                let v = tag_store.get_value_by_index(idx);
                let q = tag_store
                    .get_quality_by_index(idx)
                    .unwrap_or(TagQuality::NotConnected);
                (v, q)
            } else {
                continue;
            };

            let Some(val) = tag_val else {
                continue;
            };

            let sample_val = match val {
                TagValue::Float(f) => HistoricalSampleValue::Float(*f),
                TagValue::Integer(i) => HistoricalSampleValue::Discrete(*i),
                TagValue::Bool(b) => HistoricalSampleValue::Discrete(if *b { 1 } else { 0 }),
                TagValue::BadQuality => HistoricalSampleValue::Float(f64::NAN),
                TagValue::Text(_) => continue, // Text strings are recorded in event journal
            };

            if Self::should_sample(slot, now_sec, sample_val, tag_qual) {
                self.seq += 1;
                let sample = HistoricalSample {
                    point_id: slot.config.point_id,
                    sim_time: current_time,
                    seq: self.seq,
                    value: sample_val,
                    quality: tag_qual,
                    persisted_wall_clock_ms: 0, // Filled or defaulted
                };

                match sender.try_send(HistorianRecord::Sample(sample)) {
                    Ok(()) => {
                        slot.last_sample_time = Some(current_time);
                        slot.last_sample_value = Some(sample_val);
                        slot.last_sample_quality = Some(tag_qual);
                        emitted += 1;
                    }
                    Err(_) => {
                        self.had_dropped_samples = true;
                    }
                }
            }
        }

        emitted
    }

    /// Fallback sampling against committed `StateReadView` (for headless/testing setups without RuntimeTagStore).
    pub fn sample_state_view(
        &mut self,
        current_time: SimulationTime,
        state: &StateReadView<'_>,
        sender: &HistorianSender,
    ) -> usize {
        let mut emitted = 0;
        let now_sec = current_time.seconds();

        for slot in &mut self.slots {
            let tag_val = match state.tag_value(slot.config.tag_id) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let tag_qual = state
                .tag_quality(slot.config.tag_id)
                .unwrap_or(TagQuality::NotConnected);

            let sample_val = match tag_val {
                TagValue::Float(f) => HistoricalSampleValue::Float(*f),
                TagValue::Integer(i) => HistoricalSampleValue::Discrete(*i),
                TagValue::Bool(b) => HistoricalSampleValue::Discrete(if *b { 1 } else { 0 }),
                TagValue::BadQuality => HistoricalSampleValue::Float(f64::NAN),
                TagValue::Text(_) => continue,
            };

            if Self::should_sample(slot, now_sec, sample_val, tag_qual) {
                self.seq += 1;
                let sample = HistoricalSample {
                    point_id: slot.config.point_id,
                    sim_time: current_time,
                    seq: self.seq,
                    value: sample_val,
                    quality: tag_qual,
                    persisted_wall_clock_ms: 0,
                };

                match sender.try_send(HistorianRecord::Sample(sample)) {
                    Ok(()) => {
                        slot.last_sample_time = Some(current_time);
                        slot.last_sample_value = Some(sample_val);
                        slot.last_sample_quality = Some(tag_qual);
                        emitted += 1;
                    }
                    Err(_) => {
                        self.had_dropped_samples = true;
                    }
                }
            }
        }

        emitted
    }

    /// Evaluates whether a point should be emitted on the current simulation tick.
    #[inline]
    fn should_sample(
        slot: &PointSlot,
        now_sec: f64,
        curr_val: HistoricalSampleValue,
        curr_qual: TagQuality,
    ) -> bool {
        // First sample ever for this point
        let (Some(last_time), Some(last_val), Some(last_qual)) = (
            slot.last_sample_time,
            slot.last_sample_value,
            slot.last_sample_quality,
        ) else {
            return true;
        };

        // Rule: Quality transitions ALWAYS force an immediate sample
        if curr_qual != last_qual {
            return true;
        }

        // Sampling mode specific checks
        match slot.config.sampling_mode {
            SamplingMode::Periodic {
                interval_sim_seconds,
            } => (now_sec - last_time.seconds()) >= (interval_sim_seconds - 1e-9),

            SamplingMode::Deadband {
                absolute_threshold,
                percent_threshold,
                max_interval_sim_seconds,
            } => {
                // Heartbeat check: max quiet time exceeded
                if (now_sec - last_time.seconds()) >= (max_interval_sim_seconds - 1e-9) {
                    return true;
                }

                // Magnitude delta check
                let span = slot.config.span();
                let threshold = absolute_threshold.max(percent_threshold * span);
                let delta = (curr_val.as_f64() - last_val.as_f64()).abs();
                delta >= threshold
            }

            SamplingMode::OnChange => curr_val != last_val,
        }
    }
}

impl Default for HistorianSampler {
    fn default() -> Self {
        Self::new()
    }
}
