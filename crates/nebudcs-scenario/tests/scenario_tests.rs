//! Comprehensive test suite for NebuDCS Scenario & Fault Injection Engine (Phase 9).

use nebudcs_control::pid::{PidConfig, PidDirection, PidMode};
use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_control::ControlSystem;
use nebudcs_historian::buffer::create_historian_channel;
use nebudcs_model::tag::{TagQuality, TagValue};
use nebudcs_scenario::*;
use nebudcs_types::{EquipmentId, SchemaVersion, SimulationTime, TagId};

fn create_test_tag_store() -> (RuntimeTagStore, TagId, TagId, TagId, TagId) {
    let mut store = RuntimeTagStore::new();
    let p1 = TagId::new();
    let t1 = TagId::new();
    let sp1 = TagId::new();
    let out1 = TagId::new();

    store.register_tag("PI-101", p1, TagValue::Float(30.0), TagQuality::Good);
    store.register_tag("TI-101", t1, TagValue::Float(150.0), TagQuality::Good);
    store.register_tag("LIC-101.SP", sp1, TagValue::Float(50.0), TagQuality::Good);
    store.register_tag("LIC-101.OUT", out1, TagValue::Float(50.0), TagQuality::Good);
    store.register_tag(
        "K-101.SPEED",
        TagId::new(),
        TagValue::Float(3000.0),
        TagQuality::Good,
    );
    store.register_tag(
        "K-101.STATUS",
        TagId::new(),
        TagValue::Integer(1),
        TagQuality::Good,
    );

    (store, p1, t1, sp1, out1)
}

#[test]
fn test_scenario_serialization_roundtrip() {
    let scenario = ScenarioDefinition {
        id: "SCN-TEST-001".into(),
        name: "Test Upsets".into(),
        description: "Verify serialization roundtrip".into(),
        author: "Chief Architect".into(),
        schema_version: SCENARIO_SCHEMA_VERSION,
        duration_limit_sec: Some(600.0),
        random_seed: 0x12345,
        initial_conditions: InitialConditions {
            tags: vec![TagInitialValue {
                tag: "PI-101".into(),
                value: 35.0,
            }],
            controllers: vec![ControllerInitialState {
                loop_name: "LIC-101".into(),
                mode: Some("AUTO".into()),
                sp: Some(55.0),
                manual_output: None,
            }],
            equipment: vec![EquipmentInitialState {
                equipment_name: "K-101".into(),
                speed_rpm: Some(2950.0),
                is_running: Some(true),
            }],
        },
        timeline: vec![
            ScenarioEvent {
                id: "EVT-1".into(),
                priority: 10,
                trigger: EventTrigger::Time { elapsed_sec: 10.0 },
                action: ScenarioAction::InjectFault(FaultDefinition {
                    id: "FLT-1".into(),
                    target: "PI-101".into(),
                    fault_type: FaultType::SensorBias { offset: 5.0 },
                    duration_sec: Some(30.0),
                }),
            },
            ScenarioEvent {
                id: "EVT-2".into(),
                priority: 5,
                trigger: EventTrigger::TagCondition {
                    tag: "PI-101".into(),
                    operator: ComparisonOp::GreaterThan,
                    value: 38.0,
                },
                action: ScenarioAction::EmitMarker {
                    label: "HI_PRESSURE".into(),
                    message: "Pressure exceeded 38 bar".into(),
                },
            },
        ],
        objectives: vec![ScenarioObjective {
            id: "OBJ-1".into(),
            description: "Maintain pressure within limits".into(),
            criteria: ObjectiveCriteria::TagWithinRange {
                tag: "PI-101".into(),
                min_val: 20.0,
                max_val: 45.0,
                hold_duration_sec: 20.0,
            },
            time_limit_sec: Some(300.0),
            weight: 100,
            mandatory: true,
        }],
        branches: Vec::new(),
    };

    let serialized = serde_json::to_string_pretty(&scenario).expect("Serialization failed");
    let deserialized: ScenarioDefinition =
        serde_json::from_str(&serialized).expect("Deserialization failed");

    assert_eq!(scenario, deserialized);
    assert!(validate_scenario(&deserialized).is_ok());
}

#[test]
fn test_validation_rejects_nan_infinity_and_invalid_bounds() {
    let mut scenario = ScenarioDefinition {
        id: "SCN-ERR".into(),
        name: "Invalid Scenario".into(),
        description: "Checking numerical safety".into(),
        author: "Auditor".into(),
        schema_version: SCENARIO_SCHEMA_VERSION,
        duration_limit_sec: Some(f64::NAN), // NaN rejected
        random_seed: 42,
        initial_conditions: InitialConditions::default(),
        timeline: Vec::new(),
        objectives: Vec::new(),
        branches: Vec::new(),
    };

    assert!(validate_scenario(&scenario).is_err());

    scenario.duration_limit_sec = Some(-10.0); // Negative rejected
    assert!(validate_scenario(&scenario).is_err());

    scenario.duration_limit_sec = Some(100.0);

    // Incompatible version rejected
    scenario.schema_version = SchemaVersion::new(99, 0, 0);
    assert!(matches!(
        validate_scenario(&scenario),
        Err(ScenarioError::IncompatibleVersion { .. })
    ));
    scenario.schema_version = SCENARIO_SCHEMA_VERSION;

    // Injecting NaN into fault offset rejected
    scenario.timeline.push(ScenarioEvent {
        id: "EVT-BAD".into(),
        priority: 1,
        trigger: EventTrigger::Time { elapsed_sec: 0.0 },
        action: ScenarioAction::InjectFault(FaultDefinition {
            id: "FLT-NAN".into(),
            target: "TI-101".into(),
            fault_type: FaultType::SensorBias { offset: f64::NAN },
            duration_sec: None,
        }),
    });
    assert!(validate_scenario(&scenario).is_err());
}

#[test]
fn test_deterministic_timeline_ordering_at_identical_timestamps() {
    let mut scheduler = TimelineScheduler::new(vec![
        ScenarioEvent {
            id: "EVT-LOW-PRIORITY".into(),
            priority: 1,
            trigger: EventTrigger::Time { elapsed_sec: 5.0 },
            action: ScenarioAction::EmitMarker {
                label: "M1".into(),
                message: "Low".into(),
            },
        },
        ScenarioEvent {
            id: "EVT-HIGH-PRIORITY".into(),
            priority: 100,
            trigger: EventTrigger::Time { elapsed_sec: 5.0 },
            action: ScenarioAction::EmitMarker {
                label: "M2".into(),
                message: "High".into(),
            },
        },
        ScenarioEvent {
            id: "EVT-MED-PRIORITY".into(),
            priority: 50,
            trigger: EventTrigger::Time { elapsed_sec: 5.0 },
            action: ScenarioAction::EmitMarker {
                label: "M3".into(),
                message: "Med".into(),
            },
        },
    ]);

    let (tag_store, ..) = create_test_tag_store();

    // At t=4.0, nothing fires
    let fired_4 = scheduler.step(4.0, &tag_store);
    assert!(fired_4.is_empty());

    // At t=5.0, all 3 fire, strictly in priority order: 100 -> 50 -> 1
    let fired_5 = scheduler.step(5.0, &tag_store);
    assert_eq!(fired_5.len(), 3);
    assert_eq!(fired_5[0].0, "EVT-HIGH-PRIORITY");
    assert_eq!(fired_5[1].0, "EVT-MED-PRIORITY");
    assert_eq!(fired_5[2].0, "EVT-LOW-PRIORITY");

    // Stepping again does not refire one-shot events
    let fired_again = scheduler.step(6.0, &tag_store);
    assert!(fired_again.is_empty());
}

#[test]
fn test_initial_conditions_application() {
    let (mut tag_store, p1, ..) = create_test_tag_store();
    let mut ctrl_sys = ControlSystem::new();
    let fv_eq = EquipmentId::new();
    let lic_pv = TagId::new();
    let lic_out = TagId::new();
    tag_store.register_tag(
        "LIC-101.PV",
        lic_pv,
        TagValue::Float(50.0),
        TagQuality::Good,
    );

    ctrl_sys.register_pid(
        lic_pv,
        lic_out,
        fv_eq,
        PidConfig {
            kp: 1.0,
            ti: 10.0,
            direction: PidDirection::Direct,
            ..Default::default()
        },
    );

    let mut engine = ScenarioEngine::new();
    let scenario = ScenarioDefinition {
        id: "SCN-INIT".into(),
        name: "Init Test".into(),
        description: "Checking initial condition application".into(),
        author: "Dev".into(),
        schema_version: SCENARIO_SCHEMA_VERSION,
        duration_limit_sec: None,
        random_seed: 42,
        initial_conditions: InitialConditions {
            tags: vec![TagInitialValue {
                tag: "PI-101".into(),
                value: 88.5,
            }],
            controllers: vec![ControllerInitialState {
                loop_name: "LIC-101.PV".into(),
                mode: Some("MANUAL".into()),
                sp: Some(65.0),
                manual_output: Some(42.0),
            }],
            equipment: vec![EquipmentInitialState {
                equipment_name: "K-101".into(),
                speed_rpm: Some(1500.0),
                is_running: Some(false),
            }],
        },
        timeline: Vec::new(),
        objectives: Vec::new(),
        branches: Vec::new(),
    };

    engine.load_scenario(scenario).unwrap();
    engine
        .arm_and_initialize(0.0, &mut tag_store, Some(&mut ctrl_sys))
        .unwrap();

    // Verify tag overridden
    assert_eq!(tag_store.get_value_float(p1), Some(88.5));

    // Verify controller set to Manual with output 42.0
    let pid = ctrl_sys.pid(lic_pv).unwrap();
    assert_eq!(pid.mode(), PidMode::Manual);
    assert_eq!(pid.manual_output(), 42.0);
    assert_eq!(pid.sp(), 65.0);

    // Verify equipment tags updated
    let speed_id = tag_store.resolve_name("K-101.SPEED").unwrap();
    let status_id = tag_store.resolve_name("K-101.STATUS").unwrap();
    assert_eq!(tag_store.get_value_float(speed_id), Some(1500.0));
    assert_eq!(tag_store.get_value(status_id), Some(&TagValue::Integer(0)));
}

#[test]
fn test_fault_lifecycle_and_physical_effects() {
    let (mut tag_store, p1, t1, ..) = create_test_tag_store();
    let mut fault_mgr = FaultManager::new(42);

    // 1. Sensor Bias
    fault_mgr
        .inject_fault(
            FaultDefinition {
                id: "FLT-BIAS".into(),
                target: "PI-101".into(),
                fault_type: FaultType::SensorBias { offset: 12.0 },
                duration_sec: Some(2.0),
            },
            0.0,
            &tag_store,
        )
        .unwrap();

    // 2. Sensor Frozen
    fault_mgr
        .inject_fault(
            FaultDefinition {
                id: "FLT-FROZEN".into(),
                target: "TI-101".into(),
                fault_type: FaultType::SensorFrozen { value: Some(99.0) },
                duration_sec: None, // permanent until cleared
            },
            0.0,
            &tag_store,
        )
        .unwrap();

    // Step at dt = 1.0s
    let expired = fault_mgr.step(1.0, &mut tag_store);
    assert!(expired.is_empty());
    assert_eq!(tag_store.get_value_float(p1), Some(30.0 + 12.0));
    assert_eq!(tag_store.get_value_float(t1), Some(99.0));

    // Step at dt = 1.0s (total elapsed = 2.0s for bias fault)
    let expired = fault_mgr.step(1.0, &mut tag_store);
    assert_eq!(expired, vec!["FLT-BIAS".to_string()]);

    // Manual clear of permanent frozen fault
    let cleared = fault_mgr.clear_fault("FLT-FROZEN", &mut tag_store);
    assert!(cleared);
    assert!(fault_mgr.is_empty());
}

#[test]
fn test_sensor_loss_of_signal_drops_tag_quality() {
    let (mut tag_store, p1, ..) = create_test_tag_store();
    let mut fault_mgr = FaultManager::new(42);

    fault_mgr
        .inject_fault(
            FaultDefinition {
                id: "FLT-LOS".into(),
                target: "PI-101".into(),
                fault_type: FaultType::SensorLossOfSignal,
                duration_sec: Some(1.0),
            },
            0.0,
            &tag_store,
        )
        .unwrap();

    fault_mgr.step(0.5, &mut tag_store);
    assert_eq!(tag_store.get_quality(p1), Some(TagQuality::Bad));

    // After expiration, quality is restored to Good
    let expired = fault_mgr.step(1.0, &mut tag_store);
    assert_eq!(expired, vec!["FLT-LOS".to_string()]);
    assert_eq!(tag_store.get_quality(p1), Some(TagQuality::Good));
}

#[test]
fn test_deterministic_disturbances() {
    let (mut tag_store, p1, ..) = create_test_tag_store();
    let mut dist_mgr = DisturbanceManager::new(42);

    dist_mgr.apply_disturbance(
        DisturbanceDefinition {
            id: "DST-RAMP".into(),
            target_tag: "PI-101".into(),
            disturbance_type: DisturbanceType::Ramp {
                target_value: 60.0,
                duration_sec: 10.0,
            },
            duration_sec: Some(10.0),
        },
        0.0,
        &tag_store,
    );

    // Initial baseline was 30.0. Ramp to 60.0 over 10s.
    // At t=5.0 (50%): expected value = 45.0
    let _ = dist_mgr.step(5.0, &mut tag_store);
    let val = tag_store.get_value_float(p1).unwrap();
    assert!((val - 45.0).abs() < 1e-6);

    // At t=10.0 (100%): expected value = 60.0 and expired
    let expired = dist_mgr.step(5.0, &mut tag_store);
    let val = tag_store.get_value_float(p1).unwrap();
    assert!((val - 60.0).abs() < 1e-6);
    assert_eq!(expired, vec!["DST-RAMP".to_string()]);
}

#[test]
fn test_deterministic_replay_identical_trajectories() {
    // Run two separate simulation scenarios with identical seed and verify identical states
    let run_scenario = || {
        let (mut tag_store, p1, ..) = create_test_tag_store();
        let mut engine = ScenarioEngine::new();

        let scenario = ScenarioDefinition {
            id: "SCN-REPLAY".into(),
            name: "Replay Proof".into(),
            description: "Determinism test".into(),
            author: "Auditor".into(),
            schema_version: SCENARIO_SCHEMA_VERSION,
            duration_limit_sec: Some(10.0),
            random_seed: 0xfeed_beef,
            initial_conditions: InitialConditions::default(),
            timeline: vec![ScenarioEvent {
                id: "EVT-NOISE".into(),
                priority: 1,
                trigger: EventTrigger::Time { elapsed_sec: 1.0 },
                action: ScenarioAction::InjectFault(FaultDefinition {
                    id: "FLT-NOISE".into(),
                    target: "PI-101".into(),
                    fault_type: FaultType::SensorDegradation {
                        noise_amplitude: 2.5,
                    },
                    duration_sec: Some(5.0),
                }),
            }],
            objectives: Vec::new(),
            branches: Vec::new(),
        };

        engine.load_scenario(scenario).unwrap();
        engine
            .arm_and_initialize(0.0, &mut tag_store, None)
            .unwrap();
        engine.start().unwrap();

        let mut trajectory = Vec::new();
        let dt = 0.1;
        for i in 0..50 {
            let sim_time = SimulationTime::try_new(i as f64 * dt).unwrap();
            engine.step(sim_time, dt, &mut tag_store, None, None, None);
            trajectory.push(tag_store.get_value_float(p1).unwrap());
        }
        trajectory
    };

    let trajectory1 = run_scenario();
    let trajectory2 = run_scenario();

    assert_eq!(trajectory1.len(), trajectory2.len());
    for (idx, (v1, v2)) in trajectory1.into_iter().zip(trajectory2).enumerate() {
        assert_eq!(
            v1.to_bits(),
            v2.to_bits(),
            "Non-deterministic divergence at step {idx}: {v1} vs {v2}"
        );
    }
}

#[test]
fn test_historian_event_logging() {
    let (tx, rx) = create_historian_channel(100);
    let (mut tag_store, ..) = create_test_tag_store();

    let mut engine = ScenarioEngine::new();
    let scenario = ScenarioDefinition {
        id: "SCN-HIST".into(),
        name: "Historian Log".into(),
        description: "Checking event dispatch".into(),
        author: "Dev".into(),
        schema_version: SCENARIO_SCHEMA_VERSION,
        duration_limit_sec: Some(5.0),
        random_seed: 42,
        initial_conditions: InitialConditions::default(),
        timeline: vec![ScenarioEvent {
            id: "EVT-MARK".into(),
            priority: 1,
            trigger: EventTrigger::Time { elapsed_sec: 1.0 },
            action: ScenarioAction::EmitMarker {
                label: "CHECKPOINT_ALPHA".into(),
                message: "Trainee reached Alpha".into(),
            },
        }],
        objectives: Vec::new(),
        branches: Vec::new(),
    };

    engine.load_scenario(scenario).unwrap();
    engine
        .arm_and_initialize(0.0, &mut tag_store, None)
        .unwrap();
    engine.start().unwrap();

    let sim_time = SimulationTime::try_new(1.0).unwrap();
    engine.step(sim_time, 1.0, &mut tag_store, None, None, Some(&tx));

    // Drain events from historian channel
    let mut logged_events = Vec::new();
    while let Ok(rec) = rx.try_recv() {
        if let nebudcs_historian::HistorianRecord::Event(ev) = rec {
            logged_events.push(ev);
        }
    }

    assert!(!logged_events.is_empty());
    let marker_event = logged_events
        .iter()
        .find(|e| e.event_type == "SCENARIO_MARKER")
        .expect("Marker event was not logged");
    assert_eq!(marker_event.source_id, "CHECKPOINT_ALPHA");
    assert_eq!(marker_event.new_state, "Trainee reached Alpha");
}

#[test]
fn test_objective_scoring_and_completion() {
    let (tag_store, ..) = create_test_tag_store();
    let mut evaluator = ObjectiveEvaluator::new(vec![
        ScenarioObjective {
            id: "OBJ-RANGE".into(),
            description: "Keep pressure within [25, 35] for 3 seconds".into(),
            criteria: ObjectiveCriteria::TagWithinRange {
                tag: "PI-101".into(),
                min_val: 25.0,
                max_val: 35.0,
                hold_duration_sec: 3.0,
            },
            time_limit_sec: Some(10.0),
            weight: 60,
            mandatory: true,
        },
        ScenarioObjective {
            id: "OBJ-TARGET".into(),
            description: "Reach pressure >= 30".into(),
            criteria: ObjectiveCriteria::TagReachesValue {
                tag: "PI-101".into(),
                operator: ComparisonOp::GreaterThanOrEqual,
                target_val: 30.0,
            },
            time_limit_sec: None,
            weight: 40,
            mandatory: false,
        },
    ]);

    // Initial pressure is 30.0 (satisfies TARGET immediately, and in range for RANGE)
    evaluator.step(1.0, 1.0, &tag_store, None);
    assert_eq!(evaluator.score(), 40.0); // TARGET satisfied (40%), RANGE pending (1/3s)

    evaluator.step(2.0, 1.0, &tag_store, None);
    evaluator.step(3.0, 1.0, &tag_store, None); // 3 seconds held!
    assert_eq!(evaluator.score(), 100.0); // All 100% satisfied!
    assert!(evaluator.all_mandatory_satisfied());
}
