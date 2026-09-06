use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use nebudcs_control::actuator::{
    ActuatorArbiter, ActuatorCommand, ActuatorConfig, ActuatorModel, CommandPriority, FailAction,
};
use nebudcs_control::instrument::{InstrumentConfig, InstrumentModel};
use nebudcs_control::pid::{PidConfig, PidController, PidDirection, PidMode};
use nebudcs_control::protection::{
    DegradationPolicy, ProtectionBlock, ProtectionConfig, SensorInput, VotingVoter2oo3,
};
use nebudcs_model::tag::TagQuality;

struct TrackingAllocator;
static TRACKING: AtomicBool = AtomicBool::new(false);
static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if TRACKING.load(Ordering::Relaxed) {
            ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static A: TrackingAllocator = TrackingAllocator;

#[test]
fn test_hot_path_zero_heap_allocations() {
    // 1. Initialize components outside the tracked region
    let pid_cfg = PidConfig {
        kp: 2.0,
        ti: 10.0,
        td: 1.0,
        derivative_filter_n: 10.0,
        out_min: 0.0,
        out_max: 100.0,
        direction: PidDirection::Reverse,
        sp_tracking_in_manual: false,
        scan_divisor: 1,
    };
    let mut pid = PidController::new(pid_cfg);
    pid.initialize(50.0);
    pid.set_mode(PidMode::Auto);
    pid.set_sp(55.0);

    let inst_cfg = InstrumentConfig::builder()
        .lrv(0.0)
        .urv(100.0)
        .sensor_lag_tau(1.0)
        .damping_tau(0.5)
        .noise_std_dev(0.0)
        .build()
        .unwrap();
    let mut inst = InstrumentModel::new(inst_cfg, 50.0);

    let act_cfg = ActuatorConfig {
        stroke_time: 3.0,
        deadband: 0.2,
        fail_action: FailAction::FailClosed,
    };
    let mut act = ActuatorModel::new(act_cfg, 50.0);
    let mut arbiter = ActuatorArbiter::new(FailAction::FailClosed);
    arbiter.submit(ActuatorCommand::new(CommandPriority::Auto, 50.0));
    arbiter.submit(ActuatorCommand::new(CommandPriority::Manual, 45.0));

    let prot_cfg = ProtectionConfig {
        name: "TEST_PROT".to_string(),
        threshold: 90.0,
        reset_deadband: 5.0,
    };
    let mut prot = ProtectionBlock::new(prot_cfg);
    let voter = VotingVoter2oo3::new(90.0, DegradationPolicy::DegradeTo1oo2);

    let inputs = [
        SensorInput::new(50.0, TagQuality::Good),
        SensorInput::new(51.0, TagQuality::Good),
        SensorInput::new(49.0, TagQuality::Good),
    ];

    // Warm-up step
    let _ = pid.step(0.1, 50.0);
    let _ = inst.step(0.1, 50.0, 0.1);
    let _ = arbiter.arbitrate();
    let _ = act.step(0.1, 50.0, true);
    let _ = prot.step(50.0, false);
    let _ = voter.evaluate(&inputs);

    // 2. Start tracking allocations
    ALLOC_COUNT.store(0, Ordering::SeqCst);
    TRACKING.store(true, Ordering::SeqCst);

    // 3. Execute 1,000 steps of all core automation algorithms in the hot loop
    for tick in 0..1000 {
        let pv = inst.step(0.1, 52.0 + (tick as f64 * 0.001), 0.1);
        let out = pid.step(0.1, pv);
        arbiter.submit(ActuatorCommand::new(CommandPriority::Auto, out));
        let winning = arbiter.arbitrate().unwrap();
        let _pos = act.step(0.1, winning.target_position, true);
        let _tripped = prot.step(pv, false);
        let _res = voter.evaluate(&inputs);
    }

    // 4. Stop tracking and verify
    TRACKING.store(false, Ordering::SeqCst);
    let total_allocations = ALLOC_COUNT.load(Ordering::SeqCst);

    assert_eq!(
        total_allocations, 0,
        "Hot path allocation detected! Expected 0 allocations, got {total_allocations}"
    );
}
