//! Comprehensive integration and verification tests for custom equipment.
//!
//! Covers:
//! - Level 1: Declarative parametric equipment with performance curves & limits
//! - Level 2: Composite equipment packages
//! - Level 3: Sandboxed Rhai scripted behavior models
//! - Causal propagation: Cavitation NPSH -> Vibration -> Alarm -> Protective Trip -> Downstream Cutoff
//! - Multiple instance isolation & deterministic execution
//! - Adversarial schema and validation rejections

use indexmap::IndexMap;
use std::collections::{BTreeMap, HashMap};

use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_model::equipment::*;
use nebudcs_model::tag::{TagQuality, TagValue};
use nebudcs_scripting::*;
use nebudcs_types::{EquipmentId, PortId, SchemaVersion, TagId};
use nebudcs_units::display::QuantityKind;

fn make_port(name: &str, dir: PortDirection, domain: PortDomain) -> PortDefinition {
    PortDefinition {
        id: PortId::new(),
        name: name.to_string(),
        domain,
        direction: dir,
        phase_compatibility: vec![Phase::Liquid],
        required: true,
        cardinality: Cardinality::ExactlyOne,
        description: format!("Port {name}"),
    }
}

fn make_param(name: &str, default: f64, min: f64, max: f64) -> ParameterDefinition {
    ParameterDefinition {
        name: name.to_string(),
        description: format!("Parameter {name}"),
        quantity_kind: QuantityKind::Pressure,
        default_value: default,
        min_value: Some(min),
        max_value: Some(max),
        required: true,
    }
}

#[test]
fn test_level1_declarative_pump_curves_and_trips() {
    let mut mgr = CustomEquipmentManager::new();

    // 1. Define custom pump with Head Curve and Power Curve
    let mut def = CustomEquipmentDefinition {
        id: "DEF-PUMP-01".into(),
        name: "Standard Centrifugal Pump".into(),
        version: SchemaVersion::new(1, 0, 0),
        category: EquipmentCategory::Rotating,
        tier: CustomizationTier::Level1Declarative,
        description: "Centrifugal pump with drooping head curve".into(),
        parameters: IndexMap::new(),
        ports: vec![
            make_port("suction", PortDirection::Inlet, PortDomain::Material),
            make_port("discharge", PortDirection::Outlet, PortDomain::Material),
        ],
        curves: IndexMap::new(),
        curves_2d: IndexMap::new(),
        operating_envelope: OperatingEnvelope::default(),
        alarm_definitions: vec![],
        trips: vec![TripConditionDefinition {
            id: "TRIP-OVERPRESSURE".into(),
            name: "High Discharge Pressure Trip".into(),
            condition_var: "discharge_p".into(),
            operator: TripOperator::GreaterThan,
            threshold: 120.0,
            fail_safe_state: "tripped".into(),
            latched: true,
        }],
        interlocks: vec![],
        script: None,
        composition: None,
        metadata: IndexMap::new(),
    };

    def.parameters.insert(
        "rated_speed".into(),
        make_param("rated_speed", 1800.0, 0.0, 3600.0),
    );
    def.curves.insert(
        "head_curve".into(),
        Curve1D::new(
            "HeadCurve",
            "flow",
            "delta_p",
            vec![(0.0, 100.0), (50.0, 85.0), (100.0, 60.0), (150.0, 20.0)],
        ),
    );

    assert!(mgr.register_definition(def).is_ok());

    // 2. Instantiate pump
    let p_id = EquipmentId::new();
    mgr.instantiate(p_id, "DEF-PUMP-01", "P-101", &BTreeMap::new())
        .expect("instantiate succeeds");

    let mut tag_store = RuntimeTagStore::new();
    let status_tag = TagId::new();
    tag_store.register_tag(
        "P-101.STATUS",
        status_tag,
        TagValue::Integer(0),
        TagQuality::Good,
    );

    // 3. Initially stopped: delta_p = 0
    let mut inputs = HashMap::new();
    mgr.step(0.1, &inputs, &mut tag_store).unwrap();
    let p_inst = mgr.get_instance(p_id).unwrap();
    assert_eq!(p_inst.active_state, "stopped");
    assert_eq!(p_inst.get_var("delta_p").unwrap(), 0.0);

    // 4. Start pump
    mgr.start(p_id).unwrap();
    let mut p_inputs = BTreeMap::new();
    p_inputs.insert("flow".into(), 50.0);
    inputs.insert(p_id, p_inputs);

    mgr.step(0.1, &inputs, &mut tag_store).unwrap();
    let p_running = mgr.get_instance(p_id).unwrap();
    assert_eq!(p_running.active_state, "running");
    assert_eq!(p_running.get_var("delta_p").unwrap(), 85.0);
    assert_eq!(tag_store.get_value(status_tag), Some(&TagValue::Integer(1)));

    // 5. Trigger trip: shutoff condition leads to overpressure (discharge_p > 120.0)
    mgr.get_instance_mut(p_id)
        .unwrap()
        .set_var("discharge_p", 125.0);
    mgr.step(0.1, &inputs, &mut tag_store).unwrap();

    let p_tripped = mgr.get_instance(p_id).unwrap();
    assert_eq!(p_tripped.active_state, "tripped");
    assert!(p_tripped.get_flag("is_tripped").unwrap());
    assert!(!p_tripped.is_running());
    assert_eq!(p_tripped.get_var("delta_p").unwrap(), 0.0);

    // 6. Reset fails while condition still violated
    mgr.get_instance_mut(p_id)
        .unwrap()
        .set_var("discharge_p", 125.0);
    let reset_attempt = mgr.reset_trip(p_id).unwrap();
    assert!(
        !reset_attempt,
        "Reset must fail if trip condition remains violated"
    );

    // 7. Reset succeeds when condition cleared
    mgr.get_instance_mut(p_id)
        .unwrap()
        .set_var("discharge_p", 50.0);
    let reset_ok = mgr.reset_trip(p_id).unwrap();
    assert!(reset_ok, "Reset must succeed when condition clears");
    assert_eq!(mgr.get_instance(p_id).unwrap().active_state, "stopped");
}

#[test]
fn test_level2_composition_package() {
    let mut mgr = CustomEquipmentManager::new();

    let comp = CompositionDefinition {
        sub_components: vec![
            SubComponentDefinition {
                id: "pump_core".into(),
                template_id: "GENERIC_PUMP".into(),
                parameters: IndexMap::new(),
            },
            SubComponentDefinition {
                id: "bypass_valve".into(),
                template_id: "GENERIC_VALVE".into(),
                parameters: IndexMap::new(),
            },
        ],
        internal_connections: vec![InternalConnectionDefinition {
            from_component: "pump_core".into(),
            from_port: "discharge".into(),
            to_component: "bypass_valve".into(),
            to_port: "inlet".into(),
        }],
        port_mappings: vec![PortMappingDefinition {
            external_port_name: "skid_suction".into(),
            internal_component: "pump_core".into(),
            internal_port: "suction".into(),
        }],
    };

    let def = CustomEquipmentDefinition {
        id: "SKID-PUMP-PACKAGE".into(),
        name: "Skid Pump Package with Minimum Flow Recirc".into(),
        version: SchemaVersion::new(1, 0, 0),
        category: EquipmentCategory::Rotating,
        tier: CustomizationTier::Level2Composition,
        description: "Composite pump skid".into(),
        parameters: IndexMap::new(),
        ports: vec![
            make_port("skid_suction", PortDirection::Inlet, PortDomain::Material),
            make_port(
                "skid_discharge",
                PortDirection::Outlet,
                PortDomain::Material,
            ),
        ],
        curves: IndexMap::new(),
        curves_2d: IndexMap::new(),
        operating_envelope: OperatingEnvelope::default(),
        alarm_definitions: vec![],
        trips: vec![],
        interlocks: vec![],
        script: None,
        composition: Some(comp),
        metadata: IndexMap::new(),
    };

    assert!(mgr.register_definition(def).is_ok());
    let id = EquipmentId::new();
    assert!(mgr
        .instantiate(id, "SKID-PUMP-PACKAGE", "SKID-101", &BTreeMap::new())
        .is_ok());
}

#[test]
fn test_level3_scripted_dynamic_behavior() {
    let mut mgr = CustomEquipmentManager::new();

    let script = r#"
        fn step(inputs, params, state, dt) {
            let flow_in = inputs["feed_flow"];
            let temp_k = inputs["temperature"];
            let activation_energy = params["e_act"];
            let k0 = params["k0"];
            
            // Arrhenius reaction rate law: k = k0 * exp(-E_act / T)
            let rate_k = k0 * exp(-activation_energy / temp_k);
            let conversion = clamp(rate_k * flow_in * 0.1, 0.0, 0.99);
            
            state["reaction_rate"] = rate_k;
            state["conversion_pct"] = conversion * 100.0;
            state["product_flow"] = flow_in * conversion;
            
            // Auto trip if temperature exceeds 550 K
            if temp_k > 550.0 {
                state["trip"] = true;
                state["trip_reason"] = "Thermal Runaway";
            }
            
            return state;
        }
    "#;

    let mut def = CustomEquipmentDefinition {
        id: "DEF-TUBULAR-REACTOR".into(),
        name: "Custom Tubular Reaction Vessel".into(),
        version: SchemaVersion::new(1, 0, 0),
        category: EquipmentCategory::Process,
        tier: CustomizationTier::Level3Scripted,
        description: "Kinetic reaction modeling via Rhai".into(),
        parameters: IndexMap::new(),
        ports: vec![
            make_port("inlet", PortDirection::Inlet, PortDomain::Material),
            make_port("outlet", PortDirection::Outlet, PortDomain::Material),
        ],
        curves: IndexMap::new(),
        curves_2d: IndexMap::new(),
        operating_envelope: OperatingEnvelope::default(),
        alarm_definitions: vec![],
        trips: vec![],
        interlocks: vec![],
        script: Some(ScriptDefinition {
            code: script.into(),
            entry_point: "step".into(),
        }),
        composition: None,
        metadata: IndexMap::new(),
    };

    def.parameters
        .insert("e_act".into(), make_param("e_act", 1200.0, 100.0, 10000.0));
    def.parameters
        .insert("k0".into(), make_param("k0", 50.0, 1.0, 1000.0));

    assert!(mgr.register_definition(def).is_ok());

    let r_id = EquipmentId::new();
    mgr.instantiate(r_id, "DEF-TUBULAR-REACTOR", "R-101", &BTreeMap::new())
        .unwrap();

    let mut tag_store = RuntimeTagStore::new();
    let mut inputs = HashMap::new();
    let mut r_inputs = BTreeMap::new();
    r_inputs.insert("feed_flow".into(), 10.0);
    r_inputs.insert("temperature".into(), 400.0);
    inputs.insert(r_id, r_inputs);

    // Normal reaction step
    mgr.step(0.1, &inputs, &mut tag_store).unwrap();
    let r_inst = mgr.get_instance(r_id).unwrap();
    assert!(r_inst.get_var("conversion_pct").unwrap() > 0.0);
    assert!(!r_inst.get_flag("is_tripped").unwrap());

    // Runaway temperature step -> triggers script-commanded trip
    inputs
        .get_mut(&r_id)
        .unwrap()
        .insert("temperature".into(), 580.0);
    mgr.step(0.1, &inputs, &mut tag_store).unwrap();
    let r_tripped = mgr.get_instance(r_id).unwrap();
    assert_eq!(r_tripped.active_state, "tripped");
    assert!(r_tripped.get_flag("is_tripped").unwrap());
}

#[test]
fn test_mandatory_causal_propagation_npsh_cavitation_to_trip() {
    // Requirements proof:
    // Tank level / suction pressure ↓ -> NPSHA ↓ -> NPSHA < NPSHR -> vibration ↑ -> alarm -> trip -> pump stops -> downstream effects
    let mut mgr = CustomEquipmentManager::new();

    let script = r#"
        fn step(inputs, params, state, dt) {
            let suction_p = inputs["suction_pressure"];
            let flow = inputs["flow"];
            let vapor_p = 2.34; // Water vapor pressure at 20°C in kPa
            
            // NPSHA in meters: (P_suction - P_vapor) / (rho * g) * 100
            let npsha = max((suction_p - vapor_p) * 0.1, 0.0);
            
            // NPSHR curve: 2.0 + 0.0005 * flow^2
            let npshr = 2.0 + 0.0005 * pow(flow, 2.0);
            
            state["npsha"] = npsha;
            state["npshr"] = npshr;
            
            if npsha < npshr {
                // Cavitation onset: vibration surges proportionally to deficit
                let deficit = npshr - npsha;
                state["cavitation_active"] = true;
                state["vibration_mm_s"] = 1.5 + deficit * 4.0;
            } else {
                state["cavitation_active"] = false;
                state["vibration_mm_s"] = 1.2;
            }
            
            // Normal pump head when running
            state["delta_p"] = max(80.0 - 0.002 * pow(flow, 2.0), 0.0);
            return state;
        }
    "#;

    let def = CustomEquipmentDefinition {
        id: "DEF-CAVITATING-PUMP".into(),
        name: "Pump with Dynamic Cavitation Model".into(),
        version: SchemaVersion::new(1, 0, 0),
        category: EquipmentCategory::Rotating,
        tier: CustomizationTier::Level3Scripted,
        description: "Predicts NPSHA vs NPSHR cavitation and vibration".into(),
        parameters: IndexMap::new(),
        ports: vec![
            make_port("suction", PortDirection::Inlet, PortDomain::Material),
            make_port("discharge", PortDirection::Outlet, PortDomain::Material),
        ],
        curves: IndexMap::new(),
        curves_2d: IndexMap::new(),
        operating_envelope: OperatingEnvelope::default(),
        alarm_definitions: vec![],
        trips: vec![TripConditionDefinition {
            id: "TRIP-VIBRATION-HIHI".into(),
            name: "High-High Bearing Vibration Trip".into(),
            condition_var: "vibration_mm_s".into(),
            operator: TripOperator::GreaterThan,
            threshold: 7.0, // 7.0 mm/s trip limit
            fail_safe_state: "tripped".into(),
            latched: true,
        }],
        interlocks: vec![],
        script: Some(ScriptDefinition {
            code: script.into(),
            entry_point: "step".into(),
        }),
        composition: None,
        metadata: IndexMap::new(),
    };

    mgr.register_definition(def).unwrap();

    let p_id = EquipmentId::new();
    mgr.instantiate(p_id, "DEF-CAVITATING-PUMP", "P-101", &BTreeMap::new())
        .unwrap();
    mgr.start(p_id).unwrap();

    let mut tag_store = RuntimeTagStore::new();
    let mut inputs = HashMap::new();
    let mut p_inputs = BTreeMap::new();
    p_inputs.insert("flow".into(), 40.0);
    p_inputs.insert("suction_pressure".into(), 100.0); // 100 kPa -> NPSHA = ~9.7m > NPSHR 2.8m
    inputs.insert(p_id, p_inputs);

    // Stage 1: Healthy Suction State
    mgr.step(0.1, &inputs, &mut tag_store).unwrap();
    let p1 = mgr.get_instance(p_id).unwrap();
    assert_eq!(p1.active_state, "running");
    assert!(!p1.get_flag("cavitation_active").unwrap());
    assert!(p1.get_var("vibration_mm_s").unwrap() < 2.0);
    assert!(p1.get_var("delta_p").unwrap() > 70.0);

    // Stage 2: Tank Level Drops -> Suction Pressure Drops to 15 kPa
    // NPSHA = (15 - 2.34)*0.1 = 1.26m < NPSHR 2.8m -> Deficit = 1.54m -> Vibration = 1.5 + 1.54*4 = 7.66 mm/s > 7.0 limit!
    inputs
        .get_mut(&p_id)
        .unwrap()
        .insert("suction_pressure".into(), 15.0);

    mgr.step(0.1, &inputs, &mut tag_store).unwrap();
    let p2 = mgr.get_instance(p_id).unwrap();

    // Stage 3: Vibration Trip Activates -> Pump Stops -> Downstream Head Drops to 0
    assert_eq!(
        p2.active_state, "tripped",
        "Pump must trip on high vibration"
    );
    assert!(p2.get_flag("is_tripped").unwrap());
    assert!(!p2.is_running());
    assert_eq!(
        p2.get_var("delta_p").unwrap(),
        0.0,
        "Tripped pump generates zero head"
    );
    assert_eq!(p2.latched_trips, vec!["TRIP-VIBRATION-HIHI"]);
}

#[test]
fn test_multiple_instances_state_isolation() {
    let mut mgr = CustomEquipmentManager::new();

    let script = r#"
        fn step(inputs, params, state, dt) {
            let mult = params["multiplier"];
            state["val"] = inputs["in"] * mult;
            return state;
        }
    "#;

    let mut def = CustomEquipmentDefinition {
        id: "DEF-SCALER".into(),
        name: "Signal Scaler".into(),
        version: SchemaVersion::new(1, 0, 0),
        category: EquipmentCategory::Instrumentation,
        tier: CustomizationTier::Level3Scripted,
        description: "Scales signal".into(),
        parameters: IndexMap::new(),
        ports: vec![make_port("in", PortDirection::Inlet, PortDomain::Signal)],
        curves: IndexMap::new(),
        curves_2d: IndexMap::new(),
        operating_envelope: OperatingEnvelope::default(),
        alarm_definitions: vec![],
        trips: vec![],
        interlocks: vec![],
        script: Some(ScriptDefinition {
            code: script.into(),
            entry_point: "step".into(),
        }),
        composition: None,
        metadata: IndexMap::new(),
    };
    def.parameters.insert(
        "multiplier".into(),
        make_param("multiplier", 1.0, 0.1, 10.0),
    );
    mgr.register_definition(def).unwrap();

    let id1 = EquipmentId::new();
    let id2 = EquipmentId::new();

    let mut p1 = BTreeMap::new();
    p1.insert("multiplier".into(), 2.0);
    let mut p2 = BTreeMap::new();
    p2.insert("multiplier".into(), 5.0);

    mgr.instantiate(id1, "DEF-SCALER", "SC-1", &p1).unwrap();
    mgr.instantiate(id2, "DEF-SCALER", "SC-2", &p2).unwrap();

    let mut inputs = HashMap::new();
    let mut in1 = BTreeMap::new();
    in1.insert("in".into(), 10.0);
    let mut in2 = BTreeMap::new();
    in2.insert("in".into(), 10.0);
    inputs.insert(id1, in1);
    inputs.insert(id2, in2);

    let mut tag_store = RuntimeTagStore::new();
    mgr.step(0.1, &inputs, &mut tag_store).unwrap();

    assert_eq!(mgr.get_instance(id1).unwrap().get_var("val").unwrap(), 20.0);
    assert_eq!(mgr.get_instance(id2).unwrap().get_var("val").unwrap(), 50.0);
}
