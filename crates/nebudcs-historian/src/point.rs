//! Historian point configuration and sampling mode definitions.

use nebudcs_types::TagId;
use nebudcs_units::QuantityKind;
use serde::{Deserialize, Serialize};

/// Dense, 0-indexed identifier for a registered historian point.
///
/// Enables $O(1)$ flat array lookup in the sampling hot loop without hash map overhead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HistorianPointId(pub u32);

impl HistorianPointId {
    /// Creates a new point identifier.
    #[inline]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Access the underlying index.
    #[inline]
    pub const fn as_u32(self) -> u32 {
        self.0
    }

    /// Access as usize offset for array indexing.
    #[inline]
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }
}

impl std::fmt::Display for HistorianPointId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PT#{}", self.0)
    }
}

/// Sampling policy governing when a point is evaluated and emitted to history.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum SamplingMode {
    /// Sampled on every fixed simulation interval.
    Periodic {
        /// Sampling period in simulation seconds.
        interval_sim_seconds: f64,
    },
    /// Sampled whenever value changes beyond deadband threshold or on heartbeat.
    Deadband {
        /// Absolute change threshold: $|\Delta v| \ge \text{absolute\_threshold}$.
        absolute_threshold: f64,
        /// Relative change threshold fraction of engineering span.
        percent_threshold: f64,
        /// Maximum quiet-time heartbeat in simulation seconds.
        max_interval_sim_seconds: f64,
    },
    /// Sampled on any numeric or state change.
    OnChange,
}

impl Default for SamplingMode {
    fn default() -> Self {
        Self::Periodic {
            interval_sim_seconds: 1.0,
        }
    }
}

/// Static configuration and engineering metadata for a registered historian point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistorianPointConfig {
    /// Unique point identifier.
    pub point_id: HistorianPointId,
    /// Associated runtime tag ID.
    pub tag_id: TagId,
    /// Human-readable tag path / name (e.g. `"FCV101.PV"`).
    pub name: String,
    /// Physical quantity kind (e.g. `Pressure`, `Temperature`), or `None` if dimensionless.
    pub quantity_kind: Option<QuantityKind>,
    /// Engineering unit representation string (e.g. `"bar"`, `"degC"`, `"kg/h"`, `"%"`).
    pub engineering_unit: String,
    /// Lower engineering limit for visualization/normalization.
    pub eng_min: f64,
    /// Upper engineering limit for visualization/normalization.
    pub eng_max: f64,
    /// Active sampling policy.
    pub sampling_mode: SamplingMode,
}

impl HistorianPointConfig {
    /// Creates a new historian point configuration.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        point_id: HistorianPointId,
        tag_id: TagId,
        name: impl Into<String>,
        quantity_kind: Option<QuantityKind>,
        engineering_unit: impl Into<String>,
        eng_min: f64,
        eng_max: f64,
        sampling_mode: SamplingMode,
    ) -> Self {
        Self {
            point_id,
            tag_id,
            name: name.into(),
            quantity_kind,
            engineering_unit: engineering_unit.into(),
            eng_min,
            eng_max,
            sampling_mode,
        }
    }

    /// Calculates the engineering span ($eng\_max - eng\_min$).
    #[inline]
    pub fn span(&self) -> f64 {
        (self.eng_max - self.eng_min).abs()
    }
}
