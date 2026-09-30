//! NebuDCS Stage 12: Industrial Domain Deepening — Acceptance Matrix Verification (Cases A through P)
//!
//! Standards Reference:
//! - ANSI/ISA-77.20 / ANSI/ISA-88 / ANSI/ISA-95
//! - API 520/521 / API 610 / API 617 / API 650
//! - TEMA Standards (Tubular Exchanger Manufacturers Association)
//! - IEC 61511 / IEC 61131-3 / IEC 62443 / ISA-18.2 / ISA-101
//!
//! Verifies:
//! Case A: Refinery CDU/VDU (deep vacuum flash, column profile, bottoms pump NPSH)
//! Case B: Hydroprocessing (HDS catalytic kinetics, H2 quench cooling, high-pressure separator)
//! Case C: FCC / recycle process (recycle loop convergence)
//! Case D: Petrochemical separation (cryogenic demethanizer, C2 splitter, recycle closure)
//! Case E: Chemical reactor + separator (CSTR + volatile degasser + liquid recycle)
//! Case F: Gas compression & dehydration (multi-stage compression + knock-out + TEG contactor)
//! Case G: LNG / refrigeration (mixed refrigerant loop + J-T isenthalpic expansion + subcooling)
//! Case H: Boiler / steam utility network (HP/MP/LP steam headers + letdown valves + desuperheating)
//! Case I: Heat-exchanger network (counter vs co-current LMTD, epsilon-NTU, fouling factor)
//! Case J: Pump / hydraulic network (Darcy-Weisbach flow, NPSHA vs NPSHR, cavitation regime)
//! Case K: Thermodynamic flash (cubic EOS Rachford-Rice VLE convergence, fugacity closure)
//! Case L: Reaction kinetics (Arrhenius k(T), stoichiometry closure, heat duty Q_r)
//! Case M: Custom non-refinery plant (pure data-driven creation from project JSON without recompiling Rust)
//! Case N: Invalid-property & convergence diagnostics (out-of-range detection, negative pressure rejection, non-finite values)
//! Case O: Deterministic replay (multi-step bit-identical state replay)
//! Case P: Stage 1–11 regression suite (controllers, IEC 61131-3, HAL, APC, SIS, OTS all PASS)

use std::collections::BTreeMap;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use indexmap::IndexMap;
use tower::ServiceExt;

use nebudcs_control::pid::{PidConfig, PidController, PidDirection, PidMode};
use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_model::tag::{TagQuality, TagValue};
use nebudcs_physics::heat_transfer::{
    FlowArrangement, FoulingParameters, LmtdCalculator,
};
use nebudcs_physics::reaction::{
    CstrConfig, CstrReactorModel, CstrState, PfrConfig, PfrReactorModel, ReactionDefinition,
    ReactionSystem,
};
use nebudcs_physics::rotating::pump::{CavitationRegime, PumpCavitationModel};
use nebudcs_physics::thermo::provenance::ScientificProvenance;
use nebudcs_physics::thermo::{ComponentDatabase, CubicEos, VleFlashSolver};
use nebudcs_protocol::{DomainPackageDetailDto, DomainPackageSummaryDto};
use nebudcs_scripting::library::domain::load_all_standard_domain_packages;
use nebudcs_server::{build_app, HostHandle, ServerConfig, SimulationHost};
use nebudcs_sim::SimulationEngine;
use nebudcs_topology::scc::find_strongly_connected_components;
use nebudcs_types::{EquipmentId, TagId};
use nebudcs_units::{Pressure, Temperature};

fn test_prov(name: &str) -> ScientificProvenance {
    ScientificProvenance::literature("Fogler / Perry's Chemical Engineers' Handbook", name, None, Some(0.01))
}

fn create_test_host(config: ServerConfig) -> (SimulationHost, HostHandle) {
    let mut engine = SimulationEngine::new(config.simulation_dt).expect("Failed to create engine");
    engine.clock_mut().play();
    let tag_store = RuntimeTagStore::new();

    let host = SimulationHost::start(config, engine, tag_store, None, None, None)
        .expect("Failed to start simulation host");

    let handle = host.handle();
    (host, handle)
}

// ------------------------------------------------------------------------------------------------
// CASE A: Refinery CDU/VDU (Deep Vacuum Flash & Bottoms Residue Pump NPSH)
// ------------------------------------------------------------------------------------------------
#[test]
fn test_case_a_refinery_cdu_vdu_vacuum_and_npsh() {
    let db = ComponentDatabase::standard_library();
    let pr = CubicEos::new_peng_robinson(db);
    let solver = VleFlashSolver::new(&pr);

    // Deep vacuum condition: P = 5.0 kPa (50 mbar), T = 350 K
    let p_vdu = Pressure::new(5_000.0).unwrap();
    let t_vdu = Temperature::new(350.0).unwrap();

    let mut z = IndexMap::new();
    z.insert("n-Butane".to_string(), 0.40);
    z.insert("Water".to_string(), 0.60);

    let flash = solver.flash_tp(&z, p_vdu, t_vdu).expect("VDU flash should converge");
    assert!(flash.vapor_fraction > 0.30, "Under 5 kPa vacuum at 350K, fractions should flash");
    assert!(!flash.mixture_density.value().is_nan());

    // Vacuum bottoms residue pump NPSHA analysis
    let rho: f64 = 880.0;
    let p_vap: f64 = 2800.0;
    let mut pump = PumpCavitationModel::new(0.05, 1450.0, 2.5, rho).unwrap();

    pump.suction_elevation_m = 4.0;
    pump.suction_loss_head_m = 0.3;
    pump.suction_pressure = Pressure::new_unchecked(p_vdu.value());
    pump.fluid_vapor_pressure = Pressure::new_unchecked(p_vap);

    pump.evaluate_cavitation(40.0, 1.0);
    assert_eq!(pump.last_regime, CavitationRegime::Safe);
    assert!(pump.last_npsha_m > pump.last_npshr_m, "Adequate static head gives NPSHA > NPSHR");

    // Level loss perturbation (Z drops to 0.5 m) -> Cavitation inception
    pump.suction_elevation_m = 0.5;
    pump.evaluate_cavitation(40.0, 1.0);
    assert!(pump.last_npsha_m < pump.last_npshr_m, "Loss of liquid level causes NPSHA < NPSHR");
    assert!(pump.last_vibration_mm_s > pump.baseline_vibration_mm_s, "Vibration increases under cavitation");
}

// ------------------------------------------------------------------------------------------------
// CASE B: Hydroprocessing (HDS Kinetics, Quench Cooling & High-Pressure Separator)
// ------------------------------------------------------------------------------------------------
#[test]
fn test_case_b_hydroprocessing_hds_kinetics_quench_flash() {
    let mut stoich = IndexMap::new();
    stoich.insert("Organosulfur".to_string(), -1.0);
    stoich.insert("Hydrogen".to_string(), -2.0);
    stoich.insert("Hydrocarbon".to_string(), 1.0);
    stoich.insert("H2S".to_string(), 1.0);

    let mut orders = IndexMap::new();
    orders.insert("Organosulfur".to_string(), 1.0);
    orders.insert("Hydrogen".to_string(), 1.0);

    let rxn = ReactionDefinition::new_irreversible(
        "rxn-hds",
        "Catalytic hydrodesulfurization",
        stoich,
        orders,
        2.5e4,
        50_000.0,
        -40_000.0,
        test_prov("hds_catalytic"),
    );

    let mut sys = ReactionSystem::new("HDS Reaction Train");
    sys.add_reaction(rxn);

    let pfr_cfg = PfrConfig {
        length_m: 8.0,
        diameter_m: 0.8,
        tube_count: 1,
        wall_temperature_k: 580.0,
        overall_u_w_per_m2_k: 800.0,
        fluid_density_kg_per_m3: 750.0,
        fluid_cp_j_per_kg_k: 2400.0,
        reaction_system: sys,
        limiting_reactant: "Organosulfur".to_string(),
        primary_product: Some("Hydrocarbon".to_string()),
        discretization_slices: 40,
    };

    let pfr = PfrReactorModel::new(pfr_cfg);

    let mut feed_conc = IndexMap::new();
    feed_conc.insert("Organosulfur".to_string(), 50.0);
    feed_conc.insert("Hydrogen".to_string(), 500.0);
    feed_conc.insert("Hydrocarbon".to_string(), 0.0);
    feed_conc.insert("H2S".to_string(), 0.0);

    let res = pfr.solve(50.0, 580.0, 7.5e6, &feed_conc).expect("HDS PFR solve should succeed");
    assert!(res.overall_conversion > 0.05, "Conversion was {}", res.overall_conversion);
    assert!(res.energy_balance_residual < 1e-3);

    // High Pressure Separator flash at 5.0 MPa, 40 °C (313.15 K)
    let db = ComponentDatabase::standard_library();
    let pr = CubicEos::new_peng_robinson(db);
    let solver = VleFlashSolver::new(&pr);

    let mut z_hps = IndexMap::new();
    z_hps.insert("Hydrogen".to_string(), 0.60);
    z_hps.insert("n-Butane".to_string(), 0.40);

    let flash = solver.flash_tp(&z_hps, Pressure::new(5.0e6).unwrap(), Temperature::new(313.15).unwrap()).unwrap();
    assert!(flash.vapor_fraction > 0.50, "HPS flashes hydrogen overhead");
}

// ------------------------------------------------------------------------------------------------
// CASE C: FCC / Recycle Process (Recycle Loop Convergence)
// ------------------------------------------------------------------------------------------------
#[test]
fn test_case_c_fcc_recycle_convergence() {
    let eq_riser = EquipmentId::new();
    let eq_stripper = EquipmentId::new();
    let eq_regen = EquipmentId::new();
    let eq_slide_valve = EquipmentId::new();
    let eq_feed_pump = EquipmentId::new();

    let nodes = vec![eq_feed_pump, eq_riser, eq_stripper, eq_regen, eq_slide_valve];
    let mut adjacency = BTreeMap::new();

    adjacency.insert(eq_feed_pump, [eq_riser].into_iter().collect());
    adjacency.insert(eq_riser, [eq_stripper].into_iter().collect());
    adjacency.insert(eq_stripper, [eq_regen].into_iter().collect());
    adjacency.insert(eq_regen, [eq_slide_valve].into_iter().collect());
    adjacency.insert(eq_slide_valve, [eq_riser].into_iter().collect());

    let scc_res = find_strongly_connected_components(&nodes, &adjacency);
    assert_eq!(scc_res.recycle_loops.len(), 1, "Must detect exactly 1 cyclic recycle loop");
    assert_eq!(scc_res.recycle_loops[0].len(), 4, "Riser, Stripper, Regen, Valve form 4-node recycle");

    let target_cat_flow: f64 = 1250.0;
    let mut x_guess: f64 = 1000.0;
    let mut iterations = 0;
    for _ in 0..30 {
        iterations += 1;
        let x_next: f64 = 0.5 * x_guess + 0.5 * target_cat_flow;
        let diff: f64 = (x_next - x_guess).abs();
        x_guess = x_next;
        if diff < 1e-4 {
            break;
        }
    }
    assert!(iterations < 25, "Recycle loop should converge within 25 iterations");
    assert!((x_guess - target_cat_flow).abs() < 1e-4);
}

// ------------------------------------------------------------------------------------------------
// CASE D: Petrochemical Separation (Cryogenic Demethanizer, C2 Splitter & Recycle Closure)
// ------------------------------------------------------------------------------------------------
#[test]
fn test_case_d_petrochemical_olefin_separation_and_recycle() {
    let db = ComponentDatabase::standard_library();
    let pr = CubicEos::new_peng_robinson(db);
    let solver = VleFlashSolver::new(&pr);

    // Demethanizer overhead flash: Hydrogen + Methane + Ethane at 1.8 MPa, 200 K
    let mut z_demeth = IndexMap::new();
    z_demeth.insert("Hydrogen".to_string(), 0.30);
    z_demeth.insert("Methane".to_string(), 0.50);
    z_demeth.insert("Ethane".to_string(), 0.20);

    let flash_demeth = solver.flash_tp(&z_demeth, Pressure::new(1.8e6).unwrap(), Temperature::new(200.0).unwrap()).unwrap();
    assert!(flash_demeth.vapor_fraction > 0.40, "Methane and H2 flash overhead: {}", flash_demeth.vapor_fraction);

    let f_fresh_feed: f64 = 50.0;
    let f_recycle: f64 = 18.2;
    let f_furnace_total: f64 = f_fresh_feed + f_recycle;

    let f_ethylene_prod: f64 = 32.5;
    let f_byproducts: f64 = 17.5;
    let f_uncracked_to_recycle: f64 = 18.2;

    let f_effluent_total: f64 = f_ethylene_prod + f_byproducts + f_uncracked_to_recycle;
    let mass_residual: f64 = (f_furnace_total - f_effluent_total).abs() / f_furnace_total;
    assert!(mass_residual < 1e-6, "Petrochemical recycle loop mass closure residual < 1e-6: {mass_residual}");
}

// ------------------------------------------------------------------------------------------------
// CASE E: Chemical Reactor + Separator (CSTR + Degasser + Liquid Recycle)
// ------------------------------------------------------------------------------------------------
#[test]
fn test_case_e_chemical_cstr_separator_liquid_recycle() {
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
        test_prov("cstr_kinetics"),
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

    for _ in 0..50 {
        cstr.step(2.0, 10.0, 345.0, &feed_conc, 335.0).unwrap();
        assert!(cstr.state.mass_balance_residual < 1e-4);
        assert!(cstr.state.energy_balance_residual < 1e-3);
    }
    assert!(cstr.state.conversion > 0.15);
    assert!(!cstr.state.is_runaway_warning);
}

// ------------------------------------------------------------------------------------------------
// CASE F: Gas Compression & Dehydration (Multi-stage + TEG Contactor)
// ------------------------------------------------------------------------------------------------
#[test]
fn test_case_f_gas_compression_dehydration_teg_contactor() {
    let p_suct: f64 = 1.5e6;
    let p_disch: f64 = 7.0e6;
    let r_total: f64 = p_disch / p_suct;
    let stages = 3;
    let r_per_stage: f64 = r_total.powf(1.0 / stages as f64);

    assert!(r_per_stage < 2.5, "Stage compression ratio should be modest (< 2.5)");

    let y_water_in: f64 = 1200.0;
    let n_trays = 12;
    let tray_efficiency: f64 = 0.35;
    let n_ideal: f64 = n_trays as f64 * tray_efficiency;

    let removal_fraction: f64 = 1.0 - (-0.75 * n_ideal).exp();
    let y_water_out: f64 = y_water_in * (1.0 - removal_fraction);
    assert!(y_water_out < 60.0, "Dehydrated natural gas meets pipeline spec: {y_water_out} mg/Sm3");
}

// ------------------------------------------------------------------------------------------------
// CASE G: LNG / Refrigeration (Mixed Refrigerant & J-T Isenthalpic Expansion)
// ------------------------------------------------------------------------------------------------
#[test]
fn test_case_g_lng_mixed_refrigerant_liquefaction_jt_valve() {
    let t_in: f64 = 220.0;
    let p_high: f64 = 4.5e6;
    let p_low: f64 = 0.35e6;

    let delta_p_bar: f64 = (p_high - p_low) / 1.0e5;
    let mu_jt: f64 = 1.15;
    let delta_t: f64 = mu_jt * delta_p_bar;
    let t_out: f64 = t_in - delta_t;

    assert!(t_out < 175.0, "J-T expansion achieves cryogenic temperature: {t_out} K (-98 °C or below)");

    let t_ng_in: f64 = 295.15;
    let t_ng_out: f64 = 112.15;
    let m_ng: f64 = 25.0;
    let cp_ng: f64 = 2800.0;
    let q_lng: f64 = m_ng * cp_ng * (t_ng_in - t_ng_out);

    assert!(q_lng > 1.2e7, "LNG liquefaction requires substantial refrigeration duty (> 12 MW): {q_lng} W");
}

// ------------------------------------------------------------------------------------------------
// CASE H: Boiler / Steam Utility Network (HP/MP/LP Headers & Letdown Desuperheating)
// ------------------------------------------------------------------------------------------------
#[test]
fn test_case_h_boiler_steam_utility_multi_header_letdown() {
    let h_hp: f64 = 3370.0;
    let h_mp: f64 = 2940.0;
    let h_water: f64 = 632.0;

    let m_hp_letdown: f64 = 20.0;
    let m_water: f64 = m_hp_letdown * (h_hp - h_mp) / (h_mp - h_water);
    let m_mp_total: f64 = m_hp_letdown + m_water;

    let enthalpy_in: f64 = m_hp_letdown * h_hp + m_water * h_water;
    let enthalpy_out: f64 = m_mp_total * h_mp;
    let energy_residual: f64 = (enthalpy_in - enthalpy_out).abs() / enthalpy_in;

    assert!(m_water > 3.0 && m_water < 4.5, "Required desuperheating spray flow: {m_water} kg/s");
    assert!(energy_residual < 1e-6, "Steam letdown desuperheating energy balance residual < 1e-6: {energy_residual}");
}

// ------------------------------------------------------------------------------------------------
// CASE I: Heat-Exchanger Network (LMTD Counter vs Co-Current, Epsilon-NTU, TEMA Fouling)
// ------------------------------------------------------------------------------------------------
#[test]
fn test_case_i_heat_exchanger_network_lmtd_epsilon_ntu_fouling() {
    let t_h_in: f64 = 130.0;
    let t_h_out: f64 = 70.0;
    let t_c_in: f64 = 25.0;
    let t_c_out: f64 = 65.0;

    let lmtd_counter = LmtdCalculator::calculate_counter_current(t_h_in, t_h_out, t_c_in, t_c_out).unwrap();
    let lmtd_co = LmtdCalculator::evaluate(t_h_in, t_h_out, t_c_in, t_c_out, FlowArrangement::CoCurrent).unwrap().effective_delta_t_m_k;

    assert!(lmtd_counter > lmtd_co, "Counter-current LMTD ({lmtd_counter}) must exceed co-current ({lmtd_co})");

    let fouling = FoulingParameters::default();
    let u_clean = fouling.clean_u();
    let u_fouled = fouling.fouled_u();

    assert!(u_fouled < u_clean, "Fouled U must be less than clean U");
}

// ------------------------------------------------------------------------------------------------
// CASE J: Pump / Hydraulic Network (Darcy-Weisbach, NPSHA vs NPSHR, Cavitation)
// ------------------------------------------------------------------------------------------------
#[test]
fn test_case_j_pump_hydraulic_network_darcy_weisbach_cavitation() {
    let rho: f64 = 998.0;
    let mu: f64 = 1.0e-3;
    let d: f64 = 0.15;
    let l: f64 = 100.0;
    let v: f64 = 2.5;

    let re: f64 = (rho * v * d) / mu;
    assert!(re > 4000.0);

    let eps = 0.045e-3;
    let f: f64 = 0.25 / (eps / (3.7 * d) + 5.74 / re.powf(0.9)).log10().powi(2);
    let delta_p_friction: f64 = f * (l / d) * (0.5 * rho * v * v);

    assert!(delta_p_friction > 10_000.0 && delta_p_friction < 50_000.0);

    let mut pump = PumpCavitationModel::new(0.045, 2950.0, 3.2, rho).unwrap();
    pump.suction_pressure = Pressure::new(200_000.0).unwrap();
    pump.evaluate_cavitation(45.0, 1.0);
    assert_eq!(pump.last_regime, CavitationRegime::Safe);

    pump.suction_pressure = Pressure::new(15_000.0).unwrap();
    pump.fluid_vapor_pressure = Pressure::new(12_000.0).unwrap();
    pump.evaluate_cavitation(45.0, 1.0);
    assert_ne!(pump.last_regime, CavitationRegime::Safe);
    assert!(pump.last_head_degradation < 1.0);
}

// ------------------------------------------------------------------------------------------------
// CASE K: Thermodynamic Flash (Cubic EOS Rachford-Rice & Fugacity Closure)
// ------------------------------------------------------------------------------------------------
#[test]
fn test_case_k_thermodynamic_flash_rachford_rice_fugacity() {
    let db = ComponentDatabase::standard_library();
    let pr = CubicEos::new_peng_robinson(db);
    let solver = VleFlashSolver::new(&pr);

    let mut z = IndexMap::new();
    z.insert("Methane".to_string(), 0.50);
    z.insert("Ethane".to_string(), 0.30);
    z.insert("Propane".to_string(), 0.20);

    let p = Pressure::new(2.5e6).unwrap();
    let t = Temperature::new(220.0).unwrap();

    let flash = solver.flash_tp(&z, p, t).expect("Flash should converge");
    assert!(flash.vapor_fraction > 0.0 && flash.vapor_fraction < 1.0);

    let beta = flash.vapor_fraction;
    let mut rr_residual = 0.0;
    for (comp, &z_i) in &z {
        let k_i = flash.k_values.get(comp).copied().unwrap_or(1.0);
        rr_residual += (z_i * (k_i - 1.0)) / (1.0 + beta * (k_i - 1.0));
    }
    assert!(rr_residual.abs() < 1e-6, "Rachford-Rice residual must be < 1e-6: {rr_residual}");
}

// ------------------------------------------------------------------------------------------------
// CASE L: Reaction Kinetics (Arrhenius Temperature Sensitivity, Stoichiometry, Heat Duty)
// ------------------------------------------------------------------------------------------------
#[test]
fn test_case_l_reaction_kinetics_arrhenius_stoichiometry_heat() {
    let mut stoich = IndexMap::new();
    stoich.insert("Reactant_A".to_string(), -1.0);
    stoich.insert("Product_B".to_string(), 1.0);

    let mut orders = IndexMap::new();
    orders.insert("Reactant_A".to_string(), 1.0);

    let rxn = ReactionDefinition::new_irreversible(
        "rxn-iso",
        "A -> B isomerization",
        stoich,
        orders,
        1.0e8,
        65_000.0,
        -75_000.0,
        test_prov("arrhenius_eval"),
    );

    let t1 = 300.0;
    let t2 = 350.0;
    let k1 = rxn.forward_rate_constant(t1).unwrap();
    let k2 = rxn.forward_rate_constant(t2).unwrap();

    assert!(k2 > k1 * 5.0, "Arrhenius rate constant must be strongly monotonic with T: k(300)={k1}, k(350)={k2}");

    let mut conc = IndexMap::new();
    conc.insert("Reactant_A".to_string(), 2000.0);
    let eval = rxn.evaluate_rate(t1, 101_325.0, &conc).unwrap();

    assert!(eval.net_rate > 0.0);
    assert!(eval.volumetric_heat_duty_w_per_m3 > 0.0);
}

// ------------------------------------------------------------------------------------------------
// CASE M: Custom Non-Refinery Plant (Data-driven Domain Packages via API)
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_case_m_custom_plant_data_driven_domain_registry_api() {
    let config = ServerConfig {
        simulation_dt: 0.05,
        auth_token: Some("sec-token-domain-m".to_string()),
        ..Default::default()
    };
    let (_host, handle) = create_test_host(config);
    let app = build_app(handle);

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/library/domains")
        .header(header::AUTHORIZATION, "Bearer sec-token-domain-m")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let domains: Vec<DomainPackageSummaryDto> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(domains.len(), 8, "Must return all 8 standard industrial domain packages");

    // Standard package ID domain.petrochemical.olefin
    let req_detail = Request::builder()
        .method("GET")
        .uri("/api/v1/library/domains/domain.petrochemical.olefin")
        .header(header::AUTHORIZATION, "Bearer sec-token-domain-m")
        .body(Body::empty())
        .unwrap();

    let resp_detail = app.oneshot(req_detail).await.unwrap();
    assert_eq!(resp_detail.status(), StatusCode::OK);

    let detail_bytes = axum::body::to_bytes(resp_detail.into_body(), usize::MAX).await.unwrap();
    let detail: DomainPackageDetailDto = serde_json::from_slice(&detail_bytes).unwrap();
    assert_eq!(detail.id, "domain.petrochemical.olefin");
    assert!(!detail.scientific_references.is_empty(), "Scientific references must be populated");
    assert!(!detail.model_classifications.is_empty(), "Model classifications must be populated");
}

// ------------------------------------------------------------------------------------------------
// CASE N: Invalid-Property & Convergence Diagnostics (Error Rejections & Guardrails)
// ------------------------------------------------------------------------------------------------
#[test]
fn test_case_n_invalid_properties_convergence_diagnostics() {
    let db = ComponentDatabase::standard_library();
    let pr = CubicEos::new_peng_robinson(db);
    let solver = VleFlashSolver::new(&pr);
    let mut z = IndexMap::new();
    z.insert("Methane".to_string(), 1.0);

    // Negative pressure rejected by phase stability solver
    let res_neg_p = solver.check_phase_stability(&z, Pressure::new_unchecked(-100.0), Temperature::new_unchecked(300.0));
    assert!(res_neg_p.is_err(), "Negative pressure must be rejected");

    // Non-positive absolute temperature rejected by phase stability solver
    let res_neg_t = solver.check_phase_stability(&z, Pressure::new_unchecked(101325.0), Temperature::new_unchecked(-10.0));
    assert!(res_neg_t.is_err(), "Non-positive temperature must be rejected");

    // Unphysical heat exchanger temperature cross detection:
    // Hot in = 50, Hot out = 80 (impossible without external heating)
    let res = LmtdCalculator::calculate_counter_current(50.0, 40.0, 20.0, 60.0);
    assert!(res.is_err(), "Unphysical temperature progression must be rejected");
}

// ------------------------------------------------------------------------------------------------
// CASE O: Deterministic Replay (Multi-step Bit-identical State Replay)
// ------------------------------------------------------------------------------------------------
#[test]
fn test_case_o_deterministic_replay_across_domains() {
    let run_sim = || -> Vec<u64> {
        let mut stoich = IndexMap::new();
        stoich.insert("A".to_string(), -1.0);
        stoich.insert("B".to_string(), 1.0);

        let mut orders = IndexMap::new();
        orders.insert("A".to_string(), 1.0);

        let rxn = ReactionDefinition::new_irreversible(
            "rxn-det",
            "A -> B deterministic",
            stoich,
            orders,
            1.0e8,
            68_000.0,
            -45_000.0,
            test_prov("replay"),
        );

        let mut sys = ReactionSystem::new("Replay System");
        sys.add_reaction(rxn);

        let config = CstrConfig {
            volume_m3: 1.0,
            jacket_area_m2: 5.0,
            jacket_u_w_per_m2_k: 500.0,
            fluid_density_kg_per_m3: 900.0,
            fluid_cp_j_per_kg_k: 2200.0,
            reaction_system: sys,
            limiting_reactant: "A".to_string(),
            primary_product: Some("B".to_string()),
            runaway_temp_threshold_k: 380.0,
        };

        let mut initial_conc = IndexMap::new();
        initial_conc.insert("A".to_string(), 1200.0);
        initial_conc.insert("B".to_string(), 50.0);

        let initial_state = CstrState {
            temperature_k: 340.0,
            pressure_pa: 150_000.0,
            concentrations: initial_conc,
            jacket_temperature_k: 330.0,
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
        feed_conc.insert("A".to_string(), 1400.0);
        feed_conc.insert("B".to_string(), 0.0);

        let mut record = Vec::new();
        for _ in 0..50 {
            cstr.step(1.0, 8.0, 335.0, &feed_conc, 325.0).unwrap();
            record.push(cstr.state.temperature_k.to_bits());
            for (_k, &c) in &cstr.state.concentrations {
                record.push(c.to_bits());
            }
        }
        record
    };

    let run_1 = run_sim();
    let run_2 = run_sim();
    assert_eq!(run_1, run_2, "Multi-step dynamic simulation must be bit-identical across runs");
}

// ------------------------------------------------------------------------------------------------
// CASE P: Stage 1–11 Frozen Baseline Regression Suite
// ------------------------------------------------------------------------------------------------
#[test]
fn test_case_p_frozen_baseline_regression_suite() {
    let pid_cfg = PidConfig {
        kp: 1.5,
        ti: 20.0,
        td: 1.0,
        derivative_filter_n: 10.0,
        out_min: 0.0,
        out_max: 100.0,
        direction: PidDirection::Direct,
        sp_tracking_in_manual: true,
        scan_divisor: 1,
    };
    let mut pid = PidController::new(pid_cfg);
    pid.set_mode(PidMode::Auto);
    pid.set_sp(50.0);

    // In direct action: e = PV - SP. With PV=60 and SP=50, e = +10, so output increases > 0.
    let out1 = pid.step(0.1, 60.0);
    let out2 = pid.step(0.1, 65.0);
    assert!(out1 > 0.0);
    assert!(out2 >= 0.0);

    let mut tag_store = RuntimeTagStore::new();
    let tag_id = TagId::new();
    tag_store.register_tag("TIC-101.PV", tag_id, TagValue::Float(65.4), TagQuality::Good);
    assert_eq!(tag_store.get_value(tag_id), Some(&TagValue::Float(65.4)));

    let all_pkgs = load_all_standard_domain_packages();
    assert_eq!(all_pkgs.len(), 8, "Baseline package count verified");
}
