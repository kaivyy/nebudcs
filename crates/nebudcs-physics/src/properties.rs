//! Pluggable thermodynamic property packages and phase equilibrium models.
//!
//! Provides the generic [`PropertyPackage`] trait and reference implementations:
//! - [`IdealGasMixture`]: Ideal gas law with component molecular weights and heat capacities.
//! - [`IncompressibleLiquid`]: Constant or mildly temperature-dependent liquid density and heat capacity.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use nebudcs_units::{Density, Pressure, SpecificEnthalpy, Temperature};

use crate::error::PhysicsError;
use crate::state::{ComponentComposition, Phase, PhysicalState};

/// Universal gas constant in $\text{J}/(\text{mol}\cdot\text{K})$.
pub const UNIVERSAL_GAS_CONSTANT: f64 = 8.314_462_618_153_24;

/// Standard reference temperature: 298.15 K (25 °C).
pub const STANDARD_TEMPERATURE_K: f64 = 298.15;

/// Standard reference pressure: 101,325 Pa (1 atm).
pub const STANDARD_PRESSURE_PA: f64 = 101_325.0;

/// Result of a vapor-liquid equilibrium flash calculation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhaseEquilibrium {
    pub vapor_fraction: f64,
    pub phase: Phase,
}

/// Pluggable thermodynamic property package trait.
///
/// Enables seamless replacement of thermodynamic calculation routines
/// (e.g. from Ideal Gas to Peng-Robinson, SRK, or CoolProp) without modifying
/// plant topology or equipment logic.
pub trait PropertyPackage: Send + Sync {
    /// Human-readable package identifier.
    fn name(&self) -> &str;

    /// Calculates mixture average molecular weight in kg/mol.
    fn molecular_weight(&self, comp: &ComponentComposition) -> Result<f64, PhysicsError>;

    /// Calculates fluid density in kg/m³.
    fn density(
        &self,
        p: Pressure,
        t: Temperature,
        comp: &ComponentComposition,
    ) -> Result<Density, PhysicsError>;

    /// Calculates fluid specific enthalpy in J/kg relative to standard reference state.
    fn specific_enthalpy(
        &self,
        p: Pressure,
        t: Temperature,
        comp: &ComponentComposition,
    ) -> Result<SpecificEnthalpy, PhysicsError>;

    /// Evaluates phase equilibrium at the specified temperature, pressure, and composition.
    fn phase_equilibrium(
        &self,
        p: Pressure,
        t: Temperature,
        comp: &ComponentComposition,
    ) -> Result<PhaseEquilibrium, PhysicsError>;

    /// Returns the thermodynamic capabilities supported by this package.
    fn capabilities(&self) -> crate::thermo::provenance::PropertyPackageCapabilities {
        crate::thermo::provenance::PropertyPackageCapabilities::default()
    }

    /// Returns the permissible thermodynamic operating domain.
    fn validity_domain(&self) -> crate::thermo::provenance::ValidityDomain {
        crate::thermo::provenance::ValidityDomain::default()
    }

    /// Verifies operating coordinates against the package validity domain.
    fn check_validity(
        &self,
        p: Pressure,
        t: Temperature,
        _comp: &ComponentComposition,
    ) -> crate::thermo::provenance::ValidityState {
        self.validity_domain().check(t.value(), p.value())
    }

    /// Evaluates the complete [`PhysicalState`] for the specified thermodynamic coordinates.
    fn evaluate_state(
        &self,
        p: Pressure,
        t: Temperature,
        comp: &ComponentComposition,
    ) -> Result<PhysicalState, PhysicsError> {
        let mw = self.molecular_weight(comp)?;
        let rho = self.density(p, t, comp)?;
        let h = self.specific_enthalpy(p, t, comp)?;
        let eq = self.phase_equilibrium(p, t, comp)?;

        PhysicalState::new(p, t, rho, h, mw, eq.vapor_fraction, eq.phase, comp.clone())
    }
}

/// Pure component properties for ideal gas calculations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GasComponentProperty {
    /// Molecular weight in kg/mol.
    pub molecular_weight: f64,
    /// Isobaric specific heat capacity $C_p$ in J/(kg·K).
    pub heat_capacity: f64,
}

/// Ideal gas mixture property package.
///
/// Assumes:
/// - $P \cdot M_w = \rho \cdot R \cdot T$
/// - $h = \sum w_i C_{p,i} (T - T_{ref})$
/// - Vapor fraction $q = 1.0$, Phase = `Phase::Vapor`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IdealGasMixture {
    name: String,
    components: BTreeMap<String, GasComponentProperty>,
}

impl IdealGasMixture {
    /// Creates a new ideal gas mixture package.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            components: BTreeMap::new(),
        }
    }

    /// Registers pure component properties (molecular weight in kg/mol, $C_p$ in J/(kg·K)).
    pub fn register_component(
        &mut self,
        name: impl Into<String>,
        molecular_weight: f64,
        heat_capacity: f64,
    ) {
        self.components.insert(
            name.into(),
            GasComponentProperty {
                molecular_weight,
                heat_capacity,
            },
        );
    }
}

impl PropertyPackage for IdealGasMixture {
    fn name(&self) -> &str {
        &self.name
    }

    fn molecular_weight(&self, comp: &ComponentComposition) -> Result<f64, PhysicsError> {
        let mw_map: BTreeMap<String, f64> = self
            .components
            .iter()
            .map(|(k, v)| (k.clone(), v.molecular_weight))
            .collect();
        comp.average_molecular_weight(&mw_map)
    }

    fn density(
        &self,
        p: Pressure,
        t: Temperature,
        comp: &ComponentComposition,
    ) -> Result<Density, PhysicsError> {
        let p_val = p.value();
        if p_val <= 0.0 {
            return Err(PhysicsError::NegativePressure {
                value: p_val,
                context: "IdealGasMixture density".to_string(),
            });
        }

        let t_val = t.value();
        if t_val <= 0.0 {
            return Err(PhysicsError::NonPositiveTemperature {
                value: t_val,
                context: "IdealGasMixture density".to_string(),
            });
        }

        let mw = self.molecular_weight(comp)?;
        // rho = P * Mw / (R * T)
        let rho_val = (p_val * mw) / (UNIVERSAL_GAS_CONSTANT * t_val);

        Density::new(rho_val).map_err(|_| PhysicsError::InvalidDensity {
            value: rho_val,
            context: "IdealGasMixture calculated non-finite density".to_string(),
        })
    }

    fn specific_enthalpy(
        &self,
        _p: Pressure,
        t: Temperature,
        comp: &ComponentComposition,
    ) -> Result<SpecificEnthalpy, PhysicsError> {
        let t_val = t.value();
        let delta_t = t_val - STANDARD_TEMPERATURE_K;

        let mw_map: BTreeMap<String, f64> = self
            .components
            .iter()
            .map(|(k, v)| (k.clone(), v.molecular_weight))
            .collect();

        let mass_comp = comp.to_mass_basis(&mw_map)?;
        let mut avg_cp = 0.0;
        for (name, &w_i) in mass_comp.fractions() {
            let prop = self
                .components
                .get(name)
                .ok_or_else(|| PhysicsError::MissingProperty {
                    name: format!("component {name} not registered in IdealGasMixture"),
                })?;
            avg_cp += w_i * prop.heat_capacity;
        }

        let h_val = avg_cp * delta_t;
        SpecificEnthalpy::new(h_val).map_err(|_| PhysicsError::NonFiniteValue {
            field: "specific_enthalpy".to_string(),
            value: h_val,
        })
    }

    fn phase_equilibrium(
        &self,
        _p: Pressure,
        _t: Temperature,
        _comp: &ComponentComposition,
    ) -> Result<PhaseEquilibrium, PhysicsError> {
        // IdealGasMixture explicitly assumes single-phase vapor behavior above dew point.
        Ok(PhaseEquilibrium {
            vapor_fraction: 1.0,
            phase: Phase::Vapor,
        })
    }
}

/// Incompressible liquid property package.
///
/// Suitable for liquid water, crude oil, diesel, or generic non-volatile liquids:
/// - $\rho(T) = \rho_0 \cdot [1 - \beta (T - T_{ref})]$
/// - $h = C_p (T - T_{ref}) + \frac{P - P_{ref}}{\rho}$
/// - Vapor fraction $q = 0.0$, Phase = `Phase::Liquid`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IncompressibleLiquid {
    name: String,
    reference_density: f64, // kg/m³
    heat_capacity: f64,     // J/(kg·K)
    molecular_weight: f64,  // kg/mol
    thermal_expansion: f64, // 1/K (default 0.0)
}

impl IncompressibleLiquid {
    /// Creates an incompressible liquid model.
    pub fn new(
        name: impl Into<String>,
        reference_density: f64,
        heat_capacity: f64,
        molecular_weight: f64,
    ) -> Self {
        Self {
            name: name.into(),
            reference_density,
            heat_capacity,
            molecular_weight,
            thermal_expansion: 0.0,
        }
    }

    /// Sets the volumetric thermal expansion coefficient $\beta$ (1/K).
    pub fn with_thermal_expansion(mut self, beta: f64) -> Self {
        self.thermal_expansion = beta;
        self
    }
}

impl PropertyPackage for IncompressibleLiquid {
    fn name(&self) -> &str {
        &self.name
    }

    fn molecular_weight(&self, _comp: &ComponentComposition) -> Result<f64, PhysicsError> {
        Ok(self.molecular_weight)
    }

    fn density(
        &self,
        _p: Pressure,
        t: Temperature,
        _comp: &ComponentComposition,
    ) -> Result<Density, PhysicsError> {
        let t_val = t.value();
        let delta_t = t_val - STANDARD_TEMPERATURE_K;
        let rho_val = self.reference_density * (1.0 - self.thermal_expansion * delta_t);

        if rho_val <= 0.0 || !rho_val.is_finite() {
            return Err(PhysicsError::InvalidDensity {
                value: rho_val,
                context: "IncompressibleLiquid thermal expansion density".to_string(),
            });
        }

        Density::new(rho_val).map_err(|_| PhysicsError::InvalidDensity {
            value: rho_val,
            context: "IncompressibleLiquid density".to_string(),
        })
    }

    fn specific_enthalpy(
        &self,
        p: Pressure,
        t: Temperature,
        comp: &ComponentComposition,
    ) -> Result<SpecificEnthalpy, PhysicsError> {
        let t_val = t.value();
        let delta_t = t_val - STANDARD_TEMPERATURE_K;
        let rho = self.density(p, t, comp)?.value();

        let delta_p = p.value() - STANDARD_PRESSURE_PA;
        let h_val = self.heat_capacity * delta_t + (delta_p / rho);

        SpecificEnthalpy::new(h_val).map_err(|_| PhysicsError::NonFiniteValue {
            field: "specific_enthalpy".to_string(),
            value: h_val,
        })
    }

    fn phase_equilibrium(
        &self,
        _p: Pressure,
        _t: Temperature,
        _comp: &ComponentComposition,
    ) -> Result<PhaseEquilibrium, PhysicsError> {
        // IncompressibleLiquid explicitly assumes subcooled liquid phase below bubble point.
        Ok(PhaseEquilibrium {
            vapor_fraction: 0.0,
            phase: Phase::Liquid,
        })
    }
}

/// Thermodynamic property package for a fluid exhibiting vapor-liquid equilibrium (VLE).
///
/// Models:
/// - Subcooled liquid below saturation temperature ($q = 0.0$, [`Phase::Liquid`])
/// - Saturated two-phase mixture ($0.0 < q < 1.0$, [`Phase::TwoPhase`])
/// - Superheated vapor above saturation temperature ($q = 1.0$, [`Phase::Vapor`])
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaturatedFluidPackage {
    name: String,
    component_name: String,
    molecular_weight: f64,      // kg/mol
    liquid_density: f64,        // kg/m³
    liquid_cp: f64,             // J/(kg·K)
    vapor_cp: f64,              // J/(kg·K)
    heat_of_vaporization: f64,  // J/kg
    normal_boiling_temp_k: f64, // K at 101,325 Pa
}

impl SaturatedFluidPackage {
    /// Creates a new saturated fluid package.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: impl Into<String>,
        component_name: impl Into<String>,
        molecular_weight: f64,
        liquid_density: f64,
        liquid_cp: f64,
        vapor_cp: f64,
        heat_of_vaporization: f64,
        normal_boiling_temp_k: f64,
    ) -> Self {
        Self {
            name: name.into(),
            component_name: component_name.into(),
            molecular_weight,
            liquid_density,
            liquid_cp,
            vapor_cp,
            heat_of_vaporization,
            normal_boiling_temp_k,
        }
    }

    /// Convenience constructor for saturated water/steam.
    pub fn new_water() -> Self {
        Self {
            name: "SaturatedWaterSteam".to_string(),
            component_name: "H2O".to_string(),
            molecular_weight: 0.018015,
            liquid_density: 998.0,
            liquid_cp: 4184.0,
            vapor_cp: 2080.0,
            heat_of_vaporization: 2_257_000.0,
            normal_boiling_temp_k: 373.15,
        }
    }

    /// Calculates saturation pressure in Pa using Clausius-Clapeyron equation:
    ///
    /// $\ln(P_{sat} / P_{ref}) = \frac{\Delta H_{vap} \cdot M_w}{R} \left( \frac{1}{T_b} - \frac{1}{T} \right)$
    pub fn saturation_pressure(&self, t: Temperature) -> f64 {
        let t_val = t.value();
        let exponent = (self.heat_of_vaporization * self.molecular_weight / UNIVERSAL_GAS_CONSTANT)
            * (1.0 / self.normal_boiling_temp_k - 1.0 / t_val);
        STANDARD_PRESSURE_PA * exponent.exp()
    }

    /// Calculates saturation temperature in Kelvin at specified pressure using Clausius-Clapeyron equation:
    ///
    /// $T_{sat}(P) = \left[ \frac{1}{T_b} - \frac{R \cdot \ln(P / P_{ref})}{\Delta H_{vap} \cdot M_w} \right]^{-1}$
    pub fn saturation_temperature(&self, p: Pressure) -> Result<f64, PhysicsError> {
        let p_val = p.value();
        if p_val <= 0.0 {
            return Err(PhysicsError::NegativePressure {
                value: p_val,
                context: "saturation_temperature calculation".to_string(),
            });
        }

        let r_delta_h = (UNIVERSAL_GAS_CONSTANT * (p_val / STANDARD_PRESSURE_PA).ln())
            / (self.heat_of_vaporization * self.molecular_weight);
        let inv_t = (1.0 / self.normal_boiling_temp_k) - r_delta_h;

        if inv_t <= 0.0 || !inv_t.is_finite() {
            return Err(PhysicsError::InvalidOperatingCondition {
                message: format!("pressure {p_val} Pa exceeds valid Clausius-Clapeyron domain"),
            });
        }

        Ok(1.0 / inv_t)
    }

    /// Evaluates saturated vapor density using ideal gas equation at (P, T).
    pub fn vapor_density(&self, p: Pressure, t: Temperature) -> f64 {
        (p.value() * self.molecular_weight) / (UNIVERSAL_GAS_CONSTANT * t.value())
    }

    /// Flashes fluid at specified (P, T) with explicit vapor fraction $q \in [0.0, 1.0]$.
    pub fn flash_at_enthalpy(
        &self,
        p: Pressure,
        t: Temperature,
        comp: &ComponentComposition,
        vapor_fraction: f64,
    ) -> Result<PhysicalState, PhysicsError> {
        let q = vapor_fraction.clamp(0.0, 1.0);
        let phase = if q <= 1e-4 {
            Phase::Liquid
        } else if q >= 1.0 - 1e-4 {
            Phase::Vapor
        } else {
            Phase::TwoPhase
        };

        let rho_l = self.liquid_density;
        let rho_v = self.vapor_density(p, t);
        let rho_mix = if phase == Phase::Liquid {
            rho_l
        } else if phase == Phase::Vapor {
            rho_v
        } else {
            1.0 / (q / rho_v + (1.0 - q) / rho_l)
        };

        let delta_t = t.value() - STANDARD_TEMPERATURE_K;
        let h_l = self.liquid_cp * delta_t;
        let h_mix = h_l + q * self.heat_of_vaporization;

        PhysicalState::new(
            p,
            t,
            Density::new(rho_mix).map_err(|_| PhysicsError::InvalidDensity {
                value: rho_mix,
                context: "SaturatedFluidPackage density".to_string(),
            })?,
            SpecificEnthalpy::new(h_mix).map_err(|_| PhysicsError::NonFiniteValue {
                field: "specific_enthalpy".to_string(),
                value: h_mix,
            })?,
            self.molecular_weight,
            q,
            phase,
            comp.clone(),
        )
    }

    /// Performs a rigorous $P-h$ flash calculation to determine temperature, phase, vapor fraction,
    /// and density from pressure and specific enthalpy.
    pub fn flash_from_enthalpy(
        &self,
        p: Pressure,
        h: SpecificEnthalpy,
        comp: &ComponentComposition,
    ) -> Result<PhysicalState, PhysicsError> {
        let t_sat_val = self.saturation_temperature(p)?;
        let delta_t_sat = t_sat_val - STANDARD_TEMPERATURE_K;

        let h_l_sat = self.liquid_cp * delta_t_sat;
        let h_v_sat = h_l_sat + self.heat_of_vaporization;
        let h_val = h.value();

        let (t_val, q, phase) = if h_val <= h_l_sat {
            // Subcooled liquid
            let t = STANDARD_TEMPERATURE_K + (h_val / self.liquid_cp);
            (t, 0.0, Phase::Liquid)
        } else if h_val >= h_v_sat {
            // Superheated vapor
            let t = t_sat_val + ((h_val - h_v_sat) / self.vapor_cp);
            (t, 1.0, Phase::Vapor)
        } else {
            // Saturated two-phase mixture
            let q = (h_val - h_l_sat) / self.heat_of_vaporization;
            (t_sat_val, q.clamp(0.0, 1.0), Phase::TwoPhase)
        };

        let t = Temperature::new(t_val).map_err(|e| PhysicsError::NonPositiveTemperature {
            value: t_val,
            context: format!("flash_from_enthalpy: {e}"),
        })?;

        let rho_l = self.liquid_density;
        let rho_v = self.vapor_density(p, t);
        let rho_mix = if phase == Phase::Liquid {
            rho_l
        } else if phase == Phase::Vapor {
            rho_v
        } else {
            1.0 / (q / rho_v + (1.0 - q) / rho_l)
        };

        PhysicalState::new(
            p,
            t,
            Density::new(rho_mix).map_err(|_| PhysicsError::InvalidDensity {
                value: rho_mix,
                context: "flash_from_enthalpy density".to_string(),
            })?,
            h,
            self.molecular_weight,
            q,
            phase,
            comp.clone(),
        )
    }
}

impl PropertyPackage for SaturatedFluidPackage {
    fn name(&self) -> &str {
        &self.name
    }

    fn molecular_weight(&self, _comp: &ComponentComposition) -> Result<f64, PhysicsError> {
        Ok(self.molecular_weight)
    }

    fn density(
        &self,
        p: Pressure,
        t: Temperature,
        comp: &ComponentComposition,
    ) -> Result<Density, PhysicsError> {
        let eq = self.phase_equilibrium(p, t, comp)?;
        let rho_l = self.liquid_density;
        let rho_v = self.vapor_density(p, t);

        let rho_val = match eq.phase {
            Phase::Liquid => rho_l,
            Phase::Vapor => rho_v,
            Phase::TwoPhase => {
                1.0 / (eq.vapor_fraction / rho_v + (1.0 - eq.vapor_fraction) / rho_l)
            }
            Phase::Supercritical => 0.5 * (rho_l + rho_v),
        };

        Density::new(rho_val).map_err(|_| PhysicsError::InvalidDensity {
            value: rho_val,
            context: "SaturatedFluidPackage density".to_string(),
        })
    }

    fn specific_enthalpy(
        &self,
        p: Pressure,
        t: Temperature,
        comp: &ComponentComposition,
    ) -> Result<SpecificEnthalpy, PhysicsError> {
        let eq = self.phase_equilibrium(p, t, comp)?;
        let delta_t = t.value() - STANDARD_TEMPERATURE_K;
        let h_l = self.liquid_cp * delta_t;
        let h_val = match eq.phase {
            Phase::Liquid => h_l,
            Phase::Vapor => {
                h_l + self.heat_of_vaporization
                    + self.vapor_cp * (t.value() - self.normal_boiling_temp_k).max(0.0)
            }
            Phase::TwoPhase => h_l + eq.vapor_fraction * self.heat_of_vaporization,
            Phase::Supercritical => h_l + self.heat_of_vaporization,
        };

        SpecificEnthalpy::new(h_val).map_err(|_| PhysicsError::NonFiniteValue {
            field: "specific_enthalpy".to_string(),
            value: h_val,
        })
    }

    fn phase_equilibrium(
        &self,
        p: Pressure,
        t: Temperature,
        _comp: &ComponentComposition,
    ) -> Result<PhaseEquilibrium, PhysicsError> {
        let p_sat = self.saturation_pressure(t);
        let p_val = p.value();

        if p_val > p_sat * 1.01 {
            Ok(PhaseEquilibrium {
                vapor_fraction: 0.0,
                phase: Phase::Liquid,
            })
        } else if p_val < p_sat * 0.99 {
            Ok(PhaseEquilibrium {
                vapor_fraction: 1.0,
                phase: Phase::Vapor,
            })
        } else {
            // Near saturation line: default 50% vapor unless flashed with specific enthalpy
            Ok(PhaseEquilibrium {
                vapor_fraction: 0.5,
                phase: Phase::TwoPhase,
            })
        }
    }
}
