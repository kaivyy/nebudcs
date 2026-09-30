//! Hydraulic network edge representation and pressure-drop / flow evaluation.
//!
//! Connects two [`super::node::HydraulicNode`] instances and models physical flow
//! resistances (pipes, fittings, control valves, check valves) and head sources (pumps, compressors).

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

use nebudcs_units::{Density, DifferentialPressure, MassFlow, Pressure};

use crate::equipment::STANDARD_GRAVITY;
use crate::error::PhysicsError;
use crate::hydraulics::{
    churchill_friction_factor, ControlValveHydraulics, DELTA_P_REGULARIZATION_THRESHOLD,
};

/// Type of physical flow component situated on an edge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EdgeComponent {
    /// Straight pipe with Darcy-Weisbach friction and minor fitting losses.
    Pipe {
        length: f64,
        diameter: f64,
        roughness: f64,
        loss_k: f64,
    },
    /// Throttling control valve or restriction orifice with rated $C_v$.
    ControlValve {
        valve: ControlValveHydraulics,
        opening: f64,
    },
    /// Centrifugal pump head source with quadratic head curve $H(Q) = a_0 + a_2 Q^2$.
    Pump {
        shutoff_delta_p: f64,
        flow_coeff: f64,
        speed_fraction: f64,
    },
    /// Non-return check valve allowing flow only from source to destination.
    CheckValve { forward_loss_k: f64, is_open: bool },
    /// Generic equipment resistance (e.g. filter, strainer, exchanger tube bundle).
    EquipmentResistance {
        nominal_delta_p: f64,
        nominal_flow: f64,
    },
    /// Combined pipe and in-line control valve.
    PipingWithValve {
        length: f64,
        diameter: f64,
        roughness: f64,
        loss_k: f64,
        valve: ControlValveHydraulics,
        opening: f64,
    },
}

/// A branch / edge in the hydraulic network.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HydraulicEdge {
    /// Unique edge identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// ID of upstream (from) node.
    pub from_node: String,
    /// ID of downstream (to) node.
    pub to_node: String,
    /// Hydraulic component characteristics on this edge.
    pub component: EdgeComponent,
    /// Last computed mass flow in kg/s (positive = from -> to; negative = reverse).
    pub mass_flow: MassFlow,
    /// Last computed pressure differential $P_{from} - P_{to}$ in Pascal.
    pub pressure_drop: DifferentialPressure,
    /// Average fluid linear velocity in m/s.
    pub velocity: f64,
    /// Reynolds number $Re = \frac{\rho v D}{\mu}$.
    pub reynolds: f64,
    /// Darcy friction factor $f$.
    pub friction_factor: f64,
}

impl HydraulicEdge {
    /// Creates a new pipe edge.
    #[allow(clippy::too_many_arguments)]
    pub fn new_pipe(
        id: impl Into<String>,
        name: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
        length: f64,
        diameter: f64,
        roughness: f64,
        loss_k: f64,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            from_node: from.into(),
            to_node: to.into(),
            component: EdgeComponent::Pipe {
                length: length.max(0.1),
                diameter: diameter.max(0.005),
                roughness: roughness.max(1e-6),
                loss_k: loss_k.max(0.0),
            },
            mass_flow: MassFlow::new_unchecked(0.0),
            pressure_drop: DifferentialPressure::new_unchecked(0.0),
            velocity: 0.0,
            reynolds: 0.0,
            friction_factor: 0.02,
        }
    }

    /// Creates a control valve edge.
    pub fn new_valve(
        id: impl Into<String>,
        name: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
        rated_cv: f64,
        opening: f64,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            from_node: from.into(),
            to_node: to.into(),
            component: EdgeComponent::ControlValve {
                valve: ControlValveHydraulics::new(rated_cv),
                opening: opening.clamp(0.0, 1.0),
            },
            mass_flow: MassFlow::new_unchecked(0.0),
            pressure_drop: DifferentialPressure::new_unchecked(0.0),
            velocity: 0.0,
            reynolds: 0.0,
            friction_factor: 0.0,
        }
    }

    /// Creates a pump edge.
    pub fn new_pump(
        id: impl Into<String>,
        name: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
        shutoff_delta_p: f64,
        flow_coeff: f64,
        speed_fraction: f64,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            from_node: from.into(),
            to_node: to.into(),
            component: EdgeComponent::Pump {
                shutoff_delta_p: shutoff_delta_p.max(0.0),
                flow_coeff,
                speed_fraction: speed_fraction.clamp(0.0, 2.0),
            },
            mass_flow: MassFlow::new_unchecked(0.0),
            pressure_drop: DifferentialPressure::new_unchecked(0.0),
            velocity: 0.0,
            reynolds: 0.0,
            friction_factor: 0.0,
        }
    }

    /// Creates a combined piping and in-line control valve edge.
    #[allow(clippy::too_many_arguments)]
    pub fn new_piping_with_valve(
        id: impl Into<String>,
        name: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
        length: f64,
        diameter: f64,
        roughness: f64,
        loss_k: f64,
        rated_cv: f64,
        opening: f64,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            from_node: from.into(),
            to_node: to.into(),
            component: EdgeComponent::PipingWithValve {
                length: length.max(0.1),
                diameter: diameter.max(0.005),
                roughness: roughness.max(1e-6),
                loss_k: loss_k.max(0.0),
                valve: ControlValveHydraulics::new(rated_cv),
                opening: opening.clamp(0.0, 1.0),
            },
            mass_flow: MassFlow::new_unchecked(0.0),
            pressure_drop: DifferentialPressure::new_unchecked(0.0),
            velocity: 0.0,
            reynolds: 0.0,
            friction_factor: 0.02,
        }
    }

    /// Evaluates Churchill friction factor for given Reynolds number and pipe geometry.
    pub fn churchill_friction_factor(&self, reynolds: f64, roughness: f64, diameter: f64) -> f64 {
        let rel_roughness = roughness / diameter.max(0.001);
        churchill_friction_factor(reynolds, rel_roughness)
    }

    /// Classifies flow regime based on Reynolds number.
    pub fn diagnose_regime(&self, reynolds: f64) -> crate::network::FlowRegime {
        if reynolds < 2000.0 {
            crate::network::FlowRegime::Laminar
        } else if reynolds <= 4000.0 {
            crate::network::FlowRegime::Transitional
        } else {
            crate::network::FlowRegime::Turbulent
        }
    }

    /// Calculates mass flow (kg/s) across this edge given node pressures and elevations.
    ///
    /// Positive flow means fluid moves from `from_node` to `to_node`.
    pub fn calculate_flow(
        &self,
        p_from: Pressure,
        p_to: Pressure,
        z_from: f64,
        z_to: f64,
        density: Density,
        dynamic_viscosity: f64,
    ) -> Result<f64, PhysicsError> {
        let rho = density.value().max(1.0);
        let mu = dynamic_viscosity.max(1e-6);
        let delta_z = z_to - z_from;
        let p_elev = rho * STANDARD_GRAVITY * delta_z;

        match &self.component {
            EdgeComponent::ControlValve { valve, opening } => {
                let p1 = p_from.value();
                let p2 = p_to.value() + p_elev;
                let flow = valve.calculate_mass_flow(
                    Pressure::new_unchecked(p1),
                    Pressure::new_unchecked(p2),
                    density,
                    *opening,
                )?;
                Ok(flow.value())
            }
            EdgeComponent::Pipe {
                length,
                diameter,
                roughness,
                loss_k,
            } => {
                // Effective driving pressure difference
                let delta_p_drive = (p_from.value() - p_to.value()) - p_elev;
                let abs_dp = delta_p_drive.abs();
                let sign = if delta_p_drive >= 0.0 { 1.0 } else { -1.0 };

                let d = *diameter;
                let l = *length;
                let k = *loss_k;
                let area = 0.25 * PI * d * d;

                // Initial estimate of friction factor
                let rel_roughness = roughness / d;
                let mut f = churchill_friction_factor(10_000.0, rel_roughness);

                // Resistance factor: R = f * (L/D) + K
                let mut r_loss = (f * (l / d) + k).max(1e-4);

                // Velocity from Darcy-Weisbach: v = sqrt(2 * dp / (rho * R))
                let vel_mag = if abs_dp >= DELTA_P_REGULARIZATION_THRESHOLD {
                    (2.0 * abs_dp / (rho * r_loss)).sqrt()
                } else {
                    let sqrt_thresh =
                        (2.0 * DELTA_P_REGULARIZATION_THRESHOLD / (rho * r_loss)).sqrt();
                    (abs_dp / DELTA_P_REGULARIZATION_THRESHOLD) * sqrt_thresh
                };

                // One Newton-type refinement on friction factor if in turbulent regime
                if vel_mag > 1e-4 {
                    let re = (rho * vel_mag * d) / mu;
                    f = churchill_friction_factor(re, rel_roughness);
                    r_loss = (f * (l / d) + k).max(1e-4);
                }

                let vel_refined = if abs_dp >= DELTA_P_REGULARIZATION_THRESHOLD {
                    (2.0 * abs_dp / (rho * r_loss)).sqrt()
                } else {
                    let sqrt_thresh =
                        (2.0 * DELTA_P_REGULARIZATION_THRESHOLD / (rho * r_loss)).sqrt();
                    (abs_dp / DELTA_P_REGULARIZATION_THRESHOLD) * sqrt_thresh
                };

                let m_dot = sign * rho * area * vel_refined;
                Ok(m_dot)
            }
            EdgeComponent::Pump {
                shutoff_delta_p,
                flow_coeff,
                speed_fraction,
            } => {
                // Pump head curve: Delta P_pump = omega^2 * shutoff_delta_p - a_2 * m_dot^2
                let omega = speed_fraction.clamp(0.0, 2.0);
                let head_boost = omega * omega * shutoff_delta_p;

                // Driving pressure across pump: (P_from - P_to) + head_boost - p_elev
                let delta_p_eff = (p_from.value() - p_to.value()) + head_boost - p_elev;
                let a2 = flow_coeff.abs().max(1.0);

                if delta_p_eff >= DELTA_P_REGULARIZATION_THRESHOLD {
                    let m_dot = (delta_p_eff / a2).sqrt();
                    Ok(m_dot)
                } else if delta_p_eff > 0.0 {
                    let sqrt_thresh = (DELTA_P_REGULARIZATION_THRESHOLD / a2).sqrt();
                    let m_dot = (delta_p_eff / DELTA_P_REGULARIZATION_THRESHOLD) * sqrt_thresh;
                    Ok(m_dot)
                } else {
                    // Small negative slope to maintain non-singular Jacobian when backpressure exceeds shutoff head
                    let slope = (DELTA_P_REGULARIZATION_THRESHOLD / a2).sqrt()
                        / DELTA_P_REGULARIZATION_THRESHOLD;
                    Ok(delta_p_eff * slope * 0.001)
                }
            }
            EdgeComponent::CheckValve {
                forward_loss_k,
                is_open,
            } => {
                let delta_p_drive = (p_from.value() - p_to.value()) - p_elev;
                if !*is_open || delta_p_drive <= 0.0 {
                    // Check valve blocks reverse flow with small non-singular leakage slope
                    return Ok(delta_p_drive * 1e-6);
                }
                let k = forward_loss_k.max(0.1);
                let area = 0.25 * PI * 0.1 * 0.1; // nominal 100mm seat
                let vel = (2.0 * delta_p_drive / (rho * k)).sqrt();
                Ok(rho * area * vel)
            }
            EdgeComponent::EquipmentResistance {
                nominal_delta_p,
                nominal_flow,
            } => {
                let delta_p_drive = (p_from.value() - p_to.value()) - p_elev;
                let sign = if delta_p_drive >= 0.0 { 1.0 } else { -1.0 };
                let abs_dp = delta_p_drive.abs();
                let nom_dp = nominal_delta_p.max(1.0);
                let nom_f = nominal_flow.max(1e-4);

                let k_eq = nom_dp / (nom_f * nom_f);
                let flow_mag = (abs_dp / k_eq).sqrt();
                Ok(sign * flow_mag)
            }
            EdgeComponent::PipingWithValve {
                length,
                diameter,
                roughness,
                loss_k,
                valve,
                opening,
            } => {
                let delta_p_drive = (p_from.value() - p_to.value()) - p_elev;
                let abs_dp = delta_p_drive.abs();
                let sign = if delta_p_drive >= 0.0 { 1.0 } else { -1.0 };

                let op = opening.clamp(0.0, 1.0);
                let d = *diameter;
                let l = *length;
                let k_pipe = *loss_k;
                let area = 0.25 * PI * d * d;

                // Pipe resistance: R_pipe = (f * L/D + K) / (2 * rho * A^2)
                let rel_roughness = roughness / d;
                let f = churchill_friction_factor(10_000.0, rel_roughness);
                let r_pipe = (f * (l / d) + k_pipe) / (2.0 * rho * area * area);

                // Valve resistance from Cv: R_valve = 1 / (Cv_eff^2 * rho)
                let r_valve = if op > 1e-4 {
                    let cv_eff = op * valve.rated_cv * crate::hydraulics::VALVE_CV_SI_FACTOR;
                    1.0 / (cv_eff * cv_eff * rho)
                } else {
                    1e10 // closed valve
                };

                let r_total = (r_pipe + r_valve).max(1e-4);

                let m_dot_mag = if abs_dp >= DELTA_P_REGULARIZATION_THRESHOLD {
                    (abs_dp / r_total).sqrt()
                } else {
                    let sqrt_thresh = (DELTA_P_REGULARIZATION_THRESHOLD / r_total).sqrt();
                    (abs_dp / DELTA_P_REGULARIZATION_THRESHOLD) * sqrt_thresh
                };

                Ok(sign * m_dot_mag)
            }
        }
    }

    /// Updates internal diagnostic variables for this edge after flow calculation.
    pub fn update_diagnostics(
        &mut self,
        flow: f64,
        p_from: Pressure,
        p_to: Pressure,
        density: Density,
        dynamic_viscosity: f64,
    ) {
        self.mass_flow = MassFlow::new_unchecked(flow);
        let dp = p_from.value() - p_to.value();
        self.pressure_drop = DifferentialPressure::new_unchecked(dp);

        let rho = density.value().max(1.0);
        let mu = dynamic_viscosity.max(1e-6);

        let diameter = match &self.component {
            EdgeComponent::Pipe { diameter, .. } => *diameter,
            EdgeComponent::PipingWithValve { diameter, .. } => *diameter,
            _ => 0.1, // default nominal 100mm
        };

        let area = 0.25 * PI * diameter * diameter;
        self.velocity = flow.abs() / (rho * area);
        self.reynolds = (rho * self.velocity * diameter) / mu;

        let roughness = match &self.component {
            EdgeComponent::Pipe { roughness, .. } => *roughness,
            EdgeComponent::PipingWithValve { roughness, .. } => *roughness,
            _ => 0.000045,
        };

        self.friction_factor = churchill_friction_factor(self.reynolds, roughness / diameter);
    }

    /// Sets valve opening fraction for control valve components.
    pub fn set_opening(&mut self, opening_fraction: f64) {
        match &mut self.component {
            EdgeComponent::ControlValve { opening, .. } => {
                *opening = opening_fraction.clamp(0.0, 1.0);
            }
            EdgeComponent::PipingWithValve { opening, .. } => {
                *opening = opening_fraction.clamp(0.0, 1.0);
            }
            _ => {}
        }
    }

    /// Sets pump relative rotational speed fraction.
    pub fn set_speed(&mut self, speed_frac: f64) {
        if let EdgeComponent::Pump { speed_fraction, .. } = &mut self.component {
            *speed_fraction = speed_frac.clamp(0.0, 2.0);
        }
    }
}
