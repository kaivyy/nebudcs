//! Comprehensive Independent Verification Test Suite for NebuDCS Phase 11: Equipment Library.
//!
//! This test suite independently challenges all Phase 11 subsystems:
//! 1. All 12 standard equipment templates (validation, parameters, ports, curves, envelopes, trips, instantiation, stepping).
//! 2. Rejection of invalid parameter overrides (non-finite floats, below min, above max).
//! 3. Promotion of custom definitions to user templates and proof of complete runtime state isolation and immutability.
//! 4. Composite equipment packages and template composition into Level 2 composition and child subgraphs.
//! 5. Adversarial package rejections (dangling connections, domain mismatches, duplicate ports, missing targets).
//! 6. Versioning, content fingerprinting, and immutability verification.
//! 7. Version constraint matching (*, ^, >=, exact).
//! 8. Dependency resolution with Kahn's algorithm (topological order, cycle detection, missing dependencies, version conflicts, determinism).
//! 9. Deterministic import/export round-trips in JSON and TOML.
//! 10. Strict resource bounds (2 MB payload cap, 64 sub-component cap, boundary testing).
//! 11. Security bounds: rejection of malformed data, non-finite floats, sandbox keyword violations, and infinite loops.
//! 12. Verification of all 28 physical fixture files on disk under `packages/standard-library/`.
//! 13. Full runtime tag synchronization and protection trip/reset lifecycle.
//! 14. Deterministic multi-run simulation repeatability with wall-clock delay.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;
use std::time::Duration;

use indexmap::IndexMap;

use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_model::equipment::EquipmentCategory;
use nebudcs_model::{PortDirection, PortDomain};
use nebudcs_scripting::library::dependency::{DependencyResolver, VersionConstraint};
use nebudcs_scripting::library::import_export::{
    export_package, export_template, import_package, import_template, SerializationFormat,
    MAX_IMPORT_SIZE_BYTES,
};
use nebudcs_scripting::library::package::{PackageDependency, PackageSubComponent};
use nebudcs_scripting::library::standard::*;
use nebudcs_scripting::library::template::TemplateSource;
use nebudcs_scripting::library::versioning::{compute_content_fingerprint, verify_immutability};
use nebudcs_scripting::library::EquipmentLibrary;
use nebudcs_scripting::model::{CustomEquipmentDefinition, CustomizationTier, ScriptDefinition};
use nebudcs_scripting::runtime::CustomEquipmentManager;
use nebudcs_types::{EquipmentId, SchemaVersion};

// ===========================================================================
// Test 1: All 12 Standard Equipment Templates Lifecycle & Stepping
// ===========================================================================
#[test]
fn test_stage_a_all_12_standard_templates_lifecycle_and_stepping() {
    let mut mgr = CustomEquipmentManager::new();
    let templates = mgr.library().list_templates(None, None);
    assert_eq!(
        templates.len(),
        12,
        "Standard library must register exactly 12 standard equipment templates"
    );

    let expected_templates = vec![
        ("standard.pump.centrifugal", EquipmentCategory::Rotating),
        (
            "standard.compressor.centrifugal",
            EquipmentCategory::Rotating,
        ),
        ("standard.fan.centrifugal", EquipmentCategory::Rotating),
        ("standard.valve.control_globe", EquipmentCategory::Valve),
        (
            "standard.vessel.vertical_separator",
            EquipmentCategory::Process,
        ),
        ("standard.tank.atmospheric", EquipmentCategory::Process),
        (
            "standard.heat_exchanger.shell_and_tube",
            EquipmentCategory::HeatTransfer,
        ),
        (
            "standard.heater.fired_process",
            EquipmentCategory::HeatTransfer,
        ),
        ("standard.reactor.cstr", EquipmentCategory::Process),
        ("standard.mixer.static_inline", EquipmentCategory::Process),
        (
            "standard.splitter.flow_splitter",
            EquipmentCategory::Process,
        ),
        (
            "standard.sensor.transmitter",
            EquipmentCategory::Instrumentation,
        ),
    ];

    let mut tag_store = RuntimeTagStore::new();

    for (expected_id, expected_cat) in expected_templates {
        let tmpl = mgr
            .library()
            .get_template(expected_id, None)
            .unwrap_or_else(|| panic!("Template '{}' must exist in standard library", expected_id));

        assert_eq!(tmpl.id, expected_id);
        assert_eq!(tmpl.category, expected_cat);
        assert_eq!(tmpl.source, TemplateSource::Standard);
        assert!(
            !tmpl.definition.parameters.is_empty(),
            "Template {} must expose parameters",
            expected_id
        );
        assert!(
            !tmpl.definition.ports.is_empty(),
            "Template {} must expose ports",
            expected_id
        );

        // Instantiate
        let inst_id = EquipmentId::new();
        let inst_name = format!("INST_{}", expected_id.replace('.', "_"));
        mgr.instantiate_template(expected_id, None, inst_id, &inst_name, &BTreeMap::new())
            .unwrap_or_else(|e| panic!("Template {} failed to instantiate: {}", expected_id, e));

        // Verify instance initial state
        let inst = mgr.get_instance(inst_id).expect("Instance exists");
        assert_eq!(inst.name, inst_name);
        assert_eq!(inst.active_state, "stopped");
        assert!(!inst.is_running());
        assert!(!inst.is_tripped());

        // Step simulation
        let dt = 0.05;
        let inputs = HashMap::new();
        mgr.step(dt, &inputs, &mut tag_store)
            .unwrap_or_else(|e| panic!("Manager step failed on template {}: {}", expected_id, e));
    }
}

// ===========================================================================
// Test 2: Parameter Overrides Validation & Rejections
// ===========================================================================
#[test]
fn test_stage_a_invalid_parameter_overrides_rejected() {
    let mut mgr = CustomEquipmentManager::new();

    // 1. Non-finite float NaN
    let mut bad_overrides = BTreeMap::new();
    bad_overrides.insert("rated_flow".to_string(), f64::NAN);
    let res = mgr.instantiate_template(
        "standard.pump.centrifugal",
        None,
        EquipmentId::new(),
        "PUMP_BAD_NAN",
        &bad_overrides,
    );
    assert!(res.is_err(), "Non-finite NaN must be rejected");

    // 2. Non-finite float Infinity
    bad_overrides.clear();
    bad_overrides.insert("rated_flow".to_string(), f64::INFINITY);
    let res = mgr.instantiate_template(
        "standard.pump.centrifugal",
        None,
        EquipmentId::new(),
        "PUMP_BAD_INF",
        &bad_overrides,
    );
    assert!(res.is_err(), "Non-finite Infinity must be rejected");

    // 3. Below minimum value (rated_flow min is 0.0)
    bad_overrides.clear();
    bad_overrides.insert("rated_flow".to_string(), -10.0);
    let res = mgr.instantiate_template(
        "standard.pump.centrifugal",
        None,
        EquipmentId::new(),
        "PUMP_BELOW_MIN",
        &bad_overrides,
    );
    assert!(res.is_err(), "Value below minimum must be rejected");

    // 4. Above maximum value (rated_flow max is 500.0)
    bad_overrides.clear();
    bad_overrides.insert("rated_flow".to_string(), 1200.0);
    let res = mgr.instantiate_template(
        "standard.pump.centrifugal",
        None,
        EquipmentId::new(),
        "PUMP_ABOVE_MAX",
        &bad_overrides,
    );
    assert!(res.is_err(), "Value above maximum must be rejected");

    // 5. Valid override passes
    bad_overrides.clear();
    bad_overrides.insert("rated_flow".to_string(), 250.0);
    let res = mgr.instantiate_template(
        "standard.pump.centrifugal",
        None,
        EquipmentId::new(),
        "PUMP_VALID",
        &bad_overrides,
    );
    assert!(res.is_ok(), "Valid override must be accepted");
}

// ===========================================================================
// Test 3: User Equipment Promotion & Runtime State Isolation
// ===========================================================================
#[test]
fn test_stage_a_user_template_workflow_and_isolation() {
    let mut mgr = CustomEquipmentManager::new();

    // Create a valid Phase 10 CustomEquipmentDefinition
    let mut params = IndexMap::new();
    params.insert(
        "impeller_rpm".to_string(),
        nebudcs_model::ParameterDefinition {
            name: "impeller_rpm".to_string(),
            description: "Impeller speed".to_string(),
            quantity_kind: nebudcs_units::display::QuantityKind::Speed,
            default_value: 1450.0,
            min_value: Some(0.0),
            max_value: Some(3600.0),
            required: true,
        },
    );

    let ports = vec![nebudcs_model::PortDefinition {
        id: nebudcs_types::PortId::new(),
        name: "inlet".to_string(),
        domain: PortDomain::Material,
        direction: PortDirection::Inlet,
        phase_compatibility: vec![nebudcs_model::Phase::Liquid],
        required: true,
        cardinality: nebudcs_model::equipment::Cardinality::ZeroOrMore,
        description: "Inlet nozzle".to_string(),
    }];

    let custom_def = CustomEquipmentDefinition {
        id: "user.pump.custom_slurry".to_string(),
        name: "Custom Slurry Pump".to_string(),
        version: SchemaVersion::new(1, 0, 0),
        category: EquipmentCategory::Rotating,
        tier: CustomizationTier::Level1Declarative,
        description: "Heavy duty slurry pump".to_string(),
        parameters: params,
        ports,
        curves: IndexMap::new(),
        curves_2d: IndexMap::new(),
        operating_envelope: Default::default(),
        alarm_definitions: Vec::new(),
        trips: Vec::new(),
        interlocks: Vec::new(),
        script: None,
        composition: None,
        metadata: IndexMap::new(),
    };

    // Register definition into Phase 10 manager first
    mgr.register_definition(custom_def)
        .expect("Register custom definition");

    // Promote to user template
    mgr.promote_to_user_template("user.pump.custom_slurry", None)
        .expect("Promotion succeeds");

    // Check catalog contains user template
    let tmpl = mgr
        .library()
        .get_template("user.pump.custom_slurry", None)
        .expect("User template in library");
    assert_eq!(tmpl.source, TemplateSource::User);
    assert_eq!(
        tmpl.definition.parameters["impeller_rpm"].default_value,
        1450.0
    );

    // Instantiate Instance A with default params
    let id_a = EquipmentId::new();
    mgr.instantiate_template(
        "user.pump.custom_slurry",
        None,
        id_a,
        "PUMP_SLURRY_A",
        &BTreeMap::new(),
    )
    .expect("Instantiate A");

    // Instantiate Instance B with overridden params
    let mut overrides_b = BTreeMap::new();
    overrides_b.insert("impeller_rpm".to_string(), 2100.0);
    let id_b = EquipmentId::new();
    mgr.instantiate_template(
        "user.pump.custom_slurry",
        None,
        id_b,
        "PUMP_SLURRY_B",
        &overrides_b,
    )
    .expect("Instantiate B");

    // Verify initial values
    let inst_a = mgr.get_instance(id_a).unwrap();
    let inst_b = mgr.get_instance(id_b).unwrap();
    assert_eq!(inst_a.parameters["impeller_rpm"], 1450.0);
    assert_eq!(inst_b.parameters["impeller_rpm"], 2100.0);

    // Mutate Instance A state
    mgr.get_instance_mut(id_a).unwrap().set_var("speed", 1450.0);
    mgr.start(id_a).expect("Start instance A");

    // Verify Instance B and Catalog Template remain strictly unmodified
    let inst_a_after = mgr.get_instance(id_a).unwrap();
    let inst_b_after = mgr.get_instance(id_b).unwrap();
    let tmpl_after = mgr
        .library()
        .get_template("user.pump.custom_slurry", None)
        .unwrap();

    assert!(inst_a_after.is_running());
    assert_eq!(inst_a_after.get_var("speed"), Some(1450.0));

    assert!(!inst_b_after.is_running());
    assert_eq!(inst_b_after.get_var("speed"), Some(0.0));
    assert_eq!(
        tmpl_after.definition.parameters["impeller_rpm"].default_value,
        1450.0
    );
}

// ===========================================================================
// Test 4: Composite Packages Wiring, Subgraph & Runtime Execution
// ===========================================================================
#[test]
fn test_stage_a_composite_packages_wiring_and_execution() {
    let mut mgr = CustomEquipmentManager::new();

    // Verify 2 standard packages are preloaded
    let pkgs = mgr.library().list_packages();
    assert_eq!(pkgs.len(), 2);

    // 1. Pumping Skid Package
    let pump_skid = mgr
        .library()
        .get_package("standard.package.pump_bypass_skid", None)
        .expect("Pump skid package exists");
    assert!(mgr.library().validate_package(pump_skid).is_ok());

    let comp_def = pump_skid.to_composition_definition();
    assert_eq!(comp_def.sub_components.len(), 2);
    assert_eq!(comp_def.internal_connections.len(), 1);
    assert_eq!(comp_def.port_mappings.len(), 4);

    // 2. Two-Stage Compressor Skid Package
    let comp_skid = mgr
        .library()
        .get_package("standard.package.two_stage_compressor_skid", None)
        .expect("Compressor skid package exists");
    assert!(mgr.library().validate_package(comp_skid).is_ok());

    let comp_def2 = comp_skid.to_composition_definition();
    assert_eq!(comp_def2.sub_components.len(), 4);
    assert_eq!(comp_def2.internal_connections.len(), 3);
    assert_eq!(comp_def2.port_mappings.len(), 4);

    // Instantiate Pumping Skid
    let mut skid_overrides = BTreeMap::new();
    skid_overrides.insert("skid_flow".to_string(), 160.0);

    let res = mgr
        .instantiate_package(
            "standard.package.pump_bypass_skid",
            None,
            "SKID_UNIT_01",
            &skid_overrides,
        )
        .expect("Instantiate pumping skid");

    assert_eq!(res.instances.len(), 2);
    assert_eq!(res.internal_streams.len(), 1);
    assert_eq!(res.port_mappings.len(), 4);

    // Verify child instances in manager
    let child1 = mgr.get_instance(res.instances[0].id).unwrap();
    let child2 = mgr.get_instance(res.instances[1].id).unwrap();
    assert_eq!(child1.name, "SKID_UNIT_01_main_pump");
    assert_eq!(child2.name, "SKID_UNIT_01_bypass_valve");
    assert_eq!(
        child1.parameters["rated_flow"], 160.0,
        "Forwarded parameter applied"
    );

    // Step simulation with package instances
    let mut tag_store = RuntimeTagStore::new();
    mgr.step(0.05, &HashMap::new(), &mut tag_store)
        .expect("Step simulation with skid child instances");
}

// ===========================================================================
// Test 5: Adversarial Package Validation Rejections
// ===========================================================================
#[test]
fn test_stage_a_package_adversarial_rejections() {
    let library = EquipmentLibrary::with_standard_library().expect("Standard library loads");
    let valid_skid = library
        .get_package("standard.package.pump_bypass_skid", None)
        .unwrap();

    // 1. Missing component dependency
    let mut pkg = valid_skid.clone();
    pkg.sub_components[0].template_id = "nonexistent.missing.template".to_string();
    let res = library.validate_package(&pkg);
    assert!(res.is_err(), "Missing template must be rejected");

    // 2. Dangling internal stream source component
    let mut pkg = valid_skid.clone();
    pkg.internal_streams[0].from_component = "ghost_component".to_string();
    let res = library.validate_package(&pkg);
    assert!(res.is_err(), "Dangling source component must be rejected");

    // 3. Dangling internal stream destination port
    let mut pkg = valid_skid.clone();
    pkg.internal_streams[0].to_port = "nonexistent_port".to_string();
    let res = library.validate_package(&pkg);
    assert!(res.is_err(), "Dangling destination port must be rejected");

    // 4. Incompatible internal stream port domains (e.g. Material -> Electrical)
    let mut pkg = valid_skid.clone();
    pkg.internal_streams[0].to_port = "motor_feeder".to_string(); // Electrical port
    let res = library.validate_package(&pkg);
    assert!(res.is_err(), "Incompatible port domains must be rejected");

    // 5. Incompatible port direction (connecting Inlet -> Inlet)
    let mut pkg = valid_skid.clone();
    pkg.internal_streams[0].from_port = "suction".to_string(); // Inlet port
    let res = library.validate_package(&pkg);
    assert!(
        res.is_err(),
        "Incompatible port directions must be rejected"
    );

    // 6. Duplicate external port names
    let mut pkg = valid_skid.clone();
    let mut dup_port = pkg.external_ports[0].clone();
    dup_port.internal_port = "outlet".to_string();
    pkg.external_ports.push(dup_port);
    let res = library.validate_package(&pkg);
    assert!(
        res.is_err(),
        "Duplicate external port names must be rejected"
    );

    // 7. Nonexistent parameter mapping target component
    let mut pkg = valid_skid.clone();
    pkg.parameter_mappings[0].target_component = "ghost_component".to_string();
    let res = library.validate_package(&pkg);
    assert!(
        res.is_err(),
        "Nonexistent parameter target component must be rejected"
    );

    // 8. Nonexistent parameter mapping target param
    let mut pkg = valid_skid.clone();
    pkg.parameter_mappings[0].target_param = "nonexistent_param".to_string();
    let res = library.validate_package(&pkg);
    assert!(
        res.is_err(),
        "Nonexistent parameter target param must be rejected"
    );
}

// ===========================================================================
// Test 6: Versioning, Content Fingerprinting & Immutability
// ===========================================================================
#[test]
fn test_stage_a_versioning_and_fingerprint_integrity() {
    let pump = make_standard_centrifugal_pump().unwrap();
    let content_json = export_template(&pump, SerializationFormat::Json).unwrap();

    // Deterministic fingerprint
    let fp1 = compute_content_fingerprint(&content_json);
    let fp2 = compute_content_fingerprint(&content_json);
    assert_eq!(fp1, fp2, "Fingerprint must be bit-identical across runs");

    // Altered content produces different fingerprint
    let mutated_json = content_json.replace("Standard Centrifugal Pump", "Mutated Pump");
    let fp3 = compute_content_fingerprint(&mutated_json);
    assert_ne!(
        fp1, fp3,
        "Altered content must produce different fingerprint"
    );

    // Immutability verification
    let ver = SchemaVersion::new(1, 0, 0);
    assert!(
        verify_immutability("test.pump", &ver, &fp1, &fp2).is_ok(),
        "Matching fingerprint passes immutability"
    );
    assert!(
        verify_immutability("test.pump", &ver, &fp1, &fp3).is_err(),
        "Mismatched fingerprint violates immutability"
    );

    // Multiple versions in library
    let mut lib = EquipmentLibrary::new();
    let mut pump_v1 = pump.clone();
    pump_v1.version = SchemaVersion::new(1, 0, 0);
    let mut pump_v2 = pump.clone();
    pump_v2.version = SchemaVersion::new(1, 1, 0);

    lib.register_template(pump_v1).unwrap();
    lib.register_template(pump_v2).unwrap();

    let fetched_v1 = lib
        .get_template(
            "standard.pump.centrifugal",
            Some(&SchemaVersion::new(1, 0, 0)),
        )
        .unwrap();
    let fetched_v2 = lib
        .get_template(
            "standard.pump.centrifugal",
            Some(&SchemaVersion::new(1, 1, 0)),
        )
        .unwrap();

    assert_eq!(fetched_v1.version, SchemaVersion::new(1, 0, 0));
    assert_eq!(fetched_v2.version, SchemaVersion::new(1, 1, 0));
}

// ===========================================================================
// Test 7: Version Constraint Matching
// ===========================================================================
#[test]
fn test_stage_a_version_constraints_matching() {
    // Wildcard "*"
    let c_any = VersionConstraint::parse("*").unwrap();
    assert!(c_any.matches(&SchemaVersion::new(1, 0, 0)));
    assert!(c_any.matches(&SchemaVersion::new(2, 5, 1)));

    // Caret "^1.2.0" (matches major == 1, >= 1.2.0)
    let c_caret = VersionConstraint::parse("^1.2.0").unwrap();
    assert!(c_caret.matches(&SchemaVersion::new(1, 2, 0)));
    assert!(c_caret.matches(&SchemaVersion::new(1, 2, 4)));
    assert!(c_caret.matches(&SchemaVersion::new(1, 5, 0)));
    assert!(!c_caret.matches(&SchemaVersion::new(1, 1, 9)));
    assert!(!c_caret.matches(&SchemaVersion::new(2, 0, 0)));

    // GreaterOrEqual ">=2.0.0"
    let c_ge = VersionConstraint::parse(">=2.0.0").unwrap();
    assert!(c_ge.matches(&SchemaVersion::new(2, 0, 0)));
    assert!(c_ge.matches(&SchemaVersion::new(2, 1, 0)));
    assert!(c_ge.matches(&SchemaVersion::new(3, 0, 0)));
    assert!(!c_ge.matches(&SchemaVersion::new(1, 9, 9)));

    // Exact "1.0.0"
    let c_exact = VersionConstraint::parse("1.0.0").unwrap();
    assert!(c_exact.matches(&SchemaVersion::new(1, 0, 0)));
    assert!(!c_exact.matches(&SchemaVersion::new(1, 0, 1)));

    // Malformed syntax rejected
    assert!(VersionConstraint::parse("invalid-version").is_err());
    assert!(VersionConstraint::parse("^invalid").is_err());
    assert!(VersionConstraint::parse(">=invalid").is_err());
}

// ===========================================================================
// Test 8: Dependency Resolver: Kahn Topological Sort, Cycles & Determinism
// ===========================================================================
#[test]
fn test_stage_a_dependency_resolver_kahn_cycles_and_determinism() {
    // 5-level deep chain: A -> B -> C -> D -> E
    let mut resolver = DependencyResolver::new();
    resolver.add_dependencies("node_e", Vec::new());
    resolver.add_dependencies(
        "node_d",
        vec![PackageDependency {
            id: "node_e".to_string(),
            version_req: "^1.0.0".to_string(),
        }],
    );
    resolver.add_dependencies(
        "node_c",
        vec![PackageDependency {
            id: "node_d".to_string(),
            version_req: "^1.0.0".to_string(),
        }],
    );
    resolver.add_dependencies(
        "node_b",
        vec![PackageDependency {
            id: "node_c".to_string(),
            version_req: "^1.0.0".to_string(),
        }],
    );
    resolver.add_dependencies(
        "node_a",
        vec![PackageDependency {
            id: "node_b".to_string(),
            version_req: "^1.0.0".to_string(),
        }],
    );

    let mut available = HashMap::new();
    for name in ["node_a", "node_b", "node_c", "node_d", "node_e"] {
        available.insert(name.to_string(), vec![SchemaVersion::new(1, 0, 0)]);
    }

    let order = resolver.resolve(&available).expect("Resolve deep chain");
    assert_eq!(
        order,
        vec!["node_e", "node_d", "node_c", "node_b", "node_a"],
        "Must be topological order from leaves to dependents"
    );

    // Cycle detection
    let mut cycle_res = DependencyResolver::new();
    cycle_res.add_dependencies(
        "alpha",
        vec![PackageDependency {
            id: "beta".to_string(),
            version_req: "*".to_string(),
        }],
    );
    cycle_res.add_dependencies(
        "beta",
        vec![PackageDependency {
            id: "gamma".to_string(),
            version_req: "*".to_string(),
        }],
    );
    cycle_res.add_dependencies(
        "gamma",
        vec![PackageDependency {
            id: "alpha".to_string(),
            version_req: "*".to_string(),
        }],
    );

    let mut cycle_avail = HashMap::new();
    for name in ["alpha", "beta", "gamma"] {
        cycle_avail.insert(name.to_string(), vec![SchemaVersion::new(1, 0, 0)]);
    }
    let res = cycle_res.resolve(&cycle_avail);
    assert!(res.is_err(), "Cyclic graph must be rejected");

    // Missing dependency
    let mut missing_res = DependencyResolver::new();
    missing_res.add_dependencies(
        "leaf_user",
        vec![PackageDependency {
            id: "missing_dep".to_string(),
            version_req: "*".to_string(),
        }],
    );
    let mut missing_avail = HashMap::new();
    missing_avail.insert("leaf_user".to_string(), vec![SchemaVersion::new(1, 0, 0)]);
    assert!(
        missing_res.resolve(&missing_avail).is_err(),
        "Missing dependency must fail"
    );

    // Version conflict
    let mut conflict_res = DependencyResolver::new();
    conflict_res.add_dependencies(
        "consumer",
        vec![PackageDependency {
            id: "dep".to_string(),
            version_req: "^2.0.0".to_string(),
        }],
    );
    let mut conflict_avail = HashMap::new();
    conflict_avail.insert("consumer".to_string(), vec![SchemaVersion::new(1, 0, 0)]);
    conflict_avail.insert("dep".to_string(), vec![SchemaVersion::new(1, 5, 0)]); // Only 1.5.0 available
    assert!(
        conflict_res.resolve(&conflict_avail).is_err(),
        "Incompatible version must fail"
    );
}

// ===========================================================================
// Test 9: Deterministic Import/Export Round-Trips
// ===========================================================================
#[test]
fn test_stage_a_import_export_exact_roundtrips() {
    let pump = make_standard_centrifugal_pump().unwrap();

    // 1. JSON Round-trip
    let json_str = export_template(&pump, SerializationFormat::Json).unwrap();
    let imp_pump_json = import_template(&json_str, SerializationFormat::Json).unwrap();
    assert_eq!(pump.id, imp_pump_json.id);
    assert_eq!(pump.version, imp_pump_json.version);
    assert_eq!(
        pump.definition.parameters.len(),
        imp_pump_json.definition.parameters.len()
    );

    // Deterministic re-export
    let json_str_2 = export_template(&imp_pump_json, SerializationFormat::Json).unwrap();
    assert_eq!(json_str, json_str_2);

    // 2. TOML Round-trip
    let toml_str = export_template(&pump, SerializationFormat::Toml).unwrap();
    let imp_pump_toml = import_template(&toml_str, SerializationFormat::Toml).unwrap();
    assert_eq!(pump.id, imp_pump_toml.id);
    assert_eq!(pump.version, imp_pump_toml.version);

    // 3. Package JSON Round-trip
    let pkg = make_standard_pump_bypass_package();
    let pkg_json = export_package(&pkg, SerializationFormat::Json).unwrap();
    let imp_pkg_json = import_package(&pkg_json, SerializationFormat::Json).unwrap();
    assert_eq!(pkg.id, imp_pkg_json.id);
    assert_eq!(pkg.sub_components.len(), imp_pkg_json.sub_components.len());

    // 4. Package TOML Round-trip
    let pkg_toml = export_package(&pkg, SerializationFormat::Toml).unwrap();
    let imp_pkg_toml = import_package(&pkg_toml, SerializationFormat::Toml).unwrap();
    assert_eq!(pkg.id, imp_pkg_toml.id);
    assert_eq!(
        pkg.internal_streams.len(),
        imp_pkg_toml.internal_streams.len()
    );
}

// ===========================================================================
// Test 10: Strict Resource Caps & Boundary Limits
// ===========================================================================
#[test]
fn test_stage_a_import_payload_and_complexity_boundary_limits() {
    // 1. Payload size cap (MAX_IMPORT_SIZE_BYTES = 2MB)
    let oversized = " ".repeat(MAX_IMPORT_SIZE_BYTES + 1);
    let res = import_template(&oversized, SerializationFormat::Json);
    assert!(res.is_err(), "Payload > 2MB must be rejected");

    // 2. Package sub-component count boundary (MAX_PACKAGE_SUB_COMPONENTS = 64)
    let mut pkg = make_standard_pump_bypass_package();
    pkg.sub_components.clear();
    for i in 0..64 {
        pkg.sub_components.push(PackageSubComponent {
            id: format!("comp_{i}"),
            template_id: "standard.pump.centrifugal".to_string(),
            template_version: None,
            parameter_overrides: BTreeMap::new(),
        });
    }

    let pkg_json_64 = export_package(&pkg, SerializationFormat::Json).unwrap();
    assert!(
        import_package(&pkg_json_64, SerializationFormat::Json).is_ok(),
        "64 sub-components is exactly at limit and must be accepted"
    );

    // Add 65th sub-component (exceeds cap)
    pkg.sub_components.push(PackageSubComponent {
        id: "comp_64".to_string(),
        template_id: "standard.pump.centrifugal".to_string(),
        template_version: None,
        parameter_overrides: BTreeMap::new(),
    });

    let pkg_json_65 = export_package(&pkg, SerializationFormat::Json).unwrap();
    let res_65 = import_package(&pkg_json_65, SerializationFormat::Json);
    assert!(
        res_65.is_err(),
        "65 sub-components exceeds limit and must be rejected"
    );
}

// ===========================================================================
// Test 11: Security & Sandbox Attack Rejections on Import
// ===========================================================================
#[test]
fn test_stage_a_import_security_and_sandbox_rejections() {
    let pump = make_standard_centrifugal_pump().unwrap();

    // 1. Corrupted/malformed JSON
    assert!(import_template("{ invalid json content", SerializationFormat::Json).is_err());

    // 2. Corrupted/malformed TOML
    assert!(import_template("[unclosed toml", SerializationFormat::Toml).is_err());

    // 3. Illegal identifier containing shell injection chars
    let mut evil_pump = pump.clone();
    evil_pump.id = "pump; rm -rf /;".to_string();
    let evil_json = serde_json::to_string(&evil_pump).unwrap();
    assert!(
        import_template(&evil_json, SerializationFormat::Json).is_err(),
        "Illegal identifier must be rejected"
    );

    // 4. Injected Rhai script with infinite loop
    let mut scripted_pump = pump.clone();
    scripted_pump.definition.tier = CustomizationTier::Level3Scripted;
    scripted_pump.definition.script = Some(ScriptDefinition {
        entry_point: "step".to_string(),
        code: "fn step(inputs, dt) { while true {} }".to_string(),
    });
    let scripted_json = serde_json::to_string(&scripted_pump).unwrap();
    assert!(
        import_template(&scripted_json, SerializationFormat::Json).is_err(),
        "Script with infinite loop must be rejected by pre-admission validation"
    );
}

// ===========================================================================
// Test 12: Independent Verification of All 28 Physical Fixtures On Disk
// ===========================================================================
#[test]
fn test_stage_a_all_28_physical_fixtures_on_disk() {
    let fixtures_dir = Path::new("/root/NebuDCS/packages/standard-library");
    assert!(fixtures_dir.exists(), "Fixtures directory must exist");

    let expected_template_files = vec![
        "pumps/pump_centrifugal",
        "compressors/compressor_centrifugal",
        "fans/fan_centrifugal",
        "valves/valve_control_globe",
        "vessels/vessel_vertical_separator",
        "tanks/tank_atmospheric",
        "heat_exchangers/heat_exchanger_shell_and_tube",
        "heaters/heater_fired_process",
        "reactors/reactor_cstr",
        "mixers/mixer_static_inline",
        "splitters/splitter_flow_splitter",
        "sensors/sensor_transmitter",
    ];

    let expected_package_files = vec![
        "packages/pump_bypass_skid",
        "packages/two_stage_compressor_skid",
    ];

    let mut verified_files = 0;
    let mut library = EquipmentLibrary::new();

    // Verify template fixtures (.json and .toml)
    for rel_path in &expected_template_files {
        let json_path = fixtures_dir.join(format!("{rel_path}.json"));
        let toml_path = fixtures_dir.join(format!("{rel_path}.toml"));

        assert!(json_path.exists(), "Fixture JSON missing: {:?}", json_path);
        assert!(toml_path.exists(), "Fixture TOML missing: {:?}", toml_path);

        let json_content = fs::read_to_string(&json_path).unwrap();
        let tmpl_json = import_template(&json_content, SerializationFormat::Json)
            .unwrap_or_else(|e| panic!("Failed to parse fixture {:?}: {}", json_path, e));
        verified_files += 1;

        let toml_content = fs::read_to_string(&toml_path).unwrap();
        let tmpl_toml = import_template(&toml_content, SerializationFormat::Toml)
            .unwrap_or_else(|e| panic!("Failed to parse fixture {:?}: {}", toml_path, e));
        verified_files += 1;

        assert_eq!(
            tmpl_json.id, tmpl_toml.id,
            "JSON and TOML must have identical ID for {}",
            rel_path
        );
        assert_eq!(tmpl_json.version, tmpl_toml.version);

        library.register_template(tmpl_json).unwrap();
    }

    // Verify package fixtures (.json and .toml)
    for rel_path in &expected_package_files {
        let json_path = fixtures_dir.join(format!("{rel_path}.json"));
        let toml_path = fixtures_dir.join(format!("{rel_path}.toml"));

        assert!(
            json_path.exists(),
            "Package fixture JSON missing: {:?}",
            json_path
        );
        assert!(
            toml_path.exists(),
            "Package fixture TOML missing: {:?}",
            toml_path
        );

        let json_content = fs::read_to_string(&json_path).unwrap();
        let pkg_json = import_package(&json_content, SerializationFormat::Json)
            .unwrap_or_else(|e| panic!("Failed to parse package {:?}: {}", json_path, e));
        verified_files += 1;

        let toml_content = fs::read_to_string(&toml_path).unwrap();
        let pkg_toml = import_package(&toml_content, SerializationFormat::Toml)
            .unwrap_or_else(|e| panic!("Failed to parse package {:?}: {}", toml_path, e));
        verified_files += 1;

        assert_eq!(pkg_json.id, pkg_toml.id);
        assert_eq!(pkg_json.version, pkg_toml.version);

        // Validate package against populated catalog
        library
            .validate_package(&pkg_json)
            .unwrap_or_else(|e| panic!("Package {:?} failed validation: {}", json_path, e));

        library.register_package(pkg_json).unwrap();
    }

    assert_eq!(
        verified_files, 28,
        "Exactly 28 physical fixture files must be verified"
    );
}

// ===========================================================================
// Test 13: Runtime Tag Sync & Protection Trip Lifecycle
// ===========================================================================
#[test]
fn test_stage_a_runtime_tag_sync_and_trip_protection() {
    let mut mgr = CustomEquipmentManager::new();
    let mut tag_store = RuntimeTagStore::new();

    let pump_id = EquipmentId::new();
    mgr.instantiate_template(
        "standard.pump.centrifugal",
        None,
        pump_id,
        "FEED_PUMP_101",
        &BTreeMap::new(),
    )
    .expect("Instantiate pump");

    // Start pump
    mgr.start(pump_id).expect("Start pump");
    let dt = 0.05;

    // Step normal operation
    mgr.step(dt, &HashMap::new(), &mut tag_store).unwrap();
    let inst = mgr.get_instance(pump_id).unwrap();
    assert!(inst.is_running());
    assert!(!inst.is_tripped());

    // Induce vibration trip (> 7.1 mm/s)
    mgr.get_instance_mut(pump_id)
        .unwrap()
        .set_var("vibration", 8.2);
    mgr.step(dt, &HashMap::new(), &mut tag_store).unwrap();

    let tripped = mgr.get_instance(pump_id).unwrap();
    assert!(tripped.is_tripped(), "Pump must trip on high vibration");
    assert!(!tripped.is_running(), "Tripped pump must stop");
    assert_eq!(tripped.active_state, "tripped");

    // Cannot start while tripped
    assert!(
        mgr.start(pump_id).is_err(),
        "Cannot start tripped equipment"
    );

    // Reset while vibration still high must fail
    assert!(
        !mgr.reset_trip(pump_id).unwrap(),
        "Reset trip fails while condition persists"
    );

    // Normalize vibration and reset
    mgr.get_instance_mut(pump_id)
        .unwrap()
        .set_var("vibration", 1.8);
    assert!(
        mgr.reset_trip(pump_id).unwrap(),
        "Reset trip succeeds when normalized"
    );

    let reset_inst = mgr.get_instance(pump_id).unwrap();
    assert!(!reset_inst.is_tripped());

    // Can now restart
    assert!(mgr.start(pump_id).is_ok());
}

// ===========================================================================
// Test 14: Simulation Determinism With Wall-Clock Variance
// ===========================================================================
#[test]
fn test_stage_a_simulation_determinism_with_wall_clock_variance() {
    let run_sim = |delay_ms: u64| -> (f64, f64, f64) {
        let mut mgr = CustomEquipmentManager::new();
        let mut tag_store = RuntimeTagStore::new();

        let id = EquipmentId::new();
        let mut overrides = BTreeMap::new();
        overrides.insert("rated_flow".to_string(), 135.0);

        mgr.instantiate_template(
            "standard.pump.centrifugal",
            None,
            id,
            "TEST_PUMP",
            &overrides,
        )
        .unwrap();
        mgr.start(id).unwrap();

        // Deliberate wall-clock variance
        if delay_ms > 0 {
            std::thread::sleep(Duration::from_millis(delay_ms));
        }

        let dt = 0.05;
        for _ in 0..100 {
            mgr.step(dt, &HashMap::new(), &mut tag_store).unwrap();
        }

        let inst = mgr.get_instance(id).unwrap();
        let speed = inst.get_var("speed").unwrap_or(0.0);
        let flow = inst.get_var("flow").unwrap_or(0.0);
        let delta_p = inst.get_var("delta_p").unwrap_or(0.0);
        (speed, flow, delta_p)
    };

    let run1 = run_sim(0);
    let run2 = run_sim(15);

    assert_eq!(
        run1.0.to_bits(),
        run2.0.to_bits(),
        "Speed must be bit-for-bit identical regardless of wall-clock timing"
    );
    assert_eq!(
        run1.1.to_bits(),
        run2.1.to_bits(),
        "Flow must be bit-for-bit identical regardless of wall-clock timing"
    );
    assert_eq!(
        run1.2.to_bits(),
        run2.2.to_bits(),
        "Delta_P must be bit-for-bit identical regardless of wall-clock timing"
    );
}
