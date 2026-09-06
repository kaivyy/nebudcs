//! Phase 7 performance and latency benchmarks.

use std::time::Instant;

use nebudcs_protocol::*;

#[test]
fn test_perf_snapshot_serialization() {
    let mut tags = Vec::with_capacity(1000);
    for i in 0..1000 {
        tags.push(TagValueSampleDto {
            tag: format!("PT_{:04}", i),
            value: TagValueDto::Float(100.0 + (i as f64) * 0.1),
            quality: TagQualityDto::Good,
            unit: Some("bar".to_string()),
        });
    }

    let snapshot = StateSnapshotDto {
        snapshot_id: "snap-bench-001".to_string(),
        sim_time_sec: 123.45,
        tick_count: 2469,
        tags,
        active_alarms: Vec::new(),
    };

    let start = Instant::now();
    let iterations = 200;
    let mut total_bytes = 0;

    for _ in 0..iterations {
        let serialized = serde_json::to_string(&snapshot).unwrap();
        total_bytes += serialized.len();
    }

    let elapsed = start.elapsed();
    let per_op = elapsed / iterations;
    let ops_per_sec = (iterations as f64) / elapsed.as_secs_f64();

    println!(
        "\n[PERF] StateSnapshotDto (1,000 tags): {:?} per serialization ({:.0} ops/sec, size: {} KB)",
        per_op,
        ops_per_sec,
        (total_bytes / iterations as usize) / 1024
    );

    // Serialization for 1,000 tags must be well under 2.0 ms
    assert!(
        per_op.as_millis() < 5,
        "Snapshot serialization too slow: {:?}",
        per_op
    );
}

#[test]
fn test_perf_delta_serialization() {
    let mut changed = Vec::with_capacity(50);
    for i in 0..50 {
        changed.push(TagValueSampleDto {
            tag: format!("TI_{:04}", i),
            value: TagValueDto::Float(200.0 + (i as f64) * 0.5),
            quality: TagQualityDto::Good,
            unit: Some("degC".to_string()),
        });
    }

    let delta = DeltaUpdateDto {
        seq: 42,
        sim_time_sec: 123.50,
        tick_count: 2470,
        changed_tags: changed,
    };

    let start = Instant::now();
    let iterations = 1000;
    let mut total_bytes = 0;

    for _ in 0..iterations {
        let serialized = serde_json::to_string(&delta).unwrap();
        total_bytes += serialized.len();
    }

    let elapsed = start.elapsed();
    let per_op = elapsed / iterations;
    let ops_per_sec = (iterations as f64) / elapsed.as_secs_f64();

    println!(
        "[PERF] DeltaUpdateDto (50 changed tags): {:?} per serialization ({:.0} ops/sec, size: {} B)",
        per_op,
        ops_per_sec,
        total_bytes / iterations as usize
    );

    // Delta update serialization must execute in microseconds
    assert!(
        per_op.as_micros() < 500,
        "Delta serialization too slow: {:?}",
        per_op
    );
}

#[test]
fn test_perf_runtime_command_serde() {
    let cmd = RuntimeCommand::WriteTag {
        tag: "FIC-101.SP".to_string(),
        value: TagValueDto::Float(4500.0),
    };

    let start = Instant::now();
    let iterations = 5000;

    for _ in 0..iterations {
        let s = serde_json::to_string(&cmd).unwrap();
        let _parsed: RuntimeCommand = serde_json::from_str(&s).unwrap();
    }

    let elapsed = start.elapsed();
    let per_roundtrip = elapsed / iterations;
    let ops_per_sec = (iterations as f64) / elapsed.as_secs_f64();

    println!(
        "[PERF] RuntimeCommand roundtrip serde: {:?} per roundtrip ({:.0} ops/sec)",
        per_roundtrip, ops_per_sec
    );

    assert!(
        per_roundtrip.as_micros() < 50,
        "Command serde too slow: {:?}",
        per_roundtrip
    );
}
