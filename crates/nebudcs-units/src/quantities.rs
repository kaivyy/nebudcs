//! Strongly typed physical quantity newtypes.
//!
//! Every quantity wraps an `f64` that is always stored in **canonical SI
//! units**.  Display conversion happens at the boundary (see
//! [`conversion`](super::conversion)).
//!
//! # Canonical units
//!
//! | Type                 | Canonical unit |
//! |----------------------|----------------|
//! | `Pressure`           | Pascal (Pa)    |
//! | `Temperature`        | Kelvin (K)     |
//! | `MassFlow`           | kg/s           |
//! | `VolumetricFlow`     | m³/s           |
//! | `Power`              | Watt (W)       |
//! | `Level`              | fraction 0–1   |
//! | `Volume`             | m³             |
//! | `Speed`              | rad/s          |
//! | `VibrationVelocity`  | m/s (RMS)      |
//! | `DifferentialPressure` | Pascal (Pa)  |

use serde::{Deserialize, Serialize};

use nebudcs_types::{NebuError, NebuResult};

// ---------------------------------------------------------------------------
// Internal macro for quantity newtypes.
// ---------------------------------------------------------------------------
macro_rules! quantity {
    (
        $(#[$meta:meta])*
        $name:ident, $unit_label:literal
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(f64);

        impl $name {
            /// Construct from a canonical SI value.
            ///
            /// Returns an error if the value is NaN or infinite.
            pub fn new(value: f64) -> NebuResult<Self> {
                if value.is_nan() || value.is_infinite() {
                    return Err(NebuError::OutOfRange {
                        field: stringify!($name).to_owned(),
                        value,
                        min: f64::NEG_INFINITY,
                        max: f64::INFINITY,
                    });
                }
                Ok(Self(value))
            }

            /// Construct without checking for NaN/Inf.
            ///
            /// # Safety
            ///
            /// Caller must guarantee `value` is finite.  Prefer [`new`](Self::new)
            /// unless in a performance-critical path with a proved-finite input.
            #[inline]
            pub fn new_unchecked(value: f64) -> Self {
                debug_assert!(
                    value.is_finite(),
                    concat!(stringify!($name), " must be finite, got {}"),
                    value
                );
                Self(value)
            }

            /// Return the canonical SI value.
            #[must_use]
            #[inline]
            pub fn value(self) -> f64 {
                self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{} {}", self.0, $unit_label)
            }
        }

        impl std::ops::Add for $name {
            type Output = Self;
            fn add(self, rhs: Self) -> Self {
                // Safety: finite + finite is finite (barring overflow to ±∞,
                // which we accept for now — physics will catch it).
                Self(self.0 + rhs.0)
            }
        }

        impl std::ops::Sub for $name {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self {
                Self(self.0 - rhs.0)
            }
        }

        impl std::ops::Mul<f64> for $name {
            type Output = Self;
            fn mul(self, rhs: f64) -> Self {
                Self(self.0 * rhs)
            }
        }
    };
}

// ---------------------------------------------------------------------------
// Quantity definitions
// ---------------------------------------------------------------------------

quantity!(
    /// Absolute pressure in Pascal (Pa).
    ///
    /// Use `DifferentialPressure` for pressure differences.
    Pressure,
    "Pa"
);

quantity!(
    /// Thermodynamic temperature in Kelvin (K).
    ///
    /// Temperatures are always absolute.  Display conversion to °C or °F
    /// happens at the boundary layer.
    Temperature,
    "K"
);

quantity!(
    /// Mass flow rate in kg/s.
    MassFlow,
    "kg/s"
);

quantity!(
    /// Volumetric flow rate in m³/s.
    VolumetricFlow,
    "m³/s"
);

quantity!(
    /// Power in Watt (W).
    Power,
    "W"
);

quantity!(
    /// Fractional level dimensionless in the range [0.0, 1.0].
    ///
    /// 0.0 = empty, 1.0 = full.
    Level,
    "fraction"
);

quantity!(
    /// Volume in m³.
    Volume,
    "m³"
);

quantity!(
    /// Angular speed in rad/s.
    ///
    /// Convert from RPM: `rpm * 2π / 60`.
    Speed,
    "rad/s"
);

quantity!(
    /// Vibration velocity (RMS) in m/s.
    VibrationVelocity,
    "m/s"
);

quantity!(
    /// Differential pressure in Pascal (Pa).
    DifferentialPressure,
    "Pa"
);

quantity!(
    /// Fluid density in kg/m³.
    Density,
    "kg/m³"
);

quantity!(
    /// Specific enthalpy in J/kg.
    SpecificEnthalpy,
    "J/kg"
);

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pressure_new_valid() {
        let p = Pressure::new(101_325.0).unwrap();
        assert_eq!(p.value(), 101_325.0);
    }

    #[test]
    fn test_pressure_rejects_nan() {
        assert!(Pressure::new(f64::NAN).is_err());
    }

    #[test]
    fn test_pressure_rejects_infinite() {
        assert!(Pressure::new(f64::INFINITY).is_err());
    }

    #[test]
    fn test_temperature_arithmetic() {
        let t1 = Temperature::new_unchecked(300.0);
        let t2 = Temperature::new_unchecked(50.0);
        let sum = t1 + t2;
        assert!((sum.value() - 350.0).abs() < 1e-10);
    }

    #[test]
    fn test_level_scalar_mul() {
        let l = Level::new_unchecked(0.5);
        let scaled = l * 2.0;
        assert!((scaled.value() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_display() {
        let p = Pressure::new_unchecked(101_325.0);
        let s = p.to_string();
        assert!(s.contains("101325") && s.contains("Pa"), "got: {s}");
    }

    #[test]
    fn test_serde_round_trip_pressure() {
        let p = Pressure::new(200_000.0).unwrap();
        let json = serde_json::to_string(&p).unwrap();
        let decoded: Pressure = serde_json::from_str(&json).unwrap();
        assert_eq!(p, decoded);
    }

    #[test]
    fn test_density_and_specific_enthalpy() {
        let rho = Density::new(1000.0).unwrap();
        assert_eq!(rho.value(), 1000.0);
        let h = SpecificEnthalpy::new(250_000.0).unwrap();
        assert_eq!(h.value(), 250_000.0);
    }
}
