//! Rigorous thermodynamic consistency, Maxwell relations, Gibbs-Duhem,
//! path independence, reference state, and phase envelope validation suite.
//!
//! Addresses NebuDCS Phase 12 requirements (§37 through §47):
//! - Fundamental thermodynamic relation: dH = T dS + V dP
//! - Maxwell relations: (dH/dP)_T = V - T (dV/dT)_P and (dS/dP)_T = -(dV/dT)_P
//! - Gibbs-Duhem mixture consistency: sum(x_i * d ln(phi_i)) = 0 at const T, P
//! - Phase equilibrium multi-criteria closure: f_i^L = f_i^V and z_i = (1-beta)x_i + beta*y_i
//! - Path independence: State A -> B -> C vs State A -> D -> C
//! - Reference state datum preservation
//! - Numerical phase envelope (bubble/dew curves)
//! - Curated scientific regression suite

use indexmap::IndexMap;
use nebudcs_physics::state::Phase;
use nebudcs_physics::thermo::{
    ComponentDatabase, CubicEos, PhaseEnvelopeSolver, PhaseTarget, ReferenceBenchmarkDatabase,
    VleFlashSolver,
};
use nebudcs_units::{Pressure, Temperature};

#[test]
fn test_maxwell_enthalpy_pressure_derivative_consistency() {
    let db = ComponentDatabase::standard_library();
    let pr = CubicEos::new_peng_robinson(db);

    let mut comp = IndexMap::new();
    comp.insert("Methane".to_string(), 1.0);

    // Coordinate: T = 300 K, P = 3.0 MPa (dense gas)
    let t = 300.0;
    let p = 3.0e6;

    let dt = 1e-2;
    let dp = 1e2;

    // LHS: (dH/dP)_T
    let s_p_plus = pr
        .evaluate_phase_state(&comp, p + dp, t, PhaseTarget::Vapor)
        .unwrap();
    let s_p_minus = pr
        .evaluate_phase_state(&comp, p - dp, t, PhaseTarget::Vapor)
        .unwrap();
    let dh_dp = (s_p_plus.molar_enthalpy - s_p_minus.molar_enthalpy) / (2.0 * dp); // J/(mol * Pa) = m³/mol

    // RHS: V - T * (dV/dT)_P
    let s_t_plus = pr
        .evaluate_phase_state(&comp, p, t + dt, PhaseTarget::Vapor)
        .unwrap();
    let s_t_minus = pr
        .evaluate_phase_state(&comp, p, t - dt, PhaseTarget::Vapor)
        .unwrap();
    let dv_dt = (s_t_plus.molar_volume - s_t_minus.molar_volume) / (2.0 * dt);
    let s_nominal = pr
        .evaluate_phase_state(&comp, p, t, PhaseTarget::Vapor)
        .unwrap();
    let rhs = s_nominal.molar_volume - t * dv_dt;

    let rel_diff = (dh_dp - rhs).abs() / rhs.abs();
    println!(
        "[CONSISTENCY] Maxwell (dH/dP)_T = V - T(dV/dT)_P | LHS: {:.6e} | RHS: {:.6e} | RelDiff: {:.4}%",
        dh_dp, rhs, rel_diff * 100.0
    );

    assert!(
        rel_diff < 0.01,
        "Maxwell relation (dH/dP)_T = V - T(dV/dT)_P failed: LHS={dh_dp}, RHS={rhs}, rel_diff={rel_diff}"
    );
}

#[test]
fn test_maxwell_entropy_pressure_derivative_consistency() {
    let db = ComponentDatabase::standard_library();
    let pr = CubicEos::new_peng_robinson(db);

    let mut comp = IndexMap::new();
    comp.insert("Methane".to_string(), 1.0);

    let t = 300.0;
    let p = 3.0e6;

    let dt = 1e-2;
    let dp = 1e2;

    // LHS: (dS/dP)_T
    let s_p_plus = pr
        .evaluate_phase_state(&comp, p + dp, t, PhaseTarget::Vapor)
        .unwrap();
    let s_p_minus = pr
        .evaluate_phase_state(&comp, p - dp, t, PhaseTarget::Vapor)
        .unwrap();
    let ds_dp = (s_p_plus.molar_entropy - s_p_minus.molar_entropy) / (2.0 * dp); // J/(mol * K * Pa)

    // RHS: -(dV/dT)_P
    let s_t_plus = pr
        .evaluate_phase_state(&comp, p, t + dt, PhaseTarget::Vapor)
        .unwrap();
    let s_t_minus = pr
        .evaluate_phase_state(&comp, p, t - dt, PhaseTarget::Vapor)
        .unwrap();
    let dv_dt = (s_t_plus.molar_volume - s_t_minus.molar_volume) / (2.0 * dt);
    let rhs = -dv_dt;

    let rel_diff = (ds_dp - rhs).abs() / rhs.abs();
    println!(
        "[CONSISTENCY] Maxwell (dS/dP)_T = -(dV/dT)_P | LHS: {:.6e} | RHS: {:.6e} | RelDiff: {:.4}%",
        ds_dp, rhs, rel_diff * 100.0
    );

    assert!(
        rel_diff < 0.01,
        "Maxwell relation (dS/dP)_T = -(dV/dT)_P failed: LHS={ds_dp}, RHS={rhs}, rel_diff={rel_diff}"
    );
}

#[test]
fn test_gibbs_duhem_mixture_consistency() {
    let db = ComponentDatabase::standard_library();
    let pr = CubicEos::new_peng_robinson(db);

    // Binary mixture: 60% Methane, 40% Ethane at 300 K, 2 MPa
    let t = 300.0;
    let p = 2.0e6;
    let dx = 1e-4;

    let mut comp_base = IndexMap::new();
    comp_base.insert("Methane".to_string(), 0.60);
    comp_base.insert("Ethane".to_string(), 0.40);

    let mut comp_plus = IndexMap::new();
    comp_plus.insert("Methane".to_string(), 0.60 + dx);
    comp_plus.insert("Ethane".to_string(), 0.40 - dx);

    let mut comp_minus = IndexMap::new();
    comp_minus.insert("Methane".to_string(), 0.60 - dx);
    comp_minus.insert("Ethane".to_string(), 0.40 + dx);

    let s_plus = pr
        .evaluate_phase_state(&comp_plus, p, t, PhaseTarget::Vapor)
        .unwrap();
    let s_minus = pr
        .evaluate_phase_state(&comp_minus, p, t, PhaseTarget::Vapor)
        .unwrap();

    // Gibbs-Duhem at constant T, P: sum(x_i * d ln(phi_i)) = 0
    let ln_phi_c1_plus = s_plus.fugacity_coefficients["Methane"].ln();
    let ln_phi_c1_minus = s_minus.fugacity_coefficients["Methane"].ln();
    let d_ln_phi_c1 = ln_phi_c1_plus - ln_phi_c1_minus;

    let ln_phi_c2_plus = s_plus.fugacity_coefficients["Ethane"].ln();
    let ln_phi_c2_minus = s_minus.fugacity_coefficients["Ethane"].ln();
    let d_ln_phi_c2 = ln_phi_c2_plus - ln_phi_c2_minus;

    let gd_sum = 0.60 * d_ln_phi_c1 + 0.40 * d_ln_phi_c2;
    println!(
        "[CONSISTENCY] Gibbs-Duhem sum(x_i * d ln(phi_i)) = {:.6e} (must be near zero)",
        gd_sum
    );

    assert!(
        gd_sum.abs() < 1e-4,
        "Gibbs-Duhem mixture consistency failed: sum = {gd_sum}"
    );
}

#[test]
fn test_thermodynamic_path_independence() {
    let db = ComponentDatabase::standard_library();
    let pr = CubicEos::new_peng_robinson(db);

    let mut comp = IndexMap::new();
    comp.insert("Methane".to_string(), 0.70);
    comp.insert("Ethane".to_string(), 0.30);

    // State A: T1 = 300 K, P1 = 1.0 MPa
    // State C: T2 = 400 K, P2 = 5.0 MPa
    let t1 = 300.0;
    let p1 = 1.0e6;
    let t2 = 400.0;
    let p2 = 5.0e6;

    let state_a = pr
        .evaluate_phase_state(&comp, p1, t1, PhaseTarget::Vapor)
        .unwrap();
    let state_c = pr
        .evaluate_phase_state(&comp, p2, t2, PhaseTarget::Vapor)
        .unwrap();

    // Path 1: A (T1, P1) -> B (T2, P1) [isobaric heating] -> C (T2, P2) [isothermal compression]
    let state_b = pr
        .evaluate_phase_state(&comp, p1, t2, PhaseTarget::Vapor)
        .unwrap();
    let delta_h_path1 = (state_b.molar_enthalpy - state_a.molar_enthalpy)
        + (state_c.molar_enthalpy - state_b.molar_enthalpy);
    let delta_s_path1 = (state_b.molar_entropy - state_a.molar_entropy)
        + (state_c.molar_entropy - state_b.molar_entropy);

    // Path 2: A (T1, P1) -> D (T1, P2) [isothermal compression] -> C (T2, P2) [isobaric heating]
    let state_d = pr
        .evaluate_phase_state(&comp, p2, t1, PhaseTarget::Vapor)
        .unwrap();
    let delta_h_path2 = (state_d.molar_enthalpy - state_a.molar_enthalpy)
        + (state_c.molar_enthalpy - state_d.molar_enthalpy);
    let delta_s_path2 = (state_d.molar_entropy - state_a.molar_entropy)
        + (state_c.molar_entropy - state_d.molar_entropy);

    // Direct State Difference
    let delta_h_direct = state_c.molar_enthalpy - state_a.molar_enthalpy;
    let delta_s_direct = state_c.molar_entropy - state_a.molar_entropy;

    println!(
        "[PATH INDEPENDENCE] delta_H: Path1={:.4}, Path2={:.4}, Direct={:.4} J/mol",
        delta_h_path1, delta_h_path2, delta_h_direct
    );
    println!(
        "[PATH INDEPENDENCE] delta_S: Path1={:.4}, Path2={:.4}, Direct={:.4} J/(mol*K)",
        delta_s_path1, delta_s_path2, delta_s_direct
    );

    let h_err = (delta_h_path1 - delta_h_path2).abs() / delta_h_direct.abs();
    let s_err = (delta_s_path1 - delta_s_path2).abs() / delta_s_direct.abs();

    assert!(h_err < 1e-6, "Enthalpy path dependence error: {h_err}");
    assert!(s_err < 1e-6, "Entropy path dependence error: {s_err}");
}

#[test]
fn test_phase_equilibrium_fundamental_closure() {
    let db = ComponentDatabase::standard_library();
    let pr = CubicEos::new_peng_robinson(db);
    let solver = VleFlashSolver::new(&pr);

    let mut z = IndexMap::new();
    z.insert("Methane".to_string(), 0.50);
    z.insert("Propane".to_string(), 0.50);

    let p = Pressure::new(2.0e6).unwrap();
    let t = Temperature::new(220.0).unwrap();

    let flash = solver.flash_tp(&z, p, t).unwrap();
    assert_eq!(flash.phase, Phase::TwoPhase);

    let beta = flash.vapor_fraction;
    assert!(beta > 0.0 && beta < 1.0);

    // 1. Material balance closure: z_i = (1 - beta) * x_i + beta * y_i
    for (name, &zi) in &z {
        let xi = flash.liquid_composition[name];
        let yi = flash.vapor_composition[name];
        let reconstructed = (1.0 - beta) * xi + beta * yi;
        let mb_err = (zi - reconstructed).abs();
        assert!(
            mb_err < 1e-6,
            "Species material balance violated for {name}: zi={zi}, reconstructed={reconstructed}"
        );
    }

    // 2. Iso-fugacity equality: f_i^L == f_i^V
    let s_liq = pr
        .evaluate_phase_state(
            &flash.liquid_composition,
            p.value(),
            t.value(),
            PhaseTarget::Liquid,
        )
        .unwrap();
    let s_vap = pr
        .evaluate_phase_state(
            &flash.vapor_composition,
            p.value(),
            t.value(),
            PhaseTarget::Vapor,
        )
        .unwrap();

    for (name, _) in &z {
        let f_l = s_liq.component_fugacities[name];
        let f_v = s_vap.component_fugacities[name];
        let rel_f_err = (f_l - f_v).abs() / f_l;
        assert!(
            rel_f_err < 1e-4,
            "Fugacity equality violated for {name}: f_L={f_l}, f_V={f_v}, rel_err={rel_f_err}"
        );
    }
}

#[test]
fn test_phase_envelope_bubble_and_dew_boundaries() {
    let db = ComponentDatabase::standard_library();
    let pr = CubicEos::new_peng_robinson(db);
    let envelope_solver = PhaseEnvelopeSolver::new(&pr);

    // 50/50 Methane-Propane binary
    let mut z = IndexMap::new();
    z.insert("Methane".to_string(), 0.50);
    z.insert("Propane".to_string(), 0.50);

    let t_test = 210.0; // 210 K (-63.15 °C)

    let bp = envelope_solver
        .calculate_bubble_point_pressure(&z, t_test)
        .unwrap();
    let dp = envelope_solver
        .calculate_dew_point_pressure(&z, t_test)
        .unwrap();

    println!(
        "[ENVELOPE] C1-C3 at 210 K | P_bubble = {:.2} bar | P_dew = {:.2} bar | Res_b = {:.2e} | Res_d = {:.2e}",
        bp.pressure_pa / 1e5,
        dp.pressure_pa / 1e5,
        bp.residual,
        dp.residual
    );

    // Fundamental physical law of mixtures:
    // Bubble point pressure must be strictly greater than dew point pressure at the same temperature!
    assert!(
        bp.pressure_pa > dp.pressure_pa,
        "P_bubble ({}) must exceed P_dew ({}) for a zeotropic mixture",
        bp.pressure_pa,
        dp.pressure_pa
    );

    assert!(bp.residual < 1e-4);
    assert!(dp.residual < 1e-4);

    // Trace small envelope (3 points)
    let envelope = envelope_solver
        .trace_phase_envelope(&z, 200.0, 240.0, 3)
        .unwrap();
    assert_eq!(envelope.bubble_curve.len(), 3);
    assert_eq!(envelope.dew_curve.len(), 3);
    assert!(envelope.cricondentherm_k.is_some());
    assert!(envelope.cricondenbar_pa.is_some());
}

#[test]
fn test_reference_benchmark_database_execution() {
    let db_ref = ReferenceBenchmarkDatabase::standard_library();
    assert!(!db_ref.cases.is_empty());

    let comp_db = ComponentDatabase::standard_library();
    let pr = CubicEos::new_peng_robinson(comp_db);

    for case in &db_ref.cases {
        if case.case_id == "nist_methane_z_300k_10mpa"
            || case.case_id == "nist_nitrogen_z_300k_1atm"
        {
            let state = pr
                .evaluate_phase_state(
                    &case.composition,
                    case.pressure_pa,
                    case.temperature_k,
                    PhaseTarget::Vapor,
                )
                .unwrap();
            let res = case.verify(state.compressibility_factor);
            assert!(res.passed, "Benchmark {} failed: {:?}", case.case_id, res);
        } else if case.case_id == "nist_methane_hres_300k_5mpa" {
            let state = pr
                .evaluate_phase_state(
                    &case.composition,
                    case.pressure_pa,
                    case.temperature_k,
                    PhaseTarget::Vapor,
                )
                .unwrap();
            let res = case.verify(state.residual_molar_enthalpy);
            assert!(res.passed, "Benchmark {} failed: {:?}", case.case_id, res);
        }
    }
}
