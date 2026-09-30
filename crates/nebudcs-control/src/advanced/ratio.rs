//! Industrial ratio control block.
//!
//! Regulates the ratio between a wild (uncontrolled or independent) stream and a
//! controlled (dependent) stream according to:
//!
//! $$SP_{controlled} = \text{Ratio} \times PV_{wild} + \text{Bias}$$

use serde::{Deserialize, Serialize};

/// Ratio controller configuration and runtime state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RatioController {
    /// Desired ratio target (e.g. 0.5 air/fuel ratio or reflux ratio).
    pub ratio: f64,
    /// Minimum permissible ratio.
    pub ratio_min: f64,
    /// Maximum permissible ratio.
    pub ratio_max: f64,
    /// Additive bias trim.
    pub bias: f64,
    /// Lower limit for calculated setpoint.
    pub sp_min: f64,
    /// Upper limit for calculated setpoint.
    pub sp_max: f64,
    /// Current calculated setpoint output.
    output_sp: f64,
    /// Last measured actual ratio.
    actual_ratio: f64,
}

impl RatioController {
    /// Creates a ratio controller with the given target ratio.
    pub fn new(ratio: f64) -> Self {
        Self {
            ratio,
            ratio_min: 0.0,
            ratio_max: 100.0,
            bias: 0.0,
            sp_min: -1.0e9,
            sp_max: 1.0e9,
            output_sp: 0.0,
            actual_ratio: ratio,
        }
    }

    /// Sets the active target ratio, clamped within configured bounds.
    pub fn set_ratio(&mut self, ratio: f64) {
        if ratio.is_finite() {
            self.ratio = ratio.clamp(self.ratio_min, self.ratio_max);
        }
    }

    /// Sets the additive bias.
    pub fn set_bias(&mut self, bias: f64) {
        if bias.is_finite() {
            self.bias = bias;
        }
    }

    /// Configures ratio bounds $[R_{min}, R_{max}]$.
    pub fn set_ratio_limits(&mut self, min: f64, max: f64) {
        if min.is_finite() && max.is_finite() && min < max {
            self.ratio_min = min;
            self.ratio_max = max;
            self.ratio = self.ratio.clamp(min, max);
        }
    }

    /// Configures calculated setpoint bounds $[SP_{min}, SP_{max}]$.
    pub fn set_sp_limits(&mut self, min: f64, max: f64) {
        if min.is_finite() && max.is_finite() && min < max {
            self.sp_min = min;
            self.sp_max = max;
            self.output_sp = self.output_sp.clamp(min, max);
        }
    }

    /// Tracks the actual process ratio from current measurements for bumpless transfer.
    pub fn track_actual(&mut self, pv_wild: f64, pv_controlled: f64) {
        if pv_wild.abs() > 1e-6 {
            let r = (pv_controlled - self.bias) / pv_wild;
            if r.is_finite() {
                self.actual_ratio = r;
                self.ratio = r.clamp(self.ratio_min, self.ratio_max);
            }
        }
    }

    /// Calculates the controlled stream setpoint from the wild stream measurement.
    pub fn calculate(&mut self, pv_wild: f64) -> f64 {
        if !pv_wild.is_finite() {
            return self.output_sp;
        }

        let raw_sp = self.ratio * pv_wild + self.bias;
        self.output_sp = raw_sp.clamp(self.sp_min, self.sp_max);
        self.output_sp
    }

    /// Returns the current calculated setpoint.
    pub fn output_sp(&self) -> f64 {
        self.output_sp
    }

    /// Returns the last computed actual ratio.
    pub fn actual_ratio(&self) -> f64 {
        self.actual_ratio
    }
}
