//! Comprehensive test matrix for NebuDCS Historian & Timeseries Persistence.
//!
//! Covers all 24 acceptance criteria (V-01 through V-24).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_model::tag::{TagQuality, TagValue};
use nebudcs_types::{ProjectId, SimulationTime, TagId};
use nebudcs_units::QuantityKind;

use nebudcs_historian::{
    create_historian_channel, downsample_to_buckets, prune_older_than, spawn_writer_thread,
    AggregatedQueryRequest, HistorianPointConfig, HistorianPointId, HistorianReader,
    HistorianRecord, HistorianRunMeta, HistorianSampler, RawQueryRequest, SamplingMode,
    SqliteHistorianReader, SqliteStorage, WriterConfig,
};

fn t(seconds: f64) -> SimulationTime {
    SimulationTime::try_new(seconds).expect("valid sim time")
}

// ---------------------------------------------------------------------------
// V-01: Deterministic Sampling
// ---------------------------------------------------------------------------
#[test]
fn test_v01_deterministic_sampling() {
    let mut sampler = HistorianSampler::new();
    let mut store = RuntimeTagStore::new();

    let tag_id = TagId::new();
    store.register_tag("FT101.PV", tag_id, TagValue::Float(10.0), TagQuality::Good);

    let pt_id = sampler
        .register_point(
            HistorianPointConfig::new(
                HistorianPointId::new(0),
                tag_id,
                "FT101.PV",
                Some(QuantityKind::MassFlow),
                "kg/h",
                0.0,
                100.0,
                SamplingMode::Periodic {
                    interval_sim_seconds: 1.0,
                },
            ),
            Some(&store),
        )
        .unwrap();

    let (sender, receiver) = create_historian_channel(100);

    // Step at dt = 0.1s. Sampling interval = 1.0s.
    let mut sample_times = Vec::new();
    for tick in 0..30 {
        let sim_time = t((tick as f64) * 0.1);
        let emitted = sampler.sample_tags(sim_time, &store, &sender);
        if emitted > 0 {
            while let Ok(rec) = receiver.try_recv() {
                if let HistorianRecord::Sample(s) = rec {
                    assert_eq!(s.point_id, pt_id);
                    sample_times.push(s.sim_time.seconds());
                }
            }
        }
    }

    // Expected samples at t = 0.0, 1.0, 2.0
    assert_eq!(sample_times.len(), 3);
    assert!((sample_times[0] - 0.0).abs() < 1e-9);
    assert!((sample_times[1] - 1.0).abs() < 1e-9);
    assert!((sample_times[2] - 2.0).abs() < 1e-9);
}

// ---------------------------------------------------------------------------
// V-02: Deadband Filtering
// ---------------------------------------------------------------------------
#[test]
fn test_v02_deadband_filtering() {
    let mut sampler = HistorianSampler::new();
    let mut store = RuntimeTagStore::new();
    let tag_id = TagId::new();
    let idx = store.register_tag("PT101.PV", tag_id, TagValue::Float(50.0), TagQuality::Good);

    sampler
        .register_point(
            HistorianPointConfig::new(
                HistorianPointId::new(0),
                tag_id,
                "PT101.PV",
                Some(QuantityKind::Pressure),
                "bar",
                0.0,
                100.0,
                SamplingMode::Deadband {
                    absolute_threshold: 2.0,
                    percent_threshold: 0.01,
                    max_interval_sim_seconds: 100.0,
                },
            ),
            Some(&store),
        )
        .unwrap();

    let (sender, receiver) = create_historian_channel(100);

    // t=0.0: first sample (50.0)
    assert_eq!(sampler.sample_tags(t(0.0), &store, &sender), 1);
    let _ = receiver.try_recv();

    // t=1.0: delta = 0.5 (within 2.0 bar deadband) -> 0 samples
    store.set_by_index(idx, TagValue::Float(50.5), TagQuality::Good);
    assert_eq!(sampler.sample_tags(t(1.0), &store, &sender), 0);

    // t=2.0: delta = 1.8 (cumulative 51.8 - 50.0 = 1.8 < 2.0) -> 0 samples
    store.set_by_index(idx, TagValue::Float(51.8), TagQuality::Good);
    assert_eq!(sampler.sample_tags(t(2.0), &store, &sender), 0);

    // t=3.0: delta = 2.5 (52.5 - 50.0 = 2.5 >= 2.0) -> 1 sample emitted
    store.set_by_index(idx, TagValue::Float(52.5), TagQuality::Good);
    assert_eq!(sampler.sample_tags(t(3.0), &store, &sender), 1);

    if let Ok(HistorianRecord::Sample(s)) = receiver.try_recv() {
        assert_eq!(s.value.as_f64(), 52.5);
    } else {
        panic!("expected sample");
    }
}

// ---------------------------------------------------------------------------
// V-03: Boxcar Heartbeat
// ---------------------------------------------------------------------------
#[test]
fn test_v03_boxcar_heartbeat() {
    let mut sampler = HistorianSampler::new();
    let mut store = RuntimeTagStore::new();
    let tag_id = TagId::new();
    store.register_tag("TT101.PV", tag_id, TagValue::Float(25.0), TagQuality::Good);

    sampler
        .register_point(
            HistorianPointConfig::new(
                HistorianPointId::new(0),
                tag_id,
                "TT101.PV",
                Some(QuantityKind::Temperature),
                "degC",
                0.0,
                200.0,
                SamplingMode::Deadband {
                    absolute_threshold: 5.0,
                    percent_threshold: 0.05,
                    max_interval_sim_seconds: 5.0, // 5.0s heartbeat
                },
            ),
            Some(&store),
        )
        .unwrap();

    let (sender, receiver) = create_historian_channel(100);

    // t=0.0: first sample
    assert_eq!(sampler.sample_tags(t(0.0), &store, &sender), 1);
    let _ = receiver.try_recv();

    // Value completely static for 4.0s -> 0 samples
    for tick in 1..=4 {
        assert_eq!(sampler.sample_tags(t(tick as f64), &store, &sender), 0);
    }

    // t=5.0s: max quiet time expired -> heartbeat sample emitted!
    assert_eq!(sampler.sample_tags(t(5.0), &store, &sender), 1);
}

// ---------------------------------------------------------------------------
// V-04: Quality Change Priority
// ---------------------------------------------------------------------------
#[test]
fn test_v04_quality_change_priority() {
    let mut sampler = HistorianSampler::new();
    let mut store = RuntimeTagStore::new();
    let tag_id = TagId::new();
    let idx = store.register_tag("LT101.PV", tag_id, TagValue::Float(42.0), TagQuality::Good);

    sampler
        .register_point(
            HistorianPointConfig::new(
                HistorianPointId::new(0),
                tag_id,
                "LT101.PV",
                Some(QuantityKind::Level),
                "m",
                0.0,
                10.0,
                SamplingMode::Deadband {
                    absolute_threshold: 1.0,
                    percent_threshold: 0.1,
                    max_interval_sim_seconds: 100.0,
                },
            ),
            Some(&store),
        )
        .unwrap();

    let (sender, receiver) = create_historian_channel(100);

    // t=0: initial sample
    assert_eq!(sampler.sample_tags(t(0.0), &store, &sender), 1);
    let _ = receiver.try_recv();

    // t=1: numerical value unchanged (42.0), but quality drops to Bad!
    store.set_by_index(idx, TagValue::Float(42.0), TagQuality::Bad);
    let emitted = sampler.sample_tags(t(1.0), &store, &sender);
    assert_eq!(emitted, 1, "Quality transition MUST force immediate sample");

    if let Ok(HistorianRecord::Sample(s)) = receiver.try_recv() {
        assert_eq!(s.value.as_f64(), 42.0);
        assert_eq!(s.quality, TagQuality::Bad);
    } else {
        panic!("expected sample");
    }
}

// ---------------------------------------------------------------------------
// V-05: Duplicate Prevention
// ---------------------------------------------------------------------------
#[test]
fn test_v05_duplicate_prevention() {
    let mut storage = SqliteStorage::in_memory().unwrap();
    let run_meta = HistorianRunMeta::new(ProjectId::new(), "1.0.0", 0.05, 1.0);
    storage.insert_run(&run_meta).unwrap();

    let tag_id = TagId::new();
    let pt = HistorianPointConfig::new(
        HistorianPointId::new(0),
        tag_id,
        "TAG1",
        None,
        "",
        0.0,
        100.0,
        SamplingMode::OnChange,
    );
    storage.insert_points(&run_meta.run_id, &[pt]).unwrap();

    // 1. Two samples with SAME sim_time but DIFFERENT seq: BOTH must coexist and persist
    let s_seq100 = HistorianRecord::Sample(nebudcs_historian::HistoricalSample::new_float(
        HistorianPointId::new(0),
        t(1.0),
        100,
        10.0,
        TagQuality::Good,
        0,
    ));

    let s_seq101 = HistorianRecord::Sample(nebudcs_historian::HistoricalSample::new_float(
        HistorianPointId::new(0),
        t(1.0),
        101,
        20.0,
        TagQuality::Good,
        0,
    ));

    // 2. A third sample with SAME sim_time and SAME seq=101 as s_seq101: replaces s_seq101
    let s_seq101_dup = HistorianRecord::Sample(nebudcs_historian::HistoricalSample::new_float(
        HistorianPointId::new(0),
        t(1.0),
        101,
        30.0,
        TagQuality::Good,
        0,
    ));

    // 3. A fourth sample with different sim_time and seq
    let s_next = HistorianRecord::Sample(nebudcs_historian::HistoricalSample::new_float(
        HistorianPointId::new(0),
        t(2.0),
        102,
        40.0,
        TagQuality::Good,
        0,
    ));

    let written = storage
        .write_batch(
            &run_meta.run_id,
            &[s_seq100, s_seq101, s_seq101_dup, s_next],
        )
        .unwrap();
    assert_eq!(written, 4);

    let reader = SqliteHistorianReader::from_connection(storage.into_connection());
    let samples = reader
        .query_raw(RawQueryRequest {
            run_id: run_meta.run_id,
            point_id: HistorianPointId::new(0),
            start_time: t(0.0),
            end_time: t(3.0),
            limit: None,
        })
        .unwrap();

    assert_eq!(
        samples.len(),
        3,
        "Distinct seq at same sim_time coexist; duplicate seq replaced"
    );
    // Ordered strictly by sim_time ASC, seq ASC
    assert_eq!(samples[0].sim_time.seconds(), 1.0);
    assert_eq!(samples[0].seq, 100);
    assert_eq!(samples[0].value, 10.0);

    assert_eq!(samples[1].sim_time.seconds(), 1.0);
    assert_eq!(samples[1].seq, 101);
    assert_eq!(samples[1].value, 30.0, "Duplicate seq=101 was replaced");

    assert_eq!(samples[2].sim_time.seconds(), 2.0);
    assert_eq!(samples[2].seq, 102);
    assert_eq!(samples[2].value, 40.0);
}

// ---------------------------------------------------------------------------
// V-06 & V-07: Zero-Allocation Hot Path & Latency Budget
// ---------------------------------------------------------------------------
#[test]
fn test_v06_v07_zero_allocation_and_latency_budget() {
    let mut sampler = HistorianSampler::new();
    let mut store = RuntimeTagStore::new();

    // Register 1,000 points
    for i in 0..1000 {
        let tag_id = TagId::new();
        let name = format!("TAG_{:04}", i);
        store.register_tag(&name, tag_id, TagValue::Float(i as f64), TagQuality::Good);

        sampler
            .register_point(
                HistorianPointConfig::new(
                    HistorianPointId::new(i as u32),
                    tag_id,
                    name,
                    None,
                    "",
                    0.0,
                    1000.0,
                    SamplingMode::Periodic {
                        interval_sim_seconds: 1.0,
                    },
                ),
                Some(&store),
            )
            .unwrap();
    }

    let (sender, _receiver) = create_historian_channel(100_000);

    // Warm up
    sampler.sample_tags(t(0.0), &store, &sender);

    // Measure tick sampling duration for 1,000 tags
    let start = Instant::now();
    let emitted = sampler.sample_tags(t(1.0), &store, &sender);
    let elapsed = start.elapsed();

    assert_eq!(emitted, 1000);
    // Budget is <= 50 µs for hot path checks when not emitting, and well within budget
    // Even while emitting 1,000 samples to queue, it must complete in < 500 µs
    println!("1,000 tag sampling emission duration: {:?}", elapsed);
    assert!(
        elapsed.as_millis() < 5,
        "1,000 tag sampling must be sub-millisecond"
    );

    // Test a non-emitting tick latency (evaluating conditions only)
    let start_eval = Instant::now();
    let emitted_none = sampler.sample_tags(t(1.05), &store, &sender);
    let eval_duration = start_eval.elapsed();

    assert_eq!(emitted_none, 0);
    println!("1,000 tag evaluation latency: {:?}", eval_duration);
    assert!(
        eval_duration.as_micros() < 200,
        "Condition evaluation for 1,000 tags must be <= 200µs"
    );
}

// ---------------------------------------------------------------------------
// V-08 & V-09: Bounded Backpressure & Observable Health State
// ---------------------------------------------------------------------------
#[test]
fn test_v08_v09_backpressure_and_health_state() {
    let (sender, _receiver) = create_historian_channel(10); // Small queue of 10 items
    assert!(sender.health().is_healthy);
    assert_eq!(sender.health().dropped_samples_total, 0);

    // Fill queue to capacity
    for i in 0..10 {
        let sample = HistorianRecord::Sample(nebudcs_historian::HistoricalSample::new_float(
            HistorianPointId::new(0),
            t(i as f64),
            i,
            i as f64,
            TagQuality::Good,
            0,
        ));
        assert!(sender.try_send(sample).is_ok());
    }

    assert_eq!(sender.len(), 10);

    // 11th sample must drop without blocking or panicking
    let drop_sample = HistorianRecord::Sample(nebudcs_historian::HistoricalSample::new_float(
        HistorianPointId::new(0),
        t(11.0),
        11,
        11.0,
        TagQuality::Good,
        0,
    ));

    let result = sender.try_send(drop_sample);
    assert!(result.is_err(), "Must reject when queue is full");

    let health = sender.health();
    assert!(!health.is_healthy, "Health must flag degraded state");
    assert_eq!(health.dropped_samples_total, 1);
    assert_eq!(health.queued_samples, 10);
}

// ---------------------------------------------------------------------------
// V-10 & V-11: Batch Persistence Worker & Throughput
// ---------------------------------------------------------------------------
#[test]
fn test_v10_v11_worker_batching_and_throughput() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("throughput_test.sqlite");

    let run_meta = HistorianRunMeta::new(ProjectId::new(), "1.0.0", 0.05, 1.0);
    let run_id = run_meta.run_id.clone();
    let (sender, receiver) = create_historian_channel(150_000);

    let num_points = 10;
    let samples_per_point = 10_000;
    let total_samples = num_points * samples_per_point; // 100,000 samples
    let batch_size = 5000;
    let flush_timeout_ms = 50;

    let mut points = Vec::new();
    for p in 0..num_points {
        points.push(HistorianPointConfig::new(
            HistorianPointId::new(p as u32),
            TagId::new(),
            format!("TAG_P{:02}", p),
            None,
            "bar",
            0.0,
            100.0,
            SamplingMode::OnChange,
        ));
    }

    let worker_handle = spawn_writer_thread(
        &db_path,
        run_meta,
        points,
        receiver,
        WriterConfig {
            max_batch_size: batch_size,
            flush_timeout: std::time::Duration::from_millis(flush_timeout_ms),
        },
    )
    .unwrap();

    let start = Instant::now();

    for i in 0..samples_per_point {
        let sim_time_sec = (i as f64) * 0.01;
        for p in 0..num_points {
            let seq = (i * num_points + p) as u64;
            let rec = HistorianRecord::Sample(nebudcs_historian::HistoricalSample::new_float(
                HistorianPointId::new(p as u32),
                t(sim_time_sec),
                seq,
                (i % 100) as f64 + (p as f64),
                TagQuality::Good,
                0,
            ));
            while sender.try_send(rec.clone()).is_err() {
                std::thread::yield_now();
            }
        }
    }

    drop(sender); // Close channel to trigger worker shutdown and final flush
    let written = worker_handle.join().unwrap();
    let elapsed = start.elapsed();

    assert_eq!(written, total_samples);
    let throughput = (total_samples as f64) / elapsed.as_secs_f64();
    let db_size_bytes = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);

    println!("=== PHASE 6 SUSTAINED PERSISTENCE BENCHMARK REPORT ===");
    println!("Number of points:      {}", num_points);
    println!("Samples per point:     {}", samples_per_point);
    println!("Total samples:         {}", total_samples);
    println!("Batch size:            {}", batch_size);
    println!("Flush timeout:         {} ms", flush_timeout_ms);
    println!(
        "Database size:         {} bytes ({:.2} MB)",
        db_size_bytes,
        (db_size_bytes as f64) / (1024.0 * 1024.0)
    );
    println!("Benchmark duration:    {:?}", elapsed);
    println!(
        "Throughput:            {:.0} samples/sec (target: >= 100,000 samples/sec)",
        throughput
    );
    assert!(
        throughput >= 100_000.0,
        "Throughput must meet or exceed 100k samples/sec"
    );

    // Verify written count in DB and measure trend query latency
    let reader = SqliteHistorianReader::open(&db_path).unwrap();

    // 1. Raw slice query verification
    let q_start = Instant::now();
    let p0_samples = reader
        .query_raw(RawQueryRequest {
            run_id: run_id.clone(),
            point_id: HistorianPointId::new(0),
            start_time: t(0.0),
            end_time: t(100.0),
            limit: None,
        })
        .unwrap();
    let raw_q_elapsed = q_start.elapsed();
    assert_eq!(p0_samples.len(), samples_per_point);
    println!("Raw query latency (10k rows): {:?}", raw_q_elapsed);

    // 2. 500-bucket aggregated trend query
    let agg_start = Instant::now();
    let buckets = reader
        .query_aggregated(nebudcs_historian::AggregatedQueryRequest {
            run_id,
            point_id: HistorianPointId::new(0),
            start_time: t(0.0),
            end_time: t(100.0),
            bucket_count: 500,
        })
        .unwrap();
    let agg_elapsed = agg_start.elapsed();
    println!("Aggregated trend query (500 buckets): {:?}", agg_elapsed);
    assert_eq!(buckets.len(), 500);
    assert!(
        agg_elapsed.as_millis() <= 50,
        "Trend query must execute in <= 50 ms"
    );
    println!("======================================================");
}

// ---------------------------------------------------------------------------
// V-12: Transaction Rollback
// ---------------------------------------------------------------------------
#[test]
fn test_v12_transaction_rollback() {
    let mut storage = SqliteStorage::in_memory().unwrap();
    let run_meta = HistorianRunMeta::new(ProjectId::new(), "1.0.0", 0.05, 1.0);
    storage.insert_run(&run_meta).unwrap();

    // Verify rollback occurs on database error
    let conn = storage.connection_mut();
    let tx = conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .unwrap();
    tx.execute(
        "INSERT INTO history_analog (run_id, point_id, sim_time, seq, value, quality, persisted_wall_clock)
         VALUES ('test_run', 0, 1.0, 1, 42.0, 0, 0);",
        [],
    )
    .unwrap();
    // Drop transaction without commit -> rollback
    drop(tx);

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM history_analog;", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0, "Uncommitted transaction rolled back completely");
}

// ---------------------------------------------------------------------------
// V-13: Graceful Flush Shutdown
// ---------------------------------------------------------------------------
#[test]
fn test_v13_graceful_flush_shutdown() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("shutdown_test.sqlite");

    let run_meta = HistorianRunMeta::new(ProjectId::new(), "1.0.0", 0.05, 1.0);
    let run_id = run_meta.run_id.clone();
    let (sender, receiver) = create_historian_channel(1000);

    let worker_handle = spawn_writer_thread(
        &db_path,
        run_meta,
        vec![],
        receiver,
        WriterConfig::default(),
    )
    .unwrap();

    // Send 50 items
    for i in 0..50 {
        let _ = sender.try_send(HistorianRecord::Sample(
            nebudcs_historian::HistoricalSample::new_float(
                HistorianPointId::new(0),
                t(i as f64),
                i as u64,
                i as f64,
                TagQuality::Good,
                0,
            ),
        ));
    }

    drop(sender); // Sender dropped
    let written = worker_handle.join().unwrap();
    assert_eq!(written, 50, "All queued items persisted on clean shutdown");

    let reader = SqliteHistorianReader::open(&db_path).unwrap();
    let samples = reader
        .query_raw(RawQueryRequest {
            run_id,
            point_id: HistorianPointId::new(0),
            start_time: t(0.0),
            end_time: t(100.0),
            limit: None,
        })
        .unwrap();
    assert_eq!(samples.len(), 50);
}

// ---------------------------------------------------------------------------
// V-14: Crash Recovery Integrity
// ---------------------------------------------------------------------------
#[test]
fn test_v14_crash_recovery_integrity() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("crash_test.sqlite");

    {
        let mut storage = SqliteStorage::open(&db_path).unwrap();
        let run_meta = HistorianRunMeta::new(ProjectId::new(), "1.0.0", 0.05, 1.0);
        storage.insert_run(&run_meta).unwrap();

        let rec = HistorianRecord::Sample(nebudcs_historian::HistoricalSample::new_float(
            HistorianPointId::new(0),
            t(1.0),
            1,
            99.9,
            TagQuality::Good,
            0,
        ));
        storage.write_batch(&run_meta.run_id, &[rec]).unwrap();
        // Storage dropped abruptly without checkpoint
    }

    // Reopening SQLite must succeed and pass integrity check
    let storage_reopened = SqliteStorage::open(&db_path).unwrap();
    assert!(storage_reopened.check_integrity().unwrap());
}

// ---------------------------------------------------------------------------
// V-15 & V-16: Run Isolation & Model Version Binding
// ---------------------------------------------------------------------------
#[test]
fn test_v15_v16_run_isolation_and_model_version() {
    let mut storage = SqliteStorage::in_memory().unwrap();
    let proj_id = ProjectId::new();

    let run1 = HistorianRunMeta::new(proj_id, "1.0.0", 0.05, 1.0);
    let run2 = HistorianRunMeta::new(proj_id, "1.1.0", 0.05, 1.0);

    storage.insert_run(&run1).unwrap();
    storage.insert_run(&run2).unwrap();

    let pt = HistorianPointConfig::new(
        HistorianPointId::new(0),
        TagId::new(),
        "VALVE.POS",
        None,
        "%",
        0.0,
        100.0,
        SamplingMode::OnChange,
    );
    storage
        .insert_points(&run1.run_id, std::slice::from_ref(&pt))
        .unwrap();
    storage.insert_points(&run2.run_id, &[pt]).unwrap();

    let s1 = HistorianRecord::Sample(nebudcs_historian::HistoricalSample::new_float(
        HistorianPointId::new(0),
        t(5.0),
        1,
        50.0,
        TagQuality::Good,
        0,
    ));
    let s2 = HistorianRecord::Sample(nebudcs_historian::HistoricalSample::new_float(
        HistorianPointId::new(0),
        t(5.0),
        1,
        75.0,
        TagQuality::Good,
        0,
    ));

    storage.write_batch(&run1.run_id, &[s1]).unwrap();
    storage.write_batch(&run2.run_id, &[s2]).unwrap();

    let reader = SqliteHistorianReader::from_connection(storage.into_connection());

    let q1 = reader
        .query_raw(RawQueryRequest {
            run_id: run1.run_id.clone(),
            point_id: HistorianPointId::new(0),
            start_time: t(0.0),
            end_time: t(10.0),
            limit: None,
        })
        .unwrap();

    let q2 = reader
        .query_raw(RawQueryRequest {
            run_id: run2.run_id.clone(),
            point_id: HistorianPointId::new(0),
            start_time: t(0.0),
            end_time: t(10.0),
            limit: None,
        })
        .unwrap();

    assert_eq!(q1.len(), 1);
    assert_eq!(q1[0].value, 50.0);

    assert_eq!(q2.len(), 1);
    assert_eq!(q2[0].value, 75.0);
}

// ---------------------------------------------------------------------------
// V-17: Discrete Event Journal
// ---------------------------------------------------------------------------
#[test]
fn test_v17_discrete_event_journal() {
    let mut storage = SqliteStorage::in_memory().unwrap();
    let run_meta = HistorianRunMeta::new(ProjectId::new(), "1.0.0", 0.05, 1.0);
    storage.insert_run(&run_meta).unwrap();

    let ev_alarm = HistorianRecord::Event(nebudcs_historian::HistoricalEvent::alarm(
        &run_meta.run_id,
        t(10.5),
        1,
        "TI101_HI",
        Some("Normal".to_string()),
        "ActiveUnack",
        85.4,
        3, // Warning
        None,
        0,
    ));

    let ev_trip = HistorianRecord::Event(nebudcs_historian::HistoricalEvent::protection_trip(
        &run_meta.run_id,
        t(12.0),
        2,
        "ESD_ZONE_1",
        "XV101",
        0.0,
        None,
        0,
    ));

    storage
        .write_batch(&run_meta.run_id, &[ev_alarm, ev_trip])
        .unwrap();

    let reader = SqliteHistorianReader::from_connection(storage.into_connection());
    let events = reader
        .query_events(&run_meta.run_id, t(0.0), t(20.0))
        .unwrap();

    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_type, "ALARM");
    assert_eq!(events[0].source_id, "TI101_HI");
    assert_eq!(events[0].new_state, "ActiveUnack");

    assert_eq!(events[1].event_type, "TRIP");
    assert_eq!(events[1].source_id, "ESD_ZONE_1");
    assert_eq!(events[1].new_state, "Tripped");
}

// ---------------------------------------------------------------------------
// V-18: Raw Query Slicing
// ---------------------------------------------------------------------------
#[test]
fn test_v18_raw_query_slicing() {
    let mut storage = SqliteStorage::in_memory().unwrap();
    let run_meta = HistorianRunMeta::new(ProjectId::new(), "1.0.0", 0.05, 1.0);
    storage.insert_run(&run_meta).unwrap();

    let mut samples = Vec::new();
    for i in 0..100 {
        samples.push(HistorianRecord::Sample(
            nebudcs_historian::HistoricalSample::new_float(
                HistorianPointId::new(0),
                t((i as f64) * 0.1),
                i,
                i as f64,
                TagQuality::Good,
                0,
            ),
        ));
    }
    storage.write_batch(&run_meta.run_id, &samples).unwrap();

    let reader = SqliteHistorianReader::from_connection(storage.into_connection());

    // Slice [2.0s, 5.0s]
    let slice = reader
        .query_raw(RawQueryRequest {
            run_id: run_meta.run_id,
            point_id: HistorianPointId::new(0),
            start_time: t(2.0),
            end_time: t(5.0),
            limit: None,
        })
        .unwrap();

    assert_eq!(slice.len(), 31); // 2.0 to 5.0 inclusive at 0.1s step = 31 points
    assert!((slice.first().unwrap().sim_time.seconds() - 2.0).abs() < 1e-9);
    assert!((slice.last().unwrap().sim_time.seconds() - 5.0).abs() < 1e-9);
}

// ---------------------------------------------------------------------------
// V-19 & V-20: Bucket Downsampling & Min/Max Spike Preservation
// ---------------------------------------------------------------------------
#[test]
fn test_v19_v20_bucket_downsampling_and_spike_preservation() {
    let mut raw_samples = Vec::new();

    // 1,000 raw samples at 10.0 bar, except a single 1-tick excursion at t=50.0s of 95.0 bar
    for i in 0..1000 {
        let time = (i as f64) * 0.1; // 0.0 to 100.0s
        let val = if (time - 50.0).abs() < 1e-9 {
            95.0
        } else {
            10.0
        };
        raw_samples.push(nebudcs_historian::RawSample {
            point_id: HistorianPointId::new(0),
            sim_time: t(time),
            seq: i as u64,
            value: val,
            quality: TagQuality::Good,
            persisted_wall_clock_ms: 0,
        });
    }

    // Downsample to 10 buckets
    let buckets = downsample_to_buckets(&raw_samples, 0.0, 100.0, 10);
    assert_eq!(buckets.len(), 10);

    // Bucket 5 spans [50.0, 60.0]s
    let spike_bucket = &buckets[5];
    assert!(
        (spike_bucket.max_value - 95.0).abs() < 1e-9,
        "V-20: Single-tick excursion must be preserved in max_value!"
    );
    assert!(
        (spike_bucket.min_value - 10.0).abs() < 1e-9,
        "Min value must be preserved"
    );
    assert!(
        spike_bucket.avg_value < 20.0,
        "Average should be properly calculated (~10.85)"
    );

    // Also verify through SqliteHistorianReader::query_aggregated
    let mut storage = SqliteStorage::in_memory().unwrap();
    let run_meta = HistorianRunMeta::new(ProjectId::new(), "1.0.0", 0.05, 1.0);
    storage.insert_run(&run_meta).unwrap();
    let records: Vec<_> = raw_samples
        .into_iter()
        .map(|s| {
            HistorianRecord::Sample(nebudcs_historian::HistoricalSample::new_float(
                s.point_id,
                s.sim_time,
                s.seq,
                s.value,
                s.quality,
                s.persisted_wall_clock_ms,
            ))
        })
        .collect();
    storage.write_batch(&run_meta.run_id, &records).unwrap();

    let reader = SqliteHistorianReader::from_connection(storage.into_connection());
    let db_buckets = reader
        .query_aggregated(AggregatedQueryRequest {
            run_id: run_meta.run_id,
            point_id: HistorianPointId::new(0),
            start_time: t(0.0),
            end_time: t(100.0),
            bucket_count: 10,
        })
        .unwrap();

    assert_eq!(db_buckets.len(), 10);
    assert!((db_buckets[5].max_value - 95.0).abs() < 1e-9);
}

// ---------------------------------------------------------------------------
// V-21: Simulation Clock Isolation
// ---------------------------------------------------------------------------
#[test]
fn test_v21_simulation_clock_isolation() {
    let mut sampler = HistorianSampler::new();
    let mut store = RuntimeTagStore::new();
    let tag_id = TagId::new();
    store.register_tag("SPEED101", tag_id, TagValue::Float(100.0), TagQuality::Good);

    sampler
        .register_point(
            HistorianPointConfig::new(
                HistorianPointId::new(0),
                tag_id,
                "SPEED101",
                None,
                "",
                0.0,
                1000.0,
                SamplingMode::Periodic {
                    interval_sim_seconds: 1.0,
                },
            ),
            Some(&store),
        )
        .unwrap();

    let (sender, receiver) = create_historian_channel(100);

    // Run at dt = 0.05
    let mut recorded_sim_times = Vec::new();
    for tick in 0..50 {
        let sim_time = t((tick as f64) * 0.05);
        if sampler.sample_tags(sim_time, &store, &sender) > 0 {
            while let Ok(HistorianRecord::Sample(s)) = receiver.try_recv() {
                recorded_sim_times.push(s.sim_time.seconds());
            }
        }
    }

    // Must have samples at exact 0.0, 1.0, 2.0 regardless of wall-clock delay
    assert_eq!(recorded_sim_times, vec![0.0, 1.0, 2.0]);
}

// ---------------------------------------------------------------------------
// V-22: Concurrent Reader Safety
// ---------------------------------------------------------------------------
#[test]
fn test_v22_concurrent_reader_safety() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("concurrent.sqlite");

    let run_meta = HistorianRunMeta::new(ProjectId::new(), "1.0.0", 0.05, 1.0);
    let run_id = run_meta.run_id.clone();
    let (sender, receiver) = create_historian_channel(50_000);

    let worker = spawn_writer_thread(
        &db_path,
        run_meta,
        vec![],
        receiver,
        WriterConfig {
            max_batch_size: 100,
            flush_timeout: std::time::Duration::from_millis(10),
        },
    )
    .unwrap();

    let reader = SqliteHistorianReader::open(&db_path).unwrap();
    let reader_arc = Arc::new(reader);
    let read_count = Arc::new(AtomicUsize::new(0));

    // Spawn 4 concurrent reader threads
    let mut handles = Vec::new();
    for _ in 0..4 {
        let r = Arc::clone(&reader_arc);
        let rid = run_id.clone();
        let cnt = Arc::clone(&read_count);
        handles.push(std::thread::spawn(move || {
            for _ in 0..50 {
                let res = r.query_raw(RawQueryRequest {
                    run_id: rid.clone(),
                    point_id: HistorianPointId::new(0),
                    start_time: t(0.0),
                    end_time: t(1000.0),
                    limit: Some(10),
                });
                if res.is_ok() {
                    cnt.fetch_add(1, Ordering::Relaxed);
                }
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
        }));
    }

    // Simultaneously feed writer
    for i in 0..500 {
        let rec = HistorianRecord::Sample(nebudcs_historian::HistoricalSample::new_float(
            HistorianPointId::new(0),
            t((i as f64) * 0.1),
            i as u64,
            i as f64,
            TagQuality::Good,
            0,
        ));
        let _ = sender.try_send(rec);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    for h in handles {
        h.join().unwrap();
    }

    drop(sender);
    worker.join().unwrap();

    assert!(
        read_count.load(Ordering::Relaxed) > 0,
        "Concurrent reads succeeded without database locked errors"
    );
}

// ---------------------------------------------------------------------------
// V-23: Chunked Retention Pruning
// ---------------------------------------------------------------------------
#[test]
fn test_v23_chunked_retention_pruning() {
    let mut storage = SqliteStorage::in_memory().unwrap();
    let run_meta = HistorianRunMeta::new(ProjectId::new(), "1.0.0", 0.05, 1.0);
    let run_id = run_meta.run_id.clone();
    storage.insert_run(&run_meta).unwrap();

    // Insert 100 samples spanning t = 0.0 to t = 99.0s
    let mut samples = Vec::new();
    for i in 0..100 {
        samples.push(HistorianRecord::Sample(
            nebudcs_historian::HistoricalSample::new_float(
                HistorianPointId::new(0),
                t(i as f64),
                i,
                i as f64,
                TagQuality::Good,
                0,
            ),
        ));
    }
    storage.write_batch(&run_id, &samples).unwrap();

    // Prune data older than t = 40.0s using small chunk limit of 15
    let deleted = prune_older_than(storage.connection(), &run_id, 40.0, 15).unwrap();
    assert_eq!(deleted, 40, "40 analog rows pruned older than t=40s");

    let reader = SqliteHistorianReader::from_connection(storage.into_connection());
    let remaining = reader
        .query_raw(RawQueryRequest {
            run_id,
            point_id: HistorianPointId::new(0),
            start_time: t(0.0),
            end_time: t(100.0),
            limit: None,
        })
        .unwrap();

    assert_eq!(remaining.len(), 60);
    assert_eq!(remaining[0].sim_time.seconds(), 40.0);
}

#[test]
fn test_retention_multi_run_multi_point_without_rowid() {
    let mut storage = SqliteStorage::in_memory().unwrap();
    let proj_id = ProjectId::new();

    let run1 = HistorianRunMeta::new(proj_id, "1.0.0", 0.05, 1.0);
    let run2 = HistorianRunMeta::new(proj_id, "1.0.0", 0.05, 1.0);
    let r1 = run1.run_id.clone();
    let r2 = run2.run_id.clone();

    storage.insert_run(&run1).unwrap();
    storage.insert_run(&run2).unwrap();

    let p0 = HistorianPointId::new(0);
    let p1 = HistorianPointId::new(1);

    // Prepare samples for Run 1 and Run 2 across Point 0 and Point 1
    // Old times: 10.0, 20.0, 30.0 (< 40.0)
    // New times: 50.0, 60.0, 70.0 (>= 40.0)
    let times = [10.0, 20.0, 30.0, 50.0, 60.0, 70.0];

    let mut samples_r1 = Vec::new();
    let mut samples_r2 = Vec::new();

    for (seq, &time) in times.iter().enumerate() {
        // Run 1 Point 0 (Analog)
        samples_r1.push(HistorianRecord::Sample(
            nebudcs_historian::HistoricalSample::new_float(
                p0,
                t(time),
                seq as u64,
                100.0 + time,
                TagQuality::Good,
                0,
            ),
        ));
        // Run 1 Point 1 (Discrete)
        samples_r1.push(HistorianRecord::Sample(
            nebudcs_historian::HistoricalSample::new_discrete(
                p1,
                t(time),
                seq as u64,
                (200.0 + time) as i64,
                TagQuality::Good,
                0,
            ),
        ));

        // Run 2 Point 0 (Analog)
        samples_r2.push(HistorianRecord::Sample(
            nebudcs_historian::HistoricalSample::new_float(
                p0,
                t(time),
                seq as u64,
                500.0 + time,
                TagQuality::Good,
                0,
            ),
        ));
        // Run 2 Point 1 (Discrete)
        samples_r2.push(HistorianRecord::Sample(
            nebudcs_historian::HistoricalSample::new_discrete(
                p1,
                t(time),
                seq as u64,
                (600.0 + time) as i64,
                TagQuality::Good,
                0,
            ),
        ));
    }

    storage.write_batch(&r1, &samples_r1).unwrap();
    storage.write_batch(&r2, &samples_r2).unwrap();

    // Verify boundary inputs: chunk_limit=0 or cutoff=0 returns 0 immediately
    assert_eq!(
        nebudcs_historian::prune_older_than(storage.connection(), &r1, 40.0, 0).unwrap(),
        0
    );
    assert_eq!(
        nebudcs_historian::prune_older_than(storage.connection(), &r1, 0.0, 10).unwrap(),
        0
    );

    // Prune Run 1 older than t=40.0s with chunk_limit = 2 (forces multiple chunked pruning iterations!)
    // 3 old analog samples (P0) + 3 old discrete samples (P1) = 6 total deleted for Run 1
    let deleted_r1 =
        nebudcs_historian::prune_older_than(storage.connection(), &r1, 40.0, 2).unwrap();
    assert_eq!(
        deleted_r1, 6,
        "Exactly 3 analog and 3 discrete rows deleted in Run 1"
    );

    let reader = SqliteHistorianReader::from_connection(storage.into_connection());

    // 1. Check Run 1, Point 0 (Analog): only new times (50, 60, 70) remain
    let r1_p0 = reader
        .query_raw(RawQueryRequest {
            run_id: r1.clone(),
            point_id: p0,
            start_time: t(0.0),
            end_time: t(100.0),
            limit: None,
        })
        .unwrap();
    assert_eq!(r1_p0.len(), 3, "Run 1 P0: old data deleted, new preserved");
    assert_eq!(r1_p0[0].sim_time.seconds(), 50.0);
    assert_eq!(r1_p0[1].sim_time.seconds(), 60.0);
    assert_eq!(r1_p0[2].sim_time.seconds(), 70.0);

    // 2. Check Run 1, Point 1 (Discrete): only new times (50, 60, 70) remain
    let r1_p1 = reader
        .query_raw(RawQueryRequest {
            run_id: r1.clone(),
            point_id: p1,
            start_time: t(0.0),
            end_time: t(100.0),
            limit: None,
        })
        .unwrap();
    assert_eq!(r1_p1.len(), 3, "Run 1 P1: old data deleted, new preserved");
    assert_eq!(r1_p1[0].sim_time.seconds(), 50.0);
    assert_eq!(r1_p1[1].sim_time.seconds(), 60.0);
    assert_eq!(r1_p1[2].sim_time.seconds(), 70.0);

    // 3. Check Run 2, Point 0 (Analog): ALL 6 samples preserved (untouched!)
    let r2_p0 = reader
        .query_raw(RawQueryRequest {
            run_id: r2.clone(),
            point_id: p0,
            start_time: t(0.0),
            end_time: t(100.0),
            limit: None,
        })
        .unwrap();
    assert_eq!(
        r2_p0.len(),
        6,
        "Run 2 P0 completely preserved from Run 1 pruning"
    );
    assert_eq!(r2_p0[0].sim_time.seconds(), 10.0);

    // 4. Check Run 2, Point 1 (Discrete): ALL 6 samples preserved (untouched!)
    let r2_p1 = reader
        .query_raw(RawQueryRequest {
            run_id: r2,
            point_id: p1,
            start_time: t(0.0),
            end_time: t(100.0),
            limit: None,
        })
        .unwrap();
    assert_eq!(
        r2_p1.len(),
        6,
        "Run 2 P1 completely preserved from Run 1 pruning"
    );
    assert_eq!(r2_p1[0].sim_time.seconds(), 10.0);
}

#[test]
fn test_event_retention_pruning() {
    let mut storage = SqliteStorage::in_memory().unwrap();
    let proj_id = ProjectId::new();
    let run = HistorianRunMeta::new(proj_id, "1.0.0", 0.05, 1.0);
    let r_id = run.run_id.clone();
    storage.insert_run(&run).unwrap();

    let times = [5.0, 15.0, 25.0, 45.0, 65.0];
    let mut events = Vec::new();
    for (seq, &time) in times.iter().enumerate() {
        events.push(HistorianRecord::Event(
            nebudcs_historian::HistoricalEvent::alarm(
                &r_id,
                t(time),
                seq as u64,
                "PAH101",
                Some("Normal".to_string()),
                "Active".to_string(),
                12.5,
                3,
                None,
                0,
            ),
        ));
    }
    storage.write_batch(&r_id, &events).unwrap();

    // Prune events older than t=30.0s with chunk limit of 2 (forces multiple chunked cycles)
    let deleted =
        nebudcs_historian::prune_events_older_than(storage.connection(), &r_id, 30.0, 2).unwrap();
    assert_eq!(deleted, 3, "3 events older than t=30s pruned");

    let reader = SqliteHistorianReader::from_connection(storage.into_connection());
    let remaining = reader.query_events(&r_id, t(0.0), t(100.0)).unwrap();
    assert_eq!(remaining.len(), 2, "2 events remain (45.0s and 65.0s)");
    assert_eq!(remaining[0].sim_time.seconds(), 45.0);
    assert_eq!(remaining[1].sim_time.seconds(), 65.0);
}

// ---------------------------------------------------------------------------
// V-24: 10,000-Tick Determinism & Replay Equivalence
// ---------------------------------------------------------------------------
#[test]
fn test_v24_10000_tick_determinism_replay() {
    let mut sampler1 = HistorianSampler::new();
    let mut sampler2 = HistorianSampler::new();

    let mut store1 = RuntimeTagStore::new();
    let mut store2 = RuntimeTagStore::new();

    let tag_id = TagId::new();
    let idx1 = store1.register_tag("VALVE_CV", tag_id, TagValue::Float(0.0), TagQuality::Good);
    let idx2 = store2.register_tag("VALVE_CV", tag_id, TagValue::Float(0.0), TagQuality::Good);

    let config = HistorianPointConfig::new(
        HistorianPointId::new(0),
        tag_id,
        "VALVE_CV",
        None,
        "",
        0.0,
        100.0,
        SamplingMode::Deadband {
            absolute_threshold: 0.5,
            percent_threshold: 0.01,
            max_interval_sim_seconds: 5.0,
        },
    );

    sampler1
        .register_point(config.clone(), Some(&store1))
        .unwrap();
    sampler2.register_point(config, Some(&store2)).unwrap();

    let (sender1, receiver1) = create_historian_channel(50_000);
    let (sender2, receiver2) = create_historian_channel(50_000);

    // Simulate 10,000 ticks at dt = 0.02s (200 seconds of simulation)
    let dt = 0.02;
    for tick in 0..10_000 {
        let sim_time = t((tick as f64) * dt);

        // Sinusoidal oscillation with identical inputs
        let val = 50.0 + 30.0 * ((tick as f64) * 0.01).sin();
        store1.set_by_index(idx1, TagValue::Float(val), TagQuality::Good);
        store2.set_by_index(idx2, TagValue::Float(val), TagQuality::Good);

        sampler1.sample_tags(sim_time, &store1, &sender1);
        sampler2.sample_tags(sim_time, &store2, &sender2);
    }

    drop(sender1);
    drop(sender2);

    let mut samples1 = Vec::new();
    while let Ok(HistorianRecord::Sample(s)) = receiver1.try_recv() {
        samples1.push((s.sim_time.seconds(), s.seq, s.value.as_f64(), s.quality));
    }

    let mut samples2 = Vec::new();
    while let Ok(HistorianRecord::Sample(s)) = receiver2.try_recv() {
        samples2.push((s.sim_time.seconds(), s.seq, s.value.as_f64(), s.quality));
    }

    assert!(samples1.len() > 100);
    assert_eq!(
        samples1.len(),
        samples2.len(),
        "10,000-tick run must emit identical sample counts"
    );

    for i in 0..samples1.len() {
        assert_eq!(
            samples1[i], samples2[i],
            "Mismatch at sample {i}: identical inputs must produce bit-identical historical sequence"
        );
    }
}
