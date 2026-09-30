//! Control valve characterization curves (IEC 60534 / ISA-75 standards).
//!
//! Transforms nominal stem position $x \in [0.0, 1.0]$ into effective flow capacity
//! fraction $f(x) \in [0.0, 1.0]$:
//! - [`ValveCharacteristic::Linear`]: $f(x) = x$.
//! - [`ValveCharacteristic::EqualPercentage`]: $f(x) = \frac{R^x - 1}{R - 1}$ with rangeability $R$ (e.g. 50).
//! - [`ValveCharacteristic::QuickOpening`]: $f(x) = \sqrt{x}$.
//! - [`ValveCharacteristic::Custom`]: Piecewise linear breakpoint table.

use serde::{Deserialize, Serialize};

/// Standard industrial valve trim characteristics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(tag = "type", content = "params")]
pub enum ValveCharacteristic {
    /// Linear characteristic: flow is directly proportional to stem lift.
    #[default]
    Linear,
    /// Equal-percentage characteristic with rangeability parameter $R$ (typically $30\text{--}50$).
    EqualPercentage {
        /// Inherent rangeability $R = C_{v,max} / C_{v,min}$ (default 50.0).
        rangeability: f64,
    },
    /// Quick-opening characteristic: steep initial flow increase at low lift.
    QuickOpening,
    /// Custom piecewise linear curve defined by ordered $(x, y)$ coordinate pairs.
    Custom {
        /// Breakpoints with $x \in [0, 1]$ and $y \in [0, 1]$.
        breakpoints: Vec<(f64, f64)>,
    },
}

/// Valve characterizer block evaluating effective flow coefficient fraction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValveCharacterizer {
    /// Configured trim characteristic.
    pub characteristic: ValveCharacteristic,
}

impl ValveCharacterizer {
    /// Creates a characterizer with the specified trim type.
    pub fn new(characteristic: ValveCharacteristic) -> Self {
        Self { characteristic }
    }

    /// Creates an equal-percentage characterizer with standard rangeability $R=50$.
    pub fn equal_percentage(rangeability: f64) -> Self {
        Self {
            characteristic: ValveCharacteristic::EqualPercentage {
                rangeability: if rangeability > 1.0 {
                    rangeability
                } else {
                    50.0
                },
            },
        }
    }

    /// Evaluates effective flow coefficient fraction $f(x) \in [0.0, 1.0]$ from stem position $x \in [0.0, 1.0]$.
    pub fn evaluate(&self, stem_position: f64) -> f64 {
        let x = stem_position.clamp(0.0, 1.0);
        if x <= 1e-6 {
            return 0.0;
        }
        if x >= 1.0 - 1e-6 {
            return 1.0;
        }

        match &self.characteristic {
            ValveCharacteristic::Linear => x,
            ValveCharacteristic::EqualPercentage { rangeability } => {
                let r = if *rangeability > 1.0 {
                    *rangeability
                } else {
                    50.0
                };
                // Normalized continuous equal-percentage curve: f(0)=0, f(1)=1
                (r.powf(x) - 1.0) / (r - 1.0)
            }
            ValveCharacteristic::QuickOpening => x.sqrt(),
            ValveCharacteristic::Custom { breakpoints } => {
                if breakpoints.is_empty() {
                    return x;
                }
                if breakpoints.len() == 1 {
                    return breakpoints[0].1.clamp(0.0, 1.0);
                }

                // If x is before first point
                if x <= breakpoints[0].0 {
                    return breakpoints[0].1.clamp(0.0, 1.0);
                }

                // Search interval
                for i in 1..breakpoints.len() {
                    let (x0, y0) = breakpoints[i - 1];
                    let (x1, y1) = breakpoints[i];
                    if x <= x1 {
                        let dx = x1 - x0;
                        if dx.abs() <= 1e-9 {
                            return y1.clamp(0.0, 1.0);
                        }
                        let frac = (x - x0) / dx;
                        return (y0 + frac * (y1 - y0)).clamp(0.0, 1.0);
                    }
                }

                // Beyond last point
                breakpoints
                    .last()
                    .map(|(_, y)| *y)
                    .unwrap_or(1.0)
                    .clamp(0.0, 1.0)
            }
        }
    }

    /// Converts an effective flow demand $y \in [0.0, 1.0]$ into required stem position $x$ (inverse mapping).
    pub fn inverse(&self, flow_fraction: f64) -> f64 {
        let y = flow_fraction.clamp(0.0, 1.0);
        if y <= 1e-6 {
            return 0.0;
        }
        if y >= 1.0 - 1e-6 {
            return 1.0;
        }

        match &self.characteristic {
            ValveCharacteristic::Linear => y,
            ValveCharacteristic::EqualPercentage { rangeability } => {
                let r = if *rangeability > 1.0 {
                    *rangeability
                } else {
                    50.0
                };
                // y = (r^x - 1) / (r - 1) => r^x = y * (r - 1) + 1 => x = ln(y*(r-1) + 1) / ln(r)
                let arg = y * (r - 1.0) + 1.0;
                (arg.ln() / r.ln()).clamp(0.0, 1.0)
            }
            ValveCharacteristic::QuickOpening => (y * y).clamp(0.0, 1.0),
            ValveCharacteristic::Custom { breakpoints: _ } => {
                // Bisection or linear search on custom curve
                let mut best_x = y;
                let mut min_diff = f64::INFINITY;
                for step in 0..=100 {
                    let test_x = step as f64 / 100.0;
                    let test_y = self.evaluate(test_x);
                    let diff = (test_y - y).abs();
                    if diff < min_diff {
                        min_diff = diff;
                        best_x = test_x;
                    }
                }
                best_x
            }
        }
    }
}
