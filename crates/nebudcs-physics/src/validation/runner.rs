//! Unified physics validation benchmark suite runner.

use serde::{Deserialize, Serialize};
use crate::validation::benchmarks::*;
use crate::validation::evidence::{
    EvidenceClassification, ScientificCitation, ToleranceThresholdProvenance, ValidationStatus,
};

/// Record of a completed scientific benchmark case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationCaseRecord {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub model_id: String,
    pub property_name: String,
    pub reference_source: String,
    pub citation: ScientificCitation,
    pub evidence_class: EvidenceClassification,
    pub expected_value: f64,
    pub predicted_value: f64,
    pub abs_error: f64,
    pub rel_error: f64,
    pub tolerance: f64,
    pub tolerance_provenance: ToleranceThresholdProvenance,
    pub status: ValidationStatus,
    pub passed: bool,
    pub notes: String,
}

/// Runs all built-in physics, thermodynamic, reaction, and hydraulic benchmarks.
pub fn run_all_physics_benchmarks() -> Vec<ValidationCaseRecord> {
    let mut records = Vec::new();

    // 1. Thermo benchmarks
    let t1 = benchmark_methane_compressibility_nist();
    records.push(ValidationCaseRecord {
        id: t1.case_id.to_string(),
        name: t1.case_name.to_string(),
        domain: "Thermodynamics".to_string(),
        model_id: "model.thermo.peng_robinson".to_string(),
        property_name: t1.property_name.to_string(),
        reference_source: t1.reference_source.to_string(),
        citation: t1.citation,
        evidence_class: t1.evidence_class,
        expected_value: t1.expected_value,
        predicted_value: t1.predicted_value,
        abs_error: t1.abs_error,
        rel_error: t1.rel_error,
        tolerance: t1.tolerance,
        tolerance_provenance: t1.tolerance_provenance,
        status: t1.status,
        passed: t1.passed,
        notes: t1.notes.to_string(),
    });

    let t2 = benchmark_binary_vle_rachford_rice();
    records.push(ValidationCaseRecord {
        id: t2.case_id.to_string(),
        name: t2.case_name.to_string(),
        domain: "Thermodynamics".to_string(),
        model_id: "model.thermo.rachford_rice_vle".to_string(),
        property_name: t2.property_name.to_string(),
        reference_source: t2.reference_source.to_string(),
        citation: t2.citation,
        evidence_class: t2.evidence_class,
        expected_value: t2.expected_value,
        predicted_value: t2.predicted_value,
        abs_error: t2.abs_error,
        rel_error: t2.rel_error,
        tolerance: t2.tolerance,
        tolerance_provenance: t2.tolerance_provenance,
        status: t2.status,
        passed: t2.passed,
        notes: t2.notes.to_string(),
    });

    let t3 = benchmark_bubble_dew_point_consistency();
    records.push(ValidationCaseRecord {
        id: t3.case_id.to_string(),
        name: t3.case_name.to_string(),
        domain: "Thermodynamics".to_string(),
        model_id: "model.thermo.bubble_dew_point".to_string(),
        property_name: t3.property_name.to_string(),
        reference_source: t3.reference_source.to_string(),
        citation: t3.citation,
        evidence_class: t3.evidence_class,
        expected_value: t3.expected_value,
        predicted_value: t3.predicted_value,
        abs_error: t3.abs_error,
        rel_error: t3.rel_error,
        tolerance: t3.tolerance,
        tolerance_provenance: t3.tolerance_provenance,
        status: t3.status,
        passed: t3.passed,
        notes: t3.notes.to_string(),
    });

    let t4 = benchmark_fugacity_consistency();
    records.push(ValidationCaseRecord {
        id: t4.case_id.to_string(),
        name: t4.case_name.to_string(),
        domain: "Thermodynamics".to_string(),
        model_id: "model.thermo.fugacity_consistency".to_string(),
        property_name: t4.property_name.to_string(),
        reference_source: t4.reference_source.to_string(),
        citation: t4.citation,
        evidence_class: t4.evidence_class,
        expected_value: t4.expected_value,
        predicted_value: t4.predicted_value,
        abs_error: t4.abs_error,
        rel_error: t4.rel_error,
        tolerance: t4.tolerance,
        tolerance_provenance: t4.tolerance_provenance,
        status: t4.status,
        passed: t4.passed,
        notes: t4.notes.to_string(),
    });

    // 2. Reaction benchmarks
    let r1 = benchmark_arrhenius_rate_constant();
    records.push(ValidationCaseRecord {
        id: r1.case_id.to_string(),
        name: r1.case_name.to_string(),
        domain: "ChemicalReaction".to_string(),
        model_id: "model.reaction.arrhenius_kinetics".to_string(),
        property_name: r1.property_name.to_string(),
        reference_source: r1.reference_source.to_string(),
        citation: r1.citation,
        evidence_class: r1.evidence_class,
        expected_value: r1.expected_value,
        predicted_value: r1.predicted_value,
        abs_error: r1.abs_error,
        rel_error: r1.rel_error,
        tolerance: r1.tolerance,
        tolerance_provenance: r1.tolerance_provenance,
        status: r1.status,
        passed: r1.passed,
        notes: r1.notes.to_string(),
    });

    let r2 = benchmark_cstr_analytical_conversion();
    records.push(ValidationCaseRecord {
        id: r2.case_id.to_string(),
        name: r2.case_name.to_string(),
        domain: "ChemicalReaction".to_string(),
        model_id: "model.reactor.cstr_isothermal".to_string(),
        property_name: r2.property_name.to_string(),
        reference_source: r2.reference_source.to_string(),
        citation: r2.citation,
        evidence_class: r2.evidence_class,
        expected_value: r2.expected_value,
        predicted_value: r2.predicted_value,
        abs_error: r2.abs_error,
        rel_error: r2.rel_error,
        tolerance: r2.tolerance,
        tolerance_provenance: r2.tolerance_provenance,
        status: r2.status,
        passed: r2.passed,
        notes: r2.notes.to_string(),
    });

    let r3 = benchmark_pfr_analytical_profile();
    records.push(ValidationCaseRecord {
        id: r3.case_id.to_string(),
        name: r3.case_name.to_string(),
        domain: "ChemicalReaction".to_string(),
        model_id: "model.reactor.pfr_1d".to_string(),
        property_name: r3.property_name.to_string(),
        reference_source: r3.reference_source.to_string(),
        citation: r3.citation,
        evidence_class: r3.evidence_class,
        expected_value: r3.expected_value,
        predicted_value: r3.predicted_value,
        abs_error: r3.abs_error,
        rel_error: r3.rel_error,
        tolerance: r3.tolerance,
        tolerance_provenance: r3.tolerance_provenance,
        status: r3.status,
        passed: r3.passed,
        notes: r3.notes.to_string(),
    });

    // 3. Hydraulic benchmarks
    let h1 = benchmark_friction_factor_turbulent();
    records.push(ValidationCaseRecord {
        id: h1.case_id.to_string(),
        name: h1.case_name.to_string(),
        domain: "Hydraulics".to_string(),
        model_id: "model.hydraulic.darcy_weisbach".to_string(),
        property_name: h1.property_name.to_string(),
        reference_source: h1.reference_source.to_string(),
        citation: h1.citation,
        evidence_class: h1.evidence_class,
        expected_value: h1.expected_value,
        predicted_value: h1.predicted_value,
        abs_error: h1.abs_error,
        rel_error: h1.rel_error,
        tolerance: h1.tolerance,
        tolerance_provenance: h1.tolerance_provenance,
        status: h1.status,
        passed: h1.passed,
        notes: h1.notes.to_string(),
    });

    let h2 = benchmark_valve_cv_flow();
    records.push(ValidationCaseRecord {
        id: h2.case_id.to_string(),
        name: h2.case_name.to_string(),
        domain: "Hydraulics".to_string(),
        model_id: "model.valve.control_globe_isa75".to_string(),
        property_name: h2.property_name.to_string(),
        reference_source: h2.reference_source.to_string(),
        citation: h2.citation,
        evidence_class: h2.evidence_class,
        expected_value: h2.expected_value,
        predicted_value: h2.predicted_value,
        abs_error: h2.abs_error,
        rel_error: h2.rel_error,
        tolerance: h2.tolerance,
        tolerance_provenance: h2.tolerance_provenance,
        status: h2.status,
        passed: h2.passed,
        notes: h2.notes.to_string(),
    });

    let h3 = benchmark_pump_head_and_npsh();
    records.push(ValidationCaseRecord {
        id: h3.case_id.to_string(),
        name: h3.case_name.to_string(),
        domain: "RotatingEquipment".to_string(),
        model_id: "model.pump.centrifugal_api610".to_string(),
        property_name: h3.property_name.to_string(),
        reference_source: h3.reference_source.to_string(),
        citation: h3.citation,
        evidence_class: h3.evidence_class,
        expected_value: h3.expected_value,
        predicted_value: h3.predicted_value,
        abs_error: h3.abs_error,
        rel_error: h3.rel_error,
        tolerance: h3.tolerance,
        tolerance_provenance: h3.tolerance_provenance,
        status: h3.status,
        passed: h3.passed,
        notes: h3.notes.to_string(),
    });

    let h4 = benchmark_compressor_performance_api617();
    records.push(ValidationCaseRecord {
        id: h4.case_id.to_string(),
        name: h4.case_name.to_string(),
        domain: "RotatingEquipment".to_string(),
        model_id: "model.compressor.centrifugal_api617".to_string(),
        property_name: h4.property_name.to_string(),
        reference_source: h4.reference_source.to_string(),
        citation: h4.citation,
        evidence_class: h4.evidence_class,
        expected_value: h4.expected_value,
        predicted_value: h4.predicted_value,
        abs_error: h4.abs_error,
        rel_error: h4.rel_error,
        tolerance: h4.tolerance,
        tolerance_provenance: h4.tolerance_provenance,
        status: h4.status,
        passed: h4.passed,
        notes: h4.notes.to_string(),
    });

    // 4. Heat Transfer benchmarks
    let ht1 = benchmark_lmtd_counter_current();
    records.push(ValidationCaseRecord {
        id: ht1.case_id.to_string(),
        name: ht1.case_name.to_string(),
        domain: "HeatTransfer".to_string(),
        model_id: "model.heat_transfer.lmtd".to_string(),
        property_name: ht1.property_name.to_string(),
        reference_source: ht1.reference_source.to_string(),
        citation: ht1.citation,
        evidence_class: ht1.evidence_class,
        expected_value: ht1.expected_value,
        predicted_value: ht1.predicted_value,
        abs_error: ht1.abs_error,
        rel_error: ht1.rel_error,
        tolerance: ht1.tolerance,
        tolerance_provenance: ht1.tolerance_provenance,
        status: ht1.status,
        passed: ht1.passed,
        notes: ht1.notes.to_string(),
    });

    let ht2 = benchmark_lmtd_taylor_series_limit();
    records.push(ValidationCaseRecord {
        id: ht2.case_id.to_string(),
        name: ht2.case_name.to_string(),
        domain: "HeatTransfer".to_string(),
        model_id: "model.heat_transfer.lmtd_taylor".to_string(),
        property_name: ht2.property_name.to_string(),
        reference_source: ht2.reference_source.to_string(),
        citation: ht2.citation,
        evidence_class: ht2.evidence_class,
        expected_value: ht2.expected_value,
        predicted_value: ht2.predicted_value,
        abs_error: ht2.abs_error,
        rel_error: ht2.rel_error,
        tolerance: ht2.tolerance,
        tolerance_provenance: ht2.tolerance_provenance,
        status: ht2.status,
        passed: ht2.passed,
        notes: ht2.notes.to_string(),
    });

    let ht3 = benchmark_epsilon_ntu_and_second_law();
    records.push(ValidationCaseRecord {
        id: ht3.case_id.to_string(),
        name: ht3.case_name.to_string(),
        domain: "HeatTransfer".to_string(),
        model_id: "model.heat_transfer.epsilon_ntu".to_string(),
        property_name: ht3.property_name.to_string(),
        reference_source: ht3.reference_source.to_string(),
        citation: ht3.citation,
        evidence_class: ht3.evidence_class,
        expected_value: ht3.expected_value,
        predicted_value: ht3.predicted_value,
        abs_error: ht3.abs_error,
        rel_error: ht3.rel_error,
        tolerance: ht3.tolerance,
        tolerance_provenance: ht3.tolerance_provenance,
        status: ht3.status,
        passed: ht3.passed,
        notes: ht3.notes.to_string(),
    });

    let ht4 = benchmark_tema_fouling_resistance();
    records.push(ValidationCaseRecord {
        id: ht4.case_id.to_string(),
        name: ht4.case_name.to_string(),
        domain: "HeatTransfer".to_string(),
        model_id: "model.heat_transfer.tema_fouling".to_string(),
        property_name: ht4.property_name.to_string(),
        reference_source: ht4.reference_source.to_string(),
        citation: ht4.citation,
        evidence_class: ht4.evidence_class,
        expected_value: ht4.expected_value,
        predicted_value: ht4.predicted_value,
        abs_error: ht4.abs_error,
        rel_error: ht4.rel_error,
        tolerance: ht4.tolerance,
        tolerance_provenance: ht4.tolerance_provenance,
        status: ht4.status,
        passed: ht4.passed,
        notes: ht4.notes.to_string(),
    });

    // 5. Steam benchmark
    let s1 = benchmark_iapws_steam_saturation_pressure();
    records.push(ValidationCaseRecord {
        id: s1.case_id.to_string(),
        name: s1.case_name.to_string(),
        domain: "PowerSteam".to_string(),
        model_id: "model.steam.iapws_if97".to_string(),
        property_name: s1.property_name.to_string(),
        reference_source: s1.reference_source.to_string(),
        citation: s1.citation,
        evidence_class: s1.evidence_class,
        expected_value: s1.expected_value,
        predicted_value: s1.predicted_value,
        abs_error: s1.abs_error,
        rel_error: s1.rel_error,
        tolerance: s1.tolerance,
        tolerance_provenance: s1.tolerance_provenance,
        status: s1.status,
        passed: s1.passed,
        notes: s1.notes.to_string(),
    });

    records
}
