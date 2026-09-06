//! Bucket downsampling and statistical aggregation preserving min/max excursions.

use nebudcs_model::tag::TagQuality;
use serde::{Deserialize, Serialize};

use crate::sample::RawSample;

/// Summary statistics for an aggregated time bucket.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AggregatedBucket {
    /// Zero-based index of this bucket in the query response.
    pub bucket_index: usize,
    /// Simulation start time of bucket window.
    pub start_time: f64,
    /// Simulation end time of bucket window.
    pub end_time: f64,
    /// Minimum raw value observed in this bucket.
    pub min_value: f64,
    /// Maximum raw value observed in this bucket.
    pub max_value: f64,
    /// First chronological sample value in this bucket.
    pub first_value: f64,
    /// Last chronological sample value in this bucket.
    pub last_value: f64,
    /// Arithmetic mean value over all samples in this bucket.
    pub avg_value: f64,
    /// Worst signal quality encountered across all points in this bucket.
    pub worst_quality: TagQuality,
    /// Total count of raw samples aggregated into this bucket.
    pub count: usize,
}

/// Computes a numerical severity rank for [`TagQuality`] for worst-quality tracking.
#[inline]
fn quality_rank(q: TagQuality) -> u8 {
    match q {
        TagQuality::Good => 0,
        TagQuality::NotConnected => 1,
        TagQuality::Uncertain => 2,
        TagQuality::Bad => 3,
    }
}

/// Downsamples a contiguous series of raw samples into uniform temporal buckets.
///
/// # Invariants
/// - **Min/Max Preservation**: Spikes and transient trips are preserved and never lost in averages.
/// - **Deterministic Output**: Processing the same sample stream always yields identical buckets.
pub fn downsample_to_buckets(
    samples: &[RawSample],
    start_time: f64,
    end_time: f64,
    bucket_count: usize,
) -> Vec<AggregatedBucket> {
    if bucket_count == 0 || start_time >= end_time {
        return Vec::new();
    }

    let bucket_dt = (end_time - start_time) / (bucket_count as f64);
    let mut buckets: Vec<AggregatedBucket> = Vec::with_capacity(bucket_count);

    // Initialize empty buckets
    for i in 0..bucket_count {
        let b_start = start_time + (i as f64) * bucket_dt;
        let b_end = if i == bucket_count - 1 {
            end_time
        } else {
            b_start + bucket_dt
        };

        buckets.push(AggregatedBucket {
            bucket_index: i,
            start_time: b_start,
            end_time: b_end,
            min_value: f64::INFINITY,
            max_value: f64::NEG_INFINITY,
            first_value: 0.0,
            last_value: 0.0,
            avg_value: 0.0,
            worst_quality: TagQuality::Good,
            count: 0,
        });
    }

    // Accumulate samples into buckets
    let mut sums: Vec<f64> = vec![0.0; bucket_count];

    for s in samples {
        let t = s.sim_time.seconds();
        if t < start_time || t > end_time {
            continue;
        }

        let mut b_idx = ((t - start_time) / bucket_dt).floor() as usize;
        if b_idx >= bucket_count {
            b_idx = bucket_count - 1;
        }

        let b = &mut buckets[b_idx];
        let val = s.value;

        if b.count == 0 {
            b.first_value = val;
            b.min_value = val;
            b.max_value = val;
            b.worst_quality = s.quality;
        } else {
            if val < b.min_value {
                b.min_value = val;
            }
            if val > b.max_value {
                b.max_value = val;
            }
            if quality_rank(s.quality) > quality_rank(b.worst_quality) {
                b.worst_quality = s.quality;
            }
        }

        b.last_value = val;
        sums[b_idx] += val;
        b.count += 1;
    }

    // Finalize averages and handle empty buckets
    let mut last_known_val = 0.0;
    let mut last_known_qual = TagQuality::Good;

    for (i, b) in buckets.iter_mut().enumerate() {
        if b.count > 0 {
            b.avg_value = sums[i] / (b.count as f64);
            last_known_val = b.last_value;
            last_known_qual = b.worst_quality;
        } else {
            // Carry forward last known state if empty
            b.min_value = last_known_val;
            b.max_value = last_known_val;
            b.first_value = last_known_val;
            b.last_value = last_known_val;
            b.avg_value = last_known_val;
            b.worst_quality = last_known_qual;
        }
    }

    buckets
}
