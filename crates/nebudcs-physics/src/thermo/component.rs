//! Thermodynamic component data structures and validated physical properties database.
//!
//! Provides the core component definition [`ThermodynamicComponent`], binary interaction
//! parameters matrix [`BinaryInteractionMatrix`], and the standard validated component
//! library [`ComponentDatabase`].

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::correlation::AntoineVaporPressureCorrelation;
use super::provenance::{ScientificProvenance, ValidityState};
use crate::error::PhysicsError;

/// Standard reference temperature: 298.15 K (25 °C).
pub const STANDARD_REFERENCE_TEMPERATURE_K: f64 = 298.15;

/// Standard reference pressure: 101,325 Pa (1 atm).
pub const STANDARD_REFERENCE_PRESSURE_PA: f64 = 101_325.0;

/// Universal gas constant in $\text{J}/(\text{mol}\cdot\text{K})$.
pub const R_GAS_CONSTANT: f64 = 8.314_462_618_153_24;

/// Reference state convention for thermodynamic enthalpy and entropy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ReferenceStateConvention {
    /// IUPAC / DIPPR Standard: 298.15 K, 101,325 Pa ideal gas state.
    #[default]
    Standard101325Pa298K,
    /// Normal Boiling Point (NBP) saturated liquid state: $H_{\text{liq}}(T_b) = 0, S_{\text{liq}}(T_b) = 0$.
    NormalBoilingPoint,
    /// Element Reference State: Elements in standard state at 298.15 K have $\Delta H_f^\circ = 0$.
    ElementReferenceState,
}

/// Architectural schema placeholder for future transport property models.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TransportPropertiesSchema {
    pub liquid_viscosity_model: Option<String>,
    pub vapor_viscosity_model: Option<String>,
    pub thermal_conductivity_model: Option<String>,
    pub surface_tension_model: Option<String>,
    pub binary_diffusion_model: Option<String>,
}

/// Pure chemical component thermodynamic specification.
///
/// Contains critical properties, acentric factor, ideal gas heat capacity polynomial
/// coefficients, standard state formation properties, vapor pressure correlations, and provenance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermodynamicComponent {
    /// Stable component identifier (e.g. `"comp_methane"`).
    pub component_id: String,
    /// Chemical identifier / canonical name (e.g. `"Methane"`).
    pub name: String,
    /// Recognized component aliases (e.g. `["CH4", "marsh_gas"]`).
    pub aliases: Vec<String>,
    /// Chemical formula or symbol (e.g. `"CH4"`).
    pub formula: String,
    /// Chemical Abstracts Service (CAS) registry number where available.
    pub cas_number: Option<String>,
    /// Molecular weight in kg/mol (e.g. 0.016043 for Methane).
    pub molecular_weight: f64,
    /// Critical temperature $T_c$ in Kelvin.
    pub critical_temperature: f64,
    /// Critical pressure $P_c$ in Pascal.
    pub critical_pressure: f64,
    /// Critical molar volume $V_c$ in $\text{m}^3/\text{mol}$.
    pub critical_volume: Option<f64>,
    /// Critical compressibility factor $Z_c = P_c V_c / (R T_c)$.
    pub critical_compressibility: Option<f64>,
    /// Pitzer acentric factor $\omega$ (dimensionless).
    pub acentric_factor: f64,
    /// Normal boiling point temperature at 101,325 Pa in Kelvin.
    pub normal_boiling_point: Option<f64>,
    /// Triple-point temperature in Kelvin.
    pub triple_point_temperature: Option<f64>,
    /// Triple-point absolute pressure in Pascal.
    pub triple_point_pressure: Option<f64>,
    /// Standard heat of vaporization $\Delta H_{\text{vap}}$ at normal boiling point in J/mol.
    pub heat_of_vaporization: Option<f64>,
    /// Standard heat of fusion $\Delta H_{\text{fus}}$ at melting point in J/mol.
    pub heat_of_fusion: Option<f64>,
    /// Melting point temperature in Kelvin.
    pub melting_point: Option<f64>,
    /// Reference state convention used for enthalpy and entropy datums.
    pub reference_convention: ReferenceStateConvention,
    /// Ideal gas molar isobaric heat capacity polynomial coefficients:
    /// $C_p^{ig}(T) = a_0 + a_1 T + a_2 T^2 + a_3 T^3 + a_4 T^4$ in $\text{J}/(\text{mol}\cdot\text{K})$.
    pub cp_ideal_coefficients: [f64; 5],
    /// Standard molar enthalpy of formation $\Delta H_f^\circ$ at 298.15 K in J/mol.
    pub reference_enthalpy: f64,
    /// Standard molar entropy $S^\circ$ at 298.15 K, 101,325 Pa in J/(mol·K).
    pub reference_entropy: f64,
    /// Calibrated Antoine vapor pressure correlation.
    pub antoine_vapor_pressure: Option<AntoineVaporPressureCorrelation>,
    /// Schema definition for future transport property models.
    pub transport_schema: TransportPropertiesSchema,
    /// Full scientific provenance and literature citation.
    pub provenance: Option<ScientificProvenance>,
}

impl ThermodynamicComponent {
    /// Creates and validates a new thermodynamic component.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: impl Into<String>,
        formula: impl Into<String>,
        molecular_weight: f64,
        critical_temperature: f64,
        critical_pressure: f64,
        acentric_factor: f64,
        normal_boiling_point: Option<f64>,
        cp_ideal_coefficients: [f64; 5],
        reference_enthalpy: f64,
        reference_entropy: f64,
    ) -> Result<Self, PhysicsError> {
        let n: String = name.into();
        let f: String = formula.into();
        let cid = format!("comp_{}", n.to_lowercase().replace(['-', ' '], "_"));

        let comp = Self {
            component_id: cid,
            name: n,
            aliases: vec![f.clone()],
            formula: f,
            cas_number: None,
            molecular_weight,
            critical_temperature,
            critical_pressure,
            critical_volume: None,
            critical_compressibility: None,
            acentric_factor,
            normal_boiling_point,
            triple_point_temperature: None,
            triple_point_pressure: None,
            heat_of_vaporization: None,
            heat_of_fusion: None,
            melting_point: None,
            reference_convention: ReferenceStateConvention::Standard101325Pa298K,
            cp_ideal_coefficients,
            reference_enthalpy,
            reference_entropy,
            antoine_vapor_pressure: None,
            transport_schema: TransportPropertiesSchema::default(),
            provenance: None,
        };
        comp.validate()?;
        Ok(comp)
    }

    /// Builder setter for CAS number.
    pub fn with_cas(mut self, cas: impl Into<String>) -> Self {
        self.cas_number = Some(cas.into());
        self
    }

    /// Builder setter for critical volume and compressibility.
    pub fn with_critical_volume(mut self, vc_m3_mol: f64) -> Self {
        self.critical_volume = Some(vc_m3_mol);
        let zc =
            (self.critical_pressure * vc_m3_mol) / (R_GAS_CONSTANT * self.critical_temperature);
        self.critical_compressibility = Some(zc);
        self
    }

    /// Builder setter for triple-point data.
    pub fn with_triple_point(mut self, t_tp_k: f64, p_tp_pa: f64) -> Self {
        self.triple_point_temperature = Some(t_tp_k);
        self.triple_point_pressure = Some(p_tp_pa);
        self
    }

    /// Builder setter for phase transition enthalpies.
    pub fn with_phase_transitions(
        mut self,
        h_vap_j_mol: f64,
        h_fus_j_mol: f64,
        t_m_k: f64,
    ) -> Self {
        self.heat_of_vaporization = Some(h_vap_j_mol);
        self.heat_of_fusion = Some(h_fus_j_mol);
        self.melting_point = Some(t_m_k);
        self
    }

    /// Builder setter for Antoine vapor pressure correlation.
    pub fn with_antoine(mut self, antoine: AntoineVaporPressureCorrelation) -> Self {
        self.antoine_vapor_pressure = Some(antoine);
        self
    }

    /// Builder setter for scientific provenance.
    pub fn with_provenance(mut self, provenance: ScientificProvenance) -> Self {
        self.provenance = Some(provenance);
        self
    }

    /// Validates physical invariants:
    /// - Molecular weight must be strictly positive ($M_w > 0$).
    /// - Critical temperature must be strictly positive ($T_c > 0$).
    /// - Critical pressure must be strictly positive ($P_c > 0$).
    /// - Acentric factor must be finite and $\ge -0.5$.
    /// - Heat capacity coefficients and reference properties must be finite.
    pub fn validate(&self) -> Result<(), PhysicsError> {
        if self.name.trim().is_empty() {
            return Err(PhysicsError::InvalidComposition {
                reason: "component name cannot be blank".to_string(),
            });
        }

        if !self.molecular_weight.is_finite() || self.molecular_weight <= 0.0 {
            return Err(PhysicsError::NonFiniteValue {
                field: format!("{}_molecular_weight", self.name),
                value: self.molecular_weight,
            });
        }

        if !self.critical_temperature.is_finite() || self.critical_temperature <= 0.0 {
            return Err(PhysicsError::NonPositiveTemperature {
                value: self.critical_temperature,
                context: format!("{}_critical_temperature", self.name),
            });
        }

        if !self.critical_pressure.is_finite() || self.critical_pressure <= 0.0 {
            return Err(PhysicsError::NegativePressure {
                value: self.critical_pressure,
                context: format!("{}_critical_pressure", self.name),
            });
        }

        if !self.acentric_factor.is_finite() || self.acentric_factor < -0.5 {
            return Err(PhysicsError::InvalidOperatingCondition {
                message: format!(
                    "component {} acentric factor {} must be finite and >= -0.5",
                    self.name, self.acentric_factor
                ),
            });
        }

        for (idx, coeff) in self.cp_ideal_coefficients.iter().enumerate() {
            if !coeff.is_finite() {
                return Err(PhysicsError::NonFiniteValue {
                    field: format!("{}_cp_coeff_{idx}", self.name),
                    value: *coeff,
                });
            }
        }

        if !self.reference_enthalpy.is_finite() {
            return Err(PhysicsError::NonFiniteValue {
                field: format!("{}_reference_enthalpy", self.name),
                value: self.reference_enthalpy,
            });
        }

        if !self.reference_entropy.is_finite() {
            return Err(PhysicsError::NonFiniteValue {
                field: format!("{}_reference_entropy", self.name),
                value: self.reference_entropy,
            });
        }

        if let Some(tb) = self.normal_boiling_point {
            if !tb.is_finite() || tb <= 0.0 {
                return Err(PhysicsError::NonPositiveTemperature {
                    value: tb,
                    context: format!("{}_normal_boiling_point", self.name),
                });
            }
        }

        Ok(())
    }

    /// Evaluates molar isobaric heat capacity $C_p^{\text{ig}}(T)$ in $\text{J}/(\text{mol}\cdot\text{K})$ at temperature $T$ (K).
    pub fn cp_ideal(&self, t_k: f64) -> f64 {
        let t = t_k.max(1.0);
        let [a0, a1, a2, a3, a4] = self.cp_ideal_coefficients;
        let t2 = t * t;
        let t3 = t2 * t;
        let t4 = t3 * t;
        a0 + a1 * t + a2 * t2 + a3 * t3 + a4 * t4
    }

    /// Evaluates ideal gas molar enthalpy departure $\int_{T_{\text{ref}}}^T C_p^{\text{ig}}(T') dT'$ in J/mol.
    pub fn delta_h_ideal(&self, t_k: f64) -> f64 {
        let t = t_k.max(1.0);
        let t_ref = STANDARD_REFERENCE_TEMPERATURE_K;
        let [a0, a1, a2, a3, a4] = self.cp_ideal_coefficients;

        let int_t = a0 * t
            + 0.5 * a1 * t * t
            + (1.0 / 3.0) * a2 * t.powi(3)
            + 0.25 * a3 * t.powi(4)
            + 0.2 * a4 * t.powi(5);
        let int_ref = a0 * t_ref
            + 0.5 * a1 * t_ref * t_ref
            + (1.0 / 3.0) * a2 * t_ref.powi(3)
            + 0.25 * a3 * t_ref.powi(4)
            + 0.2 * a4 * t_ref.powi(5);

        int_t - int_ref
    }

    /// Evaluates ideal gas molar entropy departure $\int_{T_{\text{ref}}}^T \frac{C_p^{\text{ig}}(T')}{T'} dT'$ in J/(mol·K).
    pub fn delta_s_ideal_temp(&self, t_k: f64) -> f64 {
        let t = t_k.max(1.0);
        let t_ref = STANDARD_REFERENCE_TEMPERATURE_K;
        let [a0, a1, a2, a3, a4] = self.cp_ideal_coefficients;

        let int_t = a0 * t.ln()
            + a1 * t
            + 0.5 * a2 * t * t
            + (1.0 / 3.0) * a3 * t.powi(3)
            + 0.25 * a4 * t.powi(4);
        let int_ref = a0 * t_ref.ln()
            + a1 * t_ref
            + 0.5 * a2 * t_ref * t_ref
            + (1.0 / 3.0) * a3 * t_ref.powi(3)
            + 0.25 * a4 * t_ref.powi(4);

        int_t - int_ref
    }

    /// Evaluates saturation vapor pressure via Antoine correlation if calibrated.
    pub fn vapor_pressure_antoine(&self, t_k: f64) -> Result<(f64, ValidityState), PhysicsError> {
        match &self.antoine_vapor_pressure {
            Some(antoine) => {
                use super::correlation::ScientificCorrelation;
                antoine.evaluate(t_k)
            }
            None => Err(PhysicsError::MissingProperty {
                name: format!("{}_antoine_vapor_pressure", self.name),
            }),
        }
    }
}

/// Binary interaction parameters ($k_{ij}$) matrix for quadratic mixing rules.
///
/// Strictly enforces:
/// - Symmetry: $k_{ij} = k_{ji}$
/// - Diagonal identity: $k_{ii} = 0.0$
/// - Explicit missing-pair convention: defaults to 0.0
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BinaryInteractionMatrix {
    /// Sparse canonical representation: key is sorted pair `(min(c1, c2), max(c1, c2))`.
    matrix: BTreeMap<(String, String), f64>,
}

impl BinaryInteractionMatrix {
    /// Creates an empty binary interaction matrix.
    pub fn new() -> Self {
        Self {
            matrix: BTreeMap::new(),
        }
    }

    /// Sets a binary interaction parameter $k_{ij}$ between component pair.
    ///
    /// Automatically enforces symmetry $k_{ij} = k_{ji}$ and ignores $k_{ii}$.
    pub fn set(
        &mut self,
        c1: impl Into<String>,
        c2: impl Into<String>,
        kij: f64,
    ) -> Result<(), PhysicsError> {
        let name1 = c1.into();
        let name2 = c2.into();

        if !kij.is_finite() {
            return Err(PhysicsError::NonFiniteValue {
                field: format!("kij_{name1}_{name2}"),
                value: kij,
            });
        }

        if name1 == name2 {
            return Ok(());
        }

        let key = if name1 < name2 {
            (name1, name2)
        } else {
            (name2, name1)
        };

        self.matrix.insert(key, kij);
        Ok(())
    }

    /// Retrieves binary interaction parameter $k_{ij}$ with automatic symmetry and default 0.0.
    pub fn get(&self, c1: &str, c2: &str) -> f64 {
        if c1 == c2 {
            return 0.0;
        }

        let key = if c1 < c2 {
            (c1.to_string(), c2.to_string())
        } else {
            (c2.to_string(), c1.to_string())
        };

        self.matrix.get(&key).copied().unwrap_or(0.0)
    }

    /// Returns the number of explicitly registered interaction parameter pairs.
    pub fn len(&self) -> usize {
        self.matrix.len()
    }

    /// Returns true if no interaction parameters have been registered.
    pub fn is_empty(&self) -> bool {
        self.matrix.is_empty()
    }
}

/// Validated registry of pure components and binary interaction parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentDatabase {
    /// Schema / database version string.
    pub version: String,
    /// Provenance metadata for this database release.
    pub provenance: ScientificProvenance,
    /// Version identifier of the binary interaction dataset.
    pub kij_version: String,
    components: IndexMap<String, ThermodynamicComponent>,
    alias_map: IndexMap<String, String>,
    binary_interactions: BinaryInteractionMatrix,
}

impl Default for ComponentDatabase {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentDatabase {
    /// Creates an empty component database.
    pub fn new() -> Self {
        Self {
            version: "1.0.0".to_string(),
            provenance: ScientificProvenance::nist(
                "NIST Chemistry WebBook SRD 69 & DIPPR 801 (2024)",
                Some("10.18434/T4D303"),
                "SI",
            ),
            kij_version: "API-Literature-1.0.0".to_string(),
            components: IndexMap::new(),
            alias_map: IndexMap::new(),
            binary_interactions: BinaryInteractionMatrix::new(),
        }
    }

    /// Registers a component in the database.
    pub fn register_component(
        &mut self,
        component: ThermodynamicComponent,
    ) -> Result<(), PhysicsError> {
        component.validate()?;
        let canonical_name = component.name.clone();

        self.alias_map
            .insert(canonical_name.to_lowercase(), canonical_name.clone());
        self.alias_map
            .insert(component.formula.to_lowercase(), canonical_name.clone());
        for alias in &component.aliases {
            self.alias_map
                .insert(alias.to_lowercase(), canonical_name.clone());
        }

        self.components.insert(canonical_name, component);
        Ok(())
    }

    /// Resolves canonical component name from an alias or chemical formula.
    pub fn resolve_canonical_name(&self, identifier: &str) -> Option<&str> {
        if let Some((k, _)) = self.components.get_key_value(identifier) {
            return Some(k.as_str());
        }
        self.alias_map
            .get(&identifier.to_lowercase())
            .map(|s| s.as_str())
    }

    /// Retrieves component definition by canonical name or alias.
    pub fn get(&self, identifier: &str) -> Option<&ThermodynamicComponent> {
        let canonical = self.resolve_canonical_name(identifier)?;
        self.components.get(canonical)
    }

    /// Returns a slice of all registered components in deterministic registration order.
    pub fn components(&self) -> &IndexMap<String, ThermodynamicComponent> {
        &self.components
    }

    /// Returns true if the database contains the given component or alias.
    pub fn contains(&self, identifier: &str) -> bool {
        self.resolve_canonical_name(identifier).is_some()
    }

    /// Sets binary interaction parameter between two components.
    pub fn set_kij(&mut self, c1: &str, c2: &str, kij: f64) -> Result<(), PhysicsError> {
        let can1 = self
            .resolve_canonical_name(c1)
            .ok_or_else(|| PhysicsError::MissingProperty {
                name: c1.to_string(),
            })?
            .to_string();
        let can2 = self
            .resolve_canonical_name(c2)
            .ok_or_else(|| PhysicsError::MissingProperty {
                name: c2.to_string(),
            })?
            .to_string();

        self.binary_interactions.set(can1, can2, kij)
    }

    /// Retrieves binary interaction parameter between two components.
    pub fn get_kij(&self, c1: &str, c2: &str) -> f64 {
        let can1 = match self.resolve_canonical_name(c1) {
            Some(n) => n,
            None => return 0.0,
        };
        let can2 = match self.resolve_canonical_name(c2) {
            Some(n) => n,
            None => return 0.0,
        };

        self.binary_interactions.get(can1, can2)
    }

    /// Returns a reference to the binary interaction matrix.
    pub fn binary_interactions(&self) -> &BinaryInteractionMatrix {
        &self.binary_interactions
    }

    /// Pre-populates the database with verified standard industrial chemical components
    /// with complete provenance, critical properties, and Antoine correlations.
    pub fn standard_library() -> Self {
        let mut db = Self::new();
        let prov_nist = ScientificProvenance::nist(
            "NIST Chemistry WebBook SRD 69 (Lemmon et al., 2024)",
            Some("10.18434/T4D303"),
            "SI",
        );

        // 1. Methane (CH4, CAS: 74-82-8)
        let antoine_c1 = AntoineVaporPressureCorrelation::new(
            3.98950,
            443.011,
            -0.49,
            90.7,
            190.0,
            prov_nist.clone(),
        );
        let methane = ThermodynamicComponent::new(
            "Methane",
            "CH4",
            0.016043,
            190.56,
            4.5992e6,
            0.0115,
            Some(111.66),
            [34.398, -1.249e-2, 7.397e-5, -4.792e-8, 9.69e-12],
            -74_870.0,
            186.25,
        )
        .expect("valid methane")
        .with_cas("74-82-8")
        .with_critical_volume(0.0986e-3)
        .with_triple_point(90.69, 11_690.0)
        .with_phase_transitions(8_170.0, 940.0, 90.69)
        .with_antoine(antoine_c1)
        .with_provenance(prov_nist.clone());
        db.register_component(methane).unwrap();

        // 2. Ethane (C2H6, CAS: 74-84-0)
        let antoine_c2 = AntoineVaporPressureCorrelation::new(
            4.08412,
            708.97,
            -15.42,
            130.0,
            305.0,
            prov_nist.clone(),
        );
        let ethane = ThermodynamicComponent::new(
            "Ethane",
            "C2H6",
            0.030070,
            305.32,
            4.8722e6,
            0.0995,
            Some(184.55),
            [28.140, 0.109, -2.48e-5, -1.2e-8, 3.2e-12],
            -84_000.0,
            229.60,
        )
        .expect("valid ethane")
        .with_cas("74-84-0")
        .with_critical_volume(0.1455e-3)
        .with_triple_point(90.35, 1.1)
        .with_phase_transitions(14_690.0, 2_860.0, 90.35)
        .with_antoine(antoine_c2)
        .with_provenance(prov_nist.clone());
        db.register_component(ethane).unwrap();

        // 3. Propane (C3H8, CAS: 74-98-6)
        let antoine_c3 = AntoineVaporPressureCorrelation::new(
            4.01140,
            834.26,
            -22.71,
            165.0,
            369.0,
            prov_nist.clone(),
        );
        let propane = ThermodynamicComponent::new(
            "Propane",
            "C3H8",
            0.044096,
            369.83,
            4.2480e6,
            0.1523,
            Some(231.05),
            [28.277, 0.116, 1.96e-4, -2.327e-7, 6.885e-11],
            -104_700.0,
            270.31,
        )
        .expect("valid propane")
        .with_cas("74-98-6")
        .with_critical_volume(0.2000e-3)
        .with_triple_point(85.53, 0.17)
        .with_phase_transitions(19_040.0, 3_530.0, 85.53)
        .with_antoine(antoine_c3)
        .with_provenance(prov_nist.clone());
        db.register_component(propane).unwrap();

        // 4. Isobutane (i-C4H10, CAS: 75-28-5)
        let antoine_ic4 = AntoineVaporPressureCorrelation::new(
            4.00270,
            947.54,
            -25.26,
            188.0,
            407.0,
            prov_nist.clone(),
        );
        let isobutane = ThermodynamicComponent::new(
            "Isobutane",
            "i-C4H10",
            0.058122,
            407.82,
            3.6290e6,
            0.1848,
            Some(261.42),
            [34.18, 0.177, 1.5e-4, -2.2e-7, 6.5e-11],
            -134_500.0,
            295.40,
        )
        .expect("valid isobutane")
        .with_cas("75-28-5")
        .with_critical_volume(0.2627e-3)
        .with_triple_point(113.73, 0.019)
        .with_phase_transitions(21_290.0, 4_540.0, 113.73)
        .with_antoine(antoine_ic4)
        .with_provenance(prov_nist.clone());
        db.register_component(isobutane).unwrap();

        // 5. n-Butane (n-C4H10, CAS: 106-97-8)
        let antoine_nc4 = AntoineVaporPressureCorrelation::new(
            4.07800,
            992.88,
            -34.83,
            195.0,
            425.0,
            prov_nist.clone(),
        );
        let n_butane = ThermodynamicComponent::new(
            "n-Butane",
            "n-C4H10",
            0.058122,
            425.12,
            3.7960e6,
            0.2002,
            Some(272.65),
            [36.10, 0.185, 1.4e-4, -2.1e-7, 6.3e-11],
            -126_500.0,
            310.23,
        )
        .expect("valid n-butane")
        .with_cas("106-97-8")
        .with_critical_volume(0.2550e-3)
        .with_triple_point(134.86, 0.7)
        .with_phase_transitions(22_440.0, 4_660.0, 134.86)
        .with_antoine(antoine_nc4)
        .with_provenance(prov_nist.clone());
        db.register_component(n_butane).unwrap();

        // 6. Nitrogen (N2, CAS: 7727-37-9)
        let antoine_n2 = AntoineVaporPressureCorrelation::new(
            3.73620,
            264.65,
            -6.79,
            63.0,
            126.0,
            prov_nist.clone(),
        );
        let nitrogen = ThermodynamicComponent::new(
            "Nitrogen",
            "N2",
            0.028013,
            126.19,
            3.3958e6,
            0.0372,
            Some(77.36),
            [31.15, -0.01357, 2.68e-5, -1.17e-8, 1.0e-12],
            0.0,
            191.61,
        )
        .expect("valid nitrogen")
        .with_cas("7727-37-9")
        .with_critical_volume(0.0892e-3)
        .with_triple_point(63.15, 12_520.0)
        .with_phase_transitions(5_560.0, 720.0, 63.15)
        .with_antoine(antoine_n2)
        .with_provenance(prov_nist.clone());
        db.register_component(nitrogen).unwrap();

        // 7. Carbon Dioxide (CO2, CAS: 124-38-9)
        let antoine_co2 = AntoineVaporPressureCorrelation::new(
            4.45330,
            654.51,
            -23.15,
            154.0,
            304.0,
            prov_nist.clone(),
        );
        let co2 = ThermodynamicComponent::new(
            "CarbonDioxide",
            "CO2",
            0.044010,
            304.13,
            7.3773e6,
            0.2239,
            Some(194.67),
            [24.997, 0.055, -3.37e-5, 7.9e-9, -6.0e-13],
            -393_510.0,
            213.74,
        )
        .expect("valid co2")
        .with_cas("124-38-9")
        .with_critical_volume(0.0940e-3)
        .with_triple_point(216.58, 518_500.0)
        .with_phase_transitions(25_200.0, 9_020.0, 216.58)
        .with_antoine(antoine_co2)
        .with_provenance(prov_nist.clone());
        db.register_component(co2).unwrap();

        // 8. Hydrogen (H2, CAS: 1333-74-0)
        let antoine_h2 = AntoineVaporPressureCorrelation::new(
            3.54300,
            43.14,
            2.15,
            14.0,
            33.0,
            prov_nist.clone(),
        );
        let hydrogen = ThermodynamicComponent::new(
            "Hydrogen",
            "H2",
            0.002016,
            33.14,
            1.2964e6,
            -0.216,
            Some(20.28),
            [27.14, 0.00927, -1.38e-5, 7.64e-9, -1.0e-12],
            0.0,
            130.68,
        )
        .expect("valid hydrogen")
        .with_cas("1333-74-0")
        .with_critical_volume(0.0641e-3)
        .with_triple_point(13.96, 7_200.0)
        .with_phase_transitions(900.0, 120.0, 13.96)
        .with_antoine(antoine_h2)
        .with_provenance(prov_nist.clone());
        db.register_component(hydrogen).unwrap();

        // 9. Water (H2O, CAS: 7732-18-5)
        let antoine_h2o = AntoineVaporPressureCorrelation::new(
            5.11564,
            1687.537,
            -42.98,
            273.0,
            647.0,
            prov_nist.clone(),
        );
        let water = ThermodynamicComponent::new(
            "Water",
            "H2O",
            0.018015,
            647.10,
            22.064e6,
            0.3443,
            Some(373.15),
            [32.24, 0.00192, 1.055e-5, -3.596e-9, 4.0e-13],
            -241_818.0,
            188.84,
        )
        .expect("valid water")
        .with_cas("7732-18-5")
        .with_critical_volume(0.0559e-3)
        .with_triple_point(273.16, 611.65)
        .with_phase_transitions(40_660.0, 6_010.0, 273.15)
        .with_antoine(antoine_h2o)
        .with_provenance(prov_nist);
        db.register_component(water).unwrap();

        // Standard hydrocarbon binary interaction parameters (Literature PR values)
        let _ = db.set_kij("Methane", "Ethane", 0.0);
        let _ = db.set_kij("CH4", "C2H6", 0.0);
        let _ = db.set_kij("Methane", "Propane", 0.008);
        let _ = db.set_kij("CH4", "C3H8", 0.008);
        let _ = db.set_kij("Methane", "n-Butane", 0.015);
        let _ = db.set_kij("CH4", "n-C4H10", 0.015);
        let _ = db.set_kij("Nitrogen", "Methane", 0.036);
        let _ = db.set_kij("N2", "CH4", 0.036);
        let _ = db.set_kij("CarbonDioxide", "Methane", 0.095);
        let _ = db.set_kij("CO2", "CH4", 0.095);
        let _ = db.set_kij("Nitrogen", "Ethane", 0.042);
        let _ = db.set_kij("N2", "C2H6", 0.042);

        db
    }
}
