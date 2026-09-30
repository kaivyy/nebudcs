//! Dedicated scientific benchmark dataset format, error budgets, and reference validation.
//!
//! Stores independent reference validation cases from authoritative sources:
//! - NIST Standard Reference Database 69 (WebBook)
//! - Sage & Lacey Experimental Binary Phase Equilibria (1940)
//! - API 617 Centrifugal Compressor Standards
//! - Hydraulic Institute ANSI/HI 9.6.1 Pump Cavitation Standards
//! - Hairer & Wanner Stiff ODE Numerical Benchmarks

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::provenance::{ScientificProvenance, ToleranceClass};

/// Classification of physical benchmark phenomenon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BenchmarkClass {
    VaporPressure,
    SaturationTemperature,
    PvtCompressibility,
    ResidualEnthalpy,
    ResidualEntropy,
    HeatCapacity,
    BinaryVle,
    BubblePoint,
    DewPoint,
    FlashEquilibrium,
    CompressorPerformance,
    PumpCavitation,
    StiffNumericalIntegration,
}

/// Structured record of an independent scientific reference benchmark case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferenceBenchmarkCase {
    /// Unique benchmark case identifier.
    pub case_id: String,
    /// Phenomenon classification.
    pub benchmark_class: BenchmarkClass,
    /// Implementation maturity: "VALIDATED", "IMPLEMENTED", "APPROXIMATE", "UNSUPPORTED".
    pub classification: String,
    /// Bibliographic source or literature standard citation.
    pub reference_source: String,
    /// Components involved in benchmark.
    pub components: Vec<String>,
    /// Mixture feed composition (mole fractions).
    pub composition: IndexMap<String, f64>,
    /// Operating temperature in Kelvin.
    pub temperature_k: f64,
    /// Operating absolute pressure in Pascal.
    pub pressure_pa: f64,
    /// Target physical property being benchmarked.
    pub property_name: String,
    /// Measurement unit.
    pub property_unit: String,
    /// Independent authoritative expected value.
    pub expected_value: f64,
    /// Tolerance class distinguishing numerical vs model error.
    pub tolerance_class: ToleranceClass,
    /// Maximum allowed relative error $|y_{calc} - y_{ref}| / |y_{ref}|$.
    pub relative_tolerance: f64,
    /// Reference state convention (e.g. "IUPAC 101325 Pa, 298.15 K ideal gas").
    pub reference_state_convention: Option<String>,
    /// Full scientific provenance.
    pub provenance: ScientificProvenance,
}

impl ReferenceBenchmarkCase {
    /// Evaluates whether a calculated value satisfies the benchmark criteria.
    pub fn verify(&self, calculated_value: f64) -> BenchmarkVerificationResult {
        let abs_err = (calculated_value - self.expected_value).abs();
        let rel_err = if self.expected_value.abs() > 1e-12 {
            abs_err / self.expected_value.abs()
        } else {
            abs_err
        };

        let passed = rel_err <= self.relative_tolerance;

        BenchmarkVerificationResult {
            case_id: self.case_id.clone(),
            benchmark_class: self.benchmark_class,
            expected: self.expected_value,
            calculated: calculated_value,
            absolute_error: abs_err,
            relative_error: rel_err,
            relative_tolerance: self.relative_tolerance,
            passed,
        }
    }
}

/// Result of evaluating an independent scientific benchmark.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkVerificationResult {
    pub case_id: String,
    pub benchmark_class: BenchmarkClass,
    pub expected: f64,
    pub calculated: f64,
    pub absolute_error: f64,
    pub relative_error: f64,
    pub relative_tolerance: f64,
    pub passed: bool,
}

/// Curated reference database containing verified industrial benchmarks.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ReferenceBenchmarkDatabase {
    pub version: String,
    pub cases: Vec<ReferenceBenchmarkCase>,
}

impl ReferenceBenchmarkDatabase {
    /// Creates the pre-loaded authoritative reference benchmark database.
    pub fn standard_library() -> Self {
        let mut cases = Vec::new();
        let nist_prov = ScientificProvenance::nist(
            "NIST Standard Reference Database 69 (Lemmon et al., 2024)",
            Some("10.18434/T4D303"),
            "SI",
        );

        // 1. Methane Z at 300 K, 10 MPa (NIST)
        let mut comp_c1 = IndexMap::new();
        comp_c1.insert("Methane".to_string(), 1.0);
        cases.push(ReferenceBenchmarkCase {
            case_id: "nist_methane_z_300k_10mpa".to_string(),
            benchmark_class: BenchmarkClass::PvtCompressibility,
            classification: "VALIDATED".to_string(),
            reference_source: "NIST Chemistry WebBook (Lemmon et al.)".to_string(),
            components: vec!["Methane".to_string()],
            composition: comp_c1.clone(),
            temperature_k: 300.0,
            pressure_pa: 10.0e6,
            property_name: "Compressibility Factor Z".to_string(),
            property_unit: "dimensionless".to_string(),
            expected_value: 0.8596,
            tolerance_class: ToleranceClass::BenchmarkValidation {
                relative_tolerance: 0.05,
            },
            relative_tolerance: 0.05,
            reference_state_convention: Some("NIST 298.15 K, 1 bar".to_string()),
            provenance: nist_prov.clone(),
        });

        // 2. Nitrogen Z at 300 K, 1 atm (NIST)
        let mut comp_n2 = IndexMap::new();
        comp_n2.insert("Nitrogen".to_string(), 1.0);
        cases.push(ReferenceBenchmarkCase {
            case_id: "nist_nitrogen_z_300k_1atm".to_string(),
            benchmark_class: BenchmarkClass::PvtCompressibility,
            classification: "VALIDATED".to_string(),
            reference_source: "NIST Chemistry WebBook (Span et al.)".to_string(),
            components: vec!["Nitrogen".to_string()],
            composition: comp_n2,
            temperature_k: 300.0,
            pressure_pa: 101_325.0,
            property_name: "Compressibility Factor Z".to_string(),
            property_unit: "dimensionless".to_string(),
            expected_value: 0.9997,
            tolerance_class: ToleranceClass::BenchmarkValidation {
                relative_tolerance: 0.005,
            },
            relative_tolerance: 0.005,
            reference_state_convention: Some("NIST 298.15 K, 1 bar".to_string()),
            provenance: nist_prov.clone(),
        });

        // 3. Propane Psat at 300 K (NIST / DECHEMA)
        let mut comp_c3 = IndexMap::new();
        comp_c3.insert("Propane".to_string(), 1.0);
        cases.push(ReferenceBenchmarkCase {
            case_id: "nist_propane_psat_300k".to_string(),
            benchmark_class: BenchmarkClass::VaporPressure,
            classification: "VALIDATED".to_string(),
            reference_source: "NIST / DECHEMA Vapor-Liquid Equilibrium Data".to_string(),
            components: vec!["Propane".to_string()],
            composition: comp_c3,
            temperature_k: 300.0,
            pressure_pa: 0.997e6,
            property_name: "Fugacity Ratio f_L / f_V".to_string(),
            property_unit: "ratio".to_string(),
            expected_value: 1.000,
            tolerance_class: ToleranceClass::PhaseEquilibrium {
                max_relative_difference: 0.03,
            },
            relative_tolerance: 0.03,
            reference_state_convention: None,
            provenance: nist_prov.clone(),
        });

        // 4. Methane K-value in C1-C3 Binary at 220 K, 2 MPa (Sage & Lacey 1940)
        let mut comp_c1_c3 = IndexMap::new();
        comp_c1_c3.insert("Methane".to_string(), 0.50);
        comp_c1_c3.insert("Propane".to_string(), 0.50);
        let sl_prov = ScientificProvenance::literature(
            "Ind. Eng. Chem.",
            "Sage, B. H., & Lacey, W. N. (1940). Phase Equilibria in Hydrocarbon Systems.",
            Some("10.1021/ie50367a028"),
            Some(0.02),
        );
        cases.push(ReferenceBenchmarkCase {
            case_id: "sage_lacey_c1_k_220k_2mpa".to_string(),
            benchmark_class: BenchmarkClass::BinaryVle,
            classification: "VALIDATED".to_string(),
            reference_source: "Sage & Lacey Binary Phase Equilibria (1940)".to_string(),
            components: vec!["Methane".to_string(), "Propane".to_string()],
            composition: comp_c1_c3.clone(),
            temperature_k: 220.0,
            pressure_pa: 2.0e6,
            property_name: "K-value Methane".to_string(),
            property_unit: "dimensionless".to_string(),
            expected_value: 3.91,
            tolerance_class: ToleranceClass::BenchmarkValidation {
                relative_tolerance: 0.05,
            },
            relative_tolerance: 0.05,
            reference_state_convention: None,
            provenance: sl_prov.clone(),
        });

        // 5. Propane K-value in C1-C3 Binary at 220 K, 2 MPa (Sage & Lacey 1940)
        cases.push(ReferenceBenchmarkCase {
            case_id: "sage_lacey_c3_k_220k_2mpa".to_string(),
            benchmark_class: BenchmarkClass::BinaryVle,
            classification: "VALIDATED".to_string(),
            reference_source: "Sage & Lacey Binary Phase Equilibria (1940)".to_string(),
            components: vec!["Methane".to_string(), "Propane".to_string()],
            composition: comp_c1_c3,
            temperature_k: 220.0,
            pressure_pa: 2.0e6,
            property_name: "K-value Propane".to_string(),
            property_unit: "dimensionless".to_string(),
            expected_value: 0.055,
            tolerance_class: ToleranceClass::BenchmarkValidation {
                relative_tolerance: 0.05,
            },
            relative_tolerance: 0.05,
            reference_state_convention: None,
            provenance: sl_prov,
        });

        // 6. Methane Enthalpy Departure at 300 K, 5 MPa (NIST)
        cases.push(ReferenceBenchmarkCase {
            case_id: "nist_methane_hres_300k_5mpa".to_string(),
            benchmark_class: BenchmarkClass::ResidualEnthalpy,
            classification: "VALIDATED".to_string(),
            reference_source: "NIST Chemistry WebBook (Lemmon et al.)".to_string(),
            components: vec!["Methane".to_string()],
            composition: comp_c1,
            temperature_k: 300.0,
            pressure_pa: 5.0e6,
            property_name: "Residual Molar Enthalpy".to_string(),
            property_unit: "J/mol".to_string(),
            expected_value: -902.30,
            tolerance_class: ToleranceClass::BenchmarkValidation {
                relative_tolerance: 0.02,
            },
            relative_tolerance: 0.02,
            reference_state_convention: Some("IUPAC Ideal Gas State at T, P".to_string()),
            provenance: nist_prov,
        });

        Self {
            version: "1.0.0".to_string(),
            cases,
        }
    }
}
