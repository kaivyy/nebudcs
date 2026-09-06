//! NebuDCS Historian & Timeseries Persistence Engine.
//!
//! # Architectural Invariants
//!
//! 1. **Observer-Only Boundary**: The historian is strictly a consumer of committed simulation state.
//!    It never mutates physical or control state.
//! 2. **Zero-Blocking Invariant**: The simulation loop never blocks on SQLite transactions,
//!    filesystem I/O, or database lock acquisition.
//! 3. **Deterministic Time Basis**: All sampling decisions and indices use canonical
//!    [`SimulationTime`](nebudcs_types::SimulationTime).
//! 4. **Zero-Allocation Hot Path**: Sampling inside the simulation loop executes without heap allocation.
//! 5. **Quality Preservation**: Transitions in [`TagQuality`](nebudcs_model::tag::TagQuality) force
//!    an immediate sample regardless of deadband settings.
//! 6. **Clustered Storage**: High-density timeseries tables leverage SQLite WAL mode and
//!    `WITHOUT ROWID` clustering.

pub mod aggregate;
pub mod buffer;
pub mod error;
pub mod event;
pub mod health;
pub mod point;
pub mod query;
pub mod retention;
pub mod run;
pub mod sample;
pub mod sampler;
pub mod sqlite;
pub mod writer;

pub use aggregate::{downsample_to_buckets, AggregatedBucket};
pub use buffer::{
    create_historian_channel, HistorianReceiver, HistorianRecord, HistorianSender,
    DEFAULT_QUEUE_CAPACITY,
};
pub use error::{HistorianError, HistorianResult};
pub use event::HistoricalEvent;
pub use health::{HistorianHealth, HistorianHealthTracker, SharedHealthTracker};
pub use point::{HistorianPointConfig, HistorianPointId, SamplingMode};
pub use query::{AggregatedQueryRequest, HistorianReader, RawQueryRequest, SqliteHistorianReader};
pub use retention::{prune_events_older_than, prune_older_than, RetentionPolicy};
pub use run::{generate_run_id, HistorianRunMeta};
pub use sample::{HistoricalSample, HistoricalSampleValue, RawSample};
pub use sampler::HistorianSampler;
pub use sqlite::SqliteStorage;
pub use writer::{
    spawn_in_memory_writer_thread, spawn_writer_thread, HistorianWriterHandle, WriterConfig,
};
