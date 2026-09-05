//! Deterministic simulation time.
//!
//! # Design
//!
//! [`SimulationTime`] represents elapsed time **within the simulation** in
//! seconds.  It is:
//!
//! - Always non-negative (construction rejects negative values).
//! - Backed by `f64` for sufficient precision across long simulations.
//! - Completely independent of wall-clock time (`std::time::SystemTime`,
//!   `std::time::Instant`, Tokio timers, etc.).
//!
//! # Invariant
//!
//! The simulation clock must never advance by referencing real-world time.
//! Using any system clock in the simulation tick loop is a forbidden pattern
//! (see `coding-agent-rules.md`).

use serde::{Deserialize, Serialize};

use crate::error::{NebuError, NebuResult};

/// Elapsed simulation time in seconds.
///
/// The value is always ≥ 0.0.  The simulation engine advances this value by
/// a fixed `dt` each tick.  It is **not** a wall-clock measurement.
///
/// # Example
///
/// ```
/// use nebudcs_types::time::SimulationTime;
///
/// let t = SimulationTime::try_new(0.0).unwrap();
/// let t2 = t.advance(0.01); // one 10 ms tick
/// assert!(t2.seconds() > t.seconds());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SimulationTime(f64);

impl SimulationTime {
    /// The simulation epoch — time zero.
    pub const ZERO: SimulationTime = SimulationTime(0.0);

    /// Construct a `SimulationTime` from a non-negative seconds value.
    ///
    /// # Errors
    ///
    /// Returns [`NebuError::InvalidSimulationTime`] if `seconds` is negative
    /// or NaN.
    pub fn try_new(seconds: f64) -> NebuResult<Self> {
        if seconds.is_nan() || seconds < 0.0 {
            return Err(NebuError::InvalidSimulationTime {
                value: seconds,
                reason: "simulation time must be ≥ 0.0 and finite".to_owned(),
            });
        }
        Ok(Self(seconds))
    }

    /// Return the time value in seconds.
    #[must_use]
    #[inline]
    pub fn seconds(self) -> f64 {
        self.0
    }

    /// Advance simulation time by `dt` seconds.
    ///
    /// # Panics
    ///
    /// Panics if `dt` is negative — the simulation clock must not go
    /// backwards.  In production code the caller is always the simulation
    /// tick loop which controls `dt`.
    #[must_use]
    pub fn advance(self, dt: f64) -> Self {
        assert!(dt >= 0.0, "simulation dt must be non-negative, got {dt}");
        // Safety: self.0 ≥ 0.0 and dt ≥ 0.0, therefore the sum is ≥ 0.0.
        Self(self.0 + dt)
    }
}

impl std::fmt::Display for SimulationTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.6}s", self.0)
    }
}

impl Default for SimulationTime {
    fn default() -> Self {
        Self::ZERO
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulation_time_zero() {
        let t = SimulationTime::try_new(0.0).expect("zero is valid");
        assert_eq!(t.seconds(), 0.0);
    }

    #[test]
    fn test_simulation_time_positive() {
        let t = SimulationTime::try_new(3600.0).expect("positive is valid");
        assert_eq!(t.seconds(), 3600.0);
    }

    #[test]
    fn test_simulation_time_rejects_negative() {
        let result = SimulationTime::try_new(-1.0);
        assert!(result.is_err(), "negative simulation time must be rejected");
        match result {
            Err(NebuError::InvalidSimulationTime { value, .. }) => {
                assert_eq!(value, -1.0);
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn test_simulation_time_rejects_nan() {
        let result = SimulationTime::try_new(f64::NAN);
        assert!(result.is_err(), "NaN simulation time must be rejected");
    }

    #[test]
    fn test_simulation_time_monotonic() {
        // Advancing time produces a larger value.
        let t0 = SimulationTime::ZERO;
        let t1 = t0.advance(0.01);
        let t2 = t1.advance(0.01);
        assert!(t1 > t0);
        assert!(t2 > t1);
    }

    #[test]
    fn test_simulation_time_serde_round_trip() {
        let t = SimulationTime::try_new(42.5).unwrap();
        let json = serde_json::to_string(&t).expect("serialize");
        let decoded: SimulationTime = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(t, decoded);
    }

    #[test]
    fn test_simulation_time_display() {
        let t = SimulationTime::try_new(1.5).unwrap();
        let s = t.to_string();
        assert!(s.contains("1.500000s"), "got: {s}");
    }

    #[test]
    #[should_panic(expected = "dt must be non-negative")]
    fn test_advance_panics_on_negative_dt() {
        let t = SimulationTime::ZERO;
        let _ = t.advance(-0.001);
    }
}
