use indexmap::IndexMap;
use nebudcs_physics::error::PhysicsError;
use nebudcs_physics::state::{ComponentComposition, Phase, PhysicalState, StreamPhysicalState};
use nebudcs_types::StreamId;
use nebudcs_units::{Density, MassFlow, Pressure, SpecificEnthalpy, Temperature};

#[test]
fn test_physical_state_validation_valid() {
    let mut comp_map = IndexMap::new();
    comp_map.insert("H2O".to_string(), 1.0);
    let comp = ComponentComposition::new(comp_map).expect("valid composition");

    let state = PhysicalState::new(
        Pressure::new(101_325.0).unwrap(),
        Temperature::new(300.0).unwrap(),
        Density::new(997.0).unwrap(),
        SpecificEnthalpy::new(113_000.0).unwrap(),
        0.018015, // 18 g/mol in kg/mol
        0.0,
        Phase::Liquid,
        comp,
    );

    assert!(state.is_ok());
    let s = state.unwrap();
    assert_eq!(s.pressure().value(), 101_325.0);
    assert_eq!(s.temperature().value(), 300.0);
    assert_eq!(s.phase(), Phase::Liquid);
}

#[test]
fn test_physical_state_rejects_negative_pressure() {
    let mut comp_map = IndexMap::new();
    comp_map.insert("H2O".to_string(), 1.0);
    let comp = ComponentComposition::new(comp_map).unwrap();

    let err = PhysicalState::new(
        Pressure::new_unchecked(-50.0),
        Temperature::new(300.0).unwrap(),
        Density::new(997.0).unwrap(),
        SpecificEnthalpy::new(100.0).unwrap(),
        0.018,
        0.0,
        Phase::Liquid,
        comp,
    )
    .unwrap_err();

    assert!(matches!(err, PhysicsError::NegativePressure { .. }));
}

#[test]
fn test_physical_state_rejects_non_positive_temperature() {
    let mut comp_map = IndexMap::new();
    comp_map.insert("H2O".to_string(), 1.0);
    let comp = ComponentComposition::new(comp_map).unwrap();

    let err = PhysicalState::new(
        Pressure::new(101_325.0).unwrap(),
        Temperature::new_unchecked(0.0),
        Density::new(997.0).unwrap(),
        SpecificEnthalpy::new(100.0).unwrap(),
        0.018,
        0.0,
        Phase::Liquid,
        comp,
    )
    .unwrap_err();

    assert!(matches!(err, PhysicsError::NonPositiveTemperature { .. }));
}

#[test]
fn test_physical_state_rejects_invalid_vapor_fraction() {
    let mut comp_map = IndexMap::new();
    comp_map.insert("H2O".to_string(), 1.0);
    let comp = ComponentComposition::new(comp_map).unwrap();

    let err = PhysicalState::new(
        Pressure::new(101_325.0).unwrap(),
        Temperature::new(300.0).unwrap(),
        Density::new(997.0).unwrap(),
        SpecificEnthalpy::new(100.0).unwrap(),
        0.018,
        1.5, // > 1.0 is invalid
        Phase::TwoPhase,
        comp,
    )
    .unwrap_err();

    assert!(matches!(err, PhysicsError::InvalidVaporFraction { .. }));
}

#[test]
fn test_composition_normalization_and_fractions() {
    let mut comp_map = IndexMap::new();
    comp_map.insert("H2".to_string(), 2.0);
    comp_map.insert("CH4".to_string(), 6.0);

    let comp = ComponentComposition::new(comp_map).expect("valid composition");
    assert!((comp.fraction("H2") - 0.25).abs() < 1e-10);
    assert!((comp.fraction("CH4") - 0.75).abs() < 1e-10);
    assert_eq!(comp.fraction("NON_EXISTENT"), 0.0);
}

#[test]
fn test_stream_physical_state_directional_flow() {
    let comp = ComponentComposition::pure("H2O");
    let state = PhysicalState::new(
        Pressure::new(101_325.0).unwrap(),
        Temperature::new(300.0).unwrap(),
        Density::new(997.0).unwrap(),
        SpecificEnthalpy::new(100.0).unwrap(),
        0.018,
        0.0,
        Phase::Liquid,
        comp,
    )
    .unwrap();

    let stream_id = StreamId::new();
    let stream_fwd =
        StreamPhysicalState::new(stream_id, MassFlow::new(12.5).unwrap(), state.clone());
    assert!(stream_fwd.is_forward());
    assert!(!stream_fwd.is_reverse());
    assert!(!stream_fwd.is_zero());
    assert_eq!(stream_fwd.mass_flow_rate().value(), 12.5);

    let stream_rev = StreamPhysicalState::new(stream_id, MassFlow::new(-4.2).unwrap(), state);
    assert!(!stream_rev.is_forward());
    assert!(stream_rev.is_reverse());
    assert_eq!(stream_rev.mass_flow_rate().value(), -4.2);
}

#[test]
fn test_ideal_gas_density_calculation() {
    use nebudcs_physics::properties::{IdealGasMixture, PropertyPackage};

    let mut gas_pkg = IdealGasMixture::new("IdealGasRefinery");
    gas_pkg.register_component("N2", 0.0280134, 1040.0); // 28 g/mol, Cp = 1040 J/(kg*K)

    let comp = ComponentComposition::pure("N2");
    let p = Pressure::new(101_325.0).unwrap();
    let t = Temperature::new(298.15).unwrap();

    let state = gas_pkg
        .evaluate_state(p, t, &comp)
        .expect("state evaluation ok");
    assert_eq!(state.phase(), Phase::Vapor);
    assert_eq!(state.vapor_fraction(), 1.0);

    // Expected rho = P * Mw / (R * T) = 101325 * 0.0280134 / (8.314462618 * 298.15) ≈ 1.1450 kg/m3
    let rho = state.density().value();
    assert!((rho - 1.1450).abs() < 1e-3, "calculated rho: {rho}");
}

#[test]
fn test_ideal_gas_mixture_molecular_weight() {
    use nebudcs_physics::properties::{IdealGasMixture, PropertyPackage};

    let mut gas_pkg = IdealGasMixture::new("H2_CH4_Mix");
    gas_pkg.register_component("H2", 0.002016, 14300.0);
    gas_pkg.register_component("CH4", 0.016043, 2220.0);

    let mut comp_map = IndexMap::new();
    comp_map.insert("H2".to_string(), 0.5);
    comp_map.insert("CH4".to_string(), 0.5);
    let comp = ComponentComposition::new(comp_map).unwrap();

    let mw = gas_pkg.molecular_weight(&comp).expect("mw calculation ok");
    let expected_mw = 0.5 * 0.002016 + 0.5 * 0.016043;
    assert!((mw - expected_mw).abs() < 1e-6);
}

#[test]
fn test_incompressible_liquid_properties() {
    use nebudcs_physics::properties::{IncompressibleLiquid, PropertyPackage};

    let water_pkg = IncompressibleLiquid::new("LiquidWater", 998.0, 4184.0, 0.018015);
    let comp = ComponentComposition::pure("H2O");
    let p = Pressure::new(200_000.0).unwrap();
    let t = Temperature::new(323.15).unwrap(); // 50 C

    let state = water_pkg
        .evaluate_state(p, t, &comp)
        .expect("water state ok");
    assert_eq!(state.phase(), Phase::Liquid);
    assert_eq!(state.vapor_fraction(), 0.0);
    assert!((state.density().value() - 998.0).abs() < 1e-3);
    // Enthalpy at 323.15 (delta T = 25 K): 4184 * 25 + (200000 - 101325)/998 ≈ 104600 + 98.87 ≈ 104698.87
    let h = state.enthalpy().value();
    assert!((h - 104698.87).abs() < 2.0);
}

#[test]
fn test_mass_conservation_steady_state() {
    use nebudcs_physics::conservation::MassBalance;

    let inflows = vec![MassFlow::new(5.0).unwrap(), MassFlow::new(3.0).unwrap()];
    let outflows = vec![MassFlow::new(8.0).unwrap()];

    let balance = MassBalance::evaluate(&inflows, &outflows, 0.0);
    assert!(balance.residual().abs() < 1e-12);
    assert!(balance.is_conserved(1e-6));
}

#[test]
fn test_dynamic_mass_accumulation() {
    use nebudcs_physics::conservation::MassBalance;

    let inflows = vec![MassFlow::new(10.0).unwrap()];
    let outflows = vec![MassFlow::new(6.0).unwrap()];

    let initial_mass = 1000.0;
    let dt = 0.5;

    let new_mass = MassBalance::accumulate(initial_mass, &inflows, &outflows, dt).unwrap();
    assert_eq!(new_mass, 1002.0);
}

#[test]
fn test_species_balance_accumulation() {
    use nebudcs_physics::conservation::SpeciesBalance;

    let initial_mass = 100.0;
    let initial_comp = ComponentComposition::pure("A");

    let comp_b = ComponentComposition::pure("B");
    let inflows = vec![(MassFlow::new(10.0).unwrap(), comp_b)];
    let outflows = vec![];

    let dt = 1.0;
    let (new_mass, new_comp) =
        SpeciesBalance::accumulate(initial_mass, &initial_comp, &inflows, &outflows, dt).unwrap();

    assert_eq!(new_mass, 110.0);
    assert!((new_comp.fraction("A") - 100.0 / 110.0).abs() < 1e-10);
    assert!((new_comp.fraction("B") - 10.0 / 110.0).abs() < 1e-10);
    assert!((new_comp.fraction("A") + new_comp.fraction("B") - 1.0).abs() < 1e-12);
}

#[test]
fn test_energy_conservation_adiabatic_mixing() {
    use nebudcs_physics::conservation::EnergyBalance;

    let stream1 = (
        MassFlow::new(10.0).unwrap(),
        SpecificEnthalpy::new(100_000.0).unwrap(),
    );
    let stream2 = (
        MassFlow::new(10.0).unwrap(),
        SpecificEnthalpy::new(300_000.0).unwrap(),
    );

    let mixed_h = EnergyBalance::mix_adiabatic_streams(&[stream1, stream2]).unwrap();
    assert_eq!(mixed_h.value(), 200_000.0);
}

#[test]
fn test_darcy_weisbach_churchill_friction_laminar() {
    use nebudcs_physics::hydraulics::churchill_friction_factor;

    let re_laminar = 1000.0;
    let roughness_ratio = 0.0001;
    let f = churchill_friction_factor(re_laminar, roughness_ratio);

    // Expected f = 64 / 1000 = 0.064
    assert!((f - 0.064).abs() < 1e-4, "got f = {f}");
}

#[test]
fn test_darcy_weisbach_churchill_friction_turbulent() {
    use nebudcs_physics::hydraulics::churchill_friction_factor;

    let re_turbulent = 100_000.0;
    let roughness_ratio = 0.00005;
    let f = churchill_friction_factor(re_turbulent, roughness_ratio);

    // Blasius / Swamee-Jain / Churchill gives ~ 0.018 for smooth pipe at 10^5
    assert!(f > 0.015 && f < 0.022, "got f = {f}");
}

#[test]
fn test_valve_cv_forward_and_reverse_flow() {
    use nebudcs_physics::hydraulics::ControlValveHydraulics;

    let valve = ControlValveHydraulics::new(50.0); // Cv = 50
    let rho = Density::new(1000.0).unwrap();

    let p_high = Pressure::new(300_000.0).unwrap();
    let p_low = Pressure::new(100_000.0).unwrap();

    // 1. Forward flow: P1 > P2
    let fwd_flow = valve.calculate_mass_flow(p_high, p_low, rho, 1.0).unwrap();
    assert!(fwd_flow.value() > 0.0);

    // 2. Reverse flow: P1 < P2
    let rev_flow = valve.calculate_mass_flow(p_low, p_high, rho, 1.0).unwrap();
    assert!(rev_flow.value() < 0.0);
    assert!((fwd_flow.value() + rev_flow.value()).abs() < 1e-6);

    // 3. Zero flow: P1 == P2
    let zero_flow = valve.calculate_mass_flow(p_high, p_high, rho, 1.0).unwrap();
    assert_eq!(zero_flow.value(), 0.0);

    // 4. Closed valve: opening = 0.0
    let closed_flow = valve.calculate_mass_flow(p_high, p_low, rho, 0.0).unwrap();
    assert_eq!(closed_flow.value(), 0.0);
}

#[test]
fn test_head_source_pump_curve_and_affinity_laws() {
    use nebudcs_physics::equipment::HeadSourceModel;

    // HeadSource with shutoff Delta P = 500 kPa, rated flow Delta P = 400 kPa
    let pump = HeadSourceModel::new_quadratic(500_000.0, -100_000.0);

    // 100% speed, zero flow
    let dp_full = pump.calculate_delta_p(0.0, 1.0);
    assert_eq!(dp_full.value(), 500_000.0);

    // 50% speed: Delta P scales with speed^2
    let dp_half = pump.calculate_delta_p(0.0, 0.5);
    assert!((dp_half.value() - 500_000.0 * 0.25).abs() < 1e-6);
}

#[test]
fn test_accumulator_volume_level_and_pressure_dynamics() {
    use nebudcs_physics::equipment::AccumulatorVolumeModel;

    let total_volume = 10.0; // 10 m3
    let cross_section = 2.0; // 2 m2
    let rho = Density::new(1000.0).unwrap();

    let mut vessel = AccumulatorVolumeModel::new(total_volume, cross_section, 2000.0, rho);
    assert_eq!(vessel.liquid_level(), 0.2); // 20% full

    // Inflow: 20 kg/s, Outflow: 0 kg/s, dt = 50 s -> delta_m = 1000 kg
    let inflows = vec![MassFlow::new(20.0).unwrap()];
    let outflows = vec![];
    vessel.step(50.0, &inflows, &outflows, rho).unwrap();

    assert_eq!(vessel.holdup_mass(), 3000.0);
    assert!((vessel.liquid_level() - 0.3).abs() < 1e-6); // 30% full
}

#[test]
fn test_heat_exchange_model_duty() {
    use nebudcs_physics::equipment::HeatExchangeModel;

    let hx = HeatExchangeModel::new_with_ua(5000.0); // UA = 5000 W/K
    let t_hot = Temperature::new(373.15).unwrap();
    let t_cold = Temperature::new(303.15).unwrap();

    let q = hx.calculate_heat_duty(t_hot, t_cold).unwrap();
    assert_eq!(q, 350_000.0); // 5000 * 70 K = 350 kW
}

#[test]
fn test_recycle_solver_successive_substitution() {
    use nebudcs_physics::solver::{RecycleSolver, SolverConfig};

    let config = SolverConfig::new(50, 1e-7, 1e-7, false);
    let solver = RecycleSolver::new(config);

    // g(x) = 0.5 * x + 2.0 -> fixed point x = 4.0
    let result = solver.solve_scalar(0.0, |x| 0.5 * x + 2.0).unwrap();

    assert!((result.value - 4.0).abs() < 1e-5);
    assert!(result.converged);
    assert!(result.iterations > 0);
}

#[test]
fn test_recycle_solver_wegstein_acceleration() {
    use nebudcs_physics::solver::{RecycleSolver, SolverConfig};

    // g(x) = 0.85 * x + 1.5 -> fixed point x = 10.0
    let f = |x: f64| 0.85 * x + 1.5;

    let solver_ss = RecycleSolver::new(SolverConfig::new(100, 1e-7, 1e-7, false));
    let res_ss = solver_ss.solve_scalar(0.0, f).unwrap();

    let solver_weg = RecycleSolver::new(SolverConfig::new(100, 1e-7, 1e-7, true));
    let res_weg = solver_weg.solve_scalar(0.0, f).unwrap();

    assert!((res_weg.value - 10.0).abs() < 1e-5);
    assert!(
        res_weg.iterations < res_ss.iterations,
        "Wegstein should take fewer iterations: weg={}, ss={}",
        res_weg.iterations,
        res_ss.iterations
    );
}

#[test]
fn test_recycle_solver_divergence_detection() {
    use nebudcs_physics::solver::{RecycleSolver, SolverConfig};

    let config = SolverConfig::new(20, 1e-6, 1e-6, false);
    let solver = RecycleSolver::new(config);

    // g(x) = 2.0 * x + 1.0 -> diverges
    let err = solver.solve_scalar(1.0, |x| 2.0 * x + 1.0).unwrap_err();
    assert!(matches!(err, PhysicsError::RecycleConvergenceFailed { .. }));
}

#[test]
fn test_recycle_solver_damping_for_oscillating_system() {
    use nebudcs_physics::solver::{RecycleSolver, SolverConfig};

    // Oscillating system: g(x) = -0.9 * x + 3.8 -> fixed point x = 2.0
    // Pure successive substitution oscillates back and forth slowly.
    // Damped substitution with damping = 0.5 converges faster and without overshoot.
    let f = |x: f64| -0.9 * x + 3.8;

    let config = SolverConfig::new(100, 1e-6, 1e-6, false).with_damping(0.5);
    let solver = RecycleSolver::new(config);

    let res = solver.solve_scalar(0.0, f).unwrap();
    assert!((res.value - 2.0).abs() < 1e-5);
    assert!(res.converged);
}

#[test]
fn test_recycle_solver_multivariable_vector() {
    use nebudcs_physics::solver::{RecycleSolver, SolverConfig};

    // Coupled 2-variable recycle system:
    // x1 = 0.5 * x2 + 1.0
    // x2 = 0.4 * x1 + 2.0
    // Solution:
    // x1 = 0.5 * (0.4 * x1 + 2.0) + 1.0 = 0.2 * x1 + 2.0 => 0.8 * x1 = 2.0 => x1 = 2.5
    // x2 = 0.4 * 2.5 + 2.0 = 3.0
    let config = SolverConfig::new(50, 1e-6, 1e-6, true);
    let solver = RecycleSolver::new(config);

    let g = |x: &[f64]| vec![0.5 * x[1] + 1.0, 0.4 * x[0] + 2.0];

    let res = solver.solve_vector(&[0.0, 0.0], g).unwrap();
    assert!(res.converged);
    assert!((res.value[0] - 2.5).abs() < 1e-5);
    assert!((res.value[1] - 3.0).abs() < 1e-5);
}

#[test]
fn test_recycle_solver_deterministic_repeatability() {
    use nebudcs_physics::solver::{RecycleSolver, SolverConfig};

    let config = SolverConfig::new(50, 1e-6, 1e-6, true);
    let solver = RecycleSolver::new(config);

    let g = |x: &[f64]| vec![0.3 * x[1] + 1.5, 0.2 * x[0] + 0.8];

    let res1 = solver.solve_vector(&[1.0, 1.0], g).unwrap();
    let res2 = solver.solve_vector(&[1.0, 1.0], g).unwrap();

    assert_eq!(res1.value, res2.value);
    assert_eq!(res1.iterations, res2.iterations);
    assert_eq!(res1.residual, res2.residual);
}

#[test]
fn test_network_source_pipe_valve_vessel_dynamic_simulation() {
    use nebudcs_physics::equipment::{AccumulatorVolumeModel, FlowResistanceModel};
    use nebudcs_physics::hydraulics::pipe_pressure_drop;

    let rho = Density::new(1000.0).unwrap();
    let mu = 0.001; // 1 cP water viscosity

    let p_source = Pressure::new(400_000.0).unwrap(); // 4 bar

    let valve = FlowResistanceModel::new_from_cv(100.0);
    let mut vessel = AccumulatorVolumeModel::new(20.0, 4.0, 2000.0, rho); // 10% full (2000 kg out of 20000 kg)
    let initial_level = vessel.liquid_level();
    assert_eq!(initial_level, 0.1);

    // Run 10 ticks of dynamic simulation with dt = 1.0 s
    let dt = 1.0;
    for tick in 0..10 {
        // 1. Vessel back-pressure (headspace + liquid hydrostatic head)
        let p_vessel = vessel.bottom_pressure(rho);
        assert!(p_vessel.value() < p_source.value());

        // 2. Solve branch flow: for simple series, Delta P = P_source - P_vessel
        // Valve opening = 80% (0.8)
        let flow = valve.calculate_flow(p_source, p_vessel, rho, 0.8).unwrap();
        assert!(flow.value() > 0.0, "tick {tick}: expected positive flow");

        // Pipe pressure drop along the line (horizontal, delta_z = 0.0)
        let dp_pipe = pipe_pressure_drop(flow, 50.0, 0.1, 0.000045, 0.0, rho, mu).unwrap();
        assert!(dp_pipe.value() > 0.0);

        // 3. Step vessel accumulation
        let inflows = vec![flow];
        let outflows = vec![];
        vessel.step(dt, &inflows, &outflows, rho).unwrap();
    }

    // After 10 seconds of positive inflow, vessel mass and level must have increased monotonically
    assert!(vessel.holdup_mass() > 2000.0);
    assert!(vessel.liquid_level() > initial_level);

    // Now test valve closing (0% opening)
    let p_vessel = vessel.bottom_pressure(rho);
    let zero_flow = valve.calculate_flow(p_source, p_vessel, rho, 0.0).unwrap();
    assert_eq!(zero_flow.value(), 0.0);

    // Vessel mass remains unchanged when flow is 0
    let mass_before = vessel.holdup_mass();
    vessel.step(dt, &[zero_flow], &[], rho).unwrap();
    assert_eq!(vessel.holdup_mass(), mass_before);

    // Now test flow reversal when source pressure drops below vessel bottom pressure
    let p_depressurized_source = Pressure::new(80_000.0).unwrap(); // 0.8 bar
    let reverse_flow = valve
        .calculate_flow(p_depressurized_source, p_vessel, rho, 1.0)
        .unwrap();
    assert!(
        reverse_flow.value() < 0.0,
        "flow should be reverse without graph mutation"
    );
}

#[test]
fn test_physics_simulation_system_integration_with_sim_engine() {
    use nebudcs_model::tag::TagValue;
    use nebudcs_physics::system::GenericPhysicsSystem;
    use nebudcs_sim::{SimulationEngine, StateBuffer};
    use nebudcs_types::{EquipmentId, TagId};

    let eq_id = EquipmentId::new();
    let level_tag_id = TagId::new();

    let mut physics_sys = GenericPhysicsSystem::new();
    physics_sys.register_tank(eq_id, level_tag_id, 10.0, 2.0, 1000.0, 1000.0);

    let mut initial_state = StateBuffer::new();
    let eq_state = nebudcs_sim::EquipmentState::new("running");
    initial_state.equipment.insert(eq_id, eq_state);
    initial_state
        .tags
        .insert(level_tag_id, TagValue::Float(0.1));
    initial_state
        .qualities
        .insert(level_tag_id, nebudcs_model::tag::TagQuality::Good);

    let mut engine = SimulationEngine::new(0.5).unwrap(); // dt = 0.5s
    engine.initialize(initial_state);
    engine.add_system(physics_sys);
    engine.clock_mut().play();

    // Run 5 ticks
    for _ in 0..5 {
        engine.step().unwrap();
    }

    assert_eq!(engine.clock().tick_count(), 5);
    assert_eq!(engine.clock().time().seconds(), 2.5);

    // Tag value should be present and valid
    let read_view = engine.state();
    let tag_val = read_view.tag_value(level_tag_id).unwrap();
    match tag_val {
        TagValue::Float(val) => assert!(*val >= 0.0 && *val <= 1.0),
        _ => panic!("expected float tag value"),
    }
}

#[test]
fn test_physical_state_phase_consistency() {
    let comp = ComponentComposition::pure("H2O");

    // Liquid with q = 0.5 must fail
    let err1 = PhysicalState::new(
        Pressure::new(101_325.0).unwrap(),
        Temperature::new(300.0).unwrap(),
        Density::new(997.0).unwrap(),
        SpecificEnthalpy::new(100.0).unwrap(),
        0.018,
        0.5,
        Phase::Liquid,
        comp.clone(),
    )
    .unwrap_err();
    assert!(matches!(err1, PhysicsError::InvalidPhaseEquilibrium { .. }));

    // Vapor with q = 0.0 must fail
    let err2 = PhysicalState::new(
        Pressure::new(101_325.0).unwrap(),
        Temperature::new(400.0).unwrap(),
        Density::new(0.6).unwrap(),
        SpecificEnthalpy::new(2_000_000.0).unwrap(),
        0.018,
        0.0,
        Phase::Vapor,
        comp.clone(),
    )
    .unwrap_err();
    assert!(matches!(err2, PhysicsError::InvalidPhaseEquilibrium { .. }));

    // TwoPhase with q = 0.0 or 1.0 must fail
    let err3 = PhysicalState::new(
        Pressure::new(101_325.0).unwrap(),
        Temperature::new(373.15).unwrap(),
        Density::new(500.0).unwrap(),
        SpecificEnthalpy::new(1_000_000.0).unwrap(),
        0.018,
        1.0,
        Phase::TwoPhase,
        comp,
    )
    .unwrap_err();
    assert!(matches!(err3, PhysicsError::InvalidPhaseEquilibrium { .. }));
}

#[test]
fn test_composition_mole_to_mass_conversion() {
    use nebudcs_physics::state::CompositionBasis;
    use std::collections::BTreeMap;

    // Mixture: 50 mol% H2 (Mw = 2.0 g/mol = 0.002 kg/mol), 50 mol% CH4 (Mw = 16.0 g/mol = 0.016 kg/mol)
    let mut mw_map = BTreeMap::new();
    mw_map.insert("H2".to_string(), 0.002);
    mw_map.insert("CH4".to_string(), 0.016);

    let mut comp_map = IndexMap::new();
    comp_map.insert("H2".to_string(), 0.5);
    comp_map.insert("CH4".to_string(), 0.5);

    let mole_comp = ComponentComposition::with_basis(comp_map, CompositionBasis::Mole).unwrap();
    assert_eq!(mole_comp.basis(), CompositionBasis::Mole);

    let avg_mw = mole_comp.average_molecular_weight(&mw_map).unwrap();
    assert!((avg_mw - 0.009).abs() < 1e-6); // 0.5 * 2 + 0.5 * 16 = 9 g/mol = 0.009 kg/mol

    let mass_comp = mole_comp.to_mass_basis(&mw_map).unwrap();
    assert_eq!(mass_comp.basis(), CompositionBasis::Mass);

    // Mass fractions:
    // H2 mass in 1 mol = 0.5 * 2 = 1 g
    // CH4 mass in 1 mol = 0.5 * 16 = 8 g
    // Total mass = 9 g
    // w_H2 = 1/9 ≈ 0.111111, w_CH4 = 8/9 ≈ 0.888889
    assert!((mass_comp.fraction("H2") - 1.0 / 9.0).abs() < 1e-6);
    assert!((mass_comp.fraction("CH4") - 8.0 / 9.0).abs() < 1e-6);

    // Round trip back to mole basis
    let round_trip_mole = mass_comp.to_mole_basis(&mw_map).unwrap();
    assert!((round_trip_mole.fraction("H2") - 0.5).abs() < 1e-6);
    assert!((round_trip_mole.fraction("CH4") - 0.5).abs() < 1e-6);
}

#[test]
fn test_ideal_gas_mixture_enthalpy_mass_fraction_consistency() {
    use nebudcs_physics::properties::{IdealGasMixture, PropertyPackage};

    let mut gas_pkg = IdealGasMixture::new("H2_CH4_Rigorous");
    gas_pkg.register_component("H2", 0.002, 14300.0);
    gas_pkg.register_component("CH4", 0.016, 2220.0);

    // 50 mol% H2 and 50 mol% CH4
    let mut comp_map = IndexMap::new();
    comp_map.insert("H2".to_string(), 0.5);
    comp_map.insert("CH4".to_string(), 0.5);
    let comp = ComponentComposition::new(comp_map).unwrap(); // Mole basis

    let t = Temperature::new(348.15).unwrap(); // Delta T = 50 K
    let p = Pressure::new(101_325.0).unwrap();

    let h = gas_pkg.specific_enthalpy(p, t, &comp).unwrap().value();

    // Mw = 0.5 * 0.002 + 0.5 * 0.016 = 0.009 kg/mol
    // w_H2 = (0.5 * 0.002) / 0.009 = 1/9
    // w_CH4 = (0.5 * 0.016) / 0.009 = 8/9
    // Cp_mix = (1/9 * 14300) + (8/9 * 2220) ≈ 1588.889 + 1973.333 = 3562.222 J/(kg*K)
    // Expected h = 3562.222 * 50 = 178_111.1 J/kg
    let expected_h = ((1.0 / 9.0) * 14300.0 + (8.0 / 9.0) * 2220.0) * 50.0;
    assert!(
        (h - expected_h).abs() < 1.0,
        "calculated h: {h}, expected: {expected_h}"
    );
}

#[test]
fn test_saturated_fluid_package_vle() {
    use nebudcs_physics::properties::{PropertyPackage, SaturatedFluidPackage};

    // Saturated water model: boiling point at 101,325 Pa is 373.15 K (100 C)
    let water_pkg = SaturatedFluidPackage::new_water();
    let comp = ComponentComposition::pure("H2O");

    // 1. Subcooled liquid at 50 C (323.15 K) and 1 atm
    let subcooled = water_pkg
        .evaluate_state(
            Pressure::new(101_325.0).unwrap(),
            Temperature::new(323.15).unwrap(),
            &comp,
        )
        .unwrap();
    assert_eq!(subcooled.phase(), Phase::Liquid);
    assert_eq!(subcooled.vapor_fraction(), 0.0);
    assert!(subcooled.density().value() > 900.0);

    // 2. Superheated vapor at 150 C (423.15 K) and 1 atm
    let superheated = water_pkg
        .evaluate_state(
            Pressure::new(101_325.0).unwrap(),
            Temperature::new(423.15).unwrap(),
            &comp,
        )
        .unwrap();
    assert_eq!(superheated.phase(), Phase::Vapor);
    assert_eq!(superheated.vapor_fraction(), 1.0);
    assert!(superheated.density().value() < 2.0); // Steam density ~ 0.5-1 kg/m3

    // 3. Two-phase condition at saturation: 100 C (373.15 K) and 1 atm with intermediate enthalpy
    let sat_p = Pressure::new(101_325.0).unwrap();
    let sat_t = Temperature::new(373.15).unwrap();
    let flash_state = water_pkg
        .flash_at_enthalpy(sat_p, sat_t, &comp, 0.5)
        .unwrap();
    assert_eq!(flash_state.phase(), Phase::TwoPhase);
    assert!((flash_state.vapor_fraction() - 0.5).abs() < 1e-4);
    // Two-phase density must be between liquid and vapor
    assert!(flash_state.density().value() < subcooled.density().value());
    assert!(flash_state.density().value() > superheated.density().value());
}

#[test]
fn test_pipe_pressure_drop_with_elevation_and_reverse_flow() {
    use nebudcs_physics::hydraulics::pipe_pressure_drop;
    use nebudcs_units::DifferentialPressure;

    let rho = Density::new(1000.0).unwrap();
    let mu = 0.001; // water viscosity 1 cP

    // 1. Horizontal forward flow (delta_z = 0.0)
    let m_fwd = MassFlow::new(10.0).unwrap();
    let dp_fwd: DifferentialPressure =
        pipe_pressure_drop(m_fwd, 100.0, 0.1, 0.000045, 0.0, rho, mu).unwrap();
    assert!(dp_fwd.value() > 0.0);

    // 2. Horizontal reverse flow: Delta P friction reverses sign symmetrically
    let m_rev = MassFlow::new(-10.0).unwrap();
    let dp_rev: DifferentialPressure =
        pipe_pressure_drop(m_rev, 100.0, 0.1, 0.000045, 0.0, rho, mu).unwrap();
    assert!(dp_rev.value() < 0.0);
    assert!((dp_fwd.value() + dp_rev.value()).abs() < 1e-6);

    // 3. Elevation head: pipe going 10 meters uphill (delta_z = +10.0 m)
    // Additional hydrostatic head drop = rho * g * delta_z = 1000 * 9.80665 * 10 = 98,066.5 Pa
    let dp_uphill = pipe_pressure_drop(m_fwd, 100.0, 0.1, 0.000045, 10.0, rho, mu).unwrap();
    assert!((dp_uphill.value() - (dp_fwd.value() + 98_066.5)).abs() < 1.0);

    // 4. Stagnant pipe (zero flow): friction is zero, hydrostatic head remains
    let m_zero = MassFlow::new(0.0).unwrap();
    let dp_zero = pipe_pressure_drop(m_zero, 100.0, 0.1, 0.000045, 10.0, rho, mu).unwrap();
    assert!((dp_zero.value() - 98_066.5).abs() < 1.0);
}

#[test]
fn test_valve_cv_monotonicity_and_inverse_pressure_drop() {
    use nebudcs_physics::hydraulics::ControlValveHydraulics;

    let valve = ControlValveHydraulics::new(50.0);
    let rho = Density::new(1000.0).unwrap();

    // Monotonicity: Delta P = 50 kPa, 100 kPa, 200 kPa
    let p_out = Pressure::new(100_000.0).unwrap();
    let flow1 = valve
        .calculate_mass_flow(Pressure::new(150_000.0).unwrap(), p_out, rho, 1.0)
        .unwrap();
    let flow2 = valve
        .calculate_mass_flow(Pressure::new(200_000.0).unwrap(), p_out, rho, 1.0)
        .unwrap();
    let flow3 = valve
        .calculate_mass_flow(Pressure::new(300_000.0).unwrap(), p_out, rho, 1.0)
        .unwrap();

    assert!(flow1.value() < flow2.value());
    assert!(flow2.value() < flow3.value());

    // Opening sensitivity: opening = 20%, 50%, 100% at fixed Delta P = 100 kPa
    let p_in = Pressure::new(200_000.0).unwrap();
    let flow_20 = valve.calculate_mass_flow(p_in, p_out, rho, 0.2).unwrap();
    let flow_50 = valve.calculate_mass_flow(p_in, p_out, rho, 0.5).unwrap();
    let flow_100 = valve.calculate_mass_flow(p_in, p_out, rho, 1.0).unwrap();

    assert!(flow_20.value() < flow_50.value());
    assert!(flow_50.value() < flow_100.value());

    // Inverse pressure drop test:
    // For flow_100, calculate_pressure_drop must reproduce ~ 100,000 Pa!
    let dp_calc = valve.calculate_pressure_drop(flow_100, rho, 1.0).unwrap();
    assert!(
        (dp_calc.value() - 100_000.0).abs() < 200.0,
        "calculated dp: {}",
        dp_calc.value()
    );
}

#[test]
fn test_accumulator_volume_step_detailed_species_and_enthalpy() {
    use indexmap::IndexMap;
    use nebudcs_physics::equipment::AccumulatorVolumeModel;
    use nebudcs_physics::state::ComponentComposition;
    use nebudcs_units::{Density, MassFlow, SpecificEnthalpy};

    let mut comp_initial = IndexMap::new();
    comp_initial.insert("water".to_string(), 0.8);
    comp_initial.insert("ethanol".to_string(), 0.2);
    let comp = ComponentComposition::new(comp_initial).unwrap();

    let rho = Density::new(900.0).unwrap();
    let mut vessel = AccumulatorVolumeModel::new(10.0, 2.0, 1000.0, rho);
    vessel.set_composition(comp);
    vessel.set_specific_enthalpy(SpecificEnthalpy::new(200_000.0).unwrap());

    // Inflow: 10 kg/s of pure ethanol at higher enthalpy 400 kJ/kg
    let mut comp_in = IndexMap::new();
    comp_in.insert("water".to_string(), 0.0);
    comp_in.insert("ethanol".to_string(), 1.0);
    let comp_feed = ComponentComposition::new(comp_in).unwrap();

    let feed_stream = (
        MassFlow::new(10.0).unwrap(),
        comp_feed,
        SpecificEnthalpy::new(400_000.0).unwrap(),
    );
    let outflows = [MassFlow::new(5.0).unwrap()];

    // Step 2 seconds
    vessel
        .step_detailed(2.0, &[feed_stream], &outflows, 0.0, 0.0)
        .unwrap();

    // Mass: 1000 + (10 - 5)*2 = 1010 kg
    assert!((vessel.holdup_mass() - 1010.0).abs() < 1e-6);

    // Ethanol fraction must have increased from 0.2
    let new_comp = vessel.composition().unwrap();
    let ethanol_frac = new_comp.fractions().get("ethanol").copied().unwrap();
    assert!(
        ethanol_frac > 0.20,
        "ethanol frac should increase, got {}",
        ethanol_frac
    );

    // Enthalpy must have increased from 200 kJ/kg
    let new_h = vessel.specific_enthalpy().unwrap().value();
    assert!(new_h > 200_000.0, "enthalpy should increase, got {}", new_h);
}

#[test]
fn test_custom_equipment_capability_composition_and_engine_integration() {
    use nebudcs_model::tag::{TagQuality, TagValue};
    use nebudcs_physics::equipment::{
        AccumulatorVolumeModel, EquipmentCapability, EquipmentExecutionContext,
        GenericEquipmentPhysics,
    };
    use nebudcs_physics::error::PhysicsError;
    use nebudcs_physics::properties::{PropertyPackage, SaturatedFluidPackage};
    use nebudcs_physics::system::GenericPhysicsSystem;
    use nebudcs_sim::{EquipmentState, SimulationEngine, StateBuffer};
    use nebudcs_types::{EquipmentId, TagId};
    use nebudcs_units::{Density, MassFlow, Pressure, Temperature};

    // Define a custom flash separation capability completely outside core simulation crates
    #[derive(Debug)]
    struct CustomFlashSeparationCapability {
        water_package: SaturatedFluidPackage,
        overhead_vapor_rate: f64,
        operating_p: Pressure,
        operating_t: Temperature,
    }

    impl CustomFlashSeparationCapability {
        fn new() -> Self {
            Self {
                water_package: SaturatedFluidPackage::new_water(),
                overhead_vapor_rate: 0.0,
                operating_p: Pressure::new_unchecked(101_325.0),
                operating_t: Temperature::new_unchecked(373.15),
            }
        }
    }

    impl EquipmentCapability for CustomFlashSeparationCapability {
        fn capability_name(&self) -> &str {
            "CustomFlashSeparation"
        }

        fn evaluate(&mut self, ctx: &EquipmentExecutionContext<'_>) -> Result<(), PhysicsError> {
            let total_inflow: f64 = ctx.inlet_flows.iter().map(|f| f.value()).sum();
            let dummy_comp = ComponentComposition::pure("water");
            let state = self.water_package.evaluate_state(
                self.operating_p,
                self.operating_t,
                &dummy_comp,
            )?;
            let vapor_fraction = if state.phase() == Phase::Vapor {
                1.0
            } else {
                0.4
            };
            self.overhead_vapor_rate = total_inflow * vapor_fraction;
            Ok(())
        }

        fn get_variable(&self, name: &str) -> Option<f64> {
            match name {
                "overhead_vapor_rate" => Some(self.overhead_vapor_rate),
                "operating_pressure" => Some(self.operating_p.value()),
                "operating_temperature" => Some(self.operating_t.value()),
                _ => None,
            }
        }

        fn set_variable(&mut self, name: &str, value: f64) -> Result<(), PhysicsError> {
            match name {
                "operating_pressure" => {
                    self.operating_p = Pressure::new(value).map_err(|e| {
                        PhysicsError::InvalidOperatingCondition {
                            message: e.to_string(),
                        }
                    })?;
                    Ok(())
                }
                "operating_temperature" => {
                    self.operating_t = Temperature::new(value).map_err(|e| {
                        PhysicsError::InvalidOperatingCondition {
                            message: e.to_string(),
                        }
                    })?;
                    Ok(())
                }
                _ => Err(PhysicsError::InvalidOperatingCondition {
                    message: format!("unknown variable {name}"),
                }),
            }
        }
    }

    let eq_id = EquipmentId::new();
    let level_tag_id = TagId::new();
    let vapor_tag_id = TagId::new();

    // 1. Build composite generic equipment
    let mut custom_eq = GenericEquipmentPhysics::new(eq_id);

    // Capability 1: AccumulatorVolume (mass, level, hydrostatic pressure)
    let rho = Density::new(1000.0).unwrap();
    let vessel_vol = AccumulatorVolumeModel::new(10.0, 2.0, 5000.0, rho);
    custom_eq.add_capability(vessel_vol);

    // Capability 2: Custom flash separation capability
    let flash_cap = CustomFlashSeparationCapability::new();
    custom_eq.add_capability(flash_cap);

    // Set inlet flows and tag bindings
    custom_eq.set_inlet_flows(vec![MassFlow::new(20.0).unwrap()]);
    custom_eq.bind_tag("level", level_tag_id);
    custom_eq.bind_tag("overhead_vapor_rate", vapor_tag_id);

    // 2. Register into GenericPhysicsSystem
    let mut physics_sys = GenericPhysicsSystem::new();
    physics_sys.register_equipment(custom_eq);

    // 3. Register into Phase 3 SimulationEngine
    let mut initial_state = StateBuffer::new();
    let mut eq_state = EquipmentState::new("running");
    eq_state.set_var("mass", 5000.0);
    initial_state.equipment.insert(eq_id, eq_state);

    initial_state
        .tags
        .insert(level_tag_id, TagValue::Float(0.5));
    initial_state
        .qualities
        .insert(level_tag_id, TagQuality::Good);
    initial_state
        .tags
        .insert(vapor_tag_id, TagValue::Float(0.0));
    initial_state
        .qualities
        .insert(vapor_tag_id, TagQuality::Good);

    let mut engine = SimulationEngine::new(0.5).unwrap();
    engine.initialize(initial_state);
    engine.add_system(physics_sys);
    engine.clock_mut().play();

    // Step 5 ticks
    for _ in 0..5 {
        engine.step().unwrap();
    }

    assert_eq!(engine.clock().tick_count(), 5);
    assert_eq!(engine.clock().time().seconds(), 2.5);

    let read_view = engine.state();

    // Verify level tag is published
    let level_val = read_view.tag_value(level_tag_id).unwrap();
    match level_val {
        TagValue::Float(val) => assert!(*val >= 0.5 && *val <= 1.0, "level: {val}"),
        _ => panic!("expected float tag"),
    }

    // Verify custom vapor tag is published: 20 kg/s * 0.4 = 8 kg/s
    let vapor_val = read_view.tag_value(vapor_tag_id).unwrap();
    match vapor_val {
        TagValue::Float(val) => assert!((*val - 8.0).abs() < 1e-6, "vapor rate: {val}"),
        _ => panic!("expected float tag for vapor rate"),
    }

    // Verify equipment state variables updated in double-buffered storage
    let read_eq = read_view.equipment(eq_id).unwrap();
    assert!(read_eq.get_var("mass").is_some());
    assert!(read_eq.get_var("bottom_pressure").is_some());
}

#[test]
fn test_species_balance_basis_mismatch_rejection() {
    use nebudcs_physics::conservation::SpeciesBalance;
    use nebudcs_physics::state::{ComponentComposition, CompositionBasis};
    use nebudcs_units::MassFlow;

    let initial_comp = ComponentComposition::pure_with_basis("A", CompositionBasis::Mole);
    let inflow_comp = ComponentComposition::pure_with_basis("B", CompositionBasis::Mass);

    let inflows = vec![(MassFlow::new(5.0).unwrap(), inflow_comp)];
    let err = SpeciesBalance::accumulate(100.0, &initial_comp, &inflows, &[], 1.0).unwrap_err();

    assert!(matches!(
        err,
        nebudcs_physics::PhysicsError::InvalidComposition { .. }
    ));
}

#[test]
fn test_ideal_gas_mixture_molecular_weight_mass_basis() {
    use indexmap::IndexMap;
    use nebudcs_physics::properties::{IdealGasMixture, PropertyPackage};
    use nebudcs_physics::state::{ComponentComposition, CompositionBasis};

    let mut pkg = IdealGasMixture::new("H2_CH4_Mix");
    pkg.register_component("H2", 0.002, 14300.0);
    pkg.register_component("CH4", 0.016, 2220.0);

    // 50 wt% H2 and 50 wt% CH4
    let mut map = IndexMap::new();
    map.insert("H2".to_string(), 0.5);
    map.insert("CH4".to_string(), 0.5);
    let mass_comp = ComponentComposition::with_basis(map, CompositionBasis::Mass).unwrap();

    // Mw = 1 / (0.5/0.002 + 0.5/0.016) = 1 / (250 + 31.25) = 1 / 281.25 ≈ 0.0035555 kg/mol
    let mw = pkg.molecular_weight(&mass_comp).unwrap();
    let expected_mw = 1.0 / (0.5 / 0.002 + 0.5 / 0.016);
    assert!(
        (mw - expected_mw).abs() < 1e-7,
        "mw: {mw}, expected: {expected_mw}"
    );
}

#[test]
fn test_saturated_fluid_package_flash_from_enthalpy_and_saturation_temperature() {
    use indexmap::IndexMap;
    use nebudcs_physics::properties::SaturatedFluidPackage;
    use nebudcs_physics::state::{ComponentComposition, Phase};
    use nebudcs_units::{Pressure, SpecificEnthalpy};

    let pkg = SaturatedFluidPackage::new_water();
    let mut map = IndexMap::new();
    map.insert("H2O".to_string(), 1.0);
    let comp = ComponentComposition::new(map).unwrap();

    // 1. Saturation temperature at 1 atm (101,325 Pa) must equal 373.15 K
    let p_1atm = Pressure::new(101_325.0).unwrap();
    let t_sat_1atm = pkg.saturation_temperature(p_1atm).unwrap();
    assert!((t_sat_1atm - 373.15).abs() < 1e-4);

    // 2. Saturation temperature at 2 atm (202,650 Pa)
    let p_2atm = Pressure::new(202_650.0).unwrap();
    let t_sat_2atm = pkg.saturation_temperature(p_2atm).unwrap();
    assert!(t_sat_2atm > 373.15 && t_sat_2atm < 400.0);

    // 3. Flash from enthalpy at 1 atm:
    // T_ref = 298.15 K, T_sat = 373.15 K -> delta_T = 75 K
    // h_L_sat = 4184 * 75 = 313,800 J/kg
    // h_V_sat = 313,800 + 2,257,000 = 2,570,800 J/kg

    // Case A: Subcooled liquid at h = 100,000 J/kg
    let state_sub = pkg
        .flash_from_enthalpy(p_1atm, SpecificEnthalpy::new(100_000.0).unwrap(), &comp)
        .unwrap();
    assert_eq!(state_sub.phase(), Phase::Liquid);
    assert_eq!(state_sub.vapor_fraction(), 0.0);
    assert!(state_sub.temperature().value() < 373.15);

    // Case B: Saturated two-phase at h = 1,442,300 J/kg (exactly 50% vapor)
    let h_50 = 313_800.0 + 0.5 * 2_257_000.0;
    let state_2p = pkg
        .flash_from_enthalpy(p_1atm, SpecificEnthalpy::new(h_50).unwrap(), &comp)
        .unwrap();
    assert_eq!(state_2p.phase(), Phase::TwoPhase);
    assert!((state_2p.vapor_fraction() - 0.5).abs() < 1e-4);
    assert!((state_2p.temperature().value() - 373.15).abs() < 1e-4);

    // Case C: Superheated vapor at h = 2,700,000 J/kg
    let state_sup = pkg
        .flash_from_enthalpy(p_1atm, SpecificEnthalpy::new(2_700_000.0).unwrap(), &comp)
        .unwrap();
    assert_eq!(state_sup.phase(), Phase::Vapor);
    assert_eq!(state_sup.vapor_fraction(), 1.0);
    assert!(state_sup.temperature().value() > 373.15);
}

#[test]
fn test_recycle_solver_max_iterations_failure_slow_convergence() {
    use nebudcs_physics::solver::{RecycleSolver, SolverConfig};

    // Very slowly converging contraction: g(x) = 0.999 * x + 0.01 (fixed point = 10.0)
    // Starting from x0 = 0.0, after 10 iterations x10 ≈ 0.0955 (residual ≈ 0.0099 >> 1e-6)
    let config = SolverConfig::new(10, 1e-6, 1e-6, false);
    let solver = RecycleSolver::new(config);

    let err = solver.solve_scalar(0.0, |x| 0.999 * x + 0.01).unwrap_err();
    match err {
        nebudcs_physics::PhysicsError::RecycleConvergenceFailed {
            iterations,
            max_iterations,
            residual,
            tolerance,
        } => {
            assert_eq!(iterations, 10);
            assert_eq!(max_iterations, 10);
            assert!(residual > tolerance);
        }
        other => panic!("expected RecycleConvergenceFailed, got {other:?}"),
    }
}

#[test]
fn test_dynamic_causal_propagation_network() {
    use nebudcs_physics::equipment::{AccumulatorVolumeModel, FlowResistanceModel};
    use nebudcs_physics::hydraulics::pipe_pressure_drop;
    use nebudcs_units::{Density, Pressure};

    let rho = Density::new(1000.0).unwrap();
    let mu = 0.001;

    let valve = FlowResistanceModel::new_from_cv(80.0);
    let mut vessel = AccumulatorVolumeModel::new(10.0, 2.0, 1000.0, rho);

    let p_vessel_init = vessel.bottom_pressure(rho);

    // 1. Nominal forward flow at P_source = 300 kPa, opening = 100%
    let p_src_high = Pressure::new(300_000.0).unwrap();
    let flow_nominal = valve
        .calculate_flow(p_src_high, p_vessel_init, rho, 1.0)
        .unwrap();
    assert!(flow_nominal.value() > 0.0);

    // 2. Causal effect 1: Source pressure decreases -> flow decreases
    let p_src_low = Pressure::new(200_000.0).unwrap();
    let flow_reduced_press = valve
        .calculate_flow(p_src_low, p_vessel_init, rho, 1.0)
        .unwrap();
    assert!(
        flow_reduced_press.value() < flow_nominal.value(),
        "reducing source pressure must reduce flow"
    );

    // 3. Causal effect 2: Valve opening decreases -> flow decreases
    let flow_throttled = valve
        .calculate_flow(p_src_high, p_vessel_init, rho, 0.3)
        .unwrap();
    assert!(
        flow_throttled.value() < flow_nominal.value(),
        "throttling valve opening must reduce flow"
    );

    // 4. Causal effect 3: Accumulation rate responds directly to flow
    let dt = 1.0;
    let mut vessel_fast = vessel.clone();
    let mut vessel_slow = vessel.clone();

    vessel_fast.step(dt, &[flow_nominal], &[], rho).unwrap();
    vessel_slow
        .step(dt, &[flow_reduced_press], &[], rho)
        .unwrap();

    let d_mass_fast = vessel_fast.holdup_mass() - vessel.holdup_mass();
    let d_mass_slow = vessel_slow.holdup_mass() - vessel.holdup_mass();
    assert!(
        d_mass_fast > d_mass_slow,
        "higher inflow must produce greater accumulation: fast={d_mass_fast}, slow={d_mass_slow}"
    );

    // 5. Causal effect 4: Pressure reversal reverses flow and drains vessel inventory
    let p_src_vacuum = Pressure::new(50_000.0).unwrap(); // 0.5 bar < vessel ~1.1 bar
    let flow_reverse = valve
        .calculate_flow(p_src_vacuum, p_vessel_init, rho, 1.0)
        .unwrap();
    assert!(
        flow_reverse.value() < 0.0,
        "reverse flow must have negative value"
    );

    // In reverse flow, line pipe pressure drop also reverses sign
    let dp_pipe_rev = pipe_pressure_drop(flow_reverse, 20.0, 0.05, 0.000045, 0.0, rho, mu).unwrap();
    assert!(
        dp_pipe_rev.value() < 0.0,
        "reverse pipe pressure drop must be negative"
    );

    // Stepping vessel with outflow equal to reverse flow rate drains inventory
    let outflow_rate = nebudcs_units::MassFlow::new(-flow_reverse.value()).unwrap();
    vessel.step(dt, &[], &[outflow_rate], rho).unwrap();
    assert!(
        vessel.holdup_mass() < 1000.0,
        "draining vessel must decrease inventory: {}",
        vessel.holdup_mass()
    );
}
