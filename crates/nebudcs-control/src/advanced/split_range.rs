//! Split-range controller block.
//!
//! Divides a single master controller output demand ($0\text{--}100\%$) across multiple
//! final control elements (e.g. heating and cooling valves, or small trim and large bypass valves).

use serde::{Deserialize, Serialize};

/// A single output segment in a split-range controller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SplitRangeSegment {
    /// Segment identifier or tag name.
    pub name: String,
    /// Master input lower breakpoint ($X_1$).
    pub x_start: f64,
    /// Master input upper breakpoint ($X_2$).
    pub x_end: f64,
    /// Actuator output command at $X_1$ ($Y_1$).
    pub y_start: f64,
    /// Actuator output command at $X_2$ ($Y_2$).
    pub y_end: f64,
}

impl SplitRangeSegment {
    /// Creates a segment mapping master range $[x_1, x_2]$ to actuator range $[y_1, y_2]$.
    pub fn new(
        name: impl Into<String>,
        x_start: f64,
        x_end: f64,
        y_start: f64,
        y_end: f64,
    ) -> Self {
        Self {
            name: name.into(),
            x_start,
            x_end,
            y_start,
            y_end,
        }
    }

    /// Evaluates the output for a given master input demand.
    pub fn evaluate(&self, master_input: f64) -> f64 {
        if !master_input.is_finite() {
            return self.y_start;
        }

        let dx = self.x_end - self.x_start;
        if dx.abs() <= 1e-9 {
            return if master_input >= self.x_start {
                self.y_end
            } else {
                self.y_start
            };
        }

        if master_input <= self.x_start {
            self.y_start
        } else if master_input >= self.x_end {
            self.y_end
        } else {
            let fraction = (master_input - self.x_start) / dx;
            self.y_start + fraction * (self.y_end - self.y_start)
        }
    }
}

/// Multi-actuator split-range controller block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SplitRangeController {
    /// Ordered collection of split-range segments.
    pub segments: Vec<SplitRangeSegment>,
}

impl SplitRangeController {
    /// Creates an empty split-range controller.
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    /// Creates a standard 2-valve split (e.g. heating 0-50% and cooling 50-100%).
    pub fn standard_two_valve(name_a: &str, name_b: &str) -> Self {
        Self {
            segments: vec![
                SplitRangeSegment::new(name_a, 0.0, 50.0, 0.0, 100.0),
                SplitRangeSegment::new(name_b, 50.0, 100.0, 0.0, 100.0),
            ],
        }
    }

    /// Creates a 2-valve complementary split (e.g. Valve A open 0-50%, Valve B open 50-100%).
    pub fn standard_complementary(name_a: &str, name_b: &str) -> Self {
        Self {
            segments: vec![
                SplitRangeSegment::new(name_a, 0.0, 50.0, 100.0, 0.0),
                SplitRangeSegment::new(name_b, 50.0, 100.0, 0.0, 100.0),
            ],
        }
    }

    /// Adds a custom segment to the split range configuration.
    pub fn add_segment(&mut self, segment: SplitRangeSegment) {
        self.segments.push(segment);
    }

    /// Evaluates all split outputs for a master demand input.
    ///
    /// Returns a vector of `(segment_name, output_value)`.
    pub fn evaluate_all(&self, master_input: f64) -> Vec<(String, f64)> {
        self.segments
            .iter()
            .map(|s| (s.name.clone(), s.evaluate(master_input)))
            .collect()
    }

    /// Evaluates the output for a specific segment by name.
    pub fn evaluate(&self, name: &str, master_input: f64) -> Option<f64> {
        self.segments
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.evaluate(master_input))
    }
}

impl Default for SplitRangeController {
    fn default() -> Self {
        Self::new()
    }
}
