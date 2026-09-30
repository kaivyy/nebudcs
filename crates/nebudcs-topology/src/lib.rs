//! Generic graph-based topology engine, port validation, piping, signal networks,
//! and recycle (SCC) detection for NebuDCS.
//!
//! # Core Responsibilities
//!
//! - Validates equipment port interconnections across domain, direction, phase, and cardinality constraints.
//! - Separates process/material connectivity, first-class utility networks, and instrument signal networks.
//! - Models physical piping connections with independent flow direction semantics.
//! - Performs deterministic graph queries (neighbors, upstream/downstream, reachability, connected components).
//! - Identifies cyclic recycle regions using Tarjan's Strongly Connected Components (SCC) algorithm.
//!
//! # Invariants
//!
//! - This crate answers "WHAT IS CONNECTED TO WHAT?".
//! - It does NOT calculate physical flows, pressures, temperatures, or simulation steps.
//! - All operations and query outputs are deterministic.

pub mod error;
pub mod graph;
pub mod pipe;
pub mod port;
pub mod scc;
pub mod signal;
pub mod validation;

pub use error::TopologyError;
pub use graph::TopologyGraph;
pub use pipe::{PipeEndpoint, PipeSegment};
pub use port::{
    validate_cardinality, validate_directions, validate_domains, validate_phases,
    validate_port_compatibility, validate_stream_type_domain,
};
pub use scc::{find_strongly_connected_components, SccAnalysisResult};
pub use signal::{SignalConnection, SignalEndpoint, SignalMedium};
pub use validation::{validate_project, validate_unit_contents, ProjectValidationSummary};
