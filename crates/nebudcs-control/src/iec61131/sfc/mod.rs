//! Sequential Function Chart (SFC) Engine for IEC 61131-3.

pub mod compiler;
pub mod model;

pub use compiler::compile_sfc;
pub use model::{SfcAction, SfcActionQualifier, SfcChart, SfcStep, SfcTransition};
