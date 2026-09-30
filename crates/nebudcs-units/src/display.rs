//! Display unit enumerations and the `QuantityKind` discriminant.
//!
//! Each physical quantity type has a corresponding display unit enum.  These
//! are used only at the I/O boundary — the simulation core always works in
//! canonical SI.

use serde::{Deserialize, Serialize};

/// Discriminant for runtime identification of which physical quantity a value
/// represents.  Used by [`UnitAwareValue`](super::conversion::UnitAwareValue).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuantityKind {
    Pressure,
    Temperature,
    MassFlow,
    VolumetricFlow,
    Power,
    Level,
    Volume,
    Speed,
    VibrationVelocity,
    DifferentialPressure,
    Density,
    SpecificEnthalpy,
    MolarFlow,
    MolarEnthalpy,
    MolarEntropy,
    SpecificEntropy,
    MolarHeatCapacity,
    SpecificHeatCapacity,
    MolarVolume,
    MolarDensity,
    Fugacity,
    CompressibilityFactor,
}

// ---------------------------------------------------------------------------
// Display unit enumerations — one per quantity type.
// ---------------------------------------------------------------------------

/// Display units for pressure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PressureUnit {
    /// Pascal (canonical)
    Pascal,
    /// Kilopascal
    KiloPascal,
    /// Bar (1 bar = 100 000 Pa)
    Bar,
    /// Pounds per square inch (1 psi ≈ 6894.76 Pa)
    Psi,
    /// Bar gauge (display only — same scale as bar, offset by 1 atm when
    /// converting absolute to gauge, which requires a reference)
    BarG,
    /// kg/cm² gauge (1 kg/cm² = 98 066.5 Pa)
    KgPerCm2G,
}

/// Display units for temperature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemperatureUnit {
    /// Kelvin (canonical)
    Kelvin,
    /// Degrees Celsius
    Celsius,
    /// Degrees Fahrenheit
    Fahrenheit,
}

/// Display units for mass flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MassFlowUnit {
    /// kg/s (canonical)
    KgPerSec,
    /// kg/h
    KgPerHour,
    /// t/h (metric tonnes per hour)
    TonnesPerHour,
    /// t/d (metric tonnes per day)
    TonnesPerDay,
    /// lb/h (pounds per hour)
    PoundsPerHour,
}

/// Display units for volumetric flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VolumetricFlowUnit {
    /// m³/s (canonical)
    M3PerSec,
    /// m³/h
    M3PerHour,
    /// m³/day
    M3PerDay,
    /// US barrels per day (1 BPD ≈ 1.840130728e-6 m³/s)
    BarrelsPerDay,
    /// Barrels per stream day (BPSD, 1 BPSD = 1 BPD)
    BarrelsPerStreamDay,
    /// Litres per minute (1 L/min = 1.666667e-5 m³/s)
    LitresPerMin,
    /// Normal cubic meters per hour (Nm³/h, at 0 °C, 1.01325 bar)
    NormalM3PerHour,
    /// Standard cubic feet per hour (SCFH, at 60 °F, 14.696 psi)
    StandardCubicFeetPerHour,
}

/// Display units for power.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerUnit {
    /// Watt (canonical)
    Watt,
    /// Kilowatt
    KiloWatt,
    /// Megawatt
    MegaWatt,
}

/// Display units for speed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeedUnit {
    /// rad/s (canonical)
    RadPerSec,
    /// Revolutions per minute
    Rpm,
}

/// Display units for vibration velocity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VibrationUnit {
    /// m/s RMS (canonical)
    MPerSec,
    /// mm/s RMS
    MmPerSec,
    /// inches/s RMS
    InchesPerSec,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantity_kind_serde() {
        let k = QuantityKind::Pressure;
        let json = serde_json::to_string(&k).unwrap();
        let decoded: QuantityKind = serde_json::from_str(&json).unwrap();
        assert_eq!(k, decoded);
    }

    #[test]
    fn test_unit_aware_value_quantity_kind() {
        // Verify QuantityKind::Pressure can be distinguished from Temperature.
        assert_ne!(QuantityKind::Pressure, QuantityKind::Temperature);
        assert_ne!(QuantityKind::Speed, QuantityKind::VibrationVelocity);
    }

    #[test]
    fn test_all_quantity_kinds_are_distinct() {
        let kinds = [
            QuantityKind::Pressure,
            QuantityKind::Temperature,
            QuantityKind::MassFlow,
            QuantityKind::VolumetricFlow,
            QuantityKind::Power,
            QuantityKind::Level,
            QuantityKind::Volume,
            QuantityKind::Speed,
            QuantityKind::VibrationVelocity,
            QuantityKind::DifferentialPressure,
            QuantityKind::Density,
            QuantityKind::SpecificEnthalpy,
            QuantityKind::MolarFlow,
            QuantityKind::MolarEnthalpy,
            QuantityKind::MolarEntropy,
            QuantityKind::SpecificEntropy,
            QuantityKind::MolarHeatCapacity,
            QuantityKind::SpecificHeatCapacity,
            QuantityKind::MolarVolume,
            QuantityKind::MolarDensity,
            QuantityKind::Fugacity,
            QuantityKind::CompressibilityFactor,
        ];
        let set: std::collections::HashSet<_> = kinds
            .iter()
            .map(|k| serde_json::to_string(k).unwrap())
            .collect();
        assert_eq!(set.len(), kinds.len(), "duplicate QuantityKind variants");
    }
}
