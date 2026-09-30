//! Engineering unit scaling, curve linearization, and NAMUR NE43 fault evaluation.

use serde::{Deserialize, Serialize};

use crate::hal::error::HalError;
use crate::hal::signal::{AnalogFaultState, AnalogSignalRange, ConversionCurve, IoSignalQuality};

/// Configuration governing signal transduction between raw integer counts and engineering units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalogScalingConfig {
    /// Lower range value in engineering units (corresponding to 0% span, e.g. 4 mA).
    pub lrv: f64,
    /// Upper range value in engineering units (corresponding to 100% span, e.g. 20 mA).
    pub urv: f64,
    /// Raw converter counts corresponding to LRV (e.g. 6554 counts for 4 mA on 16-bit 0–20 mA ADC).
    pub raw_min: i32,
    /// Raw converter counts corresponding to URV (e.g. 32767 counts for 20 mA).
    pub raw_max: i32,
    /// Electrical signaling standard.
    pub signal_range: AnalogSignalRange,
    /// Mathematical calibration curve.
    pub curve: ConversionCurve,
    /// Whether to clamp calculated engineering units to [LRV, URV].
    pub clamp: bool,
    /// Engineering unit representation string (e.g. "bar", "°C", "kg/s").
    pub unit: Option<String>,
}

impl AnalogScalingConfig {
    /// Creates a standard 4–20 mA linear scaling configuration.
    pub fn new_4_20ma(lrv: f64, urv: f64, unit: impl Into<String>) -> Result<Self, HalError> {
        let (raw_min, raw_max) = AnalogSignalRange::Current4_20mA.default_raw_bounds();
        let config = Self {
            lrv,
            urv,
            raw_min,
            raw_max,
            signal_range: AnalogSignalRange::Current4_20mA,
            curve: ConversionCurve::Linear,
            clamp: false,
            unit: Some(unit.into()),
        };
        config.validate()?;
        Ok(config)
    }

    /// Creates a square root extraction 4–20 mA configuration (e.g. for DP orifice flow).
    pub fn new_dp_flow_4_20ma(max_flow: f64, unit: impl Into<String>) -> Result<Self, HalError> {
        let (raw_min, raw_max) = AnalogSignalRange::Current4_20mA.default_raw_bounds();
        let config = Self {
            lrv: 0.0,
            urv: max_flow,
            raw_min,
            raw_max,
            signal_range: AnalogSignalRange::Current4_20mA,
            curve: ConversionCurve::SquareRoot,
            clamp: true,
            unit: Some(unit.into()),
        };
        config.validate()?;
        Ok(config)
    }

    /// Validates scaling parameters to prevent division by zero or invalid spans.
    pub fn validate(&self) -> Result<(), HalError> {
        if (self.urv - self.lrv).abs() < 1e-12 {
            return Err(HalError::ZeroSpan {
                lrv: self.lrv,
                urv: self.urv,
            });
        }
        if self.raw_max == self.raw_min {
            return Err(HalError::InvalidScaling {
                detail: format!(
                    "raw_max ({}) must not equal raw_min ({})",
                    self.raw_max, self.raw_min
                ),
            });
        }
        Ok(())
    }

    /// Converts raw ADC integer counts into scaled engineering units,
    /// evaluating NAMUR NE43 criteria and determining signal quality.
    pub fn raw_to_eu(
        &self,
        raw_counts: i32,
    ) -> Result<(f64, IoSignalQuality, AnalogFaultState), HalError> {
        self.validate()?;

        // 1. NAMUR NE43 Analog Fault Detection (for 4-20 mA current loops)
        let (quality, fault_state) = self.evaluate_namur_ne43(raw_counts);

        // 2. Compute normalized fractional span
        let raw_span = (self.raw_max - self.raw_min) as f64;
        let mut fraction = (raw_counts - self.raw_min) as f64 / raw_span;

        if self.clamp {
            fraction = fraction.clamp(0.0, 1.0);
        }

        // 3. Apply conversion curve
        let eu_span = self.urv - self.lrv;
        let eu = match &self.curve {
            ConversionCurve::Linear => self.lrv + fraction * eu_span,
            ConversionCurve::SquareRoot => {
                let non_negative = fraction.max(0.0);
                self.lrv + non_negative.sqrt() * eu_span
            }
            ConversionCurve::Polynomial(coeffs) => {
                // c0 + c1*x + c2*x^2 + c3*x^3
                let x = fraction;
                coeffs[0] + coeffs[1] * x + coeffs[2] * x.powi(2) + coeffs[3] * x.powi(3)
            }
        };

        Ok((eu, quality, fault_state))
    }

    /// Converts engineering units to raw DAC integer counts for output channels.
    pub fn eu_to_raw(&self, eu: f64) -> Result<(i32, IoSignalQuality), HalError> {
        self.validate()?;

        let eu_span = self.urv - self.lrv;
        let mut fraction = match &self.curve {
            ConversionCurve::Linear => (eu - self.lrv) / eu_span,
            ConversionCurve::SquareRoot => {
                let norm = ((eu - self.lrv) / eu_span).max(0.0);
                norm * norm
            }
            ConversionCurve::Polynomial(_) => (eu - self.lrv) / eu_span,
        };

        let quality = if fraction < -0.05 || fraction > 1.05 {
            IoSignalQuality::OutOfRange
        } else {
            IoSignalQuality::Good
        };

        if self.clamp {
            fraction = fraction.clamp(0.0, 1.0);
        }

        let raw_span = (self.raw_max - self.raw_min) as f64;
        let raw = self.raw_min as f64 + fraction * raw_span;
        let raw_counts = raw.round() as i32;

        Ok((raw_counts, quality))
    }

    /// Evaluates NAMUR NE43 analog fault criteria for standard 4–20 mA loops.
    ///
    /// # Standard Thresholds:
    /// - Normal operation: 3.8 mA to 20.5 mA (`Good`, `Normal`)
    /// - Under-range: 3.6 mA <= I < 3.8 mA (`Uncertain`, `UnderRange`)
    /// - Over-range: 20.5 mA < I <= 21.0 mA (`Uncertain`, `OverRange`)
    /// - Wire-break: I < 3.6 mA (`Bad`, `WireBreak`)
    /// - Short-circuit / High fault: I > 21.0 mA (`Bad`, `ShortCircuit`)
    pub fn evaluate_namur_ne43(&self, raw_counts: i32) -> (IoSignalQuality, AnalogFaultState) {
        if self.signal_range != AnalogSignalRange::Current4_20mA {
            // Non-4-20mA range: simple raw bounds check
            let min_bound = self.raw_min.min(self.raw_max);
            let max_bound = self.raw_min.max(self.raw_max);
            if raw_counts < min_bound - (max_bound - min_bound) / 20 {
                return (IoSignalQuality::Bad, AnalogFaultState::UnderRange);
            }
            if raw_counts > max_bound + (max_bound - min_bound) / 20 {
                return (IoSignalQuality::Bad, AnalogFaultState::OverRange);
            }
            return (IoSignalQuality::Good, AnalogFaultState::Normal);
        }

        // 4-20 mA calculation:
        // raw_min = 4 mA (0% span), raw_max = 20 mA (100% span)
        let span = (self.raw_max - self.raw_min) as f64;
        let milliamp = 4.0 + ((raw_counts - self.raw_min) as f64 / span) * 16.0;

        if milliamp < 3.6 {
            (IoSignalQuality::Bad, AnalogFaultState::WireBreak)
        } else if milliamp < 3.8 {
            (IoSignalQuality::Uncertain, AnalogFaultState::UnderRange)
        } else if milliamp <= 20.5 {
            (IoSignalQuality::Good, AnalogFaultState::Normal)
        } else if milliamp <= 21.0 {
            (IoSignalQuality::Uncertain, AnalogFaultState::OverRange)
        } else {
            (IoSignalQuality::Bad, AnalogFaultState::ShortCircuit)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_scaling_and_roundtrip() {
        let config = AnalogScalingConfig::new_4_20ma(0.0, 100.0, "bar").unwrap();

        // 4 mA (raw_min) -> 0.0 bar, Good
        let (eu_0, q_0, f_0) = config.raw_to_eu(config.raw_min).unwrap();
        assert!((eu_0 - 0.0).abs() < 1e-4);
        assert_eq!(q_0, IoSignalQuality::Good);
        assert_eq!(f_0, AnalogFaultState::Normal);

        // 20 mA (raw_max) -> 100.0 bar, Good
        let (eu_100, q_100, f_100) = config.raw_to_eu(config.raw_max).unwrap();
        assert!((eu_100 - 100.0).abs() < 1e-4);
        assert_eq!(q_100, IoSignalQuality::Good);
        assert_eq!(f_100, AnalogFaultState::Normal);

        // 12 mA (50% span) -> 50.0 bar
        let mid_raw = config.raw_min + (config.raw_max - config.raw_min) / 2;
        let (eu_50, q_50, _) = config.raw_to_eu(mid_raw).unwrap();
        assert!((eu_50 - 50.0).abs() < 0.1);
        assert_eq!(q_50, IoSignalQuality::Good);

        // Reverse conversion
        let (raw_back, q_back) = config.eu_to_raw(50.0).unwrap();
        assert!((raw_back - mid_raw).abs() <= 1);
        assert_eq!(q_back, IoSignalQuality::Good);
    }

    #[test]
    fn test_square_root_dp_flow() {
        let config = AnalogScalingConfig::new_dp_flow_4_20ma(10.0, "kg/s").unwrap();

        // 25% DP (head) -> sqrt(0.25) = 50% flow -> 5.0 kg/s
        let raw_25 = config.raw_min + (config.raw_max - config.raw_min) / 4;
        let (flow, _, _) = config.raw_to_eu(raw_25).unwrap();
        assert!((flow - 5.0).abs() < 0.05);

        // Reverse conversion: 5.0 kg/s -> 25% DP
        let (raw_back, _) = config.eu_to_raw(5.0).unwrap();
        assert!((raw_back - raw_25).abs() <= 1);
    }

    #[test]
    fn test_namur_ne43_fault_detection() {
        let config = AnalogScalingConfig::new_4_20ma(0.0, 50.0, "m").unwrap();
        let span = (config.raw_max - config.raw_min) as f64;

        // Wire break (< 3.6 mA, e.g. 3.0 mA)
        let raw_3ma = (config.raw_min as f64 + ((3.0 - 4.0) / 16.0) * span).round() as i32;
        let (_, q_wb, f_wb) = config.raw_to_eu(raw_3ma).unwrap();
        assert_eq!(q_wb, IoSignalQuality::Bad);
        assert_eq!(f_wb, AnalogFaultState::WireBreak);

        // Under-range (3.7 mA)
        let raw_3_7ma = (config.raw_min as f64 + ((3.7 - 4.0) / 16.0) * span).round() as i32;
        let (_, q_ur, f_ur) = config.raw_to_eu(raw_3_7ma).unwrap();
        assert_eq!(q_ur, IoSignalQuality::Uncertain);
        assert_eq!(f_ur, AnalogFaultState::UnderRange);

        // Over-range (20.8 mA)
        let raw_20_8ma = (config.raw_min as f64 + ((20.8 - 4.0) / 16.0) * span).round() as i32;
        let (_, q_or, f_or) = config.raw_to_eu(raw_20_8ma).unwrap();
        assert_eq!(q_or, IoSignalQuality::Uncertain);
        assert_eq!(f_or, AnalogFaultState::OverRange);

        // Short circuit (> 21.0 mA, e.g. 22.0 mA)
        let raw_22ma = (config.raw_min as f64 + ((22.0 - 4.0) / 16.0) * span).round() as i32;
        let (_, q_sc, f_sc) = config.raw_to_eu(raw_22ma).unwrap();
        assert_eq!(q_sc, IoSignalQuality::Bad);
        assert_eq!(f_sc, AnalogFaultState::ShortCircuit);
    }

    #[test]
    fn test_zero_span_rejection() {
        let res = AnalogScalingConfig::new_4_20ma(10.0, 10.0, "bar");
        assert!(res.is_err());
    }
}
