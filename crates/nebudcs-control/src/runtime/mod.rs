//! Virtual Industrial Controller Runtime Engine (Stage 1).
//!
//! Exposes virtual controllers, deterministic task scheduling, I/O memory images (%I/%Q),
//! program execution routines, diagnostics, and watchdog timers.

pub mod controller;
pub mod io_image;
pub mod module;
pub mod program;
pub mod task;

pub use controller::{
    ControllerConfig, ControllerDiagnostics, ControllerFault, ControllerLifecycle, CpuProfile,
    FirmwareProfile, MemoryProfile, RedundancyRole, RedundancyState, RedundancyStatus,
    RetentiveStore, VirtualController,
};
pub use io_image::{IoImageTable, IoSignalValue};
pub use module::{ControlModuleMode, ControlModuleStatus, DcsControlModule, ModuleStrategy};
pub use program::{
    ControllerProgram, CustomRoutineProgram, InterlockProgram, PidLoopProgram, RoutineProgram,
};
pub use task::{ControllerTask, TaskType};
