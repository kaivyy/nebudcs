//! Mathematical error metrics, residual formulations, and step-response evaluators.
//!
//! Provides mathematically rigorous implementations of:
//! - Absolute error & guarded relative error (preventing division-by-zero on near-zero references)
//! - Mean Absolute Error (MAE) and Root Mean Squared Error (RMSE)
//! - Normalized Root Mean Squared Error (NRMSE)
//! - Maximum Absolute Deviation & Bias
//! - Integral Absolute Error (IAE)
//! - Dynamic step response characteristics (Rise time, Settling time, Overshoot, Steady-State error)
//! - Conservation balance residuals and thermodynamic deviations

use serde::{Deserialize, Serialize};

/// Computes absolute error: $|y_{\text{calc}} - y_{\text{ref}}|$.
#[inline]
pub fn absolute_error(calc: f64, ref_val: f64) -> f64 {
    (calc - ref_val).abs()
}

/// Computes relative error: $\frac{|y_{\text{calc}} - y_{\text{ref}}|}{|y_{\text{ref}}|}$.
///
/// If $|y_{\text{ref}}| < 1.0 \times 10^{-6}$, falls back to absolute error to prevent numerical singularity.
#[inline]
pub fn relative_error(calc: f64, ref_val: f64) -> f64 {
    let abs_err = (calc - ref_val).abs();
    if ref_val.abs() > 1.0e-6 {
        abs_err / ref_val.abs()
    } else {
        abs_err
    }
}

/// Computes Mean Absolute Error (MAE): $\frac{1}{N} \sum_{i=1}^N |y_{\text{calc},i} - y_{\text{ref},i}|$.
pub fn mae(calc: &[f64], ref_val: &[f64]) -> f64 {
    if calc.is_empty() || calc.len() != ref_val.len() {
        return 0.0;
    }
    let sum: f64 = calc
        .iter()
        .zip(ref_val.iter())
        .map(|(&c, &r)| (c - r).abs())
        .sum();
    sum / calc.len() as f64
}

/// Computes Root Mean Squared Error (RMSE): $\sqrt{\frac{1}{N} \sum_{i=1}^N (y_{\text{calc},i} - y_{\text{ref},i})^2}$.
pub fn rmse(calc: &[f64], ref_val: &[f64]) -> f64 {
    if calc.is_empty() || calc.len() != ref_val.len() {
        return 0.0;
    }
    let sum_sq: f64 = calc
        .iter()
        .zip(ref_val.iter())
        .map(|(&c, &r)| {
            let diff = c - r;
            diff * diff
        })
        .sum();
    (sum_sq / calc.len() as f64).sqrt()
}

/// Computes Normalized Root Mean Squared Error (NRMSE): $\frac{\text{RMSE}}{y_{\max} - y_{\min}}$.
pub fn nrmse(calc: &[f64], ref_val: &[f64], range: f64) -> f64 {
    let r = rmse(calc, ref_val);
    if range.abs() > 1.0e-12 {
        r / range.abs()
    } else {
        r
    }
}

/// Computes Maximum Absolute Deviation: $\max_i |y_{\text{calc},i} - y_{\text{ref},i}|$.
pub fn max_absolute_deviation(calc: &[f64], ref_val: &[f64]) -> f64 {
    calc.iter()
        .zip(ref_val.iter())
        .map(|(&c, &r)| (c - r).abs())
        .fold(0.0, f64::max)
}

/// Computes average bias (mean error): $\frac{1}{N} \sum_{i=1}^N (y_{\text{calc},i} - y_{\text{ref},i})$.
pub fn bias(calc: &[f64], ref_val: &[f64]) -> f64 {
    if calc.is_empty() || calc.len() != ref_val.len() {
        return 0.0;
    }
    let sum: f64 = calc
        .iter()
        .zip(ref_val.iter())
        .map(|(&c, &r)| c - r)
        .sum();
    sum / calc.len() as f64
}

/// Computes Integral Absolute Error (IAE): $\int |e(t)| dt \approx \sum |e_i| \Delta t_i$.
pub fn integral_absolute_error(dt: f64, errors: &[f64]) -> f64 {
    errors.iter().map(|&e| e.abs() * dt).sum()
}

/// Dynamic step-response performance metrics for control loops.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepResponseMetrics {
    /// Time in seconds to rise from 10% to 90% of final setpoint change.
    pub rise_time_s: f64,
    /// Maximum overshoot as a fraction of step magnitude ($> 0$).
    pub overshoot_fraction: f64,
    /// Time in seconds until response remains within $\pm 2\%$ of setpoint.
    pub settling_time_s: f64,
    /// Asymptotic steady-state error: $|y_{\text{final}} - \text{SP}|$.
    pub steady_state_error: f64,
}

/// Evaluates dynamic step response from time-series trajectory.
pub fn evaluate_step_response(
    times: &[f64],
    values: &[f64],
    initial_val: f64,
    setpoint: f64,
    settling_band_pct: f64,
) -> StepResponseMetrics {
    let step_mag = setpoint - initial_val;
    if step_mag.abs() < 1.0e-9 || times.len() < 2 {
        return StepResponseMetrics {
            rise_time_s: 0.0,
            overshoot_fraction: 0.0,
            settling_time_s: 0.0,
            steady_state_error: (values.last().copied().unwrap_or(0.0) - setpoint).abs(),
        };
    }

    let y_10 = initial_val + 0.10 * step_mag;
    let y_90 = initial_val + 0.90 * step_mag;

    let mut t_10 = times[0];
    let mut t_90 = times[times.len() - 1];
    let mut found_10 = false;
    let mut found_90 = false;

    for (&t, &y) in times.iter().zip(values.iter()) {
        if !found_10 && ((step_mag > 0.0 && y >= y_10) || (step_mag < 0.0 && y <= y_10)) {
            t_10 = t;
            found_10 = true;
        }
        if !found_90 && ((step_mag > 0.0 && y >= y_90) || (step_mag < 0.0 && y <= y_90)) {
            t_90 = t;
            found_90 = true;
        }
    }
    let rise_time_s = (t_90 - t_10).max(0.0);

    // Peak overshoot
    let peak_val = if step_mag > 0.0 {
        values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b))
    } else {
        values.iter().fold(f64::INFINITY, |a, &b| a.min(b))
    };

    let overshoot_fraction = if step_mag > 0.0 {
        ((peak_val - setpoint) / step_mag).max(0.0)
    } else {
        ((setpoint - peak_val) / step_mag.abs()).max(0.0)
    };

    // Settling time (2% band by default)
    let band = (settling_band_pct / 100.0) * step_mag.abs();
    let mut settling_time_s = times[times.len() - 1];
    for (i, (&t, &y)) in times.iter().zip(values.iter()).enumerate().rev() {
        if (y - setpoint).abs() > band {
            if i + 1 < times.len() {
                settling_time_s = times[i + 1];
            } else {
                settling_time_s = t;
            }
            break;
        }
    }

    let final_val = values.last().copied().unwrap_or(0.0);
    let steady_state_error = (final_val - setpoint).abs();

    StepResponseMetrics {
        rise_time_s,
        overshoot_fraction,
        settling_time_s,
        steady_state_error,
    }
}

/// Conservation residual: $\text{Residual} = |\dot{m}_{\text{in}} - \dot{m}_{\text{out}} - \frac{dm}{dt}|$.
#[inline]
pub fn conservation_residual(inflow: f64, outflow: f64, accumulation: f64) -> f64 {
    (inflow - outflow - accumulation).abs()
}
