//! Standard DCS and PLC logic primitives: timers, flip-flops/latches, and permissives.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// On-Delay Timer (TON).
///
/// When `in_val` becomes `true`, the timer accumulates elapsed time.
/// When elapsed time reaches or exceeds `preset`, output `q` becomes `true`.
/// If `in_val` goes `false`, elapsed time resets to 0.0 and `q` becomes `false`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TonTimer {
    preset: f64,
    elapsed: f64,
    q: bool,
}

impl TonTimer {
    /// Creates a new on-delay timer with preset duration in seconds.
    pub fn new(preset: f64) -> Self {
        Self {
            preset: preset.max(0.0),
            elapsed: 0.0,
            q: false,
        }
    }

    /// Steps the timer by $\Delta t$ given current boolean input.
    pub fn step(&mut self, dt: f64, in_val: bool) -> bool {
        if in_val {
            self.elapsed += dt;
            if self.elapsed >= self.preset {
                self.q = true;
            }
        } else {
            self.elapsed = 0.0;
            self.q = false;
        }
        self.q
    }

    /// Current timer output status.
    pub fn q(&self) -> bool {
        self.q
    }

    /// Elapsed time in seconds.
    pub fn elapsed(&self) -> f64 {
        self.elapsed
    }
}

/// Off-Delay Timer (TOF).
///
/// When `in_val` is `true`, output `q` is immediately `true` and elapsed time is 0.0.
/// When `in_val` transitions to `false`, the timer begins accumulating elapsed time.
/// While elapsed time < `preset`, output `q` remains `true`.
/// When elapsed time reaches `preset`, output `q` turns `false`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TofTimer {
    preset: f64,
    elapsed: f64,
    q: bool,
}

impl TofTimer {
    /// Creates a new off-delay timer with preset duration in seconds.
    pub fn new(preset: f64) -> Self {
        Self {
            preset: preset.max(0.0),
            elapsed: 0.0,
            q: false,
        }
    }

    /// Steps the timer by $\Delta t$ given current boolean input.
    pub fn step(&mut self, dt: f64, in_val: bool) -> bool {
        if in_val {
            self.q = true;
            self.elapsed = 0.0;
        } else if self.q {
            self.elapsed += dt;
            if self.elapsed >= self.preset {
                self.q = false;
            }
        }
        self.q
    }

    /// Current timer output status.
    pub fn q(&self) -> bool {
        self.q
    }
}

/// Set-Dominant Latch (SR Flip-Flop).
///
/// If `set` is true, output `q` is set to `true`, regardless of `reset`.
/// If `reset` is true and `set` is false, output `q` is set to `false`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SrLatch {
    q: bool,
}

impl SrLatch {
    /// Creates a new SR latch with initial state.
    pub fn new(initial: bool) -> Self {
        Self { q: initial }
    }

    /// Steps the latch with current set and reset inputs.
    pub fn step(&mut self, set: bool, reset: bool) -> bool {
        if set {
            self.q = true;
        } else if reset {
            self.q = false;
        }
        self.q
    }

    /// Output state.
    pub fn q(&self) -> bool {
        self.q
    }
}

/// Reset-Dominant Latch (RS Flip-Flop).
///
/// If `reset` is true, output `q` is set to `false`, regardless of `set`.
/// If `set` is true and `reset` is false, output `q` is set to `true`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RsLatch {
    q: bool,
}

impl RsLatch {
    /// Creates a new RS latch with initial state.
    pub fn new(initial: bool) -> Self {
        Self { q: initial }
    }

    /// Steps the latch with current set and reset inputs.
    pub fn step(&mut self, set: bool, reset: bool) -> bool {
        if reset {
            self.q = false;
        } else if set {
            self.q = true;
        }
        self.q
    }

    /// Output state.
    pub fn q(&self) -> bool {
        self.q
    }
}

/// Permissive guard aggregating multiple binary interlock/permissive conditions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PermissiveGuard {
    name: String,
    conditions: BTreeMap<String, bool>,
}

impl PermissiveGuard {
    /// Creates a new permissive guard with an identifier.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            conditions: BTreeMap::new(),
        }
    }

    /// Identifier.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Adds or registers a required condition.
    pub fn add_condition(&mut self, condition_name: impl Into<String>, satisfied: bool) {
        self.conditions.insert(condition_name.into(), satisfied);
    }

    /// Sets or updates an existing condition.
    pub fn set_condition(&mut self, condition_name: &str, satisfied: bool) {
        self.conditions
            .insert(condition_name.to_string(), satisfied);
    }

    /// Returns true if and only if ALL configured conditions are satisfied.
    pub fn is_satisfied(&self) -> bool {
        !self.conditions.is_empty() && self.conditions.values().all(|&v| v)
    }

    /// Returns list of currently unsatisfied condition names for diagnostics.
    pub fn unsatisfied_conditions(&self) -> Vec<String> {
        self.conditions
            .iter()
            .filter(|(_, &satisfied)| !satisfied)
            .map(|(name, _)| name.clone())
            .collect()
    }
}
