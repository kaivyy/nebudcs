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

pub use command::{
    EquipmentCommand, LibraryCommand, PidModeDto, RuntimeCommand, ScenarioCommand,
    SimControlCommand, TagValueDto,
};
pub use dto::{
    ActiveFaultDto, AlarmSeverityDto, AlarmSnapshotDto, AlarmStateDto, DeltaUpdateDto,
    EquipmentControlRequestDto, EquipmentDefinitionSummaryDto, EquipmentInstanceSummaryDto,
    ExportLibraryItemResponseDto, HistoricalEventDto, ImportLibraryItemRequestDto,
    InstantiateEquipmentRequestDto, InstantiatePackageRequestDto, InstantiateTemplateRequestDto,
    PackageDetailDto, PackageSummaryDto, ScenarioStatusDto, ServerHealthDto, StateSnapshotDto,
    TagQualityDto, TagValueSampleDto, TemplateDetailDto, TemplateSummaryDto,
    ValidateEquipmentResponseDto, ValidatePackageResponseDto,
};
pub use error::{ProtocolError, ProtocolErrorCode, ProtocolResult};
pub use message::{ClientCapabilities, ClientMessage, ServerMessage};
pub use version::{is_compatible_protocol_version, PROTOCOL_VERSION};
