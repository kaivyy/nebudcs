//! NebuDCS Realtime Protocol, DTOs, and Serialization Contracts.
//!
//! # Responsibilities
//!
//! - [`PROTOCOL_VERSION`]: Definitive semantic protocol version string (`"1.0.0"`).
//! - [`ClientMessage`]: Inbound WebSocket command and subscription requests.
//! - [`ServerMessage`]: Outbound WebSocket snapshots, delta updates, events, and acks.
//! - [`RuntimeCommand`]: Strongly typed operator and simulation control actions.
//! - [`StateSnapshotDto`] & [`DeltaUpdateDto`]: Externalized state representations.

pub mod cluster;
pub mod command;
pub mod dto;
pub mod error;
pub mod industrial;
pub mod message;
pub mod security;
pub mod training;
pub mod validation;
pub mod version;

pub use command::{
    EquipmentCommand, LibraryCommand, PidModeDto, RuntimeCommand, ScenarioCommand,
    SimControlCommand, TagValueDto,
};
pub use dto::{
    ActiveFaultDto, AdvancedControlStrategyDto, AlarmFloodMetricsDto, AlarmSeverityDto,
    AlarmSnapshotDto, AlarmStateDto, AlarmSummaryDto, BoundaryConditionDto, CauseEffectCellDto,
    CauseEffectMatrixDto, CauseEntryDto, CommunicationsConfigDto, ConnectionTestResultDto,
    ControlModuleActionRequestDto, InitialConditionDto,
    ControllerConfigDto, ControllerDetailDto, ControllerFaultDto,
    ControllerFaultInjectionRequestDto, ControllerSummaryDto, ControllerTaskDto, CpuProfileDto,
    CreateProjectRequestDto, CrossReferenceEntryDto, CrossReferenceQueryDto,
    CrossReferenceResultDto, DcsControlModuleDto, DeltaUpdateDto, DisplayDto,
    DomainPackageDetailDto, DomainPackageSummaryDto, EffectEntryDto, EquipmentControlRequestDto,
    EquipmentDefinitionSummaryDto, EquipmentDiagnosticsDto, EquipmentInstanceSummaryDto,
    ExportLibraryItemResponseDto, FirmwareProfileDto, FirstOutEventDto, HalChannelDto,
    HalChassisDto, HalDiagnosticsDto, HalFaultRequestDto, HalMappingDto, HalModuleDto, HalRackDto,
    HalSystemSummaryDto, HistoricalEventDto, IecCompileRequestDto, IecCompileResponseDto,
    IecRoutineDto, IecVariableDto, ImportLibraryItemRequestDto, IndustrialZoneDto,
    InstantiateEquipmentRequestDto, InstantiatePackageRequestDto, InstantiateTemplateRequestDto,
    MemoryProfileDto, NavigationLinkDto, DisplayElementPlacementDto, OperatingRangeDto, OperatorActionAuditDto, PackageDetailDto,
    PackageSummaryDto, PlantAreaDto, PlantStructureDto, PlantSubsystemDto, PlantSystemDto,
    PlantUnitDto, Position2dDto, ProgramConfigDto, ProjectDetailDto, ProjectSummaryDto,
    ProjectTagDto, ProjectValidationResponseDto, ProofTestExecutionRequestDto, ProofTestRecordDto,
    ProofTestStepResultDto, ProtocolConnectionDto, ProtocolDiagnosticsDto, RedundancyConfigDto,
    RedundancyStatusDto, SaveIecRoutineRequestDto, SaveProjectRequestDto, ScenarioStatusDto,
    ServerHealthDto, SifBypassDto, SifBypassRequestDto, SifConfigDto, SifDetailDto,
    SifDiagnosticsDto, SifFinalElementConfigDto, SifFinalElementDto, SifResetRequestDto,
    SifSensorChannelDto, SisConfigDto, SisStatusSummaryDto, StateSnapshotDto, StudioAlarmDto,
    StudioControlLoopDto, StudioEquipmentDto, StudioInstrumentDto, StudioInterlockDto,
    StudioStreamDto, TagMappingDto, TagQualityDto, TagValueSampleDto, TaskConfigDto,
    TemplateDetailDto, TemplatePortDto, TemplateSummaryDto, UpdateControllerStateRequestDto,
    ValidateEquipmentResponseDto, ValidatePackageResponseDto, ValidationResultDto,
    ValidityStateDto, ViewportConfigDto,
};
pub use cluster::*;
pub use error::{ProtocolError, ProtocolErrorCode, ProtocolResult};
pub use industrial::*;
pub use message::{ClientCapabilities, ClientMessage, ServerMessage};
pub use security::*;
pub use training::*;
pub use validation::*;
pub use version::{is_compatible_protocol_version, PROTOCOL_VERSION};
