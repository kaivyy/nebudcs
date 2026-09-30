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

quantity!(
    /// Molar flow rate in mol/s.
    MolarFlow,
    "mol/s"
);

quantity!(
    /// Molar enthalpy in J/mol.
    MolarEnthalpy,
    "J/mol"
);

quantity!(
    /// Molar entropy in J/(mol·K).
    MolarEntropy,
    "J/(mol·K)"
);

quantity!(
    /// Specific entropy in J/(kg·K).
    SpecificEntropy,
    "J/(kg·K)"
);

quantity!(
    /// Molar heat capacity in J/(mol·K).
    MolarHeatCapacity,
    "J/(mol·K)"
);

quantity!(
    /// Specific heat capacity in J/(kg·K).
    SpecificHeatCapacity,
    "J/(kg·K)"
);

quantity!(
    /// Molar volume in m³/mol.
    MolarVolume,
    "m³/mol"
);

quantity!(
    /// Molar density in mol/m³.
    MolarDensity,
    "mol/m³"
);

quantity!(
    /// Fugacity in Pascal (Pa).
    Fugacity,
    "Pa"
);

quantity!(
    /// Compressibility factor Z (dimensionless).
    CompressibilityFactor,
    "dimensionless"
);

// ---------------------------------------------------------------------------
// Explicit basis and dimensional conversions
// ---------------------------------------------------------------------------

impl MassFlow {
    /// Explicit conversion to molar flow given molecular weight in kg/mol.
    ///
    /// Explicit conversion method prevents accidental operations like mass_flow + molar_flow.
    pub fn to_molar_flow(self, mw_kg_per_mol: f64) -> NebuResult<MolarFlow> {
        if mw_kg_per_mol <= 0.0 || !mw_kg_per_mol.is_finite() {
            return Err(NebuError::OutOfRange {
                field: "molecular_weight".to_string(),
                value: mw_kg_per_mol,
                min: 1e-6,
                max: 1e3,
            });
        }
        MolarFlow::new(self.0 / mw_kg_per_mol)
    }
}

impl MolarFlow {
    /// Explicit conversion to mass flow given molecular weight in kg/mol.
    pub fn to_mass_flow(self, mw_kg_per_mol: f64) -> NebuResult<MassFlow> {
        if mw_kg_per_mol <= 0.0 || !mw_kg_per_mol.is_finite() {
            return Err(NebuError::OutOfRange {
                field: "molecular_weight".to_string(),
                value: mw_kg_per_mol,
                min: 1e-6,
                max: 1e3,
            });
        }
        MassFlow::new(self.0 * mw_kg_per_mol)
    }
}

impl SpecificEnthalpy {
    /// Explicit conversion to molar enthalpy given molecular weight in kg/mol.
    ///
    /// Prevents accidental mixing of J/kg (specific) and J/mol (molar).
    pub fn to_molar_enthalpy(self, mw_kg_per_mol: f64) -> NebuResult<MolarEnthalpy> {
        if mw_kg_per_mol <= 0.0 || !mw_kg_per_mol.is_finite() {
            return Err(NebuError::OutOfRange {
                field: "molecular_weight".to_string(),
                value: mw_kg_per_mol,
                min: 1e-6,
                max: 1e3,
            });
        }
        MolarEnthalpy::new(self.0 * mw_kg_per_mol)
    }
}

impl MolarEnthalpy {
    /// Explicit conversion to specific enthalpy given molecular weight in kg/mol.
    pub fn to_specific_enthalpy(self, mw_kg_per_mol: f64) -> NebuResult<SpecificEnthalpy> {
        if mw_kg_per_mol <= 0.0 || !mw_kg_per_mol.is_finite() {
            return Err(NebuError::OutOfRange {
                field: "molecular_weight".to_string(),
                value: mw_kg_per_mol,
                min: 1e-6,
                max: 1e3,
            });
        }
        SpecificEnthalpy::new(self.0 / mw_kg_per_mol)
    }
}

impl SpecificEntropy {
    /// Explicit conversion to molar entropy given molecular weight in kg/mol.
    pub fn to_molar_entropy(self, mw_kg_per_mol: f64) -> NebuResult<MolarEntropy> {
        if mw_kg_per_mol <= 0.0 || !mw_kg_per_mol.is_finite() {
            return Err(NebuError::OutOfRange {
                field: "molecular_weight".to_string(),
                value: mw_kg_per_mol,
                min: 1e-6,
                max: 1e3,
            });
        }
        MolarEntropy::new(self.0 * mw_kg_per_mol)
    }
}

impl MolarEntropy {
    /// Explicit conversion to specific entropy given molecular weight in kg/mol.
    pub fn to_specific_entropy(self, mw_kg_per_mol: f64) -> NebuResult<SpecificEntropy> {
        if mw_kg_per_mol <= 0.0 || !mw_kg_per_mol.is_finite() {
            return Err(NebuError::OutOfRange {
                field: "molecular_weight".to_string(),
                value: mw_kg_per_mol,
                min: 1e-6,
                max: 1e3,
            });
        }
        SpecificEntropy::new(self.0 / mw_kg_per_mol)
    }
}

impl SpecificHeatCapacity {
    /// Explicit conversion to molar heat capacity given molecular weight in kg/mol.
    pub fn to_molar_heat_capacity(self, mw_kg_per_mol: f64) -> NebuResult<MolarHeatCapacity> {
        if mw_kg_per_mol <= 0.0 || !mw_kg_per_mol.is_finite() {
            return Err(NebuError::OutOfRange {
                field: "molecular_weight".to_string(),
                value: mw_kg_per_mol,
                min: 1e-6,
                max: 1e3,
            });
        }
        MolarHeatCapacity::new(self.0 * mw_kg_per_mol)
    }
}

impl MolarHeatCapacity {
    /// Explicit conversion to specific heat capacity given molecular weight in kg/mol.
    pub fn to_specific_heat_capacity(self, mw_kg_per_mol: f64) -> NebuResult<SpecificHeatCapacity> {
        if mw_kg_per_mol <= 0.0 || !mw_kg_per_mol.is_finite() {
            return Err(NebuError::OutOfRange {
                field: "molecular_weight".to_string(),
                value: mw_kg_per_mol,
                min: 1e-6,
                max: 1e3,
            });
        }
        SpecificHeatCapacity::new(self.0 / mw_kg_per_mol)
    }
}

impl Density {
    /// Explicit conversion to molar density given molecular weight in kg/mol.
    pub fn to_molar_density(self, mw_kg_per_mol: f64) -> NebuResult<MolarDensity> {
        if mw_kg_per_mol <= 0.0 || !mw_kg_per_mol.is_finite() {
            return Err(NebuError::OutOfRange {
                field: "molecular_weight".to_string(),
                value: mw_kg_per_mol,
                min: 1e-6,
                max: 1e3,
            });
        }
        MolarDensity::new(self.0 / mw_kg_per_mol)
    }
}

impl MolarDensity {
    /// Explicit conversion to mass density given molecular weight in kg/mol.
    pub fn to_mass_density(self, mw_kg_per_mol: f64) -> NebuResult<Density> {
        if mw_kg_per_mol <= 0.0 || !mw_kg_per_mol.is_finite() {
            return Err(NebuError::OutOfRange {
                field: "molecular_weight".to_string(),
                value: mw_kg_per_mol,
                min: 1e-6,
                max: 1e3,
            });
        }
        Density::new(self.0 * mw_kg_per_mol)
    }

    /// Explicit conversion to molar volume in m³/mol.
    pub fn to_molar_volume(self) -> NebuResult<MolarVolume> {
        if self.0 <= 0.0 || !self.0.is_finite() {
            return Err(NebuError::OutOfRange {
                field: "molar_density".to_string(),
                value: self.0,
                min: 1e-12,
                max: 1e12,
            });
        }
        MolarVolume::new(1.0 / self.0)
    }
}

impl MolarVolume {
    /// Explicit conversion to molar density in mol/m³.
    pub fn to_molar_density(self) -> NebuResult<MolarDensity> {
        if self.0 <= 0.0 || !self.0.is_finite() {
            return Err(NebuError::OutOfRange {
                field: "molar_volume".to_string(),
                value: self.0,
                min: 1e-12,
                max: 1e12,
            });
        }
        MolarDensity::new(1.0 / self.0)
    }
}

impl Pressure {
    /// Explicit conversion from gauge bar to absolute Pascal with explicit atmospheric reference.
    /// Prevents gauge pressure being accidentally used as absolute pressure.
    pub fn from_gauge_bar(gauge_bar: f64, p_atm_pa: f64) -> NebuResult<Self> {
        let p_abs = gauge_bar * 100_000.0 + p_atm_pa;
        Self::new(p_abs)
    }

    /// Explicit conversion to gauge bar with explicit atmospheric reference.
    pub fn to_gauge_bar(self, p_atm_pa: f64) -> f64 {
        (self.0 - p_atm_pa) / 100_000.0
    }
}

impl Temperature {
    /// Explicit conversion from Celsius to absolute Kelvin.
    /// Prevents Celsius values being accidentally used directly as thermodynamic Kelvin.
    pub fn from_celsius(deg_c: f64) -> NebuResult<Self> {
        let k = deg_c + 273.15;
        Self::new(k)
    }

    /// Explicit conversion from absolute Kelvin to Celsius.
    pub fn to_celsius(self) -> f64 {
        self.0 - 273.15
    }
}

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

    #[test]
    fn test_molar_and_mass_flow_conversion() {
        let mw = 0.01604; // Methane 16.04 g/mol = 0.01604 kg/mol
        let mass_flow = MassFlow::new(1.604).unwrap(); // 1.604 kg/s
        let molar_flow = mass_flow.to_molar_flow(mw).unwrap();
        assert!((molar_flow.value() - 100.0).abs() < 1e-9);

        let round_trip = molar_flow.to_mass_flow(mw).unwrap();
        assert!((round_trip.value() - 1.604).abs() < 1e-9);

        // Rejection of non-positive/non-finite MW
        assert!(mass_flow.to_molar_flow(0.0).is_err());
        assert!(mass_flow.to_molar_flow(-0.016).is_err());
        assert!(mass_flow.to_molar_flow(f64::NAN).is_err());
    }

    #[test]
    fn test_enthalpy_entropy_molar_conversions() {
        let mw = 0.018015; // Water
        let h_spec = SpecificEnthalpy::new(2_000_000.0).unwrap(); // J/kg
        let h_molar = h_spec.to_molar_enthalpy(mw).unwrap();
        assert!((h_molar.value() - 36_030.0).abs() < 1e-6); // J/mol
        let h_back = h_molar.to_specific_enthalpy(mw).unwrap();
        assert!((h_back.value() - 2_000_000.0).abs() < 1e-6);

        let s_spec = SpecificEntropy::new(4184.0).unwrap();
        let s_molar = s_spec.to_molar_entropy(mw).unwrap();
        assert!((s_molar.value() - 4184.0 * mw).abs() < 1e-6);
        let s_back = s_molar.to_specific_entropy(mw).unwrap();
        assert!((s_back.value() - 4184.0).abs() < 1e-6);
    }

    #[test]
    fn test_molar_density_and_volume() {
        let mw = 0.018015;
        let rho_mass = Density::new(1000.0).unwrap();
        let rho_molar = rho_mass.to_molar_density(mw).unwrap();
        assert!((rho_molar.value() - (1000.0 / 0.018015)).abs() < 1e-6);

        let vol_molar = rho_molar.to_molar_volume().unwrap();
        assert!((vol_molar.value() - (0.018015 / 1000.0)).abs() < 1e-9);

        let rho_molar_back = vol_molar.to_molar_density().unwrap();
        assert!((rho_molar_back.value() - rho_molar.value()).abs() < 1e-6);
    }

    #[test]
    fn test_pressure_gauge_conversion() {
        let p_atm = 101_325.0;
        let p = Pressure::from_gauge_bar(2.0, p_atm).unwrap(); // 2 bar gauge
        assert_eq!(p.value(), 301_325.0);
        let gauge = p.to_gauge_bar(p_atm);
        assert!((gauge - 2.0).abs() < 1e-12);
    }

    #[test]
    fn test_temperature_celsius_conversion() {
        let t = Temperature::from_celsius(25.0).unwrap();
        assert_eq!(t.value(), 298.15);
        assert!((t.to_celsius() - 25.0).abs() < 1e-12);
    }
}
