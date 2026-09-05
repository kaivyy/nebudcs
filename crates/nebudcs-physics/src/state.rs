//! Physical state models for streams, ports, and equipment holdup.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use nebudcs_types::StreamId;
use nebudcs_units::{Density, MassFlow, Pressure, SpecificEnthalpy, Temperature, VolumetricFlow};

use crate::error::PhysicsError;

/// Physical thermodynamic phase of a stream or holdup volume.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Pure liquid phase ($q = 0$).
    Liquid,
    /// Pure vapor/gas phase ($q = 1$).
    Vapor,
    /// Coexisting liquid and vapor ($0 < q < 1$).
    TwoPhase,
    /// Above critical temperature and pressure.
    Supercritical,
}

/// Physical basis of component fractions in a mixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompositionBasis {
    /// Molar basis (mole fractions $y_i$, strictly satisfying $\sum y_i = 1.0$).
    Mole,
    /// Mass basis (mass fractions $w_i$, strictly satisfying $\sum w_i = 1.0$).
    Mass,
}

/// Normalized chemical composition of a mixture.
///
/// Keys are component formulas or names (e.g. `"H2"`, `"CH4"`, `"N2"`).
/// Values are normalized mass or mole fractions strictly satisfying $\sum f_i = 1.0$.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentComposition {
    fractions: IndexMap<String, f64>,
    basis: CompositionBasis,
}

impl ComponentComposition {
    /// Constructs a normalized composition from component fractions with default `Mole` basis.
    ///
    /// Fractions are normalized to sum to 1.0. All fractions must be non-negative,
    /// finite, and their sum must be strictly positive.
    pub fn new(raw_fractions: IndexMap<String, f64>) -> Result<Self, PhysicsError> {
        Self::with_basis(raw_fractions, CompositionBasis::Mole)
    }

    /// Constructs a normalized composition with an explicit basis (`Mole` or `Mass`).
    pub fn with_basis(
        raw_fractions: IndexMap<String, f64>,
        basis: CompositionBasis,
    ) -> Result<Self, PhysicsError> {
        if raw_fractions.is_empty() {
            return Err(PhysicsError::InvalidComposition {
                reason: "composition cannot be empty".to_string(),
            });
        }

        let mut sum = 0.0;
        for (name, &val) in &raw_fractions {
            if !val.is_finite() {
                return Err(PhysicsError::NonFiniteValue {
                    field: format!("fraction_{name}"),
                    value: val,
                });
            }
            if val < 0.0 {
                return Err(PhysicsError::InvalidComposition {
                    reason: format!("component {name} has negative fraction {val}"),
                });
            }
            sum += val;
        }

        if sum <= 0.0 {
            return Err(PhysicsError::InvalidComposition {
                reason: "total composition sum must be strictly positive".to_string(),
            });
        }

        let mut fractions = IndexMap::with_capacity(raw_fractions.len());
        for (name, val) in raw_fractions {
            fractions.insert(name, val / sum);
        }

        Ok(Self { fractions, basis })
    }

    /// Creates a 100% pure component composition (default Mole basis).
    pub fn pure(name: impl Into<String>) -> Self {
        Self::pure_with_basis(name, CompositionBasis::Mole)
    }

    /// Creates a 100% pure component composition with explicit basis.
    pub fn pure_with_basis(name: impl Into<String>, basis: CompositionBasis) -> Self {
        let mut map = IndexMap::new();
        map.insert(name.into(), 1.0);
        Self {
            fractions: map,
            basis,
        }
    }

    /// Returns the fraction basis (`Mole` or `Mass`).
    pub fn basis(&self) -> CompositionBasis {
        self.basis
    }

    /// Returns the fraction of a specific component, or 0.0 if not present.
    pub fn fraction(&self, component: &str) -> f64 {
        self.fractions.get(component).copied().unwrap_or(0.0)
    }

    /// Returns a reference to all component fractions.
    pub fn fractions(&self) -> &IndexMap<String, f64> {
        &self.fractions
    }

    /// Calculates mixture average molecular weight in kg/mol given component molecular weights.
    pub fn average_molecular_weight(
        &self,
        mw_map: &std::collections::BTreeMap<String, f64>,
    ) -> Result<f64, PhysicsError> {
        match self.basis {
            CompositionBasis::Mole => {
                let mut avg_mw = 0.0;
                for (name, &y_i) in &self.fractions {
                    let mw_i = mw_map
                        .get(name)
                        .ok_or_else(|| PhysicsError::MissingProperty {
                            name: format!("molecular weight for component {name}"),
                        })?;
                    if *mw_i <= 0.0 || !mw_i.is_finite() {
                        return Err(PhysicsError::NonFiniteValue {
                            field: format!("molecular_weight_{name}"),
                            value: *mw_i,
                        });
                    }
                    avg_mw += y_i * mw_i;
                }
                Ok(avg_mw)
            }
            CompositionBasis::Mass => {
                let mut inv_mw = 0.0;
                for (name, &w_i) in &self.fractions {
                    let mw_i = mw_map
                        .get(name)
                        .ok_or_else(|| PhysicsError::MissingProperty {
                            name: format!("molecular weight for component {name}"),
                        })?;
                    if *mw_i <= 0.0 || !mw_i.is_finite() {
                        return Err(PhysicsError::NonFiniteValue {
                            field: format!("molecular_weight_{name}"),
                            value: *mw_i,
                        });
                    }
                    inv_mw += w_i / mw_i;
                }
                if inv_mw <= 0.0 {
                    return Err(PhysicsError::InvalidComposition {
                        reason: "invalid reciprocal molecular weight sum".to_string(),
                    });
                }
                Ok(1.0 / inv_mw)
            }
        }
    }

    /// Converts this composition to a mass fraction basis.
    ///
    /// If already on a mass basis, returns a clone.
    /// If on a mole basis, converts using $w_i = \frac{y_i M_{w,i}}{\sum y_k M_{w,k}}$.
    pub fn to_mass_basis(
        &self,
        mw_map: &std::collections::BTreeMap<String, f64>,
    ) -> Result<Self, PhysicsError> {
        if self.basis == CompositionBasis::Mass {
            return Ok(self.clone());
        }

        let avg_mw = self.average_molecular_weight(mw_map)?;
        let mut mass_fractions = IndexMap::with_capacity(self.fractions.len());

        for (name, &y_i) in &self.fractions {
            let mw_i = mw_map
                .get(name)
                .ok_or_else(|| PhysicsError::MissingProperty {
                    name: format!("molecular weight for component {name}"),
                })?;
            let w_i = (y_i * mw_i) / avg_mw;
            mass_fractions.insert(name.clone(), w_i);
        }

        Self::with_basis(mass_fractions, CompositionBasis::Mass)
    }

    /// Converts this composition to a mole fraction basis.
    ///
    /// If already on a mole basis, returns a clone.
    /// If on a mass basis, converts using $y_i = \frac{w_i / M_{w,i}}{\sum (w_k / M_{w,k})}$.
    pub fn to_mole_basis(
        &self,
        mw_map: &std::collections::BTreeMap<String, f64>,
    ) -> Result<Self, PhysicsError> {
        if self.basis == CompositionBasis::Mole {
            return Ok(self.clone());
        }

        let mut moles_map = IndexMap::with_capacity(self.fractions.len());
        for (name, &w_i) in &self.fractions {
            let mw_i = mw_map
                .get(name)
                .ok_or_else(|| PhysicsError::MissingProperty {
                    name: format!("molecular weight for component {name}"),
                })?;
            let n_i = w_i / mw_i;
            moles_map.insert(name.clone(), n_i);
        }

        Self::with_basis(moles_map, CompositionBasis::Mole)
    }

    /// Mixes multiple compositions given their absolute masses or flows.
    pub fn mix(inputs: &[(&Self, f64)]) -> Result<Self, PhysicsError> {
        if inputs.is_empty() {
            return Err(PhysicsError::InvalidComposition {
                reason: "cannot mix zero streams".to_string(),
            });
        }

        let basis = inputs[0].0.basis();
        let mut total_mass = 0.0;
        let mut component_masses: IndexMap<String, f64> = IndexMap::new();

        for (comp, mass) in inputs {
            if !mass.is_finite() || *mass < 0.0 {
                return Err(PhysicsError::InvalidComposition {
                    reason: format!("invalid stream mass for mixing: {mass}"),
                });
            }
            total_mass += mass;
            for (name, frac) in comp.fractions() {
                *component_masses.entry(name.clone()).or_insert(0.0) += frac * mass;
            }
        }

        if total_mass <= 0.0 {
            return Ok(inputs[0].0.clone());
        }

        Self::with_basis(component_masses, basis)
    }
}

/// Complete physical and thermodynamic state of a fluid element or stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhysicalState {
    pressure: Pressure,
    temperature: Temperature,
    density: Density,
    enthalpy: SpecificEnthalpy,
    molecular_weight: f64, // in kg/mol
    vapor_fraction: f64,   // 0.0 = all liquid, 1.0 = all vapor
    phase: Phase,
    composition: ComponentComposition,
}

impl PhysicalState {
    /// Creates and validates a new physical state.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pressure: Pressure,
        temperature: Temperature,
        density: Density,
        enthalpy: SpecificEnthalpy,
        molecular_weight: f64,
        vapor_fraction: f64,
        phase: Phase,
        composition: ComponentComposition,
    ) -> Result<Self, PhysicsError> {
        let state = Self {
            pressure,
            temperature,
            density,
            enthalpy,
            molecular_weight,
            vapor_fraction,
            phase,
            composition,
        };
        state.validate()?;
        Ok(state)
    }

    /// Validates core physical invariants:
    /// - Absolute pressure must be strictly positive ($P > 0$)
    /// - Thermodynamic temperature must be strictly positive ($T > 0$ K)
    /// - Density must be strictly positive ($\rho > 0$)
    /// - Molecular weight must be strictly positive ($M_w > 0$)
    /// - Vapor fraction must be in $[0.0, 1.0]$
    pub fn validate(&self) -> Result<(), PhysicsError> {
        let p = self.pressure.value();
        if !p.is_finite() || p <= 0.0 {
            return Err(PhysicsError::NegativePressure {
                value: p,
                context: "PhysicalState pressure invariant".to_string(),
            });
        }

        let t = self.temperature.value();
        if !t.is_finite() || t <= 0.0 {
            return Err(PhysicsError::NonPositiveTemperature {
                value: t,
                context: "PhysicalState temperature invariant".to_string(),
            });
        }

        let rho = self.density.value();
        if !rho.is_finite() || rho <= 0.0 {
            return Err(PhysicsError::InvalidDensity {
                value: rho,
                context: "PhysicalState density invariant".to_string(),
            });
        }

        if !self.molecular_weight.is_finite() || self.molecular_weight <= 0.0 {
            return Err(PhysicsError::NonFiniteValue {
                field: "molecular_weight".to_string(),
                value: self.molecular_weight,
            });
        }

        if !self.vapor_fraction.is_finite() || !(0.0..=1.0).contains(&self.vapor_fraction) {
            return Err(PhysicsError::InvalidVaporFraction {
                value: self.vapor_fraction,
            });
        }

        if !self.enthalpy.value().is_finite() {
            return Err(PhysicsError::NonFiniteValue {
                field: "enthalpy".to_string(),
                value: self.enthalpy.value(),
            });
        }

        // Validate consistency between phase and vapor fraction
        match self.phase {
            Phase::Liquid => {
                if self.vapor_fraction > 1e-4 {
                    return Err(PhysicsError::InvalidPhaseEquilibrium {
                        phase: self.phase,
                        vapor_fraction: self.vapor_fraction,
                    });
                }
            }
            Phase::Vapor => {
                if self.vapor_fraction < 1.0 - 1e-4 {
                    return Err(PhysicsError::InvalidPhaseEquilibrium {
                        phase: self.phase,
                        vapor_fraction: self.vapor_fraction,
                    });
                }
            }
            Phase::TwoPhase => {
                if self.vapor_fraction <= 0.0 || self.vapor_fraction >= 1.0 {
                    return Err(PhysicsError::InvalidPhaseEquilibrium {
                        phase: self.phase,
                        vapor_fraction: self.vapor_fraction,
                    });
                }
            }
            Phase::Supercritical => {
                // In supercritical state, vapor/liquid boundaries do not apply.
            }
        }

        Ok(())
    }

    pub fn pressure(&self) -> Pressure {
        self.pressure
    }

    pub fn temperature(&self) -> Temperature {
        self.temperature
    }

    pub fn density(&self) -> Density {
        self.density
    }

    pub fn enthalpy(&self) -> SpecificEnthalpy {
        self.enthalpy
    }

    pub fn molecular_weight(&self) -> f64 {
        self.molecular_weight
    }

    pub fn vapor_fraction(&self) -> f64 {
        self.vapor_fraction
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn composition(&self) -> &ComponentComposition {
        &self.composition
    }
}

/// Runtime physical state of a stream connecting equipment ports.
///
/// Supports signed mass flow rates:
/// - Positive mass flow: forward flow from source port to target port.
/// - Negative mass flow: reverse flow from target port back into source port.
/// - Zero flow: stagnant conditions.
///
/// Notice: flow reversals DO NOT mutate the graph topology.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamPhysicalState {
    stream_id: StreamId,
    mass_flow: MassFlow,
    state: PhysicalState,
}

impl StreamPhysicalState {
    /// Creates a stream physical state.
    pub fn new(stream_id: StreamId, mass_flow: MassFlow, state: PhysicalState) -> Self {
        Self {
            stream_id,
            mass_flow,
            state,
        }
    }

    pub fn stream_id(&self) -> StreamId {
        self.stream_id
    }

    pub fn mass_flow_rate(&self) -> MassFlow {
        self.mass_flow
    }

    pub fn physical_state(&self) -> &PhysicalState {
        &self.state
    }

    /// True if fluid is flowing in the nominal design direction (source -> target).
    pub fn is_forward(&self) -> bool {
        self.mass_flow.value() > 1e-9
    }

    /// True if fluid is flowing in reverse (target -> source).
    pub fn is_reverse(&self) -> bool {
        self.mass_flow.value() < -1e-9
    }

    /// True if stream flow is effectively zero / stagnant.
    pub fn is_zero(&self) -> bool {
        self.mass_flow.value().abs() <= 1e-9
    }

    /// Calculates the signed volumetric flow rate in m³/s: $\dot{V} = \frac{\dot{m}}{\rho}$.
    pub fn volumetric_flow(&self) -> Result<VolumetricFlow, PhysicsError> {
        let v_dot = self.mass_flow.value() / self.state.density().value();
        VolumetricFlow::new(v_dot).map_err(|_| PhysicsError::NonFiniteValue {
            field: "stream_volumetric_flow".to_string(),
            value: v_dot,
        })
    }

    /// Calculates the signed total enthalpy flow rate in Watts: $\dot{H} = \dot{m} \cdot h$.
    pub fn enthalpy_flow_rate(&self) -> f64 {
        self.mass_flow.value() * self.state.enthalpy().value()
    }
}
