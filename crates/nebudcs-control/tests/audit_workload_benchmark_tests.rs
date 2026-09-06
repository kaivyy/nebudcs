use std::time::Instant;

use nebudcs_control::actuator::{ActuatorConfig, CommandPriority, FailAction};
use nebudcs_control::instrument::InstrumentConfig;
use nebudcs_control::pid::{PidConfig, PidDirection, PidMode};
use nebudcs_control::protection::ProtectionConfig;
use nebudcs_control::system::ControlSystem;
use nebudcs_sim::{EquipmentState, SimulationEngine, StateBuffer};
use nebudcs_types::{EquipmentId, TagId};

#[test]
fn test_standard_and_large_workload_benchmarks() {
    // ---------------------------------------------------------------------
    // Workload 1: Standard 250 PIDs / 250 Instruments / 250 Actuators
    // ---------------------------------------------------------------------
    let dt = 0.05;
    let mut engine = SimulationEngine::new(dt).unwrap();
    let mut ctrl = ControlSystem::new();
    let mut initial_state = StateBuffer::new();

    let num_loops = 250;
    for i in 0..num_loops {
        let eq_id = EquipmentId::new();
        let pv_tag = TagId::new();
        let out_tag = TagId::new();

        let inst_cfg = InstrumentConfig::builder()
            .lrv(0.0)
            .urv(100.0)
            .sensor_lag_tau(1.0)
            .damping_tau(0.5)
            .noise_std_dev(0.01)
            .scan_interval(dt)
            .build()
            .unwrap();
        ctrl.register_instrument(eq_id, "pv", pv_tag, inst_cfg, 50.0);

        let act_cfg = ActuatorConfig {
            stroke_time: 3.0,
            deadband: 0.1,
            fail_action: FailAction::FailClosed,
        };
        ctrl.register_actuator(eq_id, "opening", act_cfg, 50.0, None);

        let pid_cfg = PidConfig {
            kp: 1.5,
            ti: 10.0,
            td: 0.2,
            derivative_filter_n: 10.0,
            out_min: 0.0,
            out_max: 100.0,
            direction: PidDirection::Direct,
            sp_tracking_in_manual: false,
            scan_divisor: 1,
        };
        ctrl.register_pid(pv_tag, out_tag, eq_id, pid_cfg);
        ctrl.pid_mut(out_tag).unwrap().initialize(50.0);
        ctrl.pid_mut(out_tag).unwrap().set_mode(PidMode::Auto);
        ctrl.pid_mut(out_tag).unwrap().set_sp(55.0);

        let mut eq = EquipmentState::new("running");
        eq.set_var("pv", 50.0 + (i as f64 * 0.1));
        eq.set_var("opening", 50.0);
        initial_state.equipment.insert(eq_id, eq);
    }

    engine.add_system(ctrl);
    engine.initialize(initial_state);

    // Warm-up: 50 ticks
    for _ in 0..50 {
        engine.step().unwrap();
    }

    // Benchmark: 1,000 ticks
    let num_ticks = 1000;
    let mut durations = Vec::with_capacity(num_ticks);

    for _ in 0..num_ticks {
        let t0 = Instant::now();
        engine.step().unwrap();
        durations.push(t0.elapsed());
    }

    durations.sort();

    let total_nanos: u128 = durations.iter().map(|d| d.as_nanos()).sum();
    let mean_us = (total_nanos as f64 / num_ticks as f64) / 1000.0;
    let median_us = durations[num_ticks / 2].as_nanos() as f64 / 1000.0;
    let p95_us = durations[num_ticks * 95 / 100].as_nanos() as f64 / 1000.0;
    let p99_us = durations[num_ticks * 99 / 100].as_nanos() as f64 / 1000.0;
    let worst_us = durations.last().unwrap().as_nanos() as f64 / 1000.0;

    println!("\n=== Workload 1: 250 PIDs / 250 Instruments / 250 Actuators ===");
    println!("  Mean tick duration:   {mean_us:.2} µs");
    println!("  Median (p50):         {median_us:.2} µs");
    println!("  p95:                  {p95_us:.2} µs");
    println!("  p99:                  {p99_us:.2} µs");
    println!("  Worst observed:       {worst_us:.2} µs");
    println!("  Target Budget:        1500.00 µs (1.5 ms)");

    // ---------------------------------------------------------------------
    // Workload 2: Large Mixed Workload (1,000 Tags, 250 PIDs, 100 Interlocks)
    // ---------------------------------------------------------------------
    let mut engine_large = SimulationEngine::new(dt).unwrap();
    let mut ctrl_large = ControlSystem::new();
    let mut initial_large = StateBuffer::new();

    for i in 0..250 {
        let eq_id = EquipmentId::new();
        let pv_tag = TagId::new();
        let out_tag = TagId::new();

        let inst_cfg = InstrumentConfig::builder()
            .lrv(0.0)
            .urv(100.0)
            .sensor_lag_tau(1.0)
            .damping_tau(0.5)
            .noise_std_dev(0.01)
            .scan_interval(dt)
            .build()
            .unwrap();
        ctrl_large.register_instrument(eq_id, "pv", pv_tag, inst_cfg, 50.0);

        let act_cfg = ActuatorConfig {
            stroke_time: 3.0,
            deadband: 0.1,
            fail_action: FailAction::FailClosed,
        };
        ctrl_large.register_actuator(eq_id, "opening", act_cfg, 50.0, None);

        let pid_cfg = PidConfig {
            kp: 1.5,
            ti: 10.0,
            td: 0.2,
            derivative_filter_n: 10.0,
            out_min: 0.0,
            out_max: 100.0,
            direction: PidDirection::Direct,
            sp_tracking_in_manual: false,
            scan_divisor: 1,
        };
        ctrl_large.register_pid(pv_tag, out_tag, eq_id, pid_cfg);
        ctrl_large.pid_mut(out_tag).unwrap().initialize(50.0);
        ctrl_large.pid_mut(out_tag).unwrap().set_mode(PidMode::Auto);
        ctrl_large.pid_mut(out_tag).unwrap().set_sp(55.0);

        if i < 100 {
            let prot_cfg = ProtectionConfig {
                name: format!("PROT_{i}"),
                threshold: 95.0,
                reset_deadband: 5.0,
            };
            ctrl_large.register_protection(
                format!("PROT_{i}"),
                pv_tag,
                eq_id,
                CommandPriority::Interlock,
                0.0,
                prot_cfg,
            );
        }

        let mut eq = EquipmentState::new("running");
        eq.set_var("pv", 50.0 + (i as f64 * 0.1));
        eq.set_var("opening", 50.0);
        initial_large.equipment.insert(eq_id, eq);
    }

    // Add extra tags to reach 1,000 tags in StateBuffer
    for i in 0..500 {
        let tag = TagId::new();
        initial_large
            .tags
            .insert(tag, nebudcs_model::tag::TagValue::Float(i as f64));
        initial_large
            .qualities
            .insert(tag, nebudcs_model::tag::TagQuality::Good);
    }

    engine_large.add_system(ctrl_large);
    engine_large.initialize(initial_large);

    // Warm-up: 50 ticks
    for _ in 0..50 {
        engine_large.step().unwrap();
    }

    let mut large_durations = Vec::with_capacity(num_ticks);
    for _ in 0..num_ticks {
        let t0 = Instant::now();
        engine_large.step().unwrap();
        large_durations.push(t0.elapsed());
    }

    large_durations.sort();

    let large_total_nanos: u128 = large_durations.iter().map(|d| d.as_nanos()).sum();
    let large_mean_us = (large_total_nanos as f64 / num_ticks as f64) / 1000.0;
    let large_median_us = large_durations[num_ticks / 2].as_nanos() as f64 / 1000.0;
    let large_p95_us = large_durations[num_ticks * 95 / 100].as_nanos() as f64 / 1000.0;
    let large_p99_us = large_durations[num_ticks * 99 / 100].as_nanos() as f64 / 1000.0;
    let large_worst_us = large_durations.last().unwrap().as_nanos() as f64 / 1000.0;

    println!("\n=== Workload 2: 1,000 Tags / 250 PIDs / 100 Interlocks ===");
    println!("  Mean tick duration:   {large_mean_us:.2} µs");
    println!("  Median (p50):         {large_median_us:.2} µs");
    println!("  p95:                  {large_p95_us:.2} µs");
    println!("  p99:                  {large_p99_us:.2} µs");
    println!("  Worst observed:       {large_worst_us:.2} µs");
    println!("  Target Budget:        1500.00 µs (1.5 ms)");

    assert!(
        mean_us < 1500.0,
        "Workload 1 mean tick duration ({mean_us} µs) must be < 1500 µs"
    );
    assert!(
        large_mean_us < 1500.0,
        "Workload 2 mean tick duration ({large_mean_us} µs) must be < 1500 µs"
    );
}
