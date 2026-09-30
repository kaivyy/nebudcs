//! Configurable Cause and Effect Matrix for Safety Instrumented Systems.
//!
//! Standards Reference: IEC 61511-2 / ISA-84.
//! Dynamically models the mapping from Initiating Causes to Protective Effects.

use crate::sis::sif::{SafetyInstrumentedFunction, SifActionType, SifSafeState};
use serde::{Deserialize, Serialize};

/// Cause row descriptor in the Cause and Effect matrix.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CauseEntry {
    /// SIF identifier.
    pub sif_id: String,
    /// Human-readable cause description (e.g. "Reactor Temp > 150 degC (TAHH-101)").
    pub description: String,
    /// Hazard identifier.
    pub hazard_id: Option<String>,
    /// Initiating threshold.
    pub threshold: f64,
    /// Voting scheme (e.g. "2oo3").
    pub voting: String,
    /// Current live status of this cause (Active / Normal / Bypassed).
    pub status: String,
}

/// Effect column descriptor in the Cause and Effect matrix.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectEntry {
    /// Target final element equipment ID (e.g. "XV-101").
    pub equipment_id: String,
    /// Controlled variable name (e.g. "opening").
    pub variable: String,
    /// Safe state description (e.g. "CLOSED", "TRIP").
    pub safe_state: String,
    /// Action category.
    pub action_type: SifActionType,
}

/// Intersection cell representing whether a Cause activates an Effect.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CauseEffectCell {
    pub cause_index: usize,
    pub effect_index: usize,
    /// Whether this cause-effect link is configured (true = 'X' in matrix).
    pub active_link: bool,
    /// Safe state commanded at this intersection.
    pub commanded_state: Option<SifSafeState>,
    /// Whether the link is currently asserting a trip in real-time.
    pub is_asserted: bool,
}

/// Complete Cause & Effect Matrix representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CauseEffectMatrix {
    pub causes: Vec<CauseEntry>,
    pub effects: Vec<EffectEntry>,
    pub cells: Vec<CauseEffectCell>,
}

impl CauseEffectMatrix {
    /// Builds a Cause & Effect matrix from a collection of Safety Instrumented Functions.
    pub fn from_sifs(sifs: &[SafetyInstrumentedFunction]) -> Self {
        let mut causes = Vec::new();
        let mut effects = Vec::new();

        // 1. Collect all unique effects (final elements)
        for sif in sifs {
            for fe in &sif.config().final_elements {
                let exists = effects.iter().any(|e: &EffectEntry| {
                    e.equipment_id == fe.target_equipment && e.variable == fe.actuator_var
                });
                if !exists {
                    let safe_desc = match fe.safe_state {
                        SifSafeState::ValveClosed => "CLOSED".to_string(),
                        SifSafeState::ValveOpen => "OPEN".to_string(),
                        SifSafeState::DriverTrip => "TRIP".to_string(),
                        SifSafeState::CustomPosition(pos) => format!("{:.1}%", pos),
                    };
                    effects.push(EffectEntry {
                        equipment_id: fe.target_equipment.clone(),
                        variable: fe.actuator_var.clone(),
                        safe_state: safe_desc,
                        action_type: fe.action_type.clone(),
                    });
                }
            }
        }

        // 2. Collect causes
        for sif in sifs {
            let desc = if !sif.config().description.is_empty() {
                sif.config().description.clone()
            } else {
                format!(
                    "{} {:?} {:.1}",
                    sif.name(),
                    sif.config().direction,
                    sif.config().threshold
                )
            };
            let status_str = format!("{:?}", sif.state());
            causes.push(CauseEntry {
                sif_id: sif.id().to_string(),
                description: desc,
                hazard_id: sif.config().hazard_id.clone(),
                threshold: sif.config().threshold,
                voting: format!("{:?}", sif.config().architecture),
                status: status_str,
            });
        }

        // 3. Build intersection cells
        let mut cells = Vec::new();
        for (c_idx, sif) in sifs.iter().enumerate() {
            let is_tripped = sif.is_tripped();
            for (e_idx, eff) in effects.iter().enumerate() {
                let matching_fe = sif.config().final_elements.iter().find(|fe| {
                    fe.target_equipment == eff.equipment_id && fe.actuator_var == eff.variable
                });

                if let Some(fe) = matching_fe {
                    cells.push(CauseEffectCell {
                        cause_index: c_idx,
                        effect_index: e_idx,
                        active_link: true,
                        commanded_state: Some(fe.safe_state.clone()),
                        is_asserted: is_tripped,
                    });
                } else {
                    cells.push(CauseEffectCell {
                        cause_index: c_idx,
                        effect_index: e_idx,
                        active_link: false,
                        commanded_state: None,
                        is_asserted: false,
                    });
                }
            }
        }

        Self {
            causes,
            effects,
            cells,
        }
    }
}
