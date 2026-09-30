//! Generic scientific correlation abstraction and standard thermodynamic correlations.
//!
//! Provides structured, reusable correlations with explicit:
//! - Input variables and output quantities
//! - Engineering units
//! - Valid domain enforcement
//! - Analytical derivatives
//! - Scientific provenance

use serde::{Deserialize, Serialize};

use super::provenance::{ScientificProvenance, ValidityDomain, ValidityState};
use crate::error::PhysicsError;

/// Generic scientific correlation contract.
pub trait ScientificCorrelation<In, Out>: Send + Sync {
    /// Correlation name or model identification.
    fn name(&self) -> &str;

    /// List of input variable names (e.g. `["temperature"]`).
    fn input_variables(&self) -> &[&str];

    /// Output physical quantity name (e.g. `"heat_capacity"`).
    fn output_quantity(&self) -> &str;

    /// Output unit string in SI (e.g. `"J/(mol*K)"`).
    fn output_unit(&self) -> &str;

    /// Permissible operating domain.
    fn validity_domain(&self) -> &ValidityDomain;

    /// Full bibliographic provenance.
    fn provenance(&self) -> &ScientificProvenance;

    /// Evaluates the correlation at the given input coordinates.
    fn evaluate(&self, input: In) -> Result<(Out, ValidityState), PhysicsError>;

    /// Evaluates the first analytical derivative with respect to primary variable if supported.
    fn evaluate_derivative(&self, input: In) -> Result<Out, PhysicsError>;
}

// ---------------------------------------------------------------------------
// 1. Ideal Gas Heat Capacity Cp(T) Correlation
// ---------------------------------------------------------------------------

/// 5-term polynomial ideal gas heat capacity correlation:
///
/// $C_p^{ig}(T) = A + B T + C T^2 + D T^3 + E T^4 \quad [\text{J}/(\text{mol}\cdot\text{K})]$
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IdealGasCpCorrelation {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub domain: ValidityDomain,
    pub provenance: ScientificProvenance,
}

impl IdealGasCpCorrelation {
    /// Creates a new polynomial heat capacity correlation with domain and provenance.
    pub fn new(
        coeffs: [f64; 5],
        t_min_k: f64,
        t_max_k: f64,
        provenance: ScientificProvenance,
    ) -> Self {
        Self {
            a: coeffs[0],
            b: coeffs[1],
            c: coeffs[2],
            d: coeffs[3],
            e: coeffs[4],
            domain: ValidityDomain {
                t_min_k,
                t_max_k,
                p_min_pa: 0.0,
                p_max_pa: 1.0e9,
                extrapolation_margin: 0.15,
            },
            provenance,
        }
    }

    /// Evaluates definite enthalpy integral:
    ///
    /// $\Delta H^{ig}(T_{ref} \to T) = \int_{T_{ref}}^T C_p^{ig}(T) dT$ in J/mol.
    pub fn integrate_enthalpy(&self, t_ref_k: f64, t_k: f64) -> Result<f64, PhysicsError> {
        if t_ref_k <= 0.0 || t_k <= 0.0 {
            return Err(PhysicsError::NonPositiveTemperature {
                value: t_k.min(t_ref_k),
                context: "IdealGasCpCorrelation::integrate_enthalpy".to_string(),
            });
        }

        let int_t = self.indefinite_enthalpy_integral(t_k);
        let int_ref = self.indefinite_enthalpy_integral(t_ref_k);
        Ok(int_t - int_ref)
    }

    /// Evaluates definite entropy integral:
    ///
    /// $\Delta S^{ig}(T_{ref} \to T) = \int_{T_{ref}}^T \frac{C_p^{ig}(T)}{T} dT$ in J/(mol·K).
    pub fn integrate_entropy(&self, t_ref_k: f64, t_k: f64) -> Result<f64, PhysicsError> {
        if t_ref_k <= 0.0 || t_k <= 0.0 {
            return Err(PhysicsError::NonPositiveTemperature {
                value: t_k.min(t_ref_k),
                context: "IdealGasCpCorrelation::integrate_entropy".to_string(),
            });
        }

        let int_t = self.indefinite_entropy_integral(t_k);
        let int_ref = self.indefinite_entropy_integral(t_ref_k);
        Ok(int_t - int_ref)
    }

    fn indefinite_enthalpy_integral(&self, t: f64) -> f64 {
        let t2 = t * t;
        let t3 = t2 * t;
        let t4 = t3 * t;
        let t5 = t4 * t;

        self.a * t
            + (self.b / 2.0) * t2
            + (self.c / 3.0) * t3
            + (self.d / 4.0) * t4
            + (self.e / 5.0) * t5
    }

    fn indefinite_entropy_integral(&self, t: f64) -> f64 {
        let t2 = t * t;
        let t3 = t2 * t;
        let t4 = t3 * t;

        self.a * t.ln()
            + self.b * t
            + (self.c / 2.0) * t2
            + (self.d / 3.0) * t3
            + (self.e / 4.0) * t4
    }
}

impl ScientificCorrelation<f64, f64> for IdealGasCpCorrelation {
    fn name(&self) -> &str {
        "5-Term Polynomial Ideal Gas Heat Capacity"
    }

    fn input_variables(&self) -> &[&str] {
        &["temperature"]
    }

    fn output_quantity(&self) -> &str {
        "isobaric_heat_capacity"
    }

    fn output_unit(&self) -> &str {
        "J/(mol*K)"
    }

    fn validity_domain(&self) -> &ValidityDomain {
        &self.domain
    }

    fn provenance(&self) -> &ScientificProvenance {
        &self.provenance
    }

    fn evaluate(&self, t_k: f64) -> Result<(f64, ValidityState), PhysicsError> {
        let state = self.domain.check(t_k, 101_325.0);
        if state == ValidityState::OutOfRange {
            return Err(PhysicsError::PropertyOutOfRange {
                variable: "temperature".to_string(),
                value: t_k,
                min: self.domain.t_min_k,
                max: self.domain.t_max_k,
                context: "IdealGasCpCorrelation::evaluate".to_string(),
            });
        }

        let t2 = t_k * t_k;
        let t3 = t2 * t_k;
        let t4 = t3 * t_k;
        let cp = self.a + self.b * t_k + self.c * t2 + self.d * t3 + self.e * t4;

        if !cp.is_finite() || cp <= 0.0 {
            return Err(PhysicsError::NonFiniteValue {
                field: "heat_capacity".to_string(),
                value: cp,
            });
        }

        Ok((cp, state))
    }

    fn evaluate_derivative(&self, t_k: f64) -> Result<f64, PhysicsError> {
        if t_k <= 0.0 {
            return Err(PhysicsError::NonPositiveTemperature {
                value: t_k,
                context: "IdealGasCpCorrelation::evaluate_derivative".to_string(),
            });
        }

        let t2 = t_k * t_k;
        let t3 = t2 * t_k;
        // dCp/dT = B + 2 C T + 3 D T^2 + 4 E T^3
        let dcp_dt = self.b + 2.0 * self.c * t_k + 3.0 * self.d * t2 + 4.0 * self.e * t3;
        Ok(dcp_dt)
    }
}

// ---------------------------------------------------------------------------
// 2. Antoine Vapor Pressure Correlation
// ---------------------------------------------------------------------------

/// Antoine equation for saturation vapor pressure:
///
/// $\log_{10}(P_{\text{sat}} / \text{bar}) = A - \frac{B}{T + C} \quad (T \text{ in K})$
///
/// Evaluates $P_{\text{sat}}$ in Pascal ($\text{bar} \times 10^5$).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AntoineVaporPressureCorrelation {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub domain: ValidityDomain,
    pub provenance: ScientificProvenance,
}

impl AntoineVaporPressureCorrelation {
    /// Creates a new Antoine correlation with domain and provenance.
    pub fn new(
        a: f64,
        b: f64,
        c: f64,
        t_min_k: f64,
        t_max_k: f64,
        provenance: ScientificProvenance,
    ) -> Self {
        Self {
            a,
            b,
            c,
            domain: ValidityDomain {
                t_min_k,
                t_max_k,
                p_min_pa: 0.1,
                p_max_pa: 50.0e6,
                extrapolation_margin: 0.15,
            },
            provenance,
        }
    }
}

impl ScientificCorrelation<f64, f64> for AntoineVaporPressureCorrelation {
    fn name(&self) -> &str {
        "Antoine Vapor Pressure Correlation"
    }

    fn input_variables(&self) -> &[&str] {
        &["temperature"]
    }

    fn output_quantity(&self) -> &str {
        "saturation_pressure"
    }

    fn output_unit(&self) -> &str {
        "Pa"
    }

    fn validity_domain(&self) -> &ValidityDomain {
        &self.domain
    }

    fn provenance(&self) -> &ScientificProvenance {
        &self.provenance
    }

    fn evaluate(&self, t_k: f64) -> Result<(f64, ValidityState), PhysicsError> {
        let state = self.domain.check(t_k, 101_325.0);
        if state == ValidityState::OutOfRange {
            return Err(PhysicsError::PropertyOutOfRange {
                variable: "temperature".to_string(),
                value: t_k,
                min: self.domain.t_min_k,
                max: self.domain.t_max_k,
                context: "AntoineVaporPressureCorrelation::evaluate".to_string(),
            });
        }

        let denom = t_k + self.c;
        if denom.abs() < 1e-6 {
            return Err(PhysicsError::InvalidOperatingCondition {
                message: format!("Antoine denominator zero at T = {t_k} K"),
            });
        }

        let log10_p_bar = self.a - (self.b / denom);
        let p_bar = 10.0_f64.powf(log10_p_bar);
        let p_pa = p_bar * 100_000.0; // bar to Pa

        if !p_pa.is_finite() || p_pa <= 0.0 {
            return Err(PhysicsError::NonFiniteValue {
                field: "saturation_pressure".to_string(),
                value: p_pa,
            });
        }

        Ok((p_pa, state))
    }

    fn evaluate_derivative(&self, t_k: f64) -> Result<f64, PhysicsError> {
        let (p_pa, _) = self.evaluate(t_k)?;
        let denom = t_k + self.c;
        // d(P_sat)/dT = P_sat * ln(10) * B / (T + C)^2
        let dp_dt = p_pa * std::f64::consts::LN_10 * (self.b / (denom * denom));
        Ok(dp_dt)
    }
}

// ---------------------------------------------------------------------------
// 3. Watson Enthalpy of Vaporization Correlation
// ---------------------------------------------------------------------------

/// Watson correlation for temperature-dependent latent heat of vaporization:
///
/// $\Delta H_{\text{vap}}(T) = \Delta H_{\text{vap}, 0} \left(\frac{1 - T/T_c}{1 - T_0/T_c}\right)^{0.38} \quad [\text{J}/\text{mol}]$
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatsonHeatOfVaporizationCorrelation {
    pub h_vap_ref_j_mol: f64,
    pub t_ref_k: f64,
    pub t_crit_k: f64,
    pub domain: ValidityDomain,
    pub provenance: ScientificProvenance,
}

impl WatsonHeatOfVaporizationCorrelation {
    pub fn new(
        h_vap_ref_j_mol: f64,
        t_ref_k: f64,
        t_crit_k: f64,
        provenance: ScientificProvenance,
    ) -> Self {
        Self {
            h_vap_ref_j_mol,
            t_ref_k,
            t_crit_k,
            domain: ValidityDomain {
                t_min_k: 50.0,
                t_max_k: t_crit_k,
                p_min_pa: 0.1,
                p_max_pa: 50.0e6,
                extrapolation_margin: 0.05,
            },
            provenance,
        }
    }
}

impl ScientificCorrelation<f64, f64> for WatsonHeatOfVaporizationCorrelation {
    fn name(&self) -> &str {
        "Watson Heat of Vaporization Correlation"
    }

    fn input_variables(&self) -> &[&str] {
        &["temperature"]
    }

    fn output_quantity(&self) -> &str {
        "heat_of_vaporization"
    }

    fn output_unit(&self) -> &str {
        "J/mol"
    }

    fn validity_domain(&self) -> &ValidityDomain {
        &self.domain
    }

    fn provenance(&self) -> &ScientificProvenance {
        &self.provenance
    }

    fn evaluate(&self, t_k: f64) -> Result<(f64, ValidityState), PhysicsError> {
        let state = self.domain.check(t_k, 101_325.0);
        if t_k >= self.t_crit_k {
            // At and above critical point, latent heat of vaporization is identically zero
            return Ok((0.0, ValidityState::Valid));
        }

        let num = (1.0 - t_k / self.t_crit_k).max(0.0);
        let denom = (1.0 - self.t_ref_k / self.t_crit_k).max(1e-6);
        let ratio = num / denom;
        let h_vap = self.h_vap_ref_j_mol * ratio.powf(0.38);

        Ok((h_vap, state))
    }

    fn evaluate_derivative(&self, t_k: f64) -> Result<f64, PhysicsError> {
        if t_k >= self.t_crit_k {
            return Ok(0.0);
        }
        let (h_vap, _) = self.evaluate(t_k)?;
        let num = 1.0 - t_k / self.t_crit_k;
        if num <= 1e-6 {
            return Ok(0.0);
        }
        // dHvap/dT = -0.38 * Hvap / (Tc * (1 - T/Tc))
        let dh_dt = -0.38 * h_vap / (self.t_crit_k * num);
        Ok(dh_dt)
    }
}

// ---------------------------------------------------------------------------
// 4. Rackett Saturated Liquid Density Correlation
// ---------------------------------------------------------------------------

/// Rackett correlation for saturated liquid molar volume and mass density:
///
/// $V_{\text{m,liq}}(T) = V_c \cdot Z_c^{(1 - T/T_c)^{2/7}} \quad [\text{m}^3/\text{mol}]$
///
/// $\rho_{\text{liq}}(T) = \frac{M}{V_{\text{m,liq}}(T)} \quad [\text{kg}/\text{m}^3]$
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RackettLiquidDensityCorrelation {
    pub molecular_weight_kg_mol: f64,
    pub t_crit_k: f64,
    pub p_crit_pa: f64,
    pub v_crit_m3_mol: f64,
    pub z_crit: f64,
    pub domain: ValidityDomain,
    pub provenance: ScientificProvenance,
}

impl RackettLiquidDensityCorrelation {
    pub fn new(
        mw_kg_mol: f64,
        t_crit_k: f64,
        p_crit_pa: f64,
        v_crit_m3_mol: f64,
        z_crit: f64,
        provenance: ScientificProvenance,
    ) -> Self {
        Self {
            molecular_weight_kg_mol: mw_kg_mol,
            t_crit_k,
            p_crit_pa,
            v_crit_m3_mol,
            z_crit,
            domain: ValidityDomain {
                t_min_k: 50.0,
                t_max_k: t_crit_k * 0.999,
                p_min_pa: 0.1,
                p_max_pa: 50.0e6,
                extrapolation_margin: 0.05,
            },
            provenance,
        }
    }
}

impl ScientificCorrelation<f64, f64> for RackettLiquidDensityCorrelation {
    fn name(&self) -> &str {
        "Rackett Saturated Liquid Density Correlation"
    }

    fn input_variables(&self) -> &[&str] {
        &["temperature"]
    }

    fn output_quantity(&self) -> &str {
        "mass_density"
    }

    fn output_unit(&self) -> &str {
        "kg/m^3"
    }

    fn validity_domain(&self) -> &ValidityDomain {
        &self.domain
    }

    fn provenance(&self) -> &ScientificProvenance {
        &self.provenance
    }

    fn evaluate(&self, t_k: f64) -> Result<(f64, ValidityState), PhysicsError> {
        let state = self.domain.check(t_k, 101_325.0);
        if t_k >= self.t_crit_k {
            let rho_c = self.molecular_weight_kg_mol / self.v_crit_m3_mol;
            return Ok((rho_c, ValidityState::ValidWithExtrapolation));
        }

        let tr = t_k / self.t_crit_k;
        let exponent = (1.0 - tr).powf(2.0 / 7.0);
        let v_molar = self.v_crit_m3_mol * self.z_crit.powf(exponent);

        if v_molar <= 0.0 || !v_molar.is_finite() {
            return Err(PhysicsError::InvalidDensity {
                value: 0.0,
                context: "RackettLiquidDensityCorrelation::evaluate".to_string(),
            });
        }

        let rho = self.molecular_weight_kg_mol / v_molar;
        Ok((rho, state))
    }

    fn evaluate_derivative(&self, t_k: f64) -> Result<f64, PhysicsError> {
        if t_k >= self.t_crit_k {
            return Ok(0.0);
        }
        let (rho, _) = self.evaluate(t_k)?;
        let tr = t_k / self.t_crit_k;
        let diff = (1.0 - tr).max(1e-6);
        // d(rho)/dT using chain rule on V_molar
        let drho_dt =
            rho * (2.0 / (7.0 * self.t_crit_k)) * self.z_crit.ln() * diff.powf(-5.0 / 7.0);
        Ok(drho_dt)
    }
}
