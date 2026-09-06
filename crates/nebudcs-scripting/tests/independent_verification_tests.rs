//! Stage A Independent Verification Test Suite for NebuDCS Phase 10: Custom Equipment & Runtime Authoring.
//!
//! Authoritatively verifies:
//! - A1: Three Customization Levels (Level 1 Declarative, Level 2 Composite, Level 3 Rhai Scripted)
//! - A2: Runtime Lifecycle & Multi-Instance Isolation (Definition -> Instance -> RuntimeState)
//! - A3: Rhai Sandbox Adversarial Security & Resource Quotas (50k instructions, recursion, limits, I/O denial)
//! - A4: Determinism & Temporal Independence (Bit-identical trajectories, replay, clock speed invariance)
//! - A5: Curves and Operating Envelopes (Adversarial coordinates, Fritsch-Carlson monotonicity, 2D bilinear)
//! - A6: Composite Packages and Subgraphs (Nesting, port forwarding, connection validation)
//! - A7: Control, Alarm, Interlock & Trip Integration (Latching, unsafe reset rejection, permissives)
//! - A8: Runtime Authoring & Zero-Recompilation Proof (Full lifecycle purely from runtime JSON)

use indexmap::IndexMap;
use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_model::equipment::*;
use nebudcs_scripting::curve::{Curve1D, Curve2D, ExtrapolationMode, InterpolationMethod};
use nebudcs_scripting::error::ScriptingError;
use nebudcs_scripting::model::*;
use nebudcs_scripting::runtime::CustomEquipmentManager;
use nebudcs_scripting::sandbox::{RhaiSandbox, ScriptExecutionContext};
use nebudcs_scripting::validation::validate_custom_equipment;
use nebudcs_types::{EquipmentId, PortId, SchemaVersion, TagId};
use nebudcs_units::display::QuantityKind;

fn helper_port(name: &str, dir: PortDirection, domain: PortDomain) -> PortDefinition {
    PortDefinition {
        id: PortId::new(),
        name: name.to_string(),
        domain,
        direction: dir,
        phase_compatibility: vec![Phase::Liquid, Phase::Gas],
        required: true,
        cardinality: Cardinality::ExactlyOne,
        description: format!("Port {name}"),
    }
}

fn helper_param(name: &str, default: f64, min: f64, max: f64) -> ParameterDefinition {
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

// ============================================================================
// A1: THREE CUSTOMIZATION LEVELS (GENERIC & DATA-DRIVEN)
// ============================================================================

#[test]
fn test_stage_a1_three_customization_levels_generic_and_data_driven() {
    let mut mgr = CustomEquipmentManager::new();

    // 1. Level 1: Declarative Configuration
    let mut def_lvl1 = CustomEquipmentDefinition {
        id: "DEF-LVL1-PUMP".into(),
        name: "Generic Level 1 Pump".into(),
        version: SchemaVersion::new(1, 0, 0),
        category: EquipmentCategory::Rotating,
        tier: CustomizationTier::Level1Declarative,
        description: "Purely declarative parametric pump".into(),
        parameters: IndexMap::new(),
        ports: vec![
            helper_port("inlet", PortDirection::Inlet, PortDomain::Material),
            helper_port("outlet", PortDirection::Outlet, PortDomain::Material),
        ],
        curves: IndexMap::new(),
        curves_2d: IndexMap::new(),
        operating_envelope: OperatingEnvelope::default(),
        alarm_definitions: vec![],
        trips: vec![],
        interlocks: vec![],
        script: None,
        composition: None,
        metadata: IndexMap::new(),
    };
    def_lvl1.parameters.insert(
        "design_flow".into(),
        helper_param("design_flow", 50.0, 0.0, 500.0),
    );
    def_lvl1.curves.insert(
        "head_curve".into(),
        Curve1D::new(
            "HeadCurve",
            "flow",
            "delta_p",
            vec![(0.0, 100.0), (50.0, 80.0), (100.0, 40.0)],
        ),
    );
    assert!(mgr.register_definition(def_lvl1).is_ok());

    // 2. Level 2: Composite Package Subgraph
    let def_lvl2 = CustomEquipmentDefinition {
        id: "DEF-LVL2-SKID".into(),
        name: "Generic Pumping Skid Package".into(),
        version: SchemaVersion::new(1, 0, 0),
        category: EquipmentCategory::Process,
        tier: CustomizationTier::Level2Composition,
        description: "Composite package of two pumps in parallel".into(),
        parameters: IndexMap::new(),
        ports: vec![
            helper_port("skid_inlet", PortDirection::Inlet, PortDomain::Material),
            helper_port("skid_outlet", PortDirection::Outlet, PortDomain::Material),
        ],
        curves: IndexMap::new(),
        curves_2d: IndexMap::new(),
        operating_envelope: OperatingEnvelope::default(),
        alarm_definitions: vec![],
        trips: vec![],
        interlocks: vec![],
        script: None,
        composition: Some(CompositionDefinition {
            sub_components: vec![
                SubComponentDefinition {
                    id: "P101A".into(),
                    template_id: "DEF-LVL1-PUMP".into(),
                    parameters: IndexMap::new(),
                },
                SubComponentDefinition {
                    id: "P101B".into(),
                    template_id: "DEF-LVL1-PUMP".into(),
                    parameters: IndexMap::new(),
                },
            ],
            internal_connections: vec![],
            port_mappings: vec![
                PortMappingDefinition {
                    external_port_name: "skid_inlet".into(),
                    internal_component: "P101A".into(),
                    internal_port: "inlet".into(),
                },
                PortMappingDefinition {
                    external_port_name: "skid_outlet".into(),
                    internal_component: "P101A".into(),
                    internal_port: "outlet".into(),
                },
            ],
        }),
        metadata: IndexMap::new(),
    };
    assert!(mgr.register_definition(def_lvl2).is_ok());

    // 3. Level 3: Sandboxed Rhai Script
    let def_lvl3 = CustomEquipmentDefinition {
        id: "DEF-LVL3-REACTOR".into(),
        name: "Generic Exothermic CSTR".into(),
        version: SchemaVersion::new(1, 0, 0),
        category: EquipmentCategory::Process,
        tier: CustomizationTier::Level3Scripted,
        description: "Custom dynamic CSTR reaction model".into(),
        parameters: IndexMap::new(),
        ports: vec![
            helper_port("feed", PortDirection::Inlet, PortDomain::Material),
            helper_port("product", PortDirection::Outlet, PortDomain::Material),
        ],
        curves: IndexMap::new(),
        curves_2d: IndexMap::new(),
        operating_envelope: OperatingEnvelope::default(),
        alarm_definitions: vec![],
        trips: vec![],
        interlocks: vec![],
        script: Some(ScriptDefinition {
            code: r#"
                fn step(inputs, params, state, dt) {
                    let c_in = inputs["c_in"];
                    let flow = inputs["flow"];
                    let vol = 10.0;
                    let k = 0.05;
                    let c_curr = state["conc"];
                    let dcdt = (flow * (c_in - c_curr) - k * c_curr * vol) / vol;
                    state["conc"] = max(c_curr + dcdt * dt, 0.0);
                    return state;
                }
            "#
            .into(),
            entry_point: "step".into(),
        }),
        composition: None,
        metadata: IndexMap::new(),
    };
    assert!(mgr.register_definition(def_lvl3).is_ok());

    // Verify all 3 tiers registered without recompiling
    assert_eq!(mgr.list_definitions().len(), 3);
}

// ============================================================================
// A2: RUNTIME LIFECYCLE & MULTI-INSTANCE ISOLATION
// ============================================================================

#[test]
fn test_stage_a2_runtime_lifecycle_and_multi_instance_isolation() {
    let mut mgr = CustomEquipmentManager::new();

    let mut def = CustomEquipmentDefinition {
        id: "DEF-PUMP-ISO".into(),
        name: "Centrifugal Pump Template".into(),
        version: SchemaVersion::new(1, 0, 0),
        category: EquipmentCategory::Rotating,
        tier: CustomizationTier::Level1Declarative,
        description: "Pump definition for isolation testing".into(),
        parameters: IndexMap::new(),
        ports: vec![
            helper_port("in", PortDirection::Inlet, PortDomain::Material),
            helper_port("out", PortDirection::Outlet, PortDomain::Material),
        ],
        curves: IndexMap::new(),
        curves_2d: IndexMap::new(),
        operating_envelope: OperatingEnvelope::default(),
        alarm_definitions: vec![],
        trips: vec![TripConditionDefinition {
            id: "TRIP-VIB".into(),
            name: "Vibration Trip".into(),
            condition_var: "vibration".into(),
            operator: TripOperator::GreaterThan,
            threshold: 8.0,
            fail_safe_state: "tripped".into(),
            latched: true,
        }],
        interlocks: vec![],
        script: None,
        composition: None,
        metadata: IndexMap::new(),
    };
    def.parameters.insert(
        "head_base".into(),
        helper_param("head_base", 100.0, 10.0, 500.0),
    );
    def.curves.insert(
        "head".into(),
        Curve1D::new(
            "HeadCurve",
            "flow",
            "delta_p",
            vec![(0.0, 100.0), (50.0, 80.0), (100.0, 40.0)],
        ),
    );

    mgr.register_definition(def).unwrap();

    // Instantiate 5 separate instances with distinct parameter overrides
    let mut ids = Vec::new();
    for i in 1..=5 {
        let id = EquipmentId::new();
        let mut overrides = BTreeMap::new();
        overrides.insert("head_base".into(), 100.0 * (i as f64));
        mgr.instantiate(id, "DEF-PUMP-ISO", format!("P-10{i}"), &overrides)
            .unwrap();
        ids.push(id);
    }

    // Verify initial lifecycle state: all stopped
    for id in &ids {
        let inst = mgr.get_instance(*id).unwrap();
        assert_eq!(inst.active_state, "stopped");
        assert!(!inst.is_running());
        assert!(!inst.is_tripped());
    }

    // Start instances 1, 2, 3; leave 4, 5 stopped
    mgr.start(ids[0]).unwrap();
    mgr.start(ids[1]).unwrap();
    mgr.start(ids[2]).unwrap();

    // Verify running states
    assert!(mgr.get_instance(ids[0]).unwrap().is_running());
    assert!(mgr.get_instance(ids[1]).unwrap().is_running());
    assert!(mgr.get_instance(ids[2]).unwrap().is_running());
    assert!(!mgr.get_instance(ids[3]).unwrap().is_running());
    assert!(!mgr.get_instance(ids[4]).unwrap().is_running());

    // Step simulation
    let mut inputs_by_eq = HashMap::new();
    for id in &ids {
        let mut in_map = BTreeMap::new();
        in_map.insert("flow".to_string(), 50.0);
        inputs_by_eq.insert(*id, in_map);
    }
    let mut tag_store = RuntimeTagStore::new();
    mgr.step(0.1, &inputs_by_eq, &mut tag_store).unwrap();

    // Running instances have non-zero delta_p (80.0); stopped have 0.0
    assert_eq!(
        mgr.get_instance(ids[0])
            .unwrap()
            .get_var("delta_p")
            .unwrap(),
        80.0
    );
    assert_eq!(
        mgr.get_instance(ids[1])
            .unwrap()
            .get_var("delta_p")
            .unwrap(),
        80.0
    );
    assert_eq!(
        mgr.get_instance(ids[2])
            .unwrap()
            .get_var("delta_p")
            .unwrap(),
        80.0
    );
    assert_eq!(
        mgr.get_instance(ids[3])
            .unwrap()
            .get_var("delta_p")
            .unwrap(),
        0.0
    );
    assert_eq!(
        mgr.get_instance(ids[4])
            .unwrap()
            .get_var("delta_p")
            .unwrap(),
        0.0
    );

    // Isolate mutation: modify state variable on instance 1 only
    mgr.get_instance_mut(ids[0])
        .unwrap()
        .set_var("custom_diagnostic", 42.0);
    assert_eq!(
        mgr.get_instance(ids[0])
            .unwrap()
            .get_var("custom_diagnostic")
            .unwrap(),
        42.0
    );
    assert_eq!(
        mgr.get_instance(ids[1])
            .unwrap()
            .get_var("custom_diagnostic"),
        None
    );

    // Trip instance 2 via vibration; ensure others remain unaffected
    mgr.get_instance_mut(ids[1])
        .unwrap()
        .set_var("vibration", 12.0);
    mgr.step(0.1, &inputs_by_eq, &mut tag_store).unwrap();

    assert!(!mgr.get_instance(ids[0]).unwrap().is_tripped());
    assert!(mgr.get_instance(ids[0]).unwrap().is_running());

    assert!(mgr.get_instance(ids[1]).unwrap().is_tripped());
    assert!(!mgr.get_instance(ids[1]).unwrap().is_running());
    assert_eq!(
        mgr.get_instance(ids[1])
            .unwrap()
            .get_var("delta_p")
            .unwrap(),
        0.0
    );

    assert!(!mgr.get_instance(ids[2]).unwrap().is_tripped());
    assert!(mgr.get_instance(ids[2]).unwrap().is_running());
}

// ============================================================================
// A3: RHAI SANDBOX ADVERSARIAL TESTING & QUOTA ENFORCEMENT
// ============================================================================

#[test]
fn test_stage_a3_rhai_sandbox_adversarial_security_and_quotas() {
    let sandbox = RhaiSandbox::new();

    // 1. Filesystem access attempt
    let script_fs = r#"
        fn step(inputs, params, state, dt) {
            let f = open("/etc/passwd");
            return state;
        }
    "#;
    let ast_fs = sandbox.compile_script(script_fs).unwrap();
    let ctx = ScriptExecutionContext::default();
    let res_fs = sandbox.execute_step(&ast_fs, "step", &ctx);
    assert!(
        res_fs.is_err(),
        "Host filesystem access must fail as undefined symbol"
    );

    // 2. Network connection attempt
    let script_net = r#"
        fn step(inputs, params, state, dt) {
            let socket = connect("127.0.0.1:8080");
            return state;
        }
    "#;
    let ast_net = sandbox.compile_script(script_net).unwrap();
    let res_net = sandbox.execute_step(&ast_net, "step", &ctx);
    assert!(
        res_net.is_err(),
        "Host network access must fail as undefined symbol"
    );

    // 3. OS/Shell execution attempt
    let script_os = r#"
        fn step(inputs, params, state, dt) {
            let res = system("rm -rf /");
            return state;
        }
    "#;
    let ast_os = sandbox.compile_script(script_os).unwrap();
    let res_os = sandbox.execute_step(&ast_os, "step", &ctx);
    assert!(
        res_os.is_err(),
        "Host shell execution must fail as undefined symbol"
    );

    // 4. Threading attempt (spawn is reserved/forbidden in sandboxed Rhai)
    let script_thread = r#"
        fn step(inputs, params, state, dt) {
            let t = spawn(|| { print("evil"); });
            return state;
        }
    "#;
    assert!(
        sandbox.compile_script(script_thread).is_err(),
        "Threading spawn must be rejected at compile time as forbidden/reserved keyword"
    );

    // 5. Forbidden keyword rejection during pre-scan
    let script_eval = "fn step(inputs, params, state, dt) { eval(\"2+2\"); return state; }";
    assert!(matches!(
        sandbox.compile_script(script_eval),
        Err(ScriptingError::Security(_))
    ));

    let script_import = "import \"secret\" as s; fn step(i,p,s,dt) { return s; }";
    assert!(matches!(
        sandbox.compile_script(script_import),
        Err(ScriptingError::Security(_))
    ));

    // 6. 50,000 instruction limit enforcement against infinite loop
    let script_loop = r#"
        fn step(inputs, params, state, dt) {
            let x = 0;
            while true {
                x += 1;
            }
            state["x"] = x;
            return state;
        }
    "#;
    let ast_loop = sandbox.compile_script(script_loop).unwrap();
    let res_loop = sandbox.execute_step(&ast_loop, "step", &ctx);
    assert!(
        matches!(res_loop, Err(ScriptingError::InstructionQuotaExceeded(_))),
        "Infinite loop must be terminated by the 50k instruction quota"
    );

    // 7. Stack depth 16 limit enforcement against deep recursion
    let script_recurse = r#"
        fn bomb(n) {
            return bomb(n + 1);
        }
        fn step(inputs, params, state, dt) {
            bomb(1);
            return state;
        }
    "#;
    let ast_recurse = sandbox.compile_script(script_recurse).unwrap();
    let res_recurse = sandbox.execute_step(&ast_recurse, "step", &ctx);
    assert!(
        matches!(res_recurse, Err(ScriptingError::ResourceLimit(_))),
        "Deep recursion must be caught by call stack limit"
    );

    // 8. Memory string size limit enforcement
    let script_str_bomb = r#"
        fn step(inputs, params, state, dt) {
            let s = "01234567890123456789012345678901"; // 32 bytes
            while true {
                s = s + s; // doubles: 64, 128, 256, 512...
            }
            state["s"] = s;
            return state;
        }
    "#;
    let ast_str = sandbox.compile_script(script_str_bomb).unwrap();
    let res_str = sandbox.execute_step(&ast_str, "step", &ctx);
    assert!(
        matches!(
            res_str,
            Err(ScriptingError::ResourceLimit(_))
                | Err(ScriptingError::InstructionQuotaExceeded(_))
        ),
        "Memory string explosion must be trapped by resource or instruction limit"
    );

    // 9. Floating point non-finite safety (NaN and Inf rejection)
    let script_inf = r#"
        fn step(inputs, params, state, dt) {
            state["bad"] = 1.0 / 0.0;
            return state;
        }
    "#;
    let ast_inf = sandbox.compile_script(script_inf).unwrap();
    let res_inf = sandbox.execute_step(&ast_inf, "step", &ctx);
    assert!(
        matches!(res_inf, Err(ScriptingError::Execution(_))),
        "Positive Infinity output must be rejected"
    );

    let script_nan = r#"
        fn step(inputs, params, state, dt) {
            state["bad"] = 0.0 / 0.0;
            return state;
        }
    "#;
    let ast_nan = sandbox.compile_script(script_nan).unwrap();
    let res_nan = sandbox.execute_step(&ast_nan, "step", &ctx);
    assert!(
        matches!(res_nan, Err(ScriptingError::Execution(_))),
        "NaN output must be rejected"
    );

    // 10. Math safety: sqrt(-1) clamped safely to 0.0
    let script_sqrt = r#"
        fn step(inputs, params, state, dt) {
            state["val"] = sqrt(-100.0);
            return state;
        }
    "#;
    let ast_sqrt = sandbox.compile_script(script_sqrt).unwrap();
    let out_sqrt = sandbox.execute_step(&ast_sqrt, "step", &ctx).unwrap();
    assert_eq!(*out_sqrt.variables.get("val").unwrap(), 0.0);
}

// ============================================================================
// A4: DETERMINISM & TEMPORAL INDEPENDENCE
// ============================================================================

#[test]
fn test_stage_a4_determinism_and_temporal_independence() {
    let sandbox = RhaiSandbox::new();

    // Dynamic accumulator model: dh/dt = (Fin - Fout) / A
    let script = r#"
        fn step(inputs, params, state, dt) {
            let f_in = inputs["f_in"];
            let f_out = inputs["f_out"];
            let area = params["area"];
            let h = state["height"];
            
            let dh = (f_in - f_out) / area * dt;
            let new_h = max(h + dh, 0.0);
            state["height"] = new_h;
            state["pressure"] = 9.81 * 1000.0 * new_h; // rho * g * h
            return state;
        }
    "#;
    let ast = sandbox.compile_script(script).unwrap();

    let mut ctx1 = ScriptExecutionContext {
        dt: 0.05,
        ..Default::default()
    };
    ctx1.parameters.insert("area".into(), 2.5);
    ctx1.inputs.insert("f_in".into(), 10.0);
    ctx1.inputs.insert("f_out".into(), 8.0);
    ctx1.state_variables.insert("height".into(), 1.0);
    ctx1.state_variables.insert("pressure".into(), 9810.0);

    let mut ctx2 = ctx1.clone();

    // Trajectory 1: 1,000 continuous ticks
    let mut traj1 = Vec::new();
    for _ in 0..1000 {
        let out = sandbox.execute_step(&ast, "step", &ctx1).unwrap();
        ctx1.state_variables = out.variables.clone();
        traj1.push((out.variables["height"], out.variables["pressure"]));
    }

    // Trajectory 2: 1,000 ticks with simulated wall-clock delays and pauses
    let mut traj2 = Vec::new();
    for i in 0..1000 {
        if i % 100 == 0 {
            std::thread::sleep(Duration::from_millis(1));
        }
        let out = sandbox.execute_step(&ast, "step", &ctx2).unwrap();
        ctx2.state_variables = out.variables.clone();
        traj2.push((out.variables["height"], out.variables["pressure"]));
    }

    assert_eq!(
        traj1.len(),
        traj2.len(),
        "Trajectories must have identical length"
    );
    for i in 0..1000 {
        assert_eq!(
            traj1[i].0.to_bits(),
            traj2[i].0.to_bits(),
            "Height mismatch at step {i}"
        );
        assert_eq!(
            traj1[i].1.to_bits(),
            traj2[i].1.to_bits(),
            "Pressure mismatch at step {i}"
        );
    }
}

// ============================================================================
// A5: CURVES AND OPERATING ENVELOPES
// ============================================================================

#[test]
fn test_stage_a5_curves_and_operating_envelopes_adversarial() {
    // 1. Monotonicity validation: reject non-monotonic X
    let curve_non_monotonic = Curve1D::new(
        "InvalidCurve",
        "x",
        "y",
        vec![(0.0, 10.0), (100.0, 50.0), (50.0, 30.0)],
    );
    assert!(
        curve_non_monotonic.validate().is_err(),
        "Non-monotonic X coordinates must be rejected"
    );

    // 2. Duplicate X coordinate rejection
    let curve_duplicate = Curve1D::new(
        "DuplicateX",
        "x",
        "y",
        vec![(0.0, 10.0), (50.0, 20.0), (50.0, 30.0), (100.0, 40.0)],
    );
    assert!(
        curve_duplicate.validate().is_err(),
        "Duplicate X coordinates must be rejected"
    );

    // 3. Insufficient points rejection
    let curve_single = Curve1D::new("SinglePoint", "x", "y", vec![(10.0, 20.0)]);
    assert!(
        curve_single.validate().is_err(),
        "Single-point curve must be rejected"
    );

    // 4. Fritsch-Carlson monotonic cubic Hermite spline evaluation
    let mut spline_curve = Curve1D::new(
        "MonotonicSpline",
        "x",
        "y",
        vec![(0.0, 0.0), (2.0, 10.0), (4.0, 10.0), (6.0, 30.0)],
    );
    spline_curve.interpolation = InterpolationMethod::MonotonicCubic;
    assert!(spline_curve.validate().is_ok());

    // Evaluate flat region (x between 2.0 and 4.0, where y1 = y2 = 10.0)
    // Fritsch-Carlson guarantees no overshoot: y must remain exactly 10.0
    for test_x in [2.2, 2.5, 3.0, 3.8] {
        let val = spline_curve.evaluate(test_x).unwrap();
        assert!(
            (val - 10.0).abs() < 1e-4,
            "Monotonic spline overshot in flat plateau: x={test_x}, y={val}"
        );
    }

    // 5. Boundary extrapolation clamping
    let val_below = spline_curve.evaluate(-10.0).unwrap();
    assert_eq!(val_below, 0.0, "Input below x_min must clamp to y(x_min)");

    let val_above = spline_curve.evaluate(100.0).unwrap();
    assert_eq!(val_above, 30.0, "Input above x_max must clamp to y(x_max)");

    // 6. Bilinear surface 2D interpolation
    let curve_2d = Curve2D {
        name: "CompressorEfficiencyMap".into(),
        input_x_var: "speed".into(),
        input_y_var: "flow".into(),
        output_var: "efficiency".into(),
        x_grid: vec![1000.0, 2000.0],
        y_grid: vec![10.0, 20.0],
        z_values: vec![
            vec![0.70, 0.75], // speed = 1000, flow = [10, 20]
            vec![0.80, 0.85], // speed = 2000, flow = [10, 20]
        ],
        extrapolation: ExtrapolationMode::Clamp,
    };
    assert!(curve_2d.validate().is_ok());

    // Center midpoint: speed = 1500, flow = 15 -> expected efficiency = 0.775
    let mid_eff = curve_2d.evaluate(1500.0, 15.0).unwrap();
    assert!((mid_eff - 0.775).abs() < 1e-6);

    // Clamping on 2D boundary
    let clamped_eff = curve_2d.evaluate(5000.0, 50.0).unwrap();
    assert_eq!(clamped_eff, 0.85);

    // 7. Operating Envelope validation (rejection of normal_min > normal_max)
    let bad_envelope = OperatingEnvelope {
        dimensions: vec![EnvelopeDimension {
            tag_id: TagId::new(),
            description: "Pressure envelope".into(),
            normal_min: 100.0,
            normal_max: 50.0, // Contradictory!
            warning_min: Some(0.0),
            warning_max: Some(200.0),
            critical_min: Some(0.0),
            critical_max: Some(250.0),
            trip_min: Some(0.0),
            trip_max: Some(300.0),
        }],
    };
    let bad_def = CustomEquipmentDefinition {
        id: "DEF-BAD-ENV".into(),
        name: "Bad Envelope Def".into(),
        version: SchemaVersion::new(1, 0, 0),
        category: EquipmentCategory::Rotating,
        tier: CustomizationTier::Level1Declarative,
        description: "Invalid envelope".into(),
        parameters: IndexMap::new(),
        ports: vec![
            helper_port("in", PortDirection::Inlet, PortDomain::Material),
            helper_port("out", PortDirection::Outlet, PortDomain::Material),
        ],
        curves: IndexMap::new(),
        curves_2d: IndexMap::new(),
        operating_envelope: bad_envelope,
        alarm_definitions: vec![],
        trips: vec![],
        interlocks: vec![],
        script: None,
        composition: None,
        metadata: IndexMap::new(),
    };
    assert!(
        validate_custom_equipment(&bad_def).is_err(),
        "Contradictory envelope (min > max) must fail validation"
    );
}

// ============================================================================
// A6: COMPOSITE PACKAGES AND SUBGRAPHS
// ============================================================================

#[test]
fn test_stage_a6_composite_packages_and_subgraphs() {
    let mut mgr = CustomEquipmentManager::new();

    // 1. Register child equipment definition (Valve)
    let def_valve = CustomEquipmentDefinition {
        id: "DEF-CHILD-VALVE".into(),
        name: "Isolation Valve".into(),
        version: SchemaVersion::new(1, 0, 0),
        category: EquipmentCategory::Valve,
        tier: CustomizationTier::Level1Declarative,
        description: "Child valve component".into(),
        parameters: IndexMap::new(),
        ports: vec![
            helper_port("inlet", PortDirection::Inlet, PortDomain::Material),
            helper_port("outlet", PortDirection::Outlet, PortDomain::Material),
        ],
        curves: IndexMap::new(),
        curves_2d: IndexMap::new(),
        operating_envelope: OperatingEnvelope::default(),
        alarm_definitions: vec![],
        trips: vec![],
        interlocks: vec![],
        script: None,
        composition: None,
        metadata: IndexMap::new(),
    };
    mgr.register_definition(def_valve).unwrap();

    // 2. Register child equipment definition (Pump)
    let def_pump = CustomEquipmentDefinition {
        id: "DEF-CHILD-PUMP".into(),
        name: "Booster Pump".into(),
        version: SchemaVersion::new(1, 0, 0),
        category: EquipmentCategory::Rotating,
        tier: CustomizationTier::Level1Declarative,
        description: "Child pump component".into(),
        parameters: IndexMap::new(),
        ports: vec![
            helper_port("suction", PortDirection::Inlet, PortDomain::Material),
            helper_port("discharge", PortDirection::Outlet, PortDomain::Material),
        ],
        curves: IndexMap::new(),
        curves_2d: IndexMap::new(),
        operating_envelope: OperatingEnvelope::default(),
        alarm_definitions: vec![],
        trips: vec![],
        interlocks: vec![],
        script: None,
        composition: None,
        metadata: IndexMap::new(),
    };
    mgr.register_definition(def_pump).unwrap();

    // 3. Valid composite package
    let valid_pkg = CustomEquipmentDefinition {
        id: "DEF-PKG-VALID".into(),
        name: "Valid Pumping Skid".into(),
        version: SchemaVersion::new(1, 0, 0),
        category: EquipmentCategory::Process,
        tier: CustomizationTier::Level2Composition,
        description: "Suction valve -> Booster pump".into(),
        parameters: IndexMap::new(),
        ports: vec![
            helper_port("skid_inlet", PortDirection::Inlet, PortDomain::Material),
            helper_port("skid_outlet", PortDirection::Outlet, PortDomain::Material),
        ],
        curves: IndexMap::new(),
        curves_2d: IndexMap::new(),
        operating_envelope: OperatingEnvelope::default(),
        alarm_definitions: vec![],
        trips: vec![],
        interlocks: vec![],
        script: None,
        composition: Some(CompositionDefinition {
            sub_components: vec![
                SubComponentDefinition {
                    id: "V101".into(),
                    template_id: "DEF-CHILD-VALVE".into(),
                    parameters: IndexMap::new(),
                },
                SubComponentDefinition {
                    id: "P101".into(),
                    template_id: "DEF-CHILD-PUMP".into(),
                    parameters: IndexMap::new(),
                },
            ],
            internal_connections: vec![InternalConnectionDefinition {
                from_component: "V101".into(),
                from_port: "outlet".into(),
                to_component: "P101".into(),
                to_port: "suction".into(),
            }],
            port_mappings: vec![
                PortMappingDefinition {
                    external_port_name: "skid_inlet".into(),
                    internal_component: "V101".into(),
                    internal_port: "inlet".into(),
                },
                PortMappingDefinition {
                    external_port_name: "skid_outlet".into(),
                    internal_component: "P101".into(),
                    internal_port: "discharge".into(),
                },
            ],
        }),
        metadata: IndexMap::new(),
    };
    assert!(mgr.register_definition(valid_pkg).is_ok());

    // 4. Adversarial package: references non-existent child in internal connection
    let bad_conn_pkg = CustomEquipmentDefinition {
        id: "DEF-PKG-BAD-CONN".into(),
        name: "Broken Skid".into(),
        version: SchemaVersion::new(1, 0, 0),
        category: EquipmentCategory::Process,
        tier: CustomizationTier::Level2Composition,
        description: "Connection references non-existent child".into(),
        parameters: IndexMap::new(),
        ports: vec![
            helper_port("in", PortDirection::Inlet, PortDomain::Material),
            helper_port("out", PortDirection::Outlet, PortDomain::Material),
        ],
        curves: IndexMap::new(),
        curves_2d: IndexMap::new(),
        operating_envelope: OperatingEnvelope::default(),
        alarm_definitions: vec![],
        trips: vec![],
        interlocks: vec![],
        script: None,
        composition: Some(CompositionDefinition {
            sub_components: vec![SubComponentDefinition {
                id: "V101".into(),
                template_id: "DEF-CHILD-VALVE".into(),
                parameters: IndexMap::new(),
            }],
            internal_connections: vec![InternalConnectionDefinition {
                from_component: "V101".into(),
                from_port: "outlet".into(),
                to_component: "NON_EXISTENT_PUMP".into(), // BAD!
                to_port: "suction".into(),
            }],
            port_mappings: vec![],
        }),
        metadata: IndexMap::new(),
    };
    assert!(
        validate_custom_equipment(&bad_conn_pkg).is_err(),
        "Internal connection to non-existent child must fail validation"
    );

    // 5. Adversarial package: port mapping references non-existent package port
    let bad_mapping_pkg = CustomEquipmentDefinition {
        id: "DEF-PKG-BAD-MAP".into(),
        name: "Broken Port Mapping".into(),
        version: SchemaVersion::new(1, 0, 0),
        category: EquipmentCategory::Process,
        tier: CustomizationTier::Level2Composition,
        description: "Mapping references non-existent package port".into(),
        parameters: IndexMap::new(),
        ports: vec![helper_port(
            "in",
            PortDirection::Inlet,
            PortDomain::Material,
        )],
        curves: IndexMap::new(),
        curves_2d: IndexMap::new(),
        operating_envelope: OperatingEnvelope::default(),
        alarm_definitions: vec![],
        trips: vec![],
        interlocks: vec![],
        script: None,
        composition: Some(CompositionDefinition {
            sub_components: vec![SubComponentDefinition {
                id: "V101".into(),
                template_id: "DEF-CHILD-VALVE".into(),
                parameters: IndexMap::new(),
            }],
            internal_connections: vec![],
            port_mappings: vec![PortMappingDefinition {
                external_port_name: "GHOST_PORT".into(), // BAD!
                internal_component: "V101".into(),
                internal_port: "inlet".into(),
            }],
        }),
        metadata: IndexMap::new(),
    };
    assert!(
        validate_custom_equipment(&bad_mapping_pkg).is_err(),
        "Port mapping with ghost package port must fail validation"
    );
}

// ============================================================================
// A7: CONTROL, ALARM, INTERLOCK AND TRIP INTEGRATION
// ============================================================================

#[test]
fn test_stage_a7_control_alarm_interlock_and_trip_integration() {
    let mut mgr = CustomEquipmentManager::new();

    let def = CustomEquipmentDefinition {
        id: "DEF-COMPRESSOR-PROTECT".into(),
        name: "Protected Centrifugal Compressor".into(),
        version: SchemaVersion::new(1, 0, 0),
        category: EquipmentCategory::Rotating,
        tier: CustomizationTier::Level1Declarative,
        description: "Compressor with surge interlock and overpressure trip".into(),
        parameters: IndexMap::new(),
        ports: vec![
            helper_port("suction", PortDirection::Inlet, PortDomain::Material),
            helper_port("discharge", PortDirection::Outlet, PortDomain::Material),
        ],
        curves: IndexMap::new(),
        curves_2d: IndexMap::new(),
        operating_envelope: OperatingEnvelope::default(),
        alarm_definitions: vec![],
        trips: vec![TripConditionDefinition {
            id: "TRIP-HIHI-DISCHARGE-P".into(),
            name: "High-High Discharge Pressure Trip".into(),
            condition_var: "discharge_p".into(),
            operator: TripOperator::GreaterThan,
            threshold: 150.0,
            fail_safe_state: "tripped".into(),
            latched: true,
        }],
        interlocks: vec![InterlockRuleDefinition {
            id: "INTLK-LOW-FLOW-BYPASS".into(),
            description: "Low Suction Flow Anti-Surge Recirculation".into(),
            cause_var: "suction_flow".into(),
            operator: TripOperator::LessThan,
            cause_threshold: 20.0,
            effect_var: "anti_surge_valve_open".into(),
            effect_value: 1.0,
        }],
        script: None,
        composition: None,
        metadata: IndexMap::new(),
    };
    mgr.register_definition(def).unwrap();

    let c_id = EquipmentId::new();
    mgr.instantiate(c_id, "DEF-COMPRESSOR-PROTECT", "K-101", &BTreeMap::new())
        .unwrap();

    // Start compressor
    mgr.start(c_id).unwrap();
    assert!(mgr.get_instance(c_id).unwrap().is_running());

    let empty_inputs = HashMap::new();
    let mut tag_store = RuntimeTagStore::new();

    // 1. Normal operation: suction_flow = 50.0, discharge_p = 100.0
    mgr.get_instance_mut(c_id)
        .unwrap()
        .set_var("suction_flow", 50.0);
    mgr.get_instance_mut(c_id)
        .unwrap()
        .set_var("discharge_p", 100.0);
    mgr.get_instance_mut(c_id)
        .unwrap()
        .set_var("anti_surge_valve_open", 0.0);

    mgr.step(0.1, &empty_inputs, &mut tag_store).unwrap();
    assert!(mgr.get_instance(c_id).unwrap().is_running());
    assert!(!mgr.get_instance(c_id).unwrap().is_tripped());
    assert_eq!(
        mgr.get_instance(c_id)
            .unwrap()
            .get_var("anti_surge_valve_open")
            .unwrap(),
        0.0
    );

    // 2. Interlock trigger: suction_flow drops below 20.0 -> anti_surge_valve_open becomes 1.0
    mgr.get_instance_mut(c_id)
        .unwrap()
        .set_var("suction_flow", 15.0);
    mgr.step(0.1, &empty_inputs, &mut tag_store).unwrap();
    assert!(mgr.get_instance(c_id).unwrap().is_running());
    assert_eq!(
        mgr.get_instance(c_id)
            .unwrap()
            .get_var("anti_surge_valve_open")
            .unwrap(),
        1.0,
        "Interlock must force anti-surge valve open"
    );

    // 3. Protective Trip trigger: discharge_p exceeds 150.0 (160.0)
    mgr.get_instance_mut(c_id)
        .unwrap()
        .set_var("discharge_p", 160.0);
    mgr.step(0.1, &empty_inputs, &mut tag_store).unwrap();

    let inst_tripped = mgr.get_instance(c_id).unwrap();
    assert!(inst_tripped.is_tripped());
    assert!(!inst_tripped.is_running());
    assert_eq!(inst_tripped.active_state, "tripped");
    assert_eq!(
        inst_tripped.latched_trips,
        vec!["TRIP-HIHI-DISCHARGE-P".to_string()]
    );

    // 4. Trip latching verification: discharge_p returns to safe level (90.0)
    mgr.get_instance_mut(c_id)
        .unwrap()
        .set_var("discharge_p", 90.0);
    mgr.step(0.1, &empty_inputs, &mut tag_store).unwrap();

    let inst_latched = mgr.get_instance(c_id).unwrap();
    assert!(
        inst_latched.is_tripped(),
        "Trip must remain latched even when condition returns to normal"
    );
    assert!(!inst_latched.is_running());

    // 5. Restart attempt blocked while tripped
    assert!(
        mgr.start(c_id).is_err(),
        "Cannot start equipment while trip is latched"
    );

    // 6. Reset rejection when condition is still unsafe
    mgr.get_instance_mut(c_id)
        .unwrap()
        .set_var("discharge_p", 180.0); // Active violation!
    let reset_denied = mgr.reset_trip(c_id).unwrap();
    assert!(
        !reset_denied,
        "Reset must be REJECTED when trip condition is actively violated"
    );
    assert!(mgr.get_instance(c_id).unwrap().is_tripped());

    // 7. Manual reset succeeds when conditions are safe
    mgr.get_instance_mut(c_id)
        .unwrap()
        .set_var("discharge_p", 90.0); // Safe!
    let reset_ok = mgr.reset_trip(c_id).unwrap();
    assert!(
        reset_ok,
        "Reset must SUCCEED when process variables are safe"
    );

    let inst_cleared = mgr.get_instance(c_id).unwrap();
    assert!(!inst_cleared.is_tripped());
    assert_eq!(inst_cleared.active_state, "stopped");

    // 8. Now restart succeeds
    assert!(mgr.start(c_id).is_ok());
    assert!(mgr.get_instance(c_id).unwrap().is_running());
}

// ============================================================================
// A8: RUNTIME AUTHORING & ZERO-RECOMPILATION PROOF
// ============================================================================

#[test]
fn test_stage_a8_runtime_authoring_zero_recompilation_proof() {
    let mut mgr = CustomEquipmentManager::new();

    // Raw JSON string simulating project data or REST payload
    let json_def = r#"
    {
        "id": "DEF-RUNTIME-FURNACE",
        "name": "Generic Process Fired Heater",
        "version": "1.0.0",
        "category": "heat_transfer",
        "tier": "level3_scripted",
        "description": "Fired heater defined purely in JSON at runtime without recompile",
        "parameters": {
            "duty_max_mw": {
                "name": "duty_max_mw",
                "description": "Maximum firing duty in MW",
                "quantity_kind": "power",
                "default_value": 25.0,
                "min_value": 0.0,
                "max_value": 100.0,
                "required": true
            },
            "eff_base": {
                "name": "eff_base",
                "description": "Base thermal efficiency",
                "quantity_kind": "power",
                "default_value": 0.88,
                "min_value": 0.5,
                "max_value": 0.98,
                "required": true
            }
        },
        "ports": [
            {
                "id": "01J00000000000000000000001",
                "name": "tube_inlet",
                "domain": "material",
                "direction": "inlet",
                "phase_compatibility": ["liquid", "gas"],
                "required": true,
                "cardinality": "exactly_one",
                "description": "Process fluid inlet"
            },
            {
                "id": "01J00000000000000000000002",
                "name": "tube_outlet",
                "domain": "material",
                "direction": "outlet",
                "phase_compatibility": ["liquid", "gas"],
                "required": true,
                "cardinality": "exactly_one",
                "description": "Heated process fluid outlet"
            }
        ],
        "curves": {},
        "curves_2d": {},
        "operating_envelope": {
            "dimensions": [
                {
                    "tag_id": "01J00000000000000000000003",
                    "description": "Skin temperature",
                    "normal_min": 200.0,
                    "normal_max": 650.0,
                    "warning_min": 100.0,
                    "warning_max": 750.0,
                    "critical_min": 80.0,
                    "critical_max": 780.0,
                    "trip_min": 50.0,
                    "trip_max": 800.0
                }
            ]
        },
        "alarm_definitions": [],
        "trips": [
            {
                "id": "TRIP-HIGH-SKIN-TEMP",
                "name": "Tube Skin High Temperature Trip",
                "condition_var": "skin_temp",
                "operator": "greater_than",
                "threshold": 800.0,
                "fail_safe_state": "tripped",
                "latched": true
            }
        ],
        "interlocks": [],
        "script": {
            "code": "fn step(inputs, params, state, dt) { let flow = inputs[\"flow\"]; let fuel = inputs[\"fuel_gas\"]; let max_duty = params[\"duty_max_mw\"]; let eff = params[\"eff_base\"]; let duty = fuel * max_duty * eff; let t_in = inputs[\"temp_in\"]; let cp = 2.4; let dt_temp = duty * 1000.0 / (flow * cp + 0.01); state[\"temp_out\"] = t_in + dt_temp; state[\"skin_temp\"] = state[\"temp_out\"] + 80.0; return state; }",
            "entry_point": "step"
        },
        "composition": null,
        "metadata": {}
    }
    "#;

    // 1. Parse JSON definition at runtime
    let def: CustomEquipmentDefinition =
        serde_json::from_str(json_def).expect("JSON must deserialize cleanly into definition");

    // 2. Validate definition
    assert!(validate_custom_equipment(&def).is_ok());

    // 3. Register definition in manager
    assert!(mgr.register_definition(def).is_ok());
    assert_eq!(mgr.list_definitions(), vec!["DEF-RUNTIME-FURNACE"]);

    // 4. Instantiate definition into running simulation
    let eq_id = EquipmentId::new();
    let mut overrides = BTreeMap::new();
    overrides.insert("duty_max_mw".into(), 30.0); // Override max duty to 30 MW
    mgr.instantiate(eq_id, "DEF-RUNTIME-FURNACE", "H-101", &overrides)
        .unwrap();

    // 5. Start instance
    mgr.start(eq_id).unwrap();
    assert!(mgr.get_instance(eq_id).unwrap().is_running());

    // 6. Step simulation with realistic process inputs
    let mut inputs_by_eq = HashMap::new();
    let mut furnace_inputs = BTreeMap::new();
    furnace_inputs.insert("flow".to_string(), 100.0); // 100 kg/s
    furnace_inputs.insert("fuel_gas".to_string(), 0.5); // 50% firing
    furnace_inputs.insert("temp_in".to_string(), 320.0); // 320 °C
    inputs_by_eq.insert(eq_id, furnace_inputs);

    let mut tag_store = RuntimeTagStore::new();
    mgr.step(0.1, &inputs_by_eq, &mut tag_store).unwrap();

    let inst = mgr.get_instance(eq_id).unwrap();
    let temp_out = inst.get_var("temp_out").unwrap();
    let skin_temp = inst.get_var("skin_temp").unwrap();

    // Verify calculated physical outputs
    // duty = 0.5 * 30.0 * 0.88 = 13.2 MW = 13200 kW
    // dt_temp = 13200 / (100 * 2.4 + 0.01) = 13200 / 240.01 ~= 54.997 °C
    // temp_out ~= 320 + 55 = 375 °C
    // skin_temp ~= 375 + 80 = 455 °C
    assert!((temp_out - 375.0).abs() < 1.0);
    assert!((skin_temp - 455.0).abs() < 1.0);
    assert!(!inst.is_tripped());

    // 7. Push firing to 100% and low flow (10 kg/s) to trigger protective trip
    let mut trip_inputs = HashMap::new();
    let mut trip_map = BTreeMap::new();
    trip_map.insert("flow".to_string(), 10.0); // Very low flow!
    trip_map.insert("fuel_gas".to_string(), 1.0); // 100% firing!
    trip_map.insert("temp_in".to_string(), 400.0);
    trip_inputs.insert(eq_id, trip_map);

    mgr.step(0.1, &trip_inputs, &mut tag_store).unwrap();

    let inst_after_trip = mgr.get_instance(eq_id).unwrap();
    assert!(
        inst_after_trip.is_tripped(),
        "High skin temp must trigger and latch protective trip"
    );
    assert!(!inst_after_trip.is_running());
    assert_eq!(
        inst_after_trip.latched_trips,
        vec!["TRIP-HIGH-SKIN-TEMP".to_string()]
    );
}
