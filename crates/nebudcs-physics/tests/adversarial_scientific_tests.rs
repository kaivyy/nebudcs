//! Adversarial scientific test suite for NebuDCS Phase 12.
//!
//! Tests pathological edge cases, extreme conditions, singular matrices,
//! bitwise determinism under timing jitter, and real-time execution performance budgets.

use indexmap::IndexMap;
use nebudcs_physics::equipment::{EquipmentCapability, EquipmentExecutionContext};
use nebudcs_physics::error::PhysicsError;
use nebudcs_physics::numerics::{
    Bdf2Solver, DaeSystem, ImplicitEulerSolver, NewtonSolver, OdeSystem, RadauIIASolver,
};
use nebudcs_physics::rotating::{
    CavitationRegime, CompressorMapModel, CompressorOperatingState, PumpCavitationModel,
};
use nebudcs_physics::state::{ComponentComposition, Phase, PhysicalState};
use nebudcs_physics::thermo::{
    ComponentDatabase, CoolPropAdapter, CubicEos, ExternalPropertyProvider, PhaseStabilityState,
    PhaseTarget, VleFlashSolver,
};
use nebudcs_units::{Pressure, Temperature};
use std::time::Instant;

#[test]
fn test_trace_and_extreme_composition_flash() {
    let db = ComponentDatabase::standard_library();
    let pr = CubicEos::new_peng_robinson(db);
    let solver = VleFlashSolver::new(&pr);

    // Multicomponent mixture with trace component (1 ppm = 1e-6)
    let mut z = IndexMap::new();
    z.insert("Hydrogen".to_string(), 0.70);
    z.insert("Methane".to_string(), 0.25);
    z.insert("n-Butane".to_string(), 0.049999);
    z.insert("Water".to_string(), 1.0e-6); // Trace water

    let p = Pressure::new(3.0e6).unwrap(); // 30 bar
    let t = Temperature::new(250.0).unwrap(); // -23.15 °C

    let res = solver.flash_tp(&z, p, t);
    assert!(
        res.is_ok(),
        "Flash with trace water should not fail or produce NaN: {:?}",
        res.err()
    );
    let flash = res.unwrap();

    // Verify non-NaN values
    assert!(!flash.vapor_fraction.is_nan());
    assert!(!flash.mixture_density.value().is_nan());
    for (k, val) in &flash.k_values {
        assert!(!val.is_nan(), "K-value for {k} must not be NaN");
        assert!(*val > 0.0, "K-value for {k} must be positive: {val}");
    }
}

#[test]
fn test_extreme_high_pressure_eos() {
    let db = ComponentDatabase::standard_library();
    let pr = CubicEos::new_peng_robinson(db);

    let mut comp = IndexMap::new();
    comp.insert("Methane".to_string(), 0.90);
    comp.insert("Ethane".to_string(), 0.10);

    // Extreme pressure: 50 MPa (500 bar) at 350 K
    let state = pr.evaluate_phase_state(&comp, 50.0e6, 350.0, PhaseTarget::Liquid);
    assert!(
        state.is_ok(),
        "EOS evaluation at 500 bar should succeed: {:?}",
        state.err()
    );
    let s = state.unwrap();

    assert!(s.compressibility_factor > 0.0);
    assert!(s.molar_density > 0.0);
    assert!(s.mass_density > 100.0); // Dense supercritical fluid
    assert!(s.residual_molar_enthalpy.is_finite());
}

#[test]
fn test_cryogenic_temperature_eos() {
    let db = ComponentDatabase::standard_library();
    let pr = CubicEos::new_peng_robinson(db);

    let mut comp = IndexMap::new();
    comp.insert("Nitrogen".to_string(), 1.0);

    // Cryogenic: 77 K (liquid nitrogen boiling point at 1 atm), 101.3 kPa
    let state = pr.evaluate_phase_state(&comp, 101_325.0, 77.0, PhaseTarget::Liquid);
    assert!(
        state.is_ok(),
        "Cryogenic liquid nitrogen state should evaluate: {:?}",
        state.err()
    );
    let s = state.unwrap();

    assert!(
        s.compressibility_factor < 0.05,
        "Liquid nitrogen Z should be very small: {}",
        s.compressibility_factor
    );
    assert!(
        s.mass_density > 700.0 && s.mass_density < 1000.0,
        "LN2 density: {} kg/m3",
        s.mass_density
    );
}

#[test]
fn test_near_critical_repeated_roots() {
    let db = ComponentDatabase::standard_library();
    let pr = CubicEos::new_peng_robinson(db);

    let mut comp = IndexMap::new();
    comp.insert("CarbonDioxide".to_string(), 1.0);

    // CO2 Critical Point: Tc ≈ 304.13 K, Pc ≈ 7.377 MPa
    // Exactly at critical conditions, the cubic has a triple root Z_c ≈ 0.3074
    let state_v = pr.evaluate_phase_state(&comp, 7.377e6, 304.13, PhaseTarget::Vapor);
    let state_l = pr.evaluate_phase_state(&comp, 7.377e6, 304.13, PhaseTarget::Liquid);

    assert!(state_v.is_ok());
    assert!(state_l.is_ok());

    let z_v = state_v.unwrap().compressibility_factor;
    let z_l = state_l.unwrap().compressibility_factor;

    // Near critical point, vapor and liquid roots should be very close or identical
    assert!(
        (z_v - z_l).abs() < 0.15,
        "Near critical roots should coalesce: Z_v={z_v}, Z_l={z_l}"
    );
}

#[test]
fn test_singular_jacobian_handling_in_newton_solver() {
    let solver = NewtonSolver::new();

    // Initial residual is [0, 0], so it should already be converged!
    let res = solver.solve(&[0.0, 0.0], |x| Ok(vec![x[0] * x[0], x[1] * x[1]]));
    assert!(res.is_ok());

    // Linear equations with no solution (linearly dependent Jacobian).
    let res_singular = solver.solve(&[1.0, 1.0], |x| {
        Ok(vec![x[0] + x[1] - 1.0, x[0] + x[1] - 2.0])
    });

    assert!(res_singular.is_err());
    match res_singular.err().unwrap() {
        PhysicsError::SingularJacobian { detail } => {
            assert!(
                detail.contains("row") || detail.contains("singular") || detail.contains("pivot")
            );
        }
        err => panic!("Expected SingularJacobian error, got: {:?}", err),
    }
}

#[test]
fn test_compressor_deep_surge_and_overspeed() {
    let mut comp = CompressorMapModel::new_standard(12_000.0, 15.0);

    // Test zero flow
    comp.evaluate_performance(0.0, 1.0);
    assert_eq!(comp.last_operating_state, CompressorOperatingState::Surge);
    assert!(comp.last_surge_margin < -0.5);

    // Test extreme choke flow (mass flow 25 kg/s > max 20 kg/s)
    comp.evaluate_performance(25.0, 1.0);
    assert_eq!(comp.last_operating_state, CompressorOperatingState::Choke);

    // Test overspeed limit clamping (speed ratio 1.5 clamped to 1.15)
    comp.evaluate_performance(15.0, 1.5);
    assert!(comp.last_pressure_ratio > 1.0);
}

#[test]
fn test_pump_severe_flashing_cavitation() {
    let mut pump = PumpCavitationModel::new(0.05, 3000.0, 3.2, 1000.0).unwrap();

    // Suction pressure lower than vapor pressure (P < P_sat => fluid boiling in suction!)
    pump.suction_pressure = Pressure::new(1_000.0).unwrap(); // 1 kPa < 3.169 kPa P_sat
    pump.evaluate_cavitation(50.0, 1.0);

    assert_eq!(pump.last_regime, CavitationRegime::Cavitation);
    assert!(pump.last_npsha_m >= 0.0);
    assert!(pump.last_cavitation_margin_m < -3.0);
    assert!(
        pump.last_head_degradation < 0.20,
        "Severe head degradation < 0.20: {}",
        pump.last_head_degradation
    );
    assert!(
        pump.last_vibration_mm_s >= 8.0,
        "Severe vibration > 8 mm/s: {}",
        pump.last_vibration_mm_s
    );
}

#[test]
fn test_wall_clock_determinism_under_jitter() {
    let db = ComponentDatabase::standard_library();
    let pr = CubicEos::new_peng_robinson(db);
    let solver = VleFlashSolver::new(&pr);

    let mut z = IndexMap::new();
    z.insert("Methane".to_string(), 0.60);
    z.insert("Ethane".to_string(), 0.20);
    z.insert("Propane".to_string(), 0.20);

    let p = Pressure::new(2.5e6).unwrap();
    let t = Temperature::new(230.0).unwrap();

    let mut results = Vec::new();

    for i in 0..10 {
        // Introduce small artificial thread jitter
        if i % 2 == 0 {
            std::thread::sleep(std::time::Duration::from_micros(50));
        } else {
            std::thread::yield_now();
        }

        let flash = solver.flash_tp(&z, p, t).unwrap();
        results.push(flash);
    }

    // Assert exact bitwise determinism across all 10 runs
    let ref_flash = &results[0];
    for (idx, r) in results.iter().enumerate().skip(1) {
        assert_eq!(
            ref_flash.vapor_fraction.to_bits(),
            r.vapor_fraction.to_bits(),
            "Run {idx} vapor fraction differs from run 0"
        );
        assert_eq!(
            ref_flash.mixture_density.value().to_bits(),
            r.mixture_density.value().to_bits(),
            "Run {idx} mixture density differs from run 0"
        );
        for comp_name in z.keys() {
            assert_eq!(
                ref_flash.k_values[comp_name].to_bits(),
                r.k_values[comp_name].to_bits(),
                "Run {idx} K-value for {comp_name} differs from run 0"
            );
        }
    }
}

#[test]
fn test_realtime_performance_budgets() {
    let db = ComponentDatabase::standard_library();
    let pr = CubicEos::new_peng_robinson(db);
    let solver = VleFlashSolver::new(&pr);

    let mut z = IndexMap::new();
    z.insert("Methane".to_string(), 0.70);
    z.insert("Ethane".to_string(), 0.20);
    z.insert("Propane".to_string(), 0.10);

    let p = Pressure::new(3.0e6).unwrap();
    let t = Temperature::new(240.0).unwrap();

    // Warm-up
    let _ = solver.flash_tp(&z, p, t).unwrap();

    let iterations = 1000;
    let start = Instant::now();

    for _ in 0..iterations {
        let res = solver.flash_tp(&z, p, t).unwrap();
        std::hint::black_box(res);
    }

    let elapsed = start.elapsed();
    let avg_us = (elapsed.as_secs_f64() * 1e6) / (iterations as f64);

    let budget_us = if cfg!(debug_assertions) {
        1000.0
    } else {
        250.0
    };

    println!(
        "[PERFORMANCE] Multicomponent TP-Flash average execution time: {:.2} µs (budget: < {:.1} µs)",
        avg_us, budget_us
    );

    assert!(
        avg_us < budget_us,
        "Flash calculation took {:.2} µs, exceeding {:.1} µs real-time simulation budget",
        avg_us,
        budget_us
    );
}

#[test]
fn test_f01_coolprop_adapter_model_identity_and_limitations() {
    let cp = CoolPropAdapter::default();
    assert_eq!(cp.provider_name(), "CoolProp_Native_Simplified_Adapter");
    assert_eq!(cp.backend(), "NativeSimplified");
    assert!(
        !cp.is_full_iapws_if97(),
        "Must explicitly report false for full 5-region IAPWS-IF97"
    );

    // Check subcooled liquid water density at 298.15 K (25 °C), 1 atm
    let rho = cp.pure_fluid_density("Water", 101_325.0, 298.15).unwrap();
    assert!(
        (rho - 997.0).abs() < 10.0,
        "Liquid water density at 25 C: {rho} kg/m3"
    );
}

#[test]
fn test_f02_stiff_ode_solvers_and_dae_contract() {
    struct StiffDecayOde {
        lambda: f64,
    }
    impl OdeSystem for StiffDecayOde {
        fn dimension(&self) -> usize {
            1
        }
        fn evaluate_rhs(&self, _t: f64, x: &[f64]) -> Result<Vec<f64>, PhysicsError> {
            Ok(vec![-self.lambda * x[0]])
        }
    }

    let ode = StiffDecayOde { lambda: 1000.0 };
    let x0 = vec![1.0];
    let dt = 0.001;

    let radau = RadauIIASolver::new();
    let x1_radau = radau.step(&ode, 0.0, &x0, dt).unwrap();
    assert!(
        x1_radau[0] > 0.0 && x1_radau[0] < 0.5,
        "Radau IIA stiff decay: {}",
        x1_radau[0]
    );

    let euler = ImplicitEulerSolver::new();
    let x1_euler = euler.step(&ode, 0.0, &x0, dt).unwrap();
    assert!(
        x1_euler[0] > 0.0 && x1_euler[0] < 0.6,
        "Implicit Euler stiff decay: {}",
        x1_euler[0]
    );

    let bdf2 = Bdf2Solver::new();
    let x1_bdf = bdf2.step(&ode, 0.0, &x0, None, dt).unwrap();
    assert!(
        x1_bdf[0] > 0.0 && x1_bdf[0] < 0.6,
        "BDF-2 stiff decay: {}",
        x1_bdf[0]
    );

    // DAE contract test: verify DaeSystem is an interface contract for problem specification
    struct DummyDae;
    impl DaeSystem for DummyDae {
        fn differential_dimension(&self) -> usize {
            1
        }
        fn algebraic_dimension(&self) -> usize {
            1
        }
        fn evaluate_residual(
            &self,
            _t: f64,
            x: &[f64],
            x_dot: &[f64],
            z: &[f64],
        ) -> Result<Vec<f64>, PhysicsError> {
            Ok(vec![x_dot[0] + x[0] - z[0], z[0] - 2.0 * x[0]])
        }
    }
    let dae = DummyDae;
    assert_eq!(dae.differential_dimension(), 1);
    assert_eq!(dae.algebraic_dimension(), 1);
}

#[test]
fn test_f03_michelsen_tpd_phase_stability_analysis() {
    let db = ComponentDatabase::standard_library();
    let pr = CubicEos::new_peng_robinson(db);
    let solver = VleFlashSolver::new(&pr);

    // 1. Subcooled liquid methane (110 K, 1 MPa): StableLiquid
    let mut z_liq = IndexMap::new();
    z_liq.insert("Methane".to_string(), 1.0);
    let p_liq = Pressure::new(1.0e6).unwrap();
    let t_liq = Temperature::new(110.0).unwrap();

    let stab_liq = solver.check_phase_stability(&z_liq, p_liq, t_liq).unwrap();
    assert!(
        stab_liq.is_stable,
        "Liquid methane at 110 K, 1 MPa must be stable"
    );
    assert_eq!(stab_liq.state, PhaseStabilityState::StableLiquid);
    let flash_liq = solver.flash_tp(&z_liq, p_liq, t_liq).unwrap();
    assert_eq!(flash_liq.phase, Phase::Liquid);
    assert_eq!(flash_liq.stability, PhaseStabilityState::StableLiquid);
    assert_eq!(flash_liq.vapor_fraction, 0.0);

    // 2. Superheated vapor methane (300 K, 100 kPa): StableVapor
    let p_vap = Pressure::new(100_000.0).unwrap();
    let t_vap = Temperature::new(300.0).unwrap();

    let stab_vap = solver.check_phase_stability(&z_liq, p_vap, t_vap).unwrap();
    assert!(
        stab_vap.is_stable,
        "Vapor methane at 300 K, 100 kPa must be stable"
    );
    assert_eq!(stab_vap.state, PhaseStabilityState::StableVapor);
    let flash_vap = solver.flash_tp(&z_liq, p_vap, t_vap).unwrap();
    assert_eq!(flash_vap.phase, Phase::Vapor);
    assert_eq!(flash_vap.stability, PhaseStabilityState::StableVapor);
    assert_eq!(flash_vap.vapor_fraction, 1.0);

    // 3. Two-phase hydrocarbon blend (Methane + Ethane + Propane at 230 K, 2.5 MPa)
    let mut z_mix = IndexMap::new();
    z_mix.insert("Methane".to_string(), 0.50);
    z_mix.insert("Ethane".to_string(), 0.30);
    z_mix.insert("Propane".to_string(), 0.20);
    let p_mix = Pressure::new(2.5e6).unwrap();
    let t_mix = Temperature::new(230.0).unwrap();

    let stab_mix = solver.check_phase_stability(&z_mix, p_mix, t_mix).unwrap();
    assert!(
        !stab_mix.is_stable || stab_mix.state == PhaseStabilityState::UnstablePhaseSplit,
        "Blend must be unstable or phase split: {:?}",
        stab_mix.state
    );
    let flash_mix = solver.flash_tp(&z_mix, p_mix, t_mix).unwrap();
    assert_eq!(flash_mix.phase, Phase::TwoPhase);
    assert_eq!(flash_mix.stability, PhaseStabilityState::StableTwoPhase);
    assert!(
        flash_mix.vapor_fraction > 0.0 && flash_mix.vapor_fraction < 1.0,
        "Vapor fraction must be two-phase: {}",
        flash_mix.vapor_fraction
    );
}

#[test]
fn test_f05_compressor_rigorous_thermodynamics_and_missing_context() {
    let mut comp = CompressorMapModel::new_standard(10_000.0, 10.0);
    let flow_10 = vec![nebudcs_units::MassFlow::new(10.0).unwrap()];

    // 1. Missing inlet states context when flow > 0 must return MissingProperty error
    let ctx_missing = EquipmentExecutionContext::new(0.01, &flow_10, &flow_10);
    let res_missing = comp.evaluate(&ctx_missing);
    assert!(
        res_missing.is_err(),
        "Compressor must return error if inlet stream state is missing"
    );
    match res_missing.err().unwrap() {
        PhysicsError::MissingProperty { name } => {
            assert!(
                name.contains("inlet_stream_state"),
                "Error should specify inlet_stream_state: {name}"
            );
        }
        other => panic!("Unexpected error type: {other:?}"),
    }

    // 2. Pure Hydrogen inlet stream state (Mw = 0.002016 kg/mol)
    let p_in = Pressure::new(100_000.0).unwrap();
    let t_in_300 = Temperature::new(300.0).unwrap();
    let comp_h2 = ComponentComposition::pure("Hydrogen");
    let state_h2 = PhysicalState::new(
        p_in,
        t_in_300,
        nebudcs_units::Density::new(0.08).unwrap(),
        nebudcs_units::SpecificEnthalpy::new(0.0).unwrap(),
        0.002016, // Mw in kg/mol
        1.0,
        Phase::Vapor,
        comp_h2,
    )
    .unwrap();

    let in_states_h2 = vec![state_h2];
    let ctx_h2 = EquipmentExecutionContext::new(0.01, &flow_10, &flow_10)
        .with_thermodynamics(&in_states_h2, None);

    comp.evaluate(&ctx_h2).unwrap();
    let power_h2 = comp.get_variable("power").unwrap();
    assert!(
        power_h2 > 5_000_000.0,
        "Hydrogen compressor power: {power_h2} W"
    );

    // 3. Pure Nitrogen inlet stream state (Mw = 0.028013 kg/mol)
    let comp_n2 = ComponentComposition::pure("Nitrogen");
    let state_n2 = PhysicalState::new(
        p_in,
        t_in_300,
        nebudcs_units::Density::new(1.15).unwrap(),
        nebudcs_units::SpecificEnthalpy::new(0.0).unwrap(),
        0.028013, // Mw in kg/mol
        1.0,
        Phase::Vapor,
        comp_n2.clone(),
    )
    .unwrap();

    let in_states_n2 = vec![state_n2];
    let ctx_n2 = EquipmentExecutionContext::new(0.01, &flow_10, &flow_10)
        .with_thermodynamics(&in_states_n2, None);

    comp.evaluate(&ctx_n2).unwrap();
    let power_n2 = comp.get_variable("power").unwrap();
    assert!(
        power_n2 > 500_000.0 && power_n2 < 2_500_000.0,
        "Nitrogen compressor power: {power_n2} W"
    );

    // Theoretical check: W_s scales inversely with Mw!
    let power_ratio = power_h2 / power_n2;
    assert!(
        power_ratio > 8.0 && power_ratio < 20.0,
        "H2/N2 power ratio must reflect molecular weight difference: {power_ratio}"
    );

    // 4. Higher temperature: power must increase proportionally with T_in
    let t_in_360 = Temperature::new(360.0).unwrap();
    let state_n2_hot = PhysicalState::new(
        p_in,
        t_in_360,
        nebudcs_units::Density::new(0.95).unwrap(),
        nebudcs_units::SpecificEnthalpy::new(0.0).unwrap(),
        0.028013,
        1.0,
        Phase::Vapor,
        comp_n2,
    )
    .unwrap();

    let in_states_n2_hot = vec![state_n2_hot];
    let ctx_n2_hot = EquipmentExecutionContext::new(0.01, &flow_10, &flow_10)
        .with_thermodynamics(&in_states_n2_hot, None);

    comp.evaluate(&ctx_n2_hot).unwrap();
    let power_n2_hot = comp.get_variable("power").unwrap();
    assert!(
        power_n2_hot > power_n2,
        "Hot gas compression requires more shaft work: {power_n2_hot} vs {power_n2}"
    );
    let temp_ratio = 360.0 / 300.0;
    let actual_power_ratio = power_n2_hot / power_n2;
    assert!(
        (actual_power_ratio - temp_ratio).abs() < 0.1,
        "Power ratio {actual_power_ratio} must closely match temperature ratio {temp_ratio}"
    );
}
