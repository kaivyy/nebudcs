//! Function Block Diagram (FBD) Engine for IEC 61131-3.

pub mod compiler;
pub mod model;

pub use compiler::compile_fbd;
pub use model::{FbdConnection, FbdGraph, FbdNode, FbdNodeKind};
