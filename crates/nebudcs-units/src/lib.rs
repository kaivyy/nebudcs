//! Strongly typed physical quantities and engineering unit conversion for NebuDCS.
//!
//! See [`quantities`], [`display`], and [`conversion`] modules.

pub mod conversion;
pub mod display;
pub mod quantities;

pub use display::QuantityKind;
pub use quantities::{
    Density, DifferentialPressure, Level, MassFlow, Power, Pressure, SpecificEnthalpy, Speed,
    Temperature, VibrationVelocity, Volume, VolumetricFlow,
};
