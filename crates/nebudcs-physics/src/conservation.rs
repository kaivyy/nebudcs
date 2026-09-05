//! Conservation laws: total mass balance, species mass balance, and energy/enthalpy balance.
//!
//! Provides deterministic balance evaluation and dynamic integration over simulation timestep $\Delta t$.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use nebudcs_units::{MassFlow, SpecificEnthalpy};

use crate::error::PhysicsError;
use crate::state::ComponentComposition;

/// Result of evaluating a steady-state or dynamic mass balance.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MassBalanceResult {
    pub total_inflow: f64,
    pub total_outflow: f64,
    pub accumulation_rate: f64,
    pub residual: f64,
}

impl MassBalanceResult {
    /// Returns the absolute balance residual $\sum \dot{m}_{in} - \sum \dot{m}_{out} - \frac{dm}{dt}$.
    pub fn residual(self) -> f64 {
        self.residual
    }

    /// Checks if the mass balance closes within the specified numerical tolerance (kg/s).
    pub fn is_conserved(self, tolerance: f64) -> bool {
        self.residual.abs() <= tolerance
    }
}

/// Generic total mass balance calculator.
pub struct MassBalance;

impl MassBalance {
    /// Evaluates mass balance across an equipment boundary.
    ///
    /// Residual $= \sum \dot{m}_{in} - \sum \dot{m}_{out} - \frac{dm}{dt}$.
    pub fn evaluate(
        inflows: &[MassFlow],
        outflows: &[MassFlow],
        accumulation_rate: f64,
    ) -> MassBalanceResult {
        let total_inflow: f64 = inflows.iter().map(|m| m.value()).sum();
        let total_outflow: f64 = outflows.iter().map(|m| m.value()).sum();
        let residual = total_inflow - total_outflow - accumulation_rate;

        MassBalanceResult {
            total_inflow,
            total_outflow,
            accumulation_rate,
            residual,
        }
    }

    /// Dynamically integrates mass holdup over a fixed simulation timestep $\Delta t$.
    ///
    /// $m_{new} = m_{old} + (\sum \dot{m}_{in} - \sum \dot{m}_{out}) \cdot \Delta t$
    pub fn accumulate(
        initial_mass: f64,
        inflows: &[MassFlow],
        outflows: &[MassFlow],
        dt: f64,
    ) -> Result<f64, PhysicsError> {
        if !initial_mass.is_finite() || initial_mass < 0.0 {
            return Err(PhysicsError::NonFiniteValue {
                field: "initial_mass".to_string(),
                value: initial_mass,
            });
        }
        if !dt.is_finite() || dt <= 0.0 {
            return Err(PhysicsError::InvalidOperatingCondition {
                message: format!("dt must be positive and finite, got {dt}"),
            });
        }

        let total_inflow: f64 = inflows.iter().map(|m| m.value()).sum();
        let total_outflow: f64 = outflows.iter().map(|m| m.value()).sum();

        let net_flow = total_inflow - total_outflow;
        let new_mass = initial_mass + net_flow * dt;

        if new_mass < 0.0 {
            // Cannot hold negative mass in a physical vessel
            Ok(0.0)
        } else {
            Ok(new_mass)
        }
    }
}

/// Generic multi-component species mass balance calculator.
pub struct SpeciesBalance;

impl SpeciesBalance {
    /// Dynamically integrates individual component masses over timestep $\Delta t$
    /// and calculates the resulting normalized mixture composition.
    pub fn accumulate(
        initial_mass: f64,
        initial_comp: &ComponentComposition,
        inflows: &[(MassFlow, ComponentComposition)],
        outflows: &[MassFlow],
        dt: f64,
    ) -> Result<(f64, ComponentComposition), PhysicsError> {
        for (_, comp) in inflows {
            if comp.basis() != initial_comp.basis() {
                return Err(PhysicsError::InvalidComposition {
                    reason: format!(
                        "inflow composition basis {:?} does not match vessel composition basis {:?}",
                        comp.basis(),
                        initial_comp.basis()
                    ),
                });
            }
        }

        let total_outflow_rate: f64 = outflows.iter().map(|m| m.value()).sum();

        let mut comp_masses: IndexMap<String, f64> = IndexMap::new();

        // 1. Initial component masses: m_i = m_total * w_i
        for (name, &frac) in initial_comp.fractions() {
            comp_masses.insert(name.clone(), initial_mass * frac);
        }

        // 2. Add inflows: m_i += (m_dot_in * w_i_in) * dt
        for (flow, comp) in inflows {
            let flow_rate = flow.value();
            for (name, &frac) in comp.fractions() {
                *comp_masses.entry(name.clone()).or_insert(0.0) += flow_rate * frac * dt;
            }
        }

        // 3. Subtract outflows (assumed well-mixed at current composition):
        //    m_i -= (m_dot_out * w_i_current) * dt
        if initial_mass > 1e-12 {
            for (name, &frac) in initial_comp.fractions() {
                if let Some(mass) = comp_masses.get_mut(name) {
                    *mass -= total_outflow_rate * frac * dt;
                    if *mass < 0.0 {
                        *mass = 0.0;
                    }
                }
            }
        }

        let new_total_mass: f64 = comp_masses.values().sum();
        if new_total_mass <= 1e-12 {
            // Holdup is empty; preserve initial composition signature
            Ok((0.0, initial_comp.clone()))
        } else {
            let new_comp = ComponentComposition::with_basis(comp_masses, initial_comp.basis())?;
            Ok((new_total_mass, new_comp))
        }
    }
}

/// Generic energy and enthalpy balance calculator.
pub struct EnergyBalance;

/// Inputs for dynamic enthalpy accumulation in a control volume.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EnthalpyAccumulationInputs {
    pub initial_mass: f64,
    pub initial_enthalpy: SpecificEnthalpy,
    pub heat_duty_watts: f64,
    pub shaft_work_watts: f64,
    pub dt: f64,
    pub new_mass: f64,
}

impl EnergyBalance {
    /// Calculates the adiabatic mixing specific enthalpy for combined inlet streams.
    ///
    /// $h_{mix} = \frac{\sum (\dot{m}_i \cdot h_i)}{\sum \dot{m}_i}$
    pub fn mix_adiabatic_streams(
        streams: &[(MassFlow, SpecificEnthalpy)],
    ) -> Result<SpecificEnthalpy, PhysicsError> {
        if streams.is_empty() {
            return Err(PhysicsError::InvalidOperatingCondition {
                message: "cannot mix zero streams for enthalpy balance".to_string(),
            });
        }

        let mut total_mass_flow = 0.0;
        let mut total_enthalpy_rate = 0.0;

        for (m, h) in streams {
            let m_val = m.value();
            let h_val = h.value();
            if m_val < 0.0 {
                return Err(PhysicsError::InvalidOperatingCondition {
                    message: format!("cannot mix negative stream flow: {m_val}"),
                });
            }
            total_mass_flow += m_val;
            total_enthalpy_rate += m_val * h_val;
        }

        if total_mass_flow <= 0.0 {
            // Zero flow fallback: average of input enthalpies
            let avg_h =
                streams.iter().map(|(_, h)| h.value()).sum::<f64>() / (streams.len() as f64);
            return SpecificEnthalpy::new(avg_h).map_err(|_| PhysicsError::NonFiniteValue {
                field: "mixed_enthalpy".to_string(),
                value: avg_h,
            });
        }

        let mixed_h = total_enthalpy_rate / total_mass_flow;
        SpecificEnthalpy::new(mixed_h).map_err(|_| PhysicsError::NonFiniteValue {
            field: "mixed_enthalpy".to_string(),
            value: mixed_h,
        })
    }

    /// Dynamically integrates total enthalpy in a control volume:
    ///
    /// $H_{new} = H_{old} + \left[ \sum (\dot{m}_{in} \cdot h_{in}) - \sum (\dot{m}_{out} \cdot h_{old}) + Q + W_s \right] \cdot \Delta t$
    ///
    /// Returns the new specific enthalpy $h_{new} = \frac{H_{new}}{m_{new}}$.
    pub fn accumulate_enthalpy(
        inputs: EnthalpyAccumulationInputs,
        inflows: &[(MassFlow, SpecificEnthalpy)],
        outflows: &[MassFlow],
    ) -> Result<SpecificEnthalpy, PhysicsError> {
        let h_old = inputs.initial_enthalpy.value();
        let total_initial_enthalpy = inputs.initial_mass * h_old;

        let inflow_rate: f64 = inflows.iter().map(|(m, h)| m.value() * h.value()).sum();
        let outflow_mass_rate: f64 = outflows.iter().map(|m| m.value()).sum();
        let outflow_rate = outflow_mass_rate * h_old;

        let net_power =
            inflow_rate - outflow_rate + inputs.heat_duty_watts + inputs.shaft_work_watts;
        let total_new_enthalpy = total_initial_enthalpy + net_power * inputs.dt;

        if inputs.new_mass <= 1e-12 {
            Ok(inputs.initial_enthalpy)
        } else {
            let h_new = total_new_enthalpy / inputs.new_mass;
            SpecificEnthalpy::new(h_new).map_err(|_| PhysicsError::NonFiniteValue {
                field: "accumulated_specific_enthalpy".to_string(),
                value: h_new,
            })
        }
    }
}
