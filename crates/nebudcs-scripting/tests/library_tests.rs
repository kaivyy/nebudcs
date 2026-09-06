//! Phase 11 Equipment Library & Template Packages Verification Tests.

use std::collections::BTreeMap;

use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_model::equipment::{
    Cardinality, EquipmentCategory, Phase, PortDefinition, PortDirection, PortDomain,
};
use nebudcs_scripting::library::dependency::DependencyResolver;
use nebudcs_scripting::library::import_export::{
    export_package, export_template, import_package, import_template, SerializationFormat,
    MAX_IMPORT_SIZE_BYTES,
};
use nebudcs_scripting::library::package::PackageDependency;
use nebudcs_scripting::library::standard::{
    make_standard_centrifugal_pump, make_standard_pump_bypass_package,
};
use nebudcs_scripting::library::template::TemplateSource;
use nebudcs_scripting::library::EquipmentLibrary;
use nebudcs_scripting::model::{CustomEquipmentDefinition, CustomizationTier};
use nebudcs_scripting::runtime::CustomEquipmentManager;
use nebudcs_types::{EquipmentId, PortId, SchemaVersion};
use nebudcs_units::display::QuantityKind;

// ---------------------------------------------------------------------------
// Test 1: Standard Templates Catalog & Discovery
// ---------------------------------------------------------------------------
#[test]
fn test_standard_templates_catalog_and_discovery() {
    let mut library = EquipmentLibrary::new();
    let count = library
        .load_standard_catalog()
        .expect("Standard catalog loads successfully");
    assert_eq!(
        count, 12,
        "Must contain all 12 standard industrial templates"
    );

    // Discover templates
    let all = library.list_templates(None, None);
    assert_eq!(all.len(), 12);

    let rotating = library.list_templates(Some(EquipmentCategory::Rotating), None);
    assert_eq!(rotating.len(), 3, "Pump, compressor, fan");

    let valves = library.list_templates(Some(EquipmentCategory::Valve), None);
    assert_eq!(valves.len(), 1, "Control valve");

    let standard_src = library.list_templates(None, Some(TemplateSource::Standard));
    assert_eq!(standard_src.len(), 12);

    let user_src = library.list_templates(None, Some(TemplateSource::User));
    assert_eq!(user_src.len(), 0);

    // Specific template lookup
    let pump = library
        .get_template("standard.pump.centrifugal", None)
        .expect("Pump template exists");
    assert_eq!(pump.id, "standard.pump.centrifugal");
    assert_eq!(pump.category, EquipmentCategory::Rotating);
    assert!(pump.definition.parameters.contains_key("rated_flow"));
    assert!(pump.definition.curves.contains_key("head_flow"));
    assert_eq!(pump.definition.ports.len(), 3);
}

// ---------------------------------------------------------------------------
// Test 2: User Template Creation & Immutable Instantiation
// ---------------------------------------------------------------------------
#[test]
fn test_user_template_creation_and_isolation() {
    let mut library = EquipmentLibrary::new();

    // Create a custom equipment definition
    let def = CustomEquipmentDefinition {
        id: "user.custom_evaporator".to_string(),
        name: "Thin Film Evaporator".to_string(),
        version: SchemaVersion::new(1, 0, 0),
        category: EquipmentCategory::Process,
        tier: CustomizationTier::Level1Declarative,
        description: "Rotary wiped-film evaporator".to_string(),
        parameters: {
            let mut p = indexmap::IndexMap::new();
            p.insert(
                "rotor_speed".to_string(),
                nebudcs_model::equipment::ParameterDefinition {
                    name: "rotor_speed".to_string(),
                    description: "RPM".to_string(),
                    quantity_kind: QuantityKind::Speed,
                    default_value: 450.0,
                    min_value: Some(100.0),
                    max_value: Some(1000.0),
                    required: false,
                },
            );
            p
        },
        ports: vec![PortDefinition {
            id: PortId::new(),
            name: "feed".to_string(),
            direction: PortDirection::Inlet,
            domain: PortDomain::Material,
            phase_compatibility: vec![Phase::Liquid],
            required: true,
            cardinality: Cardinality::ZeroOrMore,
            description: "Feed".to_string(),
        }],
        curves: indexmap::IndexMap::new(),
        curves_2d: indexmap::IndexMap::new(),
        operating_envelope: Default::default(),
        alarm_definitions: Vec::new(),
        trips: Vec::new(),
        interlocks: Vec::new(),
        script: None,
        composition: None,
        metadata: indexmap::IndexMap::new(),
    };

    // Promote to user template
    let user_tmpl = library
        .register_user_template(def, None)
        .expect("Register user template");
    assert_eq!(user_tmpl.source, TemplateSource::User);

    // Instantiation 1: with parameter overrides
    let mut overrides1 = BTreeMap::new();
    overrides1.insert("rotor_speed".to_string(), 600.0);
    let id1 = EquipmentId::new();
    let inst1 = user_tmpl
        .instantiate(id1, "EVAP-101", &overrides1)
        .expect("Instantiate 1");
    assert_eq!(inst1.id, id1);
    assert_eq!(inst1.parameters.get("rotor_speed"), Some(&600.0));

    // Instantiation 2: default parameter
    let id2 = EquipmentId::new();
    let inst2 = user_tmpl
        .instantiate(id2, "EVAP-102", &BTreeMap::new())
        .expect("Instantiate 2");
    assert_eq!(inst2.id, id2);
    assert_eq!(inst2.parameters.get("rotor_speed"), Some(&450.0));

    // Verify template immutability
    let lookup = library
        .get_template("user.custom_evaporator", None)
        .unwrap();
    assert_eq!(
        lookup.definition.parameters["rotor_speed"].default_value,
        450.0
    );

    // State isolation check: mutating inst1 does not mutate inst2 or template
    let mut inst1_mut = inst1.clone();
    inst1_mut.set_var("speed", 750.0);
    assert_eq!(inst1_mut.get_var("speed"), Some(750.0));
    assert_eq!(inst2.get_var("speed"), Some(0.0));
}

// ---------------------------------------------------------------------------
// Test 3: Equipment Packages & Hierarchical Composition
// ---------------------------------------------------------------------------
#[test]
fn test_equipment_package_validation_and_composition() {
    let library = EquipmentLibrary::with_standard_library().expect("Standard library loads");

    let packages = library.list_packages();
    assert_eq!(
        packages.len(),
        2,
        "Pump bypass skid and two-stage compressor skid"
    );

    // Inspect pump bypass skid
    let pump_skid = library
        .get_package("standard.package.pump_bypass_skid", None)
        .expect("Pump skid package exists");
    assert_eq!(pump_skid.sub_components.len(), 2);
    assert_eq!(pump_skid.internal_streams.len(), 1);
    assert_eq!(pump_skid.external_ports.len(), 4);

    // Validation against catalog passes
    assert!(library.validate_package(pump_skid).is_ok());

    // Instantiation of package into sub-components
    let mut overrides = BTreeMap::new();
    overrides.insert("skid_flow".to_string(), 150.0);

    let res = library
        .instantiate_package(
            "standard.package.pump_bypass_skid",
            None,
            "SKID_A",
            &overrides,
        )
        .expect("Instantiate pump skid");
    assert_eq!(res.instances.len(), 2);
    assert_eq!(res.instances[0].name, "SKID_A_main_pump");
    assert_eq!(res.instances[1].name, "SKID_A_bypass_valve");
    assert_eq!(res.instances[0].parameters.get("rated_flow"), Some(&150.0));
}

// ---------------------------------------------------------------------------
// Test 4: Package Validation Edge Cases & Rejections
// ---------------------------------------------------------------------------
#[test]
fn test_package_validation_rejections() {
    let library = EquipmentLibrary::with_standard_library().expect("Standard library loads");

    // Case 1: Unknown child template
    let mut bad_pkg = make_standard_pump_bypass_package();
    bad_pkg.id = "bad.pkg.1".to_string();
    bad_pkg.sub_components[0].template_id = "nonexistent.template".to_string();
    let err = library.validate_package(&bad_pkg).unwrap_err();
    assert!(err.to_string().contains("references unknown template"));

    // Case 2: Duplicate sub-component ID
    let mut bad_pkg2 = make_standard_pump_bypass_package();
    bad_pkg2.id = "bad.pkg.2".to_string();
    bad_pkg2.sub_components[1].id = bad_pkg2.sub_components[0].id.clone();
    let err2 = library.validate_package(&bad_pkg2).unwrap_err();
    assert!(err2.to_string().contains("Duplicate sub-component ID"));

    // Case 3: Self-connection in internal stream
    let mut bad_pkg3 = make_standard_pump_bypass_package();
    bad_pkg3.id = "bad.pkg.3".to_string();
    bad_pkg3.internal_streams[0].to_component = bad_pkg3.internal_streams[0].from_component.clone();
    let err3 = library.validate_package(&bad_pkg3).unwrap_err();
    assert!(err3.to_string().contains("self-connection"));

    // Case 4: Nonexistent internal port in external port mapping
    let mut bad_pkg4 = make_standard_pump_bypass_package();
    bad_pkg4.id = "bad.pkg.4".to_string();
    bad_pkg4.external_ports[0].internal_port = "nonexistent_port".to_string();
    let err4 = library.validate_package(&bad_pkg4).unwrap_err();
    assert!(err4.to_string().contains("does not exist"));
}

// ---------------------------------------------------------------------------
// Test 5: Versioning, Immutability & Content Fingerprinting
// ---------------------------------------------------------------------------
#[test]
fn test_versioning_and_immutability() {
    let mut library = EquipmentLibrary::new();
    let pump = make_standard_centrifugal_pump().unwrap();

    // First registration succeeds
    library
        .register_template(pump.clone())
        .expect("First registration ok");

    // Identical re-registration succeeds (idempotent)
    assert!(library.register_template(pump.clone()).is_ok());

    // Altered definition under same (id, version) MUST be rejected
    let mut altered = pump.clone();
    altered.definition.description = "Altered description".to_string();
    let err = library.register_template(altered).unwrap_err();
    assert!(err.to_string().contains("Immutable version violation"));

    // Bumping version allows new registration
    let mut v2 = pump.clone();
    v2.version = SchemaVersion::new(1, 1, 0);
    v2.definition.version = SchemaVersion::new(1, 1, 0);
    v2.definition.description = "Upgraded v1.1.0 pump".to_string();
    assert!(library.register_template(v2).is_ok());

    // Both versions exist independently
    let v1_lookup = library
        .get_template(
            "standard.pump.centrifugal",
            Some(&SchemaVersion::new(1, 0, 0)),
        )
        .unwrap();
    let v2_lookup = library
        .get_template(
            "standard.pump.centrifugal",
            Some(&SchemaVersion::new(1, 1, 0)),
        )
        .unwrap();
    assert_ne!(
        v1_lookup.definition.description,
        v2_lookup.definition.description
    );

    // Latest version defaults to v1.1.0
    let latest = library
        .get_template("standard.pump.centrifugal", None)
        .unwrap();
    assert_eq!(latest.version, SchemaVersion::new(1, 1, 0));
}

// ---------------------------------------------------------------------------
// Test 6: Deterministic Dependency Resolution & Cycle Detection
// ---------------------------------------------------------------------------
#[test]
fn test_deterministic_dependency_resolution() {
    let mut resolver = DependencyResolver::new();

    // pkg_c -> pkg_b -> pkg_a
    resolver.add_dependencies("pkg_a", Vec::new());
    resolver.add_dependencies(
        "pkg_b",
        vec![PackageDependency {
            id: "pkg_a".to_string(),
            version_req: "^1.0.0".to_string(),
        }],
    );
    resolver.add_dependencies(
        "pkg_c",
        vec![
            PackageDependency {
                id: "pkg_b".to_string(),
                version_req: "^1.0.0".to_string(),
            },
            PackageDependency {
                id: "pkg_a".to_string(),
                version_req: "^1.0.0".to_string(),
            },
        ],
    );

    let mut available = std::collections::HashMap::new();
    available.insert(
        "pkg_a".to_string(),
        vec![SchemaVersion::new(1, 0, 0), SchemaVersion::new(1, 2, 0)],
    );
    available.insert("pkg_b".to_string(), vec![SchemaVersion::new(1, 0, 0)]);
    available.insert("pkg_c".to_string(), vec![SchemaVersion::new(1, 0, 0)]);

    let order = resolver.resolve(&available).expect("Resolve dependencies");
    assert_eq!(
        order,
        vec!["pkg_a", "pkg_b", "pkg_c"],
        "Leaves must be resolved before dependents"
    );

    // Cycle detection: a -> b -> c -> a
    let mut cycle_resolver = DependencyResolver::new();
    cycle_resolver.add_dependencies(
        "node_a",
        vec![PackageDependency {
            id: "node_b".to_string(),
            version_req: "*".to_string(),
        }],
    );
    cycle_resolver.add_dependencies(
        "node_b",
        vec![PackageDependency {
            id: "node_c".to_string(),
            version_req: "*".to_string(),
        }],
    );
    cycle_resolver.add_dependencies(
        "node_c",
        vec![PackageDependency {
            id: "node_a".to_string(),
            version_req: "*".to_string(),
        }],
    );

    let mut cycle_available = std::collections::HashMap::new();
    cycle_available.insert("node_a".to_string(), vec![SchemaVersion::new(1, 0, 0)]);
    cycle_available.insert("node_b".to_string(), vec![SchemaVersion::new(1, 0, 0)]);
    cycle_available.insert("node_c".to_string(), vec![SchemaVersion::new(1, 0, 0)]);

    let err = cycle_resolver.resolve(&cycle_available).unwrap_err();
    assert!(err.to_string().contains("Dependency cycle detected"));
}

// ---------------------------------------------------------------------------
// Test 7: Import & Export Round-Trip (JSON and TOML)
// ---------------------------------------------------------------------------
#[test]
fn test_import_export_round_trip() {
    let pump = make_standard_centrifugal_pump().unwrap();

    // JSON round-trip
    let json_str = export_template(&pump, SerializationFormat::Json).expect("Export JSON");
    let imported_json = import_template(&json_str, SerializationFormat::Json).expect("Import JSON");
    assert_eq!(pump.id, imported_json.id);
    assert_eq!(
        pump.definition.parameters.len(),
        imported_json.definition.parameters.len()
    );

    // Deterministic export check: re-exporting produce identical string
    let json_str_2 =
        export_template(&imported_json, SerializationFormat::Json).expect("Export JSON 2");
    assert_eq!(
        json_str, json_str_2,
        "Export must be bit-for-bit deterministic"
    );

    // TOML round-trip
    let toml_str = export_template(&pump, SerializationFormat::Toml).expect("Export TOML");
    let imported_toml = import_template(&toml_str, SerializationFormat::Toml).expect("Import TOML");
    assert_eq!(pump.id, imported_toml.id);

    // Package JSON round-trip
    let pkg = make_standard_pump_bypass_package();
    let pkg_json = export_package(&pkg, SerializationFormat::Json).expect("Export package JSON");
    let imported_pkg =
        import_package(&pkg_json, SerializationFormat::Json).expect("Import package JSON");
    assert_eq!(pkg.id, imported_pkg.id);
    assert_eq!(pkg.sub_components.len(), imported_pkg.sub_components.len());
}

// ---------------------------------------------------------------------------
// Test 8: Security, Oversized Payload & Adversarial Rejections
// ---------------------------------------------------------------------------
#[test]
fn test_import_security_and_resource_caps() {
    // 1. Oversized payload rejection (> 2 MB)
    let huge_content = " ".repeat(MAX_IMPORT_SIZE_BYTES + 1024);
    let err = import_template(&huge_content, SerializationFormat::Json).unwrap_err();
    assert!(err.to_string().contains("exceeds maximum limit"));

    // 2. Malformed JSON syntax
    let malformed = "{ id: 'pump', broken: ";
    let err2 = import_template(malformed, SerializationFormat::Json).unwrap_err();
    assert!(err2.to_string().contains("Malformed JSON document"));

    // 3. Illegal identifier characters
    let bad_id_json = r#"{
        "id": "pump/evil$name",
        "name": "Bad Pump",
        "version": "1.0.0",
        "category": "rotating",
        "source": "user",
        "metadata": { "author": "attacker", "description": "test" },
        "definition": {
            "id": "pump.bad",
            "name": "Bad",
            "version": "1.0.0",
            "category": "rotating"
        }
    }"#;
    let err3 = import_template(bad_id_json, SerializationFormat::Json).unwrap_err();
    assert!(err3.to_string().contains("contains illegal character"));

    // 4. Non-finite parameter value rejection
    let nan_param_json = r#"{
        "id": "pump.nan",
        "name": "Nan Pump",
        "version": "1.0.0",
        "category": "rotating",
        "source": "user",
        "metadata": { "author": "attacker", "description": "test" },
        "definition": {
            "id": "pump.nan",
            "name": "Nan",
            "version": "1.0.0",
            "category": "rotating",
            "parameters": {
                "flow": {
                    "name": "flow",
                    "description": "test",
                    "quantity_kind": "volumetric_flow",
                    "default_value": "NaN"
                }
            }
        }
    }"#;
    assert!(import_template(nan_param_json, SerializationFormat::Json).is_err());
}

// ---------------------------------------------------------------------------
// Test 9: End-to-End Simulation Runtime Execution & State Isolation
// ---------------------------------------------------------------------------
#[test]
fn test_runtime_manager_integration_and_stepping() {
    let mut mgr = CustomEquipmentManager::new();
    let mut tag_store = RuntimeTagStore::new();

    // Instantiate a standard centrifugal pump template through CustomEquipmentManager
    let mut overrides = BTreeMap::new();
    overrides.insert("rated_flow".to_string(), 140.0);
    let pump_id = EquipmentId::new();
    mgr.instantiate_template(
        "standard.pump.centrifugal",
        None,
        pump_id,
        "PUMP-101",
        &overrides,
    )
    .expect("Instantiate template in manager");

    // Verify initial instance state
    let inst = mgr
        .get_instance(pump_id)
        .expect("Instance exists in manager");
    assert_eq!(inst.name, "PUMP-101");
    assert_eq!(inst.parameters["rated_flow"], 140.0);
    assert_eq!(inst.active_state, "stopped");
    assert!(!inst.is_running());

    // Command start
    mgr.start(pump_id).expect("Start pump");
    assert!(mgr.get_instance(pump_id).unwrap().is_running());

    // Simulate 50 ticks
    let dt = 0.05;
    for _ in 0..50 {
        mgr.step(dt, &std::collections::HashMap::new(), &mut tag_store)
            .expect("Step manager");
    }

    // Trip the equipment via high vibration
    if let Some(inst_mut) = mgr.get_instance_mut(pump_id) {
        inst_mut.set_var("vibration", 8.5); // exceeds 7.1 threshold
    }
    mgr.step(dt, &std::collections::HashMap::new(), &mut tag_store)
        .expect("Step after vibration spike");

    let tripped_inst = mgr.get_instance(pump_id).unwrap();
    assert!(
        tripped_inst.is_tripped(),
        "Pump must trip due to vibration > 7.1"
    );
    assert_eq!(tripped_inst.active_state, "tripped");

    // Cannot restart while trip condition is active
    assert!(mgr.start(pump_id).is_err(), "Cannot start while tripped");

    // Reset while vibration still high must fail
    assert!(
        !mgr.reset_trip(pump_id).unwrap(),
        "Reset fails while vibration condition persists"
    );

    // Clear vibration and reset trip
    mgr.get_instance_mut(pump_id)
        .unwrap()
        .set_var("vibration", 2.0);
    assert!(
        mgr.reset_trip(pump_id).unwrap(),
        "Reset succeeds when vibration normalizes"
    );
    assert!(!mgr.get_instance(pump_id).unwrap().is_tripped());
}
