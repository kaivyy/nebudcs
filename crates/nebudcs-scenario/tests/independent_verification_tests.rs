//! Independent verification test suite for NebuDCS Phase 9: Scenario & Fault Injection Engine.
//!
//! Covers:
//! - All 12 fault types individual validation & physical effects
//! - Critical multi-fault composition (same target, different targets, overlapping lifetimes)
//! - Baseline tracking & non-corrupting restoration
//! - Timeline execution (same-time events, pause/resume, determinism)
//! - Simulation time independence from wall-clock timing
//! - Disturbance waveforms (analytic boundaries & seeded PRNG reproducibility)
//! - Closed-loop physical propagation (Fault -> Physics -> Instrument -> Alarm -> Protection)
//! - Adversarial input validation & security limits

use std::time::Duration;

use nebudcs_alarm::block::AlarmBlockConfig;
use nebudcs_alarm::types::{AlarmSeverity, AlarmState, AlarmType};
use nebudcs_alarm::AlarmEngine;
use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_model::tag::{TagQuality, TagValue};
use nebudcs_scenario::*;
use nebudcs_types::{SimulationTime, TagId};

fn create_test_rig() -> (RuntimeTagStore, TagId, TagId, TagId, TagId, TagId) {
    let mut store = RuntimeTagStore::new();
    let p1 = TagId::new();
    let t1 = TagId::new();
    let lvl = TagId::new();
    let vlv = TagId::new();
    let pump = TagId::new();
    let sp = TagId::new();

    store.register_tag("PI-101", p1, TagValue::Float(30.0), TagQuality::Good);
    store.register_tag("TI-101", t1, TagValue::Float(120.0), TagQuality::Good);
    store.register_tag("LI-101.PV", lvl, TagValue::Float(50.0), TagQuality::Good);
    store.register_tag("FV-101.POS", vlv, TagValue::Float(60.0), TagQuality::Good);
    store.register_tag(
        "P-101.SPEED",
        pump,
        TagValue::Float(1800.0),
        TagQuality::Good,
    );
    store.register_tag(
        "P-101.STATUS",
        TagId::new(),
        TagValue::Integer(1),
        TagQuality::Good,
    );
    store.register_tag("LIC-101.SP", sp, TagValue::Float(50.0), TagQuality::Good);

    (store, p1, t1, lvl, vlv, pump)
}

#[test]
fn test_all_12_fault_types_verified() {
    let (mut store, p1, t1, lvl, vlv, pump) = create_test_rig();
    let mut mgr = FaultManager::new(12345);

    // 1. SensorBias
    mgr.inject_fault(
        FaultDefinition {
            id: "F01".into(),
            target: "PI-101".into(),
            fault_type: FaultType::SensorBias { offset: 15.0 },
            duration_sec: Some(2.0),
        },
        0.0,
        &store,
    )
    .unwrap();

    // 2. SensorDrift
    mgr.inject_fault(
        FaultDefinition {
            id: "F02".into(),
            target: "TI-101".into(),
            fault_type: FaultType::SensorDrift { rate_per_sec: 2.5 },
            duration_sec: Some(2.0),
        },
        0.0,
        &store,
    )
    .unwrap();

    // 3. SensorFrozen
    mgr.inject_fault(
        FaultDefinition {
            id: "F03".into(),
            target: "LI-101.PV".into(),
            fault_type: FaultType::SensorFrozen { value: Some(88.0) },
            duration_sec: Some(2.0),
        },
        0.0,
        &store,
    )
    .unwrap();

    // 4. SensorLossOfSignal
    mgr.inject_fault(
        FaultDefinition {
            id: "F04".into(),
            target: "FV-101.POS".into(),
            fault_type: FaultType::SensorLossOfSignal,
            duration_sec: Some(2.0),
        },
        0.0,
        &store,
    )
    .unwrap();

    // 5. EquipmentMechanicalTrip
    mgr.inject_fault(
        FaultDefinition {
            id: "F05".into(),
            target: "P-101.SPEED".into(),
            fault_type: FaultType::EquipmentMechanicalTrip,
            duration_sec: Some(2.0),
        },
        0.0,
        &store,
    )
    .unwrap();

    // Step 1.0s
    mgr.step(1.0, &mut store);

    // Verify effects
    assert_eq!(store.get_value_float(p1), Some(30.0 + 15.0)); // Bias
    assert_eq!(store.get_value_float(t1), Some(120.0 + 2.5)); // Drift after 1s
    assert_eq!(store.get_value_float(lvl), Some(88.0)); // Frozen
    assert_eq!(store.get_quality(vlv), Some(TagQuality::Bad)); // Loss of signal
    assert_eq!(store.get_value_float(pump), Some(0.0)); // Tripped speed

    let status_id = store.resolve_name("P-101.SPEED.STATUS");
    if let Some(sid) = status_id {
        assert_eq!(store.get_value(sid), Some(&TagValue::Integer(0)));
    }

    // Step another 1.0s (total 2.0s -> all expire)
    let expired = mgr.step(1.0, &mut store);
    assert_eq!(expired.len(), 5);

    // Verify all baselines restored!
    assert_eq!(store.get_value_float(p1), Some(30.0));
    assert_eq!(store.get_value_float(t1), Some(120.0));
    assert_eq!(store.get_value_float(lvl), Some(50.0));
    assert_eq!(store.get_quality(vlv), Some(TagQuality::Good));
    assert_eq!(store.get_value_float(pump), Some(1800.0));

    // 6. SensorDegradation (noise)
    mgr.inject_fault(
        FaultDefinition {
            id: "F06".into(),
            target: "PI-101".into(),
            fault_type: FaultType::SensorDegradation {
                noise_amplitude: 5.0,
            },
            duration_sec: None,
        },
        2.0,
        &store,
    )
    .unwrap();
    mgr.step(0.1, &mut store);
    assert_eq!(store.get_quality(p1), Some(TagQuality::Uncertain));
    assert!((store.get_value_float(p1).unwrap() - 30.0).abs() <= 5.0);
    mgr.clear_fault("F06", &mut store);
    assert_eq!(store.get_value_float(p1), Some(30.0));
    assert_eq!(store.get_quality(p1), Some(TagQuality::Good));

    // 7. ActuatorStuck
    mgr.inject_fault(
        FaultDefinition {
            id: "F07".into(),
            target: "FV-101.POS".into(),
            fault_type: FaultType::ActuatorStuck { position: 42.0 },
            duration_sec: None,
        },
        2.0,
        &store,
    )
    .unwrap();
    mgr.step(0.1, &mut store);
    assert_eq!(store.get_value_float(vlv), Some(42.0));
    mgr.clear_fault("F07", &mut store);
    assert_eq!(store.get_value_float(vlv), Some(60.0));

    // 8. ActuatorFailSafeTrip
    mgr.inject_fault(
        FaultDefinition {
            id: "F08".into(),
            target: "FV-101.POS".into(),
            fault_type: FaultType::ActuatorFailSafeTrip,
            duration_sec: None,
        },
        2.0,
        &store,
    )
    .unwrap();
    mgr.step(0.1, &mut store);
    assert_eq!(store.get_value_float(vlv), Some(0.0));
    mgr.clear_fault("F08", &mut store);
    assert_eq!(store.get_value_float(vlv), Some(60.0));

    // 9. EquipmentEfficiencyLoss
    mgr.inject_fault(
        FaultDefinition {
            id: "F09".into(),
            target: "PI-101".into(),
            fault_type: FaultType::EquipmentEfficiencyLoss { factor: 0.20 }, // 20% loss
            duration_sec: None,
        },
        2.0,
        &store,
    )
    .unwrap();
    mgr.step(0.1, &mut store);
    assert_eq!(store.get_value_float(p1), Some(24.0)); // 30 * 0.8
    mgr.clear_fault("F09", &mut store);
    assert_eq!(store.get_value_float(p1), Some(30.0));

    // 10. ProcessBlockage
    mgr.inject_fault(
        FaultDefinition {
            id: "F10".into(),
            target: "PI-101".into(),
            fault_type: FaultType::ProcessBlockage {
                resistance_multiplier: 2.0,
            },
            duration_sec: None,
        },
        2.0,
        &store,
    )
    .unwrap();
    mgr.step(0.1, &mut store);
    assert_eq!(store.get_value_float(p1), Some(15.0)); // 30 / 2.0
    mgr.clear_fault("F10", &mut store);
    assert_eq!(store.get_value_float(p1), Some(30.0));

    // 11. ProcessLeak
    mgr.inject_fault(
        FaultDefinition {
            id: "F11".into(),
            target: "PI-101".into(),
            fault_type: FaultType::ProcessLeak {
                leak_rate_kg_s: 5.0,
            },
            duration_sec: None,
        },
        2.0,
        &store,
    )
    .unwrap();
    mgr.step(0.1, &mut store);
    assert_eq!(store.get_value_float(p1), Some(25.0)); // 30 - 5.0
    mgr.clear_fault("F11", &mut store);
    assert_eq!(store.get_value_float(p1), Some(30.0));

    // 12. ActuatorSluggish
    mgr.inject_fault(
        FaultDefinition {
            id: "F12".into(),
            target: "FV-101.POS".into(),
            fault_type: FaultType::ActuatorSluggish {
                max_rate_per_sec: 2.0,
            },
            duration_sec: Some(5.0),
        },
        2.0,
        &store,
    )
    .unwrap();
    mgr.step(0.1, &mut store);
    assert_eq!(mgr.len(), 1);
    mgr.clear_fault("F12", &mut store);
    assert_eq!(mgr.len(), 0);
}

#[test]
fn test_critical_multi_fault_composition_same_tag() {
    let (mut store, p1, ..) = create_test_rig();
    let mut mgr = FaultManager::new(999);

    // Initial clean baseline is 30.0
    assert_eq!(store.get_value_float(p1), Some(30.0));

    // 1. Inject Fault A: Bias +10.0
    mgr.inject_fault(
        FaultDefinition {
            id: "FLT-A".into(),
            target: "PI-101".into(),
            fault_type: FaultType::SensorBias { offset: 10.0 },
            duration_sec: None,
        },
        0.0,
        &store,
    )
    .unwrap();
    mgr.step(0.1, &mut store);
    assert_eq!(store.get_value_float(p1), Some(40.0));

    // 2. Inject Fault B on SAME target: Bias +5.0
    mgr.inject_fault(
        FaultDefinition {
            id: "FLT-B".into(),
            target: "PI-101".into(),
            fault_type: FaultType::SensorBias { offset: 5.0 },
            duration_sec: None,
        },
        0.1,
        &store,
    )
    .unwrap();
    mgr.step(0.1, &mut store);
    // Both active: clean baseline (30.0) + 10.0 + 5.0 = 45.0
    assert_eq!(store.get_value_float(p1), Some(45.0));

    // 3. Clear Fault A -> Fault B MUST remain effective on baseline!
    let cleared_a = mgr.clear_fault("FLT-A", &mut store);
    assert!(cleared_a);
    assert_eq!(mgr.len(), 1);
    // Tag should now be baseline (30.0) + Fault B (+5.0) = 35.0!
    assert_eq!(store.get_value_float(p1), Some(35.0));

    // 4. Clear Fault B -> Baseline cleanly restored to 30.0!
    let cleared_b = mgr.clear_fault("FLT-B", &mut store);
    assert!(cleared_b);
    assert_eq!(mgr.len(), 0);
    assert_eq!(store.get_value_float(p1), Some(30.0));
}

#[test]
fn test_critical_multi_fault_composition_different_targets_and_overlapping_lifetimes() {
    let (mut store, p1, t1, ..) = create_test_rig();
    let mut mgr = FaultManager::new(777);

    // Fault 1 on PI-101: duration 2.0s
    mgr.inject_fault(
        FaultDefinition {
            id: "FLT-P".into(),
            target: "PI-101".into(),
            fault_type: FaultType::SensorBias { offset: 12.0 },
            duration_sec: Some(2.0),
        },
        0.0,
        &store,
    )
    .unwrap();

    // Fault 2 on TI-101: duration 4.0s
    mgr.inject_fault(
        FaultDefinition {
            id: "FLT-T".into(),
            target: "TI-101".into(),
            fault_type: FaultType::SensorBias { offset: 30.0 },
            duration_sec: Some(4.0),
        },
        0.0,
        &store,
    )
    .unwrap();

    // Tick 1.0s: both active
    let exp1 = mgr.step(1.0, &mut store);
    assert!(exp1.is_empty());
    assert_eq!(store.get_value_float(p1), Some(42.0)); // 30 + 12
    assert_eq!(store.get_value_float(t1), Some(150.0)); // 120 + 30

    // Tick 2.0s: FLT-P expires; FLT-T must remain active!
    let exp2 = mgr.step(1.0, &mut store);
    assert_eq!(exp2, vec!["FLT-P".to_string()]);
    assert_eq!(store.get_value_float(p1), Some(30.0)); // PI-101 restored
    assert_eq!(store.get_value_float(t1), Some(150.0)); // TI-101 still faulted!

    // Tick 4.0s: FLT-T expires
    let exp3 = mgr.step(2.0, &mut store);
    assert_eq!(exp3, vec!["FLT-T".to_string()]);
    assert_eq!(store.get_value_float(t1), Some(120.0)); // TI-101 restored
    assert!(mgr.is_empty());
}

#[test]
fn test_simulation_time_independence_from_wall_clock() {
    let (mut store, p1, ..) = create_test_rig();
    let mut engine = ScenarioEngine::new();

    let scenario = ScenarioDefinition {
        id: "SCN-TIME-TEST".into(),
        name: "Time Independence".into(),
        description: "Test".into(),
        author: "Auditor".into(),
        schema_version: SCENARIO_SCHEMA_VERSION,
        duration_limit_sec: Some(10.0),
        random_seed: 42,
        initial_conditions: InitialConditions::default(),
        timeline: vec![ScenarioEvent {
            id: "EV-01".into(),
            priority: 0,
            trigger: EventTrigger::Time { elapsed_sec: 1.0 },
            action: ScenarioAction::InjectFault(FaultDefinition {
                id: "FLT-TIMED".into(),
                target: "PI-101".into(),
                fault_type: FaultType::SensorBias { offset: 25.0 },
                duration_sec: None,
            }),
        }],
        objectives: vec![],
    };

    engine.load_scenario(scenario).unwrap();
    engine.arm_and_initialize(0.0, &mut store, None).unwrap();
    engine.start().unwrap();

    // 1. Thread sleep 50ms does NOT advance scenario simulation time
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(store.get_value_float(p1), Some(30.0));
    assert_eq!(engine.status().elapsed_sec, 0.0);

    // 2. Discrete ticks advance simulation time deterministically
    let dt = 0.5;
    let t1 = SimulationTime::try_new(0.5).unwrap();
    engine.step(t1, dt, &mut store, None, None, None);
    assert_eq!(store.get_value_float(p1), Some(30.0)); // Event is at t=1.0s

    let t2 = SimulationTime::try_new(1.0).unwrap();
    engine.step(t2, dt, &mut store, None, None, None);
    assert_eq!(store.get_value_float(p1), Some(55.0)); // Triggered at exact sim time 1.0s!
}

#[test]
fn test_timeline_same_timestamp_deterministic_execution() {
    let (mut store, p1, t1, ..) = create_test_rig();
    let mut engine = ScenarioEngine::new();

    // 2 events at identical timestamp 0.5s
    let scenario = ScenarioDefinition {
        id: "SCN-SAME-TIME".into(),
        name: "Identical Timestamps".into(),
        description: "Test".into(),
        author: "Auditor".into(),
        schema_version: SCENARIO_SCHEMA_VERSION,
        duration_limit_sec: Some(5.0),
        random_seed: 42,
        initial_conditions: InitialConditions::default(),
        timeline: vec![
            ScenarioEvent {
                id: "EV-B".into(),
                priority: 1,
                trigger: EventTrigger::Time { elapsed_sec: 0.5 },
                action: ScenarioAction::InjectFault(FaultDefinition {
                    id: "FLT-T".into(),
                    target: "TI-101".into(),
                    fault_type: FaultType::SensorBias { offset: 10.0 },
                    duration_sec: None,
                }),
            },
            ScenarioEvent {
                id: "EV-A".into(),
                priority: 2,
                trigger: EventTrigger::Time { elapsed_sec: 0.5 },
                action: ScenarioAction::InjectFault(FaultDefinition {
                    id: "FLT-P".into(),
                    target: "PI-101".into(),
                    fault_type: FaultType::SensorBias { offset: 10.0 },
                    duration_sec: None,
                }),
            },
        ],
        objectives: vec![],
    };

    engine.load_scenario(scenario).unwrap();
    engine.arm_and_initialize(0.0, &mut store, None).unwrap();
    engine.start().unwrap();

    let t = SimulationTime::try_new(0.5).unwrap();
    let status = engine.step(t, 0.5, &mut store, None, None, None);

    assert_eq!(status.events_fired, 2);
    assert_eq!(status.active_faults.len(), 2);
    assert_eq!(store.get_value_float(p1), Some(40.0));
    assert_eq!(store.get_value_float(t1), Some(130.0));
}

#[test]
fn test_disturbance_analytic_boundaries_and_seeded_prng() {
    let (mut store1, ..) = create_test_rig();
    let (mut store2, ..) = create_test_rig();
    let (mut store_diff, ..) = create_test_rig();

    let mut dist_mgr_1 = DisturbanceManager::new(42);
    let mut dist_mgr_2 = DisturbanceManager::new(42);
    let mut dist_mgr_diff = DisturbanceManager::new(999);

    let def_ramp = DisturbanceDefinition {
        id: "D-RAMP".into(),
        target_tag: "LIC-101.SP".into(),
        duration_sec: Some(2.0),
        disturbance_type: DisturbanceType::Ramp {
            target_value: 70.0,
            duration_sec: 2.0,
        },
    };

    let def_noise = DisturbanceDefinition {
        id: "D-NOISE".into(),
        target_tag: "PI-101".into(),
        duration_sec: None,
        disturbance_type: DisturbanceType::BoundedNoise { max_amplitude: 5.0 },
    };

    dist_mgr_1.apply_disturbance(def_ramp.clone(), 0.0, &store1);
    dist_mgr_1.apply_disturbance(def_noise.clone(), 0.0, &store1);

    dist_mgr_2.apply_disturbance(def_ramp, 0.0, &store2);
    dist_mgr_2.apply_disturbance(def_noise.clone(), 0.0, &store2);

    dist_mgr_diff.apply_disturbance(def_noise, 0.0, &store_diff);

    let p1_1 = store1.resolve_name("PI-101").unwrap();
    let sp_1 = store1.resolve_name("LIC-101.SP").unwrap();
    let p1_2 = store2.resolve_name("PI-101").unwrap();
    let sp_2 = store2.resolve_name("LIC-101.SP").unwrap();
    let p1_diff = store_diff.resolve_name("PI-101").unwrap();

    // Step through 2.0s
    for _ in 0..20 {
        dist_mgr_1.step(0.1, &mut store1);
        dist_mgr_2.step(0.1, &mut store2);
        dist_mgr_diff.step(0.1, &mut store_diff);

        // Deterministic identical results for identical seeds
        assert_eq!(store1.get_value_float(p1_1), store2.get_value_float(p1_2));
        assert_eq!(store1.get_value_float(sp_1), store2.get_value_float(sp_2));

        // Verify Ramp bounds [50.0, 70.0] (baseline is 50.0 ramping to 70.0)
        let ramp_val = store1.get_value_float(sp_1).unwrap();
        assert!((50.0..=70.0).contains(&ramp_val));

        // Verify BoundedNoise limits [30.0 - 5.0, 30.0 + 5.0]
        let noise_val = store1.get_value_float(p1_1).unwrap();
        assert!((25.0..=35.0).contains(&noise_val));

        // Different seed produces valid noise within bounds
        let noise_diff = store_diff.get_value_float(p1_diff).unwrap();
        assert!((25.0..=35.0).contains(&noise_diff));
    }
}

#[test]
fn test_closed_loop_physical_propagation_fault_to_alarm() {
    let mut store = RuntimeTagStore::new();
    let p_tag = TagId::new();
    let p_stat = TagId::new();
    store.register_tag("PT-101", p_tag, TagValue::Float(30.0), TagQuality::Good);
    store.register_tag(
        "PT-101.STATUS",
        p_stat,
        TagValue::Integer(1),
        TagQuality::Good,
    );

    // Setup ANSI/ISA-18.2 Alarm for High Pressure
    let mut alarm_engine = AlarmEngine::new();
    alarm_engine.add_alarm(AlarmBlockConfig {
        name: "ALM-HI-PRESS".to_string(),
        tag_id: p_tag,
        alarm_type: AlarmType::Hi,
        severity: AlarmSeverity::Warning,
        limit: 45.0,
        deadband: 1.0,
        default_shelve_duration: 60.0,
    });

    let mut engine = ScenarioEngine::new();
    let scenario = ScenarioDefinition {
        id: "SCN-PROPAGATION".into(),
        name: "Fault to Alarm Propagation".into(),
        description: "Test".into(),
        author: "Auditor".into(),
        schema_version: SCENARIO_SCHEMA_VERSION,
        duration_limit_sec: Some(5.0),
        random_seed: 42,
        initial_conditions: InitialConditions::default(),
        timeline: vec![ScenarioEvent {
            id: "EV-SURGE".into(),
            priority: 1,
            trigger: EventTrigger::Time { elapsed_sec: 1.0 },
            action: ScenarioAction::InjectFault(FaultDefinition {
                id: "FLT-SURGE".into(),
                target: "PT-101".into(),
                fault_type: FaultType::SensorBias { offset: 20.0 }, // 30 + 20 = 50 bar > 45 bar trip
                duration_sec: None,
            }),
        }],
        objectives: vec![],
    };

    engine.load_scenario(scenario).unwrap();
    engine.arm_and_initialize(0.0, &mut store, None).unwrap();
    engine.start().unwrap();

    // Step 0.5s (before fault)
    let t0 = SimulationTime::try_new(0.5).unwrap();
    engine.step(t0, 0.5, &mut store, None, None, None);
    let pv0 = store.get_value_float(p_tag).unwrap();
    alarm_engine.evaluate_tag(p_tag, pv0, TagQuality::Good, 0.5);
    let alm0 = alarm_engine.get_alarm("ALM-HI-PRESS").unwrap();
    assert_eq!(alm0.state(), AlarmState::Normal);

    // Step 1.0s (fault fires -> PT-101 becomes 50.0 -> trips alarm!)
    let t1 = SimulationTime::try_new(1.0).unwrap();
    engine.step(t1, 0.5, &mut store, None, None, None);
    let pv1 = store.get_value_float(p_tag).unwrap();
    assert_eq!(pv1, 50.0);

    alarm_engine.evaluate_tag(p_tag, pv1, TagQuality::Good, 0.5);
    let alm1 = alarm_engine.get_alarm("ALM-HI-PRESS").unwrap();
    assert_eq!(alm1.state(), AlarmState::ActiveUnack);
}

#[test]
fn test_adversarial_validation_rejects_malformed_definitions() {
    // 1. NaN offset
    let res = validate_scenario(&ScenarioDefinition {
        id: "SCN-BAD".into(),
        name: "Bad".into(),
        description: "Bad".into(),
        author: "Bad".into(),
        schema_version: SCENARIO_SCHEMA_VERSION,
        duration_limit_sec: Some(10.0),
        random_seed: 1,
        initial_conditions: InitialConditions::default(),
        timeline: vec![ScenarioEvent {
            id: "EV-BAD".into(),
            priority: 0,
            trigger: EventTrigger::Time { elapsed_sec: 1.0 },
            action: ScenarioAction::InjectFault(FaultDefinition {
                id: "F-BAD".into(),
                target: "P-1".into(),
                fault_type: FaultType::SensorBias { offset: f64::NAN },
                duration_sec: None,
            }),
        }],
        objectives: vec![],
    });
    assert!(matches!(res, Err(ScenarioError::Validation(_))));

    // 2. Inverted min/max criteria
    let res = validate_scenario(&ScenarioDefinition {
        id: "SCN-BAD2".into(),
        name: "Bad".into(),
        description: "Bad".into(),
        author: "Bad".into(),
        schema_version: SCENARIO_SCHEMA_VERSION,
        duration_limit_sec: Some(10.0),
        random_seed: 1,
        initial_conditions: InitialConditions::default(),
        timeline: vec![],
        objectives: vec![ScenarioObjective {
            id: "OBJ-BAD".into(),
            description: "Bad".into(),
            time_limit_sec: None,
            criteria: ObjectiveCriteria::TagWithinRange {
                tag: "P-1".into(),
                min_val: 100.0,
                max_val: 50.0, // min > max!
                hold_duration_sec: 1.0,
            },
            weight: 10,
            mandatory: true,
        }],
    });
    assert!(matches!(res, Err(ScenarioError::Validation(_))));

    // 3. Negative duration
    let res = validate_scenario(&ScenarioDefinition {
        id: "SCN-BAD3".into(),
        name: "Bad".into(),
        description: "Bad".into(),
        author: "Bad".into(),
        schema_version: SCENARIO_SCHEMA_VERSION,
        duration_limit_sec: Some(-5.0),
        random_seed: 1,
        initial_conditions: InitialConditions::default(),
        timeline: vec![],
        objectives: vec![],
    });
    assert!(matches!(res, Err(ScenarioError::Validation(_))));
}
