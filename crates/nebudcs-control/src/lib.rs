//! Control engine, PID controllers, actuator arbitration, and protection for NebuDCS.

pub mod actuator;
pub mod advanced;
pub mod cascade;
pub mod error;
pub mod hal;
pub mod iec61131;
pub mod instrument;
pub mod logic;
pub mod pid;
pub mod protection;
pub mod runtime;
pub mod signal;
pub mod sis;
pub mod system;
pub mod validation;

pub use actuator::{
    ActuatorArbiter, ActuatorCommand, ActuatorConfig, ActuatorModel, CommandPriority, FailAction,
};
pub use advanced::{
    AdvancedControlStrategy, AntiSurgeAlarmState, AntiSurgeController, DeadTimeBuffer,
    FirstOrderFilter, HighSelector, LeadLagCompensator, LowSelector, OverrideSelector,
    RatioController, SelectedSource, SelectorType, SplitRangeController, SplitRangeSegment,
    StrategyKind, ValveCharacteristic, ValveCharacterizer,
};
pub use cascade::CascadePair;
pub use error::ControlError;
pub use hal::{
    AnalogFaultState, AnalogInputChannel, AnalogOutputChannel, AnalogScalingConfig,
    AnalogSignalRange, ChannelKind, ChannelStatus, ControllerIoMapping, ConversionCurve,
    DigitalFallbackBehavior, DigitalFaultState, DigitalInputChannel, DigitalOutputChannel,
    HalDiagnostics, HalError, IoChannel, IoChannelAddress, IoChassis, IoModule, IoModuleType,
    IoRack, IoSignalQuality, IoSystem, MappingDirection, OutputFallbackBehavior,
};
pub use iec61131::{
    compile_fbd, compile_ladder, compile_sfc, compile_st, AntiSurgeBlock, CtdBlock, CtuBlock,
    FTrigBlock, FbdConnection, FbdGraph, FbdNode, FbdNodeKind, FilterFirstOrderBlock,
    FunctionBlockInstance, HighSelectorBlock, IecExpr, IecLanguage, IecProgram, IecRoutine,
    IecStatement, IecType, IecValue, IecVm, LdCoil, LdCoilKind, LdContact, LdContactKind,
    LdDiagram, LdElement, LdParallelBranch, LdRung, LeadLagBlock, LowSelectorBlock, MemoryStore,
    PidIecBlock, RTrigBlock, RatioBlock, RsBlock, SfcAction, SfcActionQualifier, SfcChart, SfcStep,
    SfcTransition, SplitRangeBlock, SrBlock, SymbolTable, TofBlock, TonBlock, TpBlock,
    ValveCharBlock, VarScope, VariableDeclaration,
};
pub use instrument::{InstrumentConfig, InstrumentConfigBuilder, InstrumentModel};
pub use logic::{PermissiveGuard, RsLatch, SrLatch, TofTimer, TonTimer};
pub use pid::{PidConfig, PidController, PidDirection, PidMode};
pub use protection::{
    DegradationPolicy, ProtectionBlock, ProtectionConfig, ProtectionState, SensorInput,
    VotingResult, VotingVoter2oo3,
};
pub use runtime::{
    ControllerConfig, ControllerDiagnostics, ControllerFault, ControllerLifecycle,
    ControllerProgram, ControllerTask, CustomRoutineProgram, InterlockProgram, IoImageTable,
    IoSignalValue, PidLoopProgram, TaskType, VirtualController,
};
pub use signal::{ResolvedTag, RuntimeTagStore};
pub use sis::{
    run_simulated_proof_test, CauseEffectCell, CauseEffectMatrix, CauseEntry, EffectEntry,
    ProofTestRecord, ProofTestStepResult, SafetyAuditEvent, SafetyInstrumentedFunction,
    SafetyInstrumentedSystem, SensorChannel, SifActionType, SifBypass, SifConfig, SifDiagnostics,
    SifFinalElement, SifSafeState, SifState, SifVotingResult, TripDirection, VotingArchitecture,
};
pub use system::ControlSystem;
