//! Comprehensive scientific verification tests for Stage 12 Reaction Kinetics and Heat Transfer.

use indexmap::IndexMap;
use nebudcs_physics::heat_transfer::{
    EpsilonNtuSolver, FlowArrangement, FoulingParameters, HeatExchangerUnit, LmtdCalculator,
};
use nebudcs_physics::reaction::{
    CstrConfig, CstrReactorModel, CstrState, PfrConfig, PfrReactorModel, ReactionDefinition,
    ReactionSystem,
};
use nebudcs_physics::thermo::provenance::ScientificProvenance;

fn test_provenance(model_name: &str) -> ScientificProvenance {
    ScientificProvenance::literature(
        "Fogler, H.S., Elements of Chemical Reaction Engineering (4th ed., 2006)",
        model_name,
        None,
        Some(0.01),
    )
}

#[test]
fn test_arrhenius_kinetics_analytical_and_monotonicity() {
    let mut stoich = IndexMap::new();
    stoich.insert("Reactant_A".to_string(), -1.0);
    stoich.insert("Product_B".to_string(), 1.0);

    let mut orders = IndexMap::new();
    orders.insert("Reactant_A".to_string(), 1.0);

    // Elementary 1st order reaction: k0 = 1.0e8 s^-1, Ea = 65,000 J/mol, dHr = -75,000 J/mol
    let rxn = ReactionDefinition::new_irreversible(
        "rxn-01",
        "A -> B exothermic isomerization",
        stoich,
        orders,
        1.0e8,
        65_000.0,
        -75_000.0,
        test_provenance("arrhenius_kinetics"),
    );

    let t1 = 300.0;
    let t2 = 350.0;
    let k1 = rxn.forward_rate_constant(t1).unwrap();
    let k2 = rxn.forward_rate_constant(t2).unwrap();

    // Arrhenius temperature dependence must be strictly monotonically increasing with T
    assert!(k2 > k1, "k(350K)={k2} must exceed k(300K)={k1}");

    // Analytical verification: k = A * exp(-Ea / (R * T))
    let r_gas: f64 = 8.314462618;
    let expected_k1 = 1.0e8 * (-65_000.0f64 / (r_gas * 300.0)).exp();
    let rel_err = (k1 - expected_k1).abs() / expected_k1;
    assert!(rel_err < 1e-6, "relative error {rel_err} exceeds 1e-6");

    // Rate evaluation
    let mut conc = IndexMap::new();
    conc.insert("Reactant_A".to_string(), 2000.0); // 2000 mol/m³
    let rate_res = rxn.evaluate_rate(t1, 101_325.0, &conc).unwrap();

    let expected_rate = expected_k1 * 2000.0;
    assert!((rate_res.net_rate - expected_rate).abs() / expected_rate < 1e-5);
    // Exothermic: volumetric heat duty = rate * (-dHr) > 0
    assert!(rate_res.volumetric_heat_duty_w_per_m3 > 0.0);
}

#[test]
fn test_reversible_reaction_chemical_equilibrium() {
    let mut stoich = IndexMap::new();
    stoich.insert("A".to_string(), -1.0);
    stoich.insert("B".to_string(), 1.0);

    let mut fwd_orders = IndexMap::new();
    fwd_orders.insert("A".to_string(), 1.0);

    let mut rev_orders = IndexMap::new();
    rev_orders.insert("B".to_string(), 1.0);

    let rxn = ReactionDefinition::new_reversible(
        "rxn-rev",
        "A <=> B reversible",
        stoich,
        fwd_orders,
        rev_orders,
        1.0e7,
        50_000.0,
        2.0e7,
        55_000.0,
        -5_000.0,
        test_provenance("reversible_reaction"),
    );

    let t = 350.0;
    let k_f = rxn.forward_rate_constant(t).unwrap();
    let k_r = rxn.reverse_rate_constant(t).unwrap().unwrap();
    let k_eq = k_f / k_r;

    // At chemical equilibrium, C_B / C_A = K_eq, so net rate must equal 0.0
    let c_a = 1000.0;
    let c_b = c_a * k_eq;
    let mut conc = IndexMap::new();
    conc.insert("A".to_string(), c_a);
    conc.insert("B".to_string(), c_b);

    let res = rxn.evaluate_rate(t, 200_000.0, &conc).unwrap();
    assert!(res.net_rate.abs() < 1e-4, "net rate at equilibrium must be near zero, got {}", res.net_rate);
}

#[test]
fn test_cstr_dynamic_simulation_and_runaway_warning() {
    let mut stoich = IndexMap::new();
    stoich.insert("A".to_string(), -1.0);
    stoich.insert("B".to_string(), 1.0);

    let mut orders = IndexMap::new();
    orders.insert("A".to_string(), 1.0);

    let rxn = ReactionDefinition::new_irreversible(
        "rxn-cstr",
        "A -> B exothermic reactor",
        stoich,
        orders,
        1.0e8,
        70_000.0,
        -50_000.0,
        test_provenance("cstr_kinetics"),
    );

    let mut sys = ReactionSystem::new("CSTR Exothermic System");
    sys.add_reaction(rxn);

    let config = CstrConfig {
        volume_m3: 1.0,
        jacket_area_m2: 6.0,
        jacket_u_w_per_m2_k: 600.0,
        fluid_density_kg_per_m3: 950.0,
        fluid_cp_j_per_kg_k: 2400.0,
        reaction_system: sys,
        limiting_reactant: "A".to_string(),
        primary_product: Some("B".to_string()),
        runaway_temp_threshold_k: 375.0,
    };

    let mut initial_conc = IndexMap::new();
    initial_conc.insert("A".to_string(), 1000.0);
    initial_conc.insert("B".to_string(), 200.0);

    let initial_state = CstrState {
        temperature_k: 350.0,
        pressure_pa: 200_000.0,
        concentrations: initial_conc,
        jacket_temperature_k: 335.0,
        conversion: 0.0,
        yield_fraction: 0.0,
        heat_generation_w: 0.0,
        heat_removal_w: 0.0,
        mass_balance_residual: 0.0,
        energy_balance_residual: 0.0,
        is_runaway_warning: false,
        reaction_rates: Vec::new(),
    };

    let mut cstr = CstrReactorModel::new(config, initial_state);

    let mut feed_conc = IndexMap::new();
    feed_conc.insert("A".to_string(), 1500.0);
    feed_conc.insert("B".to_string(), 0.0);

    // Run for 50 steps with normal jacket cooling (335 K)
    for _ in 0..50 {
        cstr.step(2.0, 10.0, 345.0, &feed_conc, 335.0).unwrap();
    }

    // Process should reach meaningful conversion and stable state
    assert!(cstr.state.conversion > 0.15, "conversion was {}", cstr.state.conversion);
    assert!(!cstr.state.is_runaway_warning);
    assert!(cstr.state.mass_balance_residual < 1e-4);

    // Simulate cooling loss: jacket coolant failure spikes jacket to 420 K and feed to 390 K
    for _ in 0..80 {
        cstr.step(2.0, 10.0, 390.0, &feed_conc, 420.0).unwrap();
    }

    // Should detect runaway condition or high temperature trip warning
    assert!(
        cstr.state.is_runaway_warning || cstr.state.temperature_k > 375.0,
        "CSTR must detect thermal runaway when cooling fails: T={}, warning={}",
        cstr.state.temperature_k,
        cstr.state.is_runaway_warning
    );
}

#[test]
fn test_pfr_spatial_profiles_and_conservation() {
    let mut stoich = IndexMap::new();
    stoich.insert("A".to_string(), -1.0);
    stoich.insert("B".to_string(), 1.0);

    let mut orders = IndexMap::new();
    orders.insert("A".to_string(), 1.0);

    let rxn = ReactionDefinition::new_irreversible(
        "rxn-pfr",
        "A -> B tubular reaction",
        stoich,
        orders,
        2.0e7,
        58_000.0,
        -45_000.0,
        test_provenance("pfr_kinetics"),
    );

    let mut sys = ReactionSystem::new("PFR System");
    sys.add_reaction(rxn);

    let config = PfrConfig {
        length_m: 10.0,
        diameter_m: 0.15,
        tube_count: 4,
        wall_temperature_k: 320.0,
        overall_u_w_per_m2_k: 450.0,
        fluid_density_kg_per_m3: 850.0,
        fluid_cp_j_per_kg_k: 2200.0,
        reaction_system: sys,
        limiting_reactant: "A".to_string(),
        primary_product: Some("B".to_string()),
        discretization_slices: 50,
    };

    let pfr = PfrReactorModel::new(config);

    let mut feed_conc = IndexMap::new();
    feed_conc.insert("A".to_string(), 1200.0);
    feed_conc.insert("B".to_string(), 0.0);

    let result = pfr.solve(5.0, 310.0, 400_000.0, &feed_conc).unwrap();

    // Axial profile monotonicity checks
    assert_eq!(result.axial_profiles.len(), 51);
    for i in 1..result.axial_profiles.len() {
        let prev = &result.axial_profiles[i - 1];
        let curr = &result.axial_profiles[i];
        let ca_prev = prev.concentrations.get("A").unwrap();
        let ca_curr = curr.concentrations.get("A").unwrap();
        let cb_prev = prev.concentrations.get("B").unwrap();
        let cb_curr = curr.concentrations.get("B").unwrap();

        assert!(ca_curr <= ca_prev, "Reactant A concentration must monotonically decrease along PFR");
        assert!(cb_curr >= cb_prev, "Product B concentration must monotonically increase along PFR");
        assert!(curr.conversion >= prev.conversion, "Conversion must increase along PFR");
    }

    // Overall conversion and yield must be positive
    assert!(result.overall_conversion > 0.20, "PFR conversion={}", result.overall_conversion);
    assert!(result.overall_yield > 0.20);
    // Strict energy balance residual
    assert!(result.energy_balance_residual < 1e-3, "residual={}", result.energy_balance_residual);
}

#[test]
fn test_heat_exchanger_epsilon_ntu_and_thermodynamic_bounds() {
    let ua = 25_000.0; // W/K
    let m_hot = 5.0; // kg/s
    let cp_hot = 4180.0; // J/(kg*K) => C_h = 20,900 W/K
    let t_h_in = 368.15; // 95 °C

    let m_cold = 8.0; // kg/s
    let cp_cold = 2500.0; // J/(kg*K) => C_c = 20,000 W/K
    let t_c_in = 293.15; // 20 °C

    // Rate counter-current
    let res_counter = EpsilonNtuSolver::rate_exchanger(
        ua,
        m_hot,
        cp_hot,
        t_h_in,
        m_cold,
        cp_cold,
        t_c_in,
        FlowArrangement::CounterCurrent,
    ).unwrap();

    // Rate co-current
    let res_co = EpsilonNtuSolver::rate_exchanger(
        ua,
        m_hot,
        cp_hot,
        t_h_in,
        m_cold,
        cp_cold,
        t_c_in,
        FlowArrangement::CoCurrent,
    ).unwrap();

    // Rate 1-2 Shell-and-Tube
    let res_st = EpsilonNtuSolver::rate_exchanger(
        ua,
        m_hot,
        cp_hot,
        t_h_in,
        m_cold,
        cp_cold,
        t_c_in,
        FlowArrangement::ShellAndTube12,
    ).unwrap();

    // 2nd Law of Thermodynamics: Counter-current effectiveness strictly exceeds co-current
    assert!(
        res_counter.effectiveness > res_co.effectiveness,
        "counter-current eff ({}) must exceed co-current eff ({})",
        res_counter.effectiveness,
        res_co.effectiveness
    );
    assert!(res_counter.duty_w > res_co.duty_w);

    // Shell-and-tube falls between counter-current and co-current
    assert!(res_st.effectiveness <= res_counter.effectiveness);
    assert!(res_st.effectiveness >= res_co.effectiveness);

    // Energy balance check on both
    assert!(res_counter.energy_balance_residual < 1e-6);
    assert!(res_co.energy_balance_residual < 1e-6);
    assert!(res_st.energy_balance_residual < 1e-6);
}

#[test]
fn test_fouling_factor_reduces_heat_transfer_duty() {
    let fouling = FoulingParameters::default();
    let u_clean = fouling.clean_u();
    let u_fouled = fouling.fouled_u();

    // Fouled overall coefficient must be strictly lower than clean
    assert!(u_fouled < u_clean, "U_fouled ({u_fouled}) must be less than U_clean ({u_clean})");

    let clean_unit = HeatExchangerUnit::new(
        20.0,
        FlowArrangement::CounterCurrent,
        FoulingParameters {
            r_fouling_hot: 0.0,
            r_fouling_cold: 0.0,
            ..FoulingParameters::default()
        },
        test_provenance("heat_exchanger_clean"),
    );

    let fouled_unit = HeatExchangerUnit::new(
        20.0,
        FlowArrangement::CounterCurrent,
        FoulingParameters::default(),
        test_provenance("heat_exchanger_fouled"),
    );

    let res_clean = clean_unit.rate(4.0, 4180.0, 360.0, 6.0, 2000.0, 300.0).unwrap();
    let res_fouled = fouled_unit.rate(4.0, 4180.0, 360.0, 6.0, 2000.0, 300.0).unwrap();

    assert!(
        res_clean.duty_w > res_fouled.duty_w,
        "Clean exchanger duty ({}) must exceed fouled duty ({})",
        res_clean.duty_w,
        res_fouled.duty_w
    );
}

#[test]
fn test_lmtd_counter_current_and_smooth_taylor_series() {
    // Standard distinct delta T:
    let lmtd = LmtdCalculator::calculate_counter_current(360.0, 330.0, 300.0, 320.0).unwrap();
    // dt1 = 360 - 320 = 40 K, dt2 = 330 - 300 = 30 K
    // LMTD = (40 - 30) / ln(40/30) = 10 / 0.287682 = 34.76 K
    assert!((lmtd - 34.76).abs() < 0.05);

    // Equal delta T (dt1 == dt2): tests numerical stability without division by zero
    let lmtd_equal = LmtdCalculator::calculate_counter_current(360.0, 330.0, 300.0, 330.0).unwrap();
    // dt1 = 360 - 330 = 30 K, dt2 = 330 - 300 = 30 K => LMTD = 30.0 K
    assert!((lmtd_equal - 30.0).abs() < 1e-4);
}
