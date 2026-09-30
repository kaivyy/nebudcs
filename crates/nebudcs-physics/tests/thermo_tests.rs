//! Comprehensive test suite for Phase 12 thermodynamic models:
//! - Component database and binary interaction parameters
//! - Generic cubic equations of state (Peng-Robinson and SRK)
//! - Analytical derivatives and departure functions (residual enthalpy & entropy)
//! - Multicomponent VLE flash solver (Rachford-Rice & successive substitution)
//! - Pluggable `CubicPropertyPackage` implementing `PropertyPackage`

use indexmap::IndexMap;
use nebudcs_physics::properties::PropertyPackage;
use nebudcs_physics::state::{ComponentComposition, Phase};
use nebudcs_physics::thermo::{
    BinaryInteractionMatrix, ComponentDatabase, CoolPropAdapter, CubicEos, CubicPropertyPackage,
    ExternalPropertyPackage, PhaseTarget, VleFlashSolver,
};
use nebudcs_units::{Pressure, Temperature};

#[test]
fn test_component_database_standard_library_lookup() {
    let db = ComponentDatabase::standard_library();

    let ch4 = db.get("Methane").expect("Methane exists");
    assert_eq!(ch4.formula, "CH4");
    assert!((ch4.critical_temperature - 190.56).abs() < 1e-2);
    assert!((ch4.critical_pressure - 4.5992e6).abs() < 1e2);
    assert!((ch4.molecular_weight - 0.016043).abs() < 1e-5);

    // Formula lookup works
    let c2h6 = db.get("C2H6").expect("C2H6 formula lookup");
    assert_eq!(c2h6.name, "Ethane");

    let h2o = db.get("Water").expect("Water exists");
    assert!((h2o.critical_temperature - 647.10).abs() < 1e-2);
}

#[test]
fn test_binary_interaction_matrix_symmetry_and_defaults() {
    let mut matrix = BinaryInteractionMatrix::new();

    // Default missing pair is 0.0
    assert_eq!(matrix.get("Methane", "Propane"), 0.0);
    assert_eq!(matrix.get("Propane", "Methane"), 0.0);

    // Diagonal is 0.0
    assert_eq!(matrix.get("Methane", "Methane"), 0.0);

    // Set parameter
    matrix.set("Methane", "Propane", 0.008).unwrap();
    assert_eq!(matrix.get("Methane", "Propane"), 0.008);
    assert_eq!(matrix.get("Propane", "Methane"), 0.008); // Symmetric!

    // Reject non-finite
    assert!(matrix.set("Methane", "Propane", f64::NAN).is_err());
}

#[test]
fn test_cubic_eos_constants_and_pure_parameters() {
    let db = ComponentDatabase::standard_library();
    let pr = CubicEos::new_peng_robinson(db.clone());
    let srk = CubicEos::new_srk(db);

    let ch4 = pr.database().get("Methane").unwrap();

    let pr_params = pr.pure_parameters(ch4, 300.0).unwrap();
    assert!(pr_params.a > 0.0);
    assert!(pr_params.b > 0.0);
    assert!(pr_params.da_dt < 0.0); // da/dT is negative above absolute zero

    let srk_params = srk.pure_parameters(ch4, 300.0).unwrap();
    assert!(srk_params.a > 0.0);
    assert!(srk_params.b > 0.0);
    assert!(srk_params.da_dt < 0.0);

    // Verify constants
    let (u_pr, w_pr, omega_a_pr, omega_b_pr) = pr.constants();
    assert_eq!(u_pr, 2.0);
    assert_eq!(w_pr, -1.0);
    assert!((omega_a_pr - 0.45724).abs() < 1e-4);
    assert!((omega_b_pr - 0.07780).abs() < 1e-4);

    let (u_srk, w_srk, omega_a_srk, omega_b_srk) = srk.constants();
    assert_eq!(u_srk, 1.0);
    assert_eq!(w_srk, 0.0);
    assert!((omega_a_srk - 0.42748).abs() < 1e-4);
    assert!((omega_b_srk - 0.08664).abs() < 1e-4);
}

#[test]
fn test_analytical_vs_finite_difference_da_dt() {
    let db = ComponentDatabase::standard_library();
    let pr = CubicEos::new_peng_robinson(db);
    let comp = pr.database().get("Propane").unwrap();

    let t = 300.0;
    let delta = 1e-4;

    let params = pr.pure_parameters(comp, t).unwrap();
    let da_dt_analytical = params.da_dt;

    let params_plus = pr.pure_parameters(comp, t + delta).unwrap();
    let params_minus = pr.pure_parameters(comp, t - delta).unwrap();
    let da_dt_numerical = (params_plus.a - params_minus.a) / (2.0 * delta);

    let rel_err = (da_dt_analytical - da_dt_numerical).abs() / da_dt_analytical.abs();
    assert!(
        rel_err < 1e-6,
        "analytical da/dt {da_dt_analytical} does not match numerical {da_dt_numerical} (rel_err: {rel_err})"
    );
}

#[test]
fn test_cubic_root_solving_and_z_factor() {
    let db = ComponentDatabase::standard_library();
    let pr = CubicEos::new_peng_robinson(db);

    let mut comp = IndexMap::new();
    comp.insert("Methane".to_string(), 1.0);

    // Methane at atmospheric pressure and ambient temp: ideal gas behavior (Z ~ 0.998)
    let state = pr
        .evaluate_phase_state(&comp, 101_325.0, 300.0, PhaseTarget::Vapor)
        .unwrap();
    assert!((state.compressibility_factor - 0.998).abs() < 0.005);
    assert!(state.mass_density > 0.6 && state.mass_density < 0.7);

    // High pressure methane at 300 K, 10 MPa (100 bar): Z ~ 0.86 (well-known NIST value)
    let state_hp = pr
        .evaluate_phase_state(&comp, 10.0e6, 300.0, PhaseTarget::Vapor)
        .unwrap();
    assert!(
        (state_hp.compressibility_factor - 0.86).abs() < 0.04,
        "Z={}",
        state_hp.compressibility_factor
    );
}

#[test]
fn test_enthalpy_departure_temperature_derivative_consistency() {
    let db = ComponentDatabase::standard_library();
    let pr = CubicEos::new_peng_robinson(db);

    let mut comp = IndexMap::new();
    comp.insert("Methane".to_string(), 1.0);

    let p = 2.0e6; // 2 MPa
    let t = 300.0;
    let delta_t = 1e-3;

    let s_plus = pr
        .evaluate_phase_state(&comp, p, t + delta_t, PhaseTarget::Vapor)
        .unwrap();
    let s_minus = pr
        .evaluate_phase_state(&comp, p, t - delta_t, PhaseTarget::Vapor)
        .unwrap();

    let cp_numerical = (s_plus.molar_enthalpy - s_minus.molar_enthalpy) / (2.0 * delta_t);

    // Ideal gas Cp at 300 K for methane is ~ 35.8 J/(mol*K)
    // At 2 MPa real gas Cp is slightly higher ~ 38-40 J/(mol*K)
    assert!(
        cp_numerical > 35.0 && cp_numerical < 45.0,
        "numerical Cp = {cp_numerical}"
    );
}

#[test]
fn test_ideal_gas_limiting_behavior_as_pressure_approaches_zero() {
    let db = ComponentDatabase::standard_library();
    let pr = CubicEos::new_peng_robinson(db);

    let mut comp = IndexMap::new();
    comp.insert("Methane".to_string(), 1.0);

    // At very low pressure (1 Pa), Z must approach 1.0, residual enthalpy must approach 0.0,
    // and fugacity coefficient must approach 1.0
    let state = pr
        .evaluate_phase_state(&comp, 1.0, 300.0, PhaseTarget::Vapor)
        .unwrap();

    assert!((state.compressibility_factor - 1.0).abs() < 1e-6);
    assert!(state.residual_molar_enthalpy.abs() < 1e-3);
    let phi = state.fugacity_coefficients.get("Methane").unwrap();
    assert!((phi - 1.0).abs() < 1e-6);
}

#[test]
fn test_rachford_rice_solution_bounds() {
    let db = ComponentDatabase::standard_library();
    let pr = CubicEos::new_peng_robinson(db);
    let solver = VleFlashSolver::new(&pr);

    let mut z = IndexMap::new();
    z.insert("Methane".to_string(), 0.5);
    z.insert("Propane".to_string(), 0.5);

    let mut k_values = IndexMap::new();
    k_values.insert("Methane".to_string(), 5.0); // Light component favors vapor
    k_values.insert("Propane".to_string(), 0.2); // Heavy component favors liquid

    let beta = solver.solve_rachford_rice(&z, &k_values).unwrap();
    assert!(beta > 0.0 && beta < 1.0);

    // Verify Rachford-Rice equation sum = 0
    let mut sum_rr = 0.0;
    for (name, &zi) in &z {
        let ki = k_values[name];
        sum_rr += zi * (ki - 1.0) / (1.0 + beta * (ki - 1.0));
    }
    assert!(sum_rr.abs() < 1e-10, "Rachford-Rice residual: {sum_rr}");
}

#[test]
fn test_multicomponent_vle_flash_equilibrium_and_material_balance() {
    let db = ComponentDatabase::standard_library();
    let pr = CubicEos::new_peng_robinson(db);
    let solver = VleFlashSolver::new(&pr);

    // Hydrocarbon blend: 40% Methane, 30% Ethane, 30% Propane
    let mut z = IndexMap::new();
    z.insert("Methane".to_string(), 0.40);
    z.insert("Ethane".to_string(), 0.30);
    z.insert("Propane".to_string(), 0.30);

    // At 200 K (-73 °C) and 1.5 MPa, this mixture exhibits two-phase equilibrium
    let p = Pressure::new(1.5e6).unwrap();
    let t = Temperature::new(200.0).unwrap();

    let flash = solver.flash_tp(&z, p, t).unwrap();

    assert_eq!(flash.phase, Phase::TwoPhase);
    assert!(flash.vapor_fraction > 0.0 && flash.vapor_fraction < 1.0);

    // 1. Verify mole fraction sum closure
    let sum_x: f64 = flash.liquid_composition.values().sum();
    let sum_y: f64 = flash.vapor_composition.values().sum();
    assert!(
        (sum_x - 1.0).abs() < 1e-7,
        "Liquid composition sum: {sum_x}"
    );
    assert!((sum_y - 1.0).abs() < 1e-7, "Vapor composition sum: {sum_y}");

    // 2. Verify species conservation: z_i = (1 - beta) * x_i + beta * y_i
    assert!(
        flash.material_balance_error < 1e-6,
        "material balance error: {}",
        flash.material_balance_error
    );

    // 3. Verify fugacity equilibrium: f_i^L ≈ f_i^V
    assert!(
        flash.equilibrium_error < 1e-3,
        "equilibrium error: {}",
        flash.equilibrium_error
    );

    // Density consistency: liquid denser than vapor
    assert!(flash.liquid_density.value() > flash.vapor_density.value());
    assert!(flash.liquid_density.value() > 400.0);
    assert!(flash.vapor_density.value() > 5.0 && flash.vapor_density.value() < 50.0);
}

#[test]
fn test_cubic_property_package_interface() {
    let pkg = CubicPropertyPackage::standard_peng_robinson();
    assert_eq!(pkg.name(), "Standard_PengRobinson");

    let mut comp_map = IndexMap::new();
    comp_map.insert("Methane".to_string(), 0.8);
    comp_map.insert("Ethane".to_string(), 0.2);
    let comp = ComponentComposition::new(comp_map).unwrap();

    let p = Pressure::new(2.0e6).unwrap();
    let t = Temperature::new(300.0).unwrap();

    let mw = pkg.molecular_weight(&comp).unwrap();
    assert!((mw - (0.8 * 0.016043 + 0.2 * 0.030070)).abs() < 1e-6);

    let rho = pkg.density(p, t, &comp).unwrap();
    assert!(rho.value() > 10.0 && rho.value() < 25.0);

    let h = pkg.specific_enthalpy(p, t, &comp).unwrap();
    assert!(h.value().is_finite());

    let eq = pkg.phase_equilibrium(p, t, &comp).unwrap();
    assert_eq!(eq.phase, Phase::Vapor);
    assert!((eq.vapor_fraction - 1.0).abs() < 1e-3);
}

#[test]
fn test_coolprop_adapter_water_properties() {
    let adapter = Box::new(CoolPropAdapter::default());
    let pkg = ExternalPropertyPackage::new("CoolProp_Water", "Water", 0.018015, adapter);

    let comp = ComponentComposition::pure("Water");

    // Subcooled water at 1 atm (101,325 Pa), 25 °C (298.15 K)
    let p = Pressure::new(101_325.0).unwrap();
    let t = Temperature::new(298.15).unwrap();

    let rho = pkg.density(p, t, &comp).unwrap();
    assert!(
        (rho.value() - 997.0).abs() < 10.0,
        "Water density at 25 °C: {}",
        rho.value()
    );

    let eq = pkg.phase_equilibrium(p, t, &comp).unwrap();
    assert_eq!(eq.phase, Phase::Liquid);
    assert_eq!(eq.vapor_fraction, 0.0);
}
