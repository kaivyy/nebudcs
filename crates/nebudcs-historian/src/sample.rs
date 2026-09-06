//! Historical timeseries sample representation.

use nebudcs_model::tag::TagQuality;
use nebudcs_types::SimulationTime;
use serde::{Deserialize, Serialize};

use crate::point::HistorianPointId;

/// Value payload for a historical sample (supporting continuous analog or discrete signals).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum HistoricalSampleValue {
    /// 64-bit IEEE 754 floating point process value.
    Float(f64),
    /// 64-bit integer or discrete state.
    Discrete(i64),
}

impl HistoricalSampleValue {
    /// Extracts the value as an `f64` (discrete integers are cast to f64 for trend queries).
    #[inline]
    pub fn as_f64(&self) -> f64 {
        match *self {
            Self::Float(v) => v,
            Self::Discrete(i) => i as f64,
        }
    }
}

/// A single timeseries sample record ready for persistence.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HistoricalSample {
    /// Dense point ID.
    pub point_id: HistorianPointId,
    /// Canonical deterministic simulation time.
    pub sim_time: SimulationTime,
    /// Monotonic sequence counter within the simulation run.
    pub seq: u64,
    /// Measured or computed value.
    pub value: HistoricalSampleValue,
    /// Signal quality status.
    pub quality: TagQuality,
    /// Administrative UTC wall-clock timestamp in epoch milliseconds.
    pub persisted_wall_clock_ms: i64,
}

impl HistoricalSample {
    /// Creates a new analog (floating point) sample.
    pub fn new_float(
        point_id: HistorianPointId,
        sim_time: SimulationTime,
        seq: u64,
        value: f64,
        quality: TagQuality,
        persisted_wall_clock_ms: i64,
    ) -> Self {
        Self {
            point_id,
            sim_time,
            seq,
            value: HistoricalSampleValue::Float(value),
            quality,
            persisted_wall_clock_ms,
        }
    }

    /// Creates a new discrete (integer / boolean) sample.
    pub fn new_discrete(
        point_id: HistorianPointId,
        sim_time: SimulationTime,
        seq: u64,
        value: i64,
        quality: TagQuality,
        persisted_wall_clock_ms: i64,
    ) -> Self {
        Self {
            point_id,
            sim_time,
            seq,
            value: HistoricalSampleValue::Discrete(value),
            quality,
            persisted_wall_clock_ms,
        }
    }
}

/// Raw timeseries sample returned by query endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RawSample {
    pub point_id: HistorianPointId,
    pub sim_time: SimulationTime,
    pub seq: u64,
    pub value: f64,
    pub quality: TagQuality,
    pub persisted_wall_clock_ms: i64,
}
