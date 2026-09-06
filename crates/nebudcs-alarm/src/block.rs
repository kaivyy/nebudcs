//! Single alarm block evaluating limit thresholds, hysteresis deadband, and ISA-18.2 states.

use nebudcs_model::tag::TagQuality;
use nebudcs_types::TagId;
use serde::{Deserialize, Serialize};

use crate::types::{AlarmSeverity, AlarmState, AlarmType};

/// Configuration for an individual alarm detection block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlarmBlockConfig {
    /// Alarm identifier (e.g. "TI101_HI").
    pub name: String,
    /// Measured process tag ID.
    pub tag_id: TagId,
    /// Type of limit check.
    pub alarm_type: AlarmType,
    /// Criticality severity level.
    pub severity: AlarmSeverity,
    /// Engineering limit threshold.
    pub limit: f64,
    /// Hysteresis deadband margin.
    pub deadband: f64,
    /// Default shelving duration in seconds.
    pub default_shelve_duration: f64,
}

impl AlarmBlockConfig {
    /// Validates the alarm block configuration parameters.
    pub fn validate(&self) -> Result<(), crate::error::AlarmError> {
        if !self.limit.is_finite() {
            return Err(crate::error::AlarmError::InvalidConfiguration {
                alarm_name: self.name.clone(),
                detail: "limit must be finite".to_string(),
            });
        }
        if !self.deadband.is_finite() || self.deadband < 0.0 {
            return Err(crate::error::AlarmError::InvalidConfiguration {
                alarm_name: self.name.clone(),
                detail: format!(
                    "deadband must be non-negative and finite, got {}",
                    self.deadband
                ),
            });
        }
        if !self.default_shelve_duration.is_finite() || self.default_shelve_duration < 0.0 {
            return Err(crate::error::AlarmError::InvalidConfiguration {
                alarm_name: self.name.clone(),
                detail: format!(
                    "default_shelve_duration must be non-negative and finite, got {}",
                    self.default_shelve_duration
                ),
            });
        }
        Ok(())
    }
}

/// Runtime alarm evaluation block with full ISA-18.2 lifecycle state tracking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlarmBlock {
    config: AlarmBlockConfig,
    state: AlarmState,
    shelve_remaining: f64,
    last_pv: f64,
}

impl AlarmBlock {
    /// Creates a new alarm block initialized in the `Normal` state.
    pub fn new(config: AlarmBlockConfig) -> Self {
        Self {
            config,
            state: AlarmState::Normal,
            shelve_remaining: 0.0,
            last_pv: 0.0,
        }
    }

    /// Configuration.
    pub fn config(&self) -> &AlarmBlockConfig {
        &self.config
    }

    /// Current operational lifecycle state.
    pub fn state(&self) -> AlarmState {
        self.state
    }

    /// True if currently annunciating (either active unacknowledged or acknowledged).
    pub fn is_active(&self) -> bool {
        matches!(self.state, AlarmState::ActiveUnack | AlarmState::ActiveAck)
    }

    /// True if unacknowledged.
    pub fn is_unack(&self) -> bool {
        self.state == AlarmState::ActiveUnack
    }

    /// Operator manual acknowledgement.
    pub fn acknowledge(&mut self) {
        match self.state {
            AlarmState::ActiveUnack => self.state = AlarmState::ActiveAck,
            AlarmState::Cleared => self.state = AlarmState::Normal,
            _ => {}
        }
    }

    /// Operator shelving for a duration in seconds.
    pub fn shelve(&mut self, duration_seconds: f64) {
        self.state = AlarmState::Shelved;
        self.shelve_remaining = duration_seconds.max(0.0);
    }

    /// Operator manual unshelving.
    pub fn unshelve(&mut self) {
        if self.state == AlarmState::Shelved {
            // Re-evaluate on next step or clear
            self.state = AlarmState::Normal;
            self.shelve_remaining = 0.0;
        }
    }

    /// Evaluates whether the process variable violates the alarm condition with hysteresis.
    pub fn is_condition_violated(&self, pv: f64, quality: TagQuality) -> bool {
        let was_active = matches!(
            self.state,
            AlarmState::ActiveUnack | AlarmState::ActiveAck | AlarmState::Shelved
        );

        match self.config.alarm_type {
            AlarmType::Hi | AlarmType::HiHi => {
                if was_active {
                    // Stays violated until dropping below limit - deadband
                    pv >= (self.config.limit - self.config.deadband)
                } else {
                    // Triggers when reaching or exceeding limit
                    pv >= self.config.limit
                }
            }
            AlarmType::Lo | AlarmType::LoLo => {
                if was_active {
                    // Stays violated until rising above limit + deadband
                    pv <= (self.config.limit + self.config.deadband)
                } else {
                    // Triggers when reaching or dropping below limit
                    pv <= self.config.limit
                }
            }
            AlarmType::BadQuality => quality == TagQuality::Bad,
            AlarmType::Deviation => {
                if was_active {
                    pv.abs() >= (self.config.limit - self.config.deadband)
                } else {
                    pv.abs() >= self.config.limit
                }
            }
            AlarmType::RateOfChange => {
                let roc = (pv - self.last_pv).abs();
                roc >= self.config.limit
            }
        }
    }

    /// Steps the alarm block lifecycle for one simulation timestep $\Delta t$.
    pub fn step(&mut self, pv: f64, quality: TagQuality, dt: f64) -> AlarmState {
        let violated = self.is_condition_violated(pv, quality);

        match self.state {
            AlarmState::Normal => {
                if violated {
                    self.state = AlarmState::ActiveUnack;
                }
            }
            AlarmState::ActiveUnack => {
                if !violated {
                    self.state = AlarmState::Cleared;
                }
            }
            AlarmState::ActiveAck => {
                if !violated {
                    self.state = AlarmState::Normal;
                }
            }
            AlarmState::Cleared => {
                if violated {
                    self.state = AlarmState::ActiveUnack;
                }
            }
            AlarmState::Shelved => {
                self.shelve_remaining -= dt;
                if self.shelve_remaining <= 0.0 {
                    if violated {
                        self.state = AlarmState::ActiveUnack;
                    } else {
                        self.state = AlarmState::Normal;
                    }
                }
            }
        }

        self.last_pv = pv;
        self.state
    }

    /// Last recorded process variable value.
    pub fn last_pv(&self) -> f64 {
        self.last_pv
    }
}
