//! I/O Hardware Abstraction Layer (HAL).
//!
//! Bridges physical process instruments, field transmitters, and actuators
//! to Virtual Controller Input and Output Image Tables (%I and %Q).

pub mod address;
pub mod channel;
pub mod chassis;
pub mod error;
pub mod mapping;
pub mod module;
pub mod rack;
pub mod scaling;
pub mod signal;
pub mod system;

pub use address::{ChannelKind, IoChannelAddress};
pub use channel::{
    AnalogInputChannel, AnalogOutputChannel, DigitalInputChannel, DigitalOutputChannel, IoChannel,
};
pub use chassis::IoChassis;
pub use error::HalError;
pub use mapping::{validate_mappings, ControllerIoMapping, MappingDirection};
pub use module::{IoModule, IoModuleType};
pub use rack::IoRack;
pub use scaling::AnalogScalingConfig;
pub use signal::{
    AnalogFaultState, AnalogSignalRange, ChannelStatus, ConversionCurve, DigitalFallbackBehavior,
    DigitalFaultState, IoSignalQuality, OutputFallbackBehavior,
};
pub use system::{HalDiagnostics, IoSystem};
