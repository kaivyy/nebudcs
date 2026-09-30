//! Advanced rotating equipment physics: compressor performance maps and pump cavitation.

pub mod compressor;
pub mod pump;

pub use compressor::{
    CompressorMapModel, CompressorMapPoint, CompressorOperatingState, CompressorSpeedLine,
};
pub use pump::{CavitationRegime, PumpCavitationModel};
