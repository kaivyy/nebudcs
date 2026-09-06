//! Control engine, PID controllers, actuator arbitration, and protection for NebuDCS.

pub mod actuator;
pub mod cascade;
pub mod error;
pub mod instrument;
pub mod logic;
pub mod pid;
pub mod protection;
pub mod signal;
pub mod system;

pub use actuator::{
    ActuatorArbiter, ActuatorCommand, ActuatorConfig, ActuatorModel, CommandPriority, FailAction,
};
pub use cascade::CascadePair;
pub use error::ControlError;
pub use instrument::{InstrumentConfig, InstrumentConfigBuilder, InstrumentModel};
pub use logic::{PermissiveGuard, RsLatch, SrLatch, TofTimer, TonTimer};
pub use pid::{PidConfig, PidController, PidDirection, PidMode};
pub use protection::{
    DegradationPolicy, ProtectionBlock, ProtectionConfig, ProtectionState, SensorInput,
    VotingResult, VotingVoter2oo3,
};
pub use signal::{ResolvedTag, RuntimeTagStore};
pub use system::ControlSystem;
