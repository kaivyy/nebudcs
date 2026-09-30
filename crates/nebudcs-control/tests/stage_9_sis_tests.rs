//! Stage 9: Safety Instrumented System (SIS) and Functional Safety Verification Suite.
//!
//! Standards Reference: IEC 61511 / ISA-84.
//! Note: Virtual simulation tests. Does not certify hardware for real-world safety duty.

use nebudcs_control::actuator::{ActuatorArbiter, ActuatorCommand, CommandPriority, FailAction};
use nebudcs_control::runtime::IoImageTable;
use nebudcs_control::sis::{
    evaluate_voting, run_simulated_proof_test, DegradationPolicy, SafetyInstrumentedFunction,
    SafetyInstrumentedSystem, SensorChannel, SifActionType, SifBypass, SifConfig, SifFinalElement,
    SifSafeState, SifState, TripDirection, VotingArchitecture,
};
use nebudcs_model::tag::TagQuality;

#[test]
fn test_01_voting_1oo1_architecture() {
    let ch_normal = vec![SensorChannel::new("PT-101", "PT-101", 10.0)];
    let res_normal = evaluate_voting(
        VotingArchitecture::OneOutOfOne,
        DegradationPolicy::DegradeTo1oo2,
        TripDirection::High,
        15.0,
        &ch_normal,
    );
    assert!(!res_normal.trip_demanded);
    assert_eq!(res_normal.healthy_channels, 1);

    // Trip when PV >= threshold
    let ch_trip = vec![SensorChannel::new("PT-101", "PT-101", 16.5)];
    let res_trip = evaluate_voting(
        VotingArchitecture::OneOutOfOne,
        DegradationPolicy::DegradeTo1oo2,
        TripDirection::High,
        15.0,
        &ch_trip,
    );
    assert!(res_trip.trip_demanded);
    assert_eq!(res_trip.voting_votes, 1);

    // Fault on sensor with TripOnFault policy -> Fail-safe trip
    let ch_bad = vec![SensorChannel::new("PT-101", "PT-101", 0.0).with_quality(TagQuality::Bad)];
    let res_bad = evaluate_voting(
        VotingArchitecture::OneOutOfOne,
        DegradationPolicy::TripOnFault,
        TripDirection::High,
        15.0,
        &ch_bad,
    );
    assert!(res_bad.trip_demanded);
    assert_eq!(res_bad.faulted_channels, 1);
}

#[test]
fn test_02_voting_1oo2_architecture() {
    // Normal: both channels below threshold
    let chs = vec![
        SensorChannel::new("LT-101A", "LT-101A", 40.0),
        SensorChannel::new("LT-101B", "LT-101B", 42.0),
    ];
    let res = evaluate_voting(
        VotingArchitecture::OneOutOfTwo,
        DegradationPolicy::DegradeTo1oo1,
        TripDirection::High,
        80.0,
        &chs,
    );
    assert!(!res.trip_demanded);

    // Either channel exceeding triggers 1oo2 trip
    let chs_trip1 = vec![
        SensorChannel::new("LT-101A", "LT-101A", 85.0),
        SensorChannel::new("LT-101B", "LT-101B", 42.0),
    ];
    let res_trip1 = evaluate_voting(
        VotingArchitecture::OneOutOfTwo,
        DegradationPolicy::DegradeTo1oo1,
        TripDirection::High,
        80.0,
        &chs_trip1,
    );
    assert!(res_trip1.trip_demanded);
    assert_eq!(res_trip1.voting_votes, 1);

    // 1 channel fault degrades to 1oo1
    let chs_deg = vec![
        SensorChannel::new("LT-101A", "LT-101A", 0.0).with_quality(TagQuality::Bad),
        SensorChannel::new("LT-101B", "LT-101B", 45.0),
    ];
    let res_deg = evaluate_voting(
        VotingArchitecture::OneOutOfTwo,
        DegradationPolicy::DegradeTo1oo1,
        TripDirection::High,
        80.0,
        &chs_deg,
    );
    assert!(!res_deg.trip_demanded);
    assert!(res_deg.is_degraded);
    assert_eq!(res_deg.effective_scheme, "1oo2 -> 1oo1");

    // Both channels faulted -> Unsafe, fail-safe trip
    let chs_fail = vec![
        SensorChannel::new("LT-101A", "LT-101A", 0.0).with_quality(TagQuality::Bad),
        SensorChannel::new("LT-101B", "LT-101B", 0.0).with_quality(TagQuality::Bad),
    ];
    let res_fail = evaluate_voting(
        VotingArchitecture::OneOutOfTwo,
        DegradationPolicy::DegradeTo1oo1,
        TripDirection::High,
        80.0,
        &chs_fail,
    );
    assert!(res_fail.trip_demanded);
    assert_eq!(res_fail.faulted_channels, 2);
}

#[test]
fn test_03_voting_2oo3_comprehensive_truth_table() {
    let thresh = 100.0;

    // Case 1: 0 faults, 0 votes -> No trip
    let c0_v0 = vec![
        SensorChannel::new("TT-1A", "TT-1A", 80.0),
        SensorChannel::new("TT-1B", "TT-1B", 82.0),
        SensorChannel::new("TT-1C", "TT-1C", 81.0),
    ];
    let r = evaluate_voting(
        VotingArchitecture::TwoOutOfThree,
        DegradationPolicy::DegradeTo1oo2,
        TripDirection::High,
        thresh,
        &c0_v0,
    );
    assert!(!r.trip_demanded);
    assert_eq!(r.voting_votes, 0);

    // Case 2: 0 faults, 1 vote -> No trip (2oo3 rejects spurious single-sensor spike)
    let c0_v1 = vec![
        SensorChannel::new("TT-1A", "TT-1A", 115.0),
        SensorChannel::new("TT-1B", "TT-1B", 82.0),
        SensorChannel::new("TT-1C", "TT-1C", 81.0),
    ];
    let r = evaluate_voting(
        VotingArchitecture::TwoOutOfThree,
        DegradationPolicy::DegradeTo1oo2,
        TripDirection::High,
        thresh,
        &c0_v1,
    );
    assert!(!r.trip_demanded);
    assert_eq!(r.voting_votes, 1);

    // Case 3: 0 faults, 2 votes -> Trip!
    let c0_v2 = vec![
        SensorChannel::new("TT-1A", "TT-1A", 115.0),
        SensorChannel::new("TT-1B", "TT-1B", 112.0),
        SensorChannel::new("TT-1C", "TT-1C", 81.0),
    ];
    let r = evaluate_voting(
        VotingArchitecture::TwoOutOfThree,
        DegradationPolicy::DegradeTo1oo2,
        TripDirection::High,
        thresh,
        &c0_v2,
    );
    assert!(r.trip_demanded);
    assert_eq!(r.voting_votes, 2);

    // Case 4: 0 faults, 3 votes -> Trip!
    let c0_v3 = vec![
        SensorChannel::new("TT-1A", "TT-1A", 115.0),
        SensorChannel::new("TT-1B", "TT-1B", 112.0),
        SensorChannel::new("TT-1C", "TT-1C", 120.0),
    ];
    let r = evaluate_voting(
        VotingArchitecture::TwoOutOfThree,
        DegradationPolicy::DegradeTo1oo2,
        TripDirection::High,
        thresh,
        &c0_v3,
    );
    assert!(r.trip_demanded);
    assert_eq!(r.voting_votes, 3);

    // Case 5: 1 fault with DegradeTo1oo2 policy
    // With 1 fault and 1 vote on healthy channel -> Degrades to 1oo2, trips!
    let c1_v1 = vec![
        SensorChannel::new("TT-1A", "TT-1A", 0.0).with_quality(TagQuality::Bad),
        SensorChannel::new("TT-1B", "TT-1B", 105.0),
        SensorChannel::new("TT-1C", "TT-1C", 80.0),
    ];
    let r = evaluate_voting(
        VotingArchitecture::TwoOutOfThree,
        DegradationPolicy::DegradeTo1oo2,
        TripDirection::High,
        thresh,
        &c1_v1,
    );
    assert!(r.trip_demanded);
    assert!(r.is_degraded);
    assert_eq!(r.effective_scheme, "2oo3 -> 1oo2");

    // Case 6: 1 fault with DegradeTo2oo2 policy
    // With 1 fault and 1 vote -> 2oo2 requires 2 votes, does NOT trip
    let r_2oo2 = evaluate_voting(
        VotingArchitecture::TwoOutOfThree,
        DegradationPolicy::DegradeTo2oo2,
        TripDirection::High,
        thresh,
        &c1_v1,
    );
    assert!(!r_2oo2.trip_demanded);
    assert_eq!(r_2oo2.effective_scheme, "2oo3 -> 2oo2");

    // Case 7: 2 faults -> Voting majority lost, immediate fail-safe trip
    let c2_v0 = vec![
        SensorChannel::new("TT-1A", "TT-1A", 0.0).with_quality(TagQuality::Bad),
        SensorChannel::new("TT-1B", "TT-1B", 0.0).with_quality(TagQuality::Bad),
        SensorChannel::new("TT-1C", "TT-1C", 50.0),
    ];
    let r_c2 = evaluate_voting(
        VotingArchitecture::TwoOutOfThree,
        DegradationPolicy::DegradeTo1oo2,
        TripDirection::High,
        thresh,
        &c2_v0,
    );
    assert!(r_c2.trip_demanded);
    assert_eq!(r_c2.faulted_channels, 2);

    // Case 8: 3 faults -> All sensors lost, immediate fail-safe trip
    let c3_v0 = vec![
        SensorChannel::new("TT-1A", "TT-1A", 0.0).with_quality(TagQuality::Bad),
        SensorChannel::new("TT-1B", "TT-1B", 0.0).with_quality(TagQuality::Bad),
        SensorChannel::new("TT-1C", "TT-1C", 0.0).with_quality(TagQuality::Bad),
    ];
    let r_c3 = evaluate_voting(
        VotingArchitecture::TwoOutOfThree,
        DegradationPolicy::DegradeTo1oo2,
        TripDirection::High,
        thresh,
        &c3_v0,
    );
    assert!(r_c3.trip_demanded);
    assert_eq!(r_c3.faulted_channels, 3);
}

#[test]
fn test_04_sif_trip_latching_and_reset_semantics() {
    let cfg = SifConfig {
        id: "SIF-001".to_string(),
        name: "Reactor High Pressure Shutdown".to_string(),
        description: "Closes feed valve XV-101 on reactor pressure HIHI".to_string(),
        hazard_id: Some("HAZ-R101-OVERPRESSURE".to_string()),
        hazard_scenario: Some("Overpressure leading to vessel loss of containment".to_string()),
        target_sil: Some("SIL 2".to_string()),
        direction: TripDirection::High,
        threshold: 25.0,
        reset_deadband: 2.0, // Clear threshold is 23.0
        trip_delay_seconds: 0.0,
        architecture: VotingArchitecture::OneOutOfOne,
        degradation_policy: DegradationPolicy::DegradeTo1oo2,
        sensor_tags: vec!["PT-101".to_string()],
        final_elements: vec![SifFinalElement::new(
            "XV-101",
            SifSafeState::ValveClosed,
            SifActionType::FeedIsolation,
        )],
        latched: true,
        reset_permissives: vec!["RESET_ENABLE".to_string()],
        pfd_avg_estimate: Some(0.0042),
    };

    let mut sif = SafetyInstrumentedFunction::new(cfg);
    let mut io = IoImageTable::new();
    let mut cmds = Vec::new();

    // Step 1: Normal operating pressure (18 bar < 25 bar) -> Armed
    io.write_input("PT-101", 18.0, TagQuality::Good, 0.0);
    io.write_input("RESET_ENABLE", 1.0, TagQuality::Good, 0.0);
    sif.step(0.05, 0.05, &io, &mut cmds);
    assert_eq!(sif.state(), SifState::Armed);
    assert!(cmds.is_empty());

    // Step 2: High pressure excursion (28 bar >= 25 bar) -> Tripped!
    io.write_input("PT-101", 28.0, TagQuality::Good, 0.10);
    sif.step(0.05, 0.10, &io, &mut cmds);
    assert_eq!(sif.state(), SifState::Tripped);
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].0, "XV-101");
    assert_eq!(cmds[0].1.priority, CommandPriority::Esd);
    assert_eq!(cmds[0].1.target_position, 0.0); // Safe state: ValveClosed

    // Step 3: PV drops back to 24.0 (below 25.0, but still above 23.0 clear threshold)
    // SIF MUST REMAIN LATCHED!
    cmds.clear();
    io.write_input("PT-101", 24.0, TagQuality::Good, 0.15);
    sif.step(0.05, 0.15, &io, &mut cmds);
    assert_eq!(
        sif.state(),
        SifState::Tripped,
        "Tripped SIF must remain latched!"
    );
    assert_eq!(
        cmds.len(),
        1,
        "Must continuously assert safe state while latched"
    );

    // Step 4: Attempt reset while PV is 24.0 -> Rejected (within deadband)
    let reset_res = sif.request_reset(0.20, &io);
    assert!(
        reset_res.is_err(),
        "Reset must be rejected when PV is not cleared below deadband"
    );

    // Step 5: PV drops to safe 20.0 bar (< 23.0 bar), but permissive missing (0.0)
    io.write_input("PT-101", 20.0, TagQuality::Good, 0.25);
    io.write_input("RESET_ENABLE", 0.0, TagQuality::Good, 0.25);
    let reset_res2 = sif.request_reset(0.25, &io);
    assert!(
        reset_res2.is_err(),
        "Reset must be rejected when permissive is false"
    );

    // Step 6: PV safe (20.0 bar) AND permissive satisfied (1.0) -> Reset successful!
    io.write_input("RESET_ENABLE", 1.0, TagQuality::Good, 0.30);
    assert!(sif.request_reset(0.30, &io).is_ok());
    assert_eq!(sif.state(), SifState::Armed);

    // Next tick: Armed, no trip commands asserted
    cmds.clear();
    sif.step(0.05, 0.35, &io, &mut cmds);
    assert_eq!(sif.state(), SifState::Armed);
    assert!(cmds.is_empty());
}

#[test]
fn test_05_sif_administrative_bypass() {
    let cfg = SifConfig {
        id: "SIF-BYPASS-TEST".to_string(),
        name: "Test SIF".to_string(),
        description: "Bypass verification".to_string(),
        hazard_id: None,
        hazard_scenario: None,
        target_sil: None,
        direction: TripDirection::High,
        threshold: 50.0,
        reset_deadband: 5.0,
        trip_delay_seconds: 0.0,
        architecture: VotingArchitecture::OneOutOfOne,
        degradation_policy: DegradationPolicy::DegradeTo1oo2,
        sensor_tags: vec!["TT-TEST".to_string()],
        final_elements: vec![SifFinalElement::new(
            "HV-TEST",
            SifSafeState::ValveClosed,
            SifActionType::HeatingCutoff,
        )],
        latched: true,
        reset_permissives: Vec::new(),
        pfd_avg_estimate: None,
    };

    let mut sif = SafetyInstrumentedFunction::new(cfg);
    let mut io = IoImageTable::new();
    let mut cmds = Vec::new();

    // Apply administrative bypass for 10.0 seconds
    sif.set_bypass(Some(SifBypass {
        bypass_type: "Maintenance".to_string(),
        authorized_by: "SafetyEngineer".to_string(),
        reason: "Transmitter calibration in progress".to_string(),
        start_sim_time: 1.0,
        duration_seconds: Some(10.0),
        active: true,
    }));
    assert_eq!(sif.state(), SifState::Bypassed);

    // Trip condition occurs while bypassed: PV = 90.0 >= 50.0
    io.write_input("TT-TEST", 90.0, TagQuality::Good, 2.0);
    sif.step(0.05, 2.0, &io, &mut cmds);
    assert_eq!(sif.state(), SifState::Bypassed);
    assert!(
        cmds.is_empty(),
        "Bypassed SIF must never assert trip commands!"
    );

    // Time elapses past expiration: 12.0s - 1.0s = 11.0s >= 10.0s duration
    sif.step(0.05, 12.0, &io, &mut cmds);
    assert_eq!(
        sif.state(),
        SifState::Tripped,
        "SIF must trip immediately once bypass expires with active condition"
    );
    assert_eq!(cmds.len(), 1);
}

#[test]
fn test_06_bpcs_vs_sis_arbitration_priority() {
    let mut arbiter = ActuatorArbiter::new(FailAction::FailClosed);

    // 1. BPCS Modulating PID demands valve at 75.0% (Priority 5: Auto)
    arbiter.submit(ActuatorCommand::new(CommandPriority::Auto, 75.0).with_source("PID-101"));
    assert_eq!(arbiter.arbitrate().unwrap().target_position, 75.0);

    // 2. Operator manual override demands 50.0% (Priority 4: Manual)
    arbiter.submit(
        ActuatorCommand::new(CommandPriority::Manual, 50.0).with_source("OperatorFaceplate"),
    );
    assert_eq!(arbiter.arbitrate().unwrap().target_position, 50.0);

    // 3. Permissive constraint demands 20.0% (Priority 3: Permissive)
    arbiter.submit(
        ActuatorCommand::new(CommandPriority::Permissive, 20.0).with_source("PumpInterlock"),
    );
    assert_eq!(arbiter.arbitrate().unwrap().target_position, 20.0);

    // 4. SIS Safety Trip demands ValveClosed 0.0% (Priority 1: Esd)
    arbiter.submit(ActuatorCommand::new(CommandPriority::Esd, 0.0).with_source("SIS:SIF-101-TAHH"));
    let final_decision = arbiter.arbitrate().unwrap();
    assert_eq!(final_decision.priority, CommandPriority::Esd);
    assert_eq!(
        final_decision.target_position, 0.0,
        "SIS CommandPriority::Esd must win arbitration!"
    );
    assert_eq!(
        final_decision.source_id.as_deref(),
        Some("SIS:SIF-101-TAHH")
    );
}

#[test]
fn test_07_proof_test_simulation() {
    let cfg = SifConfig {
        id: "SIF-201-PUMP-TRIP".to_string(),
        name: "Feed Pump P-101 Low Suction Trip".to_string(),
        description: "Trips pump P-101 on suction pressure LOLO".to_string(),
        hazard_id: Some("HAZ-CAVITATION".to_string()),
        hazard_scenario: Some("Suction loss causing dry run and seal failure".to_string()),
        target_sil: Some("SIL 1".to_string()),
        direction: TripDirection::Low,
        threshold: 1.5,
        reset_deadband: 0.2,
        trip_delay_seconds: 0.5,
        architecture: VotingArchitecture::OneOutOfTwo,
        degradation_policy: DegradationPolicy::DegradeTo1oo1,
        sensor_tags: vec!["PT-201A".to_string(), "PT-201B".to_string()],
        final_elements: vec![SifFinalElement::new(
            "P-101",
            SifSafeState::DriverTrip,
            SifActionType::DriverTrip,
        )],
        latched: true,
        reset_permissives: Vec::new(),
        pfd_avg_estimate: Some(0.015),
    };

    let mut sif = SafetyInstrumentedFunction::new(cfg);
    let mut io = IoImageTable::new();
    let mut cmds = Vec::new();

    io.write_input("PT-201A", 2.0, TagQuality::Good, 0.0);
    io.write_input("PT-201B", 2.1, TagQuality::Good, 0.0);
    sif.step(0.05, 0.05, &io, &mut cmds);

    // Execute proof test
    let record = run_simulated_proof_test(&sif, "LeadInstrumentTech", 10.0);
    assert!(
        record.passed,
        "Proof test should pass for fully configured healthy loop"
    );
    assert_eq!(record.sif_id, "SIF-201-PUMP-TRIP");
    assert_eq!(record.step_results.len(), 3);
    assert!(record.step_results.iter().all(|s| s.passed));
}

#[test]
fn test_08_cause_effect_matrix_generation() {
    let mut sis = SafetyInstrumentedSystem::new();

    // SIF 1: Closes XV-101
    sis.register_sif(SafetyInstrumentedFunction::new(SifConfig {
        id: "SIF-01".to_string(),
        name: "High Level Trip".to_string(),
        description: "Level > 90% closes feed valve".to_string(),
        hazard_id: None,
        hazard_scenario: None,
        target_sil: None,
        direction: TripDirection::High,
        threshold: 90.0,
        reset_deadband: 5.0,
        trip_delay_seconds: 0.0,
        architecture: VotingArchitecture::TwoOutOfThree,
        degradation_policy: DegradationPolicy::DegradeTo1oo2,
        sensor_tags: vec![
            "LT-1A".to_string(),
            "LT-1B".to_string(),
            "LT-1C".to_string(),
        ],
        final_elements: vec![SifFinalElement::new(
            "XV-101",
            SifSafeState::ValveClosed,
            SifActionType::FeedIsolation,
        )],
        latched: true,
        reset_permissives: Vec::new(),
        pfd_avg_estimate: None,
    }));

    // SIF 2: Trips Pump P-101 and Closes XV-101
    sis.register_sif(SafetyInstrumentedFunction::new(SifConfig {
        id: "SIF-02".to_string(),
        name: "Low Pressure Trip".to_string(),
        description: "Pressure < 1.0 bar trips pump and closes feed".to_string(),
        hazard_id: None,
        hazard_scenario: None,
        target_sil: None,
        direction: TripDirection::Low,
        threshold: 1.0,
        reset_deadband: 0.2,
        trip_delay_seconds: 0.0,
        architecture: VotingArchitecture::OneOutOfTwo,
        degradation_policy: DegradationPolicy::DegradeTo1oo1,
        sensor_tags: vec!["PT-2A".to_string(), "PT-2B".to_string()],
        final_elements: vec![
            SifFinalElement::new("P-101", SifSafeState::DriverTrip, SifActionType::DriverTrip),
            SifFinalElement::new(
                "XV-101",
                SifSafeState::ValveClosed,
                SifActionType::FeedIsolation,
            ),
        ],
        latched: true,
        reset_permissives: Vec::new(),
        pfd_avg_estimate: None,
    }));

    let matrix = sis.cause_effect_matrix();
    assert_eq!(matrix.causes.len(), 2);
    assert_eq!(matrix.effects.len(), 2); // XV-101 and P-101
    assert_eq!(matrix.cells.len(), 4);

    // Verify cell connections
    let active_links: Vec<_> = matrix.cells.iter().filter(|c| c.active_link).collect();
    assert_eq!(
        active_links.len(),
        3,
        "SIF-01 has 1 effect; SIF-02 has 2 effects = 3 active intersections"
    );
}
