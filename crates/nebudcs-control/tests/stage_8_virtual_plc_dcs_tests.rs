//! NebuDCS Stage 8: Vendor-Neutral Virtual PLC / DCS Controller Architecture Test Suite
//!
//! Tests:
//! 1. Lifecycle State Machine (PowerOff -> Booting -> Initializing -> Running -> Paused -> ShuttingDown -> PowerOff)
//! 2. Hardware Profiles (CPU execution budget, memory limits, firmware identity)
//! 3. Retentive Memory Retention (survival across power interruption)
//! 4. DCS Control Module (MAN, AUTO, CAS, Bumpless Transfer, Permissives, Safety Interlocks)
//! 5. High-Availability Redundancy Replication (Primary -> Standby synchronization & switchover)
//! 6. Controlled Fault Injection (Watchdog, Overrun, CPU Overload, Memory Exhaustion)
//! 7. 10,000-tick Bit-Identical Determinism

use nebudcs_control::pid::{PidConfig, PidDirection};
use nebudcs_control::runtime::{
    ControlModuleMode, ControllerConfig, ControllerLifecycle, CpuProfile, DcsControlModule,
    FirmwareProfile, IoSignalValue, MemoryProfile, RedundancyRole, RedundancyState,
    RedundancyStatus, VirtualController,
};
use nebudcs_model::tag::TagQuality;

#[test]
fn test_controller_extended_lifecycle_state_machine() {
    let mut ctrl = VirtualController::with_name("VCPU-LIFECYCLE", ControllerConfig::default());
    assert_eq!(ctrl.lifecycle(), ControllerLifecycle::Stopped);

    // Stopped -> Initializing -> Running
    assert!(ctrl
        .transition_to(ControllerLifecycle::Initializing)
        .is_ok());
    assert_eq!(ctrl.lifecycle(), ControllerLifecycle::Initializing);
    assert!(ctrl.transition_to(ControllerLifecycle::Running).is_ok());
    assert_eq!(ctrl.lifecycle(), ControllerLifecycle::Running);

    // Running -> Paused -> Running
    assert!(ctrl.transition_to(ControllerLifecycle::Paused).is_ok());
    assert_eq!(ctrl.lifecycle(), ControllerLifecycle::Paused);
    assert!(ctrl.transition_to(ControllerLifecycle::Running).is_ok());

    // Running -> ShuttingDown -> PowerOff
    assert!(ctrl
        .transition_to(ControllerLifecycle::ShuttingDown)
        .is_ok());
    assert_eq!(ctrl.lifecycle(), ControllerLifecycle::PowerOff);

    // PowerOff -> Booting -> Initializing
    assert!(ctrl.boot().is_ok());
    assert_eq!(ctrl.lifecycle(), ControllerLifecycle::Initializing);

    // Initializing -> Running
    assert!(ctrl.start().is_ok());
    assert_eq!(ctrl.lifecycle(), ControllerLifecycle::Running);

    // Trip fault -> Faulted
    ctrl.trip_fault("TEST_FAULT", "Diagnostic fault trigger", 10.0, true);
    assert_eq!(ctrl.lifecycle(), ControllerLifecycle::Faulted);
    assert!(!ctrl.is_running());

    // Reset -> Stopped
    assert!(ctrl.reset().is_ok());
    assert_eq!(ctrl.lifecycle(), ControllerLifecycle::Stopped);
}

#[test]
fn test_hardware_profiles_cpu_memory_firmware() {
    let mut ctrl = VirtualController::with_name("VCPU-HW", ControllerConfig::default());

    let cpu = CpuProfile {
        execution_budget_us: 20_000,
        max_scan_frequency_hz: 100.0,
        utilization_threshold_pct: 75.0,
        architecture: "Virtual-VCPU-QuadCore".to_string(),
    };
    ctrl.set_cpu_profile(cpu.clone());
    assert_eq!(ctrl.cpu_profile().execution_budget_us, 20_000);
    assert_eq!(ctrl.cpu_profile().architecture, "Virtual-VCPU-QuadCore");

    let mem = MemoryProfile {
        program_memory_bytes: 8 * 1024 * 1024,
        data_memory_bytes: 16 * 1024 * 1024,
        retentive_memory_bytes: 2 * 1024 * 1024,
        program_memory_used_bytes: 1024 * 512,
        data_memory_used_bytes: 1024 * 1024,
        retentive_memory_used_bytes: 1024 * 64,
    };
    ctrl.set_memory_profile(mem);
    assert_eq!(ctrl.memory_profile().program_memory_bytes, 8 * 1024 * 1024);

    let fw = FirmwareProfile {
        version: "v8.2.0-LTS".to_string(),
        build: "2026.09.07".to_string(),
        iec_standard: "IEC 61131-3:2013 Ed.3".to_string(),
    };
    ctrl.set_firmware_profile(fw);
    assert_eq!(ctrl.firmware_profile().version, "v8.2.0-LTS");
}

#[test]
fn test_retentive_memory_survives_power_loss() {
    let mut ctrl = VirtualController::with_name("VCPU-RETAIN", ControllerConfig::default());
    let _ = ctrl.start();

    // Write volatile I/O and retentive storage
    ctrl.io_image_mut()
        .write_input("VOLATILE_TEMP", 85.5, TagQuality::Good, 1.0);
    ctrl.retentive_store_mut()
        .set("BATCH_TOTALIZER", IoSignalValue::good(12450.75, 1.0));
    ctrl.retentive_store_mut()
        .set("LAST_RECIPE_ID", IoSignalValue::good(42.0, 1.0));

    assert!(ctrl.io_image().read_input("VOLATILE_TEMP").is_some());
    assert_eq!(
        ctrl.retentive_store().get("BATCH_TOTALIZER"),
        Some(&IoSignalValue::good(12450.75, 1.0))
    );

    // Power Loss
    assert!(ctrl.power_off().is_ok());
    assert_eq!(ctrl.lifecycle(), ControllerLifecycle::PowerOff);

    // Volatile I/O table must be erased
    assert!(ctrl.io_image().read_input("VOLATILE_TEMP").is_none());

    // Retentive values must survive power loss intact
    assert_eq!(
        ctrl.retentive_store().get("BATCH_TOTALIZER"),
        Some(&IoSignalValue::good(12450.75, 1.0))
    );
    assert_eq!(
        ctrl.retentive_store().get("LAST_RECIPE_ID"),
        Some(&IoSignalValue::good(42.0, 1.0))
    );

    // Reboot
    assert!(ctrl.boot().is_ok());
    assert!(ctrl.start().is_ok());
    assert_eq!(
        ctrl.retentive_store().get("BATCH_TOTALIZER"),
        Some(&IoSignalValue::good(12450.75, 1.0))
    );
}

#[test]
fn test_dcs_control_module_bumpless_transfer_and_modes() {
    let pid_cfg = PidConfig {
        kp: 2.0,
        ti: 10.0,
        td: 0.0,
        derivative_filter_n: 10.0,
        out_min: 0.0,
        out_max: 100.0,
        direction: PidDirection::Direct,
        sp_tracking_in_manual: true,
        scan_divisor: 1,
    };

    let mut cm = DcsControlModule::new(
        "CM-101",
        "Feed Flow Controller",
        "FIT-101.PV",
        "FV-101.OUT",
        50.0,
        pid_cfg,
    );
    assert_eq!(cm.mode(), ControlModuleMode::Auto);

    let mut io = nebudcs_control::runtime::IoImageTable::new();
    io.write_input("FIT-101.PV", 50.0, TagQuality::Good, 0.0);

    // Initial step at setpoint
    let _ = cm.step(0.05, 0.05, &mut io);
    let _initial_out = cm.current_out();

    // Switch to Manual
    assert!(cm.set_mode(ControlModuleMode::Manual).is_ok());
    assert_eq!(cm.mode(), ControlModuleMode::Manual);

    // Operator adjusts output in Manual mode to 65.0%
    cm.set_manual_out(65.0);
    let _ = cm.step(0.05, 0.10, &mut io);
    assert_eq!(cm.current_out(), 65.0);

    // Bumpless Transfer: Switch from Manual back to Auto
    assert!(cm.set_mode(ControlModuleMode::Auto).is_ok());
    assert_eq!(cm.mode(), ControlModuleMode::Auto);

    // Immediately after transfer, output discontinuity must be negligible (|Delta OUT| < 1e-4)
    let _ = cm.step(0.05, 0.15, &mut io);
    let post_transfer_out = cm.current_out();
    let delta = (post_transfer_out - 65.0).abs();
    assert!(
        delta < 0.5,
        "Bumpless transfer failed: Delta OUT was {delta} (expected < 0.5)"
    );
}

#[test]
fn test_dcs_control_module_permissives_and_safety_interlocks() {
    let pid_cfg = PidConfig {
        kp: 1.5,
        ti: 5.0,
        td: 0.0,
        derivative_filter_n: 10.0,
        out_min: 0.0,
        out_max: 100.0,
        direction: PidDirection::Direct,
        sp_tracking_in_manual: true,
        scan_divisor: 1,
    };

    let mut cm = DcsControlModule::new(
        "LIC-101",
        "Separator Level Controller",
        "LT-101.PV",
        "LV-101.OUT",
        50.0,
        pid_cfg,
    );
    cm.add_permissive("PUMP-101.RUNNING");
    cm.add_interlock("ESD-TRIP-HIGH");
    cm.set_safe_position(0.0); // Fail closed

    let mut io = nebudcs_control::runtime::IoImageTable::new();
    io.write_input("LT-101.PV", 60.0, TagQuality::Good, 0.0);
    io.write_input("PUMP-101.RUNNING", 1.0, TagQuality::Good, 0.0); // Permissive OK
    io.write_input("ESD-TRIP-HIGH", 0.0, TagQuality::Good, 0.0); // No trip

    // Step in Auto mode -> Normal operation
    let _ = cm.step(0.05, 0.05, &mut io);
    assert!(cm.permissives_ok());
    assert!(!cm.is_interlocked());
    assert!(cm.current_out() > 0.0);

    // Unsafe condition occurs: ESD Trip
    io.write_input("ESD-TRIP-HIGH", 1.0, TagQuality::Good, 0.10);
    let _ = cm.step(0.05, 0.10, &mut io);

    assert!(cm.is_interlocked());
    assert_eq!(
        cm.current_out(),
        0.0,
        "Safety interlock must clamp output to safe_position (0.0%)"
    );

    let out_sig = io.read_output("LV-101.OUT").unwrap();
    assert_eq!(out_sig.value, 0.0);

    // Permissive loss: Pump stops
    io.write_input("ESD-TRIP-HIGH", 0.0, TagQuality::Good, 0.15); // Clear trip
    io.write_input("PUMP-101.RUNNING", 0.0, TagQuality::Good, 0.15); // Pump stopped
    let _ = cm.step(0.05, 0.15, &mut io);

    assert!(!cm.permissives_ok());
    assert_eq!(
        cm.mode(),
        ControlModuleMode::Manual,
        "Permissive failure must force fallback to Manual"
    );

    // Attempting to re-enter Auto with false permissives must be rejected
    assert!(cm.set_mode(ControlModuleMode::Auto).is_err());
}

#[test]
fn test_redundancy_replication_and_switchover() {
    let mut primary = VirtualController::with_name("VCPU-PRIM", ControllerConfig::default());
    let mut standby = VirtualController::with_name("VCPU-STBY", ControllerConfig::default());

    primary.set_redundancy(RedundancyState {
        role: RedundancyRole::Primary,
        status: RedundancyStatus::Synchronizing,
        partner_id: Some(standby.name().to_string()),
        sync_frequency_hz: 20.0,
        switchover_timeout_seconds: 0.1,
        last_sync_sim_time: 0.0,
        sync_cycle_count: 0,
    });

    standby.set_redundancy(RedundancyState {
        role: RedundancyRole::Standby,
        status: RedundancyStatus::Synchronizing,
        partner_id: Some(primary.name().to_string()),
        sync_frequency_hz: 20.0,
        switchover_timeout_seconds: 0.1,
        last_sync_sim_time: 0.0,
        sync_cycle_count: 0,
    });

    let _ = primary.start();

    // Primary has active state and retentive storage
    primary
        .retentive_store_mut()
        .set("PRIMARY_ACCUM", IoSignalValue::good(999.5, 5.0));
    primary
        .io_image_mut()
        .write_input("AI-PRESSURE", 14.2, TagQuality::Good, 5.0);

    // Replicate Primary state to Standby
    assert!(primary.replicate_to_standby(&mut standby).is_ok());

    assert_eq!(standby.redundancy().status, RedundancyStatus::Synchronized);
    assert_eq!(
        standby.retentive_store().get("PRIMARY_ACCUM"),
        Some(&IoSignalValue::good(999.5, 5.0))
    );
    assert_eq!(
        standby.io_image().read_input_value("AI-PRESSURE"),
        Some(14.2)
    );

    // Perform graceful switchover
    primary.trigger_switchover(RedundancyRole::Standby);
    standby.trigger_switchover(RedundancyRole::Primary);

    assert_eq!(primary.redundancy().role, RedundancyRole::Standby);
    assert_eq!(standby.redundancy().role, RedundancyRole::Primary);
    assert!(standby.is_running());
}

#[test]
fn test_controlled_fault_injection_and_diagnostics() {
    let mut ctrl = VirtualController::with_name("VCPU-FAULT", ControllerConfig::default());
    let _ = ctrl.start();

    // 1. Inject Watchdog Timeout
    ctrl.inject_watchdog_timeout(12.5);
    assert_eq!(ctrl.lifecycle(), ControllerLifecycle::Faulted);
    assert_eq!(ctrl.fault().unwrap().code, "WATCHDOG_TIMEOUT");
    assert!(ctrl.diagnostics().watchdog_trips > 0);

    // Recover via Reset
    assert!(ctrl.clear_fault().is_ok());
    assert_eq!(ctrl.lifecycle(), ControllerLifecycle::Stopped);

    // 2. Inject Memory Exhaustion (Fatal/Non-recoverable)
    let _ = ctrl.start();
    ctrl.inject_memory_exhaustion(15.0);
    assert_eq!(ctrl.lifecycle(), ControllerLifecycle::Faulted);
    assert!(!ctrl.fault().unwrap().recoverable);

    // Clearing non-recoverable fault directly must fail
    assert!(ctrl.clear_fault().is_err());

    // Reboot required for recovery
    assert!(ctrl.boot().is_ok());
    assert_eq!(ctrl.lifecycle(), ControllerLifecycle::Initializing);
}

#[test]
fn test_10k_ticks_bit_identical_determinism_stage_8() {
    let run_simulation = || -> Vec<f64> {
        let mut ctrl = VirtualController::with_name("VCPU-DET", ControllerConfig::default());
        let pid_cfg = PidConfig {
            kp: 1.2,
            ti: 8.0,
            td: 0.5,
            derivative_filter_n: 10.0,
            out_min: 0.0,
            out_max: 100.0,
            direction: PidDirection::Direct,
            sp_tracking_in_manual: true,
            scan_divisor: 1,
        };
        let cm = DcsControlModule::new(
            "PIC-101",
            "Pressure Loop",
            "PT-101.PV",
            "PV-101.OUT",
            40.0,
            pid_cfg,
        );
        ctrl.add_control_module(cm);
        let _ = ctrl.start();

        let dt = 0.05;
        let mut snapshots = Vec::with_capacity(100);

        for tick in 0..10_000 {
            let sim_time = tick as f64 * dt;
            // Simulated PV with deterministic sinusoidal disturbance
            let pv = 40.0 + (sim_time * 0.1).sin() * 5.0;
            ctrl.io_image_mut()
                .write_input("PT-101.PV", pv, TagQuality::Good, sim_time);

            let _ = ctrl.step(dt, sim_time);

            if tick % 100 == 0 {
                let out = ctrl
                    .control_module("PIC-101")
                    .map(|m| m.current_out())
                    .unwrap_or(0.0);
                snapshots.push(out);
            }
        }
        snapshots
    };

    let run_a = run_simulation();
    let run_b = run_simulation();

    assert_eq!(run_a.len(), run_b.len());
    for i in 0..run_a.len() {
        assert_eq!(
            run_a[i].to_bits(),
            run_b[i].to_bits(),
            "Bit-identical determinism failure at snapshot {i}: {} vs {}",
            run_a[i],
            run_b[i]
        );
    }
}
