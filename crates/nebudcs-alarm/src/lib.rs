//! Industrial alarm engine, ANSI/ISA-18.2 lifecycle, shelving, and limit detection for NebuDCS.

pub mod block;
pub mod engine;
pub mod error;
pub mod system;
pub mod types;

pub use block::{AlarmBlock, AlarmBlockConfig};
pub use engine::AlarmEngine;
pub use error::AlarmError;
pub use system::AlarmSystem;
pub use types::{AlarmSeverity, AlarmState, AlarmType};
