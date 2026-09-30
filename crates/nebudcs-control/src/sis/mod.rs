//! Safety Instrumented System (SIS) and Functional Safety Architecture.
//!
//! Standards Reference: IEC 61511 / ISA-84 / IEC 61508.
//! Note: Virtual simulation model for engineering verification and training.

pub mod cause_effect;
pub mod proof_test;
pub mod sif;
pub mod voting;

use crate::actuator::ActuatorCommand;
use crate::runtime::IoImageTable;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub use cause_effect::{CauseEffectCell, CauseEffectMatrix, CauseEntry, EffectEntry};
pub use proof_test::{run_simulated_proof_test, ProofTestRecord, ProofTestStepResult};
pub use sif::{
    SafetyInstrumentedFunction, SifActionType, SifBypass, SifConfig, SifDiagnostics,
    SifFinalElement, SifSafeState, SifState,
};
pub use voting::{
    evaluate_voting, DegradationPolicy, SensorChannel, SifVotingResult, TripDirection,
    VotingArchitecture,
};

/// Safety audit trail event (bypass, reset, trip, proof-test).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SafetyAuditEvent {
    pub timestamp_sim: f64,
    pub event_type: String,
    pub sif_id: String,
    pub actor: String,
    pub detail: String,
}

/// The top-level Safety Instrumented System (SIS) engine.
///
/// Executes independent of the Basic Process Control System (BPCS)
/// with absolute safety command override authority.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SafetyInstrumentedSystem {
    /// Whether the safety system is armed and actively evaluating.
    pub enabled: bool,
    /// Registered Safety Instrumented Functions keyed by unique SIF ID.
    sifs: BTreeMap<String, SafetyInstrumentedFunction>,
    /// Historical proof test reports.
    proof_tests: Vec<ProofTestRecord>,
    /// Safety audit journal.
    audit_trail: Vec<SafetyAuditEvent>,
}

impl Default for SafetyInstrumentedSystem {
    fn default() -> Self {
        Self {
            enabled: true,
            sifs: BTreeMap::new(),
            proof_tests: Vec::new(),
            audit_trail: Vec::new(),
        }
    }
}

impl SafetyInstrumentedSystem {
    /// Creates a new empty Safety Instrumented System.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the Safety Instrumented System is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enables or disables the Safety Instrumented System.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Registers a new Safety Instrumented Function.
    pub fn register_sif(&mut self, sif: SafetyInstrumentedFunction) {
        self.sifs.insert(sif.id().to_string(), sif);
    }

    /// Removes an SIF by identifier.
    pub fn remove_sif(&mut self, sif_id: &str) -> Option<SafetyInstrumentedFunction> {
        self.sifs.remove(sif_id)
    }

    /// Access to an immutable reference to an SIF by identifier.
    pub fn sif(&self, sif_id: &str) -> Option<&SafetyInstrumentedFunction> {
        self.sifs.get(sif_id)
    }

    /// Access to a mutable reference to an SIF by identifier.
    pub fn sif_mut(&mut self, sif_id: &str) -> Option<&mut SafetyInstrumentedFunction> {
        self.sifs.get_mut(sif_id)
    }

    /// Returns all registered SIFs.
    pub fn sifs(&self) -> &BTreeMap<String, SafetyInstrumentedFunction> {
        &self.sifs
    }

    /// Returns mutable map of all registered SIFs.
    pub fn sifs_mut(&mut self) -> &mut BTreeMap<String, SafetyInstrumentedFunction> {
        &mut self.sifs
    }

    /// Returns historical proof test records.
    pub fn proof_tests(&self) -> &[ProofTestRecord] {
        &self.proof_tests
    }

    /// Returns the complete safety audit trail.
    pub fn audit_trail(&self) -> &[SafetyAuditEvent] {
        &self.audit_trail
    }

    /// Records an entry into the safety audit trail.
    pub fn log_audit(
        &mut self,
        sim_time: f64,
        event_type: impl Into<String>,
        sif_id: impl Into<String>,
        actor: impl Into<String>,
        detail: impl Into<String>,
    ) {
        self.audit_trail.push(SafetyAuditEvent {
            timestamp_sim: sim_time,
            event_type: event_type.into(),
            sif_id: sif_id.into(),
            actor: actor.into(),
            detail: detail.into(),
        });
    }

    /// Requests manual reset of a tripped SIF.
    pub fn reset_sif(
        &mut self,
        sif_id: &str,
        actor: impl Into<String>,
        sim_time: f64,
        io_image: &IoImageTable,
    ) -> Result<(), String> {
        let actor_name = actor.into();
        let sif = self
            .sifs
            .get_mut(sif_id)
            .ok_or_else(|| format!("SIF '{}' not found in Safety Instrumented System", sif_id))?;

        sif.request_reset(sim_time, io_image)?;

        self.audit_trail.push(SafetyAuditEvent {
            timestamp_sim: sim_time,
            event_type: "SIF_RESET".to_string(),
            sif_id: sif_id.to_string(),
            actor: actor_name,
            detail: "Authorized operator manual safety reset successful".to_string(),
        });

        Ok(())
    }

    /// Sets or clears an administrative bypass on an SIF.
    pub fn set_bypass(
        &mut self,
        sif_id: &str,
        bypass: Option<SifBypass>,
        sim_time: f64,
    ) -> Result<(), String> {
        let sif = self
            .sifs
            .get_mut(sif_id)
            .ok_or_else(|| format!("SIF '{}' not found in Safety Instrumented System", sif_id))?;

        let (actor, detail) = if let Some(ref b) = bypass {
            (
                b.authorized_by.clone(),
                format!(
                    "Bypass activated (type: {}, reason: '{}')",
                    b.bypass_type, b.reason
                ),
            )
        } else {
            (
                "SYSTEM".to_string(),
                "Bypass removed: function restored to normal monitoring".to_string(),
            )
        };

        sif.set_bypass(bypass);

        self.audit_trail.push(SafetyAuditEvent {
            timestamp_sim: sim_time,
            event_type: "SIF_BYPASS_CHANGE".to_string(),
            sif_id: sif_id.to_string(),
            actor,
            detail,
        });

        Ok(())
    }

    /// Executes a simulated proof test on a designated SIF.
    pub fn execute_proof_test(
        &mut self,
        sif_id: &str,
        technician: impl Into<String>,
        sim_time: f64,
    ) -> Result<ProofTestRecord, String> {
        let tech_name = technician.into();
        let sif = self
            .sifs
            .get(sif_id)
            .ok_or_else(|| format!("SIF '{}' not found in Safety Instrumented System", sif_id))?;

        let record = run_simulated_proof_test(sif, tech_name.clone(), sim_time);
        self.proof_tests.push(record.clone());

        self.audit_trail.push(SafetyAuditEvent {
            timestamp_sim: sim_time,
            event_type: "PROOF_TEST".to_string(),
            sif_id: sif_id.to_string(),
            actor: tech_name,
            detail: format!(
                "Proof test {}: overall verdict = {}",
                record.test_id,
                if record.passed { "PASS" } else { "FAIL" }
            ),
        });

        Ok(record)
    }

    /// Convenience alias for execute_proof_test.
    pub fn run_proof_test(
        &mut self,
        sif_id: &str,
        technician: impl Into<String>,
        sim_time: f64,
    ) -> Result<ProofTestRecord, String> {
        self.execute_proof_test(sif_id, technician, sim_time)
    }

    /// Generates the live Cause and Effect matrix for the safety system.
    pub fn cause_effect_matrix(&self) -> CauseEffectMatrix {
        let sifs_vec: Vec<SafetyInstrumentedFunction> = self.sifs.values().cloned().collect();
        CauseEffectMatrix::from_sifs(&sifs_vec)
    }

    /// Deterministic 50 Hz simulation tick execution.
    ///
    /// Evaluates all SIFs and appends safety override actuator commands.
    pub fn step(
        &mut self,
        dt: f64,
        sim_time: f64,
        io_image: &IoImageTable,
        actuator_commands: &mut Vec<(String, ActuatorCommand)>,
    ) {
        if !self.enabled {
            return;
        }

        for sif in self.sifs.values_mut() {
            let was_tripped = sif.is_tripped();
            sif.step(dt, sim_time, io_image, actuator_commands);
            if !was_tripped && sif.is_tripped() {
                // SIF transitioned to Tripped on this tick
                let cause = sif
                    .diagnostics()
                    .last_trip_cause
                    .clone()
                    .unwrap_or_else(|| "Limit exceeded".to_string());
                self.audit_trail.push(SafetyAuditEvent {
                    timestamp_sim: sim_time,
                    event_type: "SIF_TRIP".to_string(),
                    sif_id: sif.id().to_string(),
                    actor: "SIS_LOGIC_SOLVER".to_string(),
                    detail: format!("Safety trip activated: {}", cause),
                });
            }
        }
    }
}
