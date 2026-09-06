//! Performance curve modeling and interpolation for custom equipment.
//!
//! Provides data-driven 1D and 2D curves (pump head-flow curves, valve Cv
//! characteristics, compressor maps, heat exchanger UA vs flow, etc.)
//! with monotonic validation, endpoint clamping, and non-finite value protection.

use serde::{Deserialize, Serialize};

use crate::error::{ScriptingError, ScriptingResult};

/// Interpolation strategy for 1-D performance curves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InterpolationMethod {
    #[default]
    Linear,
    Step,
    MonotonicCubic,
}

/// Extrapolation handling beyond the defined curve bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExtrapolationMode {
    /// Safely clamp to the nearest endpoint value (industrial default).
    #[default]
    Clamp,
    /// Return an evaluation error if input is out of bounds.
    Error,
    /// Extend the slope of the nearest boundary interval linearly.
    Linear,
}

/// A 1-D performance curve $y = f(x)$.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Curve1D {
    pub name: String,
    pub input_var: String,
    pub output_var: String,
    /// Ordered control points (x, y). Must have at least 2 points.
    pub points: Vec<(f64, f64)>,
    #[serde(default)]
    pub interpolation: InterpolationMethod,
    #[serde(default)]
    pub extrapolation: ExtrapolationMode,
}

impl Curve1D {
    /// Creates a new 1D curve.
    pub fn new(
        name: impl Into<String>,
        input_var: impl Into<String>,
        output_var: impl Into<String>,
        points: Vec<(f64, f64)>,
    ) -> Self {
        Self {
            name: name.into(),
            input_var: input_var.into(),
            output_var: output_var.into(),
            points,
            interpolation: InterpolationMethod::Linear,
            extrapolation: ExtrapolationMode::Clamp,
        }
    }

    /// Validates the curve data integrity.
    pub fn validate(&self) -> ScriptingResult<()> {
        if self.points.len() < 2 {
            return Err(ScriptingError::Curve(format!(
                "Curve '{}' must have at least 2 points, found {}",
                self.name,
                self.points.len()
            )));
        }

        for (i, (x, y)) in self.points.iter().enumerate() {
            if !x.is_finite() || !y.is_finite() {
                return Err(ScriptingError::Curve(format!(
                    "Curve '{}' point {} contains non-finite value: ({x}, {y})",
                    self.name, i
                )));
            }
        }

        for i in 1..self.points.len() {
            let prev_x = self.points[i - 1].0;
            let curr_x = self.points[i].0;
            if curr_x <= prev_x {
                return Err(ScriptingError::Curve(format!(
                    "Curve '{}' x-coordinates must be strictly increasing: point {} has x={}, point {} has x={}",
                    self.name,
                    i - 1,
                    prev_x,
                    i,
                    curr_x
                )));
            }
        }

        Ok(())
    }

    /// Evaluates the curve at input value $x$.
    pub fn evaluate(&self, x: f64) -> ScriptingResult<f64> {
        if !x.is_finite() {
            return Err(ScriptingError::Curve(format!(
                "Cannot evaluate curve '{}' with non-finite input: {x}",
                self.name
            )));
        }

        if self.points.len() < 2 {
            return Err(ScriptingError::Curve(format!(
                "Curve '{}' has insufficient points",
                self.name
            )));
        }

        let first = self.points[0];
        let last = self.points[self.points.len() - 1];

        // Extrapolation check
        if x < first.0 {
            return match self.extrapolation {
                ExtrapolationMode::Clamp => Ok(first.1),
                ExtrapolationMode::Error => Err(ScriptingError::Curve(format!(
                    "Input {x} is below curve '{}' lower bound {}",
                    self.name, first.0
                ))),
                ExtrapolationMode::Linear => {
                    let next = self.points[1];
                    let slope = (next.1 - first.1) / (next.0 - first.0);
                    Ok(first.1 + slope * (x - first.0))
                }
            };
        }

        if x > last.0 {
            return match self.extrapolation {
                ExtrapolationMode::Clamp => Ok(last.1),
                ExtrapolationMode::Error => Err(ScriptingError::Curve(format!(
                    "Input {x} is above curve '{}' upper bound {}",
                    self.name, last.0
                ))),
                ExtrapolationMode::Linear => {
                    let prev = self.points[self.points.len() - 2];
                    let slope = (last.1 - prev.1) / (last.0 - prev.0);
                    Ok(last.1 + slope * (x - last.0))
                }
            };
        }

        // Binary search for enclosing interval [i, i+1]
        let mut low = 0;
        let mut high = self.points.len() - 1;
        while low + 1 < high {
            let mid = (low + high) / 2;
            if self.points[mid].0 <= x {
                low = mid;
            } else {
                high = mid;
            }
        }

        let (x0, y0) = self.points[low];
        let (x1, y1) = self.points[high];

        if (x1 - x0).abs() < 1e-12 {
            return Ok(y0);
        }

        let t = (x - x0) / (x1 - x0);

        match self.interpolation {
            InterpolationMethod::Step => Ok(y0),
            InterpolationMethod::Linear => Ok(y0 + t * (y1 - y0)),
            InterpolationMethod::MonotonicCubic => {
                // Fritsch-Carlson monotonic cubic interpolation
                let delta = (y1 - y0) / (x1 - x0);

                let m0 = if low > 0 {
                    let (x_prev, y_prev) = self.points[low - 1];
                    let delta_prev = (y0 - y_prev) / (x0 - x_prev);
                    0.5 * (delta_prev + delta)
                } else {
                    delta
                };

                let m1 = if high + 1 < self.points.len() {
                    let (x_next, y_next) = self.points[high + 1];
                    let delta_next = (y_next - y1) / (x_next - x1);
                    0.5 * (delta + delta_next)
                } else {
                    delta
                };

                // Clamp slopes to ensure monotonicity if delta == 0
                let (m0_clamped, m1_clamped) = if delta.abs() < 1e-12 {
                    (0.0, 0.0)
                } else {
                    (m0, m1)
                };

                let h = x1 - x0;
                let t2 = t * t;
                let t3 = t2 * t;

                // Hermite basis functions
                let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
                let h10 = t3 - 2.0 * t2 + t;
                let h01 = -2.0 * t3 + 3.0 * t2;
                let h11 = t3 - t2;

                Ok(h00 * y0 + h10 * h * m0_clamped + h01 * y1 + h11 * h * m1_clamped)
            }
        }
    }
}

/// A 2-D surface performance curve $z = f(x, y)$ (e.g. Compressor speed & flow -> pressure ratio).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Curve2D {
    pub name: String,
    pub input_x_var: String,
    pub input_y_var: String,
    pub output_var: String,
    /// X grid coordinates (must be strictly increasing).
    pub x_grid: Vec<f64>,
    /// Y grid coordinates (must be strictly increasing).
    pub y_grid: Vec<f64>,
    /// Z matrix indexed as [x_idx][y_idx].
    pub z_values: Vec<Vec<f64>>,
    #[serde(default)]
    pub extrapolation: ExtrapolationMode,
}

impl Curve2D {
    /// Validates the 2D curve dimensions and ordering.
    pub fn validate(&self) -> ScriptingResult<()> {
        if self.x_grid.len() < 2 || self.y_grid.len() < 2 {
            return Err(ScriptingError::Curve(format!(
                "Curve2D '{}' grid must be at least 2x2, found {}x{}",
                self.name,
                self.x_grid.len(),
                self.y_grid.len()
            )));
        }

        for (i, &x) in self.x_grid.iter().enumerate() {
            if !x.is_finite() {
                return Err(ScriptingError::Curve(format!(
                    "Curve2D '{}' x_grid[{i}] is non-finite: {x}",
                    self.name
                )));
            }
        }
        for i in 1..self.x_grid.len() {
            if self.x_grid[i] <= self.x_grid[i - 1] {
                return Err(ScriptingError::Curve(format!(
                    "Curve2D '{}' x_grid must be strictly increasing",
                    self.name
                )));
            }
        }

        for (j, &y) in self.y_grid.iter().enumerate() {
            if !y.is_finite() {
                return Err(ScriptingError::Curve(format!(
                    "Curve2D '{}' y_grid[{j}] is non-finite: {y}",
                    self.name
                )));
            }
        }
        for j in 1..self.y_grid.len() {
            if self.y_grid[j] <= self.y_grid[j - 1] {
                return Err(ScriptingError::Curve(format!(
                    "Curve2D '{}' y_grid must be strictly increasing",
                    self.name
                )));
            }
        }

        if self.z_values.len() != self.x_grid.len() {
            return Err(ScriptingError::Curve(format!(
                "Curve2D '{}' z_values row count {} does not match x_grid length {}",
                self.name,
                self.z_values.len(),
                self.x_grid.len()
            )));
        }

        for (i, row) in self.z_values.iter().enumerate() {
            if row.len() != self.y_grid.len() {
                return Err(ScriptingError::Curve(format!(
                    "Curve2D '{}' z_values[{i}] column count {} does not match y_grid length {}",
                    self.name,
                    row.len(),
                    self.y_grid.len()
                )));
            }
            for (j, &z) in row.iter().enumerate() {
                if !z.is_finite() {
                    return Err(ScriptingError::Curve(format!(
                        "Curve2D '{}' z_values[{i}][{j}] is non-finite: {z}",
                        self.name
                    )));
                }
            }
        }

        Ok(())
    }

    /// Evaluates the 2D surface at $(x, y)$ using bilinear interpolation.
    pub fn evaluate(&self, x: f64, y: f64) -> ScriptingResult<f64> {
        if !x.is_finite() || !y.is_finite() {
            return Err(ScriptingError::Curve(format!(
                "Cannot evaluate Curve2D '{}' with non-finite coordinates: ({x}, {y})",
                self.name
            )));
        }

        let clamped_x = x.clamp(self.x_grid[0], self.x_grid[self.x_grid.len() - 1]);
        let clamped_y = y.clamp(self.y_grid[0], self.y_grid[self.y_grid.len() - 1]);

        if self.extrapolation == ExtrapolationMode::Error && (x != clamped_x || y != clamped_y) {
            return Err(ScriptingError::Curve(format!(
                "Coordinates ({x}, {y}) out of bounds for Curve2D '{}'",
                self.name
            )));
        }

        let mut i = 0;
        while i + 2 < self.x_grid.len() && self.x_grid[i + 1] <= clamped_x {
            i += 1;
        }

        let mut j = 0;
        while j + 2 < self.y_grid.len() && self.y_grid[j + 1] <= clamped_y {
            j += 1;
        }

        let x0 = self.x_grid[i];
        let x1 = self.x_grid[i + 1];
        let y0 = self.y_grid[j];
        let y1 = self.y_grid[j + 1];

        let tx = if (x1 - x0).abs() > 1e-12 {
            (clamped_x - x0) / (x1 - x0)
        } else {
            0.0
        };
        let ty = if (y1 - y0).abs() > 1e-12 {
            (clamped_y - y0) / (y1 - y0)
        } else {
            0.0
        };

        let z00 = self.z_values[i][j];
        let z01 = self.z_values[i][j + 1];
        let z10 = self.z_values[i + 1][j];
        let z11 = self.z_values[i + 1][j + 1];

        let z0 = z00 * (1.0 - ty) + z01 * ty;
        let z1 = z10 * (1.0 - ty) + z11 * ty;

        Ok(z0 * (1.0 - tx) + z1 * tx)
    }
}
