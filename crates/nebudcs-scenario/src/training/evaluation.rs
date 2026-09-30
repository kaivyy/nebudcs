//! Deterministic, configuration-driven trainee evaluation and explainable grading.

use nebudcs_historian::HistoricalEvent;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::action::{
    ActionStatus, OperatorErrorClassification, TraineeActionRecord, TraineeActionType,
};
use super::session::TrainingSession;

/// Safety trip policy defining how an SIF trip impacts the trainee's assessment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SafetyTripPolicy {
    /// Trip was expected / prescribed in this scenario step (e.g. testing emergency shutdown).
    Expected,
    /// Trip should have been prevented by proper operator intervention; counts as a failure.
    Avoidable,
    /// Trip is acceptable under extreme conditions without failure penalty.
    Acceptable,
    /// Any trip constitutes complete scenario failure.
    Failure,
}

/// Criticality severity level for evaluation criteria.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CriterionSeverity {
    Advisory,
    Standard,
    Critical,
    Fatal,
}

/// Supported evaluation condition rules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CriterionKind {
    /// Alarm must be acknowledged within `max_allowed_response_sec` after triggering.
    AlarmResponseTime {
        alarm_name: String,
        max_allowed_response_sec: f64,
    },
    /// Trainee must execute a specific action on a target entity.
    RequiredAction {
        target_entity: String,
        action_type: String,
        expected_value: Option<f64>,
        tolerance: Option<f64>,
        within_sec: Option<f64>,
    },
    /// Mandatory critical action; failure severely penalizes or fails the entire session.
    CriticalAction {
        target_entity: String,
        action_type: String,
        expected_value: Option<f64>,
        tolerance: Option<f64>,
        within_sec: Option<f64>,
    },
    /// Trainee must NOT execute this action (e.g. emergency override, illegal bypass).
    ForbiddenAction {
        target_entity: String,
        action_type: String,
    },
    /// Controller must be placed into the designated mode ("MAN", "AUTO", "CAS").
    CorrectControlMode {
        loop_name: String,
        required_mode: String,
    },
    /// Controller setpoint must match target value within tolerance.
    CorrectSetpoint {
        loop_name: String,
        target_sp: f64,
        tolerance: f64,
    },
    /// Equipment must end up in the desired running or stopped state.
    CorrectEquipmentState {
        equipment_name: String,
        must_be_running: bool,
    },
    /// Process tag must enter `[min_val, max_val]` and stay within it for `hold_duration_sec`.
    SafeRecovery {
        tag: String,
        min_val: f64,
        max_val: f64,
        hold_duration_sec: f64,
    },
    /// No equipment trip or emergency ESD trip must occur throughout session.
    NoTripOccurred,
    /// Tag must not exceed high boundary or breach low boundary during session.
    TagLimitBound {
        tag: String,
        limit: f64,
        is_upper: bool,
    },
    /// SIF safety trip policy evaluation.
    SifTripAssessment {
        sif_id: String,
        policy: SafetyTripPolicy,
    },
    /// Verify operational separation: BPCS alarm must precede SIS emergency trip.
    OperationalSeparationCheck { alarm_name: String, sif_id: String },
    /// Startup sequence step verification with optional timeout.
    StartupSequenceStep {
        step_name: String,
        order: u32,
        timeout_sec: Option<f64>,
    },
    /// Tag must remain strictly within specified operating band.
    TagWithinBand {
        tag: String,
        min_val: f64,
        max_val: f64,
    },
}

/// Authoritative declarative evaluation criterion specification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationCriterion {
    pub criterion_id: String,
    pub title: String,
    pub description: String,
    pub kind: CriterionKind,
    /// If true, failing this criterion automatically sets session report to overall FAIL.
    #[serde(default = "default_true")]
    pub required: bool,
    /// Score points awarded upon passing.
    #[serde(default = "default_weight")]
    pub weight: u32,
    /// Penalty points deducted upon failure.
    #[serde(default)]
    pub penalty: u32,
    /// Criticality severity.
    #[serde(default = "default_severity")]
    pub severity: CriterionSeverity,
}

fn default_true() -> bool {
    true
}

fn default_weight() -> u32 {
    100
}

fn default_severity() -> CriterionSeverity {
    CriterionSeverity::Standard
}

/// Structured training debrief summarizing performance, safety adherence, and recommendations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingDebrief {
    pub process_performance: String,
    pub operator_performance: String,
    pub safety_assessment: String,
    pub outcome_verdict: String,
    pub recommendations: Vec<String>,
}

/// Discrete outcome of a single evaluation criterion evaluation with structured evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CriterionResult {
    pub criterion_id: String,
    pub title: String,
    pub passed: bool,
    pub score_awarded: u32,
    pub score_possible: u32,
    pub penalty_deducted: u32,
    pub explanation: String,
    pub evidence: BTreeMap<String, String>,
    #[serde(default)]
    pub error_classification: Option<OperatorErrorClassification>,
    #[serde(default)]
    pub safety_trip_assessment: Option<String>,
}

/// Comprehensive, explainable trainee assessment report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationReport {
    pub session_id: String,
    pub scenario_id: String,
    pub trainee_id: String,
    pub total_score: u32,
    pub max_possible_score: u32,
    pub percentage: f64,
    pub passed: bool,
    pub results: Vec<CriterionResult>,
    pub evaluated_at_ms: i64,
    #[serde(default)]
    pub debrief: Option<TrainingDebrief>,
}

/// Evaluates all session criteria against authoritative simulation evidence.
pub fn evaluate_session(
    session: &TrainingSession,
    criteria: &[EvaluationCriterion],
    actions: &[TraineeActionRecord],
    events: &[HistoricalEvent],
    final_tags: &BTreeMap<String, f64>,
    samples: &[(f64, String, f64)], // (sim_time, tag, value)
    evaluation_wall_clock_ms: i64,
) -> EvaluationReport {
    let mut results = Vec::new();
    let mut total_score: u32 = 0;
    let mut max_possible_score: u32 = 0;
    let mut all_mandatory_passed = true;

    for crit in criteria {
        max_possible_score += crit.weight;
        let mut evidence = BTreeMap::new();
        let mut error_classification: Option<OperatorErrorClassification> = None;
        let mut safety_trip_assessment: Option<String> = None;

        let (passed, explanation) = match &crit.kind {
            CriterionKind::AlarmResponseTime {
                alarm_name,
                max_allowed_response_sec,
            } => {
                let alarm_event = events.iter().find(|e| {
                    e.event_type == "ALARM"
                        && e.source_id == *alarm_name
                        && (e.new_state == "ACTIVE" || e.new_state == "UNACK")
                });

                if let Some(trigger) = alarm_event {
                    let trigger_time = trigger.sim_time.seconds();
                    evidence.insert(
                        "alarm_triggered_sim_time_sec".to_string(),
                        format!("{:.3}", trigger_time),
                    );
                    evidence.insert(
                        "max_allowed_response_sec".to_string(),
                        format!("{:.1}", max_allowed_response_sec),
                    );

                    let ack_action = actions.iter().find(|a| {
                        a.status == ActionStatus::Accepted
                            && a.target_entity == *alarm_name
                            && a.action_type == "ACK_ALARM"
                            && a.sim_time_sec >= trigger_time
                    });

                    if let Some(ack) = ack_action {
                        let response_time = ack.sim_time_sec - trigger_time;
                        evidence.insert(
                            "ack_sim_time_sec".to_string(),
                            format!("{:.3}", ack.sim_time_sec),
                        );
                        evidence.insert(
                            "response_time_sec".to_string(),
                            format!("{:.3}", response_time),
                        );

                        if response_time <= *max_allowed_response_sec {
                            (
                                true,
                                format!(
                                    "Alarm '{}' acknowledged in {:.2}s (allowed: {:.1}s)",
                                    alarm_name, response_time, max_allowed_response_sec
                                ),
                            )
                        } else {
                            (
                                false,
                                format!(
                                    "Alarm '{}' acknowledged late in {:.2}s (exceeded limit of {:.1}s)",
                                    alarm_name, response_time, max_allowed_response_sec
                                ),
                            )
                        }
                    } else {
                        (
                            false,
                            format!(
                                "Alarm '{}' was triggered but never acknowledged by trainee",
                                alarm_name
                            ),
                        )
                    }
                } else {
                    evidence.insert(
                        "status".to_string(),
                        "Alarm remained clear throughout session".to_string(),
                    );
                    (
                        true,
                        format!("Alarm '{}' did not activate during the session", alarm_name),
                    )
                }
            }

            CriterionKind::RequiredAction {
                target_entity,
                action_type,
                expected_value,
                tolerance,
                within_sec,
            }
            | CriterionKind::CriticalAction {
                target_entity,
                action_type,
                expected_value,
                tolerance,
                within_sec,
            } => {
                evidence.insert("target_entity".to_string(), target_entity.clone());
                evidence.insert("expected_action_type".to_string(), action_type.clone());

                let matching_action = actions.iter().find(|a| {
                    if a.status != ActionStatus::Accepted {
                        return false;
                    }
                    if a.target_entity != *target_entity || a.action_type != *action_type {
                        return false;
                    }
                    if let Some(limit) = within_sec {
                        if (a.sim_time_sec - session.simulation_start_time_sec) > *limit {
                            return false;
                        }
                    }
                    if let Some(expected) = expected_value {
                        let tol = tolerance.unwrap_or(1e-4);
                        if let Some(val) = a.requested_value {
                            (val - expected).abs() <= tol
                        } else {
                            false
                        }
                    } else {
                        true
                    }
                });

                if let Some(act) = matching_action {
                    evidence.insert(
                        "action_sim_time_sec".to_string(),
                        format!("{:.3}", act.sim_time_sec),
                    );
                    if let Some(val) = act.requested_value {
                        evidence.insert("submitted_value".to_string(), format!("{:.4}", val));
                    }
                    (
                        true,
                        format!(
                            "Required action '{}' on '{}' successfully executed at t={:.2}s",
                            action_type, target_entity, act.sim_time_sec
                        ),
                    )
                } else {
                    (
                        false,
                        format!(
                            "Required action '{}' on '{}' was not performed",
                            action_type, target_entity
                        ),
                    )
                }
            }

            CriterionKind::ForbiddenAction {
                target_entity,
                action_type,
            } => {
                evidence.insert("forbidden_target".to_string(), target_entity.clone());
                evidence.insert("forbidden_action".to_string(), action_type.clone());

                let violation = actions.iter().find(|a| {
                    a.status == ActionStatus::Accepted
                        && a.target_entity == *target_entity
                        && a.action_type == *action_type
                });

                if let Some(viol) = violation {
                    evidence.insert(
                        "violation_sim_time_sec".to_string(),
                        format!("{:.3}", viol.sim_time_sec),
                    );
                    (
                        false,
                        format!(
                            "Forbidden action '{}' on '{}' detected at t={:.2}s",
                            action_type, target_entity, viol.sim_time_sec
                        ),
                    )
                } else {
                    (
                        true,
                        format!(
                            "No forbidden action '{}' on '{}' was committed",
                            action_type, target_entity
                        ),
                    )
                }
            }

            CriterionKind::CorrectControlMode {
                loop_name,
                required_mode,
            } => {
                evidence.insert("loop_name".to_string(), loop_name.clone());
                evidence.insert("required_mode".to_string(), required_mode.clone());

                let mode_tag = format!("{}.MODE", loop_name);
                let latest_mode_action = actions.iter().rfind(|a| {
                    a.status == ActionStatus::Accepted
                        && a.target_entity == *loop_name
                        && a.action_type == "SET_MODE"
                });

                let current_mode = if let Some(act) = latest_mode_action {
                    match &act.action {
                        super::action::TraineeActionType::SetMode { mode, .. } => mode.clone(),
                        _ => String::new(),
                    }
                } else if let Some(&mode_val) = final_tags.get(&mode_tag) {
                    match mode_val as i64 {
                        0 => "MAN".to_string(),
                        1 => "AUTO".to_string(),
                        2 => "CAS".to_string(),
                        3 => "RCAS".to_string(),
                        _ => "UNKNOWN".to_string(),
                    }
                } else {
                    "UNKNOWN".to_string()
                };

                evidence.insert("observed_mode".to_string(), current_mode.clone());
                if current_mode.eq_ignore_ascii_case(required_mode) {
                    (
                        true,
                        format!(
                            "Controller '{}' is in required mode '{}'",
                            loop_name, required_mode
                        ),
                    )
                } else {
                    (
                        false,
                        format!(
                            "Controller '{}' mode is '{}', expected '{}'",
                            loop_name, current_mode, required_mode
                        ),
                    )
                }
            }

            CriterionKind::CorrectSetpoint {
                loop_name,
                target_sp,
                tolerance,
            } => {
                evidence.insert("loop_name".to_string(), loop_name.clone());
                evidence.insert("target_sp".to_string(), format!("{:.4}", target_sp));
                evidence.insert("tolerance".to_string(), format!("{:.4}", tolerance));

                let sp_tag = format!("{}.SP", loop_name);
                let actual_sp = final_tags.get(&sp_tag).copied().or_else(|| {
                    actions
                        .iter()
                        .filter_map(|a| {
                            if a.status == ActionStatus::Accepted
                                && a.target_entity == *loop_name
                                && a.action_type == "SET_SETPOINT"
                            {
                                a.requested_value
                            } else {
                                None
                            }
                        })
                        .next_back()
                });

                if let Some(val) = actual_sp {
                    evidence.insert("observed_sp".to_string(), format!("{:.4}", val));
                    if (val - target_sp).abs() <= *tolerance {
                        (
                            true,
                            format!(
                                "Setpoint for '{}' is {:.2} (within tolerance of {:.2})",
                                loop_name, val, target_sp
                            ),
                        )
                    } else {
                        (
                            false,
                            format!("Setpoint for '{}' is {:.2}, deviation from target {:.2} exceeds tolerance {:.2}", loop_name, val, target_sp, tolerance),
                        )
                    }
                } else {
                    (
                        false,
                        format!(
                            "Unable to determine final setpoint for controller '{}'",
                            loop_name
                        ),
                    )
                }
            }

            CriterionKind::CorrectEquipmentState {
                equipment_name,
                must_be_running,
            } => {
                let status_tag = format!("{}.STATUS", equipment_name);
                let speed_tag = format!("{}.SPEED", equipment_name);

                let is_running = final_tags
                    .get(&status_tag)
                    .map(|&v| v > 0.5)
                    .or_else(|| final_tags.get(&speed_tag).map(|&s| s > 50.0))
                    .unwrap_or(false);

                evidence.insert("equipment_name".to_string(), equipment_name.clone());
                evidence.insert("must_be_running".to_string(), must_be_running.to_string());
                evidence.insert("observed_running".to_string(), is_running.to_string());

                if is_running == *must_be_running {
                    (
                        true,
                        format!(
                            "Equipment '{}' is correctly {}",
                            equipment_name,
                            if *must_be_running {
                                "running"
                            } else {
                                "stopped"
                            }
                        ),
                    )
                } else {
                    (
                        false,
                        format!(
                            "Equipment '{}' was expected to be {}, but was {}",
                            equipment_name,
                            if *must_be_running {
                                "running"
                            } else {
                                "stopped"
                            },
                            if is_running { "running" } else { "stopped" }
                        ),
                    )
                }
            }

            CriterionKind::SafeRecovery {
                tag,
                min_val,
                max_val,
                hold_duration_sec,
            } => {
                evidence.insert("tag".to_string(), tag.clone());
                evidence.insert(
                    "allowed_range".to_string(),
                    format!("[{:.2}, {:.2}]", min_val, max_val),
                );
                evidence.insert(
                    "required_hold_sec".to_string(),
                    format!("{:.1}", hold_duration_sec),
                );

                let mut tag_samples: Vec<(f64, f64)> = samples
                    .iter()
                    .filter(|(_, t, _)| t == tag)
                    .map(|(time, _, val)| (*time, *val))
                    .collect();
                tag_samples
                    .sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

                if tag_samples.is_empty() {
                    if let Some(&final_val) = final_tags.get(tag) {
                        if final_val >= *min_val && final_val <= *max_val {
                            (
                                true,
                                format!(
                                    "Tag '{}' final value {:.2} is within recovery range",
                                    tag, final_val
                                ),
                            )
                        } else {
                            (
                                false,
                                format!(
                                    "Tag '{}' final value {:.2} is outside recovery range",
                                    tag, final_val
                                ),
                            )
                        }
                    } else {
                        (
                            false,
                            format!("No telemetry samples recorded for recovery tag '{}'", tag),
                        )
                    }
                } else {
                    let final_time = tag_samples.last().unwrap().0;
                    let window_start = (final_time - hold_duration_sec).max(0.0);
                    let samples_in_window: Vec<&(f64, f64)> = tag_samples
                        .iter()
                        .filter(|(t, _)| *t >= window_start)
                        .collect();

                    let all_in_range = !samples_in_window.is_empty()
                        && samples_in_window
                            .iter()
                            .all(|(_, v)| *v >= *min_val && *v <= *max_val);
                    if all_in_range && (final_time - window_start) >= (*hold_duration_sec * 0.9) {
                        (
                            true,
                            format!("Tag '{}' maintained within [{:.2}, {:.2}] for required hold duration", tag, min_val, max_val),
                        )
                    } else {
                        (
                            false,
                            format!(
                                "Tag '{}' failed to remain continuously within recovery envelope",
                                tag
                            ),
                        )
                    }
                }
            }

            CriterionKind::NoTripOccurred => {
                let trips: Vec<&HistoricalEvent> =
                    events.iter().filter(|e| e.event_type == "TRIP").collect();

                if trips.is_empty() {
                    (
                        true,
                        "Plant operated cleanly with zero safety trips throughout session"
                            .to_string(),
                    )
                } else {
                    let tripped_sources: Vec<String> =
                        trips.iter().map(|t| t.source_id.clone()).collect();
                    evidence.insert("trip_count".to_string(), trips.len().to_string());
                    evidence.insert("tripped_sources".to_string(), tripped_sources.join(", "));
                    (
                        false,
                        format!(
                            "Protective trip(s) occurred on: {}",
                            tripped_sources.join(", ")
                        ),
                    )
                }
            }

            CriterionKind::TagLimitBound {
                tag,
                limit,
                is_upper,
            } => {
                evidence.insert("tag".to_string(), tag.clone());
                evidence.insert("limit".to_string(), format!("{:.4}", limit));
                evidence.insert("is_upper".to_string(), is_upper.to_string());

                let mut breached = false;

                for (time, t, val) in samples {
                    if t == tag && ((*is_upper && *val > *limit) || (!*is_upper && *val < *limit)) {
                        breached = true;
                        evidence.insert("breach_sim_time_sec".to_string(), format!("{:.3}", time));
                        evidence.insert("breach_value".to_string(), format!("{:.4}", val));
                        break;
                    }
                }

                if !breached {
                    (
                        true,
                        format!("Tag '{}' stayed within safety boundary {:.2}", tag, limit),
                    )
                } else {
                    (
                        false,
                        format!("Tag '{}' breached safety limit {:.2}", tag, limit),
                    )
                }
            }

            CriterionKind::SifTripAssessment { sif_id, policy } => {
                let sif_tripped = events.iter().any(|e| {
                    (e.event_type == "TRIP" || e.event_type == "SIS_TRIP")
                        && (e.source_id == *sif_id || e.source_id.contains(sif_id))
                });
                evidence.insert("sif_id".to_string(), sif_id.clone());
                evidence.insert("policy".to_string(), format!("{:?}", policy));
                evidence.insert("sif_tripped".to_string(), sif_tripped.to_string());

                let (p, expl, assess) = match policy {
                    SafetyTripPolicy::Expected => {
                        if sif_tripped {
                            (
                                true,
                                format!(
                                    "Expected safety trip on SIF '{}' executed correctly",
                                    sif_id
                                ),
                                "EXPECTED_TRIP_OCCURRED",
                            )
                        } else {
                            (
                                false,
                                format!("Expected safety trip on SIF '{}' failed to occur", sif_id),
                                "EXPECTED_TRIP_MISSED",
                            )
                        }
                    }
                    SafetyTripPolicy::Avoidable => {
                        if !sif_tripped {
                            (
                                true,
                                format!("Avoidable trip on SIF '{}' successfully averted", sif_id),
                                "AVOIDABLE_TRIP_PREVENTED",
                            )
                        } else {
                            (false, format!("Avoidable trip on SIF '{}' occurred due to late/missing operator action", sif_id), "AVOIDABLE_TRIP_UNPREVENTED")
                        }
                    }
                    SafetyTripPolicy::Acceptable => (
                        true,
                        format!(
                            "Trip on SIF '{}' is classified acceptable (tripped={})",
                            sif_id, sif_tripped
                        ),
                        if sif_tripped {
                            "ACCEPTABLE_TRIP_OCCURRED"
                        } else {
                            "NO_TRIP"
                        },
                    ),
                    SafetyTripPolicy::Failure => {
                        if !sif_tripped {
                            (
                                true,
                                format!("Zero-trip rule satisfied for SIF '{}'", sif_id),
                                "NO_TRIP",
                            )
                        } else {
                            (
                                false,
                                format!("Fatal safety trip occurred on SIF '{}'", sif_id),
                                "FATAL_TRIP_OCCURRED",
                            )
                        }
                    }
                };
                safety_trip_assessment = Some(assess.to_string());
                error_classification = if p {
                    Some(OperatorErrorClassification::Correct)
                } else {
                    Some(OperatorErrorClassification::Incorrect)
                };
                (p, expl)
            }

            CriterionKind::OperationalSeparationCheck { alarm_name, sif_id } => {
                let alarm_ev = events
                    .iter()
                    .find(|e| e.event_type == "ALARM" && e.source_id == *alarm_name);
                let trip_ev = events.iter().find(|e| {
                    (e.event_type == "TRIP" || e.event_type == "SIS_TRIP")
                        && (e.source_id == *sif_id || e.source_id.contains(sif_id))
                });

                evidence.insert("alarm_name".to_string(), alarm_name.clone());
                evidence.insert("sif_id".to_string(), sif_id.clone());

                match (alarm_ev, trip_ev) {
                    (Some(a), Some(t)) => {
                        let a_time = a.sim_time.seconds();
                        let t_time = t.sim_time.seconds();
                        evidence.insert("alarm_time_sec".to_string(), format!("{:.3}", a_time));
                        evidence.insert("trip_time_sec".to_string(), format!("{:.3}", t_time));
                        if a_time <= t_time {
                            (true, format!("Operational separation verified: BPCS alarm '{}' triggered at {:.2}s prior to SIS trip at {:.2}s", alarm_name, a_time, t_time))
                        } else {
                            (false, format!("Operational separation violated: SIS trip occurred at {:.2}s before BPCS alarm at {:.2}s", t_time, a_time))
                        }
                    }
                    (Some(a), None) => {
                        let a_time = a.sim_time.seconds();
                        evidence.insert("alarm_time_sec".to_string(), format!("{:.3}", a_time));
                        (true, format!("BPCS alarm '{}' triggered at {:.2}s with SIS remaining secure (no trip)", alarm_name, a_time))
                    }
                    (None, Some(t)) => {
                        let t_time = t.sim_time.seconds();
                        evidence.insert("trip_time_sec".to_string(), format!("{:.3}", t_time));
                        (false, format!("Operational separation failure: SIF '{}' tripped at {:.2}s without prior BPCS alarm '{}'", sif_id, t_time, alarm_name))
                    }
                    (None, None) => (
                        true,
                        "Both BPCS and SIS remained within normal non-alarm operational envelope"
                            .to_string(),
                    ),
                }
            }

            CriterionKind::StartupSequenceStep {
                step_name,
                order,
                timeout_sec,
            } => {
                evidence.insert("step_name".to_string(), step_name.clone());
                evidence.insert("required_order".to_string(), order.to_string());
                if let Some(to) = timeout_sec {
                    evidence.insert("timeout_sec".to_string(), format!("{:.1}", to));
                }

                let matching_action = actions.iter().find(|a| {
                    a.target_entity == *step_name
                        || a.action_type == *step_name
                        || match &a.action {
                            TraineeActionType::ProcedureStepAck { step_id } => step_id == step_name,
                            _ => false,
                        }
                });

                if let Some(act) = matching_action {
                    evidence.insert(
                        "executed_at_sec".to_string(),
                        format!("{:.3}", act.sim_time_sec),
                    );
                    if let Some(to) = timeout_sec {
                        if act.sim_time_sec <= *to {
                            (true, format!("Startup step '{}' (order {}) executed at {:.2}s within timeout {:.1}s", step_name, order, act.sim_time_sec, to))
                        } else {
                            error_classification = Some(OperatorErrorClassification::Late);
                            (
                                false,
                                format!(
                                    "Startup step '{}' executed at {:.2}s exceeding timeout {:.1}s",
                                    step_name, act.sim_time_sec, to
                                ),
                            )
                        }
                    } else {
                        (
                            true,
                            format!(
                                "Startup step '{}' (order {}) completed successfully at {:.2}s",
                                step_name, order, act.sim_time_sec
                            ),
                        )
                    }
                } else {
                    error_classification = Some(OperatorErrorClassification::Missing);
                    (
                        false,
                        format!(
                            "Startup sequence step '{}' was not performed by trainee",
                            step_name
                        ),
                    )
                }
            }

            CriterionKind::TagWithinBand {
                tag,
                min_val,
                max_val,
            } => {
                evidence.insert("tag".to_string(), tag.clone());
                evidence.insert(
                    "allowed_band".to_string(),
                    format!("[{:.2}, {:.2}]", min_val, max_val),
                );

                let mut out_of_band = false;
                let mut bad_sample = None;

                for (time, t, val) in samples {
                    if t == tag && (*val < *min_val || *val > *max_val) {
                        out_of_band = true;
                        bad_sample = Some((*time, *val));
                        break;
                    }
                }

                if !out_of_band {
                    if let Some(&final_val) = final_tags.get(tag) {
                        if final_val < *min_val || final_val > *max_val {
                            out_of_band = true;
                            bad_sample = Some((session.current_simulation_time_sec, final_val));
                        }
                    }
                }

                if !out_of_band {
                    (
                        true,
                        format!(
                            "Tag '{}' remained continuously within operating band [{:.2}, {:.2}]",
                            tag, min_val, max_val
                        ),
                    )
                } else {
                    let (t, v) = bad_sample.unwrap_or((0.0, 0.0));
                    evidence.insert("breach_time_sec".to_string(), format!("{:.3}", t));
                    evidence.insert("breach_value".to_string(), format!("{:.4}", v));
                    (false, format!("Tag '{}' breached operating band [{:.2}, {:.2}] with value {:.2} at {:.2}s", tag, min_val, max_val, v, t))
                }
            }
        };

        let score_awarded = if passed { crit.weight } else { 0 };
        let penalty_deducted = if !passed { crit.penalty } else { 0 };
        total_score = total_score
            .saturating_add(score_awarded)
            .saturating_sub(penalty_deducted);

        if crit.required && !passed {
            all_mandatory_passed = false;
        }

        if error_classification.is_none() {
            error_classification = match &crit.kind {
                CriterionKind::ForbiddenAction { .. } => {
                    if !passed {
                        Some(OperatorErrorClassification::Forbidden)
                    } else {
                        Some(OperatorErrorClassification::Correct)
                    }
                }
                CriterionKind::AlarmResponseTime { .. } => {
                    if !passed {
                        Some(OperatorErrorClassification::Late)
                    } else {
                        Some(OperatorErrorClassification::Correct)
                    }
                }
                CriterionKind::RequiredAction { .. } | CriterionKind::CriticalAction { .. } => {
                    if !passed {
                        Some(OperatorErrorClassification::Missing)
                    } else {
                        Some(OperatorErrorClassification::Correct)
                    }
                }
                CriterionKind::CorrectControlMode { .. }
                | CriterionKind::CorrectSetpoint { .. }
                | CriterionKind::CorrectEquipmentState { .. } => {
                    if !passed {
                        Some(OperatorErrorClassification::Incorrect)
                    } else {
                        Some(OperatorErrorClassification::Correct)
                    }
                }
                _ => {
                    if passed {
                        Some(OperatorErrorClassification::Correct)
                    } else {
                        Some(OperatorErrorClassification::Inefficient)
                    }
                }
            };
        }

        results.push(CriterionResult {
            criterion_id: crit.criterion_id.clone(),
            title: crit.title.clone(),
            passed,
            score_awarded,
            score_possible: crit.weight,
            penalty_deducted,
            explanation,
            evidence,
            error_classification,
            safety_trip_assessment,
        });
    }

    let percentage = if max_possible_score > 0 {
        (total_score as f64 / max_possible_score as f64) * 100.0
    } else {
        100.0
    };

    let overall_passed = all_mandatory_passed && percentage >= 70.0;

    let debrief = {
        let mut recommendations = Vec::new();
        for r in &results {
            if !r.passed {
                recommendations.push(format!("Criterion '{}': {}", r.title, r.explanation));
            }
        }
        if recommendations.is_empty() {
            recommendations.push(
                "Operations conformed to standard DCS operating procedures and safety envelopes."
                    .to_string(),
            );
        }

        let has_fatal_trip = results.iter().any(|r| {
            r.safety_trip_assessment.as_deref() == Some("AVOIDABLE_TRIP_UNPREVENTED")
                || r.safety_trip_assessment.as_deref() == Some("FATAL_TRIP_OCCURRED")
        });

        TrainingDebrief {
            process_performance: if overall_passed {
                format!(
                    "Process variables stabilized within normal operating envelope ({:.1}% score).",
                    percentage
                )
            } else {
                format!(
                    "Process instability observed; operational limits violated ({:.1}% score).",
                    percentage
                )
            },
            operator_performance: if all_mandatory_passed {
                "Operator executed procedural interventions within acceptable response windows."
                    .to_string()
            } else {
                "Operator missed one or more mandatory procedural steps or critical response deadlines.".to_string()
            },
            safety_assessment: if has_fatal_trip {
                "Safety barrier compromise: unprevented or fatal safety trip occurred.".to_string()
            } else {
                "Safety Instrumented System (SIS) integrity preserved with zero unprevented trips."
                    .to_string()
            },
            outcome_verdict: if overall_passed {
                "STAGE 11 OTS ASSESSMENT: SATISFACTORY / PASS".to_string()
            } else {
                "STAGE 11 OTS ASSESSMENT: UNSATISFACTORY / FAIL".to_string()
            },
            recommendations,
        }
    };

    EvaluationReport {
        session_id: session.id.clone(),
        scenario_id: session.scenario_id.clone(),
        trainee_id: session.trainee_id.clone(),
        total_score,
        max_possible_score,
        percentage: (percentage * 10.0).round() / 10.0,
        passed: overall_passed,
        results,
        evaluated_at_ms: evaluation_wall_clock_ms,
        debrief: Some(debrief),
    }
}
