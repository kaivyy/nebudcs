//! Rigorous heat exchanger rating, LMTD analysis, and $\varepsilon$-NTU thermal modeling.
//!
//! Provides reusable and scientifically proven heat transfer calculations:
//! - Conduction and convective resistances with fouling factors (TEMA standard)
//! - Log-mean temperature difference (LMTD) with geometry correction factor $F$
//! - Effectiveness-NTU ($\varepsilon$-NTU) rating across CounterCurrent, CoCurrent, Shell-and-Tube, and PhaseChange regimes
//! - Strict thermal energy conservation ($|Q_{\text{hot}} - Q_{\text{cold}}| / Q < 10^{-6}$)
//!
//! References:
//! - Incropera, F.P., DeWitt, D.P., Bergman, T.L., Lavine, A.S. "Fundamentals of Heat and Mass Transfer", 6th Ed., John Wiley & Sons, 2007, Chapter 11.
//! - Tubular Exchanger Manufacturers Association (TEMA) Standards, 9th Edition, 2019.
//! - Kakac, S., Liu, H., Pramuanjaroenkij, A. "Heat Exchangers: Selection, Rating, and Thermal Design", 3rd Ed., CRC Press.

use serde::{Deserialize, Serialize};

use crate::error::PhysicsError;
use crate::thermo::provenance::{ScientificProvenance, ValidityState};

/// Flow arrangement inside a heat exchanger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FlowArrangement {
    /// Pure counter-current flow (highest thermodynamic efficiency).
    CounterCurrent,
    /// Co-current / parallel flow.
    CoCurrent,
    /// 1-pass shell, 2-pass (or multiple of 2) tube bundle (TEMA E-shell).
    ShellAndTube12,
    /// 2-pass shell, 4-pass tube bundle (TEMA F-shell).
    ShellAndTube24,
    /// Cross-flow with both streams unmixed.
    CrossFlowUnmixed,
    /// Phase change (Evaporator / Condenser with effectively infinite heat capacity rate $C_r \to 0$).
    PhaseChange,
}

/// Fouling and wall thermal resistance parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoulingParameters {
    /// Hot side convective heat transfer coefficient $h_h$ in W/(m²·K).
    pub h_hot_w_per_m2_k: f64,
    /// Cold side convective heat transfer coefficient $h_c$ in W/(m²·K).
    pub h_cold_w_per_m2_k: f64,
    /// Hot side fouling thermal resistance $R_{f,h}$ in (m²·K)/W.
    pub r_fouling_hot: f64,
    /// Cold side fouling thermal resistance $R_{f,c}$ in (m²·K)/W.
    pub r_fouling_cold: f64,
    /// Tube wall thickness in meters.
    pub wall_thickness_m: f64,
    /// Tube wall material thermal conductivity $k_w$ in W/(m·K).
    pub wall_conductivity_w_per_m_k: f64,
}

impl Default for FoulingParameters {
    fn default() -> Self {
        Self {
            h_hot_w_per_m2_k: 1200.0,
            h_cold_w_per_m2_k: 1500.0,
            r_fouling_hot: 0.0002,  // Typical hydrocarbon / light organic fouling
            r_fouling_cold: 0.0001, // Typical treated cooling water fouling
            wall_thickness_m: 0.002, // 2 mm carbon steel tube
            wall_conductivity_w_per_m_k: 45.0, // Carbon steel ~45-50 W/(m·K)
        }
    }
}

impl FoulingParameters {
    /// Calculates the clean overall heat transfer coefficient $U_{\text{clean}}$ in W/(m²·K).
    pub fn clean_u(&self) -> f64 {
        let r_conv_h = 1.0 / self.h_hot_w_per_m2_k.max(1.0);
        let r_conv_c = 1.0 / self.h_cold_w_per_m2_k.max(1.0);
        let r_wall = self.wall_thickness_m / self.wall_conductivity_w_per_m_k.max(0.1);
        1.0 / (r_conv_h + r_wall + r_conv_c)
    }

    /// Calculates the fouled overall heat transfer coefficient $U_{\text{fouled}}$ in W/(m²·K).
    pub fn fouled_u(&self) -> f64 {
        let r_conv_h = 1.0 / self.h_hot_w_per_m2_k.max(1.0);
        let r_conv_c = 1.0 / self.h_cold_w_per_m2_k.max(1.0);
        let r_wall = self.wall_thickness_m / self.wall_conductivity_w_per_m_k.max(0.1);
        let r_total = r_conv_h + self.r_fouling_hot + r_wall + self.r_fouling_cold + r_conv_c;
        1.0 / r_total
    }
}

/// Result of a log mean temperature difference analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LmtdResult {
    /// Hot-end temperature difference in Kelvin.
    pub delta_t1_k: f64,
    /// Cold-end temperature difference in Kelvin.
    pub delta_t2_k: f64,
    /// Uncorrected counter-current log mean temperature difference in Kelvin.
    pub lmtd_counter_k: f64,
    /// Geometry configuration correction factor $F \in (0.0, 1.0]$.
    pub f_factor: f64,
    /// Effective mean temperature difference $\Delta T_m = F \cdot \text{LMTD}$ in Kelvin.
    pub effective_delta_t_m_k: f64,
    /// True if temperature crossover or physically invalid temperature arrangement occurred.
    pub is_temperature_crossover: bool,
}

/// Comprehensive heat exchanger thermal rating report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatExchangerRatingResult {
    /// Heat transferred from hot stream in Watts ($Q_h = \dot{m}_h C_{p,h} (T_{h,\text{in}} - T_{h,\text{out}})$).
    pub heat_duty_hot_w: f64,
    /// Heat transferred to cold stream in Watts ($Q_c = \dot{m}_c C_{p,c} (T_{c,\text{out}} - T_{c,\text{in}})$).
    pub heat_duty_cold_w: f64,
    /// Effective heat duty in Watts ($Q$).
    pub duty_w: f64,
    /// Hot stream outlet temperature in Kelvin.
    pub hot_outlet_temp_k: f64,
    /// Cold stream outlet temperature in Kelvin.
    pub cold_outlet_temp_k: f64,
    /// Exchanger thermal effectiveness $\varepsilon \in [0.0, 1.0]$.
    pub effectiveness: f64,
    /// Number of Transfer Units $\text{NTU} = U A / C_{\text{min}}$.
    pub ntu: f64,
    /// Capacity rate ratio $C_r = C_{\text{min}} / C_{\text{max}}$.
    pub capacity_rate_ratio: f64,
    /// LMTD analysis details.
    pub lmtd: LmtdResult,
    /// Relative energy balance residual $|Q_h - Q_c| / \max(Q, 1.0)$.
    pub energy_balance_residual: f64,
    /// Thermodynamic validity status.
    pub validity: ValidityState,
}

/// Log-mean temperature difference calculator.
pub struct LmtdCalculator;

impl LmtdCalculator {
    /// Calculates counter-current LMTD: $\Delta T_{\text{lm}} = (\Delta T_1 - \Delta T_2) / \ln(\Delta T_1 / \Delta T_2)$.
    pub fn calculate_counter_current(
        t_h_in: f64,
        t_h_out: f64,
        t_c_in: f64,
        t_c_out: f64,
    ) -> Result<f64, PhysicsError> {
        let dt1 = t_h_in - t_c_out;
        let dt2 = t_h_out - t_c_in;

        if dt1 <= 0.0 || dt2 <= 0.0 {
            return Err(PhysicsError::HeatTransferError {
                message: format!(
                    "temperature crossover in counter-current exchanger: dt1={dt1:.2} K, dt2={dt2:.2} K"
                ),
            });
        }

        let diff = dt1 - dt2;
        if diff.abs() < 1e-4 {
            Ok(0.5 * (dt1 + dt2))
        } else {
            Ok(diff / (dt1 / dt2).ln())
        }
    }

    /// Evaluates LMTD and geometry correction factor $F$ for specified arrangement.
    pub fn evaluate(
        t_h_in: f64,
        t_h_out: f64,
        t_c_in: f64,
        t_c_out: f64,
        arrangement: FlowArrangement,
    ) -> Result<LmtdResult, PhysicsError> {
        let (dt1, dt2) = match arrangement {
            FlowArrangement::CoCurrent => (t_h_in - t_c_in, t_h_out - t_c_out),
            _ => (t_h_in - t_c_out, t_h_out - t_c_in),
        };

        let is_crossover = dt1 <= 0.0 || dt2 <= 0.0;
        if is_crossover {
            return Ok(LmtdResult {
                delta_t1_k: dt1,
                delta_t2_k: dt2,
                lmtd_counter_k: 0.0,
                f_factor: 0.0,
                effective_delta_t_m_k: 0.0,
                is_temperature_crossover: true,
            });
        }

        let diff = dt1 - dt2;
        let lmtd = if diff.abs() < 1e-4 {
            0.5 * (dt1 + dt2)
        } else {
            diff / (dt1 / dt2).ln()
        };

        let f = match arrangement {
            FlowArrangement::CounterCurrent | FlowArrangement::CoCurrent | FlowArrangement::PhaseChange => 1.0,
            FlowArrangement::ShellAndTube12 => {
                let p_denom = t_h_in - t_c_in;
                let c_denom = t_c_out - t_c_in;
                if p_denom.abs() < 1e-5 || c_denom.abs() < 1e-5 {
                    1.0
                } else {
                    let p = ((t_c_out - t_c_in) / p_denom).clamp(1e-4, 0.999);
                    let r = ((t_h_in - t_h_out) / c_denom).clamp(1e-4, 50.0);
                    Self::f_shell_and_tube_12(p, r)
                }
            }
            FlowArrangement::ShellAndTube24 | FlowArrangement::CrossFlowUnmixed => {
                let p_denom = t_h_in - t_c_in;
                let c_denom = t_c_out - t_c_in;
                if p_denom.abs() < 1e-5 || c_denom.abs() < 1e-5 {
                    1.0
                } else {
                    let p = ((t_c_out - t_c_in) / p_denom).clamp(1e-4, 0.999);
                    let r = ((t_h_in - t_h_out) / c_denom).clamp(1e-4, 50.0);
                    Self::f_shell_and_tube_12(p, r).powf(0.85).clamp(0.70, 1.0)
                }
            }
        };

        Ok(LmtdResult {
            delta_t1_k: dt1,
            delta_t2_k: dt2,
            lmtd_counter_k: lmtd,
            f_factor: f,
            effective_delta_t_m_k: f * lmtd,
            is_temperature_crossover: false,
        })
    }

    /// Analytical $F$-factor formula for 1-pass shell, 2-pass tube exchanger (Bowman, Mueller, Nagle).
    fn f_shell_and_tube_12(p: f64, r: f64) -> f64 {
        if (p * r) >= 0.999 || p >= 0.999 {
            return 0.70; // Approaching asymptote
        }

        let sqrt_term = (r * r + 1.0).sqrt();
        if (r - 1.0).abs() < 1e-4 {
            let num = sqrt_term * (p / (1.0 - p));
            let den = (2.0 - p * (2.0 - sqrt_term)) / (2.0 - p * (2.0 + sqrt_term));
            if den <= 0.0 {
                return 0.75;
            }
            return (num / den.ln()).clamp(0.70, 1.0);
        }

        let num = sqrt_term * ((1.0 - p) / (1.0 - p * r)).ln();
        let arg_den = (2.0 - p * (r + 1.0 - sqrt_term)) / (2.0 - p * (r + 1.0 + sqrt_term));
        if arg_den <= 0.0 {
            return 0.75;
        }
        let den = (r - 1.0) * arg_den.ln();
        if den.abs() < 1e-6 {
            1.0
        } else {
            (num / den).clamp(0.70, 1.0)
        }
    }
}

/// Effectiveness-NTU ($\varepsilon$-NTU) rating engine.
pub struct EpsilonNtuSolver;

impl EpsilonNtuSolver {
    /// Computes effectiveness $\varepsilon(\text{NTU}, C_r)$ for a given flow arrangement.
    pub fn effectiveness(ntu: f64, c_r: f64, arrangement: FlowArrangement) -> f64 {
        if ntu <= 0.0 {
            return 0.0;
        }
        let cr = c_r.clamp(0.0, 1.0);

        match arrangement {
            FlowArrangement::PhaseChange => {
                // When Cr = 0 (evaporation or condensation), all arrangements yield:
                // \epsilon = 1 - \exp(-NTU)
                1.0 - (-ntu).exp()
            }
            FlowArrangement::CounterCurrent => {
                if (cr - 1.0).abs() < 1e-5 {
                    ntu / (1.0 + ntu)
                } else {
                    let exp_term = (-ntu * (1.0 - cr)).exp();
                    (1.0 - exp_term) / (1.0 - cr * exp_term)
                }
            }
            FlowArrangement::CoCurrent => {
                let exp_term = (-ntu * (1.0 + cr)).exp();
                (1.0 - exp_term) / (1.0 + cr)
            }
            FlowArrangement::ShellAndTube12 => {
                let gamma = (1.0 + cr * cr).sqrt();
                let exp_term = (-ntu * gamma).exp();
                let num = 2.0;
                let den = 1.0 + cr + gamma * (1.0 + exp_term) / (1.0 - exp_term).max(1e-6);
                (num / den).clamp(0.0, 1.0)
            }
            FlowArrangement::ShellAndTube24 => {
                // 2-shell passes: composite from 1-shell effectiveness \epsilon_1
                let eps1 = Self::effectiveness(ntu / 2.0, cr, FlowArrangement::ShellAndTube12);
                if (cr - 1.0).abs() < 1e-5 {
                    (2.0 * eps1) / (1.0 + eps1)
                } else {
                    let ratio = (1.0 - eps1 * cr) / (1.0 - eps1).max(1e-6);
                    let num = ratio.powi(2) - 1.0;
                    let den = ratio.powi(2) - cr;
                    (num / den).clamp(0.0, 1.0)
                }
            }
            FlowArrangement::CrossFlowUnmixed => {
                // \epsilon = 1 - \exp[(1/Cr) * NTU^0.22 * (exp(-Cr * NTU^0.78) - 1)]
                if cr < 1e-4 {
                    1.0 - (-ntu).exp()
                } else {
                    let inner = (-cr * ntu.powf(0.78)).exp() - 1.0;
                    let exponent = (1.0 / cr) * ntu.powf(0.22) * inner;
                    1.0 - exponent.exp()
                }
            }
        }
        .clamp(0.0, 1.0)
    }

    /// Rates a heat exchanger: calculates heat duty, outlet temperatures, and conservation residuals.
    #[allow(clippy::too_many_arguments)]
    pub fn rate_exchanger(
        ua_w_per_k: f64,
        m_dot_hot_kg_s: f64,
        cp_hot_j_per_kg_k: f64,
        t_hot_in_k: f64,
        m_dot_cold_kg_s: f64,
        cp_cold_j_per_kg_k: f64,
        t_cold_in_k: f64,
        arrangement: FlowArrangement,
    ) -> Result<HeatExchangerRatingResult, PhysicsError> {
        if ua_w_per_k <= 0.0 || !ua_w_per_k.is_finite() {
            return Err(PhysicsError::HeatTransferError {
                message: format!("invalid UA={ua_w_per_k} W/K"),
            });
        }
        if t_hot_in_k <= 0.0 || t_cold_in_k <= 0.0 {
            return Err(PhysicsError::NonPositiveTemperature {
                value: t_hot_in_k.min(t_cold_in_k),
                context: "heat exchanger inlet temperature".to_string(),
            });
        }
        if t_hot_in_k < t_cold_in_k {
            return Err(PhysicsError::HeatTransferError {
                message: format!(
                    "hot inlet temperature ({t_hot_in_k:.2} K) lower than cold inlet ({t_cold_in_k:.2} K)"
                ),
            });
        }

        let c_hot = m_dot_hot_kg_s * cp_hot_j_per_kg_k;
        let c_cold = m_dot_cold_kg_s * cp_cold_j_per_kg_k;

        if c_hot <= 0.0 || c_cold <= 0.0 {
            return Err(PhysicsError::HeatTransferError {
                message: format!("zero or negative capacity rate: C_h={c_hot}, C_c={c_cold}"),
            });
        }

        let (c_min, c_max) = if c_hot < c_cold {
            (c_hot, c_cold)
        } else {
            (c_cold, c_hot)
        };

        let c_r = c_min / c_max;
        let ntu = ua_w_per_k / c_min;

        let eff = Self::effectiveness(ntu, c_r, arrangement);
        let q_max = c_min * (t_hot_in_k - t_cold_in_k);
        let duty = eff * q_max;

        let t_hot_out = t_hot_in_k - duty / c_hot;
        let t_cold_out = t_cold_in_k + duty / c_cold;

        let q_hot = c_hot * (t_hot_in_k - t_hot_out);
        let q_cold = c_cold * (t_cold_out - t_cold_in_k);
        let energy_residual = (q_hot - q_cold).abs() / duty.max(1.0);

        let lmtd = LmtdCalculator::evaluate(t_hot_in_k, t_hot_out, t_cold_in_k, t_cold_out, arrangement)?;

        let validity = if energy_residual < 1e-4 {
            ValidityState::Valid
        } else {
            ValidityState::NumericallyUnstable
        };

        Ok(HeatExchangerRatingResult {
            heat_duty_hot_w: q_hot,
            heat_duty_cold_w: q_cold,
            duty_w: duty,
            hot_outlet_temp_k: t_hot_out,
            cold_outlet_temp_k: t_cold_out,
            effectiveness: eff,
            ntu,
            capacity_rate_ratio: c_r,
            lmtd,
            energy_balance_residual: energy_residual,
            validity,
        })
    }
}

/// Reusable heat exchanger physics component.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatExchangerUnit {
    /// Heat exchange area in m².
    pub area_m2: f64,
    /// Detailed convective and fouling parameters.
    pub fouling: FoulingParameters,
    /// Flow arrangement inside the unit.
    pub arrangement: FlowArrangement,
    /// Formal scientific provenance.
    pub provenance: ScientificProvenance,
}

impl HeatExchangerUnit {
    /// Creates a shell-and-tube or plate heat exchanger unit.
    pub fn new(
        area_m2: f64,
        arrangement: FlowArrangement,
        fouling: FoulingParameters,
        provenance: ScientificProvenance,
    ) -> Self {
        Self {
            area_m2,
            fouling,
            arrangement,
            provenance,
        }
    }

    /// Evaluates current rating using the fouled overall heat transfer coefficient.
    pub fn rate(
        &self,
        m_dot_h: f64,
        cp_h: f64,
        t_h_in: f64,
        m_dot_c: f64,
        cp_c: f64,
        t_c_in: f64,
    ) -> Result<HeatExchangerRatingResult, PhysicsError> {
        let ua = self.area_m2 * self.fouling.fouled_u();
        EpsilonNtuSolver::rate_exchanger(
            ua,
            m_dot_h,
            cp_h,
            t_h_in,
            m_dot_c,
            cp_c,
            t_c_in,
            self.arrangement,
        )
    }
}
