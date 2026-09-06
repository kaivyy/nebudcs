//! Safety Instrumented Protection System and configurable multi-sensor voting.

use nebudcs_model::tag::TagQuality;
use serde::{Deserialize, Serialize};

/// Operational states of a safety protection function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtectionState {
    /// Guard armed and monitoring process variables.
    Armed,
    /// Normal active monitoring.
    Active,
    /// Initiating threshold crossed, trip latched, safety overrides asserted.
    Tripped,
    /// Transient reset state upon operator reset command.
    Reset,
    /// Administratively bypassed for maintenance/testing.
    Bypassed,
    /// Redundant sensor channel fault detected; voting degraded.
    Degraded,
}

/// Static configuration for a protection block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtectionConfig {
    /// Function identifier (e.g. "ESD-V101-HIGH-PRESSURE").
    pub name: String,
    /// Trip activation threshold.
    pub threshold: f64,
    /// Margin below threshold required before trip latch can be reset.
    pub reset_deadband: f64,
}

impl ProtectionConfig {
    /// Validates the protection configuration parameters.
    pub fn validate(&self) -> Result<(), crate::error::ControlError> {
        if !self.threshold.is_finite() {
            return Err(crate::error::ControlError::InvalidConfiguration {
                component: "ProtectionConfig".to_string(),
                detail: "threshold must be finite".to_string(),
            });
        }
        if !self.reset_deadband.is_finite() || self.reset_deadband < 0.0 {
            return Err(crate::error::ControlError::InvalidConfiguration {
                component: "ProtectionConfig".to_string(),
                detail: format!(
                    "reset_deadband must be non-negative and finite, got {}",
                    self.reset_deadband
                ),
            });
        }
        Ok(())
    }
}

/// A latched protective trip block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtectionBlock {
    config: ProtectionConfig,
    state: ProtectionState,
}

impl ProtectionBlock {
    /// Creates a new protection block in the `Armed` state.
    pub fn new(config: ProtectionConfig) -> Self {
        Self {
            config,
            state: ProtectionState::Armed,
        }
    }

    /// Current operational state.
    pub fn state(&self) -> ProtectionState {
        self.state
    }

    /// Explicitly sets the operational state (for testing or administrative overrides).
    pub fn set_state(&mut self, state: ProtectionState) {
        self.state = state;
    }

    /// True if currently in the latched `Tripped` state.
    pub fn is_tripped(&self) -> bool {
        self.state == ProtectionState::Tripped
    }

    /// Sets administrative maintenance bypass.
    pub fn set_bypassed(&mut self, bypassed: bool) {
        if bypassed {
            self.state = ProtectionState::Bypassed;
        } else if self.state == ProtectionState::Bypassed {
            self.state = ProtectionState::Armed;
        }
    }

    /// Resets the protection block directly if initiating condition is cleared.
    pub fn reset(&mut self, current_pv: f64) -> bool {
        if self.state == ProtectionState::Tripped {
            let cleared_threshold = self.config.threshold - self.config.reset_deadband;
            if current_pv <= cleared_threshold {
                self.state = ProtectionState::Armed;
                return true;
            }
        }
        false
    }

    /// Steps the protection block with current PV and optional operator manual reset command.
    ///
    /// - If in `Tripped`: remains latched until condition clears below
    ///   `threshold - reset_deadband` AND `reset_command` is true.
    /// - If in `Armed`/`Active`: trips if PV reaches or exceeds `threshold`.
    pub fn step(&mut self, pv: f64, reset_command: bool) -> bool {
        if self.state == ProtectionState::Bypassed {
            return false;
        }

        if self.state == ProtectionState::Tripped {
            let cleared_threshold = self.config.threshold - self.config.reset_deadband;
            if reset_command && pv <= cleared_threshold {
                self.state = ProtectionState::Armed;
            }
        } else if pv >= self.config.threshold {
            self.state = ProtectionState::Tripped;
        } else if self.state == ProtectionState::Armed {
            self.state = ProtectionState::Active;
        }

        self.is_tripped()
    }

    /// Steps the protection block with multi-channel voting inputs.
    pub fn step_with_voting(
        &mut self,
        voter: &VotingVoter2oo3,
        inputs: &[SensorInput; 3],
        reset_command: bool,
    ) -> bool {
        if self.state == ProtectionState::Bypassed {
            return false;
        }

        let res = voter.evaluate(inputs);

        if self.state == ProtectionState::Tripped {
            // Condition for reset: trip logic no longer demands trip and reset_command is true
            if reset_command && !res.trip {
                self.state = if res.is_degraded {
                    ProtectionState::Degraded
                } else {
                    ProtectionState::Armed
                };
            }
        } else if res.trip {
            self.state = ProtectionState::Tripped;
        } else if res.is_degraded {
            self.state = ProtectionState::Degraded;
        } else {
            self.state = ProtectionState::Active;
        }

        self.is_tripped()
    }
}

/// Configurable degradation policy when a sensor channel reports `TagQuality::Bad`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DegradationPolicy {
    /// Conservative / Fail-Safe: degrades 2oo3 to 1oo2 upon 1 channel fault.
    DegradeTo1oo2,
    /// Fault-Tolerant / High Availability: degrades 2oo3 to 2oo2 upon 1 channel fault.
    DegradeTo2oo2,
    /// Trip on any channel fault.
    TripOnFault,
}

/// Transduced sensor measurement input to a voting block.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SensorInput {
    pub value: f64,
    pub quality: TagQuality,
}

impl SensorInput {
    pub fn new(value: f64, quality: TagQuality) -> Self {
        Self { value, quality }
    }

    pub fn is_healthy(&self) -> bool {
        self.quality == TagQuality::Good
    }
}

/// Result of a multi-channel voting evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VotingResult {
    pub trip: bool,
    pub is_degraded: bool,
    pub fault_count: usize,
}

/// Two-out-of-three (2oo3) multi-sensor voting block with configurable degradation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VotingVoter2oo3 {
    threshold: f64,
    policy: DegradationPolicy,
}

impl VotingVoter2oo3 {
    /// Creates a 2oo3 voter with specified trip threshold and channel fault policy.
    pub fn new(threshold: f64, policy: DegradationPolicy) -> Self {
        Self { threshold, policy }
    }

    /// Evaluates 3 redundant sensor inputs against the trip threshold and degradation rules.
    pub fn evaluate(&self, inputs: &[SensorInput; 3]) -> VotingResult {
        let mut healthy_count = 0;
        let mut healthy_trips = 0;

        for input in inputs {
            if input.is_healthy() {
                healthy_count += 1;
                if input.value >= self.threshold {
                    healthy_trips += 1;
                }
            }
        }

        let fault_count = 3 - healthy_count;

        match fault_count {
            0 => VotingResult {
                trip: healthy_trips >= 2,
                is_degraded: false,
                fault_count: 0,
            },
            1 => {
                let trip = match self.policy {
                    DegradationPolicy::DegradeTo1oo2 => healthy_trips >= 1,
                    DegradationPolicy::DegradeTo2oo2 => healthy_trips >= 2,
                    DegradationPolicy::TripOnFault => true,
                };
                VotingResult {
                    trip,
                    is_degraded: true,
                    fault_count: 1,
                }
            }
            _ => {
                // 2 or 3 failed channels: unsafe condition -> immediate fail-safe trip
                VotingResult {
                    trip: true,
                    is_degraded: true,
                    fault_count,
                }
            }
        }
    }
}
