//! Pluggable property package implementation backed by cubic equations of state.
//!
//! Exposes [`CubicPropertyPackage`], fully implementing the [`PropertyPackage`] trait
//! while supporting rigorous multicomponent VLE flash calculations.

use serde::{Deserialize, Serialize};

use nebudcs_units::{Density, Pressure, SpecificEnthalpy, Temperature};

use crate::error::PhysicsError;
use crate::properties::{PhaseEquilibrium, PropertyPackage};
use crate::state::{ComponentComposition, CompositionBasis};
use crate::thermo::component::ComponentDatabase;
use crate::thermo::cubic::CubicEos;
use crate::thermo::flash::{FlashConfig, FlashResult, VleFlashSolver};

/// Pluggable thermodynamic property package backed by Peng-Robinson or SRK cubic EOS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CubicPropertyPackage {
    name: String,
    eos: CubicEos,
    flash_config: FlashConfig,
}

impl CubicPropertyPackage {
    /// Creates a new Peng-Robinson property package with the provided component database.
    pub fn new_peng_robinson(name: impl Into<String>, database: ComponentDatabase) -> Self {
        Self {
            name: name.into(),
            eos: CubicEos::new_peng_robinson(database),
            flash_config: FlashConfig::default(),
        }
    }

    /// Creates a new Soave-Redlich-Kwong property package with the provided component database.
    pub fn new_srk(name: impl Into<String>, database: ComponentDatabase) -> Self {
        Self {
            name: name.into(),
            eos: CubicEos::new_srk(database),
            flash_config: FlashConfig::default(),
        }
    }

    /// Creates a standard Peng-Robinson package with pre-populated industrial hydrocarbon library.
    pub fn standard_peng_robinson() -> Self {
        Self::new_peng_robinson(
            "Standard_PengRobinson",
            ComponentDatabase::standard_library(),
        )
    }

    /// Creates a standard SRK package with pre-populated industrial hydrocarbon library.
    pub fn standard_srk() -> Self {
        Self::new_srk("Standard_SRK", ComponentDatabase::standard_library())
    }

    /// Configures the flash solver parameters.
    pub fn with_flash_config(mut self, config: FlashConfig) -> Self {
        self.flash_config = config;
        self
    }

    /// Returns the underlying cubic EOS engine.
    pub fn eos(&self) -> &CubicEos {
        &self.eos
    }

    /// Performs a rigorous isothermal-isobaric multicomponent flash.
    pub fn flash(
        &self,
        p: Pressure,
        t: Temperature,
        comp: &ComponentComposition,
    ) -> Result<FlashResult, PhysicsError> {
        let mole_comp = match comp.basis() {
            CompositionBasis::Mole => comp.clone(),
            CompositionBasis::Mass => {
                let mut mw_map = std::collections::BTreeMap::new();
                for name in comp.fractions().keys() {
                    let c = self.eos.database().get(name).ok_or_else(|| {
                        PhysicsError::MissingProperty {
                            name: format!("component '{name}' in composition basis conversion"),
                        }
                    })?;
                    mw_map.insert(name.clone(), c.molecular_weight);
                }
                comp.to_mole_basis(&mw_map)?
            }
        };

        let solver = VleFlashSolver::with_config(&self.eos, self.flash_config);
        solver.flash_tp(mole_comp.fractions(), p, t)
    }
}

impl PropertyPackage for CubicPropertyPackage {
    fn name(&self) -> &str {
        &self.name
    }

    fn molecular_weight(&self, comp: &ComponentComposition) -> Result<f64, PhysicsError> {
        let mole_comp = match comp.basis() {
            CompositionBasis::Mole => comp.clone(),
            CompositionBasis::Mass => {
                let mut mw_map = std::collections::BTreeMap::new();
                for name in comp.fractions().keys() {
                    let c = self.eos.database().get(name).ok_or_else(|| {
                        PhysicsError::MissingProperty {
                            name: format!("component '{name}' in molecular_weight"),
                        }
                    })?;
                    mw_map.insert(name.clone(), c.molecular_weight);
                }
                comp.to_mole_basis(&mw_map)?
            }
        };

        self.eos.mixture_molecular_weight(mole_comp.fractions())
    }

    fn density(
        &self,
        p: Pressure,
        t: Temperature,
        comp: &ComponentComposition,
    ) -> Result<Density, PhysicsError> {
        let result = self.flash(p, t, comp)?;
        Ok(result.mixture_density)
    }

    fn specific_enthalpy(
        &self,
        p: Pressure,
        t: Temperature,
        comp: &ComponentComposition,
    ) -> Result<SpecificEnthalpy, PhysicsError> {
        let result = self.flash(p, t, comp)?;
        Ok(result.mixture_enthalpy)
    }

    fn capabilities(&self) -> crate::thermo::provenance::PropertyPackageCapabilities {
        crate::thermo::provenance::PropertyPackageCapabilities::cubic_eos()
    }

    fn validity_domain(&self) -> crate::thermo::provenance::ValidityDomain {
        crate::thermo::provenance::ValidityDomain {
            t_min_k: 50.0,
            t_max_k: 1500.0,
            p_min_pa: 100.0,
            p_max_pa: 100.0e6,
            extrapolation_margin: 0.15,
        }
    }

    fn phase_equilibrium(
        &self,
        p: Pressure,
        t: Temperature,
        comp: &ComponentComposition,
    ) -> Result<PhaseEquilibrium, PhysicsError> {
        let result = self.flash(p, t, comp)?;
        Ok(PhaseEquilibrium {
            vapor_fraction: result.vapor_fraction,
            phase: result.phase,
        })
    }
}
