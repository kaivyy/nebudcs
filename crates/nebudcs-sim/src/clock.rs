//! Deterministic simulation clock.
//!
//! # Core Invariants
//!
//! - Advances only through discrete simulation ticks (`advance_tick()`).
//! - Wall-clock time never determines simulation time.
//! - Fixed timestep ($\Delta t$) remains constant regardless of `speed_multiplier`.
//! - `speed_multiplier` is metadata for external pacing / real-time governors, never changing $\Delta t$.
//! - Pause prevents automatic advancement in a tick loop.
//! - Single-step explicitly advances by one fixed tick regardless of pause state.
//! - Non-positive, NaN, and infinite timestep and speed multiplier values are strictly rejected.

use serde::{Deserialize, Serialize};

use nebudcs_types::SimulationTime;

use crate::error::SimError;

/// Operational mode of the simulation clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ClockState {
    /// Clock is running and should advance on loop iterations.
    Running,
    /// Clock is paused; automatic loop ticking is halted.
    #[default]
    Paused,
}

/// Deterministic fixed-step simulation clock.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationClock {
    /// Fixed simulation timestep (seconds) per tick.
    dt: f64,
    /// Monotonically increasing simulation time.
    time: SimulationTime,
    /// Number of ticks executed since simulation start.
    tick_count: u64,
    /// Current running or paused state.
    state: ClockState,
    /// Speed multiplier for realtime pacing (e.g. 1.0x, 2.0x, 0.5x).
    /// Does NOT alter `dt` or calculation physics.
    speed_multiplier: f64,
}

impl SimulationClock {
    /// Creates a new simulation clock with the specified fixed timestep ($\Delta t$).
    /// Defaults to `Paused` state at $t = 0.0$ with `1.0` speed multiplier.
    pub fn new(dt: f64) -> Result<Self, SimError> {
        if !dt.is_finite() || dt <= 0.0 {
            return Err(SimError::InvalidTimestep { dt });
        }
        Ok(Self {
            dt,
            time: SimulationTime::ZERO,
            tick_count: 0,
            state: ClockState::Paused,
            speed_multiplier: 1.0,
        })
    }

    /// Fixed timestep $\Delta t$ in seconds.
    #[inline]
    pub fn dt(&self) -> f64 {
        self.dt
    }

    /// Current simulation time.
    #[inline]
    pub fn time(&self) -> SimulationTime {
        self.time
    }

    /// Current total tick counter.
    #[inline]
    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }

    /// Current operational state (Running or Paused).
    #[inline]
    pub fn state(&self) -> ClockState {
        self.state
    }

    /// Returns `true` if the clock is in the `Running` state.
    #[inline]
    pub fn is_running(&self) -> bool {
        self.state == ClockState::Running
    }

    /// Returns `true` if the clock is in the `Paused` state.
    #[inline]
    pub fn is_paused(&self) -> bool {
        self.state == ClockState::Paused
    }

    /// Speed multiplier for realtime scheduling.
    #[inline]
    pub fn speed_multiplier(&self) -> f64 {
        self.speed_multiplier
    }

    /// Sets the speed multiplier (must be finite and > 0.0).
    pub fn set_speed_multiplier(&mut self, multiplier: f64) -> Result<(), SimError> {
        if !multiplier.is_finite() || multiplier <= 0.0 {
            return Err(SimError::InvalidSpeedMultiplier { multiplier });
        }
        self.speed_multiplier = multiplier;
        Ok(())
    }

    /// Transitions clock to `Running` mode.
    pub fn play(&mut self) {
        self.state = ClockState::Running;
    }

    /// Transitions clock to `Paused` mode.
    pub fn pause(&mut self) {
        self.state = ClockState::Paused;
    }

    /// Advances the clock by exactly one fixed timestep $\Delta t$ unconditionally.
    ///
    /// This represents the deterministic core transition:
    /// - Advances `time` by `dt`
    /// - Increments `tick_count` by 1
    pub fn advance_tick(&mut self) {
        self.time = self.time.advance(self.dt);
        self.tick_count = self.tick_count.saturating_add(1);
    }

    /// Resets the clock to zero simulation time and zero tick count in `Paused` state.
    pub fn reset(&mut self) {
        self.time = SimulationTime::ZERO;
        self.tick_count = 0;
        self.state = ClockState::Paused;
    }
}
