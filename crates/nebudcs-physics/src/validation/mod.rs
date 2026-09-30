//! Scientific validation, benchmark datasets, and evidence governance for NebuDCS Physics.

pub mod benchmarks;
pub mod dataset;
pub mod evidence;
pub mod metrics;
pub mod runner;

pub use benchmarks::*;
pub use dataset::*;
pub use evidence::*;
pub use metrics::*;
pub use runner::*;
