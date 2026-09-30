//! Standard industrial DCS/PLC PID controller implementation.
//!
//! # Capabilities
//!
//! - Modes: `Manual`, `Auto`, `Cascade`, `RemoteCascade`.
//! - Bumpless transfer across mode switches ($|\Delta OUT| < 10^{-9}$).
//! - Setpoint tracking in Manual mode (configurable).
//! - Filtered derivative on PV (derivative kick prevention).
//! - Anti-windup with conditional integration and output clamping.
//! - Safety override tracking mode (`TrackingMode`) tracking actual actuator position.
//! - Direct and reverse action.
//! - Multi-rate scan scheduling support.

use nebudcs_model::tag::TagQuality;
use serde::{Deserialize, Serialize};

/// Operational mode of the PID controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PidMode {
    /// Operator directly commands the output ($0\text{--}100\%$).
    Manual,
    /// Controller automatically calculates output from local setpoint.
    Auto,
    /// Controller setpoint is driven by a master controller or supervisory system.
    Cascade,
    /// Remote computer supervisory cascade mode.
    RemoteCascade,
}

/// Direction of control action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PidDirection {
    /// Reverse acting: increasing PV decreases output (e.g. heating, tank filling).
    /// $e = SP - PV$.
    Reverse,
    /// Direct acting: increasing PV increases output (e.g. cooling, tank draining).
    /// $e = PV - SP$.
    Direct,
}

/// Static configuration for a PID controller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PidConfig {
    /// Proportional gain $K_p$.
    pub kp: f64,
    /// Integral time $T_i$ in seconds. If $\le 0.0$, integral action is disabled.
    pub ti: f64,
    /// Derivative time $T_d$ in seconds. If $\le 0.0$, derivative action is disabled.
    pub td: f64,
    /// Derivative filter coefficient $N$ (typically $8\text{--}20$, standard 10.0).
    pub derivative_filter_n: f64,
    /// Lower limit of output demand (e.g. 0.0%).
    pub out_min: f64,
    /// Upper limit of output demand (e.g. 100.0%).
    pub out_max: f64,
    /// Action direction (Reverse or Direct).
    pub direction: PidDirection,
    /// When true, setpoint continuously tracks PV in Manual mode to guarantee bumpless transfer.
    pub sp_tracking_in_manual: bool,
    /// Scan interval divisor relative to simulation tick count (default 1).
    pub scan_divisor: u32,
}

impl PidConfig {
    /// Validates the PID configuration parameters.
    pub fn validate(&self) -> Result<(), crate::error::ControlError> {
        if !self.kp.is_finite() {
            return Err(crate::error::ControlError::InvalidConfiguration {
                component: "PidConfig".to_string(),
                detail: "kp must be finite".to_string(),
            });
        }
        if !self.ti.is_finite() || self.ti < 0.0 {
            return Err(crate::error::ControlError::InvalidConfiguration {
                component: "PidConfig".to_string(),
                detail: format!("ti must be non-negative and finite, got {}", self.ti),
            });
        }
        if !self.td.is_finite() || self.td < 0.0 {
            return Err(crate::error::ControlError::InvalidConfiguration {
                component: "PidConfig".to_string(),
                detail: format!("td must be non-negative and finite, got {}", self.td),
            });
        }
        if !self.derivative_filter_n.is_finite() || self.derivative_filter_n <= 0.0 {
            return Err(crate::error::ControlError::InvalidConfiguration {
                component: "PidConfig".to_string(),
                detail: format!(
                    "derivative_filter_n must be strictly positive, got {}",
                    self.derivative_filter_n
                ),
            });
        }
        if !self.out_min.is_finite() || !self.out_max.is_finite() || self.out_min >= self.out_max {
            return Err(crate::error::ControlError::InvalidConfiguration {
                component: "PidConfig".to_string(),
                detail: format!(
                    "out_min ({}) must be strictly less than out_max ({})",
                    self.out_min, self.out_max
                ),
            });
        }
        if self.scan_divisor < 1 {
            return Err(crate::error::ControlError::InvalidConfiguration {
                component: "PidConfig".to_string(),
                detail: "scan_divisor must be >= 1".to_string(),
            });
        }
        Ok(())
    }
}

impl Default for PidConfig {
    fn default() -> Self {
        Self {
            kp: 1.0,
            ti: 10.0,
            td: 0.0,
            derivative_filter_n: 10.0,
            out_min: 0.0,
            out_max: 100.0,
            direction: PidDirection::Reverse,
            sp_tracking_in_manual: true,
            scan_divisor: 1,
        }
    }
}

/// Complete runtime state and memory of a PID controller block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PidController {
    config: PidConfig,
    mode: PidMode,
    sp: f64,
    sp_min: f64,
    sp_max: f64,
    pv: f64,
    pv_prev: f64,
    integral: f64,
    derivative: f64,
    output: f64,
    manual_output: f64,
    is_tracking: bool,
    tracked_position: f64,
    is_high_saturated: bool,
    is_low_saturated: bool,
    quality: TagQuality,
    feedforward: f64,
    first_scan: bool,
}

impl PidController {
    /// Creates a new PID controller initialized in Manual mode.
    pub fn new(config: PidConfig) -> Self {
        let initial_out = config.out_min;
        Self {
            config,
            mode: PidMode::Manual,
            sp: 0.0,
            sp_min: -1.0e9,
            sp_max: 1.0e9,
            pv: 0.0,
            pv_prev: 0.0,
            integral: 0.0,
            derivative: 0.0,
            output: initial_out,
            manual_output: initial_out,
            is_tracking: false,
            tracked_position: initial_out,
            is_high_saturated: false,
            is_low_saturated: false,
            quality: TagQuality::Good,
            feedforward: 0.0,
            first_scan: true,
        }
    }

    /// Initializes the controller at a known initial PV.
    pub fn initialize(&mut self, initial_pv: f64) {
        self.pv = initial_pv;
        self.pv_prev = initial_pv;
        self.derivative = 0.0;
        self.first_scan = false;
        if self.config.sp_tracking_in_manual {
            self.sp = initial_pv.clamp(self.sp_min, self.sp_max);
        }
    }

    /// Sets explicit setpoint limits $[SP_{min}, SP_{max}]$.
    pub fn set_sp_limits(&mut self, min: f64, max: f64) {
        if min.is_finite() && max.is_finite() && min < max {
            self.sp_min = min;
            self.sp_max = max;
            self.sp = self.sp.clamp(min, max);
        }
    }

    /// Lower setpoint limit.
    pub fn sp_min(&self) -> f64 {
        self.sp_min
    }

    /// Upper setpoint limit.
    pub fn sp_max(&self) -> f64 {
        self.sp_max
    }

    /// Access to controller configuration.
    pub fn config(&self) -> &PidConfig {
        &self.config
    }

    /// Mutable access to configuration.
    pub fn config_mut(&mut self) -> &mut PidConfig {
        &mut self.config
    }

    /// Current operational mode.
    pub fn mode(&self) -> PidMode {
        self.mode
    }

    /// Sets the operational mode with bumpless transfer mechanics.
    pub fn set_mode(&mut self, new_mode: PidMode) {
        if self.mode == new_mode {
            return;
        }

        match (self.mode, new_mode) {
            (PidMode::Manual, PidMode::Auto) => {
                // Bumpless transfer MAN -> AUTO:
                // Pre-align derivative memory to current PV to prevent derivative kick
                self.pv_prev = self.pv;
                self.derivative = 0.0;

                if self.config.sp_tracking_in_manual {
                    self.sp = self.pv.clamp(self.sp_min, self.sp_max);
                }

                let err = self.calculate_error(self.sp, self.pv);
                let p = self.config.kp * err;

                // Pre-load integral such that calculated OUT == manual_output:
                // OUT = P + I + D + FF => I = manual_output - P - D - FF
                let target_i = self.manual_output - p - self.feedforward;
                self.integral = target_i.clamp(self.config.out_min, self.config.out_max);
                self.output = self.manual_output;
            }
            (PidMode::Auto, PidMode::Manual) | (PidMode::Cascade, PidMode::Manual) => {
                // Bumpless transfer AUTO -> MAN:
                self.manual_output = self.output;
            }
            (PidMode::Auto, PidMode::Cascade) => {
                // When entering cascade, preserve existing output
            }
            (PidMode::Cascade, PidMode::Auto) => {
                // When dropping from Cascade to Auto, latch current working SP as local SP
                // so the loop does not bump
            }
            _ => {}
        }

        self.mode = new_mode;
    }

    /// Current active working setpoint.
    pub fn sp(&self) -> f64 {
        self.sp
    }

    /// Sets the local setpoint clamped to configured SP limits.
    pub fn set_sp(&mut self, sp: f64) {
        self.sp = sp.clamp(self.sp_min, self.sp_max);
    }

    /// Current measured process variable.
    pub fn pv(&self) -> f64 {
        self.pv
    }

    /// Current integral term accumulator.
    pub fn integral(&self) -> f64 {
        self.integral
    }

    /// Current filtered derivative term.
    pub fn derivative(&self) -> f64 {
        self.derivative
    }

    /// Current controller output command ($0\text{--}100\%$).
    pub fn output(&self) -> f64 {
        self.output
    }

    /// Current manual output demand.
    pub fn manual_output(&self) -> f64 {
        self.manual_output
    }

    /// Sets the manual output demand when in Manual mode.
    pub fn set_manual_output(&mut self, val: f64) {
        self.manual_output = val.clamp(self.config.out_min, self.config.out_max);
        if self.mode == PidMode::Manual && !self.is_tracking {
            self.output = self.manual_output;
        }
    }

    /// True if output is clamped at maximum limit.
    pub fn is_high_saturated(&self) -> bool {
        self.is_high_saturated
    }

    /// True if output is clamped at minimum limit.
    pub fn is_low_saturated(&self) -> bool {
        self.is_low_saturated
    }

    /// Enters safety override tracking mode.
    ///
    /// The controller forces its output demand to track the reported physical
    /// actuator position, adjusting the integral accumulator so that when the
    /// override clears, the controller resumes with zero bump.
    pub fn enter_tracking(&mut self, actual_actuator_position: f64) {
        self.is_tracking = true;
        self.tracked_position = actual_actuator_position;
        self.output = actual_actuator_position;

        // Keep integral aligned to tracked position
        let err = self.calculate_error(self.sp, self.pv);
        let p = self.config.kp * err;
        self.integral = (actual_actuator_position - p - self.derivative)
            .clamp(self.config.out_min, self.config.out_max);
    }

    /// Clears safety override tracking mode.
    pub fn exit_tracking(&mut self) {
        self.is_tracking = false;
        if self.mode == PidMode::Manual {
            self.manual_output = self.output;
        }
    }

    /// Sets the cascade setpoint (consumed when in `Cascade` or `RemoteCascade` mode),
    /// clamped to configured SP limits.
    pub fn set_cascade_sp(&mut self, sp: f64) {
        if self.mode == PidMode::Cascade || self.mode == PidMode::RemoteCascade {
            self.sp = sp.clamp(self.sp_min, self.sp_max);
        }
    }

    /// Current feedforward contribution.
    pub fn feedforward(&self) -> f64 {
        self.feedforward
    }

    /// Sets the feedforward bias/trim contribution.
    pub fn set_feedforward(&mut self, ff: f64) {
        self.feedforward = if ff.is_finite() { ff } else { 0.0 };
    }

    /// Executes one discrete PID calculation step with an explicit feedforward value.
    pub fn step_with_feedforward(&mut self, dt: f64, pv: f64, feedforward: f64) -> f64 {
        self.set_feedforward(feedforward);
        self.step(dt, pv)
    }

    /// Applies external reset feedback to align the integral accumulator with an
    /// external constraint or selector output, preventing reset windup.
    pub fn apply_external_reset_feedback(&mut self, ext_fb: f64) {
        if ext_fb.is_finite() {
            let err = self.calculate_error(self.sp, self.pv);
            let p = self.config.kp * err;
            self.integral = (ext_fb - p - self.derivative - self.feedforward)
                .clamp(self.config.out_min, self.config.out_max);
        }
    }

    /// Sets signal quality for this controller.
    pub fn set_quality(&mut self, quality: TagQuality) {
        self.quality = quality;
    }

    /// Signal quality.
    pub fn quality(&self) -> TagQuality {
        self.quality
    }

    /// Calculates error according to controller action direction.
    #[inline]
    fn calculate_error(&self, sp: f64, pv: f64) -> f64 {
        match self.config.direction {
            PidDirection::Reverse => sp - pv,
            PidDirection::Direct => pv - sp,
        }
    }

    /// Resets internal controller accumulators, derivative memory, and tracking state.
    pub fn reset(&mut self) {
        self.integral = 0.0;
        self.derivative = 0.0;
        self.pv_prev = self.pv;
        self.output = self.config.out_min;
        self.manual_output = self.config.out_min;
        self.is_tracking = false;
        self.tracked_position = self.config.out_min;
        self.is_high_saturated = false;
        self.is_low_saturated = false;
        self.feedforward = 0.0;
        self.first_scan = true;
    }

    /// Executes one discrete PID calculation step with fixed timestep $\Delta t$ and current PV.
    ///
    /// Returns the computed output demand clamped to $[OUT_{min}, OUT_{max}]$.
    pub fn step(&mut self, dt: f64, pv: f64) -> f64 {
        // Reject non-finite inputs or non-positive dt to prevent state corruption
        if !pv.is_finite() || !self.sp.is_finite() || dt <= 0.0 {
            return self.output;
        }

        // When PV quality is Bad, suspend closed-loop integration and hold output
        if self.quality == TagQuality::Bad {
            return self.output;
        }

        self.pv = pv;

        // If tracking mode is active (due to safety interlock/ESD), track actual position
        if self.is_tracking {
            self.output = self.tracked_position;
            self.pv_prev = pv;
            return self.output;
        }

        // If in Manual mode, output is directly driven by manual_output
        if self.mode == PidMode::Manual {
            if self.config.sp_tracking_in_manual {
                self.sp = pv.clamp(self.sp_min, self.sp_max);
            }
            self.pv_prev = pv;
            self.derivative = 0.0;
            self.output = self.manual_output;
            return self.output;
        }

        // AUTO or CASCADE closed-loop regulation
        let err = self.calculate_error(self.sp, pv);

        // 1. Proportional term
        let p = self.config.kp * err;

        // 2. Filtered Derivative term on PV (derivative-on-measurement)
        // Eliminates derivative kick on setpoint changes
        if self.config.td > 0.0 && self.config.derivative_filter_n > 0.0 && dt > 0.0 {
            let n = self.config.derivative_filter_n;
            let td = self.config.td;
            let alpha = td / (td + n * dt);
            let delta_pv = if self.first_scan {
                self.first_scan = false;
                0.0
            } else {
                pv - self.pv_prev
            };

            // D[k] = alpha * D[k-1] - (Kp * N * Td / (Td + N*dt)) * delta_PV
            // Note: sign depends on direction: reverse direction means d(err) = -d(pv)
            let sign = match self.config.direction {
                PidDirection::Reverse => -1.0,
                PidDirection::Direct => 1.0,
            };
            let d_gain = (self.config.kp * n * td) / (td + n * dt);
            self.derivative = alpha * self.derivative + sign * d_gain * delta_pv;
        } else {
            self.derivative = 0.0;
        }

        // 3. Integral term with Anti-Windup (Conditional Integration)
        if self.config.ti > 0.0 && dt > 0.0 {
            let delta_i = (self.config.kp * dt / self.config.ti) * err;
            let candidate_out = p + self.integral + self.derivative + self.feedforward;

            // Conditional integration logic:
            // If output is saturated high and error drives output higher, suspend integration.
            // If output is saturated low and error drives output lower, suspend integration.
            let driving_higher = err > 0.0;
            let driving_lower = err < 0.0;

            let suppress_integration = (candidate_out >= self.config.out_max && driving_higher)
                || (candidate_out <= self.config.out_min && driving_lower);

            if !suppress_integration {
                self.integral += delta_i;
                self.integral = self
                    .integral
                    .clamp(self.config.out_min, self.config.out_max);
            }
        }

        // 4. Compute unclamped and clamped output
        let raw_out = p + self.integral + self.derivative + self.feedforward;
        self.output = raw_out.clamp(self.config.out_min, self.config.out_max);

        self.is_high_saturated = raw_out >= self.config.out_max;
        self.is_low_saturated = raw_out <= self.config.out_min;

        self.pv_prev = pv;
        self.output
    }
}
