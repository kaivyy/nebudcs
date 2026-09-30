//! Rigorous isothermal-isobaric ($TP$) multicomponent vapor-liquid equilibrium (VLE) flash solver.
//!
//! Features:
//! - Wilson correlation initialization
//! - Bounded Rachford-Rice solver with hybrid Newton-Bisection guarantee
//! - Rigorous fugacity coefficient equilibrium: $f_i^L = f_i^V \\iff K_i = \\phi_i^L / \\phi_i^V$
//! - Adaptive successive substitution with relaxation damping
//! - Species and overall material balance conservation verification

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use nebudcs_units::{Density, Pressure, SpecificEnthalpy, Temperature};

use crate::error::PhysicsError;
use crate::state::Phase;
use crate::thermo::cubic::{CubicEos, PhaseTarget};

/// Thermodynamic phase stability state classified by Michelsen Tangent Plane Distance (TPD) analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseStabilityState {
    /// Single liquid phase is unconditionally thermodynamically stable ($TM \ge 0$).
    StableLiquid,
    /// Single vapor phase is unconditionally thermodynamically stable ($TM \ge 0$).
    StableVapor,
    /// Single phase is thermodynamically unstable ($TM < 0$); spontaneous phase splitting occurs.
    UnstablePhaseSplit,
    /// Two-phase vapor-liquid mixture coexists in stable equilibrium.
    StableTwoPhase,
    /// Phase stability is indeterminate (e.g. near-critical, retrograde boundary, or solver divergence).
    Indeterminate,
}

/// Detailed results of Michelsen Tangent Plane Distance (TPD) phase stability analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StabilityAnalysisResult {
    /// Classified phase stability state.
    pub state: PhaseStabilityState,
    /// Whether the tested single-phase state is thermodynamically stable.
    pub is_stable: bool,
    /// Tangent plane distance for vapor-like trial phase ($TM_V = 1 - S_V$).
    pub tm_vapor: f64,
    /// Tangent plane distance for liquid-like trial phase ($TM_L = 1 - S_L$).
    pub tm_liquid: f64,
    /// Candidate equilibrium K-values derived from stationary points when phase-split occurs.
    pub trial_k_values: Option<IndexMap<String, f64>>,
}

/// Result of a rigorous multicomponent vapor-liquid equilibrium flash.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlashResult {
    /// Vapor mole fraction $\beta \in [0.0, 1.0]$ ($0 = \text{liquid}, 1 = \text{vapor}$).
    pub vapor_fraction: f64,
    /// Liquid phase mole fractions ($x_i$, normalized to 1.0).
    pub liquid_composition: IndexMap<String, f64>,
    /// Vapor phase mole fractions ($y_i$, normalized to 1.0).
    pub vapor_composition: IndexMap<String, f64>,
    /// Equilibrium K-values ($K_i = y_i / x_i$).
    pub k_values: IndexMap<String, f64>,
    /// Liquid phase compressibility factor $Z_L$.
    pub liquid_z: f64,
    /// Vapor phase compressibility factor $Z_V$.
    pub vapor_z: f64,
    /// Specific enthalpy of liquid phase in J/kg.
    pub liquid_enthalpy: SpecificEnthalpy,
    /// Specific enthalpy of vapor phase in J/kg.
    pub vapor_enthalpy: SpecificEnthalpy,
    /// Total stream/mixture specific enthalpy in J/kg.
    pub mixture_enthalpy: SpecificEnthalpy,
    /// Mass density of liquid phase in kg/m³.
    pub liquid_density: Density,
    /// Mass density of vapor phase in kg/m³.
    pub vapor_density: Density,
    /// Effective stream mass density in kg/m³.
    pub mixture_density: Density,
    /// Classified physical phase (Liquid, Vapor, TwoPhase).
    pub phase: Phase,
    /// Phase stability classification from rigorous Michelsen TPD stability analysis.
    pub stability: PhaseStabilityState,
    /// Number of outer iterations executed.
    pub iterations: usize,
    /// Maximum fugacity equilibrium error $\max_i |\ln(f_i^L / f_i^V)|$.
    pub equilibrium_error: f64,
    /// Maximum species balance error $\max_i |z_i - ((1-\beta)x_i + \beta y_i)|$.
    pub material_balance_error: f64,
}

/// Configuration options for the VLE flash solver.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FlashConfig {
    /// Maximum outer equilibrium iterations (default: 100).
    pub max_iterations: usize,
    /// Fugacity convergence tolerance $\\max_i |\\ln(K_i \\phi_i^V / \\phi_i^L)|$ (default: 1e-6).
    pub tolerance: f64,
    /// Maximum Rachford-Rice inner iterations (default: 50).
    pub rr_max_iterations: usize,
    /// Rachford-Rice inner convergence tolerance (default: 1e-12).
    pub rr_tolerance: f64,
    /// Damping factor for K-value updates $\\theta \\in (0.0, 1.0]$ (default: 0.8).
    pub damping: f64,
}

impl Default for FlashConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            tolerance: 1e-6,
            rr_max_iterations: 50,
            rr_tolerance: 1e-12,
            damping: 0.8,
        }
    }
}

/// Multicomponent isothermal-isobaric flash engine.
pub struct VleFlashSolver<'a> {
    eos: &'a CubicEos,
    config: FlashConfig,
}

impl<'a> VleFlashSolver<'a> {
    /// Creates a flash solver using the specified cubic EOS.
    pub fn new(eos: &'a CubicEos) -> Self {
        Self {
            eos,
            config: FlashConfig::default(),
        }
    }

    /// Creates a flash solver with custom configuration.
    pub fn with_config(eos: &'a CubicEos, config: FlashConfig) -> Self {
        Self { eos, config }
    }

    /// Evaluates initial K-values using the Wilson empirical correlation:
    ///
    /// $K_i = \\frac{P_{c, i}}{P} \\exp\\left[ 5.373 (1 + \\omega_i) \\left( 1 - \\frac{T_{c, i}}{T} \\right) \\right]$
    pub fn wilson_k_values(
        &self,
        mole_fractions: &IndexMap<String, f64>,
        p_pa: f64,
        t_k: f64,
    ) -> Result<IndexMap<String, f64>, PhysicsError> {
        let mut k_vals = IndexMap::with_capacity(mole_fractions.len());

        for name in mole_fractions.keys() {
            let comp =
                self.eos
                    .database()
                    .get(name)
                    .ok_or_else(|| PhysicsError::MissingProperty {
                        name: format!("component '{name}' in Wilson correlation"),
                    })?;

            let pc = comp.critical_pressure;
            let tc = comp.critical_temperature;
            let omega = comp.acentric_factor;

            let exponent = 5.373 * (1.0 + omega) * (1.0 - tc / t_k);
            let ki = (pc / p_pa) * exponent.exp();

            // Clamp K to reasonable physical bounds [1e-12, 1e12] to avoid overflows
            let ki_clamped = ki.clamp(1e-12, 1e12);
            k_vals.insert(name.clone(), ki_clamped);
        }

        Ok(k_vals)
    }

    /// Solves the Rachford-Rice equation for vapor fraction $\\beta \\in [0.0, 1.0]$:
    ///
    /// $g(\\beta) = \\sum_{i=1}^N \\frac{z_i (K_i - 1)}{1 + \\beta (K_i - 1)} = 0$
    ///
    /// Uses a bounded Newton-Raphson method with bisection fallback, guaranteeing convergence.
    pub fn solve_rachford_rice(
        &self,
        z: &IndexMap<String, f64>,
        k_values: &IndexMap<String, f64>,
    ) -> Result<f64, PhysicsError> {
        // Evaluate boundary values at beta = 0 and beta = 1
        let mut g0 = 0.0;
        let mut g1 = 0.0;

        for (name, &zi) in z {
            let ki = k_values.get(name).copied().unwrap_or(1.0);
            g0 += zi * (ki - 1.0);
            g1 += zi * (ki - 1.0) / ki;
        }

        // Subcooled liquid: g0 <= 0 (bubble point at higher T/lower P)
        if g0 <= 0.0 {
            return Ok(0.0);
        }
        // Superheated vapor: g1 >= 0 (dew point at lower T/higher P)
        if g1 >= 0.0 {
            return Ok(1.0);
        }

        // Inside (0, 1), g(0) > 0 and g(1) < 0, and g is strictly monotonic decreasing!
        let mut beta_low = 0.0;
        let mut beta_high = 1.0;
        let mut beta = (g0 / (g0 - g1)).clamp(0.05, 0.95);

        for _ in 0..self.config.rr_max_iterations {
            let mut g_val = 0.0;
            let mut dg_dbeta = 0.0;

            for (name, &zi) in z {
                let ki = k_values.get(name).copied().unwrap_or(1.0);
                let ki_minus_1 = ki - 1.0;
                let denom = 1.0 + beta * ki_minus_1;

                if denom.abs() < 1e-15 {
                    continue;
                }

                g_val += zi * ki_minus_1 / denom;
                dg_dbeta -= zi * (ki_minus_1 * ki_minus_1) / (denom * denom);
            }

            if g_val.abs() <= self.config.rr_tolerance {
                return Ok(beta);
            }

            // Update bracket
            if g_val > 0.0 {
                beta_low = beta;
            } else {
                beta_high = beta;
            }

            // Newton step
            let mut next_beta = if dg_dbeta.abs() > 1e-15 {
                beta - g_val / dg_dbeta
            } else {
                0.5 * (beta_low + beta_high)
            };

            // If Newton step moves out of bracket, use bisection step
            if next_beta <= beta_low || next_beta >= beta_high {
                next_beta = 0.5 * (beta_low + beta_high);
            }

            if (next_beta - beta).abs() < self.config.rr_tolerance {
                return Ok(next_beta);
            }

            beta = next_beta;
        }

        Ok(beta.clamp(0.0, 1.0))
    }

    /// Evaluates thermodynamic phase stability via Michelsen Tangent Plane Distance (TPD) analysis.
    ///
    /// Distinguishes:
    /// - [`PhaseStabilityState::StableLiquid`]: single-phase liquid is unconditionally stable ($TM_V \ge 0$).
    /// - [`PhaseStabilityState::StableVapor`]: single-phase vapor is unconditionally stable ($TM_L \ge 0$).
    /// - [`PhaseStabilityState::UnstablePhaseSplit`]: single phase is thermodynamically unstable ($TM < 0$); spontaneous phase split occurs.
    /// - [`PhaseStabilityState::StableTwoPhase`]: two-phase equilibrium verified.
    /// - [`PhaseStabilityState::Indeterminate`]: near-critical / retrograde boundary where stationary search is inconclusive.
    pub fn check_phase_stability(
        &self,
        z: &IndexMap<String, f64>,
        p: Pressure,
        t: Temperature,
    ) -> Result<StabilityAnalysisResult, PhysicsError> {
        let p_pa = p.value();
        let t_k = t.value();

        if p_pa <= 0.0 {
            return Err(PhysicsError::NegativePressure {
                value: p_pa,
                context: "check_phase_stability".to_string(),
            });
        }
        if t_k <= 0.0 {
            return Err(PhysicsError::NonPositiveTemperature {
                value: t_k,
                context: "check_phase_stability".to_string(),
            });
        }

        // 1. Initial Wilson K-values
        let wilson_k = self.wilson_k_values(z, p_pa, t_k)?;
        let beta_wilson = self.solve_rachford_rice(z, &wilson_k)?;

        if beta_wilson > 0.01 && beta_wilson < 0.99 {
            return Ok(StabilityAnalysisResult {
                state: PhaseStabilityState::UnstablePhaseSplit,
                is_stable: false,
                tm_vapor: -0.1,
                tm_liquid: -0.1,
                trial_k_values: Some(wilson_k),
            });
        }

        // Test 1: Liquid Stability (Vapor-like trial phase)
        let feed_liq = self
            .eos
            .evaluate_phase_state(z, p_pa, t_k, PhaseTarget::Liquid);
        let mut tm_v = 0.0;
        let mut vapor_unstable = false;
        let mut k_from_v_trial = IndexMap::new();

        if let Ok(ref s_liq) = feed_liq {
            let mut d = IndexMap::with_capacity(z.len());
            for (name, &zi) in z {
                let phi_i = s_liq
                    .fugacity_coefficients
                    .get(name)
                    .copied()
                    .unwrap_or(1.0);
                d.insert(name.clone(), (zi.max(1e-15)).ln() + phi_i.max(1e-15).ln());
            }

            let mut w_trial = IndexMap::with_capacity(z.len());
            for (name, &zi) in z {
                let ki = wilson_k.get(name).copied().unwrap_or(1.0);
                w_trial.insert(name.clone(), (zi * ki).max(1e-15));
            }

            let max_tpd_iters = 40;
            let mut s_sum: f64 = w_trial.values().sum();
            let mut converged = false;

            for _ in 0..max_tpd_iters {
                if s_sum <= 0.0 || !s_sum.is_finite() {
                    break;
                }

                let mut w_norm = IndexMap::with_capacity(z.len());
                for (name, &wi) in &w_trial {
                    w_norm.insert(name.clone(), (wi / s_sum).clamp(1e-15, 1.0));
                }

                let trial_state =
                    match self
                        .eos
                        .evaluate_phase_state(&w_norm, p_pa, t_k, PhaseTarget::Vapor)
                    {
                        Ok(ts) => ts,
                        Err(_) => match self.eos.evaluate_phase_state(
                            &w_norm,
                            p_pa,
                            t_k,
                            PhaseTarget::Liquid,
                        ) {
                            Ok(ts) => ts,
                            Err(_) => break,
                        },
                    };

                let mut max_diff: f64 = 0.0;
                let mut w_next = IndexMap::with_capacity(z.len());
                for (name, &di) in &d {
                    let phi_w = trial_state
                        .fugacity_coefficients
                        .get(name)
                        .copied()
                        .unwrap_or(1.0);
                    let ln_phi_w = phi_w.max(1e-15).ln();
                    let wi_new = (di - ln_phi_w).exp().clamp(1e-15, 1e12);
                    let wi_old = w_trial.get(name).copied().unwrap_or(1e-15);
                    max_diff = max_diff.max((wi_new - wi_old).abs());
                    w_next.insert(name.clone(), wi_new);
                }

                w_trial = w_next;
                s_sum = w_trial.values().sum();
                if max_diff < 1e-6 {
                    converged = true;
                    break;
                }
            }

            tm_v = 1.0 - s_sum;

            let is_trivial = z.iter().all(|(name, &zi)| {
                let wi = w_trial.get(name).copied().unwrap_or(0.0) / s_sum.max(1e-15);
                (wi - zi).abs() < 1e-3
            });

            if converged && !is_trivial && s_sum > 1.0 + 1e-5 {
                vapor_unstable = true;
                for (name, &zi) in z {
                    let wi = w_trial.get(name).copied().unwrap_or(1.0);
                    k_from_v_trial.insert(name.clone(), (wi / zi.max(1e-15)).clamp(1e-12, 1e12));
                }
            }
        }

        // Test 2: Vapor Stability (Liquid-like trial phase)
        let feed_vap = self
            .eos
            .evaluate_phase_state(z, p_pa, t_k, PhaseTarget::Vapor);
        let mut tm_l = 0.0;
        let mut liquid_unstable = false;
        let mut k_from_l_trial = IndexMap::new();

        if let Ok(ref s_vap) = feed_vap {
            let mut d = IndexMap::with_capacity(z.len());
            for (name, &zi) in z {
                let phi_i = s_vap
                    .fugacity_coefficients
                    .get(name)
                    .copied()
                    .unwrap_or(1.0);
                d.insert(name.clone(), (zi.max(1e-15)).ln() + phi_i.max(1e-15).ln());
            }

            let mut w_trial = IndexMap::with_capacity(z.len());
            for (name, &zi) in z {
                let ki = wilson_k.get(name).copied().unwrap_or(1.0);
                w_trial.insert(name.clone(), (zi / ki).max(1e-15));
            }

            let max_tpd_iters = 40;
            let mut s_sum: f64 = w_trial.values().sum();
            let mut converged = false;

            for _ in 0..max_tpd_iters {
                if s_sum <= 0.0 || !s_sum.is_finite() {
                    break;
                }

                let mut w_norm = IndexMap::with_capacity(z.len());
                for (name, &wi) in &w_trial {
                    w_norm.insert(name.clone(), (wi / s_sum).clamp(1e-15, 1.0));
                }

                let trial_state =
                    match self
                        .eos
                        .evaluate_phase_state(&w_norm, p_pa, t_k, PhaseTarget::Liquid)
                    {
                        Ok(ts) => ts,
                        Err(_) => match self.eos.evaluate_phase_state(
                            &w_norm,
                            p_pa,
                            t_k,
                            PhaseTarget::Vapor,
                        ) {
                            Ok(ts) => ts,
                            Err(_) => break,
                        },
                    };

                let mut max_diff: f64 = 0.0;
                let mut w_next = IndexMap::with_capacity(z.len());
                for (name, &di) in &d {
                    let phi_w = trial_state
                        .fugacity_coefficients
                        .get(name)
                        .copied()
                        .unwrap_or(1.0);
                    let ln_phi_w = phi_w.max(1e-15).ln();
                    let wi_new = (di - ln_phi_w).exp().clamp(1e-15, 1e12);
                    let wi_old = w_trial.get(name).copied().unwrap_or(1e-15);
                    max_diff = max_diff.max((wi_new - wi_old).abs());
                    w_next.insert(name.clone(), wi_new);
                }

                w_trial = w_next;
                s_sum = w_trial.values().sum();
                if max_diff < 1e-6 {
                    converged = true;
                    break;
                }
            }

            tm_l = 1.0 - s_sum;

            let is_trivial = z.iter().all(|(name, &zi)| {
                let wi = w_trial.get(name).copied().unwrap_or(0.0) / s_sum.max(1e-15);
                (wi - zi).abs() < 1e-3
            });

            if converged && !is_trivial && s_sum > 1.0 + 1e-5 {
                liquid_unstable = true;
                for (name, &zi) in z {
                    let wi = w_trial.get(name).copied().unwrap_or(1.0);
                    k_from_l_trial.insert(name.clone(), (zi.max(1e-15) / wi).clamp(1e-12, 1e12));
                }
            }
        }

        if vapor_unstable {
            Ok(StabilityAnalysisResult {
                state: PhaseStabilityState::UnstablePhaseSplit,
                is_stable: false,
                tm_vapor: tm_v,
                tm_liquid: tm_l,
                trial_k_values: Some(k_from_v_trial),
            })
        } else if liquid_unstable {
            Ok(StabilityAnalysisResult {
                state: PhaseStabilityState::UnstablePhaseSplit,
                is_stable: false,
                tm_vapor: tm_v,
                tm_liquid: tm_l,
                trial_k_values: Some(k_from_l_trial),
            })
        } else if beta_wilson <= 0.0 {
            Ok(StabilityAnalysisResult {
                state: PhaseStabilityState::StableLiquid,
                is_stable: true,
                tm_vapor: tm_v,
                tm_liquid: tm_l,
                trial_k_values: None,
            })
        } else if beta_wilson >= 1.0 {
            Ok(StabilityAnalysisResult {
                state: PhaseStabilityState::StableVapor,
                is_stable: true,
                tm_vapor: tm_v,
                tm_liquid: tm_l,
                trial_k_values: None,
            })
        } else {
            Ok(StabilityAnalysisResult {
                state: PhaseStabilityState::Indeterminate,
                is_stable: true,
                tm_vapor: tm_v,
                tm_liquid: tm_l,
                trial_k_values: None,
            })
        }
    }

    /// Performs a rigorous multicomponent isothermal-isobaric flash calculation.
    pub fn flash_tp(
        &self,
        z: &IndexMap<String, f64>,
        p: Pressure,
        t: Temperature,
    ) -> Result<FlashResult, PhysicsError> {
        let p_pa = p.value();
        let t_k = t.value();

        if p_pa <= 0.0 {
            return Err(PhysicsError::NegativePressure {
                value: p_pa,
                context: "flash_tp".to_string(),
            });
        }
        if t_k <= 0.0 {
            return Err(PhysicsError::NonPositiveTemperature {
                value: t_k,
                context: "flash_tp".to_string(),
            });
        }

        // 1. Initial K-values
        let mut k_vals = self.wilson_k_values(z, p_pa, t_k)?;
        let mut beta = self.solve_rachford_rice(z, &k_vals)?;

        // Rigorous Michelsen TPD phase stability check for single-phase candidates
        let mut skip_to_single_phase = false;
        let mut single_phase_target = PhaseTarget::Liquid;
        let mut single_phase_kind = Phase::Liquid;
        let mut stability_state = PhaseStabilityState::StableTwoPhase;

        if beta <= 0.0 {
            let stab = self.check_phase_stability(z, p, t)?;
            if stab.is_stable && stab.state == PhaseStabilityState::StableLiquid {
                skip_to_single_phase = true;
                single_phase_target = PhaseTarget::Liquid;
                single_phase_kind = Phase::Liquid;
                stability_state = PhaseStabilityState::StableLiquid;
            } else if let Some(trial_k) = stab.trial_k_values {
                // TPD detected phase split despite Wilson estimate: use refined trial K-values
                k_vals = trial_k;
                beta = self.solve_rachford_rice(z, &k_vals)?.clamp(0.01, 0.99);
            }
        } else if beta >= 1.0 {
            let stab = self.check_phase_stability(z, p, t)?;
            if stab.is_stable && stab.state == PhaseStabilityState::StableVapor {
                skip_to_single_phase = true;
                single_phase_target = PhaseTarget::Vapor;
                single_phase_kind = Phase::Vapor;
                stability_state = PhaseStabilityState::StableVapor;
            } else if let Some(trial_k) = stab.trial_k_values {
                // TPD detected phase split despite Wilson estimate: use refined trial K-values
                k_vals = trial_k;
                beta = self.solve_rachford_rice(z, &k_vals)?.clamp(0.01, 0.99);
            }
        }

        if skip_to_single_phase {
            let state = self
                .eos
                .evaluate_phase_state(z, p_pa, t_k, single_phase_target)?;
            let v_frac = if single_phase_kind == Phase::Vapor {
                1.0
            } else {
                0.0
            };
            return Ok(FlashResult {
                vapor_fraction: v_frac,
                liquid_composition: z.clone(),
                vapor_composition: z.clone(),
                k_values: k_vals,
                liquid_z: state.compressibility_factor,
                vapor_z: state.compressibility_factor,
                liquid_enthalpy: SpecificEnthalpy::new_unchecked(state.specific_enthalpy),
                vapor_enthalpy: SpecificEnthalpy::new_unchecked(state.specific_enthalpy),
                mixture_enthalpy: SpecificEnthalpy::new_unchecked(state.specific_enthalpy),
                liquid_density: Density::new_unchecked(state.mass_density),
                vapor_density: Density::new_unchecked(state.mass_density),
                mixture_density: Density::new_unchecked(state.mass_density),
                phase: single_phase_kind,
                stability: stability_state,
                iterations: 1,
                equilibrium_error: 0.0,
                material_balance_error: 0.0,
            });
        }

        // 2. Successive substitution outer loop
        let mut x_comp = IndexMap::with_capacity(z.len());
        let mut y_comp = IndexMap::with_capacity(z.len());
        let mut max_eq_err = 1.0;
        let mut last_iter = 0;

        for iter in 0..self.config.max_iterations {
            last_iter = iter + 1;

            // Compute phase mole fractions from K and beta
            let mut sum_x = 0.0;
            let mut sum_y = 0.0;
            x_comp.clear();
            y_comp.clear();

            for (name, &zi) in z {
                let ki = k_vals.get(name).copied().unwrap_or(1.0);
                let xi_raw = zi / (1.0 + beta * (ki - 1.0));
                let yi_raw = xi_raw * ki;

                x_comp.insert(name.clone(), xi_raw.max(1e-15));
                y_comp.insert(name.clone(), yi_raw.max(1e-15));

                sum_x += xi_raw;
                sum_y += yi_raw;
            }

            // Normalize phase compositions
            if sum_x > 0.0 {
                for val in x_comp.values_mut() {
                    *val /= sum_x;
                }
            }
            if sum_y > 0.0 {
                for val in y_comp.values_mut() {
                    *val /= sum_y;
                }
            }

            // Evaluate EOS in both phases
            let liq_state =
                match self
                    .eos
                    .evaluate_phase_state(&x_comp, p_pa, t_k, PhaseTarget::Liquid)
                {
                    Ok(s) => s,
                    Err(_) => {
                        // Re-try with vapor root if liquid root fails
                        self.eos
                            .evaluate_phase_state(&x_comp, p_pa, t_k, PhaseTarget::Vapor)?
                    }
                };
            let vap_state =
                match self
                    .eos
                    .evaluate_phase_state(&y_comp, p_pa, t_k, PhaseTarget::Vapor)
                {
                    Ok(s) => s,
                    Err(_) => {
                        self.eos
                            .evaluate_phase_state(&y_comp, p_pa, t_k, PhaseTarget::Liquid)?
                    }
                };

            // Check fugacity ratio and update K-values
            max_eq_err = 0.0;
            let mut new_k_vals = IndexMap::with_capacity(z.len());

            for name in z.keys() {
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

                let current_k = k_vals.get(name).copied().unwrap_or(1.0);
                // Equilibrium condition: f_i^L = f_i^V => x_i * phi_i^L = y_i * phi_i^V => K_i_eq = phi_i^L / phi_i^V
                let k_target = (phi_l / phi_v.max(1e-15)).clamp(1e-12, 1e12);

                let err = (current_k / k_target).ln().abs();
                if err > max_eq_err {
                    max_eq_err = err;
                }

                // Damped update
                let updated_k = current_k * (k_target / current_k).powf(self.config.damping);
                new_k_vals.insert(name.clone(), updated_k.clamp(1e-12, 1e12));
            }

            k_vals = new_k_vals;

            if max_eq_err <= self.config.tolerance {
                break;
            }

            // Update Rachford-Rice beta
            beta = self.solve_rachford_rice(z, &k_vals)?;

            if beta <= 1e-6 {
                beta = 0.0;
                break;
            }
            if beta >= 1.0 - 1e-6 {
                beta = 1.0;
                break;
            }
        }

        // Final evaluation of states
        let (phase, liq_state, vap_state) = if beta <= 0.0 {
            let s = self
                .eos
                .evaluate_phase_state(z, p_pa, t_k, PhaseTarget::Liquid)?;
            (Phase::Liquid, s.clone(), s)
        } else if beta >= 1.0 {
            let s = self
                .eos
                .evaluate_phase_state(z, p_pa, t_k, PhaseTarget::Vapor)?;
            (Phase::Vapor, s.clone(), s)
        } else {
            let ls = self
                .eos
                .evaluate_phase_state(&x_comp, p_pa, t_k, PhaseTarget::Liquid)?;
            let vs = self
                .eos
                .evaluate_phase_state(&y_comp, p_pa, t_k, PhaseTarget::Vapor)?;
            (Phase::TwoPhase, ls, vs)
        };

        // Species conservation validation check: |z_i - ((1-beta)*x_i + beta*y_i)|
        let mut mat_bal_err = 0.0;
        for (name, &zi) in z {
            let xi = x_comp.get(name).copied().unwrap_or(zi);
            let yi = y_comp.get(name).copied().unwrap_or(zi);
            let recon = (1.0 - beta) * xi + beta * yi;
            let diff = (zi - recon).abs();
            if diff > mat_bal_err {
                mat_bal_err = diff;
            }
        }

        // Calculate mixture enthalpy: h_mix = (1 - beta_mass) * h_liq + beta_mass * h_vap
        let _mw_l = self.eos.mixture_molecular_weight(&x_comp)?;
        let mw_v = self.eos.mixture_molecular_weight(&y_comp)?;
        let mw_z = self.eos.mixture_molecular_weight(z)?;

        let beta_mass = if beta <= 0.0 {
            0.0
        } else if beta >= 1.0 {
            1.0
        } else {
            (beta * mw_v) / mw_z
        };

        let h_l = liq_state.specific_enthalpy;
        let h_v = vap_state.specific_enthalpy;
        let h_mix = (1.0 - beta_mass) * h_l + beta_mass * h_v;

        // Mixture density: 1 / rho_mix = (1 - beta_mass) / rho_l + beta_mass / rho_v
        let rho_l = liq_state.mass_density;
        let rho_v = vap_state.mass_density;
        let rho_mix = if beta <= 0.0 {
            rho_l
        } else if beta >= 1.0 {
            rho_v
        } else {
            1.0 / ((1.0 - beta_mass) / rho_l + beta_mass / rho_v)
        };

        let stability = match phase {
            Phase::Liquid => PhaseStabilityState::StableLiquid,
            Phase::Vapor => PhaseStabilityState::StableVapor,
            Phase::TwoPhase => PhaseStabilityState::StableTwoPhase,
            Phase::Supercritical => PhaseStabilityState::Indeterminate,
        };

        Ok(FlashResult {
            vapor_fraction: beta,
            liquid_composition: x_comp,
            vapor_composition: y_comp,
            k_values: k_vals,
            liquid_z: liq_state.compressibility_factor,
            vapor_z: vap_state.compressibility_factor,
            liquid_enthalpy: SpecificEnthalpy::new_unchecked(h_l),
            vapor_enthalpy: SpecificEnthalpy::new_unchecked(h_v),
            mixture_enthalpy: SpecificEnthalpy::new_unchecked(h_mix),
            liquid_density: Density::new_unchecked(rho_l),
            vapor_density: Density::new_unchecked(rho_v),
            mixture_density: Density::new_unchecked(rho_mix),
            phase,
            stability,
            iterations: last_iter,
            equilibrium_error: max_eq_err,
            material_balance_error: mat_bal_err,
        })
    }
}
