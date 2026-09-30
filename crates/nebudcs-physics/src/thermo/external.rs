//! Standardized interface and adapter for external thermophysical property providers.
//!
//! Provides the [`ExternalPropertyProvider`] trait and [`CoolPropAdapter`] for
//! integrating external property providers (e.g. CoolProp, REFPROP, or native adapters)
//! into the simulation engine.
//!
//! # Scientific Fidelity Notice
//! The included native [`CoolPropAdapter`] implements simplified engineering correlations
//! (Tait-like liquid water and real-gas corrected steam). It is **not** a full five-region
//! IAPWS-IF97 formulation. Full multi-region IAPWS-IF97 or CoolProp HEOS calculations require
//! linking the external CoolProp C++ shared library via FFI.

use serde::{Deserialize, Serialize};

use nebudcs_units::{Density, Pressure, SpecificEnthalpy, Temperature};

use crate::error::PhysicsError;
use crate::properties::{PhaseEquilibrium, PropertyPackage};
use crate::state::{ComponentComposition, Phase};

/// Standardized interface for external thermophysical property libraries (e.g. CoolProp, REFPROP, IAPWS).
pub trait ExternalPropertyProvider: std::fmt::Debug + Send + Sync {
    /// Provider name and version string.
    fn provider_name(&self) -> &str;

    /// Evaluates mass density in kg/m³ for a pure fluid at $(P, T)$.
    fn pure_fluid_density(&self, fluid: &str, p_pa: f64, t_k: f64) -> Result<f64, PhysicsError>;

    /// Evaluates specific enthalpy in J/kg for a pure fluid at $(P, T)$.
    fn pure_fluid_enthalpy(&self, fluid: &str, p_pa: f64, t_k: f64) -> Result<f64, PhysicsError>;

    /// Evaluates saturation pressure in Pa at temperature $T$ (K).
    fn saturation_pressure(&self, fluid: &str, t_k: f64) -> Result<f64, PhysicsError>;
}

/// Native reference implementation of [`ExternalPropertyProvider`] supporting water/steam
/// engineering approximations (Tait liquid, real-gas steam) and pure industrial gases.
///
/// **IMPORTANT SCIENTIFIC NOTICE**: This native adapter is an architectural reference bridge.
/// It does **not** implement the complete 5-region IAPWS-IF97 formulation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoolPropAdapter {
    name: String,
    backend: String,
}

impl Default for CoolPropAdapter {
    fn default() -> Self {
        Self::new("CoolProp_Native_Simplified_Adapter", "NativeSimplified")
    }
}

impl CoolPropAdapter {
    /// Creates a new external property adapter.
    pub fn new(name: impl Into<String>, backend: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            backend: backend.into(),
        }
    }

    /// Backend identifier (e.g. `"NativeSimplified"`, `"HEOS"`).
    pub fn backend(&self) -> &str {
        &self.backend
    }

    /// Returns `false` because this native adapter is a simplified approximation,
    /// not a full multi-region IAPWS-IF97 implementation.
    pub fn is_full_iapws_if97(&self) -> bool {
        false
    }
}

impl ExternalPropertyProvider for CoolPropAdapter {
    fn provider_name(&self) -> &str {
        &self.name
    }

    fn pure_fluid_density(&self, fluid: &str, p_pa: f64, t_k: f64) -> Result<f64, PhysicsError> {
        match fluid {
            "Water" | "H2O" => {
                // IAPWS simplified region: subcooled liquid or superheated steam
                let psat = self.saturation_pressure("Water", t_k)?;
                if p_pa >= psat {
                    // Liquid: Tait-like equation
                    let rho0 = 1000.0 * (1.0 - (t_k - 277.15).powi(2) / (500.0 * (t_k + 67.0)));
                    let beta_comp = 4.5e-10; // 1/Pa compressibility
                    let rho = rho0 * (1.0 + beta_comp * (p_pa - psat).max(0.0));
                    Ok(rho.max(10.0))
                } else {
                    // Steam vapor: real gas correction
                    let r_steam = 461.52; // J/(kg*K)
                    let z = (1.0 - 0.05 * (p_pa / 1e6) / (t_k / 373.15)).max(0.5);
                    Ok(p_pa / (z * r_steam * t_k))
                }
            }
            "Nitrogen" | "N2" => {
                let r_n2 = 296.8; // J/(kg*K)
                Ok(p_pa / (r_n2 * t_k))
            }
            "Methane" | "CH4" => {
                let r_ch4 = 518.28; // J/(kg*K)
                Ok(p_pa / (r_ch4 * t_k))
            }
            _ => Err(PhysicsError::MissingProperty {
                name: format!("fluid '{fluid}' not supported in native external adapter"),
            }),
        }
    }

    fn pure_fluid_enthalpy(&self, fluid: &str, p_pa: f64, t_k: f64) -> Result<f64, PhysicsError> {
        match fluid {
            "Water" | "H2O" => {
                let psat = self.saturation_pressure("Water", t_k)?;
                let cp_l = 4184.0;
                let h_vap = 2_257_000.0;
                let cp_v = 2080.0;

                if p_pa >= psat {
                    Ok(cp_l * (t_k - 298.15) + (p_pa - 101_325.0) / 1000.0)
                } else {
                    Ok(cp_l * (373.15 - 298.15) + h_vap + cp_v * (t_k - 373.15))
                }
            }
            _ => {
                let cp = 1000.0;
                Ok(cp * (t_k - 298.15))
            }
        }
    }

    fn saturation_pressure(&self, fluid: &str, t_k: f64) -> Result<f64, PhysicsError> {
        match fluid {
            "Water" | "H2O" => {
                // Antoine correlation for water (valid 273 K to 647 K)
                // log10(P_bar) = A - B / (T_C + C)
                let t_c = t_k - 273.15;
                let p_bar = if t_c <= 100.0 {
                    10.0_f64.powf(8.07131 - 1730.63 / (t_c + 233.426)) / 750.062
                // mmHg to bar
                } else {
                    10.0_f64.powf(8.14019 - 1810.94 / (t_c + 244.485)) / 750.062
                };
                Ok(p_bar * 100_000.0)
            }
            _ => Err(PhysicsError::MissingProperty {
                name: format!("fluid '{fluid}' saturation not supported"),
            }),
        }
    }
}

/// Pluggable [`PropertyPackage`] wrapper over an [`ExternalPropertyProvider`].
#[derive(Debug)]
pub struct ExternalPropertyPackage {
    name: String,
    fluid_name: String,
    molecular_weight: f64,
    provider: Box<dyn ExternalPropertyProvider>,
}

impl ExternalPropertyPackage {
    /// Creates an external property package for a pure fluid.
    pub fn new(
        name: impl Into<String>,
        fluid_name: impl Into<String>,
        molecular_weight: f64,
        provider: Box<dyn ExternalPropertyProvider>,
    ) -> Self {
        Self {
            name: name.into(),
            fluid_name: fluid_name.into(),
            molecular_weight,
            provider,
        }
    }
}

impl PropertyPackage for ExternalPropertyPackage {
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
        _comp: &ComponentComposition,
    ) -> Result<Density, PhysicsError> {
        let val = self
            .provider
            .pure_fluid_density(&self.fluid_name, p.value(), t.value())?;
        Density::new(val).map_err(|_| PhysicsError::InvalidDensity {
            value: val,
            context: format!("{}: external provider density", self.name),
        })
    }

    fn specific_enthalpy(
        &self,
        p: Pressure,
        t: Temperature,
        _comp: &ComponentComposition,
    ) -> Result<SpecificEnthalpy, PhysicsError> {
        let val = self
            .provider
            .pure_fluid_enthalpy(&self.fluid_name, p.value(), t.value())?;
        SpecificEnthalpy::new(val).map_err(|_| PhysicsError::NonFiniteValue {
            field: "specific_enthalpy".to_string(),
            value: val,
        })
    }

    fn phase_equilibrium(
        &self,
        p: Pressure,
        t: Temperature,
        _comp: &ComponentComposition,
    ) -> Result<PhaseEquilibrium, PhysicsError> {
        let psat = self
            .provider
            .saturation_pressure(&self.fluid_name, t.value())?;
        let p_val = p.value();

        if p_val > psat * 1.001 {
            Ok(PhaseEquilibrium {
                vapor_fraction: 0.0,
                phase: Phase::Liquid,
            })
        } else if p_val < psat * 0.999 {
            Ok(PhaseEquilibrium {
                vapor_fraction: 1.0,
                phase: Phase::Vapor,
            })
        } else {
            Ok(PhaseEquilibrium {
                vapor_fraction: 0.5,
                phase: Phase::TwoPhase,
            })
        }
    }
}
