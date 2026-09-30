//! Phase 15: Training Studio comprehensive domain and evaluation unit test suite.

use nebudcs_historian::HistoricalEvent;
use nebudcs_scenario::training::*;
use nebudcs_types::SimulationTime;
use std::collections::BTreeMap;

#[test]
fn test_session_lifecycle_transitions() {
    let mut session = TrainingSession::new(
        "sess-test-01",
        "Test Distillation Column Recovery",
        "Operator recovery drills",
        "inst-alice",
        "trainee-bob",
        1700000000000,
    );
    assert_eq!(session.state, SessionLifecycleState::Created);

    // Cannot start from Created
    let err = session
        .transition_to(SessionLifecycleState::Running, 0.0, 1700000001000)
        .unwrap_err();
    assert!(matches!(err, TrainingError::InvalidStateTransition { .. }));

    // Load project and scenario -> Loaded
    session
        .load_project_and_scenario("proj-alpha", "scn-alpha-upset", 0.0, 1700000002000)
        .unwrap();
    assert_eq!(session.state, SessionLifecycleState::Loaded);
    assert_eq!(session.project_id, "proj-alpha");
    assert_eq!(session.scenario_id, "scn-alpha-upset");

    // Start -> Running
    session
        .transition_to(SessionLifecycleState::Running, 0.0, 1700000003000)
        .unwrap();
    assert_eq!(session.state, SessionLifecycleState::Running);
    assert_eq!(session.started_at_ms, Some(1700000003000));

    // Pause -> Paused
    session
        .transition_to(SessionLifecycleState::Paused, 15.0, 1700000018000)
        .unwrap();
    assert_eq!(session.state, SessionLifecycleState::Paused);

    // Resume -> Running
    session
        .transition_to(SessionLifecycleState::Running, 15.0, 1700000025000)
        .unwrap();
    assert_eq!(session.state, SessionLifecycleState::Running);

    // Complete -> Completed
    session
        .transition_to(SessionLifecycleState::Completed, 60.0, 1700000070000)
        .unwrap();
    assert_eq!(session.state, SessionLifecycleState::Completed);
    assert_eq!(session.completed_at_ms, Some(1700000070000));

    // Cannot transition out of Completed (terminal)
    let err = session
        .transition_to(SessionLifecycleState::Running, 60.0, 1700000080000)
        .unwrap_err();
    assert!(matches!(err, TrainingError::InvalidStateTransition { .. }));
}

#[test]
fn test_session_abort_from_any_active_state() {
    let mut session =
        TrainingSession::new("sess-abort-01", "Abort Drill", "", "inst", "trainee", 100);
    session
        .load_project_and_scenario("p1", "s1", 0.0, 110)
        .unwrap();
    session
        .transition_to(SessionLifecycleState::Running, 0.0, 120)
        .unwrap();

    // Abort from running
    session
        .transition_to(SessionLifecycleState::Aborted, 10.0, 130)
        .unwrap();
    assert_eq!(session.state, SessionLifecycleState::Aborted);
    assert_eq!(session.completed_at_ms, Some(130));

    // Reset fails from terminal
    assert!(session.reset_session().is_err());
}

#[test]
fn test_trainee_action_recording_and_properties() {
    let action_sp = TraineeActionType::SetSetpoint {
        loop_name: "PIC-101".to_string(),
        sp: 12.5,
    };
    assert_eq!(action_sp.target_entity(), "PIC-101");
    assert_eq!(action_sp.action_type_name(), "SET_SETPOINT");
    assert_eq!(action_sp.numeric_value(), Some(12.5));

    let record = TraineeActionRecord::accepted(
        "act-001",
        "sess-001",
        14.2,
        1700000014200,
        "trainee-bob",
        action_sp,
    );
    assert_eq!(record.action_id, "act-001");
    assert_eq!(record.actor, "trainee-bob");
    assert_eq!(record.sim_time_sec, 14.2);
    assert_eq!(record.status, ActionStatus::Accepted);
    assert_eq!(record.requested_value, Some(12.5));

    let rejected = TraineeActionRecord::rejected(
        "act-002",
        "sess-001",
        15.0,
        1700000015000,
        "trainee-bob",
        TraineeActionType::EmergencyStop,
        "Unauthorized role",
    );
    assert!(matches!(rejected.status, ActionStatus::Rejected { .. }));
}

#[test]
fn test_evaluation_response_time_pass_and_fail() {
    let session = TrainingSession::new("s1", "Test", "", "inst", "bob", 100);

    let criteria = vec![EvaluationCriterion {
        criterion_id: "crit-ack-alarm".to_string(),
        title: "Acknowledge High Pressure Alarm Promptly".to_string(),
        description: "Alarm must be acknowledged within 10 seconds".to_string(),
        kind: CriterionKind::AlarmResponseTime {
            alarm_name: "PAH-101".to_string(),
            max_allowed_response_sec: 10.0,
        },
        required: true,
        weight: 100,
        penalty: 50,
        severity: CriterionSeverity::Critical,
    }];

    let events = vec![HistoricalEvent::alarm(
        "run-1",
        SimulationTime::try_new(5.0).unwrap(),
        1,
        "PAH-101",
        Some("NORMAL".to_string()),
        "ACTIVE".to_string(),
        14.2,
        4,
        None,
        105,
    )];

    // Case 1: Prompt response at t=9.0s (delta = 4.0s <= 10.0s) -> PASS
    let actions_pass = vec![TraineeActionRecord::accepted(
        "a1",
        "s1",
        9.0,
        109,
        "bob",
        TraineeActionType::AcknowledgeAlarm {
            alarm_name: "PAH-101".to_string(),
        },
    )];
    let report_pass = evaluate_session(
        &session,
        &criteria,
        &actions_pass,
        &events,
        &BTreeMap::new(),
        &[],
        200,
    );
    assert!(report_pass.passed);
    assert_eq!(report_pass.total_score, 100);
    assert!(report_pass.results[0].passed);
    assert_eq!(
        report_pass.results[0]
            .evidence
            .get("response_time_sec")
            .unwrap(),
        "4.000"
    );

    // Case 2: Late response at t=18.0s (delta = 13.0s > 10.0s) -> FAIL
    let actions_fail = vec![TraineeActionRecord::accepted(
        "a2",
        "s1",
        18.0,
        118,
        "bob",
        TraineeActionType::AcknowledgeAlarm {
            alarm_name: "PAH-101".to_string(),
        },
    )];
    let report_fail = evaluate_session(
        &session,
        &criteria,
        &actions_fail,
        &events,
        &BTreeMap::new(),
        &[],
        200,
    );
    assert!(!report_fail.passed);
    assert!(!report_fail.results[0].passed);
    assert_eq!(report_fail.results[0].penalty_deducted, 50);

    // Case 3: Missing response entirely -> FAIL
    let report_missing = evaluate_session(
        &session,
        &criteria,
        &[],
        &events,
        &BTreeMap::new(),
        &[],
        200,
    );
    assert!(!report_missing.passed);
    assert!(!report_missing.results[0].passed);
}

#[test]
fn test_evaluation_required_and_forbidden_actions() {
    let session = TrainingSession::new("s1", "Actions Drill", "", "inst", "trainee", 100);

    let criteria = vec![
        EvaluationCriterion {
            criterion_id: "crit-switch-auto".to_string(),
            title: "Switch Feed Loop to AUTO".to_string(),
            description: "".to_string(),
            kind: CriterionKind::RequiredAction {
                target_entity: "FIC-201".to_string(),
                action_type: "SET_MODE".to_string(),
                expected_value: None,
                tolerance: None,
                within_sec: Some(30.0),
            },
            required: true,
            weight: 50,
            penalty: 0,
            severity: CriterionSeverity::Standard,
        },
        EvaluationCriterion {
            criterion_id: "crit-no-manual-vent".to_string(),
            title: "Do Not Open Atmospheric Vent XV-999".to_string(),
            description: "".to_string(),
            kind: CriterionKind::ForbiddenAction {
                target_entity: "XV-999".to_string(),
                action_type: "WRITE_TAG".to_string(),
            },
            required: false,
            weight: 50,
            penalty: 50,
            severity: CriterionSeverity::Critical,
        },
    ];

    // Trainee complies with required action and avoids forbidden action -> 100% PASS
    let actions_good = vec![TraineeActionRecord::accepted(
        "act-1",
        "s1",
        12.0,
        112,
        "trainee",
        TraineeActionType::SetMode {
            loop_name: "FIC-201".to_string(),
            mode: "AUTO".to_string(),
        },
    )];
    let report = evaluate_session(
        &session,
        &criteria,
        &actions_good,
        &[],
        &BTreeMap::new(),
        &[],
        200,
    );
    assert!(report.passed);
    assert_eq!(report.total_score, 100);

    // Trainee commits forbidden action -> penalized
    let actions_bad = vec![
        TraineeActionRecord::accepted(
            "act-1",
            "s1",
            12.0,
            112,
            "trainee",
            TraineeActionType::SetMode {
                loop_name: "FIC-201".to_string(),
                mode: "AUTO".to_string(),
            },
        ),
        TraineeActionRecord::accepted(
            "act-2",
            "s1",
            15.0,
            115,
            "trainee",
            TraineeActionType::WriteTag {
                tag: "XV-999".to_string(),
                value: 1.0,
            },
        ),
    ];
    let report_bad = evaluate_session(
        &session,
        &criteria,
        &actions_bad,
        &[],
        &BTreeMap::new(),
        &[],
        200,
    );
    assert_eq!(report_bad.total_score, 0); // 50 awarded - 50 penalized
    assert!(!report_bad.results[1].passed);
}

#[test]
fn test_evaluation_safe_recovery_and_no_trip() {
    let session = TrainingSession::new("s1", "Recovery", "", "inst", "trainee", 100);

    let criteria = vec![
        EvaluationCriterion {
            criterion_id: "crit-no-trip".to_string(),
            title: "Zero Protective Trips".to_string(),
            description: "".to_string(),
            kind: CriterionKind::NoTripOccurred,
            required: true,
            weight: 50,
            penalty: 50,
            severity: CriterionSeverity::Fatal,
        },
        EvaluationCriterion {
            criterion_id: "crit-level-recovery".to_string(),
            title: "Level Stabilized in [45%, 55%]".to_string(),
            description: "".to_string(),
            kind: CriterionKind::SafeRecovery {
                tag: "V-101.LEVEL".to_string(),
                min_val: 45.0,
                max_val: 55.0,
                hold_duration_sec: 10.0,
            },
            required: true,
            weight: 50,
            penalty: 0,
            severity: CriterionSeverity::Standard,
        },
    ];

    // Samples: Level dips to 30, then recovers to 50 from t=30 to t=50 (final time 50)
    let samples = vec![
        (0.0, "V-101.LEVEL".to_string(), 50.0),
        (10.0, "V-101.LEVEL".to_string(), 35.0),
        (20.0, "V-101.LEVEL".to_string(), 42.0),
        (35.0, "V-101.LEVEL".to_string(), 48.0),
        (40.0, "V-101.LEVEL".to_string(), 50.2),
        (45.0, "V-101.LEVEL".to_string(), 50.1),
        (50.0, "V-101.LEVEL".to_string(), 49.9),
    ];

    let report = evaluate_session(
        &session,
        &criteria,
        &[],
        &[],
        &BTreeMap::new(),
        &samples,
        200,
    );
    assert!(report.passed);
    assert_eq!(report.total_score, 100);

    // Now introduce an emergency trip into events
    let events_with_trip = vec![HistoricalEvent::protection_trip(
        "run-1",
        SimulationTime::try_new(12.0).unwrap(),
        1,
        "TRIP-PUMP-OVERPRESSURE",
        "P-101",
        18.5,
        None,
        112,
    )];
    let report_tripped = evaluate_session(
        &session,
        &criteria,
        &[],
        &events_with_trip,
        &BTreeMap::new(),
        &samples,
        200,
    );
    assert!(!report_tripped.passed);
    assert!(!report_tripped.results[0].passed);
}

#[test]
fn test_replay_frame_reconstruction_and_markers() {
    let mut session = TrainingSession::new("s1", "Replay Drill", "", "inst", "trainee", 100);
    session.simulation_start_time_sec = 0.0;
    session.current_simulation_time_sec = 60.0;

    let samples = vec![
        (0.0, "TI-101".to_string(), 100.0),
        (20.0, "TI-101".to_string(), 120.0),
        (40.0, "TI-101".to_string(), 140.0),
        (60.0, "TI-101".to_string(), 160.0),
    ];

    let events = vec![
        HistoricalEvent::alarm(
            "run-1",
            SimulationTime::try_new(25.0).unwrap(),
            1,
            "TAH-101",
            Some("NORMAL".to_string()),
            "ACTIVE".to_string(),
            125.0,
            3,
            None,
            125,
        ),
        HistoricalEvent::protection_trip(
            "run-1",
            SimulationTime::try_new(45.0).unwrap(),
            2,
            "ESD-TRIP-HEATER",
            "H-101",
            145.0,
            None,
            145,
        ),
    ];

    let actions = vec![TraineeActionRecord::accepted(
        "act-1",
        "s1",
        28.0,
        128,
        "trainee",
        TraineeActionType::AcknowledgeAlarm {
            alarm_name: "TAH-101".to_string(),
        },
    )];

    let replay_data = ReplaySessionData::build(&session, &samples, &events, &actions);
    assert_eq!(replay_data.duration_sec, 60.0);
    assert_eq!(replay_data.markers.len(), 3); // 2 events + 1 action

    // Frame at t=10s: TI-101 interpolated between (0, 100) and (20, 120) -> 110.0
    let f10 = replay_data.frame_at(10.0);
    assert_eq!(f10.sim_time_sec, 10.0);
    let val_10 = f10.tags.get("TI-101").copied().unwrap();
    assert!((val_10 - 110.0).abs() < 1e-3);
    assert!(f10.active_alarms.is_empty());
    assert!(f10.active_trips.is_empty());

    // Frame at t=30s: TAH-101 active, ESD not yet active, recent action present
    let f30 = replay_data.frame_at(30.0);
    assert_eq!(f30.active_alarms, vec!["TAH-101"]);
    assert!(f30.active_trips.is_empty());
    assert_eq!(f30.recent_actions.len(), 1);

    // Frame at t=50s: Both TAH-101 and ESD active
    let f50 = replay_data.frame_at(50.0);
    assert!(f50.active_alarms.contains(&"TAH-101".to_string()));
    assert!(f50.active_trips.contains(&"ESD-TRIP-HEATER".to_string()));
}

#[test]
fn test_stage_11_sif_trip_assessment_and_operational_separation() {
    let session = TrainingSession::new("s1", "S11 SIF Test", "", "inst", "trainee", 1000);

    let criteria = vec![
        EvaluationCriterion {
            criterion_id: "CRIT-SIF-AVOID".into(),
            title: "Avoid Unnecessary SIF Trip".into(),
            description: "SIF-101 must not trip".into(),
            kind: CriterionKind::SifTripAssessment {
                sif_id: "SIF-101".into(),
                policy: SafetyTripPolicy::Avoidable,
            },
            required: true,
            weight: 100,
            penalty: 50,
            severity: CriterionSeverity::Critical,
        },
        EvaluationCriterion {
            criterion_id: "CRIT-SEP".into(),
            title: "Operational Layer Separation".into(),
            description: "BPCS alarm must precede SIF trip".into(),
            kind: CriterionKind::OperationalSeparationCheck {
                alarm_name: "PAH-101".into(),
                sif_id: "SIF-102".into(),
            },
            required: false,
            weight: 50,
            penalty: 0,
            severity: CriterionSeverity::Standard,
        },
    ];

    let mut events = vec![
        HistoricalEvent::alarm(
            "run-1",
            SimulationTime::try_new(10.0).unwrap(),
            1,
            "PAH-101",
            Some("NORMAL".to_string()),
            "ACTIVE".to_string(),
            45.0,
            2,
            None,
            1000,
        ),
        HistoricalEvent::protection_trip(
            "run-1",
            SimulationTime::try_new(25.0).unwrap(),
            2,
            "SIF-102",
            "TRIPPED",
            55.0,
            None,
            1025,
        ),
    ];

    let report = evaluate_session(
        &session,
        &criteria,
        &[],
        &events,
        &BTreeMap::new(),
        &[],
        2000,
    );

    // SIF-101 did not trip -> Avoidable trip successfully averted!
    assert!(report.results[0].passed);
    assert_eq!(
        report.results[0].safety_trip_assessment.as_deref(),
        Some("AVOIDABLE_TRIP_PREVENTED")
    );

    // SIF-102 tripped at 25s, but PAH-101 alarmed at 10s -> Operational separation verified!
    assert!(report.results[1].passed);

    // Now test failure case: SIF-101 tripped (avoidable trip unprevented)
    events.push(HistoricalEvent::protection_trip(
        "run-1",
        SimulationTime::try_new(30.0).unwrap(),
        3,
        "SIF-101",
        "TRIPPED",
        60.0,
        None,
        1030,
    ));

    let report_fail = evaluate_session(
        &session,
        &criteria,
        &[],
        &events,
        &BTreeMap::new(),
        &[],
        3000,
    );

    assert!(!report_fail.results[0].passed);
    assert_eq!(
        report_fail.results[0].safety_trip_assessment.as_deref(),
        Some("AVOIDABLE_TRIP_UNPREVENTED")
    );
    assert_eq!(
        report_fail.results[0].error_classification,
        Some(OperatorErrorClassification::Incorrect)
    );
    assert!(!report_fail.passed);
    assert!(report_fail.debrief.is_some());
}

#[test]
fn test_stage_11_startup_sequence_and_tag_within_band() {
    let session = TrainingSession::new("s2", "S11 Startup", "", "inst", "trainee", 1000);

    let criteria = vec![
        EvaluationCriterion {
            criterion_id: "CRIT-STARTUP-STEP".into(),
            title: "Pump Start Step".into(),
            description: "Acknowledge pump start within 20s".into(),
            kind: CriterionKind::StartupSequenceStep {
                step_name: "STEP-START-P101".into(),
                order: 1,
                timeout_sec: Some(20.0),
            },
            required: true,
            weight: 50,
            penalty: 0,
            severity: CriterionSeverity::Standard,
        },
        EvaluationCriterion {
            criterion_id: "CRIT-BAND".into(),
            title: "Level Operating Band".into(),
            description: "Level maintained in [40.0, 60.0]".into(),
            kind: CriterionKind::TagWithinBand {
                tag: "LT-101".into(),
                min_val: 40.0,
                max_val: 60.0,
            },
            required: true,
            weight: 50,
            penalty: 0,
            severity: CriterionSeverity::Standard,
        },
    ];

    let actions = vec![TraineeActionRecord::accepted(
        "act-p1",
        "s2",
        12.0,
        1012,
        "trainee",
        TraineeActionType::ProcedureStepAck {
            step_id: "STEP-START-P101".to_string(),
        },
    )];

    let samples = vec![
        (0.0, "LT-101".to_string(), 50.0),
        (10.0, "LT-101".to_string(), 52.0),
        (20.0, "LT-101".to_string(), 48.0),
    ];

    let mut final_tags = BTreeMap::new();
    final_tags.insert("LT-101".to_string(), 49.0);

    let report = evaluate_session(
        &session,
        &criteria,
        &actions,
        &[],
        &final_tags,
        &samples,
        2000,
    );

    assert!(report.passed);
    assert!(report.results[0].passed);
    assert!(report.results[1].passed);

    let debrief = report.debrief.unwrap();
    assert_eq!(
        debrief.outcome_verdict,
        "STAGE 11 OTS ASSESSMENT: SATISFACTORY / PASS"
    );
}
