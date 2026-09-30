//! Unit conversion between canonical SI and display units.
//!
//! All conversions are stateless pure functions.  They are called **only** at
//! the I/O boundary — when reading user input or rendering values to a UI.
//! The simulation engine never calls these functions in the hot path.

use std::f64::consts::PI;

use serde::{Deserialize, Serialize};

use crate::display::{
    MassFlowUnit, PowerUnit, PressureUnit, QuantityKind, SpeedUnit, TemperatureUnit, VibrationUnit,
    VolumetricFlowUnit,
};
use crate::quantities::{
    MassFlow, Power, Pressure, Speed, Temperature, VibrationVelocity, VolumetricFlow,
};

// ---------------------------------------------------------------------------
// Pressure
// ---------------------------------------------------------------------------

impl Pressure {
    /// Convert canonical Pa value to a display unit.
    #[must_use]
    pub fn to_display(self, unit: PressureUnit) -> f64 {
        let v = self.value();
        match unit {
            PressureUnit::Pascal => v,
            PressureUnit::KiloPascal => v / 1_000.0,
            PressureUnit::Bar | PressureUnit::BarG => v / 100_000.0,
            PressureUnit::Psi => v / 6_894.757_293_168_36,
            PressureUnit::KgPerCm2G => v / 98_066.5,
        }
    }

    /// Construct from a display-unit value, converting to canonical Pa.
    #[must_use]
    pub fn from_display(value: f64, unit: PressureUnit) -> Self {
        let pa = match unit {
            PressureUnit::Pascal => value,
            PressureUnit::KiloPascal => value * 1_000.0,
            PressureUnit::Bar | PressureUnit::BarG => value * 100_000.0,
            PressureUnit::Psi => value * 6_894.757_293_168_36,
            PressureUnit::KgPerCm2G => value * 98_066.5,
        };
        Self::new_unchecked(pa)
    }
}

// ---------------------------------------------------------------------------
// Temperature
// ---------------------------------------------------------------------------

impl Temperature {
    /// Convert canonical Kelvin to a display unit.
    #[must_use]
    pub fn to_display(self, unit: TemperatureUnit) -> f64 {
        let v = self.value();
        match unit {
            TemperatureUnit::Kelvin => v,
            TemperatureUnit::Celsius => v - 273.15,
            TemperatureUnit::Fahrenheit => (v - 273.15) * 9.0 / 5.0 + 32.0,
        }
    }

    /// Construct from a display-unit value, converting to canonical Kelvin.
    #[must_use]
    pub fn from_display(value: f64, unit: TemperatureUnit) -> Self {
        let k = match unit {
            TemperatureUnit::Kelvin => value,
            TemperatureUnit::Celsius => value + 273.15,
            TemperatureUnit::Fahrenheit => (value - 32.0) * 5.0 / 9.0 + 273.15,
        };
        Self::new_unchecked(k)
    }
}

// ---------------------------------------------------------------------------
// MassFlow
// ---------------------------------------------------------------------------

impl MassFlow {
    /// Convert canonical kg/s to a display unit.
    #[must_use]
    pub fn to_display(self, unit: MassFlowUnit) -> f64 {
        let v = self.value();
        match unit {
            MassFlowUnit::KgPerSec => v,
            MassFlowUnit::KgPerHour => v * 3_600.0,
            MassFlowUnit::TonnesPerHour => v * 3.6,
            MassFlowUnit::TonnesPerDay => v * 86.4,
            MassFlowUnit::PoundsPerHour => v * 3_600.0 / 0.453_592_37,
        }
    }

    /// Construct from a display-unit value, converting to canonical kg/s.
    #[must_use]
    pub fn from_display(value: f64, unit: MassFlowUnit) -> Self {
        let kgs = match unit {
            MassFlowUnit::KgPerSec => value,
            MassFlowUnit::KgPerHour => value / 3_600.0,
            MassFlowUnit::TonnesPerHour => value / 3.6,
            MassFlowUnit::TonnesPerDay => value * 1_000.0 / 86_400.0,
            MassFlowUnit::PoundsPerHour => value * 0.453_592_37 / 3_600.0,
        };
        Self::new_unchecked(kgs)
    }
}

// ---------------------------------------------------------------------------
// VolumetricFlow
// ---------------------------------------------------------------------------

/// 1 US barrel per day in m³/s.
const BPD_TO_M3S: f64 = 1.840_130_728e-6;
/// 1 L/min in m³/s.
const LPM_TO_M3S: f64 = 1.666_666_7e-5;
/// 1 m³ to standard cubic feet.
const M3_TO_SCF: f64 = 35.314_666_7;

impl VolumetricFlow {
    /// Convert canonical m³/s to a display unit.
    #[must_use]
    pub fn to_display(self, unit: VolumetricFlowUnit) -> f64 {
        let v = self.value();
        match unit {
            VolumetricFlowUnit::M3PerSec => v,
            VolumetricFlowUnit::M3PerHour => v * 3_600.0,
            VolumetricFlowUnit::M3PerDay => v * 86_400.0,
            VolumetricFlowUnit::BarrelsPerDay | VolumetricFlowUnit::BarrelsPerStreamDay => {
                v / BPD_TO_M3S
            }
            VolumetricFlowUnit::LitresPerMin => v / LPM_TO_M3S,
            VolumetricFlowUnit::NormalM3PerHour => v * 3_600.0,
            VolumetricFlowUnit::StandardCubicFeetPerHour => v * 3_600.0 * M3_TO_SCF,
        }
    }

    /// Construct from a display-unit value, converting to canonical m³/s.
    #[must_use]
    pub fn from_display(value: f64, unit: VolumetricFlowUnit) -> Self {
        let m3s = match unit {
            VolumetricFlowUnit::M3PerSec => value,
            VolumetricFlowUnit::M3PerHour => value / 3_600.0,
            VolumetricFlowUnit::M3PerDay => value / 86_400.0,
            VolumetricFlowUnit::BarrelsPerDay | VolumetricFlowUnit::BarrelsPerStreamDay => {
                value * BPD_TO_M3S
            }
            VolumetricFlowUnit::LitresPerMin => value * LPM_TO_M3S,
            VolumetricFlowUnit::NormalM3PerHour => value / 3_600.0,
            VolumetricFlowUnit::StandardCubicFeetPerHour => value / (3_600.0 * M3_TO_SCF),
        };
        Self::new_unchecked(m3s)
    }
}

/// Canonical parsing from string unit and numeric value into canonical MassFlow and VolumetricFlow.
pub fn parse_flow(
    value: f64,
    unit_str: &str,
    density_kg_m3: f64,
    _molecular_weight: Option<f64>,
) -> Result<(MassFlow, VolumetricFlow), String> {
    let clean = unit_str.trim().to_lowercase().replace('³', "3");
    let rho = if density_kg_m3 > 0.0 {
        density_kg_m3
    } else {
        1000.0
    };

    match clean.as_str() {
        "bpsd" | "bpd" | "bbl/d" | "bbl/day" | "barrel/d" | "barrels/day" => {
            let m3s = value * BPD_TO_M3S;
            let kgs = m3s * rho;
            Ok((
                MassFlow::new_unchecked(kgs),
                VolumetricFlow::new_unchecked(m3s),
            ))
        }
        "m3/h" | "m3/hr" | "m3/hour" => {
            let m3s = value / 3_600.0;
            let kgs = m3s * rho;
            Ok((
                MassFlow::new_unchecked(kgs),
                VolumetricFlow::new_unchecked(m3s),
            ))
        }
        "m3/d" | "m3/day" => {
            let m3s = value / 86_400.0;
            let kgs = m3s * rho;
            Ok((
                MassFlow::new_unchecked(kgs),
                VolumetricFlow::new_unchecked(m3s),
            ))
        }
        "m3/s" | "m3/sec" => {
            let m3s = value;
            let kgs = m3s * rho;
            Ok((
                MassFlow::new_unchecked(kgs),
                VolumetricFlow::new_unchecked(m3s),
            ))
        }
        "l/min" | "lpm" | "litres/min" => {
            let m3s = value * LPM_TO_M3S;
            let kgs = m3s * rho;
            Ok((
                MassFlow::new_unchecked(kgs),
                VolumetricFlow::new_unchecked(m3s),
            ))
        }
        "kg/s" | "kg/sec" => {
            let kgs = value;
            let m3s = kgs / rho;
            Ok((
                MassFlow::new_unchecked(kgs),
                VolumetricFlow::new_unchecked(m3s),
            ))
        }
        "kg/h" | "kg/hr" => {
            let kgs = value / 3_600.0;
            let m3s = kgs / rho;
            Ok((
                MassFlow::new_unchecked(kgs),
                VolumetricFlow::new_unchecked(m3s),
            ))
        }
        "t/h" | "tonne/h" | "tonnes/h" | "tph" => {
            let kgs = value * 1_000.0 / 3_600.0;
            let m3s = kgs / rho;
            Ok((
                MassFlow::new_unchecked(kgs),
                VolumetricFlow::new_unchecked(m3s),
            ))
        }
        "t/d" | "tonne/d" | "tonnes/d" | "tonnes/day" => {
            let kgs = value * 1_000.0 / 86_400.0;
            let m3s = kgs / rho;
            Ok((
                MassFlow::new_unchecked(kgs),
                VolumetricFlow::new_unchecked(m3s),
            ))
        }
        "lb/h" | "lb/hr" | "lbs/h" | "lbs/hr" => {
            let kgs = value * 0.453_592_37 / 3_600.0;
            let m3s = kgs / rho;
            Ok((
                MassFlow::new_unchecked(kgs),
                VolumetricFlow::new_unchecked(m3s),
            ))
        }
        "nm3/h" | "nm3/hr" => {
            let m3s = value / 3_600.0;
            let kgs = m3s * rho;
            Ok((
                MassFlow::new_unchecked(kgs),
                VolumetricFlow::new_unchecked(m3s),
            ))
        }
        "scf/h" | "scfh" | "scf/hr" => {
            let m3s = value / (3_600.0 * M3_TO_SCF);
            let kgs = m3s * rho;
            Ok((
                MassFlow::new_unchecked(kgs),
                VolumetricFlow::new_unchecked(m3s),
            ))
        }
        unknown => Err(format!("Unsupported flow unit '{unknown}'")),
    }
}

/// Canonical parsing from string unit and numeric value into canonical Pressure.
pub fn parse_pressure(value: f64, unit_str: &str) -> Result<Pressure, String> {
    let clean = unit_str.trim().to_lowercase().replace('²', "2");
    match clean.as_str() {
        "pa" | "pascal" => Ok(Pressure::new_unchecked(value)),
        "kpa" | "kilopascal" => Ok(Pressure::new_unchecked(value * 1_000.0)),
        "bar" | "bara" | "bar(a)" => Ok(Pressure::new_unchecked(value * 100_000.0)),
        "barg" | "bar(g)" | "bar_g" => Ok(Pressure::new_unchecked((value + 1.013_25) * 100_000.0)),
        "psi" | "psia" => Ok(Pressure::new_unchecked(value * 6_894.757_293_168_36)),
        "psig" => Ok(Pressure::new_unchecked(
            (value + 14.6959) * 6_894.757_293_168_36,
        )),
        "kg/cm2" | "kg/cm2a" => Ok(Pressure::new_unchecked(value * 98_066.5)),
        "kg/cm2g" | "kg/cm2(g)" => Ok(Pressure::new_unchecked((value + 1.033_23) * 98_066.5)),
        unknown => Err(format!("Unsupported pressure unit '{unknown}'")),
    }
}

/// Canonical parsing from string unit and numeric value into canonical Temperature.
pub fn parse_temperature(value: f64, unit_str: &str) -> Result<Temperature, String> {
    let clean = unit_str.trim().to_lowercase().replace('°', "");
    match clean.as_str() {
        "k" | "kelvin" => Ok(Temperature::new_unchecked(value)),
        "c" | "celsius" | "degc" => Ok(Temperature::new_unchecked(value + 273.15)),
        "f" | "fahrenheit" | "degf" => {
            Ok(Temperature::new_unchecked((value - 32.0) * 5.0 / 9.0 + 273.15))
        }
        unknown => Err(format!("Unsupported temperature unit '{unknown}'")),
    }
}

// ---------------------------------------------------------------------------
// Power
// ---------------------------------------------------------------------------

impl Power {
    /// Convert canonical Watt to a display unit.
    #[must_use]
    pub fn to_display(self, unit: PowerUnit) -> f64 {
        let v = self.value();
        match unit {
            PowerUnit::Watt => v,
            PowerUnit::KiloWatt => v / 1_000.0,
            PowerUnit::MegaWatt => v / 1_000_000.0,
        }
    }

    /// Construct from a display-unit value, converting to canonical Watt.
    #[must_use]
    pub fn from_display(value: f64, unit: PowerUnit) -> Self {
        let w = match unit {
            PowerUnit::Watt => value,
            PowerUnit::KiloWatt => value * 1_000.0,
            PowerUnit::MegaWatt => value * 1_000_000.0,
        };
        Self::new_unchecked(w)
    }
}

// ---------------------------------------------------------------------------
// Speed
// ---------------------------------------------------------------------------

impl Speed {
    /// Convert canonical rad/s to a display unit.
    #[must_use]
    pub fn to_display(self, unit: SpeedUnit) -> f64 {
        let v = self.value();
        match unit {
            SpeedUnit::RadPerSec => v,
            SpeedUnit::Rpm => v * 60.0 / (2.0 * PI),
        }
    }

    /// Construct from a display-unit value, converting to canonical rad/s.
    #[must_use]
    pub fn from_display(value: f64, unit: SpeedUnit) -> Self {
        let rads = match unit {
            SpeedUnit::RadPerSec => value,
            SpeedUnit::Rpm => value * 2.0 * PI / 60.0,
        };
        Self::new_unchecked(rads)
    }
}

// ---------------------------------------------------------------------------
// VibrationVelocity
// ---------------------------------------------------------------------------

impl VibrationVelocity {
    /// Convert canonical m/s to a display unit.
    #[must_use]
    pub fn to_display(self, unit: VibrationUnit) -> f64 {
        let v = self.value();
        match unit {
            VibrationUnit::MPerSec => v,
            VibrationUnit::MmPerSec => v * 1_000.0,
            VibrationUnit::InchesPerSec => v / 0.0254,
        }
    }

    /// Construct from a display-unit value, converting to canonical m/s.
    #[must_use]
    pub fn from_display(value: f64, unit: VibrationUnit) -> Self {
        let ms = match unit {
            VibrationUnit::MPerSec => value,
            VibrationUnit::MmPerSec => value / 1_000.0,
            VibrationUnit::InchesPerSec => value * 0.0254,
        };
        Self::new_unchecked(ms)
    }
}

// ---------------------------------------------------------------------------
// UnitAwareValue — dynamic dispatch when quantity kind is not known at compile
// time (e.g., in generic tag display code).
// ---------------------------------------------------------------------------

/// A value paired with its quantity kind for runtime-generic handling.
///
/// The `canonical` field is always in SI.  Use this only where the quantity
/// kind is not statically known.  In typed physics code, use the concrete
/// quantity types directly.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UnitAwareValue {
    /// The value in canonical SI units.
    pub canonical: f64,
    /// The physical quantity this value represents.
    pub kind: QuantityKind,
}

impl UnitAwareValue {
    /// Construct from a canonical SI value and its quantity kind.
    #[must_use]
    pub fn new(canonical: f64, kind: QuantityKind) -> Self {
        Self { canonical, kind }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f64 = 1e-6;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < TOLERANCE
    }

    // ---- Pressure ----

    #[test]
    fn test_pressure_pascal_identity() {
        let p = Pressure::new_unchecked(101_325.0);
        assert!(approx_eq(p.to_display(PressureUnit::Pascal), 101_325.0));
    }

    #[test]
    fn test_pressure_bar_conversion() {
        let p = Pressure::new_unchecked(100_000.0);
        let bar = p.to_display(PressureUnit::Bar);
        assert!(approx_eq(bar, 1.0), "expected 1.0 bar, got {bar}");
    }

    #[test]
    fn test_pressure_psi_conversion() {
        let p = Pressure::new_unchecked(101_325.0);
        let psi = p.to_display(PressureUnit::Psi);
        assert!(
            (psi - 14.696).abs() < 0.001,
            "expected ~14.696 psi, got {psi}"
        );
    }

    #[test]
    fn test_round_trip_pressure_psi() {
        let original = Pressure::new_unchecked(500_000.0);
        let psi = original.to_display(PressureUnit::Psi);
        let back = Pressure::from_display(psi, PressureUnit::Psi);
        assert!(
            (back.value() - original.value()).abs() < 0.01,
            "round-trip error: {} vs {}",
            back.value(),
            original.value()
        );
    }

    // ---- Temperature ----

    #[test]
    fn test_temperature_celsius_conversion() {
        let t = Temperature::new_unchecked(373.15);
        let c = t.to_display(TemperatureUnit::Celsius);
        assert!(approx_eq(c, 100.0), "expected 100°C, got {c}");
    }

    #[test]
    fn test_temperature_fahrenheit_conversion() {
        let t = Temperature::new_unchecked(373.15);
        let f = t.to_display(TemperatureUnit::Fahrenheit);
        assert!(approx_eq(f, 212.0), "expected 212°F, got {f}");
    }

    #[test]
    fn test_round_trip_conversion() {
        let original = Temperature::new_unchecked(500.0);
        let celsius = original.to_display(TemperatureUnit::Celsius);
        let back = Temperature::from_display(celsius, TemperatureUnit::Celsius);
        assert!(
            (back.value() - original.value()).abs() < 1e-9,
            "round-trip error"
        );
    }

    // ---- Speed ----

    #[test]
    fn test_speed_rpm_to_rad_per_sec() {
        let speed = Speed::from_display(3000.0, SpeedUnit::Rpm);
        let expected = 3000.0 * 2.0 * PI / 60.0;
        assert!(
            (speed.value() - expected).abs() < 1e-6,
            "expected ~{expected} rad/s, got {}",
            speed.value()
        );
    }

    #[test]
    fn test_speed_round_trip_rpm() {
        let rpm_in = 1500.0;
        let speed = Speed::from_display(rpm_in, SpeedUnit::Rpm);
        let rpm_out = speed.to_display(SpeedUnit::Rpm);
        assert!((rpm_out - rpm_in).abs() < 1e-6, "RPM round-trip failed");
    }

    // ---- UnitAwareValue ----

    #[test]
    fn test_unit_aware_value_quantity_kind() {
        let v = UnitAwareValue::new(101_325.0, QuantityKind::Pressure);
        assert_eq!(v.kind, QuantityKind::Pressure);
        assert!((v.canonical - 101_325.0).abs() < 1e-10);
    }

    #[test]
    fn test_unit_aware_value_serde() {
        let v = UnitAwareValue::new(300.0, QuantityKind::Temperature);
        let json = serde_json::to_string(&v).unwrap();
        let decoded: UnitAwareValue = serde_json::from_str(&json).unwrap();
        assert_eq!(v, decoded);
    }

    #[test]
    fn test_canonical_feed_and_utility_parsing() {
        let density = 740.0; // Naphtha density kg/m3

        // 2000 BPSD
        let (mass_bpsd, vol_bpsd) = parse_flow(2000.0, "BPSD", density, None).unwrap();
        assert!((vol_bpsd.to_display(VolumetricFlowUnit::BarrelsPerStreamDay) - 2000.0).abs() < 1e-4);
        let m3_h = vol_bpsd.to_display(VolumetricFlowUnit::M3PerHour);
        assert!((m3_h - 13.2489).abs() < 0.01);
        assert!((mass_bpsd.value() - vol_bpsd.value() * density).abs() < 1e-6);

        // 199 m3/h
        let (mass_m3h, vol_m3h) = parse_flow(199.0, "m3/h", density, None).unwrap();
        assert!((vol_m3h.to_display(VolumetricFlowUnit::M3PerHour) - 199.0).abs() < 1e-6);
        assert!((mass_m3h.to_display(MassFlowUnit::KgPerHour) - (199.0 * 740.0)).abs() < 1e-4);

        // 3300 t/d
        let (mass_td, _vol_td) = parse_flow(3300.0, "t/d", density, None).unwrap();
        assert!((mass_td.to_display(MassFlowUnit::TonnesPerDay) - 3300.0).abs() < 1e-4);

        // 10000 kg/h
        let (mass_kgh, _vol_kgh) = parse_flow(10000.0, "kg/h", density, None).unwrap();
        assert!((mass_kgh.to_display(MassFlowUnit::KgPerHour) - 10000.0).abs() < 1e-4);

        // 5 barg
        let p = parse_pressure(5.0, "barg").unwrap();
        assert!((p.value() - 601_325.0).abs() < 1e-3);

        // 68 °C
        let t = parse_temperature(68.0, "°C").unwrap();
        assert!((t.to_display(TemperatureUnit::Celsius) - 68.0).abs() < 1e-6);
    }
}
