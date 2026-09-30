//! Numerical phase envelope and saturation boundary solver.
//!
//! Provides reusable numerical algorithms for:
//! - Bubble-point pressure and temperature calculation
//! - Dew-point pressure and temperature calculation
//! - Multicomponent phase envelope boundary curve tracing
//! - Estimated cricondentherm (maximum temperature) and cricondenbar (maximum pressure)

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::cubic::{CubicEos, PhaseTarget};
use crate::error::PhysicsError;

/// Single point on a phase envelope boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhaseEnvelopePoint {
    /// Temperature in Kelvin.
    pub temperature_k: f64,
    /// Absolute pressure in Pascal.
    pub pressure_pa: f64,
    /// Liquid phase mole fractions.
    pub liquid_composition: IndexMap<String, f64>,
    /// Vapor phase mole fractions.
    pub vapor_composition: IndexMap<String, f64>,
    /// Number of numerical iterations to converge.
    pub iterations: usize,
    /// Final equilibrium residual.
    pub residual: f64,
}

/// Traced multicomponent phase envelope with bubble and dew boundary branches.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhaseEnvelope {
    /// Bubble-point boundary curve (incipient vapor formation from saturated liquid).
    pub bubble_curve: Vec<PhaseEnvelopePoint>,
    /// Dew-point boundary curve (incipient liquid condensation from saturated vapor).
    pub dew_curve: Vec<PhaseEnvelopePoint>,
    /// Estimated maximum temperature where two phases can coexist (cricondentherm) in Kelvin.
    pub cricondentherm_k: Option<f64>,
    /// Estimated maximum pressure where two phases can coexist (cricondenbar) in Pascal.
    pub cricondenbar_pa: Option<f64>,
}

/// Numerical solver for phase boundaries and envelope tracing.
pub struct PhaseEnvelopeSolver<'a> {
    eos: &'a CubicEos,
    max_iterations: usize,
    tolerance: f64,
}

impl<'a> PhaseEnvelopeSolver<'a> {
    /// Creates a new phase envelope solver with default numerical tolerances.
    pub fn new(eos: &'a CubicEos) -> Self {
        Self {
            eos,
            max_iterations: 100,
            tolerance: 1.0e-5,
        }
    }

    /// Solves for bubble-point pressure $P_{\text{bubble}}$ at specified temperature $T$.
    ///
    /// At the bubble point: $x_i = z_i$ and $\sum_i y_i = \sum_i z_i K_i(P, T) = 1.0$.
    pub fn calculate_bubble_point_pressure(
        &self,
        z: &IndexMap<String, f64>,
        t_k: f64,
    ) -> Result<PhaseEnvelopePoint, PhysicsError> {
        if t_k <= 0.0 {
            return Err(PhysicsError::NonPositiveTemperature {
                value: t_k,
                context: "calculate_bubble_point_pressure".to_string(),
            });
        }

        // Wilson initial guess for pressure: P = sum(z_i * P_ci * exp(...))
        let mut p_est = 0.0;
        let mut k_values = IndexMap::with_capacity(z.len());
        for (name, &zi) in z {
            let comp = self
                .eos
                .database()
                .get(name)
                .ok_or_else(|| PhysicsError::MissingProperty { name: name.clone() })?;
            let tc = comp.critical_temperature;
            let pc = comp.critical_pressure;
            let omega = comp.acentric_factor;
            let exp_term = (5.373_f64 * (1.0 + omega) * (1.0 - tc / t_k)).exp();
            p_est += zi * pc * exp_term;
        }

        p_est = p_est.clamp(1_000.0, 50.0e6);

        // Initial K-values
        for (name, _) in z {
            let comp = &self.eos.database().components()[name];
            let exp_term = (5.373_f64
                * (1.0 + comp.acentric_factor)
                * (1.0 - comp.critical_temperature / t_k))
                .exp();
            let k0 = (comp.critical_pressure / p_est) * exp_term;
            k_values.insert(name.clone(), k0.clamp(1e-6, 1e6));
        }

        let mut p = p_est;
        let mut last_res = 1.0;

        for iter in 1..=self.max_iterations {
            // Liquid state is feed composition z
            let liq_state = match self
                .eos
                .evaluate_phase_state(z, p, t_k, PhaseTarget::Liquid)
            {
                Ok(s) => s,
                Err(_) => {
                    // Fallback to vapor root if liquid root absent
                    self.eos
                        .evaluate_phase_state(z, p, t_k, PhaseTarget::Vapor)?
                }
            };

            // Estimate vapor composition y_i = z_i * K_i / sum(z_j * K_j)
            let mut sum_zk = 0.0;
            for (name, &zi) in z {
                sum_zk += zi * k_values[name];
            }
            if sum_zk <= 0.0 || !sum_zk.is_finite() {
                sum_zk = 1.0;
            }

            let mut y = IndexMap::with_capacity(z.len());
            for (name, &zi) in z {
                y.insert(name.clone(), (zi * k_values[name]) / sum_zk);
            }

            // Vapor state
            let vap_state = match self
                .eos
                .evaluate_phase_state(&y, p, t_k, PhaseTarget::Vapor)
            {
                Ok(s) => s,
                Err(_) => self
                    .eos
                    .evaluate_phase_state(&y, p, t_k, PhaseTarget::Liquid)?,
            };

            // Update K-values and calculate residual: res = sum(z_i * K_i) - 1.0
            let mut sum_y_new = 0.0;
            let mut max_k_change: f64 = 0.0;

            for (name, &zi) in z {
                let phi_l = liq_state
                    .fugacity_coefficients
                    .get(name)
                    .copied()
                    .unwrap_or(1.0);
                let phi_v = vap_state
                    .fugacity_coefficients
                    .get(name)
                    .copied()
                    .unwrap_or(1.0);
                let k_new = (phi_l / phi_v.max(1e-12)).clamp(1e-7, 1e7);
                let k_old = k_values[name];
                max_k_change = max_k_change.max(((k_new - k_old) / k_old).abs());
                k_values.insert(name.clone(), 0.7 * k_new + 0.3 * k_old);
                sum_y_new += zi * k_new;
            }

            let res = (sum_y_new - 1.0).abs();
            last_res = res;

            if res < self.tolerance && max_k_change < self.tolerance * 10.0 {
                return Ok(PhaseEnvelopePoint {
                    temperature_k: t_k,
                    pressure_pa: p,
                    liquid_composition: z.clone(),
                    vapor_composition: y,
                    iterations: iter,
                    residual: res,
                });
            }

            // Pressure update: P_new = P * sum(z_i * K_i)
            p = (p * sum_y_new).clamp(100.0, 100.0e6);
        }

        // Return best converged estimate
        let mut y = IndexMap::with_capacity(z.len());
        let mut sum_zk = 0.0;
        for (name, &zi) in z {
            sum_zk += zi * k_values[name];
        }
        for (name, &zi) in z {
            y.insert(name.clone(), (zi * k_values[name]) / sum_zk.max(1e-12));
        }

        Ok(PhaseEnvelopePoint {
            temperature_k: t_k,
            pressure_pa: p,
            liquid_composition: z.clone(),
            vapor_composition: y,
            iterations: self.max_iterations,
            residual: last_res,
        })
    }

    /// Solves for dew-point pressure $P_{\text{dew}}$ at specified temperature $T$.
    ///
    /// At the dew point: $y_i = z_i$ and $\sum_i x_i = \sum_i \frac{z_i}{K_i(P, T)} = 1.0$.
    pub fn calculate_dew_point_pressure(
        &self,
        z: &IndexMap<String, f64>,
        t_k: f64,
    ) -> Result<PhaseEnvelopePoint, PhysicsError> {
        if t_k <= 0.0 {
            return Err(PhysicsError::NonPositiveTemperature {
                value: t_k,
                context: "calculate_dew_point_pressure".to_string(),
            });
        }

        // Wilson initial guess for dew pressure: 1 / P = sum(z_i / (P_ci * exp(...)))
        let mut inv_p = 0.0;
        let mut k_values = IndexMap::with_capacity(z.len());
        for (name, &zi) in z {
            let comp = self
                .eos
                .database()
                .get(name)
                .ok_or_else(|| PhysicsError::MissingProperty { name: name.clone() })?;
            let tc = comp.critical_temperature;
            let pc = comp.critical_pressure;
            let omega = comp.acentric_factor;
            let exp_term = (5.373_f64 * (1.0 + omega) * (1.0 - tc / t_k)).exp();
            inv_p += zi / (pc * exp_term);
        }

        let p_est = (1.0 / inv_p.max(1e-12)).clamp(100.0, 50.0e6);

        for (name, _) in z {
            let comp = &self.eos.database().components()[name];
            let exp_term = (5.373_f64
                * (1.0 + comp.acentric_factor)
                * (1.0 - comp.critical_temperature / t_k))
                .exp();
            let k0 = (comp.critical_pressure / p_est) * exp_term;
            k_values.insert(name.clone(), k0.clamp(1e-6, 1e6));
        }

        let mut p = p_est;
        let mut last_res = 1.0;

        for iter in 1..=self.max_iterations {
            // Vapor state is feed composition z
            let vap_state = match self.eos.evaluate_phase_state(z, p, t_k, PhaseTarget::Vapor) {
                Ok(s) => s,
                Err(_) => self
                    .eos
                    .evaluate_phase_state(z, p, t_k, PhaseTarget::Liquid)?,
            };

            // Estimate liquid composition x_i = (z_i / K_i) / sum(z_j / K_j)
            let mut sum_z_over_k = 0.0;
            for (name, &zi) in z {
                sum_z_over_k += zi / k_values[name];
            }
            if sum_z_over_k <= 0.0 || !sum_z_over_k.is_finite() {
                sum_z_over_k = 1.0;
            }

            let mut x = IndexMap::with_capacity(z.len());
            for (name, &zi) in z {
                x.insert(name.clone(), (zi / k_values[name]) / sum_z_over_k);
            }

            // Liquid state
            let liq_state = match self
                .eos
                .evaluate_phase_state(&x, p, t_k, PhaseTarget::Liquid)
            {
                Ok(s) => s,
                Err(_) => self
                    .eos
                    .evaluate_phase_state(&x, p, t_k, PhaseTarget::Vapor)?,
            };

            // Update K-values
            let mut sum_x_new = 0.0;
            let mut max_k_change: f64 = 0.0;

            for (name, &zi) in z {
                let phi_l = liq_state
                    .fugacity_coefficients
                    .get(name)
                    .copied()
                    .unwrap_or(1.0);
                let phi_v = vap_state
                    .fugacity_coefficients
                    .get(name)
                    .copied()
                    .unwrap_or(1.0);
                let k_new = (phi_l / phi_v.max(1e-12)).clamp(1e-7, 1e7);
                let k_old = k_values[name];
                max_k_change = max_k_change.max(((k_new - k_old) / k_old).abs());
                k_values.insert(name.clone(), 0.7 * k_new + 0.3 * k_old);
                sum_x_new += zi / k_new;
            }

            let res = (sum_x_new - 1.0).abs();
            last_res = res;

            if res < self.tolerance && max_k_change < self.tolerance * 10.0 {
                return Ok(PhaseEnvelopePoint {
                    temperature_k: t_k,
                    pressure_pa: p,
                    liquid_composition: x,
                    vapor_composition: z.clone(),
                    iterations: iter,
                    residual: res,
                });
            }

            // Pressure update: P_new = P / sum(z_i / K_i)
            p = (p / sum_x_new.max(1e-12)).clamp(100.0, 100.0e6);
        }

        let mut x = IndexMap::with_capacity(z.len());
        let mut sum_z_over_k = 0.0;
        for (name, &zi) in z {
            sum_z_over_k += zi / k_values[name];
        }
        for (name, &zi) in z {
            x.insert(
                name.clone(),
                (zi / k_values[name]) / sum_z_over_k.max(1e-12),
            );
        }

        Ok(PhaseEnvelopePoint {
            temperature_k: t_k,
            pressure_pa: p,
            liquid_composition: x,
            vapor_composition: z.clone(),
            iterations: self.max_iterations,
            residual: last_res,
        })
    }

    /// Traces the full phase envelope across a specified temperature interval.
    pub fn trace_phase_envelope(
        &self,
        z: &IndexMap<String, f64>,
        t_min_k: f64,
        t_max_k: f64,
        n_steps: usize,
    ) -> Result<PhaseEnvelope, PhysicsError> {
        if t_min_k >= t_max_k || n_steps < 2 {
            return Err(PhysicsError::InvalidOperatingCondition {
                message: format!(
                    "invalid envelope tracing bounds: [{t_min_k}, {t_max_k}] steps={n_steps}"
                ),
            });
        }

        let step = (t_max_k - t_min_k) / (n_steps as f64 - 1.0);
        let mut bubble_pts = Vec::with_capacity(n_steps);
        let mut dew_pts = Vec::with_capacity(n_steps);

        let mut max_t = t_min_k;
        let mut max_p = 0.0;

        for i in 0..n_steps {
            let t = t_min_k + (i as f64) * step;

            if let Ok(bp) = self.calculate_bubble_point_pressure(z, t) {
                if bp.pressure_pa > max_p {
                    max_p = bp.pressure_pa;
                }
                if t > max_t {
                    max_t = t;
                }
                bubble_pts.push(bp);
            }

            if let Ok(dp) = self.calculate_dew_point_pressure(z, t) {
                if dp.pressure_pa > max_p {
                    max_p = dp.pressure_pa;
                }
                if t > max_t {
                    max_t = t;
                }
                dew_pts.push(dp);
            }
        }

        Ok(PhaseEnvelope {
            bubble_curve: bubble_pts,
            dew_curve: dew_pts,
            cricondentherm_k: Some(max_t),
            cricondenbar_pa: Some(max_p),
        })
    }
}
