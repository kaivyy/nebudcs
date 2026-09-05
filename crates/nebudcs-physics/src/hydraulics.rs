//! Hydraulic calculations: Darcy-Weisbach pipe friction, control valve $C_v$ models,
//! and bidirectional flow regularization.

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

use nebudcs_units::{Density, DifferentialPressure, MassFlow, Pressure};

use crate::error::PhysicsError;

/// Standard acceleration due to gravity ($g$) in $\text{m/s}^2$.
pub const STANDARD_GRAVITY: f64 = 9.80665;

/// SI conversion factor for standard $C_v$ (US gpm / psi^0.5):
///
/// $\dot{m} = x \cdot C_v \cdot N_v \cdot \sqrt{|\Delta P| \cdot \rho}$
pub const VALVE_CV_SI_FACTOR: f64 = 2.404_67e-5;

/// Regularization pressure threshold for smooth zero-crossing in $\Delta P$ (Pa).
pub const DELTA_P_REGULARIZATION_THRESHOLD: f64 = 1.0;

/// Calculates the Darcy-Weisbach friction factor $f$ using Churchill's (1977)
/// uniformly valid formulation.
///
/// Handles laminar, transitional, and fully turbulent flow seamlessly without
/// discontinuities or numerical singularities.
///
/// - `re`: Reynolds number $Re = \frac{\rho \cdot v \cdot D}{\mu}$
/// - `relative_roughness`: $\frac{\epsilon}{D}$ (dimensionless)
pub fn churchill_friction_factor(re: f64, relative_roughness: f64) -> f64 {
    let re = re.max(1e-4);
    let rr = relative_roughness.max(0.0);

    // Laminar term: (8 / Re)^12
    let term_laminar = (8.0 / re).powi(12);

    // Turbulent component A:
    // A = [ 2.457 * ln( 1 / ( (7 / Re)^0.9 + 0.27 * (eps/D) ) ) ]^16
    let bracket = (7.0 / re).powf(0.9) + 0.27 * rr;
    let bracket_inv = if bracket > 0.0 { 1.0 / bracket } else { 1e10 };
    let a_base = 2.457 * bracket_inv.ln();
    let term_a = a_base.powi(16);

    // Turbulent component B: (37530 / Re)^16
    let term_b = (37_530.0 / re).powi(16);

    // Combined friction factor: f = 8 * [ (8/Re)^12 + (A + B)^(-1.5) ]^(1/12)
    let sum_turb = term_a + term_b;
    let inv_turb = sum_turb.powf(-1.5);

    8.0 * (term_laminar + inv_turb).powf(1.0 / 12.0)
}

/// Calculates pipe pressure drop in Pascal using the Darcy-Weisbach equation and elevation head:
///
/// $\Delta P_{total} = \Delta P_{friction} + \Delta P_{elevation}$
///
/// where $\Delta P_{friction} = \text{sign}(\dot{m}) \cdot f \cdot \frac{L}{D} \cdot \frac{\rho v^2}{2}$
/// and $\Delta P_{elevation} = \rho \cdot g \cdot \Delta z$ with $\Delta z = z_{outlet} - z_{inlet}$.
pub fn pipe_pressure_drop(
    mass_flow: MassFlow,
    length: f64,
    inner_diameter: f64,
    roughness: f64,
    elevation_change: f64, // delta_z = z_outlet - z_inlet (m)
    density: Density,
    dynamic_viscosity: f64, // Pa·s
) -> Result<DifferentialPressure, PhysicsError> {
    if inner_diameter <= 0.0 || length <= 0.0 {
        return Err(PhysicsError::InvalidOperatingCondition {
            message: "pipe length and diameter must be positive".to_string(),
        });
    }

    let m_dot = mass_flow.value();
    let rho = density.value();

    let dp_elevation = rho * STANDARD_GRAVITY * elevation_change;

    if m_dot.abs() < 1e-12 {
        return DifferentialPressure::new(dp_elevation).map_err(|_| PhysicsError::NonFiniteValue {
            field: "pipe_elevation_pressure_drop".to_string(),
            value: dp_elevation,
        });
    }

    // Velocity: v = 4 * m_dot / (pi * D^2 * rho)
    let area = 0.25 * PI * inner_diameter * inner_diameter;
    let velocity = m_dot.abs() / (area * rho);

    // Reynolds number: Re = rho * v * D / mu
    let re = (rho * velocity * inner_diameter) / dynamic_viscosity.max(1e-6);
    let rel_roughness = roughness / inner_diameter;
    let f = churchill_friction_factor(re, rel_roughness);

    // Delta P friction = f * (L / D) * (rho * v^2 / 2)
    let dp_mag = f * (length / inner_diameter) * (0.5 * rho * velocity * velocity);
    let signed_dp_friction = if m_dot >= 0.0 { dp_mag } else { -dp_mag };
    let total_dp = signed_dp_friction + dp_elevation;

    DifferentialPressure::new(total_dp).map_err(|_| PhysicsError::NonFiniteValue {
        field: "pipe_pressure_drop".to_string(),
        value: total_dp,
    })
}

/// Hydraulic model for a generic control valve or restriction orifice.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ControlValveHydraulics {
    /// Rated valve flow coefficient $C_v$ at 100% opening.
    pub rated_cv: f64,
}

impl ControlValveHydraulics {
    /// Creates a control valve hydraulic model with rated $C_v$.
    pub fn new(rated_cv: f64) -> Self {
        Self { rated_cv }
    }

    /// Calculates mass flow rate (kg/s) across the valve for given inlet and outlet pressures.
    ///
    /// Supports bidirectional flow:
    /// - If $P_{in} > P_{out}$, flow is positive (forward).
    /// - If $P_{in} < P_{out}$, flow is negative (reverse).
    /// - Regularized around $\Delta P \approx 0$ to guarantee smooth derivatives and avoid singularities.
    pub fn calculate_mass_flow(
        &self,
        inlet_p: Pressure,
        outlet_p: Pressure,
        density: Density,
        opening_fraction: f64, // 0.0 to 1.0
    ) -> Result<MassFlow, PhysicsError> {
        let opening = opening_fraction.clamp(0.0, 1.0);
        if opening <= 1e-6 || self.rated_cv <= 0.0 {
            return Ok(MassFlow::new_unchecked(0.0));
        }

        let p1 = inlet_p.value();
        let p2 = outlet_p.value();
        let rho = density.value();

        let delta_p = p1 - p2;
        let abs_dp = delta_p.abs();

        let flow_factor = opening * self.rated_cv * VALVE_CV_SI_FACTOR;

        // Regularized square root to avoid infinite slope at Delta P = 0
        let flow_mag = if abs_dp >= DELTA_P_REGULARIZATION_THRESHOLD {
            flow_factor * (abs_dp * rho).sqrt()
        } else {
            // Linear regularization below threshold: slope = sqrt(threshold * rho) / threshold
            let sqrt_thresh = (DELTA_P_REGULARIZATION_THRESHOLD * rho).sqrt();
            flow_factor * (abs_dp / DELTA_P_REGULARIZATION_THRESHOLD) * sqrt_thresh
        };

        let signed_flow = if delta_p >= 0.0 { flow_mag } else { -flow_mag };

        MassFlow::new(signed_flow).map_err(|_| PhysicsError::NonFiniteValue {
            field: "valve_mass_flow".to_string(),
            value: signed_flow,
        })
    }

    /// Inverts the valve equation to calculate differential pressure $\Delta P = P_{in} - P_{out}$
    /// from a specified mass flow rate.
    pub fn calculate_pressure_drop(
        &self,
        mass_flow: MassFlow,
        density: Density,
        opening_fraction: f64,
    ) -> Result<DifferentialPressure, PhysicsError> {
        let opening = opening_fraction.clamp(0.0, 1.0);
        let m_dot = mass_flow.value();
        let abs_m_dot = m_dot.abs();

        if opening <= 1e-6 || self.rated_cv <= 0.0 {
            if abs_m_dot < 1e-9 {
                return Ok(DifferentialPressure::new_unchecked(0.0));
            }
            return Err(PhysicsError::InvalidOperatingCondition {
                message: "cannot achieve non-zero flow through fully closed valve".to_string(),
            });
        }

        let rho = density.value();
        let flow_factor = opening * self.rated_cv * VALVE_CV_SI_FACTOR;

        let sqrt_thresh = (DELTA_P_REGULARIZATION_THRESHOLD * rho).sqrt();
        let thresh_flow = flow_factor * sqrt_thresh;

        let abs_dp = if abs_m_dot >= thresh_flow {
            let inv_flow = abs_m_dot / flow_factor;
            (inv_flow * inv_flow) / rho
        } else {
            // Linear inversion below threshold
            (abs_m_dot / thresh_flow) * DELTA_P_REGULARIZATION_THRESHOLD
        };

        let signed_dp = if m_dot >= 0.0 { abs_dp } else { -abs_dp };

        DifferentialPressure::new(signed_dp).map_err(|_| PhysicsError::NonFiniteValue {
            field: "valve_pressure_drop".to_string(),
            value: signed_dp,
        })
    }
}
