use nebudcs_control::pid::{PidConfig, PidController, PidDirection, PidMode};

#[test]
fn test_pid_parallel_form_math() {
    let config = PidConfig {
        kp: 2.0,
        ti: 10.0,
        td: 0.0,
        derivative_filter_n: 10.0,
        out_min: 0.0,
        out_max: 100.0,
        direction: PidDirection::Reverse,
        sp_tracking_in_manual: false,
        scan_divisor: 1,
    };
    let mut pid = PidController::new(config);
    pid.set_mode(PidMode::Auto);
    pid.set_sp(50.0);

    // Initial step with dt = 1.0, PV = 40.0 (error = +10.0)
    // P = Kp * e = 2.0 * 10.0 = 20.0
    // I = Kp * dt / Ti * e = 2.0 * 1.0 / 10.0 * 10.0 = 2.0
    // Total OUT = 22.0
    let out = pid.step(1.0, 40.0);
    assert!((out - 22.0).abs() < 1e-9, "Expected 22.0, got {out}");
}

#[test]
fn test_derivative_filter_kick_prevention() {
    let config = PidConfig {
        kp: 2.0,
        ti: 10.0,
        td: 2.0,
        derivative_filter_n: 10.0,
        out_min: 0.0,
        out_max: 100.0,
        direction: PidDirection::Reverse,
        sp_tracking_in_manual: false,
        scan_divisor: 1,
    };
    let mut pid = PidController::new(config);
    pid.initialize(50.0);
    pid.set_mode(PidMode::Auto);
    pid.set_sp(50.0);

    // Step 1: steady state at PV = 50.0
    let _ = pid.step(1.0, 50.0);

    // Step 2: Step change in SP from 50.0 to 70.0, but PV remains 50.0!
    // Since derivative is on PV (not error), delta_PV = 0, so derivative kick must be 0.0!
    pid.set_sp(70.0);
    let _ = pid.step(1.0, 50.0);

    assert_eq!(
        pid.derivative(),
        0.0,
        "Derivative must be 0 when PV does not change, even if SP stepped!"
    );
}

#[test]
fn test_anti_windup_conditional_integration() {
    let config = PidConfig {
        kp: 10.0,
        ti: 2.0,
        td: 0.0,
        derivative_filter_n: 10.0,
        out_min: 0.0,
        out_max: 100.0,
        direction: PidDirection::Reverse,
        sp_tracking_in_manual: false,
        scan_divisor: 1,
    };
    let mut pid = PidController::new(config);
    pid.set_mode(PidMode::Auto);
    pid.set_sp(50.0);

    // Massive error: PV = 0.0 -> e = +50.0 -> P = 500.0, instantly saturates to 100.0
    for _ in 0..100 {
        pid.step(1.0, 0.0);
    }
    assert_eq!(pid.output(), 100.0);
    assert!(
        pid.is_high_saturated(),
        "Controller must register high saturation"
    );

    // Integral term must NOT have wound up to infinity! It must be clamped at or below out_max.
    assert!(
        pid.integral() <= 100.0,
        "Integral must be clamped to prevent windup, got {}",
        pid.integral()
    );

    // Now error reverses: PV = 100.0 -> e = -50.0
    // Output must immediately begin dropping on the very next step without lag from windup!
    let out = pid.step(1.0, 100.0);
    assert!(
        out < 100.0,
        "Output must immediately drop below 100.0 once error reverses, got {out}"
    );
}

#[test]
fn test_bumpless_transfer_man_to_auto() {
    let config = PidConfig {
        kp: 2.5,
        ti: 8.0,
        td: 1.0,
        derivative_filter_n: 10.0,
        out_min: 0.0,
        out_max: 100.0,
        direction: PidDirection::Reverse,
        sp_tracking_in_manual: true,
        scan_divisor: 1,
    };
    let mut pid = PidController::new(config);
    // In manual mode, operator drives valve to 55.0%
    pid.initialize(42.0);
    pid.set_manual_output(55.0);
    let out_man = pid.step(1.0, 42.0);
    assert_eq!(out_man, 55.0);

    // Operator switches to Auto mode
    pid.set_mode(PidMode::Auto);

    // First step in Auto mode
    let out_auto = pid.step(1.0, 42.0);
    assert!(
        (out_auto - 55.0).abs() < 1e-9,
        "Bumpless transfer violation! Expected 55.0, got {out_auto}"
    );
}

#[test]
fn test_bumpless_transfer_auto_to_man() {
    let config = PidConfig {
        kp: 1.0,
        ti: 10.0,
        td: 0.0,
        derivative_filter_n: 10.0,
        out_min: 0.0,
        out_max: 100.0,
        direction: PidDirection::Reverse,
        sp_tracking_in_manual: false,
        scan_divisor: 1,
    };
    let mut pid = PidController::new(config);
    pid.initialize(50.0);
    pid.set_mode(PidMode::Auto);
    pid.set_sp(60.0);

    // Run for a few steps to let output change
    let mut last_auto_out = 0.0;
    for _ in 0..5 {
        last_auto_out = pid.step(1.0, 50.0);
    }
    assert!(last_auto_out > 10.0);

    // Switch to Manual mode
    pid.set_mode(PidMode::Manual);
    let man_out = pid.step(1.0, 50.0);
    assert_eq!(
        man_out, last_auto_out,
        "Manual mode must inherit the exact last automatic output!"
    );
}

#[test]
fn test_pid_tracking_mode_during_override() {
    let config = PidConfig {
        kp: 1.0,
        ti: 10.0,
        td: 0.0,
        derivative_filter_n: 10.0,
        out_min: 0.0,
        out_max: 100.0,
        direction: PidDirection::Reverse,
        sp_tracking_in_manual: false,
        scan_divisor: 1,
    };
    let mut pid = PidController::new(config);
    pid.initialize(50.0);
    pid.set_mode(PidMode::Auto);
    pid.set_sp(50.0);
    let _ = pid.step(1.0, 50.0);

    // Emergency trip or interlock forces actuator to 0.0%
    pid.enter_tracking(0.0);
    assert_eq!(pid.step(1.0, 20.0), 0.0);

    // Even though error is huge (SP=50, PV=20 -> e=30), integral must not wind up
    for _ in 0..10 {
        assert_eq!(pid.step(1.0, 20.0), 0.0);
    }

    // Interlock reset: exit tracking mode
    pid.exit_tracking();

    // Controller resumes from tracked position (0.0%) bumplessly!
    let resume_out = pid.step(1.0, 20.0);
    // Since it just exited tracking from 0.0%, its output should rise smoothly from 0.0%
    // without an instantaneous jump to saturated 100%
    assert!(
        resume_out < 35.0,
        "Controller must not jump to saturation upon unlatching from override, got {resume_out}"
    );
}

#[test]
fn test_logic_primitives_ton_tof_sr() {
    use nebudcs_control::logic::{PermissiveGuard, RsLatch, SrLatch, TofTimer, TonTimer};

    // 1. TON Timer: 3.0 second preset
    let mut ton = TonTimer::new(3.0);
    assert!(!ton.step(1.0, true)); // 1.0s elapsed
    assert!(!ton.step(1.0, true)); // 2.0s elapsed
    assert!(ton.step(1.0, true)); // 3.0s elapsed -> ON!
    assert!(ton.step(1.0, true)); // still ON
    assert!(!ton.step(1.0, false)); // input dropped -> immediately OFF!

    // 2. TOF Timer: 2.0 second preset
    let mut tof = TofTimer::new(2.0);
    assert!(tof.step(1.0, true)); // input ON -> output ON
    assert!(tof.step(1.0, false)); // input went FALSE -> 1.0s elapsed < 2.0s -> still ON
    assert!(!tof.step(1.0, false)); // 2.0s elapsed >= 2.0s preset -> turns OFF!

    // 3. SR Latch (Set Dominant)
    let mut sr = SrLatch::new(false);
    sr.step(true, false); // Set
    assert!(sr.q());
    sr.step(true, true); // Both Set and Reset active -> Set Dominant means Q is TRUE
    assert!(sr.q());
    sr.step(false, true); // Only Reset active -> Q is FALSE
    assert!(!sr.q());

    // 4. RS Latch (Reset Dominant)
    let mut rs = RsLatch::new(false);
    rs.step(true, false);
    assert!(rs.q());
    rs.step(true, true); // Both Set and Reset active -> Reset Dominant means Q is FALSE
    assert!(!rs.q());

    // 5. Permissive Guard
    let mut guard = PermissiveGuard::new("PUMP_START_PERMISSIVE");
    guard.add_condition("lube_oil_pressure_ok", true);
    guard.add_condition("suction_valve_open", true);
    guard.add_condition("discharge_valve_closed", false); // not satisfied
    assert!(!guard.is_satisfied());
    guard.set_condition("discharge_valve_closed", true);
    assert!(guard.is_satisfied());
}

#[test]
fn test_cascade_slave_tracking() {
    use nebudcs_control::cascade::CascadePair;

    let master_cfg = PidConfig {
        kp: 1.0,
        ti: 20.0,
        td: 0.0,
        derivative_filter_n: 10.0,
        out_min: 0.0,
        out_max: 100.0,
        direction: PidDirection::Reverse,
        sp_tracking_in_manual: false,
        scan_divisor: 1,
    };
    let slave_cfg = PidConfig {
        kp: 2.0,
        ti: 5.0,
        td: 0.0,
        derivative_filter_n: 10.0,
        out_min: 0.0,
        out_max: 100.0,
        direction: PidDirection::Reverse,
        sp_tracking_in_manual: false,
        scan_divisor: 1,
    };

    let mut cascade = CascadePair::new(master_cfg, slave_cfg);
    cascade.master_mut().set_sp(80.0);
    cascade.slave_mut().set_mode(PidMode::Auto);
    cascade.slave_mut().set_sp(35.0);

    // When slave is in Auto, master output continuously tracks slave's working SP
    cascade.step(1.0, 75.0, 35.0);
    assert_eq!(
        cascade.master().output(),
        35.0,
        "Master output must track slave setpoint while slave is in Auto/Manual"
    );

    // Switch slave to Cascade mode
    cascade.slave_mut().set_mode(PidMode::Cascade);
    cascade.step(1.0, 75.0, 35.0);

    // When in Cascade, slave SP is driven by master output
    assert_eq!(
        cascade.slave().sp(),
        cascade.master().output(),
        "Slave setpoint must equal master output in Cascade mode"
    );
}

#[test]
fn test_actuator_5_tier_arbitration() {
    use nebudcs_control::actuator::{
        ActuatorArbiter, ActuatorCommand, CommandPriority, FailAction,
    };

    let mut arbiter = ActuatorArbiter::new(FailAction::FailClosed);

    // Add demands across tiers
    arbiter.submit(ActuatorCommand::new(CommandPriority::Auto, 65.0));
    arbiter.submit(ActuatorCommand::new(CommandPriority::Manual, 40.0));
    arbiter.submit(ActuatorCommand::new(CommandPriority::Permissive, 20.0));
    arbiter.submit(ActuatorCommand::new(CommandPriority::Interlock, 10.0));
    arbiter.submit(ActuatorCommand::new(CommandPriority::Esd, 0.0));

    // Priority 1 (ESD) must win
    let selected = arbiter.arbitrate().expect("arbitration command");
    assert_eq!(selected.priority, CommandPriority::Esd);
    assert_eq!(selected.target_position, 0.0);

    // Clear ESD: Priority 2 (Interlock) must win
    arbiter.clear_priority(CommandPriority::Esd);
    let selected = arbiter.arbitrate().expect("arbitration command");
    assert_eq!(selected.priority, CommandPriority::Interlock);
    assert_eq!(selected.target_position, 10.0);

    // Clear Interlock: Priority 3 (Permissive) must win
    arbiter.clear_priority(CommandPriority::Interlock);
    let selected = arbiter.arbitrate().expect("arbitration command");
    assert_eq!(selected.priority, CommandPriority::Permissive);
    assert_eq!(selected.target_position, 20.0);

    // Clear Permissive: Priority 4 (Manual) must win
    arbiter.clear_priority(CommandPriority::Permissive);
    let selected = arbiter.arbitrate().expect("arbitration command");
    assert_eq!(selected.priority, CommandPriority::Manual);
    assert_eq!(selected.target_position, 40.0);

    // Clear Manual: Priority 5 (Auto) must win
    arbiter.clear_priority(CommandPriority::Manual);
    let selected = arbiter.arbitrate().expect("arbitration command");
    assert_eq!(selected.priority, CommandPriority::Auto);
    assert_eq!(selected.target_position, 65.0);
}

#[test]
fn test_actuator_conflicting_commands_safe_state() {
    use nebudcs_control::actuator::{
        ActuatorArbiter, ActuatorCommand, CommandPriority, FailAction,
    };

    // For FailClosed: if two commands have same priority (e.g. two interlocks), lowest (close) wins
    let mut arbiter_fc = ActuatorArbiter::new(FailAction::FailClosed);
    arbiter_fc.submit(ActuatorCommand::new(CommandPriority::Interlock, 80.0));
    arbiter_fc.submit(ActuatorCommand::new(CommandPriority::Interlock, 15.0));
    let sel_fc = arbiter_fc.arbitrate().unwrap();
    assert_eq!(
        sel_fc.target_position, 15.0,
        "FailClosed must favor smaller position (closer to closed 0%)"
    );

    // For FailOpen: highest (open) wins
    let mut arbiter_fo = ActuatorArbiter::new(FailAction::FailOpen);
    arbiter_fo.submit(ActuatorCommand::new(CommandPriority::Interlock, 25.0));
    arbiter_fo.submit(ActuatorCommand::new(CommandPriority::Interlock, 90.0));
    let sel_fo = arbiter_fo.arbitrate().unwrap();
    assert_eq!(
        sel_fo.target_position, 90.0,
        "FailOpen must favor larger position (closer to open 100%)"
    );
}

#[test]
fn test_actuator_slew_rate_and_deadband() {
    use nebudcs_control::actuator::{ActuatorConfig, ActuatorModel, FailAction};

    // Stroke time = 5.0 seconds (from 0 to 100% takes 5.0s => 20% per sec => 2% per 100ms)
    // Deadband = 0.5%
    let config = ActuatorConfig {
        stroke_time: 5.0,
        deadband: 0.5,
        fail_action: FailAction::FailClosed,
    };
    let mut actuator = ActuatorModel::new(config, 0.0);

    // Apply step command to 100.0% with dt = 0.1s (100ms)
    let pos_1 = actuator.step(0.1, 100.0, true);
    assert!(
        (pos_1 - 2.0).abs() < 1e-6,
        "Expected 2.0% travel in 0.1s, got {pos_1}"
    );

    // Run for 2.4 more seconds (24 steps of 0.1s) -> total 2.5s -> position must be 50.0%
    let mut pos = pos_1;
    for _ in 0..24 {
        pos = actuator.step(0.1, 100.0, true);
    }
    assert!(
        (pos - 50.0).abs() < 1e-4,
        "Expected 50.0% after 2.5s, got {pos}"
    );

    // Test deadband: align to steady 50.0%, then command small oscillation within deadband (50.2% vs 50.0% is 0.2% < 0.5% deadband)
    actuator.set_actual_position(50.0);
    let pos_deadband = actuator.step(0.1, 50.2, true);
    assert_eq!(
        pos_deadband, 50.0,
        "Command change within deadband must produce zero stem movement!"
    );
}

#[test]
fn test_actuator_fail_safe_position() {
    use nebudcs_control::actuator::{ActuatorConfig, ActuatorModel, FailAction};

    let config_fc = ActuatorConfig {
        stroke_time: 2.0,
        deadband: 0.1,
        fail_action: FailAction::FailClosed,
    };
    let mut actuator_fc = ActuatorModel::new(config_fc, 75.0);

    // Simulate loss of instrument air (power_or_air_ok = false)
    // Actuator must drive to fail-closed position (0.0%)
    for _ in 0..30 {
        actuator_fc.step(0.1, 75.0, false);
    }
    assert_eq!(
        actuator_fc.actual_position(),
        0.0,
        "FailClosed actuator must drive to 0.0% upon air/power loss"
    );

    let config_fo = ActuatorConfig {
        stroke_time: 2.0,
        deadband: 0.1,
        fail_action: FailAction::FailOpen,
    };
    let mut actuator_fo = ActuatorModel::new(config_fo, 25.0);
    for _ in 0..30 {
        actuator_fo.step(0.1, 25.0, false);
    }
    assert_eq!(
        actuator_fo.actual_position(),
        100.0,
        "FailOpen actuator must drive to 100.0% upon air/power loss"
    );
}

#[test]
fn test_instrument_lag_and_damping_filter() {
    use nebudcs_control::instrument::{InstrumentConfig, InstrumentModel};

    let config = InstrumentConfig::builder()
        .lrv(0.0)
        .urv(100.0)
        .sensor_lag_tau(2.0)
        .damping_tau(1.0)
        .noise_std_dev(0.0)
        .build()
        .expect("valid instrument config");

    let mut instrument = InstrumentModel::new(config, 0.0);

    // Apply step change in physical variable: X_phys = 50.0
    // Sensor lag has tau = 2.0s, damping has tau = 1.0s
    let mut pv = 0.0;
    for _ in 0..10 {
        // Step 10 times with dt = 0.2s (total 2.0s = 1 time constant of sensor)
        pv = instrument.step(0.2, 50.0, 0.2);
    }
    // After 2.0s, signal should have responded smoothly and be between 15.0 and 35.0
    assert!(
        pv > 15.0 && pv < 40.0,
        "Expected exponential lag response around 1-2 tau, got {pv}"
    );

    // After 40 more seconds (total 42s >> 10 * tau), signal must reach 50.0
    for _ in 0..200 {
        pv = instrument.step(0.2, 50.0, 0.2);
    }
    let diff: f64 = pv - 50.0;
    assert!(diff.abs() < 1e-3, "Expected steady state ~50.0, got {pv}");
}

#[test]
fn test_instrument_config_rejects_zero_span() {
    use nebudcs_control::instrument::InstrumentConfig;

    let result = InstrumentConfig::builder()
        .lrv(50.0)
        .urv(50.0) // Zero span! URV == LRV
        .build();
    assert!(
        result.is_err(),
        "InstrumentConfig must reject URV == LRV zero-span configuration"
    );
}

#[test]
fn test_sensor_stale_and_frozen_detection() {
    use nebudcs_control::instrument::{InstrumentConfig, InstrumentModel};
    use nebudcs_model::tag::TagQuality;

    let config = InstrumentConfig::builder()
        .lrv(0.0)
        .urv(100.0)
        .scan_interval(0.1) // scan interval 100ms
        .sensor_lag_tau(0.0)
        .damping_tau(0.0)
        .build()
        .unwrap();

    let mut instrument = InstrumentModel::new(config, 50.0);

    // Stale test: scan interval is 0.1s. If elapsed time without scan exceeds 3 * 0.1s = 0.3s
    let _ = instrument.step(0.5, 50.0, 0.5); // dt = 0.5s > 3 * 0.1s
    assert_eq!(
        instrument.quality(),
        TagQuality::Bad,
        "Signal must degrade to Bad when scan elapsed time > 3 * T_scan"
    );

    // Reset to healthy
    instrument.clear_diagnostics();
    assert_eq!(instrument.quality(), TagQuality::Good);

    // Frozen test: 100 scans with exact same value while process variable has variance
    for i in 0..105 {
        // Physical variable changes between 49.0 and 51.0, but instrument is stuck at 50.0
        let phys = if i % 2 == 0 { 51.0 } else { 49.0 };
        // We simulate a stuck/frozen ADC reading of 50.0
        instrument.step_frozen(0.1, phys, 50.0);
    }
    assert_eq!(
        instrument.quality(),
        TagQuality::Uncertain,
        "Signal must be flagged Uncertain(Frozen) after 100 scans of identical value with active process variance"
    );
}

#[test]
fn test_protective_trip_latching_and_reset() {
    use nebudcs_control::protection::{ProtectionBlock, ProtectionConfig, ProtectionState};

    let config = ProtectionConfig::new("HIGH_PRESSURE_TRIP", 100.0, 5.0);
    let mut prot = ProtectionBlock::new(config);
    assert_eq!(prot.state(), ProtectionState::Armed);

    // PV rises to 105.0 > 100.0 -> Trips!
    prot.step(105.0, false);
    assert_eq!(prot.state(), ProtectionState::Tripped);
    assert!(prot.is_tripped());

    // PV drops back down to 98.0 (< 100.0), but no reset command and not below deadband (95.0)
    prot.step(98.0, false);
    assert_eq!(
        prot.state(),
        ProtectionState::Tripped,
        "Trip must remain latched even when condition clears!"
    );

    // Operator issues reset command while PV is 98.0 (> 95.0 reset deadband)
    prot.step(98.0, true);
    assert_eq!(
        prot.state(),
        ProtectionState::Tripped,
        "Reset must be rejected if condition has not cleared below reset deadband!"
    );

    // PV drops to 92.0 (< 95.0), and operator issues reset command -> Unlatches to Armed!
    prot.step(92.0, true);
    assert_eq!(
        prot.state(),
        ProtectionState::Armed,
        "Protection must return to Armed after reset command and condition cleared below deadband"
    );
}

#[test]
fn test_configurable_voting_degradation_1oo2() {
    use nebudcs_control::protection::{DegradationPolicy, SensorInput, VotingVoter2oo3};
    use nebudcs_model::tag::TagQuality;

    let voter = VotingVoter2oo3::new(100.0, DegradationPolicy::DegradeTo1oo2);

    // All 3 healthy: 1 above 100.0, 2 below -> Normal 2oo3 does NOT trip
    let inputs = [
        SensorInput::new(105.0, TagQuality::Good),
        SensorInput::new(95.0, TagQuality::Good),
        SensorInput::new(90.0, TagQuality::Good),
    ];
    assert!(!voter.evaluate(&inputs).trip);

    // Sensor 3 fails (Bad quality) -> Degrades to 1oo2!
    // Now among the remaining 2 healthy sensors (105.0 and 95.0), sensor 1 exceeds threshold!
    // Under 1oo2, 1 trip input causes a trip!
    let inputs_degraded = [
        SensorInput::new(105.0, TagQuality::Good),
        SensorInput::new(95.0, TagQuality::Good),
        SensorInput::new(0.0, TagQuality::Bad),
    ];
    let result = voter.evaluate(&inputs_degraded);
    assert!(result.is_degraded, "Voter must be in degraded state");
    assert!(
        result.trip,
        "1oo2 degraded voting must trip when 1 remaining sensor exceeds threshold"
    );
}

#[test]
fn test_configurable_voting_degradation_2oo2() {
    use nebudcs_control::protection::{DegradationPolicy, SensorInput, VotingVoter2oo3};
    use nebudcs_model::tag::TagQuality;

    let voter = VotingVoter2oo3::new(100.0, DegradationPolicy::DegradeTo2oo2);

    // Sensor 3 fails (Bad quality) -> Degrades to 2oo2!
    // Sensor 1 is 105.0 (tripped), sensor 2 is 95.0 (not tripped)
    // Under 2oo2, BOTH remaining sensors must agree to trip. Since sensor 2 is 95.0, no trip yet!
    let inputs_degraded = [
        SensorInput::new(105.0, TagQuality::Good),
        SensorInput::new(95.0, TagQuality::Good),
        SensorInput::new(0.0, TagQuality::Bad),
    ];
    let result = voter.evaluate(&inputs_degraded);
    assert!(result.is_degraded);
    assert!(
        !result.trip,
        "2oo2 degraded voting must NOT trip when only 1 of 2 healthy sensors exceeds threshold"
    );

    // Both sensor 1 and sensor 2 exceed threshold -> 2oo2 trips!
    let inputs_both_tripped = [
        SensorInput::new(105.0, TagQuality::Good),
        SensorInput::new(102.0, TagQuality::Good),
        SensorInput::new(0.0, TagQuality::Bad),
    ];
    let result2 = voter.evaluate(&inputs_both_tripped);
    assert!(
        result2.trip,
        "2oo2 degraded voting must trip when both remaining healthy sensors agree"
    );
}

#[test]
fn test_control_system_closed_loop_tick() {
    use nebudcs_control::actuator::{ActuatorConfig, FailAction};
    use nebudcs_control::instrument::InstrumentConfig;
    use nebudcs_control::pid::{PidConfig, PidDirection};
    use nebudcs_control::system::ControlSystem;
    use nebudcs_sim::{
        DoubleBufferedState, EquipmentState, SimulationSystem, StateBuffer, StepContext,
    };
    use nebudcs_types::{EquipmentId, SimulationTime, TagId};

    let mut ctrl_sys = ControlSystem::new();
    let valve_id = EquipmentId::new();
    let level_tag = TagId::new();
    let out_tag = TagId::new();

    // Register instrument measuring level
    let inst_cfg = InstrumentConfig::builder()
        .lrv(0.0)
        .urv(10.0)
        .sensor_lag_tau(0.0)
        .damping_tau(0.0)
        .build()
        .unwrap();
    ctrl_sys.register_instrument(valve_id, "level", level_tag, inst_cfg, 2.0);

    // Register actuator driving "opening"
    let act_cfg = ActuatorConfig {
        stroke_time: 1.0, // 100% per sec
        deadband: 0.1,
        fail_action: FailAction::FailClosed,
    };
    ctrl_sys.register_actuator(valve_id, "opening", act_cfg, 0.0, None);

    // Register PID controller (reverse: SP=5.0, PV=2.0 -> positive error increases opening)
    let pid_cfg = PidConfig {
        kp: 10.0,
        ti: 10.0,
        td: 0.0,
        derivative_filter_n: 10.0,
        out_min: 0.0,
        out_max: 100.0,
        direction: PidDirection::Reverse,
        sp_tracking_in_manual: false,
        scan_divisor: 1,
    };
    ctrl_sys.register_pid(level_tag, out_tag, valve_id, pid_cfg);

    // Set PID mode to Auto with setpoint 5.0
    let pid = ctrl_sys.pid_mut(out_tag).unwrap();
    pid.set_mode(nebudcs_control::pid::PidMode::Auto);
    pid.set_sp(5.0);

    // Set up initial state buffer
    let mut initial_state = StateBuffer::new();
    let mut eq_state = EquipmentState::new("running");
    eq_state.set_var("level", 2.0);
    eq_state.set_var("opening", 0.0);
    initial_state.equipment.insert(valve_id, eq_state);

    let mut dbs = DoubleBufferedState::new();
    dbs.initialize(initial_state);

    // Run 1 tick with dt = 0.1s
    dbs.prepare_next(SimulationTime::try_new(0.1).unwrap());
    let (read_view, mut write_view) = dbs.rw_views();
    let ctx = StepContext {
        dt: 0.1,
        tick_count: 0,
    };

    ctrl_sys
        .step(ctx, &read_view, &mut write_view)
        .expect("step succeeds");
    dbs.commit();

    // Check that PV tag was published
    let pv_val = dbs.read_view().tag_value(level_tag).unwrap();
    assert_eq!(pv_val, &nebudcs_model::tag::TagValue::Float(2.0));

    // Check that PID calculated output demand:
    // e = 5.0 - 2.0 = 3.0. P = 10.0 * 3.0 = 30.0.
    // I = 10.0 * 0.1 / 10.0 * 3.0 = 0.3.
    // Demand = 30.3%
    let out_val = dbs.read_view().tag_value(out_tag).unwrap();
    match out_val {
        nebudcs_model::tag::TagValue::Float(v) => assert!((*v - 30.3).abs() < 1e-6),
        _ => panic!("Expected float OUT"),
    }

    // Check that actuator opening moved toward 30.3%:
    // With 1.0s stroke time, max travel in 0.1s is 10.0%
    let updated_eq = dbs.read_view().equipment(valve_id).unwrap();
    let opening = updated_eq.get_var("opening").unwrap();
    assert!(
        (opening - 10.0).abs() < 1e-4,
        "Expected actuator opening to move to 10.0%, got {opening}"
    );
}

#[test]
fn test_topological_signal_tie_breaking() {
    use nebudcs_types::TagId;

    // Create 100 random TagIds (ULIDs)
    let mut tags: Vec<TagId> = (0..100).map(|_| TagId::new()).collect();

    // Sorting by TagId (Ulid Ord) produces strict deterministic order
    let mut sorted_1 = tags.clone();
    sorted_1.sort();

    // Reverse original and sort again
    tags.reverse();
    let mut sorted_2 = tags.clone();
    sorted_2.sort();

    assert_eq!(
        sorted_1, sorted_2,
        "ULID binary sort must guarantee 100% deterministic tie-breaking regardless of initial order"
    );
}

#[test]
fn test_benchmark_1000_tags_execution_budget() {
    use nebudcs_control::actuator::{ActuatorConfig, FailAction};
    use nebudcs_control::instrument::InstrumentConfig;
    use nebudcs_control::pid::PidConfig;
    use nebudcs_control::system::ControlSystem;
    use nebudcs_sim::{
        DoubleBufferedState, EquipmentState, SimulationSystem, StateBuffer, StepContext,
    };
    use nebudcs_types::{EquipmentId, SimulationTime, TagId};
    use std::time::Instant;

    let mut ctrl_sys = ControlSystem::new();
    let mut initial_state = StateBuffer::new();

    // Populate 250 control loops (each has 1 valve equipment, 1 PV tag, 1 OUT tag, 1 instrument, 1 actuator, 1 PID)
    // Total: 250 equipment, 500 tags, 250 PIDs, 250 instruments, 250 actuators
    for i in 0..250 {
        let eq_id = EquipmentId::new();
        let pv_tag = TagId::new();
        let out_tag = TagId::new();

        let inst_cfg = InstrumentConfig::builder()
            .lrv(0.0)
            .urv(100.0)
            .sensor_lag_tau(1.0)
            .damping_tau(0.5)
            .build()
            .unwrap();
        ctrl_sys.register_instrument(eq_id, "pressure", pv_tag, inst_cfg, 50.0);

        let act_cfg = ActuatorConfig {
            stroke_time: 3.0,
            deadband: 0.2,
            fail_action: FailAction::FailClosed,
        };
        ctrl_sys.register_actuator(eq_id, "opening", act_cfg, 50.0, None);

        let pid_cfg = PidConfig::default();
        ctrl_sys.register_pid(pv_tag, out_tag, eq_id, pid_cfg);

        let mut eq_state = EquipmentState::new("running");
        eq_state.set_var("pressure", 50.0 + (i as f64 % 10.0));
        eq_state.set_var("opening", 50.0);
        initial_state.equipment.insert(eq_id, eq_state);
    }

    let mut dbs = DoubleBufferedState::new();
    dbs.initialize(initial_state);

    // Warm-up run (5 ticks)
    for tick in 0..5 {
        dbs.prepare_next(SimulationTime::try_new(tick as f64 * 0.01).unwrap());
        let (read_view, mut write_view) = dbs.rw_views();
        let ctx = StepContext {
            dt: 0.01,
            tick_count: tick,
        };
        ctrl_sys.step(ctx, &read_view, &mut write_view).unwrap();
        dbs.commit();
    }

    // Benchmark 100 consecutive ticks
    let start = Instant::now();
    let num_ticks = 100;
    for tick in 0..num_ticks {
        dbs.prepare_next(SimulationTime::try_new((tick + 5) as f64 * 0.01).unwrap());
        let (read_view, mut write_view) = dbs.rw_views();
        let ctx = StepContext {
            dt: 0.01,
            tick_count: tick + 5,
        };
        ctrl_sys.step(ctx, &read_view, &mut write_view).unwrap();
        dbs.commit();
    }
    let elapsed = start.elapsed();
    let avg_tick_duration = elapsed / num_ticks as u32;

    println!(
        "Benchmark: 250 PID loops, 250 instruments, 250 actuators: avg tick duration = {:?}",
        avg_tick_duration
    );

    // The specification target budget is < 1.5 ms per tick
    assert!(
        avg_tick_duration.as_micros() < 1500,
        "Tick duration must be well below 1.5ms budget, got {:?}",
        avg_tick_duration
    );
}

#[test]
fn test_protective_trip_low_direction_and_reset() {
    use nebudcs_control::protection::{
        ProtectionBlock, ProtectionConfig, ProtectionState, TripDirection,
    };

    let config = ProtectionConfig::new_low("LOW_LEVEL_TRIP", 10.0, 2.0);
    assert_eq!(config.direction, TripDirection::Low);

    let mut prot = ProtectionBlock::new(config);
    assert_eq!(prot.state(), ProtectionState::Armed);

    // PV is 50.0 > 10.0 -> Normal active monitoring
    assert!(!prot.step(50.0, false));
    assert_eq!(prot.state(), ProtectionState::Active);

    // PV drops to 9.5 <= 10.0 -> Trips!
    assert!(prot.step(9.5, false));
    assert_eq!(prot.state(), ProtectionState::Tripped);
    assert!(prot.is_tripped());

    // Still in trip condition (PV = 10.5 <= threshold + deadband 12.0) -> Reset rejected, remains tripped
    assert!(prot.step(10.5, true));
    assert_eq!(prot.state(), ProtectionState::Tripped);

    // PV recovers to 13.0 >= 10.0 + 2.0 -> Reset succeeds with operator command
    assert!(!prot.step(13.0, true));
    assert_eq!(prot.state(), ProtectionState::Armed);
    assert!(!prot.is_tripped());
}
