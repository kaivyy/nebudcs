//! NebuDCS Realtime Protocol, DTOs, and Serialization Contracts.
//!
//! # Responsibilities
//!
//! - [`PROTOCOL_VERSION`]: Definitive semantic protocol version string (`"1.0.0"`).
//! - [`ClientMessage`]: Inbound WebSocket command and subscription requests.
//! - [`ServerMessage`]: Outbound WebSocket snapshots, delta updates, events, and acks.
//! - [`RuntimeCommand`]: Strongly typed operator and simulation control actions.
//! - [`StateSnapshotDto`] & [`DeltaUpdateDto`]: Externalized state representations.

pub mod command;
pub mod dto;
pub mod error;
pub mod message;
pub mod version;

pub use command::{PidModeDto, RuntimeCommand, ScenarioCommand, SimControlCommand, TagValueDto};
pub use dto::{
    ActiveFaultDto, AlarmSeverityDto, AlarmSnapshotDto, AlarmStateDto, DeltaUpdateDto,
    HistoricalEventDto, ScenarioStatusDto, ServerHealthDto, StateSnapshotDto, TagQualityDto,
    TagValueSampleDto,
};
pub use error::{ProtocolError, ProtocolErrorCode, ProtocolResult};
pub use message::{ClientCapabilities, ClientMessage, ServerMessage};
pub use version::{is_compatible_protocol_version, PROTOCOL_VERSION};
