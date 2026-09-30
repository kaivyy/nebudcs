//! NebuDCS Stage 13: Validation, Benchmark & Scientific Certification Framework
//! Acceptance Matrix Verification Suite (Cases A through T).
//!
//! Standards Reference:
//! - ASME V&V 10 / ASME V&V 20 (Verification and Validation in Computational Solid Mechanics / Fluid and Thermal Science)
//! - NIST REFPROP / Chemistry WebBook (SRD 69)
//! - API 610 (Centrifugal Pumps) / API 617 (Centrifugal Compressors)
//! - ANSI/ISA-75.01.01 (Control Valve Sizing) / ANSI/ISA-51.1 (Process Instrumentation Terminology)
//! - TEMA Standards (Tubular Exchanger Manufacturers Association)
//! - IAPWS-IF97 (Industrial Formulation for Water and Steam)
//! - IEC 61131-3 (Programmable Controllers - Programming Languages)
//! - IEC 61511 / ISA-84 (Functional Safety: Safety Instrumented Systems)
//!
//! CERTIFICATION LANGUAGE POLICY:
//! Permitted terms: "INTERNALLY VALIDATED", "BENCHMARKED", "ENGINEERING CONSISTENCY VERIFIED", "VALIDATED WITH LIMITATIONS".
//! Forbidden terms: "IEC CERTIFIED", "API CERTIFIED", "ISA CERTIFIED", "SIL CERTIFIED", "INDUSTRY CERTIFIED".

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

use nebudcs_control::validation::{
    benchmark_iec61131_conformance, benchmark_pid_step_response, benchmark_sis_safety_logic,
};
use nebudcs_physics::validation::benchmarks::*;
use nebudcs_protocol::{
    ValidationCaseDetailDto, ValidationCaseSummaryDto, ValidationExecutiveSummaryDto,
    ValidationManifestDto, ValidationRunResponseDto,
};
use nebudcs_server::{build_app, HostHandle, ServerConfig, SimulationHost};
use nebudcs_sim::SimulationEngine;

fn create_test_host(config: ServerConfig) -> (SimulationHost, HostHandle) {
    let mut engine = SimulationEngine::new(config.simulation_dt).expect("Failed to create engine");
    engine.clock_mut().play();
    let tag_store = nebudcs_control::signal::RuntimeTagStore::new();
    let host = SimulationHost::start(
        config,
        engine,
        tag_store,
        None,
        None,
        None,
    ).expect("Failed to start simulation host");
    let handle = host.handle();
    (host, handle)
}

// ===========================================================================
// Cases A through D: Thermodynamics & Phase Equilibrium Benchmarks
// ===========================================================================

#[test]
fn test_case_a_nist_methane_compressibility_factor_z() {
    let res = benchmark_methane_compressibility_nist();
    assert!(res.passed, "Case A NIST Methane Z failed: {:?}", res);
    assert_eq!(res.case_id, "BENCH-TH-001");
    assert!(res.rel_error <= 0.05, "Relative error exceeds 5%: {}", res.rel_error);
    assert_eq!(res.status.to_string(), "INTERNALLY_VALIDATED");
}

#[test]
fn test_case_b_binary_vle_flash_sage_and_lacey_tie_line() {
    let res = benchmark_binary_vle_rachford_rice();
    assert!(res.passed, "Case B Sage & Lacey binary VLE failed: {:?}", res);
    assert_eq!(res.case_id, "BENCH-TH-002");
    assert!(res.rel_error <= 0.05, "Relative error exceeds 5%: {}", res.rel_error);
    assert_eq!(res.status.to_string(), "INTERNALLY_VALIDATED");
}

#[test]
fn test_case_c_pure_component_vapor_pressure_perrys_handbook() {
    let res = benchmark_bubble_dew_point_consistency();
    assert!(res.passed, "Case C Bubble/Dew Point failed: {:?}", res);
    assert_eq!(res.case_id, "BENCH-TH-003");
    assert!(res.rel_error <= 0.05, "Relative error exceeds 5%: {}", res.rel_error);
    assert_eq!(res.status.to_string(), "INTERNALLY_VALIDATED");
}

#[test]
fn test_case_d_fugacity_consistency_thermodynamic_closure() {
    let res = benchmark_fugacity_consistency();
    assert!(res.passed, "Case D Fugacity consistency failed: {:?}", res);
    assert_eq!(res.case_id, "BENCH-TH-004");
    assert!(res.abs_error < 1.0e-4, "Fugacity ln(phi_V) - ln(phi_L) exceeds 1e-4: {}", res.abs_error);
    assert_eq!(res.status.to_string(), "INTERNALLY_VALIDATED");
}

// ===========================================================================
// Cases E through G: Chemical Reaction & Reactor Physics Benchmarks
// ===========================================================================

#[test]
fn test_case_e_arrhenius_kinetics_temperature_dependence_fogler() {
    let res = benchmark_arrhenius_rate_constant();
    assert!(res.passed, "Case E Arrhenius k(T) failed: {:?}", res);
    assert_eq!(res.case_id, "BENCH-RX-001");
    assert!(res.rel_error <= 0.01, "Arrhenius k(T) error exceeds 1%: {}", res.rel_error);
    assert_eq!(res.status.to_string(), "INTERNALLY_VALIDATED");
}

#[test]
fn test_case_f_cstr_analytical_conversion_levenspiel() {
    let res = benchmark_cstr_analytical_conversion();
    assert!(res.passed, "Case F CSTR analytical conversion failed: {:?}", res);
    assert_eq!(res.case_id, "BENCH-RX-002");
    assert!(res.rel_error <= 0.01, "CSTR conversion error exceeds 1%: {}", res.rel_error);
    assert_eq!(res.status.to_string(), "INTERNALLY_VALIDATED");
}

#[test]
fn test_case_g_pfr_analytical_concentration_profile_fogler() {
    let res = benchmark_pfr_analytical_profile();
    assert!(res.passed, "Case G PFR analytical profile failed: {:?}", res);
    assert_eq!(res.case_id, "BENCH-RX-003");
    assert!(res.rel_error <= 0.02, "PFR profile error exceeds 2%: {}", res.rel_error);
    assert_eq!(res.status.to_string(), "INTERNALLY_VALIDATED");
}

// ===========================================================================
// Cases H through K: Hydraulics & Rotating Equipment Benchmarks
// ===========================================================================

#[test]
fn test_case_h_darcy_weisbach_friction_factor_crane_tp410() {
    let res = benchmark_friction_factor_turbulent();
    assert!(res.passed, "Case H Darcy-Weisbach friction failed: {:?}", res);
    assert_eq!(res.case_id, "BENCH-HY-001");
    assert!(res.rel_error <= 0.03, "Friction factor error exceeds 3%: {}", res.rel_error);
    assert_eq!(res.status.to_string(), "INTERNALLY_VALIDATED");
}

#[test]
fn test_case_i_control_valve_flow_sizing_isa_75_01() {
    let res = benchmark_valve_cv_flow();
    assert!(res.passed, "Case I Valve Cv sizing failed: {:?}", res);
    assert_eq!(res.case_id, "BENCH-HY-002");
    assert!(res.rel_error <= 0.02, "Valve Cv flow error exceeds 2%: {}", res.rel_error);
    assert_eq!(res.status.to_string(), "INTERNALLY_VALIDATED");
}

#[test]
fn test_case_j_centrifugal_pump_head_npsh_api_610() {
    let res = benchmark_pump_head_and_npsh();
    assert!(res.passed, "Case J API 610 pump head & NPSH failed: {:?}", res);
    assert_eq!(res.case_id, "BENCH-HY-003");
    assert!(res.rel_error <= 0.03, "Pump head error exceeds 3%: {}", res.rel_error);
    assert_eq!(res.status.to_string(), "INTERNALLY_VALIDATED");
}

#[test]
fn test_case_k_centrifugal_compressor_performance_api_617() {
    let res = benchmark_compressor_performance_api617();
    assert!(res.passed, "Case K API 617 compressor failed: {:?}", res);
    assert_eq!(res.case_id, "BENCH-HY-004");
    assert!(res.rel_error <= 0.05, "Compressor error exceeds 5%: {}", res.rel_error);
    assert_eq!(res.status.to_string(), "INTERNALLY_VALIDATED");
}

// ===========================================================================
// Cases L, Q, R: Heat Transfer & Thermal Rating Benchmarks
// ===========================================================================

#[test]
fn test_case_l_lmtd_counter_current_and_smooth_taylor_limit() {
    let r1 = benchmark_lmtd_counter_current();
    assert!(r1.passed, "Case L LMTD counter-current failed: {:?}", r1);
    assert_eq!(r1.case_id, "BENCH-HT-001");
    assert!(r1.rel_error <= 0.001, "LMTD error exceeds 0.1%: {}", r1.rel_error);

    let r2 = benchmark_lmtd_taylor_series_limit();
    assert!(r2.passed, "Case L LMTD Taylor limit failed: {:?}", r2);
    assert_eq!(r2.case_id, "BENCH-HT-002");
    assert!(r2.abs_error < 1.0e-5, "Taylor limit residual exceeds 1e-5: {}", r2.abs_error);
}

#[test]
fn test_case_q_epsilon_ntu_and_second_law_thermodynamic_closure() {
    let res = benchmark_epsilon_ntu_and_second_law();
    assert!(res.passed, "Case Q Epsilon-NTU & 2nd law failed: {:?}", res);
    assert_eq!(res.case_id, "BENCH-HT-003");
    assert!(res.rel_error <= 0.01, "Epsilon-NTU error exceeds 1%: {}", res.rel_error);
    assert_eq!(res.status.to_string(), "INTERNALLY_VALIDATED");
}

#[test]
fn test_case_r_tema_standards_fouling_resistance_rating() {
    let res = benchmark_tema_fouling_resistance();
    assert!(res.passed, "Case R TEMA fouling rating failed: {:?}", res);
    assert_eq!(res.case_id, "BENCH-HT-004");
    assert!(res.rel_error <= 0.001, "TEMA U rating error exceeds 0.1%: {}", res.rel_error);
    assert_eq!(res.status.to_string(), "INTERNALLY_VALIDATED");
}

// ===========================================================================
// Cases M through P: Control, IEC 61131-3, SIS & Steam Benchmarks
// ===========================================================================

#[test]
fn test_case_m_closed_loop_pid_step_response_anti_windup_bumpless() {
    let res = benchmark_pid_step_response();
    assert!(res.passed, "Case M PID step response failed: {:?}", res);
    assert_eq!(res.case_id, "BENCH-CTL-001");
    assert!(res.steady_state_error < 0.10, "PID SS error exceeds 0.10: {}", res.steady_state_error);
    assert_eq!(res.status, "BENCHMARKED");
}

#[test]
fn test_case_n_iec61131_runtime_semantic_conformance() {
    let res = benchmark_iec61131_conformance();
    assert!(res.passed, "Case N IEC 61131 conformance failed: {:?}", res);
    assert_eq!(res.case_id, "BENCH-IEC-001");
    assert_eq!(res.status, "CONFORMANCE_VERIFIED");
}

#[test]
fn test_case_o_sis_safety_logic_2oo3_voting_latching_proof_test() {
    let res = benchmark_sis_safety_logic();
    assert!(res.passed, "Case O SIS safety logic failed: {:?}", res);
    assert_eq!(res.case_id, "BENCH-SIS-001");
    assert_eq!(res.status, "BENCHMARKED");
}

#[test]
fn test_case_p_iapws_steam_saturation_pressure_100c() {
    let res = benchmark_iapws_steam_saturation_pressure();
    assert!(res.passed, "Case P IAPWS steam Psat failed: {:?}", res);
    assert_eq!(res.case_id, "BENCH-ST-001");
    assert!(res.rel_error <= 0.002, "IAPWS saturation pressure error exceeds 0.2%: {}", res.rel_error);
    assert_eq!(res.status.to_string(), "INTERNALLY_VALIDATED");
}

// ===========================================================================
// Cases S & T: REST Certification Endpoints, RBAC & Conservation Invariant
// ===========================================================================

#[tokio::test]
async fn test_case_s_validation_rest_endpoints_and_rbac_enforcement() {
    let mut config = ServerConfig::default();
    config.admin_token = Some("admin-sec-key-123".to_string());
    config.operator_token = Some("oper-sec-key-456".to_string());
    config.trainee_token = Some("train-sec-key-789".to_string());

    let (_host, handle) = create_test_host(config);
    let app = build_app(handle);

    // 1. GET /api/v1/validation/summary (Trainee token allowed)
    let req = Request::builder()
        .uri("/api/v1/validation/summary")
        .header(header::AUTHORIZATION, "Bearer train-sec-key-789")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let summary: ValidationExecutiveSummaryDto = serde_json::from_slice(&body).unwrap();
    assert!(summary.total_cases >= 20);
    assert_eq!(summary.failed_cases, 0);
    assert_eq!(summary.overall_status, "INTERNALLY VALIDATED");
    assert!(summary.compliance_disclaimer.contains("DOES NOT claim official third-party"));

    // 2. GET /api/v1/validation/cases
    let req = Request::builder()
        .uri("/api/v1/validation/cases?domain=Thermodynamics")
        .header(header::AUTHORIZATION, "Bearer train-sec-key-789")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let cases: Vec<ValidationCaseSummaryDto> = serde_json::from_slice(&body).unwrap();
    assert!(!cases.is_empty());
    for c in &cases {
        assert_eq!(c.domain, "Thermodynamics");
        assert!(c.passed);
    }

    // 3. GET /api/v1/validation/cases/BENCH-TH-001
    let req = Request::builder()
        .uri("/api/v1/validation/cases/BENCH-TH-001")
        .header(header::AUTHORIZATION, "Bearer train-sec-key-789")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let detail: ValidationCaseDetailDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(detail.id, "BENCH-TH-001");
    assert!(detail.passed);
    assert!(detail.disclaimer.contains("DOES NOT claim"));

    // 4. GET /api/v1/validation/manifest
    let req = Request::builder()
        .uri("/api/v1/validation/manifest")
        .header(header::AUTHORIZATION, "Bearer train-sec-key-789")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let manifest: ValidationManifestDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(manifest.version, "1.0.0");
    assert!(manifest.cases.len() >= 20);
    assert!(manifest.references.len() >= 10);
    assert!(manifest.models.len() >= 8);

    // 5. POST /api/v1/validation/run by Trainee -> 403 Forbidden!
    let run_payload = serde_json::json!({
        "domain_filter": null,
        "case_ids": null
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/validation/run")
        .header(header::AUTHORIZATION, "Bearer train-sec-key-789")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&run_payload).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::FORBIDDEN,
        "Trainee must NOT be permitted to trigger benchmark suite execution"
    );

    // 6. POST /api/v1/validation/run by Operator -> 200 OK!
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/validation/run")
        .header(header::AUTHORIZATION, "Bearer oper-sec-key-456")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&run_payload).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "Operator must be authorized to execute benchmark suite"
    );
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let run_res: ValidationRunResponseDto = serde_json::from_slice(&body).unwrap();
    assert!(run_res.executed_cases >= 20);
    assert_eq!(run_res.failed_cases, 0);
    assert!(run_res.total_duration_ms > 0.0);

    // 7. POST /api/v1/validation/custom by Trainee -> 403 Forbidden!
    let custom_case = serde_json::json!({
        "id": "BENCH-CUSTOM-001",
        "name": "Custom Hydrocyclone Benchmark",
        "domain": "Separation",
        "tested_feature": "Cut size d50",
        "model_name": "model.cyclone.bradley",
        "classification": "SYNTHETIC_TEST",
        "status": "ENGINEERING_CONSISTENT",
        "tolerance_provenance": "PROJECT_DEFINED",
        "citation": {
            "title": "Bradley Hydrocyclone Design",
            "authors_or_organization": "Bradley, D.",
            "publication_year": 1965,
            "doi_or_identifier": null,
            "standard_id": null,
            "url": null
        },
        "applicability_envelope": {
            "min_temperature_k": 280.0,
            "max_temperature_k": 350.0,
            "min_pressure_pa": 100000.0,
            "max_pressure_pa": 1000000.0,
            "supported_phases": ["Liquid", "Solid"],
            "supported_fluids": ["Water-Slurry"],
            "limitation_notes": null
        },
        "relative_tolerance_pct": 5.0,
        "absolute_tolerance": 0.05,
        "metrics": {
            "mean_absolute_error": 0.01,
            "root_mean_squared_error": 0.01,
            "max_absolute_deviation": 0.01,
            "max_relative_error_pct": 1.2,
            "mean_bias_error": 0.005,
            "conservation_residual": 0.0
        },
        "passed": true,
        "last_executed_iso": "2026-09-08T00:00:00Z",
        "execution_duration_ms": 0.1,
        "notes": "Custom benchmark for project cyclone",
        "disclaimer": "NebuDCS internal validation only"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/validation/custom")
        .header(header::AUTHORIZATION, "Bearer train-sec-key-789")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&custom_case).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 8. POST /api/v1/validation/custom by Admin -> 200 OK!
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/validation/custom")
        .header(header::AUTHORIZATION, "Bearer admin-sec-key-123")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&custom_case).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let custom_resp: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(custom_resp["status"], "REGISTERED");
    assert_eq!(custom_resp["case_id"], "BENCH-CUSTOM-001");
}

#[tokio::test]
async fn test_case_t_conservation_invariants_and_baseline_non_regression() {
    let mut config = ServerConfig::default();
    config.admin_token = Some("admin-sec-key-123".to_string());
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle);

    // Verify Case T invariant record exists in manifest
    let req = Request::builder()
        .uri("/api/v1/validation/cases/BENCH-LAW-001")
        .header(header::AUTHORIZATION, "Bearer admin-sec-key-123")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let law_case: ValidationCaseDetailDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(law_case.id, "BENCH-LAW-001");
    assert!(law_case.passed);
    assert!(law_case.metrics.conservation_residual < 1.0e-6);

    // Verify Stage 1–12 baseline non-regression:
    // Health live/ready endpoints respond 200 OK
    let req = Request::builder()
        .uri("/api/v1/health/live")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let req = Request::builder()
        .uri("/api/v1/health/ready")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Hydraulic diagnostics endpoint responds 200 OK
    let req = Request::builder()
        .uri("/api/v1/network/diagnostics")
        .header(header::AUTHORIZATION, "Bearer admin-sec-key-123")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // SIS summary endpoint responds 200 OK
    let req = Request::builder()
        .uri("/api/v1/sis/summary")
        .header(header::AUTHORIZATION, "Bearer admin-sec-key-123")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}
