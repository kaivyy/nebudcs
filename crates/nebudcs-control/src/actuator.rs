//! Actuator dynamic modeling and deterministic 5-tier command arbitration.

use serde::{Deserialize, Serialize};

/// 5-tier actuator command priority hierarchy.
///
/// Priority 1 (ESD) is the absolute highest priority; Priority 5 (Auto) is lowest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CommandPriority {
    /// Priority 1: Emergency Shutdown (ESD) zone / plant-wide trip.
    Esd = 1,
    /// Priority 2: Protective safety interlock override.
    Interlock = 2,
    /// Priority 3: Equipment state machine permissive constraint.
    Permissive = 3,
    /// Priority 4: Operator manual field override.
    Manual = 4,
    /// Priority 5: Automatic modulating PID controller demand.
    Auto = 5,
}

/// Mechanical fail-safe behavior of an actuator upon air or electrical power failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailAction {
    /// Air-to-open / spring-to-close: loss of power drives valve to 0.0% (closed).
    FailClosed,
    /// Air-to-close / spring-to-open: loss of power drives valve to 100.0% (open).
    FailOpen,
    /// Lock-in-place: loss of power locks stem at current position.
    FailInPlace,
}

/// A command submitted to an actuator arbiter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActuatorCommand {
    /// Priority tier of this command.
    pub priority: CommandPriority,
    /// Demanded stem/actuator position ($0\text{--}100\%$).
    pub target_position: f64,
    /// Optional identifier of the commanding subsystem.
    pub source_id: Option<String>,
}

impl ActuatorCommand {
    /// Creates a new actuator command.
    pub fn new(priority: CommandPriority, target_position: f64) -> Self {
        Self {
            priority,
            target_position: target_position.clamp(0.0, 100.0),
            source_id: None,
        }
    }

    /// Sets an optional source identifier.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source_id = Some(source.into());
        self
    }
}

/// Deterministic arbiter resolving competing commands according to the 5-tier hierarchy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActuatorArbiter {
    fail_action: FailAction,
    commands: Vec<ActuatorCommand>,
}

impl ActuatorArbiter {
    /// Creates a new arbiter configured with the actuator's fail-safe behavior.
    pub fn new(fail_action: FailAction) -> Self {
        Self {
            fail_action,
            commands: Vec::new(),
        }
    }

    /// Submits a command into the arbiter.
    pub fn submit(&mut self, cmd: ActuatorCommand) {
        // If an existing command has matching priority and source, replace it
        if let Some(existing) = self
            .commands
            .iter_mut()
            .find(|c| c.priority == cmd.priority && c.source_id == cmd.source_id)
        {
            *existing = cmd;
        } else {
            self.commands.push(cmd);
        }
    }

    /// Clears all commands at a specific priority tier.
    pub fn clear_priority(&mut self, priority: CommandPriority) {
        self.commands.retain(|c| c.priority != priority);
    }

    /// Clears all commands matching a specific source identifier.
    pub fn clear_source(&mut self, source_id: &str) {
        self.commands
            .retain(|c| c.source_id.as_deref() != Some(source_id));
    }

    /// Clears all commands.
    pub fn clear_all(&mut self) {
        self.commands.clear();
    }

    /// Arbitrates among active commands and returns the winning command.
    ///
    /// - Priority 1 (ESD) takes precedence over Priority 2 (Interlock), etc.
    /// - For simultaneous conflicting commands in the same priority tier:
    ///   - FailClosed: minimum position (close) wins.
    ///   - FailOpen: maximum position (open) wins.
    ///   - FailInPlace: minimum position (conservative) wins.
    pub fn arbitrate(&self) -> Option<ActuatorCommand> {
        if self.commands.is_empty() {
            return None;
        }

        // Find the highest priority present (lowest integer representation)
        let highest_priority = self.commands.iter().map(|c| c.priority).min()?;

        // Filter and find winning command with zero heap allocations
        let mut matching = self
            .commands
            .iter()
            .filter(|c| c.priority == highest_priority);
        let first = matching.next()?;

        let winning = match self.fail_action {
            FailAction::FailClosed | FailAction::FailInPlace => {
                matching.fold(first, |min_cmd, c| {
                    if c.target_position < min_cmd.target_position {
                        c
                    } else {
                        min_cmd
                    }
                })
            }
            FailAction::FailOpen => matching.fold(first, |max_cmd, c| {
                if c.target_position > max_cmd.target_position {
                    c
                } else {
                    max_cmd
                }
            }),
        };

        Some(winning.clone())
    }
}

/// Configuration parameters for an actuator dynamic model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActuatorConfig {
    /// Full-stroke travel time in seconds ($0\%\text{--}100\%$).
    pub stroke_time: f64,
    /// Positioner deadband percentage (e.g. 0.5%).
    pub deadband: f64,
    /// Mechanical fail-safe orientation.
    pub fail_action: FailAction,
}

impl ActuatorConfig {
    /// Validates the actuator configuration parameters.
    pub fn validate(&self) -> Result<(), crate::error::ControlError> {
        if !self.stroke_time.is_finite() || self.stroke_time < 0.0 {
            return Err(crate::error::ControlError::InvalidConfiguration {
                component: "ActuatorConfig".to_string(),
                detail: format!(
                    "stroke_time must be non-negative and finite, got {}",
                    self.stroke_time
                ),
            });
        }
        if !self.deadband.is_finite() || self.deadband < 0.0 || self.deadband > 100.0 {
            return Err(crate::error::ControlError::InvalidConfiguration {
                component: "ActuatorConfig".to_string(),
                detail: format!(
                    "deadband must be between 0.0 and 100.0%, got {}",
                    self.deadband
                ),
            });
        }
        Ok(())
    }
}

impl Default for ActuatorConfig {
    fn default() -> Self {
        Self {
            stroke_time: 5.0,
            deadband: 0.5,
            fail_action: FailAction::FailClosed,
        }
    }
}

/// Dynamic model of a physical actuator (pneumatic/electric valve positioner).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActuatorModel {
    config: ActuatorConfig,
    actual_position: f64,
    last_effective_cmd: f64,
}

impl ActuatorModel {
    /// Creates a new actuator model initialized at initial position ($0\text{--}100\%$).
    pub fn new(config: ActuatorConfig, initial_position: f64) -> Self {
        let pos = initial_position.clamp(0.0, 100.0);
        Self {
            config,
            actual_position: pos,
            last_effective_cmd: pos,
        }
    }

    /// Configuration.
    pub fn config(&self) -> &ActuatorConfig {
        &self.config
    }

    /// Current actual physical position ($0\text{--}100\%$).
    pub fn actual_position(&self) -> f64 {
        self.actual_position
    }

    /// Sets the actual position directly (for initialization or test fixture setup).
    pub fn set_actual_position(&mut self, pos: f64) {
        self.actual_position = pos.clamp(0.0, 100.0);
        self.last_effective_cmd = self.actual_position;
    }

    /// Steps actuator dynamics by fixed timestep $\Delta t$.
    ///
    /// - If `power_or_air_ok` is false: actuator drives toward mechanical fail position.
    /// - If `power_or_air_ok` is true: applies positioner deadband and slew rate limiter.
    pub fn step(&mut self, dt: f64, commanded_pos: f64, power_or_air_ok: bool) -> f64 {
        let target = if !power_or_air_ok {
            match self.config.fail_action {
                FailAction::FailClosed => 0.0,
                FailAction::FailOpen => 100.0,
                FailAction::FailInPlace => self.actual_position,
            }
        } else {
            // Apply deadband
            let cmd_clamped = commanded_pos.clamp(0.0, 100.0);
            if (cmd_clamped - self.last_effective_cmd).abs() >= self.config.deadband {
                self.last_effective_cmd = cmd_clamped;
            }
            self.last_effective_cmd
        };

        // Slew rate limiting based on stroke time
        if self.config.stroke_time > 0.0 && dt > 0.0 {
            let max_delta = (100.0 / self.config.stroke_time) * dt;
            let diff = target - self.actual_position;
            if diff.abs() <= max_delta {
                self.actual_position = target;
            } else {
                self.actual_position += diff.signum() * max_delta;
            }
        } else {
            self.actual_position = target;
        }

        self.actual_position = self.actual_position.clamp(0.0, 100.0);
        self.actual_position
    }
}
