use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use nebudcs_alarm::block::AlarmBlockConfig;
use nebudcs_alarm::engine::AlarmEngine;
use nebudcs_alarm::types::{AlarmSeverity, AlarmType};
use nebudcs_model::tag::TagQuality;
use nebudcs_types::TagId;

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
fn test_alarm_engine_hot_path_zero_heap_allocations() {
    let mut engine = AlarmEngine::new();
    let tag = TagId::new();

    let hi_cfg = AlarmBlockConfig {
        name: "TI101_HI".to_string(),
        tag_id: tag,
        alarm_type: AlarmType::Hi,
        severity: AlarmSeverity::Warning,
        limit: 80.0,
        deadband: 2.0,
        default_shelve_duration: 60.0,
    };
    let hihi_cfg = AlarmBlockConfig {
        name: "TI101_HIHI".to_string(),
        tag_id: tag,
        alarm_type: AlarmType::HiHi,
        severity: AlarmSeverity::Critical,
        limit: 95.0,
        deadband: 2.0,
        default_shelve_duration: 60.0,
    };

    engine.add_alarm(hi_cfg);
    engine.add_alarm(hihi_cfg);

    // Warm-up step
    engine.evaluate_tag(tag, 70.0, TagQuality::Good, 0.1);

    // Start tracking allocations
    ALLOC_COUNT.store(0, Ordering::SeqCst);
    TRACKING.store(true, Ordering::SeqCst);

    // Execute 1,000 steps of alarm evaluations
    for tick in 0..1000 {
        let pv = 70.0 + (tick as f64 * 0.03); // crosses 80.0 and 95.0
        engine.evaluate_tag(tag, pv, TagQuality::Good, 0.1);
    }

    // Stop tracking
    TRACKING.store(false, Ordering::SeqCst);
    let total_allocations = ALLOC_COUNT.load(Ordering::SeqCst);

    assert_eq!(
        total_allocations, 0,
        "Alarm hot path allocation detected! Expected 0 allocations, got {total_allocations}"
    );
}
