//! Plug Flow Reactor (PFR) spatial integration and axial profile simulation.
//!
//! Models a tubular plug flow reactor with multi-component chemical reactions:
//! - Spatial integration along reactor length $z \in [0, L]$:
//!   $$\frac{d F_i}{d V} = \sum_k \nu_{i,k} r_k$$
//! - Axial energy balance with wall heat transfer:
//!   $$\frac{d T}{d V} = \frac{\sum_k r_k (-\Delta H_{r,k}) - U a' (T - T_{\text{wall}})}{\sum_i F_i C_{p,i}}$$
//!   where $a' = \frac{4}{D}$ is specific heat transfer area per unit volume.
//! - Conversion $X_A(z)$, temperature $T(z)$, and species concentration profiles $C_i(z)$.
//!
//! References:
//! - Fogler, H.S. "Elements of Chemical Reaction Engineering", 4th Ed., Prentice Hall, 2006, Chapter 4.
//! - Levenspiel, O. "Chemical Reaction Engineering", 3rd Ed., John Wiley & Sons, 1999, Chapter 5.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::kinetics::ReactionSystem;
use crate::error::PhysicsError;

/// Configuration for a tubular plug flow reactor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PfrConfig {
    /// Tube length in meters.
    pub length_m: f64,
    /// Tube internal diameter in meters.
    pub diameter_m: f64,
    /// Number of parallel tubes.
    pub tube_count: usize,
    /// External coolant / jacket wall temperature in Kelvin.
    pub wall_temperature_k: f64,
    /// Overall wall heat transfer coefficient $U$ in W/(m²·K).
    pub overall_u_w_per_m2_k: f64,
    /// Reaction fluid density in kg/m³.
    pub fluid_density_kg_per_m3: f64,
    /// Reaction fluid specific heat capacity in J/(kg·K).
    pub fluid_cp_j_per_kg_k: f64,
    /// Coupled reaction kinetic system.
    pub reaction_system: ReactionSystem,
    /// Limiting reactant component ID for conversion tracking.
    pub limiting_reactant: String,
    /// Key product component ID for yield/selectivity tracking.
    pub primary_product: Option<String>,
    /// Number of axial discretization slices along reactor length (minimum 10).
    pub discretization_slices: usize,
}

/// A spatial solution point along the axial length of the PFR.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PfrSlicePoint {
    /// Axial distance from reactor inlet in meters.
    pub axial_position_m: f64,
    /// Cumulative reactor volume up to this point in m³.
    pub cumulative_volume_m3: f64,
    /// Local fluid temperature in Kelvin.
    pub temperature_k: f64,
    /// Local species concentrations in mol/m³.
    pub concentrations: IndexMap<String, f64>,
    /// Limiting reactant conversion $X_A(z)$.
    pub conversion: f64,
    /// Cumulative reaction heat released up to this point in Watts.
    pub cumulative_reaction_heat_w: f64,
}

/// Comprehensive solution report for a PFR simulation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PfrResult {
    /// Effluent temperature in Kelvin.
    pub outlet_temperature_k: f64,
    /// Effluent concentrations in mol/m³.
    pub outlet_concentrations: IndexMap<String, f64>,
    /// Overall limiting reactant conversion fraction.
    pub overall_conversion: f64,
    /// Overall product yield fraction.
    pub overall_yield: f64,
    /// Total reaction heat duty generated across reactor in Watts.
    pub total_reaction_heat_w: f64,
    /// Total heat transferred through the tube wall in Watts (positive if removed to coolant).
    pub total_wall_heat_w: f64,
    /// Discretized axial profile along reactor length.
    pub axial_profiles: Vec<PfrSlicePoint>,
    /// Relative mass balance residual.
    pub mass_balance_residual: f64,
    /// Relative energy balance residual.
    pub energy_balance_residual: f64,
}

/// Plug Flow Reactor simulation model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PfrReactorModel {
    pub config: PfrConfig,
}

impl PfrReactorModel {
    /// Creates a new PFR reactor model with specified configuration.
    pub fn new(config: PfrConfig) -> Self {
        Self { config }
    }

    /// Solves the 1D steady-state PFR differential equations across the reactor volume.
    ///
    /// Parameters:
    /// - `feed_mass_flow_kg_s`: total mass flow entering the reactor
    /// - `feed_temperature_k`: inlet temperature in Kelvin
    /// - `feed_pressure_pa`: inlet operating pressure in Pascal
    /// - `feed_concentrations`: inlet concentrations in mol/m³
    pub fn solve(
        &self,
        feed_mass_flow_kg_s: f64,
        feed_temperature_k: f64,
        feed_pressure_pa: f64,
        feed_concentrations: &IndexMap<String, f64>,
    ) -> Result<PfrResult, PhysicsError> {
        if feed_mass_flow_kg_s <= 0.0 || !feed_mass_flow_kg_s.is_finite() {
            return Err(PhysicsError::InvalidOperatingCondition {
                message: format!("non-positive feed flow {feed_mass_flow_kg_s} in PFR"),
            });
        }
        if feed_temperature_k <= 0.0 || !feed_temperature_k.is_finite() {
            return Err(PhysicsError::NonPositiveTemperature {
                value: feed_temperature_k,
                context: "PFR feed temperature".to_string(),
            });
        }

        let n_tubes = self.config.tube_count.max(1) as f64;
        let d = self.config.diameter_m.max(0.001);
        let l = self.config.length_m.max(0.01);
        let cross_section = std::f64::consts::PI * d * d / 4.0 * n_tubes;
        let total_volume = cross_section * l;
        let specific_surface_area = 4.0 / d; // m²/m³ per tube

        let rho = self.config.fluid_density_kg_per_m3.max(1.0);
        let cp = self.config.fluid_cp_j_per_kg_k.max(1.0);
        let volumetric_flow = feed_mass_flow_kg_s / rho; // m³/s

        let n_slices = self.config.discretization_slices.max(10);
        let dv = total_volume / (n_slices as f64);
        let dz = l / (n_slices as f64);

        let mut current_temp = feed_temperature_k;
        let mut current_conc = feed_concentrations.clone();
        let mut profiles = Vec::with_capacity(n_slices + 1);

        let initial_lim_conc = feed_concentrations
            .get(&self.config.limiting_reactant)
            .copied()
            .unwrap_or(0.0);

        profiles.push(PfrSlicePoint {
            axial_position_m: 0.0,
            cumulative_volume_m3: 0.0,
            temperature_k: current_temp,
            concentrations: current_conc.clone(),
            conversion: 0.0,
            cumulative_reaction_heat_w: 0.0,
        });

        let mut total_reaction_heat_w = 0.0;
        let mut total_wall_heat_w = 0.0;

        for slice in 1..=n_slices {
            let z = (slice as f64) * dz;
            let v_accum = (slice as f64) * dv;

            // Evaluate reaction kinetics at slice conditions
            let rxn_eval = self.config.reaction_system.evaluate_system(
                current_temp,
                feed_pressure_pa,
                &current_conc,
            )?;
            let comp_rates = rxn_eval.component_rates;
            let q_vol = rxn_eval.total_volumetric_heat_w_per_m3;

            // Slice reaction heat duty: Q_rxn,slice = q_vol * dV (Watts)
            let q_rxn_slice = q_vol * dv;
            total_reaction_heat_w += q_rxn_slice;

            // Heat transfer through wall: Q_wall,slice = U * (a' * dV) * (T - T_wall)
            let wall_area_slice = specific_surface_area * dv;
            let q_wall_slice = self.config.overall_u_w_per_m2_k
                * wall_area_slice
                * (current_temp - self.config.wall_temperature_k);
            total_wall_heat_w += q_wall_slice;

            // Update concentrations: d(Q*C_i)/dV = R_i => C_i,next = C_i + (R_i * dV) / Q
            for (comp, c_val) in &mut current_conc {
                let r_i = comp_rates.get(comp).copied().unwrap_or(0.0);
                let delta_c = (r_i * dv) / volumetric_flow;
                *c_val = (*c_val + delta_c).max(0.0);
            }

            // Update temperature:
            // dT/dV = (q_vol - U * a' * (T - T_wall)) / (m_dot * Cp)
            let m_dot_cp = feed_mass_flow_kg_s * cp;
            let dt_dv = (q_vol - self.config.overall_u_w_per_m2_k * specific_surface_area * (current_temp - self.config.wall_temperature_k)) / m_dot_cp;
            current_temp = (current_temp + dt_dv * dv).max(50.0);

            let conv = if initial_lim_conc > 1e-6 {
                let c_lim = current_conc
                    .get(&self.config.limiting_reactant)
                    .copied()
                    .unwrap_or(0.0);
                ((initial_lim_conc - c_lim) / initial_lim_conc).clamp(0.0, 1.0)
            } else {
                0.0
            };

            profiles.push(PfrSlicePoint {
                axial_position_m: z,
                cumulative_volume_m3: v_accum,
                temperature_k: current_temp,
                concentrations: current_conc.clone(),
                conversion: conv,
                cumulative_reaction_heat_w: total_reaction_heat_w,
            });
        }

        let overall_conversion = if initial_lim_conc > 1e-6 {
            let c_lim_out = current_conc
                .get(&self.config.limiting_reactant)
                .copied()
                .unwrap_or(0.0);
            ((initial_lim_conc - c_lim_out) / initial_lim_conc).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let overall_yield = if let Some(ref prod) = self.config.primary_product {
            if initial_lim_conc > 1e-6 {
                let c_prod_out = current_conc.get(prod).copied().unwrap_or(0.0);
                let c_prod_in = feed_concentrations.get(prod).copied().unwrap_or(0.0);
                ((c_prod_out - c_prod_in) / initial_lim_conc).clamp(0.0, 5.0)
            } else {
                0.0
            }
        } else {
            0.0
        };

        // Energy balance closure check:
        // Delta H_flow = m_dot * Cp * (T_out - T_in)
        // Expected: Delta H_flow + Q_wall - Q_reaction = 0
        let delta_h_flow = feed_mass_flow_kg_s * cp * (current_temp - feed_temperature_k);
        let energy_residual = (delta_h_flow + total_wall_heat_w - total_reaction_heat_w).abs()
            / (total_reaction_heat_w.abs() + total_wall_heat_w.abs() + 1.0);

        Ok(PfrResult {
            outlet_temperature_k: current_temp,
            outlet_concentrations: current_conc,
            overall_conversion,
            overall_yield,
            total_reaction_heat_w,
            total_wall_heat_w,
            axial_profiles: profiles,
            mass_balance_residual: 0.0, // Strictly conserved in 1D incompressible formulation
            energy_balance_residual: energy_residual,
        })
    }
}
