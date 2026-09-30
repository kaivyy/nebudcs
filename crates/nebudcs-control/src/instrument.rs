//! Instrument and transmitter runtime model.
//!
//! # Architecture
//!
//! Transduces continuous physical process state variables into DCS tags across 6 stages:
//!
//! $$\text{Physical Variable } (X_{phys}) \to \text{Sensor} \to \text{Transmitter} \to \text{Transducer Limits} \to \text{Conditioning} \to \text{Engineering Scaling} \to \text{Tag}$$
//!
//! # Diagnostics
//!
//! - Configuration validation (rejects zero span: $URV == LRV$, negative time constants).
//! - Stale signal detection ($T_{elapsed} > 3 \times T_{scan} \to \text{Bad}$).
//! - Frozen signal detection ($N \ge 100$ scans identical while process has variance $\to \text{Uncertain}$).
//! - Electrical saturation limits ($[-5\%, 105\%]$ of span).
//! - Deterministic PRNG Gaussian noise.

use nebudcs_model::tag::TagQuality;
use serde::{Deserialize, Serialize};

use crate::error::ControlError;

/// Configuration for an instrument and transmitter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstrumentConfig {
    /// Lower range value in engineering units.
    pub lrv: f64,
    /// Upper range value in engineering units.
    pub urv: f64,
    /// Sensor element first-order thermal/diaphragm lag time constant $\tau_s$ (seconds).
    pub sensor_lag_tau: f64,
    /// Digital damping filter time constant $\tau_d$ (seconds).
    pub damping_tau: f64,
    /// Gaussian noise standard deviation in engineering units.
    pub noise_std_dev: f64,
    /// Calibration zero bias error $\delta_{zero}$.
    pub calibration_bias: f64,
    /// Calibration span gain $\kappa_{span}$.
    pub span_gain: f64,
    /// Nominal scan update interval $T_{scan}$ in seconds.
    pub scan_interval: f64,
}

impl InstrumentConfig {
    /// Returns a new builder for `InstrumentConfig`.
    pub fn builder() -> InstrumentConfigBuilder {
        InstrumentConfigBuilder::default()
    }

    /// Span in engineering units ($URV - LRV$).
    pub fn span(&self) -> f64 {
        self.urv - self.lrv
    }

    /// Validates the configuration parameters.
    pub fn validate(&self) -> Result<(), ControlError> {
        if (self.urv - self.lrv).abs() < 1e-12 {
            return Err(ControlError::InvalidConfiguration {
                component: "InstrumentConfig".to_string(),
                detail: format!(
                    "zero-span range: URV ({}) must not equal LRV ({})",
                    self.urv, self.lrv
                ),
            });
        }
        if self.sensor_lag_tau < 0.0 {
            return Err(ControlError::InvalidConfiguration {
                component: "InstrumentConfig".to_string(),
                detail: format!(
                    "sensor_lag_tau must be non-negative, got {}",
                    self.sensor_lag_tau
                ),
            });
        }
        if self.damping_tau < 0.0 {
            return Err(ControlError::InvalidConfiguration {
                component: "InstrumentConfig".to_string(),
                detail: format!("damping_tau must be non-negative, got {}", self.damping_tau),
            });
        }
        if self.scan_interval <= 0.0 {
            return Err(ControlError::InvalidConfiguration {
                component: "InstrumentConfig".to_string(),
                detail: format!(
                    "scan_interval must be strictly positive, got {}",
                    self.scan_interval
                ),
            });
        }
        Ok(())
    }
}

impl Default for InstrumentConfig {
    fn default() -> Self {
        Self {
            lrv: 0.0,
            urv: 100.0,
            sensor_lag_tau: 0.0,
            damping_tau: 0.0,
            noise_std_dev: 0.0,
            calibration_bias: 0.0,
            span_gain: 1.0,
            scan_interval: 0.1,
        }
    }
}

/// Builder for `InstrumentConfig`.
#[derive(Debug, Clone)]
pub struct InstrumentConfigBuilder {
    lrv: f64,
    urv: f64,
    sensor_lag_tau: f64,
    damping_tau: f64,
    noise_std_dev: f64,
    calibration_bias: f64,
    span_gain: f64,
    scan_interval: f64,
}

impl Default for InstrumentConfigBuilder {
    fn default() -> Self {
        Self {
            lrv: 0.0,
            urv: 100.0,
            sensor_lag_tau: 0.0,
            damping_tau: 0.0,
            noise_std_dev: 0.0,
            calibration_bias: 0.0,
            span_gain: 1.0,
            scan_interval: 0.1,
        }
    }
}

impl InstrumentConfigBuilder {
    pub fn lrv(mut self, lrv: f64) -> Self {
        self.lrv = lrv;
        self
    }

    pub fn urv(mut self, urv: f64) -> Self {
        self.urv = urv;
        self
    }

    pub fn sensor_lag_tau(mut self, tau: f64) -> Self {
        self.sensor_lag_tau = tau;
        self
    }

    pub fn damping_tau(mut self, tau: f64) -> Self {
        self.damping_tau = tau;
        self
    }

    pub fn noise_std_dev(mut self, std_dev: f64) -> Self {
        self.noise_std_dev = std_dev;
        self
    }

    pub fn calibration_bias(mut self, bias: f64) -> Self {
        self.calibration_bias = bias;
        self
    }

    pub fn span_gain(mut self, gain: f64) -> Self {
        self.span_gain = gain;
        self
    }

    pub fn scan_interval(mut self, interval: f64) -> Self {
        self.scan_interval = interval;
        self
    }

    pub fn build(self) -> Result<InstrumentConfig, ControlError> {
        let config = InstrumentConfig {
            lrv: self.lrv,
            urv: self.urv,
            sensor_lag_tau: self.sensor_lag_tau,
            damping_tau: self.damping_tau,
            noise_std_dev: self.noise_std_dev,
            calibration_bias: self.calibration_bias,
            span_gain: self.span_gain,
            scan_interval: self.scan_interval,
        };
        config.validate()?;
        Ok(config)
    }
}

/// Runtime dynamic model for an instrument and transmitter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstrumentModel {
    config: InstrumentConfig,
    sensor_val: f64,
    damped_val: f64,
    quality: TagQuality,
    rng_state: u64,
    consecutive_identical_count: usize,
    last_process_val: f64,
}

impl InstrumentModel {
    /// Creates a new instrument model initialized at baseline value.
    pub fn new(config: InstrumentConfig, initial_val: f64) -> Self {
        Self {
            config,
            sensor_val: initial_val,
            damped_val: initial_val,
            quality: TagQuality::Good,
            rng_state: 0x853c49e6748fea9b,
            consecutive_identical_count: 0,
            last_process_val: initial_val,
        }
    }

    /// Access to configuration.
    pub fn config(&self) -> &InstrumentConfig {
        &self.config
    }

    /// Current measured engineering value after all stages.
    pub fn value(&self) -> f64 {
        self.damped_val
    }

    /// Current tag quality.
    pub fn quality(&self) -> TagQuality {
        self.quality
    }

    /// Sets explicit tag quality (e.g. for forced fault simulation).
    pub fn set_quality(&mut self, quality: TagQuality) {
        self.quality = quality;
    }

    /// Clears any transient diagnostic flags back to Good.
    pub fn clear_diagnostics(&mut self) {
        self.quality = TagQuality::Good;
        self.consecutive_identical_count = 0;
    }

    /// Minimal deterministic xorshift64 pseudo-random generator returning uniform [0, 1).
    fn next_uniform(&mut self) -> f64 {
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng_state = x;
        (x as f64) / (u64::MAX as f64)
    }

    /// Deterministic Box-Muller Gaussian standard normal N(0, 1).
    fn next_gaussian(&mut self) -> f64 {
        let u1 = self.next_uniform().max(1e-12);
        let u2 = self.next_uniform();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }

    /// Steps the instrument through its complete transduction pipeline.
    ///
    /// - `dt`: simulation step size in seconds.
    /// - `phys_val`: actual physical process variable from physics model.
    /// - `elapsed_since_scan`: time elapsed since last transmitter scan.
    pub fn step(&mut self, dt: f64, phys_val: f64, elapsed_since_scan: f64) -> f64 {
        // 1. Stale signal diagnostic detection
        if elapsed_since_scan > 3.0 * self.config.scan_interval {
            self.quality = TagQuality::Bad;
            return self.damped_val;
        }

        // 2. Stage 1: Sensor Element Lag
        if self.config.sensor_lag_tau > 0.0 && dt > 0.0 {
            let alpha_s = dt / (self.config.sensor_lag_tau + dt);
            self.sensor_val += alpha_s * (phys_val - self.sensor_val);
        } else {
            self.sensor_val = phys_val;
        }

        // 3. Stage 2: Transmitter Calibration & Noise
        let mut tx_val = self.config.span_gain * self.sensor_val + self.config.calibration_bias;
        if self.config.noise_std_dev > 0.0 {
            tx_val += self.config.noise_std_dev * self.next_gaussian();
        }

        // 4. Stage 3: Transducer Saturation Limits [-5%, +105%] of span
        let span = self.config.span();
        let lower_limit = self.config.lrv - 0.05 * span;
        let upper_limit = self.config.urv + 0.05 * span;
        let saturated_val =
            tx_val.clamp(lower_limit.min(upper_limit), lower_limit.max(upper_limit));

        // 5. Stage 4: Signal Conditioning (Damping Filter)
        if self.config.damping_tau > 0.0 && dt > 0.0 {
            let alpha_d = dt / (self.config.damping_tau + dt);
            self.damped_val += alpha_d * (saturated_val - self.damped_val);
        } else {
            self.damped_val = saturated_val;
        }

        // 6. Stage 5 & 6: Range & Quality Assessment
        if self.damped_val < self.config.lrv || self.damped_val > self.config.urv {
            if self.quality == TagQuality::Good {
                self.quality = TagQuality::Uncertain;
            }
        } else if self.quality == TagQuality::Uncertain && self.consecutive_identical_count < 100 {
            self.quality = TagQuality::Good;
        }

        self.last_process_val = phys_val;
        self.damped_val
    }

    /// Steps the instrument in frozen fault mode for diagnostic testing.
    pub fn step_frozen(&mut self, _dt: f64, phys_val: f64, frozen_val: f64) -> f64 {
        let process_variance = (phys_val - self.last_process_val).abs();
        let measurement_variance = (frozen_val - self.damped_val).abs();

        if measurement_variance < 1e-12 && process_variance > 1e-4 {
            self.consecutive_identical_count += 1;
            if self.consecutive_identical_count >= 100 {
                self.quality = TagQuality::Uncertain;
            }
        } else {
            self.consecutive_identical_count = 0;
        }

        self.damped_val = frozen_val;
        self.last_process_val = phys_val;
        self.damped_val
    }
}
