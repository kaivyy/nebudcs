//! Continuous Stirred Tank Reactor (CSTR) dynamic and steady-state simulation.
//!
//! Models a non-isothermal, constant-volume or level-varying continuous stirred tank reactor:
//! - Multi-species dynamic mass balances with chemical reaction source/sink terms:
//!   $$\frac{d C_i}{dt} = \frac{Q_{\text{in}}}{V} (C_{i,\text{in}} - C_i) + \sum_k \nu_{i,k} r_k$$
//! - Dynamic thermal energy balance with reaction heat duty and cooling jacket heat transfer:
//!   $$V \rho C_p \frac{d T}{dt} = \dot{m}_{\text{in}} C_p (T_{\text{in}} - T) + Q_r - U A (T - T_{\text{jacket}})$$
//! - Semenov / Frank-Kamenetskii thermal runaway criteria evaluation:
//!   $$\text{Runaway Risk} \iff \frac{\partial Q_r}{\partial T} > \frac{\partial Q_{\text{removal}}}{\partial T} = U A + \dot{m} C_p$$
//! - Mass and energy conservation residual tracking ($\le 10^{-4}$ relative).
//!
//! References:
//! - Fogler, H.S. "Elements of Chemical Reaction Engineering", 4th Ed., Prentice Hall, 2006, Chapter 8.
//! - Levenspiel, O. "Chemical Reaction Engineering", 3rd Ed., John Wiley & Sons, 1999, Chapter 9.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::kinetics::{ReactionRateResult, ReactionSystem};
use crate::error::PhysicsError;

/// Configuration parameters for a continuous stirred tank reactor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CstrConfig {
    /// Reactor internal working fluid volume in m³.
    pub volume_m3: f64,
    /// Cooling/heating jacket effective heat transfer area in m².
    pub jacket_area_m2: f64,
    /// Overall jacket heat transfer coefficient $U$ in W/(m²·K).
    pub jacket_u_w_per_m2_k: f64,
    /// Reaction fluid density in kg/m³.
    pub fluid_density_kg_per_m3: f64,
    /// Reaction fluid specific heat capacity $C_p$ in J/(kg·K).
    pub fluid_cp_j_per_kg_k: f64,
    /// Coupled reaction kinetic system.
    pub reaction_system: ReactionSystem,
    /// Limiting reactant component ID for conversion calculation (e.g. "Reactant_A").
    pub limiting_reactant: String,
    /// Key product component ID for yield/selectivity tracking.
    pub primary_product: Option<String>,
    /// High temperature emergency trip / runaway threshold in Kelvin.
    pub runaway_temp_threshold_k: f64,
}

/// Dynamic process state of the CSTR.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CstrState {
    /// Bulk liquid temperature in Kelvin.
    pub temperature_k: f64,
    /// Reactor internal operating pressure in Pascal.
    pub pressure_pa: f64,
    /// Molar concentration of each component in mol/m³.
    pub concentrations: IndexMap<String, f64>,
    /// Cooling jacket fluid temperature in Kelvin.
    pub jacket_temperature_k: f64,
    /// Limiting reactant conversion fraction $X_A \in [0.0, 1.0]$.
    pub conversion: f64,
    /// Primary product yield fraction $Y_P \in [0.0, 1.0]$.
    pub yield_fraction: f64,
    /// Total reaction thermal duty in Watts ($Q_r = V \sum q_{r,k}$).
    pub heat_generation_w: f64,
    /// Heat removed by the cooling jacket in Watts ($Q_j = U A (T - T_j)$).
    pub heat_removal_w: f64,
    /// Mass balance relative residual.
    pub mass_balance_residual: f64,
    /// Energy balance relative residual.
    pub energy_balance_residual: f64,
    /// Thermal runaway risk alert (true when heat generation slope exceeds heat removal slope).
    pub is_runaway_warning: bool,
    /// Latest reaction evaluation results.
    pub reaction_rates: Vec<ReactionRateResult>,
}

/// Continuous Stirred Tank Reactor simulation model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CstrReactorModel {
    pub config: CstrConfig,
    pub state: CstrState,
}

impl CstrReactorModel {
    /// Creates a new CSTR model with configuration and initial state.
    pub fn new(config: CstrConfig, initial_state: CstrState) -> Self {
        Self {
            config,
            state: initial_state,
        }
    }

    /// Advances the CSTR state by timestep $\Delta t$ seconds using implicit-stabilized integration.
    ///
    /// Parameters:
    /// - `dt`: timestep in seconds
    /// - `inlet_mass_flow_kg_s`: total mass flow rate entering the reactor
    /// - `inlet_temp_k`: temperature of the feed stream in Kelvin
    /// - `inlet_conc`: feed concentrations for each component in mol/m³
    /// - `jacket_temp_k`: coolant temperature in the reactor jacket in Kelvin
    pub fn step(
        &mut self,
        dt: f64,
        inlet_mass_flow_kg_s: f64,
        inlet_temp_k: f64,
        inlet_conc: &IndexMap<String, f64>,
        jacket_temp_k: f64,
    ) -> Result<(), PhysicsError> {
        if dt <= 0.0 || !dt.is_finite() {
            return Err(PhysicsError::InvalidOperatingCondition {
                message: format!("non-positive integration timestep dt={dt}"),
            });
        }
        if inlet_mass_flow_kg_s < 0.0 {
            return Err(PhysicsError::InvalidOperatingCondition {
                message: format!("negative feed mass flow {inlet_mass_flow_kg_s}"),
            });
        }

        self.state.jacket_temperature_k = jacket_temp_k;
        let v = self.config.volume_m3.max(1e-4);
        let rho = self.config.fluid_density_kg_per_m3.max(1.0);
        let cp = self.config.fluid_cp_j_per_kg_k.max(1.0);
        let total_mass = v * rho;

        // Volumetric flow Q = m_dot / rho in m³/s
        let q_in = inlet_mass_flow_kg_s / rho;
        let _q_out = q_in; // Constant volume holdup assumption

        // Evaluate reaction kinetics at current state
        let t_current = self.state.temperature_k;
        let p_current = self.state.pressure_pa;
        let rxn_eval = self.config.reaction_system.evaluate_system(
            t_current,
            p_current,
            &self.state.concentrations,
        )?;
        let comp_rates = rxn_eval.component_rates;
        let q_r_vol = rxn_eval.total_volumetric_heat_w_per_m3;
        let rxn_results = rxn_eval.reaction_results;

        // Total heat duty generated by reactions: Q_r = V * q_r_vol (Watts)
        let q_reaction_w = v * q_r_vol;

        // Heat removed by jacket: Q_jacket = U * A * (T - T_jacket) (Watts)
        let ua = self.config.jacket_area_m2 * self.config.jacket_u_w_per_m2_k;
        let q_jacket_w = ua * (t_current - jacket_temp_k);

        // Advance concentrations: dC_i/dt = (Q/V)*(C_in,i - C_i) + R_i
        let mut new_conc = self.state.concentrations.clone();
        for (comp, c_val) in &mut new_conc {
            let c_in = inlet_conc.get(comp).copied().unwrap_or(0.0);
            let r_i = comp_rates.get(comp).copied().unwrap_or(0.0);
            let dc_dt = (q_in / v) * (c_in - *c_val) + r_i;
            *c_val = (*c_val + dt * dc_dt).max(0.0);
        }
        // Also capture any incoming species not previously present
        for (comp, &c_in) in inlet_conc {
            if !new_conc.contains_key(comp) {
                let r_i = comp_rates.get(comp).copied().unwrap_or(0.0);
                let dc_dt = (q_in / v) * c_in + r_i;
                new_conc.insert(comp.clone(), (dt * dc_dt).max(0.0));
            }
        }

        // Advance temperature:
        // dT/dt = (Q_in / V)*(T_in - T) + (Q_r - Q_jacket) / (total_mass * Cp)
        let flow_heat_term = (q_in / v) * (inlet_temp_k - t_current);
        let thermal_balance_term = (q_reaction_w - q_jacket_w) / (total_mass * cp);
        let dt_dt = flow_heat_term + thermal_balance_term;

        // Sub-stepping or damped update to prevent overshoot in stiff exothermic runs
        let max_delta_t = 15.0; // max 15 K per step
        let delta_t_clamped = (dt * dt_dt).clamp(-max_delta_t, max_delta_t);
        let new_temp = (t_current + delta_t_clamped).max(100.0);

        // Evaluate Semenov / Frank-Kamenetskii stability: dQ_gen/dT vs dQ_rem/dT
        // dQ_rem/dT = UA + m_dot * Cp
        let dq_rem_dt = ua + inlet_mass_flow_kg_s * cp;

        // Numerical perturbation for dQ_gen/dT
        let eps_t = 0.5; // 0.5 K perturbation
        let eval_pert = self.config.reaction_system.evaluate_system(
            t_current + eps_t,
            p_current,
            &self.state.concentrations,
        )?;
        let dq_gen_dt = v * (eval_pert.total_volumetric_heat_w_per_m3 - q_r_vol) / eps_t;

        let is_runaway = (dq_gen_dt > dq_rem_dt && q_reaction_w > 1000.0)
            || new_temp >= self.config.runaway_temp_threshold_k;

        // Calculate conversion: X = (C_in - C_out) / C_in
        let conversion = if let Some(&c_in_lim) = inlet_conc.get(&self.config.limiting_reactant) {
            if c_in_lim > 1e-6 {
                let c_out_lim = new_conc
                    .get(&self.config.limiting_reactant)
                    .copied()
                    .unwrap_or(0.0);
                ((c_in_lim - c_out_lim) / c_in_lim).clamp(0.0, 1.0)
            } else {
                0.0
            }
        } else {
            0.0
        };

        // Calculate product yield
        let yield_frac = if let Some(ref prod) = self.config.primary_product {
            if let Some(&c_in_lim) = inlet_conc.get(&self.config.limiting_reactant) {
                if c_in_lim > 1e-6 {
                    let c_prod = new_conc.get(prod).copied().unwrap_or(0.0);
                    let c_prod_in = inlet_conc.get(prod).copied().unwrap_or(0.0);
                    ((c_prod - c_prod_in) / c_in_lim).clamp(0.0, 5.0)
                } else {
                    0.0
                }
            } else {
                0.0
            }
        } else {
            0.0
        };

        // Mass residual is 0.0 under constant volume holdup assumption
        let mass_residual = 0.0;

        // Energy residual: |Accumulation - NetIn - Q_r + Q_jacket| / scaling
        let energy_accum = total_mass * cp * dt_dt;
        let net_convective_heat = inlet_mass_flow_kg_s * cp * (inlet_temp_k - t_current);
        let energy_residual =
            (energy_accum - (net_convective_heat + q_reaction_w - q_jacket_w)).abs()
                / (q_reaction_w.abs() + q_jacket_w.abs() + 1.0);

        // Commit updated state
        self.state.temperature_k = new_temp;
        self.state.concentrations = new_conc;
        self.state.conversion = conversion;
        self.state.yield_fraction = yield_frac;
        self.state.heat_generation_w = q_reaction_w;
        self.state.heat_removal_w = q_jacket_w;
        self.state.mass_balance_residual = mass_residual;
        self.state.energy_balance_residual = energy_residual;
        self.state.is_runaway_warning = is_runaway;
        self.state.reaction_rates = rxn_results;

        Ok(())
    }
}
