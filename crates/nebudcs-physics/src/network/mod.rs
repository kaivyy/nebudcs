//! Hydraulic network modeling, nodal pressure solving, and convergence diagnostics.

pub mod diagnostics;
pub mod edge;
pub mod node;
pub mod recycle;
pub mod solver;

pub use diagnostics::{EdgeDiagnostic, FlowRegime, NetworkDiagnostics, NodeDiagnostic};
pub use edge::{EdgeComponent, HydraulicEdge};
pub use node::{HydraulicNode, NodeKind};
pub use recycle::{RecycleConvergenceManager, RecycleDiagnostics};
pub use solver::{HydraulicNetwork, HydraulicNetworkSolver};
