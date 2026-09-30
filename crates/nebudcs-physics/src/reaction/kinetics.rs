//! Reaction kinetics, stoichiometry, Arrhenius rate evaluation, and reaction systems.
//!
//! Provides mathematically rigorous and scientifically cited chemical reaction models:
//! - Configurable reaction stoichiometry: $\sum_i \nu_i A_i = 0$
//! - Forward and reversible reactions: $r_{\text{net}} = \eta_{\text{cat}} (r_f - r_r)$
//! - Generalized Arrhenius temperature dependence: $k(T) = A \cdot (T / T_{\text{ref}})^n \cdot \exp(-E_a / (R T))$
//! - Reaction enthalpy duty: $q_r = \sum_k r_k \cdot (-\Delta H_{r,k})$
//!
//! References:
//! - Fogler, H.S. "Elements of Chemical Reaction Engineering", 4th Ed., Prentice Hall, 2006.
//! - Levenspiel, O. "Chemical Reaction Engineering", 3rd Ed., John Wiley & Sons, 1999.
//! - Smith, J.M., Van Ness, H.C., Abbott, M.M. "Introduction to Chemical Engineering Thermodynamics", 7th Ed., McGraw-Hill.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::error::PhysicsError;
use crate::thermo::component::R_GAS_CONSTANT;
use crate::thermo::provenance::{ScientificProvenance, ValidityDomain, ValidityState};

/// Result of evaluating a chemical reaction rate at a specific $(T, \mathbf{C})$ condition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReactionRateResult {
    /// Forward reaction rate in mol/(m³·s).
    pub forward_rate: f64,
    /// Reverse reaction rate in mol/(m³·s) (0.0 if irreversible).
    pub reverse_rate: f64,
    /// Net reaction rate in mol/(m³·s) ($r_{\text{net}} = \eta_{\text{cat}} (r_f - r_r)$).
    pub net_rate: f64,
    /// Forward rate constant $k_f(T)$.
    pub k_forward: f64,
    /// Reverse rate constant $k_r(T)$ if reversible.
    pub k_reverse: Option<f64>,
    /// Volumetric thermal heat release in W/m³ ($q_r = r_{\text{net}} \cdot (-\Delta H_r)$).
    /// Positive for exothermic heat release, negative for endothermic absorption.
    pub volumetric_heat_duty_w_per_m3: f64,
    /// Thermodynamic validity status of the calculation point.
    pub validity: ValidityState,
}

/// A configurable chemical reaction definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReactionDefinition {
    /// Unique reaction identifier.
    pub id: String,
    /// Descriptive human-readable reaction name.
    pub name: String,
    /// Stoichiometric coefficients $\nu_i$ for each component (negative for reactants, positive for products).
    pub stoichiometry: IndexMap<String, f64>,
    /// Reaction orders $\alpha_i$ for forward rate $r_f = k_f \prod C_i^{\alpha_i}$.
    pub forward_orders: IndexMap<String, f64>,
    /// Pre-exponential frequency factor $A_f$.
    pub pre_exponential_factor: f64,
    /// Forward activation energy $E_{a,f}$ in J/mol.
    pub activation_energy_j_per_mol: f64,
    /// Temperature exponent $n_f$ for modified Arrhenius (default 0.0).
    pub temperature_exponent: f64,
    /// Standard reference temperature $T_{\text{ref}}$ in Kelvin (default 298.15 K).
    pub reference_temperature_k: f64,
    /// Standard molar heat of reaction $\Delta H_r^\circ$ in J/mol at $T_{\text{ref}}$.
    /// Negative for exothermic reactions, positive for endothermic reactions.
    pub heat_of_reaction_j_per_mol: f64,
    /// Whether the reaction is reversible.
    pub is_reversible: bool,
    /// Reverse reaction orders $\beta_j$ if reversible.
    pub reverse_orders: Option<IndexMap<String, f64>>,
    /// Reverse pre-exponential factor $A_r$ if reversible.
    pub reverse_pre_exponential: Option<f64>,
    /// Reverse activation energy $E_{a,r}$ in J/mol if reversible.
    pub reverse_activation_energy: Option<f64>,
    /// Reverse temperature exponent $n_r$ if reversible.
    pub reverse_temperature_exponent: Option<f64>,
    /// Current catalyst activity scaling factor ($\eta_{\text{cat}} \in [0.0, 5.0]$, default 1.0).
    pub catalyst_activity: f64,
    /// Valid operating domain $(T, P)$ for this reaction model.
    pub validity_domain: ValidityDomain,
    /// Formal scientific provenance and literature citation.
    pub provenance: ScientificProvenance,
}

impl ReactionDefinition {
    /// Creates a new irreversible elementary or power-law reaction.
    #[allow(clippy::too_many_arguments)]
    pub fn new_irreversible(
        id: impl Into<String>,
        name: impl Into<String>,
        stoichiometry: IndexMap<String, f64>,
        forward_orders: IndexMap<String, f64>,
        pre_exponential_factor: f64,
        activation_energy_j_per_mol: f64,
        heat_of_reaction_j_per_mol: f64,
        provenance: ScientificProvenance,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            stoichiometry,
            forward_orders,
            pre_exponential_factor,
            activation_energy_j_per_mol,
            temperature_exponent: 0.0,
            reference_temperature_k: 298.15,
            heat_of_reaction_j_per_mol,
            is_reversible: false,
            reverse_orders: None,
            reverse_pre_exponential: None,
            reverse_activation_energy: None,
            reverse_temperature_exponent: None,
            catalyst_activity: 1.0,
            validity_domain: ValidityDomain {
                t_min_k: 200.0,
                t_max_k: 1500.0,
                p_min_pa: 10_000.0,
                p_max_pa: 30.0e6,
                extrapolation_margin: 0.15,
            },
            provenance,
        }
    }

    /// Creates a reversible chemical reaction.
    #[allow(clippy::too_many_arguments)]
    pub fn new_reversible(
        id: impl Into<String>,
        name: impl Into<String>,
        stoichiometry: IndexMap<String, f64>,
        forward_orders: IndexMap<String, f64>,
        reverse_orders: IndexMap<String, f64>,
        k_forward_a: f64,
        ea_forward: f64,
        k_reverse_a: f64,
        ea_reverse: f64,
        heat_of_reaction_j_per_mol: f64,
        provenance: ScientificProvenance,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            stoichiometry,
            forward_orders,
            pre_exponential_factor: k_forward_a,
            activation_energy_j_per_mol: ea_forward,
            temperature_exponent: 0.0,
            reference_temperature_k: 298.15,
            heat_of_reaction_j_per_mol,
            is_reversible: true,
            reverse_orders: Some(reverse_orders),
            reverse_pre_exponential: Some(k_reverse_a),
            reverse_activation_energy: Some(ea_reverse),
            reverse_temperature_exponent: Some(0.0),
            catalyst_activity: 1.0,
            validity_domain: ValidityDomain {
                t_min_k: 200.0,
                t_max_k: 1500.0,
                p_min_pa: 10_000.0,
                p_max_pa: 30.0e6,
                extrapolation_margin: 0.15,
            },
            provenance,
        }
    }

    /// Evaluates the forward rate constant $k_f(T) = A_f (T / T_{\text{ref}})^{n_f} \exp(-E_a / (R T))$.
    pub fn forward_rate_constant(&self, temperature_k: f64) -> Result<f64, PhysicsError> {
        if temperature_k <= 0.0 || !temperature_k.is_finite() {
            return Err(PhysicsError::NonPositiveTemperature {
                value: temperature_k,
                context: format!("forward rate constant evaluation for reaction {}", self.id),
            });
        }

        let temp_ratio = (temperature_k / self.reference_temperature_k).max(1e-6);
        let t_factor = if self.temperature_exponent.abs() > 1e-9 {
            temp_ratio.powf(self.temperature_exponent)
        } else {
            1.0
        };

        let arg = -self.activation_energy_j_per_mol / (R_GAS_CONSTANT * temperature_k);
        let exp_term = if arg < -100.0 { 0.0 } else { arg.exp() };
        let k = self.pre_exponential_factor * t_factor * exp_term;

        if !k.is_finite() {
            return Err(PhysicsError::NonFiniteValue {
                field: format!("rate_constant_{}", self.id),
                value: k,
            });
        }
        Ok(k.max(0.0))
    }

    /// Evaluates the reverse rate constant $k_r(T)$ if reversible.
    pub fn reverse_rate_constant(&self, temperature_k: f64) -> Result<Option<f64>, PhysicsError> {
        if !self.is_reversible {
            return Ok(None);
        }

        let a_rev = match self.reverse_pre_exponential {
            Some(a) => a,
            None => return Ok(None),
        };
        let ea_rev = self.reverse_activation_energy.unwrap_or(0.0);
        let n_rev = self.reverse_temperature_exponent.unwrap_or(0.0);

        if temperature_k <= 0.0 || !temperature_k.is_finite() {
            return Err(PhysicsError::NonPositiveTemperature {
                value: temperature_k,
                context: format!("reverse rate constant evaluation for reaction {}", self.id),
            });
        }

        let temp_ratio = (temperature_k / self.reference_temperature_k).max(1e-6);
        let t_factor = if n_rev.abs() > 1e-9 {
            temp_ratio.powf(n_rev)
        } else {
            1.0
        };

        let arg = -ea_rev / (R_GAS_CONSTANT * temperature_k);
        let exp_term = if arg < -100.0 { 0.0 } else { arg.exp() };
        let k = a_rev * t_factor * exp_term;

        if !k.is_finite() {
            return Err(PhysicsError::NonFiniteValue {
                field: format!("reverse_rate_constant_{}", self.id),
                value: k,
            });
        }
        Ok(Some(k.max(0.0)))
    }

    /// Evaluates reaction rates and volumetric thermal release given temperature and concentrations.
    ///
    /// Concentrations $C_i$ must be in mol/m³.
    pub fn evaluate_rate(
        &self,
        temperature_k: f64,
        pressure_pa: f64,
        concentrations: &IndexMap<String, f64>,
    ) -> Result<ReactionRateResult, PhysicsError> {
        let validity = self.validity_domain.check(temperature_k, pressure_pa);
        if validity == ValidityState::OutOfRange {
            return Err(PhysicsError::PropertyOutOfRange {
                variable: "temperature/pressure".to_string(),
                value: temperature_k,
                min: self.validity_domain.t_min_k,
                max: self.validity_domain.t_max_k,
                context: format!("reaction {} kinetics evaluation", self.id),
            });
        }

        let k_f = self.forward_rate_constant(temperature_k)?;

        // Forward driving force: \prod C_i^{\alpha_i}
        let mut forward_conc_term = 1.0;
        for (comp, &order) in &self.forward_orders {
            let conc = concentrations.get(comp).copied().unwrap_or(0.0);
            if conc < 0.0 {
                return Err(PhysicsError::ReactionKineticsError {
                    message: format!(
                        "negative concentration {conc} mol/m³ for reactant {comp} in reaction {}",
                        self.id
                    ),
                });
            }
            if conc > 0.0 {
                forward_conc_term *= conc.powf(order);
            } else if order > 0.0 {
                forward_conc_term = 0.0;
                break;
            }
        }
        let forward_rate = k_f * forward_conc_term;

        // Reverse driving force: \prod C_j^{\beta_j}
        let mut reverse_rate = 0.0;
        let mut k_r = None;
        if self.is_reversible {
            if let Some(kr_val) = self.reverse_rate_constant(temperature_k)? {
                k_r = Some(kr_val);
                if let Some(ref rev_orders) = self.reverse_orders {
                    let mut rev_conc_term = 1.0;
                    for (comp, &order) in rev_orders {
                        let conc = concentrations.get(comp).copied().unwrap_or(0.0);
                        if conc < 0.0 {
                            return Err(PhysicsError::ReactionKineticsError {
                                message: format!(
                                    "negative concentration {conc} for product {comp} in reaction {}",
                                    self.id
                                ),
                            });
                        }
                        if conc > 0.0 {
                            rev_conc_term *= conc.powf(order);
                        } else if order > 0.0 {
                            rev_conc_term = 0.0;
                            break;
                        }
                    }
                    reverse_rate = kr_val * rev_conc_term;
                }
            }
        }

        let activity = self.catalyst_activity.clamp(0.0, 10.0);
        let net_rate = activity * (forward_rate - reverse_rate);

        // Volumetric heat release q_r = r_net * (-\Delta H_r) in W/m³
        let volumetric_heat = net_rate * (-self.heat_of_reaction_j_per_mol);

        Ok(ReactionRateResult {
            forward_rate,
            reverse_rate,
            net_rate,
            k_forward: k_f,
            k_reverse: k_r,
            volumetric_heat_duty_w_per_m3: volumetric_heat,
            validity,
        })
    }
}

/// A coupled multi-reaction system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReactionSystem {
    /// System name.
    pub name: String,
    /// List of chemical reactions.
    pub reactions: Vec<ReactionDefinition>,
}

/// Result of evaluating a coupled reaction system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReactionSystemEvaluation {
    /// Net component production rates in mol/(m³·s).
    pub component_rates: IndexMap<String, f64>,
    /// Total volumetric heat duty in W/m³.
    pub total_volumetric_heat_w_per_m3: f64,
    /// Detailed individual reaction evaluation results.
    pub reaction_results: Vec<ReactionRateResult>,
}

impl ReactionSystem {
    /// Creates an empty reaction system.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            reactions: Vec::new(),
        }
    }

    /// Adds a reaction to the system.
    pub fn add_reaction(&mut self, reaction: ReactionDefinition) {
        self.reactions.push(reaction);
    }

    /// Evaluates net generation/consumption rates for all components and total heat duty.
    pub fn evaluate_system(
        &self,
        temperature_k: f64,
        pressure_pa: f64,
        concentrations: &IndexMap<String, f64>,
    ) -> Result<ReactionSystemEvaluation, PhysicsError> {
        let mut comp_rates: IndexMap<String, f64> = IndexMap::new();
        let mut total_heat = 0.0;
        let mut results = Vec::with_capacity(self.reactions.len());

        for rxn in &self.reactions {
            let res = rxn.evaluate_rate(temperature_k, pressure_pa, concentrations)?;
            total_heat += res.volumetric_heat_duty_w_per_m3;

            for (comp, &nu) in &rxn.stoichiometry {
                *comp_rates.entry(comp.clone()).or_insert(0.0) += nu * res.net_rate;
            }
            results.push(res);
        }

        Ok(ReactionSystemEvaluation {
            component_rates: comp_rates,
            total_volumetric_heat_w_per_m3: total_heat,
            reaction_results: results,
        })
    }
}
