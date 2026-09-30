//! Observability, metrics collection, and health probes subsystem.

pub mod metrics;

pub use metrics::{collect_metrics, read_process_rss_bytes, server_start_time, ServerMetricsDto};
