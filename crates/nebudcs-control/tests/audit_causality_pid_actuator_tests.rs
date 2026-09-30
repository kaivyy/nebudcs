use nebudcs_control::actuator::{
    ActuatorArbiter, ActuatorCommand, ActuatorConfig, ActuatorModel, CommandPriority, FailAction,
};
use nebudcs_control::pid::{PidConfig, PidController, PidDirection, PidMode};
use nebudcs_control::protection::{
    DegradationPolicy, ProtectionBlock, ProtectionConfig, ProtectionState, SensorInput,
    VotingVoter2oo3,
};
use nebudcs_model::tag::{TagQuality, TagValue};
use nebudcs_sim::{DoubleBufferedState, EquipmentState, StateBuffer};
use nebudcs_types::{EquipmentId, SimulationTime, TagId};

// =========================================================================
// 1. PID Edge Cases & Validation Tests
// =========================================================================

#[test]
fn test_pid_config_validation_rejections() {
    // 1. NaN gain
    let cfg = PidConfig {
        kp: f64::NAN,
        ..Default::default()
    };
    assert!(cfg.validate().is_err(), "NaN gain must be rejected");

    // 2. Negative Ti
    let cfg = PidConfig {
        ti: -1.0,
        ..Default::default()
    };
    assert!(
        cfg.validate().is_err(),
        "Negative integral time must be rejected"
    );

    // 3. Negative Td
    let cfg = PidConfig {
        td: -0.5,
        ..Default::default()
    };
    assert!(
        cfg.validate().is_err(),
        "Negative derivative time must be rejected"
    );

    // 4. Non-positive derivative filter N
    let cfg = PidConfig {
        derivative_filter_n: 0.0,
        ..Default::default()
    };
    assert!(cfg.validate().is_err(), "Filter N <= 0 must be rejected");

    // 5. Inverted output limits (out_min >= out_max)
    let cfg = PidConfig {
        out_min: 100.0,
        out_max: 0.0,
        ..Default::default()
    };
    assert!(
        cfg.validate().is_err(),
        "out_min >= out_max must be rejected"
    );

    // 6. Zero scan divisor
    let cfg = PidConfig {
        scan_divisor: 0,
        ..Default::default()
    };
    assert!(cfg.validate().is_err(), "scan_divisor < 1 must be rejected");

    // 7. Valid config
    let valid_cfg = PidConfig::default();
    assert!(valid_cfg.validate().is_ok());
}

#[test]
fn test_pid_edge_case_inputs() {
    let cfg = PidConfig {
        kp: 2.0,
        ti: 5.0,
        td: 1.0,
        derivative_filter_n: 10.0,
        out_min: 0.0,
        out_max: 100.0,
        direction: PidDirection::Direct,
        sp_tracking_in_manual: false,
        scan_divisor: 1,
    };
    let mut pid = PidController::new(cfg);
    pid.initialize(50.0);
    pid.set_mode(PidMode::Auto);
    pid.set_sp(50.0);

    // Initial output should be at baseline
    let out_initial = pid.output();

    // 1. dt = 0.0: must not divide by zero or corrupt state
    let out_zero_dt = pid.step(0.0, 60.0);
    assert_eq!(
        out_zero_dt, out_initial,
        "dt = 0 must hold output without modification"
    );

    // 2. NaN PV: must not corrupt integral/derivative with NaN
    let out_nan = pid.step(0.1, f64::NAN);
    assert_eq!(out_nan, out_initial, "NaN PV must not corrupt output");
    assert!(!pid.integral().is_nan(), "Integral must not become NaN");
    assert!(!pid.derivative().is_nan(), "Derivative must not become NaN");

    // 3. Inf PV: must not corrupt state
    let out_inf = pid.step(0.1, f64::INFINITY);
    assert_eq!(out_inf, out_initial, "Infinite PV must not corrupt output");

    // 4. Bad signal quality: must hold output and suspend integration
    pid.set_quality(TagQuality::Bad);
    let out_bad_qual = pid.step(0.1, 75.0);
    assert_eq!(
        out_bad_qual, out_initial,
        "Bad quality must hold output and suspend integration"
    );

    // Restore Good quality
    pid.set_quality(TagQuality::Good);
    let out_good = pid.step(0.1, 60.0);
    assert!(
        out_good > out_initial,
        "Valid input with Good quality must resume regulation"
    );

    // 5. Test controller reset
    pid.reset();
    assert_eq!(pid.integral(), 0.0);
    assert_eq!(pid.derivative(), 0.0);
    assert_eq!(pid.output(), 0.0);
}

#[test]
fn test_pid_mode_transitions_and_cascade_lifecycle() {
    let cfg = PidConfig {
        kp: 2.0,
        ti: 10.0,
        td: 0.0,
        derivative_filter_n: 10.0,
        out_min: 0.0,
        out_max: 100.0,
        direction: PidDirection::Reverse,
        sp_tracking_in_manual: true,
        scan_divisor: 1,
    };
    let mut pid = PidController::new(cfg);
    pid.initialize(50.0);

    // In Manual: set output to 42.5%
    pid.set_manual_output(42.5);
    let out_m = pid.step(0.1, 50.0);
    assert_eq!(out_m, 42.5);
    assert_eq!(pid.sp(), 50.0, "Manual mode SP tracks PV");

    // Transfer to Auto: bumpless transfer test
    pid.set_mode(PidMode::Auto);
    let out_a = pid.step(0.1, 50.0);
    assert!(
        (out_a - 42.5).abs() < 1e-6,
        "Transfer MAN -> AUTO must be bumpless (|ΔOUT| < 1e-6), got {out_a}"
    );

    // Transfer to Cascade
    pid.set_mode(PidMode::Cascade);
    pid.set_cascade_sp(60.0);
    assert_eq!(pid.sp(), 60.0);
    let out_c = pid.step(0.1, 50.0);
    assert!(out_c > 0.0);

    // Drop from Cascade to Auto: SP should remain at last active cascade SP (60.0)
    pid.set_mode(PidMode::Auto);
    assert_eq!(
        pid.sp(),
        60.0,
        "Dropping from Cascade to Auto must preserve working setpoint"
    );

    // Transfer back to Manual
    pid.set_mode(PidMode::Manual);
    assert!(
        (pid.output() - out_c).abs() < 1.0,
        "Manual output initialized to last active output"
    );
}

// =========================================================================
// 2. Actuator Arbitration & Permissive Tests
// =========================================================================

#[test]
fn test_actuator_arbitration_order_independence() {
    let mut arbiter = ActuatorArbiter::new(FailAction::FailClosed);

    // Submit identical priority with different target positions
    let cmd1 = ActuatorCommand::new(CommandPriority::Interlock, 30.0).with_source("INTERLOCK_1");
    let cmd2 = ActuatorCommand::new(CommandPriority::Interlock, 10.0).with_source("INTERLOCK_2");
    let cmd3 = ActuatorCommand::new(CommandPriority::Interlock, 50.0).with_source("INTERLOCK_3");

    // Order 1: [cmd1, cmd2, cmd3]
    arbiter.submit(cmd1.clone());
    arbiter.submit(cmd2.clone());
    arbiter.submit(cmd3.clone());
    let winner1 = arbiter.arbitrate().unwrap();
    assert_eq!(
        winner1.target_position, 10.0,
        "FailClosed must choose minimum (10.0%)"
    );

    // Order 2: [cmd3, cmd2, cmd1]
    arbiter.clear_all();
    arbiter.submit(cmd3.clone());
    arbiter.submit(cmd2.clone());
    arbiter.submit(cmd1.clone());
    let winner2 = arbiter.arbitrate().unwrap();
    assert_eq!(
        winner2.target_position, 10.0,
        "Arbitration must be order independent"
    );

    // Clear specific source
    arbiter.clear_source("INTERLOCK_2");
    let winner3 = arbiter.arbitrate().unwrap();
    assert_eq!(
        winner3.target_position, 30.0,
        "After clearing 10.0, 30.0 must be the next minimum"
    );
}

#[test]
fn test_actuator_same_priority_conflict_fail_actions() {
    // FailOpen: maximum position must win
    let mut arb_open = ActuatorArbiter::new(FailAction::FailOpen);
    arb_open.submit(ActuatorCommand::new(CommandPriority::Manual, 20.0).with_source("OP_1"));
    arb_open.submit(ActuatorCommand::new(CommandPriority::Manual, 80.0).with_source("OP_2"));
    let winner_open = arb_open.arbitrate().unwrap();
    assert_eq!(
        winner_open.target_position, 80.0,
        "FailOpen must select maximum position"
    );

    // FailClosed: minimum position must win
    let mut arb_closed = ActuatorArbiter::new(FailAction::FailClosed);
    arb_closed.submit(ActuatorCommand::new(CommandPriority::Manual, 20.0).with_source("OP_1"));
    arb_closed.submit(ActuatorCommand::new(CommandPriority::Manual, 80.0).with_source("OP_2"));
    let winner_closed = arb_closed.arbitrate().unwrap();
    assert_eq!(
        winner_closed.target_position, 20.0,
        "FailClosed must select minimum position"
    );
}

#[test]
fn test_actuator_signal_loss_and_power_air_failure() {
    let act_cfg = ActuatorConfig {
        stroke_time: 1.0, // 100% in 1.0s
        deadband: 0.1,
        fail_action: FailAction::FailClosed,
    };
    act_cfg.validate().expect("valid config");

    let mut model = ActuatorModel::new(act_cfg, 50.0);

    // 1. Loss of instrument air/power: drives to FailClosed (0.0%)
    let pos_after_air_loss = model.step(0.5, 75.0, false); // power_air_ok = false
    assert!(
        pos_after_air_loss < 50.0,
        "Air loss must force movement towards FailClosed (0%)"
    );
    let pos_fully_failed = model.step(0.5, 75.0, false);
    assert_eq!(
        pos_fully_failed, 0.0,
        "Actuator must fully close on air loss"
    );

    // 2. FailOpen actuator
    let open_cfg = ActuatorConfig {
        stroke_time: 1.0,
        deadband: 0.1,
        fail_action: FailAction::FailOpen,
    };
    let mut open_model = ActuatorModel::new(open_cfg, 20.0);
    open_model.step(1.0, 10.0, false);
    assert_eq!(
        open_model.actual_position(),
        100.0,
        "FailOpen must drive to 100% on air loss"
    );

    // 3. Deadband: small movements within deadband ignored
    let mut db_model = ActuatorModel::new(
        ActuatorConfig {
            stroke_time: 1.0,
            deadband: 0.5,
            fail_action: FailAction::FailClosed,
        },
        50.0,
    );
    // Command change within 0.5% deadband (e.g. 50.3%)
    let db_pos = db_model.step(0.1, 50.3, true);
    assert_eq!(
        db_pos, 50.0,
        "Movements within deadband must produce zero stem motion"
    );

    // Command change outside deadband (e.g. 51.0%)
    let active_pos = db_model.step(0.1, 51.0, true);
    assert!(
        active_pos > 50.0,
        "Movements exceeding deadband must move stem"
    );
}

// =========================================================================
// 3. Protection Engine & Voting Tests
// =========================================================================

#[test]
fn test_protection_trip_latch_deadband_and_reset_refusal() {
    let cfg = ProtectionConfig::new("HIGH_PRESSURE_ESD", 80.0, 5.0);
    cfg.validate().expect("valid protection config");

    let mut block = ProtectionBlock::new(cfg);
    assert_eq!(block.state(), ProtectionState::Armed);

    // 1. Normal operation below threshold
    assert!(!block.step(70.0, false));
    assert_eq!(block.state(), ProtectionState::Active);

    // 2. Threshold crossed: trips and latches
    assert!(block.step(82.0, false));
    assert_eq!(block.state(), ProtectionState::Tripped);

    // 3. Condition drops below threshold (e.g. 78.0) but is INSIDE reset deadband (80 - 5 = 75)
    // Even with reset command, it MUST REFUSE to reset!
    assert!(
        block.step(78.0, true),
        "Must stay tripped when inside reset deadband"
    );
    assert_eq!(block.state(), ProtectionState::Tripped);

    // 4. Condition drops below 75.0 (e.g. 72.0) BUT reset_command is false
    // It MUST REMAIN LATCHED!
    assert!(
        block.step(72.0, false),
        "Must remain latched without operator reset command"
    );
    assert_eq!(block.state(), ProtectionState::Tripped);

    // 5. Condition is below 75.0 AND reset_command is true: resets to Armed!
    assert!(
        !block.step(72.0, true),
        "Trip must clear when condition safe and reset command asserted"
    );
    assert_eq!(block.state(), ProtectionState::Armed);
}

#[test]
fn test_protection_2oo3_voting_degradation_policies() {
    let threshold = 100.0;

    // Inputs with 1 sensor above threshold: [105, 90, 85]
    let one_bad_channel_one_trip = [
        SensorInput::new(105.0, TagQuality::Good),
        SensorInput::new(0.0, TagQuality::Bad),
        SensorInput::new(85.0, TagQuality::Good),
    ];

    // Policy 1: DegradeTo1oo2 (Conservative)
    let voter_1oo2 = VotingVoter2oo3::new(threshold, DegradationPolicy::DegradeTo1oo2);
    let res_1oo2 = voter_1oo2.evaluate(&one_bad_channel_one_trip);
    assert!(res_1oo2.is_degraded);
    assert!(
        res_1oo2.trip,
        "DegradeTo1oo2 must trip when 1 healthy sensor crosses threshold"
    );

    // Policy 2: DegradeTo2oo2 (High Availability)
    let voter_2oo2 = VotingVoter2oo3::new(threshold, DegradationPolicy::DegradeTo2oo2);
    let res_2oo2 = voter_2oo2.evaluate(&one_bad_channel_one_trip);
    assert!(res_2oo2.is_degraded);
    assert!(
        !res_2oo2.trip,
        "DegradeTo2oo2 must NOT trip on single sensor high when degraded"
    );

    // Policy 3: TripOnFault
    let voter_tof = VotingVoter2oo3::new(threshold, DegradationPolicy::TripOnFault);
    let res_tof = voter_tof.evaluate(&[
        SensorInput::new(50.0, TagQuality::Good),
        SensorInput::new(0.0, TagQuality::Bad),
        SensorInput::new(50.0, TagQuality::Good),
    ]);
    assert!(
        res_tof.trip,
        "TripOnFault must trip immediately on any bad sensor channel"
    );

    // Multi-fault condition: 2 or 3 failed sensors must ALWAYS fail-safe trip
    let multi_fault = [
        SensorInput::new(0.0, TagQuality::Bad),
        SensorInput::new(0.0, TagQuality::Bad),
        SensorInput::new(50.0, TagQuality::Good),
    ];
    let res_multi = voter_2oo2.evaluate(&multi_fault);
    assert!(
        res_multi.trip,
        "Multiple sensor channel faults must trigger fail-safe trip"
    );
}

// =========================================================================
// 4. Double-Buffer & Causality Proof
// =========================================================================

#[test]
fn test_double_buffer_causality_and_atomic_commit() {
    let mut state = DoubleBufferedState::new();
    let mut initial = StateBuffer::new();
    let eq_id = EquipmentId::new();
    let tag_id = TagId::new();

    let mut eq = EquipmentState::new("running");
    eq.set_var("temperature", 300.0);
    initial.equipment.insert(eq_id, eq);
    initial.tags.insert(tag_id, TagValue::Float(300.0));
    initial.qualities.insert(tag_id, TagQuality::Good);

    state.initialize(initial);

    // Tick N: Prepare next buffer at t = 0.1
    state.prepare_next(SimulationTime::try_new(0.1).unwrap());

    // Systems operate against (ReadView, WriteView)
    {
        let (read_view, mut write_view) = state.rw_views();

        // 1. ReadView must see committed state (300.0)
        assert_eq!(
            read_view
                .equipment(eq_id)
                .unwrap()
                .get_var("temperature")
                .unwrap(),
            300.0
        );
        assert_eq!(
            read_view.tag_value(tag_id).unwrap(),
            &TagValue::Float(300.0)
        );

        // 2. Stage A writes new temperature (350.0) to WriteView
        write_view
            .mutate_equipment(eq_id, |e| e.set_var("temperature", 350.0))
            .unwrap();

        // 3. Stage B (downstream) can observe same-tick value from WriteView
        let stage_b_observed = write_view
            .get_equipment(eq_id)
            .unwrap()
            .get_var("temperature")
            .unwrap();
        assert_eq!(
            stage_b_observed, 350.0,
            "Downstream stage observes same-tick write"
        );

        // 4. ReadView is strictly isolated and still sees committed 300.0
        assert_eq!(
            read_view
                .equipment(eq_id)
                .unwrap()
                .get_var("temperature")
                .unwrap(),
            300.0,
            "Committed State[N] must remain unmutated"
        );
    }

    // Atomic Commit: State[N+1] committed into State[N]
    state.commit();

    let read_view_after = state.read_view();
    assert_eq!(
        read_view_after
            .equipment(eq_id)
            .unwrap()
            .get_var("temperature")
            .unwrap(),
        350.0,
        "After commit, State[N] contains new committed state"
    );
}
