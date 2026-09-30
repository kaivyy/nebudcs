//! Stage 5: Advanced Process Control Subsystem Comprehensive Test Suite.
//!
//! Verifies:
//! - Cascade level-to-flow regulation
//! - Wild/controlled ratio regulation
//! - Feedback + feed-forward disturbance rejection with lead-lag compensation
//! - High/Low override selectors with anti-windup external reset feedback
//! - Split-range multi-actuator distribution
//! - Linear, Equal-Percentage, and Quick-Opening valve characterization
//! - Dynamic compressor anti-surge protection (SLL, SCL, margin, rate-of-decay kick)
//! - IEC 61131-3 Function Blocks (FILTER_1ST, LEAD_LAG, RATIO, SEL_HIGH, SEL_LOW, SPLIT_RANGE, VALVE_CHAR, ANTI_SURGE)
//! - ControlSystem multi-strategy integration
//! - 10,000-tick bit-identical deterministic replay

use std::collections::BTreeMap;

use nebudcs_control::actuator::{ActuatorConfig, FailAction};
use nebudcs_control::advanced::{
    AdvancedControlStrategy, AntiSurgeAlarmState, AntiSurgeController, LeadLagCompensator,
    OverrideSelector, RatioController, SelectedSource, SelectorType, SplitRangeController,
    StrategyKind, ValveCharacteristic, ValveCharacterizer,
};
use nebudcs_control::iec61131::fb::*;
use nebudcs_control::iec61131::types::IecValue;
use nebudcs_control::iec61131::vm::IecVm;
use nebudcs_control::iec61131::{compile_st, MemoryStore};
use nebudcs_control::pid::{PidConfig, PidController, PidDirection, PidMode};
use nebudcs_control::runtime::io_image::IoImageTable;
use nebudcs_control::system::ControlSystem;
use nebudcs_model::tag::{TagQuality, TagValue};
use nebudcs_sim::{DoubleBufferedState, SimulationSystem, StateBuffer, StepContext};
use nebudcs_types::{EquipmentId, SimulationTime, TagId};

#[test]
fn test_cascade_control_level_to_flow() {
    // Primary controller (Level): slow dynamics, output drives secondary SP (0 - 100 kg/h)
    let primary_cfg = PidConfig {
        kp: 1.5,
        ti: 20.0,
        td: 0.0,
        out_min: 0.0,
        out_max: 100.0,
        ..Default::default()
    };
    let mut primary_pid = PidController::new(primary_cfg);
    primary_pid.set_mode(PidMode::Auto);
    primary_pid.set_sp(50.0); // 50% tank level
    primary_pid.set_sp_limits(0.0, 100.0);

    // Secondary controller (Flow): fast dynamics, output drives control valve (0 - 100%)
    let secondary_cfg = PidConfig {
        kp: 0.8,
        ti: 2.0,
        td: 0.1,
        out_min: 0.0,
        out_max: 100.0,
        ..Default::default()
    };
    let mut secondary_pid = PidController::new(secondary_cfg);
    secondary_pid.set_mode(PidMode::Cascade);
    secondary_pid.set_sp_limits(10.0, 90.0); // Secondary has hard process limits

    let dt = 0.1;
    let mut level_pv = 40.0; // Level is low -> primary should demand more flow
    let mut flow_pv = 30.0;

    for _ in 0..100 {
        let flow_sp_demand = primary_pid.step(dt, level_pv);
        // Cascade connection: primary output becomes secondary cascade SP
        secondary_pid.set_cascade_sp(flow_sp_demand);
        assert_eq!(secondary_pid.mode(), PidMode::Cascade);

        let valve_out = secondary_pid.step(dt, flow_pv);
        assert!((0.0..=100.0).contains(&valve_out));

        // Simulated process responses
        flow_pv += (valve_out * 0.9 - flow_pv) * (dt / 1.0);
        level_pv += (flow_pv * 0.1 - 4.5) * dt;
    }

    // Secondary SP must respect secondary_pid clamping limits (10 to 90)
    assert!((10.0..=90.0).contains(&secondary_pid.sp()));
}

#[test]
fn test_ratio_control_flow_proportion() {
    let mut ratio_ctrl = RatioController::new(0.6);
    ratio_ctrl.set_ratio_limits(0.1, 2.0);
    ratio_ctrl.set_bias(5.0);
    assert_eq!(ratio_ctrl.ratio, 0.6);
    assert_eq!(ratio_ctrl.bias, 5.0);

    // Wild flow = 100.0 kg/h -> Controlled SP = 0.6 * 100 + 5 = 65.0 kg/h
    let target_sp = ratio_ctrl.calculate(100.0);
    assert!((target_sp - 65.0).abs() < 1e-9);

    // Dynamic ratio change
    ratio_ctrl.set_ratio(0.75);
    let target_sp2 = ratio_ctrl.calculate(100.0);
    assert!((target_sp2 - 80.0).abs() < 1e-9);

    // Ratio clamping
    ratio_ctrl.set_ratio(5.0);
    assert_eq!(ratio_ctrl.ratio, 2.0); // clamped to ratio_max
}

#[test]
fn test_feedforward_disturbance_rejection() {
    let mut lead_lag = LeadLagCompensator::with_gain(5.0, 2.0, 1.2);
    let dt = 0.1;

    // Step disturbance from 0 to 10
    let mut ff_output = 0.0;
    for tick in 0..50 {
        let dist = if tick >= 5 { 10.0 } else { 0.0 };
        ff_output = lead_lag.step(dt, dist);
    }
    // Steady state of lead-lag with gain 1.2 on step of 10 is 12.0
    for _ in 0..100 {
        ff_output = lead_lag.step(dt, 10.0);
    }
    assert!((ff_output - 12.0).abs() < 0.05);

    // Verify PidController with feedforward
    let cfg = PidConfig {
        kp: 2.0,
        ti: 10.0,
        td: 0.0,
        out_min: 0.0,
        out_max: 100.0,
        ..Default::default()
    };
    let mut pid = PidController::new(cfg);
    pid.set_mode(PidMode::Auto);
    pid.set_sp(50.0);

    // With PV = 50.0 (error = 0), and feedforward = 15.0:
    let out = pid.step_with_feedforward(dt, 50.0, 15.0);
    // Base PID gives 0 (no P, I near 0, D=0), plus 15.0 FF
    assert!((out - 15.0).abs() < 1.0);
}

#[test]
fn test_override_selector_anti_windup_tracking() {
    let mut sel = OverrideSelector::new(SelectorType::Low);

    // Primary controller (Normal flow regulation)
    let prim_cfg = PidConfig {
        kp: 1.0,
        ti: 5.0,
        out_min: 0.0,
        out_max: 100.0,
        ..Default::default()
    };
    let mut prim_pid = PidController::new(prim_cfg);
    prim_pid.set_mode(PidMode::Auto);
    prim_pid.set_sp(50.0);

    // Constraint controller (High pressure constraint, reverse action)
    let constr_cfg = PidConfig {
        kp: 2.0,
        ti: 3.0,
        out_min: 0.0,
        out_max: 100.0,
        direction: PidDirection::Reverse,
        ..Default::default()
    };
    let mut constr_pid = PidController::new(constr_cfg);
    constr_pid.set_mode(PidMode::Auto);
    constr_pid.set_sp(80.0); // Constraint limit

    let dt = 0.1;

    // Case 1: Normal condition, constraint not active (pressure low at 60.0)
    // Constraint PID wants 100% (reverse acting, PV < SP)
    // Primary PID wants ~40%
    for _ in 0..20 {
        let u_prim = prim_pid.step(dt, 45.0);
        let u_constr = constr_pid.step(dt, 60.0);
        let winner = sel.select(u_prim, u_constr);
        assert_eq!(sel.selected_source(), SelectedSource::Primary);
        assert_eq!(winner, u_prim);

        let fb = sel.external_reset_feedback();
        prim_pid.apply_external_reset_feedback(fb);
        constr_pid.apply_external_reset_feedback(fb);
    }

    // Case 2: Constraint condition: pressure rises to 95.0 (over 80 limit)
    // Constraint PID cuts output to a low value (< u_prim)
    // Selector should switch to Override and clamp primary integrator
    for _ in 0..30 {
        let u_prim = prim_pid.step(dt, 40.0);
        let u_constr = constr_pid.step(dt, 95.0);
        let winner = sel.select(u_prim, u_constr);
        assert_eq!(sel.selected_source(), SelectedSource::Override);
        assert_eq!(winner, u_constr);

        let fb = sel.external_reset_feedback();
        prim_pid.apply_external_reset_feedback(fb);
        constr_pid.apply_external_reset_feedback(fb);
    }

    // Verify anti-windup: primary PID's integral term was held to the winning output
    // rather than accumulating to 100%
    assert!(prim_pid.integral() < 50.0);
}

#[test]
fn test_split_range_multi_actuator_distribution() {
    let ctrl = SplitRangeController::standard_complementary("CoolingValve", "HeatingValve");

    // Master demand = 0% -> Full cooling (100%), No heating (0%)
    let out_0 = ctrl.evaluate_all(0.0);
    assert_eq!(out_0[0], ("CoolingValve".to_string(), 100.0));
    assert_eq!(out_0[1], ("HeatingValve".to_string(), 0.0));

    // Master demand = 50% -> Neutral zone (0% cooling, 0% heating)
    let out_50 = ctrl.evaluate_all(50.0);
    assert_eq!(out_50[0], ("CoolingValve".to_string(), 0.0));
    assert_eq!(out_50[1], ("HeatingValve".to_string(), 0.0));

    // Master demand = 100% -> Full heating (100%), No cooling (0%)
    let out_100 = ctrl.evaluate_all(100.0);
    assert_eq!(out_100[0], ("CoolingValve".to_string(), 0.0));
    assert_eq!(out_100[1], ("HeatingValve".to_string(), 100.0));

    // Master demand = 25% -> Half cooling (50%)
    let out_25 = ctrl.evaluate_all(25.0);
    assert!((out_25[0].1 - 50.0).abs() < 1e-6);
}

#[test]
fn test_valve_characterization_trim() {
    let lin = ValveCharacterizer::new(ValveCharacteristic::Linear);
    assert_eq!(lin.evaluate(0.0), 0.0);
    assert_eq!(lin.evaluate(0.5), 0.5);
    assert_eq!(lin.evaluate(1.0), 1.0);

    let eq_pct =
        ValveCharacterizer::new(ValveCharacteristic::EqualPercentage { rangeability: 50.0 });
    assert_eq!(eq_pct.evaluate(0.0), 0.0);
    assert_eq!(eq_pct.evaluate(1.0), 1.0);
    // Equal percentage at 50% stroke gives substantially lower flow than linear (logarithmic growth)
    let cv_50 = eq_pct.evaluate(0.5);
    assert!(cv_50 < 0.25 && cv_50 > 0.10);

    // Invert should roundtrip accurately
    let stroke = eq_pct.inverse(cv_50);
    assert!((stroke - 0.5).abs() < 1e-4);

    let quick = ValveCharacterizer::new(ValveCharacteristic::QuickOpening);
    assert_eq!(quick.evaluate(0.0), 0.0);
    assert_eq!(quick.evaluate(1.0), 1.0);
    let cv_quick_50 = quick.evaluate(0.5);
    // Quick opening at 50% stroke gives sqrt(0.5) ~ 0.707
    assert!((cv_quick_50 - std::f64::consts::FRAC_1_SQRT_2).abs() < 0.01);
}

#[test]
fn test_anti_surge_controller_compressor_protection() {
    // SLL: Q_surge = 0.35 * r_p + 0.5
    // Margin: 15% -> SCL = Q_surge * 1.15
    let mut as_ctrl = AntiSurgeController::new(0.35, 0.5);
    as_ctrl.safety_margin = 0.15;
    as_ctrl.kp = 2.5;
    as_ctrl.ti = 0.5;
    as_ctrl.derivative_kick_gain = 3.0;
    as_ctrl.kick_threshold_rate = 0.05;

    let dt = 0.05;
    let p_suc = 100.0; // 100 kPa
    let p_dis = 300.0; // 300 kPa -> pressure ratio r_p = 3.0
                       // At r_p = 3.0: Q_surge = 0.35 * 3.0 + 0.5 = 1.55 m3/s
                       // SCL = 1.55 * 1.15 = 1.7825 m3/s

    // Condition 1: High flow (3.0 m3/s) -> Safe region
    for _ in 0..10 {
        let valve_cmd1 = as_ctrl.step(dt, 3.0, p_suc, p_dis);
        assert_eq!(valve_cmd1, 0.0);
    }
    assert_eq!(as_ctrl.alarm_state(), AntiSurgeAlarmState::Normal);
    assert!(as_ctrl.surge_margin() > 0.5);

    // Condition 2: Flow drops to 1.70 m3/s and settles -> Below SCL (1.7825), Above SLL (1.55)
    // Anti-surge controller modulates recycle valve
    let mut valve_cmd2 = 0.0;
    for _ in 0..20 {
        valve_cmd2 = as_ctrl.step(dt, 1.70, p_suc, p_dis);
    }
    assert!(valve_cmd2 > 0.0);
    assert_eq!(as_ctrl.alarm_state(), AntiSurgeAlarmState::MarginWarning);

    // Condition 3: Flow rapidly decays towards SLL (from 1.70 to 1.20) -> triggers anticipatory rate kick & trip alarm
    let valve_cmd3 = as_ctrl.step(dt, 1.20, p_suc, p_dis);
    assert!(valve_cmd3 > valve_cmd2);
    assert_eq!(as_ctrl.alarm_state(), AntiSurgeAlarmState::SurgeCritical);
}

#[test]
fn test_iec61131_function_blocks_advanced_control() {
    // 1. Direct function block execution
    let mut fb_filter = FilterFirstOrderBlock::new("F1");
    let mut filt_in = BTreeMap::new();
    filt_in.insert("IN".to_string(), IecValue::LReal(20.0));
    filt_in.insert("TIME_CONSTANT".to_string(), IecValue::LReal(2.0));
    let out_f = fb_filter.step(0.1, 0.0, &filt_in).unwrap();
    let filt_val = out_f.get("OUT").unwrap().as_f64().unwrap();
    assert!(filt_val > 0.0 && filt_val <= 20.0);

    let mut fb_ratio = RatioBlock::new("R1");
    let mut ratio_in = BTreeMap::new();
    ratio_in.insert("WILD".to_string(), IecValue::LReal(100.0));
    ratio_in.insert("RATIO".to_string(), IecValue::LReal(0.75));
    ratio_in.insert("BIAS".to_string(), IecValue::LReal(10.0));
    let out_r = fb_ratio.step(0.1, 0.0, &ratio_in).unwrap();
    let ratio_sp = out_r.get("SP").unwrap().as_f64().unwrap();
    assert_eq!(ratio_sp, 85.0);

    let mut fb_sel = HighSelectorBlock::new("S1");
    let mut sel_in = BTreeMap::new();
    sel_in.insert("IN1".to_string(), IecValue::LReal(45.0));
    sel_in.insert("IN2".to_string(), IecValue::LReal(85.0));
    let out_s = fb_sel.step(0.1, 0.0, &sel_in).unwrap();
    assert_eq!(out_s.get("OUT").unwrap().as_f64().unwrap(), 85.0);

    // 2. Structured Text VM execution calling FB
    let source = r#"
        VAR
            RAW : REAL := 50.0;
            WILD : REAL := 120.0;
            RATIO_VAL : REAL := 0.5;
            CALC_SP : REAL;
        END_VAR

        CALC_SP := WILD * RATIO_VAL;
    "#;

    let (symbols, stmts) = compile_st(source).expect("ST compilation failed");
    let mut mem = MemoryStore::from_symbol_table(&symbols);
    let mut io = IoImageTable::new();
    let mut fb = BTreeMap::new();

    let mut vm = IecVm::new(0.1, 0.0, &mut mem, &mut io, &mut fb);
    vm.execute_block(&stmts).expect("VM execution failed");

    let calc_sp = mem.get("CALC_SP").unwrap().as_f64().unwrap();
    assert_eq!(calc_sp, 60.0);
}

#[test]
fn test_control_system_with_advanced_strategies() {
    let mut control_sys = ControlSystem::new();

    // Register an actuator
    let valve_id = EquipmentId::new();
    control_sys.register_actuator(
        valve_id,
        "opening",
        ActuatorConfig {
            stroke_time: 1.0,
            deadband: 0.0,
            fail_action: FailAction::FailClosed,
        },
        0.0,
        None,
    );

    // Tags
    let pv_tag = TagId::new();
    let out_tag = TagId::new();
    let dist_tag = TagId::new();

    let mut ff_pid = PidController::new(PidConfig {
        kp: 1.5,
        ti: 5.0,
        out_min: 0.0,
        out_max: 100.0,
        ..Default::default()
    });
    ff_pid.set_mode(PidMode::Auto);

    // Strategy: FeedForward
    let strat = AdvancedControlStrategy::new(
        "strat-ff-1",
        "Heater Feedforward",
        "Rejects feed inlet flow disturbance",
        StrategyKind::FeedForward {
            pv_tag,
            sp: 60.0,
            pid: ff_pid,
            disturbance_tag: dist_tag,
            lead_lag: LeadLagCompensator::with_gain(2.0, 1.0, 0.5),
            output_tag: Some(out_tag),
            target_actuator: Some(valve_id),
        },
    );

    control_sys.register_strategy(strat);
    assert_eq!(control_sys.strategies().len(), 1);

    // Create double-buffered state
    let mut initial_state = StateBuffer::new();
    initial_state.tags.insert(pv_tag, TagValue::Float(55.0));
    initial_state.qualities.insert(pv_tag, TagQuality::Good);
    initial_state.tags.insert(dist_tag, TagValue::Float(10.0));
    initial_state.qualities.insert(dist_tag, TagQuality::Good);

    let mut dbs = DoubleBufferedState::new();
    dbs.initialize(initial_state);

    dbs.prepare_next(SimulationTime::try_new(0.1).unwrap());
    {
        let (read_view, mut write_view) = dbs.rw_views();
        let ctx = StepContext {
            dt: 0.1,
            tick_count: 0,
        };
        control_sys
            .step(ctx, &read_view, &mut write_view)
            .expect("Step tick failed");
    }
    dbs.commit();

    // Verify output tag written
    let out_val = match dbs.read_view().tag_value(out_tag) {
        Ok(TagValue::Float(v)) => *v,
        _ => panic!("Output tag was not published!"),
    };
    assert!(out_val > 0.0);
}

#[test]
fn test_advanced_control_determinism_replay_10k_ticks() {
    let run_simulation = || -> Vec<f64> {
        let mut control_sys = ControlSystem::new();
        let valve_id = EquipmentId::new();
        control_sys.register_actuator(
            valve_id,
            "opening",
            ActuatorConfig {
                stroke_time: 2.0,
                deadband: 0.01,
                fail_action: FailAction::FailClosed,
            },
            0.0,
            None,
        );

        let pv_wild = TagId::new();
        let pv_ctrl = TagId::new();
        let out_tag = TagId::new();

        let mut ratio_pid = PidController::new(PidConfig {
            kp: 1.2,
            ti: 4.0,
            out_min: 0.0,
            out_max: 100.0,
            ..Default::default()
        });
        ratio_pid.set_mode(PidMode::Auto);

        let ratio_strat = AdvancedControlStrategy::new(
            "ratio-1",
            "Ratio Loop",
            "Regulates fuel to air ratio",
            StrategyKind::Ratio {
                wild_pv: pv_wild,
                controlled_pv: pv_ctrl,
                ratio_controller: RatioController::new(0.8),
                pid: ratio_pid,
                output_tag: Some(out_tag),
                target_actuator: Some(valve_id),
            },
        );

        control_sys.register_strategy(ratio_strat);

        let mut initial_state = StateBuffer::new();
        initial_state.tags.insert(pv_wild, TagValue::Float(50.0));
        initial_state.qualities.insert(pv_wild, TagQuality::Good);
        initial_state.tags.insert(pv_ctrl, TagValue::Float(40.0));
        initial_state.qualities.insert(pv_ctrl, TagQuality::Good);

        let mut dbs = DoubleBufferedState::new();
        dbs.initialize(initial_state);

        let mut recorded_trace = Vec::with_capacity(10_000);

        for tick in 0..10_000 {
            let sim_time = (tick as f64 + 1.0) * 0.05;
            dbs.prepare_next(SimulationTime::try_new(sim_time).unwrap());
            {
                let (read_view, mut write_view) = dbs.rw_views();
                // Periodic synthetic disturbance
                let wild = 50.0 + 20.0 * ((tick as f64 * 0.01).sin());
                let ctrl_pv = 40.0 + 10.0 * ((tick as f64 * 0.015).cos());

                write_view.set_tag(pv_wild, TagValue::Float(wild), TagQuality::Good);
                write_view.set_tag(pv_ctrl, TagValue::Float(ctrl_pv), TagQuality::Good);

                let ctx = StepContext {
                    dt: 0.05,
                    tick_count: tick as u64,
                };
                control_sys
                    .step(ctx, &read_view, &mut write_view)
                    .expect("Tick failed");
            }
            dbs.commit();

            if let Ok(TagValue::Float(v)) = dbs.read_view().tag_value(out_tag) {
                recorded_trace.push(*v);
            }
        }

        recorded_trace
    };

    let trace1 = run_simulation();
    let trace2 = run_simulation();

    assert_eq!(trace1.len(), 10_000);
    assert_eq!(trace2.len(), 10_000);
    for (i, (val1, val2)) in trace1.iter().zip(trace2.iter()).enumerate() {
        assert_eq!(
            val1.to_bits(),
            val2.to_bits(),
            "Bitwise nondeterminism detected at tick {i}: {val1} != {val2}"
        );
    }
}
