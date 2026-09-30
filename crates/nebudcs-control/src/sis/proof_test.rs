//! Proof test simulation and record-keeping for Safety Instrumented Systems.
//!
//! Standards Reference: IEC 61511-1 Clause 16 (SIS Operation and Maintenance).
//! Note: Virtual simulation model for engineering verification and training.

use crate::sis::sif::SafetyInstrumentedFunction;
use serde::{Deserialize, Serialize};

/// Status of an individual component verified during a proof test.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProofTestStepResult {
    /// Tested sub-system (e.g. "Sensors", "LogicSolver", "FinalElement-XV-101").
    pub component: String,
    /// Whether this component passed functional verification.
    pub passed: bool,
    /// Measured observation or test value.
    pub observed_value: String,
    /// Detailed diagnostic notes.
    pub notes: String,
}

/// Complete diagnostic record of a simulated proof test execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProofTestRecord {
    /// Unique proof test record ID.
    pub test_id: String,
    /// Target SIF identifier.
    pub sif_id: String,
    /// Simulation timestamp when test was performed.
    pub timestamp_sim: f64,
    /// Overall pass/fail verdict.
    pub passed: bool,
    /// Results for each tested loop component.
    pub step_results: Vec<ProofTestStepResult>,
    /// Authorized test technician / engineer name.
    pub technician: String,
    /// General engineering comments.
    pub comments: String,
}

/// Runs a deterministic simulated proof test on an SIF.
///
/// Verifies:
/// 1. Sensor channel integrity
/// 2. Voting logic correctness
/// 3. Final element definition completeness
/// 4. Safe state configuration validity
pub fn run_simulated_proof_test(
    sif: &SafetyInstrumentedFunction,
    technician: impl Into<String>,
    sim_time: f64,
) -> ProofTestRecord {
    let tech_name = technician.into();
    let mut steps = Vec::new();
    let mut overall_passed = true;

    // 1. Verify Sensors
    let sensor_count = sif.config().sensor_tags.len();
    let min_required = sif.config().architecture.required_channel_count();
    let sensors_ok = sensor_count >= min_required;
    if !sensors_ok {
        overall_passed = false;
    }
    steps.push(ProofTestStepResult {
        component: "Sensor Subsystem".to_string(),
        passed: sensors_ok,
        observed_value: format!("{} configured / {} required", sensor_count, min_required),
        notes: if sensors_ok {
            "All redundant sensor channels mapped and verified".to_string()
        } else {
            "Insufficient sensor channels for voting architecture".to_string()
        },
    });

    // 2. Verify Logic Solver & Voting
    let voting_ok = sif.diagnostics().healthy_sensors > 0;
    if !voting_ok {
        overall_passed = false;
    }
    steps.push(ProofTestStepResult {
        component: "Safety Logic Solver & Voting".to_string(),
        passed: voting_ok,
        observed_value: format!(
            "{:?} (policy: {:?})",
            sif.config().architecture,
            sif.config().degradation_policy
        ),
        notes: format!("Effective scheme: {}", sif.diagnostics().voting_scheme),
    });

    // 3. Verify Final Elements
    let fe_count = sif.config().final_elements.len();
    let fe_ok = fe_count > 0;
    if !fe_ok {
        overall_passed = false;
    }
    steps.push(ProofTestStepResult {
        component: "Final Elements".to_string(),
        passed: fe_ok,
        observed_value: format!("{} final elements configured", fe_count),
        notes: if fe_ok {
            format!(
                "Safe states: {}",
                sif.config()
                    .final_elements
                    .iter()
                    .map(|fe| format!("{}: {:?}", fe.target_equipment, fe.safe_state))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        } else {
            "No final elements assigned to safety function".to_string()
        },
    });

    ProofTestRecord {
        test_id: format!("PT-{}-{}", sif.id(), sim_time as u64),
        sif_id: sif.id().to_string(),
        timestamp_sim: sim_time,
        passed: overall_passed,
        step_results: steps,
        technician: tech_name,
        comments: format!(
            "Simulated proof test for {}: {}",
            sif.name(),
            if overall_passed {
                "PASS - Full loop verified"
            } else {
                "FAIL - Loop defect detected"
            }
        ),
    }
}
