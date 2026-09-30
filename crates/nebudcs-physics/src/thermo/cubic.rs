//! Generic cubic equations of state (EOS) framework.
//!
//! Provides a unified, highly optimized implementation of:
//! - **Peng-Robinson (PR)**
//! - **Soave-Redlich-Kwong (SRK)**
//!
//! Features:
//! - Exact analytical temperature derivatives $d\\alpha/dT, da/dT, d^2\\alpha/dT^2$
//! - Classical quadratic mixing rules with symmetric binary interaction parameters $k_{ij}$
//! - Robust Cardano/trigonometric analytical cubic root solving with physical $Z > B$ filtering
//! - Component fugacity coefficients $\\phi_i$ and fugacity $f_i = x_i \\phi_i P$
//! - Rigorous thermodynamic departure functions (residual enthalpy, residual entropy)

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

use crate::error::PhysicsError;
use crate::thermo::component::{
    ComponentDatabase, ThermodynamicComponent, R_GAS_CONSTANT, STANDARD_REFERENCE_PRESSURE_PA,
};

/// Cubic equation of state model variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CubicEosType {
    /// Peng-Robinson (1976 / 1978)
    PengRobinson,
    /// Soave-Redlich-Kwong (1972)
    SoaveRedlichKwong,
}

/// Target phase for cubic root selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhaseTarget {
    /// Vapor phase (select largest feasible root $Z > B$).
    Vapor,
    /// Liquid phase (select smallest feasible root $Z > B$).
    Liquid,
}

/// Pure component cubic parameters evaluated at temperature $T$.
#[derive(Debug, Clone, Copy)]
pub struct PureComponentParameters {
    pub a: f64,
    pub b: f64,
    pub da_dt: f64,
    pub alpha: f64,
    pub dalpha_dt: f64,
}

/// Complete thermodynamic state evaluated by a cubic equation of state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CubicEosState {
    pub compressibility_factor: f64,                  // Z
    pub molar_volume: f64,                            // m³/mol
    pub mass_density: f64,                            // kg/m³
    pub molar_density: f64,                           // mol/m³
    pub specific_enthalpy: f64,                       // J/kg
    pub molar_enthalpy: f64,                          // J/mol
    pub residual_molar_enthalpy: f64,                 // J/mol
    pub specific_entropy: f64,                        // J/(kg·K)
    pub molar_entropy: f64,                           // J/(mol·K)
    pub residual_molar_entropy: f64,                  // J/(mol·K)
    pub fugacity_coefficients: IndexMap<String, f64>, // phi_i
    pub component_fugacities: IndexMap<String, f64>,  // f_i = x_i * phi_i * P (Pa)
}

/// Generic cubic equation of state engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CubicEos {
    eos_type: CubicEosType,
    database: ComponentDatabase,
}

impl CubicEos {
    /// Creates a Peng-Robinson cubic EOS engine.
    pub fn new_peng_robinson(database: ComponentDatabase) -> Self {
        Self {
            eos_type: CubicEosType::PengRobinson,
            database,
        }
    }

    /// Creates a Soave-Redlich-Kwong cubic EOS engine.
    pub fn new_srk(database: ComponentDatabase) -> Self {
        Self {
            eos_type: CubicEosType::SoaveRedlichKwong,
            database,
        }
    }

    /// Returns the active cubic EOS model type.
    pub fn eos_type(&self) -> CubicEosType {
        self.eos_type
    }

    /// Returns a reference to the component database.
    pub fn database(&self) -> &ComponentDatabase {
        &self.database
    }

    /// Returns a mutable reference to the component database.
    pub fn database_mut(&mut self) -> &mut ComponentDatabase {
        &mut self.database
    }

    /// EOS formulation constants:
    /// Returns `(u, w, Omega_a, Omega_b)` for the cubic polynomial:
    /// $P = \\frac{RT}{v-b} - \\frac{a}{v^2 + ubv + wb^2}$
    #[inline]
    pub fn constants(&self) -> (f64, f64, f64, f64) {
        match self.eos_type {
            CubicEosType::PengRobinson => {
                (2.0, -1.0, 0.457_235_528_921_382_2, 0.077_796_073_903_888_46)
            }
            CubicEosType::SoaveRedlichKwong => {
                (1.0, 0.0, 0.427_480_233_540_341_7, 0.086_640_349_964_957_72)
            }
        }
    }

    /// Evaluates pure component attraction parameter $a(T)$, co-volume $b$,
    /// and temperature derivatives $\\frac{da}{dT}, \\frac{d\\alpha}{dT}$.
    pub fn pure_parameters(
        &self,
        comp: &ThermodynamicComponent,
        t_k: f64,
    ) -> Result<PureComponentParameters, PhysicsError> {
        let t = t_k.max(1.0);
        let tc = comp.critical_temperature;
        let pc = comp.critical_pressure;
        let omega = comp.acentric_factor;

        let (_u, _w, omega_a, omega_b) = self.constants();
        let r = R_GAS_CONSTANT;

        // b = Omega_b * R * Tc / Pc
        let b = omega_b * r * tc / pc;

        // a(Tc) = Omega_a * R^2 * Tc^2 / Pc
        let a_tc = omega_a * r * r * tc * tc / pc;

        // Reduced temperature Tr = T / Tc
        let tr = t / tc;
        let sqrt_tr = tr.sqrt();

        // Alpha correlation m
        let m = match self.eos_type {
            CubicEosType::PengRobinson => {
                if omega <= 0.491 {
                    0.37464 + 1.54226 * omega - 0.26992 * omega * omega
                } else {
                    // Extended PR (Graboski & Daubert / Boston-Mathias)
                    0.379642 + 1.48503 * omega - 0.164423 * omega * omega + 0.016666 * omega.powi(3)
                }
            }
            CubicEosType::SoaveRedlichKwong => 0.480 + 1.574 * omega - 0.176 * omega * omega,
        };

        let bracket = 1.0 + m * (1.0 - sqrt_tr);
        let alpha = bracket * bracket;

        // Analytical derivative dalpha/dT:
        // dalpha/dT = 2 * bracket * (-m / (2 * sqrt(T * Tc))) = - (m * bracket) / sqrt(T * Tc)
        let dalpha_dt = -(m * bracket) / (t * tc).sqrt();

        let a = a_tc * alpha;
        let da_dt = a_tc * dalpha_dt;

        Ok(PureComponentParameters {
            a,
            b,
            da_dt,
            alpha,
            dalpha_dt,
        })
    }

    /// Evaluates mixture parameters $a_{\\text{mix}}, b_{\\text{mix}}, \\frac{da_{\\text{mix}}}{dT}$
    /// and pure component parameters using classical quadratic mixing rules.
    pub fn mixture_parameters(
        &self,
        mole_fractions: &IndexMap<String, f64>,
        t_k: f64,
    ) -> Result<(f64, f64, f64, IndexMap<String, PureComponentParameters>), PhysicsError> {
        let n = mole_fractions.len();
        if n == 0 {
            return Err(PhysicsError::InvalidComposition {
                reason: "cannot evaluate EOS for empty mixture".to_string(),
            });
        }

        let mut pure_params = IndexMap::with_capacity(n);
        for name in mole_fractions.keys() {
            let comp = self
                .database
                .get(name)
                .ok_or_else(|| PhysicsError::MissingProperty {
                    name: format!("component '{name}' not found in EOS database"),
                })?;
            let params = self.pure_parameters(comp, t_k)?;
            pure_params.insert(name.clone(), params);
        }

        let mut b_mix = 0.0;
        for (name, &xi) in mole_fractions {
            let bi = pure_params[name].b;
            b_mix += xi * bi;
        }

        let mut a_mix = 0.0;
        let mut da_mix_dt = 0.0;

        let comp_names: Vec<&String> = mole_fractions.keys().collect();
        for (i, &name_i) in comp_names.iter().enumerate() {
            let xi = mole_fractions[name_i];
            let pi = &pure_params[name_i];

            for (j, &name_j) in comp_names.iter().enumerate() {
                let xj = mole_fractions[name_j];
                let pj = &pure_params[name_j];

                let kij = if i == j {
                    0.0
                } else {
                    self.database.get_kij(name_i, name_j)
                };

                let sqrt_ai_aj = (pi.a * pj.a).sqrt();
                let a_ij = sqrt_ai_aj * (1.0 - kij);

                a_mix += xi * xj * a_ij;

                // Derivative d(a_ij)/dT = (1 - kij) * [ (da_i/dT * a_j + a_i * da_j/dT) / (2 * sqrt(a_i * a_j)) ]
                if sqrt_ai_aj > 1e-18 {
                    let da_ij_dt =
                        (1.0 - kij) * 0.5 * (pi.da_dt * pj.a + pi.a * pj.da_dt) / sqrt_ai_aj;
                    da_mix_dt += xi * xj * da_ij_dt;
                }
            }
        }

        if b_mix <= 0.0 || !b_mix.is_finite() {
            return Err(PhysicsError::InvalidOperatingCondition {
                message: format!("invalid EOS co-volume b_mix = {b_mix}"),
            });
        }
        if a_mix < 0.0 || !a_mix.is_finite() {
            return Err(PhysicsError::InvalidOperatingCondition {
                message: format!("invalid EOS attraction a_mix = {a_mix}"),
            });
        }

        Ok((a_mix, b_mix, da_mix_dt, pure_params))
    }

    /// Solves the cubic polynomial in compressibility factor $Z$:
    ///
    /// $Z^3 + c_2 Z^2 + c_1 Z + c_0 = 0$
    ///
    /// where:
    /// $A = \\frac{a P}{(R T)^2}, \\quad B = \\frac{b P}{R T}$
    /// $c_2 = -(1 + B - u B)$
    /// $c_1 = A + w B^2 - u B - u B^2$
    /// $c_0 = -(A B + w B^2 + w B^3)$
    pub fn solve_cubic_roots(
        &self,
        a_mix: f64,
        b_mix: f64,
        p_pa: f64,
        t_k: f64,
    ) -> Result<Vec<f64>, PhysicsError> {
        let (u, w, _, _) = self.constants();
        let r = R_GAS_CONSTANT;
        let rt = r * t_k;

        let a_dimless = (a_mix * p_pa) / (rt * rt);
        let b_dimless = (b_mix * p_pa) / rt;

        let c2 = -(1.0 + b_dimless - u * b_dimless);
        let c1 = a_dimless + w * b_dimless * b_dimless - u * b_dimless - u * b_dimless * b_dimless;
        let c0 = -(a_dimless * b_dimless + w * b_dimless * b_dimless + w * b_dimless.powi(3));

        // Depressed cubic y^3 + p y + q = 0 with substitution Z = y - c2 / 3
        let p_dep = c1 - (c2 * c2) / 3.0;
        let q_dep = (2.0 * c2.powi(3)) / 27.0 - (c1 * c2) / 3.0 + c0;

        let discriminant = (q_dep * q_dep) / 4.0 + (p_dep.powi(3)) / 27.0;

        let mut raw_roots = Vec::with_capacity(3);

        if discriminant > 1e-14 {
            // One real root
            let sqrt_d = discriminant.sqrt();
            let term_u = -q_dep / 2.0 + sqrt_d;
            let term_v = -q_dep / 2.0 - sqrt_d;

            let u_root = if term_u >= 0.0 {
                term_u.cbrt()
            } else {
                -(-term_u).cbrt()
            };
            let v_root = if term_v >= 0.0 {
                term_v.cbrt()
            } else {
                -(-term_v).cbrt()
            };

            let y1 = u_root + v_root;
            raw_roots.push(y1 - c2 / 3.0);
        } else {
            // Three real roots (trigonometric solution)
            let m_term = 2.0 * (-p_dep / 3.0).max(0.0).sqrt();
            let cos_3theta = if m_term.abs() > 1e-15 {
                ((-q_dep / 2.0) / ((-p_dep.powi(3)) / 27.0).max(0.0).sqrt()).clamp(-1.0, 1.0)
            } else {
                0.0
            };
            let theta = cos_3theta.acos() / 3.0;

            let y1 = m_term * theta.cos();
            let y2 = m_term * (theta + 2.0 * PI / 3.0).cos();
            let y3 = m_term * (theta + 4.0 * PI / 3.0).cos();

            raw_roots.push(y1 - c2 / 3.0);
            raw_roots.push(y2 - c2 / 3.0);
            raw_roots.push(y3 - c2 / 3.0);
        }

        // Physical filtering: $Z > B$ is required because molar volume $v > b$.
        // Also eliminate non-finite roots.
        let mut valid_roots: Vec<f64> = raw_roots
            .into_iter()
            .filter(|&z| z.is_finite() && z > b_dimless + 1e-7)
            .collect();

        // Sort ascending: [liquid_root, ..., vapor_root]
        valid_roots.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
        valid_roots.dedup_by(|a, b| (*a - *b).abs() < 1e-6);

        if valid_roots.is_empty() {
            return Err(PhysicsError::EosRootError {
                message: format!(
                    "no physically valid real root (Z > B={b_dimless}) found at P={p_pa} Pa, T={t_k} K"
                ),
            });
        }

        Ok(valid_roots)
    }

    /// Selects the appropriate root for the specified phase target (Vapor or Liquid).
    pub fn select_root(&self, valid_roots: &[f64], target: PhaseTarget) -> f64 {
        match target {
            PhaseTarget::Vapor => *valid_roots.last().expect("at least one root"),
            PhaseTarget::Liquid => *valid_roots.first().expect("at least one root"),
        }
    }

    /// Calculates component fugacity coefficients $\\ln(\\phi_i)$ and $\\phi_i$:
    /// Calculates component fugacity coefficients $\ln(\phi_i)$ and $\phi_i$:
    ///
    /// PR ($u=2, w=-1$):
    /// $\ln\phi_i = \frac{b_i}{b}(Z - 1) - \ln(Z - B) - \frac{A}{2\sqrt{2}B} \left(\frac{2 \sum_j x_j a_{ij}}{a} - \frac{b_i}{b}\right) \ln\left(\frac{Z + (1+\sqrt{2})B}{Z + (1-\sqrt{2})B}\right)$
    ///
    /// SRK ($u=1, w=0$):
    /// $\ln\phi_i = \frac{b_i}{b}(Z - 1) - \ln(Z - B) - \frac{A}{B} \left(\frac{2 \sum_j x_j a_{ij}}{a} - \frac{b_i}{b}\right) \ln\left(\frac{Z + B}{Z}\right)$
    #[allow(clippy::too_many_arguments)]
    pub fn fugacity_coefficients(
        &self,
        mole_fractions: &IndexMap<String, f64>,
        a_mix: f64,
        b_mix: f64,
        pure_params: &IndexMap<String, PureComponentParameters>,
        p_pa: f64,
        t_k: f64,
        z: f64,
    ) -> Result<IndexMap<String, f64>, PhysicsError> {
        let (u, w, _, _) = self.constants();
        let rt = R_GAS_CONSTANT * t_k;
        let a_dimless = (a_mix * p_pa) / (rt * rt);
        let b_dimless = (b_mix * p_pa) / rt;

        if z <= b_dimless {
            return Err(PhysicsError::EosRootError {
                message: format!(
                    "compressibility factor Z={z} <= B={b_dimless} invalid for fugacity"
                ),
            });
        }

        let z_minus_b = z - b_dimless;
        let ln_z_minus_b = z_minus_b.ln();

        let delta_u_w = (u * u - 4.0 * w).sqrt();
        let ln_ratio = match self.eos_type {
            CubicEosType::PengRobinson => {
                let num = z + (1.0 + 2.0_f64.sqrt()) * b_dimless;
                let den = z + (1.0 - 2.0_f64.sqrt()) * b_dimless;
                if den <= 0.0 {
                    return Err(PhysicsError::EosRootError {
                        message: format!("denominator in PR fugacity non-positive: {den}"),
                    });
                }
                (num / den).ln()
            }
            CubicEosType::SoaveRedlichKwong => ((z + b_dimless) / z).ln(),
        };

        let mut phis = IndexMap::with_capacity(mole_fractions.len());
        let comp_names: Vec<&String> = mole_fractions.keys().collect();

        for &name_i in &comp_names {
            let bi = pure_params[name_i].b;
            let pi = &pure_params[name_i];

            let mut sum_xj_aij = 0.0;
            for &name_j in &comp_names {
                let xj = mole_fractions[name_j];
                let pj = &pure_params[name_j];
                let kij = self.database.get_kij(name_i, name_j);
                let a_ij = (pi.a * pj.a).sqrt() * (1.0 - kij);
                sum_xj_aij += xj * a_ij;
            }

            let term_a = (2.0 * sum_xj_aij / a_mix) - (bi / b_mix);
            let factor = a_dimless / (b_dimless * delta_u_w);

            let ln_phi_i = (bi / b_mix) * (z - 1.0) - ln_z_minus_b - factor * term_a * ln_ratio;

            if !ln_phi_i.is_finite() {
                return Err(PhysicsError::NonFiniteValue {
                    field: format!("ln_phi_{name_i}"),
                    value: ln_phi_i,
                });
            }

            phis.insert(name_i.clone(), ln_phi_i.exp());
        }

        Ok(phis)
    }

    /// Evaluates residual molar enthalpy $H_m^{\\text{res}}$ in J/mol:
    ///
    /// PR:
    /// $H_m^{\\text{res}} = RT(Z - 1) + \\frac{T \\frac{da}{dT} - a}{2\\sqrt{2}b} \\ln\\left(\\frac{Z + (1+\\sqrt{2})B}{Z + (1-\\sqrt{2})B}\\right)$
    ///
    /// SRK:
    /// $H_m^{\\text{res}} = RT(Z - 1) + \\frac{T \\frac{da}{dT} - a}{b} \\ln\\left(\\frac{Z + B}{Z}\\right)$
    pub fn residual_molar_enthalpy(
        &self,
        a_mix: f64,
        b_mix: f64,
        da_mix_dt: f64,
        p_pa: f64,
        t_k: f64,
        z: f64,
    ) -> Result<f64, PhysicsError> {
        let rt = R_GAS_CONSTANT * t_k;
        let b_dimless = (b_mix * p_pa) / rt;

        let ln_ratio = match self.eos_type {
            CubicEosType::PengRobinson => {
                let num = z + (1.0 + 2.0_f64.sqrt()) * b_dimless;
                let den = z + (1.0 - 2.0_f64.sqrt()) * b_dimless;
                if den <= 0.0 {
                    return Err(PhysicsError::EosRootError {
                        message: "invalid denominator in PR enthalpy departure".to_string(),
                    });
                }
                (num / den).ln()
            }
            CubicEosType::SoaveRedlichKwong => ((z + b_dimless) / z).ln(),
        };

        let delta_u_w = match self.eos_type {
            CubicEosType::PengRobinson => 2.0 * 2.0_f64.sqrt(),
            CubicEosType::SoaveRedlichKwong => 1.0,
        };

        let term1 = rt * (z - 1.0);
        let term2 = ((t_k * da_mix_dt - a_mix) / (delta_u_w * b_mix)) * ln_ratio;

        Ok(term1 + term2)
    }

    /// Evaluates residual molar entropy $S_m^{\\text{res}}$ in J/(mol·K):
    ///
    /// PR:
    /// $S_m^{\\text{res}} = R \\ln(Z - B) + \\frac{\\frac{da}{dT}}{2\\sqrt{2}b} \\ln\\left(\\frac{Z + (1+\\sqrt{2})B}{Z + (1-\\sqrt{2})B}\\right)$
    ///
    /// SRK:
    /// $S_m^{\\text{res}} = R \\ln(Z - B) + \\frac{\\frac{da}{dT}}{b} \\ln\\left(\\frac{Z + B}{Z}\\right)$
    pub fn residual_molar_entropy(
        &self,
        b_mix: f64,
        da_mix_dt: f64,
        p_pa: f64,
        t_k: f64,
        z: f64,
    ) -> Result<f64, PhysicsError> {
        let r = R_GAS_CONSTANT;
        let rt = r * t_k;
        let b_dimless = (b_mix * p_pa) / rt;

        if z <= b_dimless {
            return Err(PhysicsError::EosRootError {
                message: "Z <= B in entropy departure".to_string(),
            });
        }

        let ln_z_minus_b = (z - b_dimless).ln();

        let (ln_ratio, delta_u_w) = match self.eos_type {
            CubicEosType::PengRobinson => {
                let num = z + (1.0 + 2.0_f64.sqrt()) * b_dimless;
                let den = z + (1.0 - 2.0_f64.sqrt()) * b_dimless;
                ((num / den).ln(), 2.0 * 2.0_f64.sqrt())
            }
            CubicEosType::SoaveRedlichKwong => (((z + b_dimless) / z).ln(), 1.0),
        };

        let term1 = r * ln_z_minus_b;
        let term2 = (da_mix_dt / (delta_u_w * b_mix)) * ln_ratio;

        Ok(term1 + term2)
    }

    /// Evaluates ideal gas molar enthalpy $H_m^{\\text{ig}}(T)$ in J/mol:
    ///
    /// $H_m^{\\text{ig}} = \\sum x_i \\left[ \\Delta H_{f, i}^\\circ + \\int_{T_{\\text{ref}}}^T C_{p, i}^{\\text{ig}}(T') dT' \\right]$
    pub fn ideal_molar_enthalpy(
        &self,
        mole_fractions: &IndexMap<String, f64>,
        t_k: f64,
    ) -> Result<f64, PhysicsError> {
        let mut h_ig = 0.0;
        for (name, &xi) in mole_fractions {
            let comp = self
                .database
                .get(name)
                .ok_or_else(|| PhysicsError::MissingProperty {
                    name: format!("component '{name}' in ideal enthalpy"),
                })?;
            let h_i = comp.reference_enthalpy + comp.delta_h_ideal(t_k);
            h_ig += xi * h_i;
        }
        Ok(h_ig)
    }

    /// Evaluates ideal gas molar entropy $S_m^{\\text{ig}}(T, P)$ in J/(mol·K):
    ///
    /// $S_m^{\\text{ig}} = \\sum x_i \\left[ S_i^\\circ + \\int_{T_{\\text{ref}}}^T \\frac{C_{p, i}^{\\text{ig}}(T')}{T'} dT' - R \\ln\\left(\\frac{P x_i}{P_{\\text{ref}}}\\right) \\right]$
    pub fn ideal_molar_entropy(
        &self,
        mole_fractions: &IndexMap<String, f64>,
        p_pa: f64,
        t_k: f64,
    ) -> Result<f64, PhysicsError> {
        let r = R_GAS_CONSTANT;
        let mut s_ig = 0.0;
        for (name, &xi) in mole_fractions {
            if xi <= 1e-18 {
                continue;
            }
            let comp = self
                .database
                .get(name)
                .ok_or_else(|| PhysicsError::MissingProperty {
                    name: format!("component '{name}' in ideal entropy"),
                })?;
            let delta_s_temp = comp.delta_s_ideal_temp(t_k);
            let p_part = p_pa * xi;
            let delta_s_press = -r * (p_part / STANDARD_REFERENCE_PRESSURE_PA).ln();

            let s_i = comp.reference_entropy + delta_s_temp + delta_s_press;
            s_ig += xi * s_i;
        }
        Ok(s_ig)
    }

    /// Computes average mixture molecular weight in kg/mol.
    pub fn mixture_molecular_weight(
        &self,
        mole_fractions: &IndexMap<String, f64>,
    ) -> Result<f64, PhysicsError> {
        let mut mw = 0.0;
        for (name, &xi) in mole_fractions {
            let comp = self
                .database
                .get(name)
                .ok_or_else(|| PhysicsError::MissingProperty {
                    name: format!("component '{name}' in mixture MW"),
                })?;
            mw += xi * comp.molecular_weight;
        }
        if mw <= 0.0 || !mw.is_finite() {
            return Err(PhysicsError::InvalidComposition {
                reason: format!("invalid mixture molecular weight: {mw}"),
            });
        }
        Ok(mw)
    }

    /// Evaluates complete single-phase thermodynamic state at $(P, T, \\mathbf{x})$.
    pub fn evaluate_phase_state(
        &self,
        mole_fractions: &IndexMap<String, f64>,
        p_pa: f64,
        t_k: f64,
        phase_target: PhaseTarget,
    ) -> Result<CubicEosState, PhysicsError> {
        if p_pa <= 0.0 {
            return Err(PhysicsError::NegativePressure {
                value: p_pa,
                context: "CubicEos::evaluate_phase_state".to_string(),
            });
        }
        if t_k <= 0.0 {
            return Err(PhysicsError::NonPositiveTemperature {
                value: t_k,
                context: "CubicEos::evaluate_phase_state".to_string(),
            });
        }

        let (a_mix, b_mix, da_mix_dt, pure_params) =
            self.mixture_parameters(mole_fractions, t_k)?;
        let roots = self.solve_cubic_roots(a_mix, b_mix, p_pa, t_k)?;
        let z = self.select_root(&roots, phase_target);

        let mw = self.mixture_molecular_weight(mole_fractions)?;

        // Molar volume v = Z R T / P in m³/mol
        let v_molar = (z * R_GAS_CONSTANT * t_k) / p_pa;
        let rho_molar = 1.0 / v_molar; // mol/m³
        let rho_mass = rho_molar * mw; // kg/m³

        let phis =
            self.fugacity_coefficients(mole_fractions, a_mix, b_mix, &pure_params, p_pa, t_k, z)?;

        let mut fugacities = IndexMap::with_capacity(mole_fractions.len());
        for (name, &xi) in mole_fractions {
            let phi = phis.get(name).copied().unwrap_or(1.0);
            let fi = xi * phi * p_pa;
            fugacities.insert(name.clone(), fi);
        }

        let h_res = self.residual_molar_enthalpy(a_mix, b_mix, da_mix_dt, p_pa, t_k, z)?;
        let h_ig = self.ideal_molar_enthalpy(mole_fractions, t_k)?;
        let h_molar = h_ig + h_res;
        let h_spec = h_molar / mw;

        let s_res = self.residual_molar_entropy(b_mix, da_mix_dt, p_pa, t_k, z)?;
        let s_ig = self.ideal_molar_entropy(mole_fractions, p_pa, t_k)?;
        let s_molar = s_ig + s_res;
        let s_spec = s_molar / mw;

        Ok(CubicEosState {
            compressibility_factor: z,
            molar_volume: v_molar,
            mass_density: rho_mass,
            molar_density: rho_molar,
            specific_enthalpy: h_spec,
            molar_enthalpy: h_molar,
            residual_molar_enthalpy: h_res,
            specific_entropy: s_spec,
            molar_entropy: s_molar,
            residual_molar_entropy: s_res,
            fugacity_coefficients: phis,
            component_fugacities: fugacities,
        })
    }
}
