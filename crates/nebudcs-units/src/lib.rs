//! Strongly typed physical quantities and engineering unit conversion for NebuDCS.
//!
//! See [`quantities`], [`display`], and [`conversion`] modules.

pub mod conversion;
pub mod display;
pub mod quantities;

pub use conversion::{parse_flow, parse_pressure, parse_temperature};
pub use display::{
    MassFlowUnit, PowerUnit, PressureUnit, QuantityKind, SpeedUnit, TemperatureUnit, VibrationUnit,
    VolumetricFlowUnit,
};
pub use quantities::{
    CompressibilityFactor, Density, DifferentialPressure, Fugacity, Level, MassFlow, MolarDensity,
    MolarEnthalpy, MolarEntropy, MolarFlow, MolarHeatCapacity, MolarVolume, Power, Pressure,
    SpecificEnthalpy, SpecificEntropy, SpecificHeatCapacity, Speed, Temperature, VibrationVelocity,
    Volume, VolumetricFlow,
};
