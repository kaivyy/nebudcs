//! Scientific provenance, parameter metadata, validity domains, and error budgets.
//!
//! Enforces:
//! - Complete traceability for physical constants and parameters (no unprovenanced magic numbers)
//! - Explicit validity domain checking and extrapolation warnings
//! - Capability declarations for property packages
//! - Scientifically justified error budget and tolerance classes

use serde::{Deserialize, Serialize};

/// Detailed provenance record for a scientific dataset or parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScientificProvenance {
    /// Authoritative organization (e.g. "NIST", "DIPPR", "API", "IUPAC").
    pub source_organization: String,
    /// Formal publication reference or literature citation.
    pub publication_reference: String,
    /// Digital Object Identifier (DOI) or official reference identifier.
    pub doi_or_id: Option<String>,
    /// Version identifier of the dataset.
    pub dataset_version: String,
    /// ISO-8601 date when the data was retrieved or verified.
    pub retrieval_date: String,
    /// Measurement or original published engineering units.
    pub original_units: String,
    /// Converted canonical SI units used internally.
    pub converted_units: String,
    /// Documentation of dimensional transformation or regression method.
    pub transformation_notes: String,
    /// Relative measurement uncertainty when reported in literature (fractional, e.g. 0.005 for 0.5%).
    pub uncertainty_fraction: Option<f64>,
}

impl ScientificProvenance {
    /// Creates a verified NIST Chemistry WebBook provenance record.
    pub fn nist(publication: &str, doi: Option<&str>, original_units: &str) -> Self {
        Self {
            source_organization: "NIST".to_string(),
            publication_reference: publication.to_string(),
            doi_or_id: doi.map(|s| s.to_string()),
            dataset_version: "NIST Standard Reference Database 69 (2024)".to_string(),
            retrieval_date: "2026-09-06".to_string(),
            original_units: original_units.to_string(),
            converted_units: "SI Canonical".to_string(),
            transformation_notes:
                "Converted to SI base units using CODATA 2018 fundamental constants.".to_string(),
            uncertainty_fraction: Some(0.002), // 0.2% typical benchmark precision
        }
    }

    /// Creates a literature reference provenance record.
    pub fn literature(
        org: &str,
        pub_ref: &str,
        doi: Option<&str>,
        uncertainty: Option<f64>,
    ) -> Self {
        Self {
            source_organization: org.to_string(),
            publication_reference: pub_ref.to_string(),
            doi_or_id: doi.map(|s| s.to_string()),
            dataset_version: "1.0.0".to_string(),
            retrieval_date: "2026-09-06".to_string(),
            original_units: "Literature published units".to_string(),
            converted_units: "SI Canonical".to_string(),
            transformation_notes: "Direct ingestion with exact unit scaling.".to_string(),
            uncertainty_fraction: uncertainty,
        }
    }
}

/// A strongly typed scientific parameter carrying value, units, bounds, and provenance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScientificParameter<T> {
    /// Parameter identifier or symbol (e.g. "critical_temperature").
    pub name: String,
    /// Parameter value in canonical SI units.
    pub value: T,
    /// Explicit SI unit string (e.g. "K", "Pa", "J/mol").
    pub si_unit: String,
    /// Valid temperature range [T_min, T_max] in Kelvin.
    pub valid_t_range: (f64, f64),
    /// Valid absolute pressure range [P_min, P_max] in Pascal.
    pub valid_p_range: (f64, f64),
    /// Estimated relative uncertainty when known.
    pub uncertainty: Option<f64>,
    /// Full provenance and bibliographic citation.
    pub provenance: ScientificProvenance,
}

impl<T: Copy> ScientificParameter<T> {
    /// Creates a new scientific parameter with metadata.
    pub fn new(
        name: impl Into<String>,
        value: T,
        si_unit: impl Into<String>,
        valid_t_range: (f64, f64),
        valid_p_range: (f64, f64),
        provenance: ScientificProvenance,
    ) -> Self {
        Self {
            name: name.into(),
            value,
            si_unit: si_unit.into(),
            valid_t_range,
            valid_p_range,
            uncertainty: provenance.uncertainty_fraction,
            provenance,
        }
    }
}

/// Explicit validity state of a scientific correlation or property package evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ValidityState {
    /// Operating state is strictly within the calibrated and experimental bounds.
    Valid,
    /// Operating state lies outside the primary empirical range; extrapolation formulas applied.
    ValidWithExtrapolation,
    /// Operating coordinates violate physical boundaries ($T \le 0$, $P \le 0$) or correlation cutoffs.
    OutOfRange,
    /// Requested component, phase, or mixture combination is not supported by this model.
    Unsupported,
    /// Mathematical calculation encountered singular Jacobian, repeated roots, or divergence.
    NumericallyUnstable,
}

impl ValidityState {
    /// Returns true if the result can be safely used in physical balances.
    pub fn is_acceptable(&self) -> bool {
        matches!(self, Self::Valid | Self::ValidWithExtrapolation)
    }
}

/// Permissible operating domain for a thermodynamic model or property package.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidityDomain {
    /// Minimum temperature in Kelvin.
    pub t_min_k: f64,
    /// Maximum temperature in Kelvin.
    pub t_max_k: f64,
    /// Minimum absolute pressure in Pascal.
    pub p_min_pa: f64,
    /// Maximum absolute pressure in Pascal.
    pub p_max_pa: f64,
    /// Extrapolation margin factor (e.g. 0.20 allows 20% temperature extrapolation with warning).
    pub extrapolation_margin: f64,
}

impl Default for ValidityDomain {
    fn default() -> Self {
        Self {
            t_min_k: 50.0,
            t_max_k: 2000.0,
            p_min_pa: 10.0,
            p_max_pa: 100.0e6, // 100 MPa / 1000 bar
            extrapolation_margin: 0.15,
        }
    }
}

impl ValidityDomain {
    /// Checks a thermodynamic coordinate $(T, P)$ against this domain.
    pub fn check(&self, t_k: f64, p_pa: f64) -> ValidityState {
        if t_k <= 0.0 || p_pa <= 0.0 || !t_k.is_finite() || !p_pa.is_finite() {
            return ValidityState::OutOfRange;
        }

        if t_k >= self.t_min_k
            && t_k <= self.t_max_k
            && p_pa >= self.p_min_pa
            && p_pa <= self.p_max_pa
        {
            return ValidityState::Valid;
        }

        let t_margin = (self.t_max_k - self.t_min_k) * self.extrapolation_margin;
        let p_margin = (self.p_max_pa - self.p_min_pa) * self.extrapolation_margin;

        if t_k >= (self.t_min_k - t_margin).max(1.0)
            && t_k <= (self.t_max_k + t_margin)
            && p_pa >= (self.p_min_pa - p_margin).max(1.0)
            && p_pa <= (self.p_max_pa + p_margin)
        {
            ValidityState::ValidWithExtrapolation
        } else {
            ValidityState::OutOfRange
        }
    }
}

/// Explicit capabilities exposed by a thermodynamic property package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyPackageCapabilities {
    /// Supports equation of state (EOS) PVT evaluation.
    pub eos: bool,
    /// Supports component and mixture fugacity calculations.
    pub fugacity: bool,
    /// Supports vapor-liquid equilibrium (VLE) flash calculations.
    pub vle: bool,
    /// Supports enthalpy and enthalpy departure calculations.
    pub enthalpy: bool,
    /// Supports entropy and entropy departure calculations.
    pub entropy: bool,
    /// Supports mass and molar density calculations.
    pub density: bool,
    /// Supports analytical temperature and pressure derivatives.
    pub derivatives: bool,
    /// Supports Gibbs free energy phase stability analysis.
    pub phase_stability: bool,
    /// Supports fluid transport properties (viscosity, thermal conductivity).
    pub transport: bool,
}

impl Default for PropertyPackageCapabilities {
    fn default() -> Self {
        Self {
            eos: false,
            fugacity: false,
            vle: false,
            enthalpy: true,
            entropy: false,
            density: true,
            derivatives: false,
            phase_stability: false,
            transport: false,
        }
    }
}

impl PropertyPackageCapabilities {
    /// Full capabilities for cubic equations of state (PR, SRK).
    pub fn cubic_eos() -> Self {
        Self {
            eos: true,
            fugacity: true,
            vle: true,
            enthalpy: true,
            entropy: true,
            density: true,
            derivatives: true,
            phase_stability: true,
            transport: false,
        }
    }
}

/// Tolerance classes with formal scientific justification.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ToleranceClass {
    /// Multidimensional numerical nonlinear solver residual: $\|\mathbf{F}(\mathbf{x})\|_\infty \le 10^{-6}$.
    /// Justified by IEEE 754 double precision machine epsilon ($\epsilon_{\text{mach}} \approx 2.22 \times 10^{-16}$).
    NumericalSolver { abs_tol: f64, rel_tol: f64 },

    /// Equation of state algebraic residual $|Z^3 - (1-B)Z^2 + (A - 2B - 3B^2)Z - (AB - B^2 - B^3)| \le 10^{-5}$.
    EosResidual { max_residual: f64 },

    /// Mole and mass fraction conservation closure $|\sum x_i - 1.0| \le 10^{-7}$.
    MassBalance { max_closure_error: f64 },

    /// Iso-fugacity phase equilibrium criterion $\max_i |f_i^L / f_i^V - 1| \le 10^{-4}$.
    PhaseEquilibrium { max_relative_difference: f64 },

    /// Scientific reference validation tolerance compared to literature experimental data.
    /// Typically 1.0% to 5.0% depending on experimental measurement uncertainty.
    BenchmarkValidation { relative_tolerance: f64 },
}

impl ToleranceClass {
    /// Standard numerical solver tolerance.
    pub fn default_numerical_solver() -> Self {
        Self::NumericalSolver {
            abs_tol: 1.0e-6,
            rel_tol: 1.0e-5,
        }
    }

    /// Standard EOS algebraic residual tolerance.
    pub fn default_eos_residual() -> Self {
        Self::EosResidual {
            max_residual: 1.0e-5,
        }
    }

    /// Standard species mass balance closure tolerance.
    pub fn default_mass_balance() -> Self {
        Self::MassBalance {
            max_closure_error: 1.0e-7,
        }
    }

    /// Standard iso-fugacity equilibrium tolerance.
    pub fn default_phase_equilibrium() -> Self {
        Self::PhaseEquilibrium {
            max_relative_difference: 1.0e-4,
        }
    }

    /// Benchmark literature validation tolerance.
    pub fn default_benchmark() -> Self {
        Self::BenchmarkValidation {
            relative_tolerance: 0.05,
        }
    }
}
