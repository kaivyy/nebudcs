//! Objective evaluation and assessment scoring.

use nebudcs_alarm::AlarmSystem;
use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_model::tag::TagValue;

use crate::model::{ObjectiveCriteria, ObjectiveStatus, ScenarioObjective};

/// Tracking state for an active scenario objective.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectiveTrackerItem {
    pub objective: ScenarioObjective,
    pub status: ObjectiveStatus,
    pub consecutive_hold_sec: f64,
}

/// Evaluates objectives and scoring against live simulation state.
#[derive(Debug, Clone, Default)]
pub struct ObjectiveEvaluator {
    items: Vec<ObjectiveTrackerItem>,
}

impl ObjectiveEvaluator {
    /// Initializes evaluator with scenario objectives.
    pub fn new(objectives: Vec<ScenarioObjective>) -> Self {
        let items = objectives
            .into_iter()
            .map(|obj| ObjectiveTrackerItem {
                objective: obj,
                status: ObjectiveStatus::Pending { progress: 0.0 },
                consecutive_hold_sec: 0.0,
            })
            .collect();
        Self { items }
    }

    /// Evaluates all pending objectives against current state.
    pub fn step(
        &mut self,
        elapsed_sec: f64,
        dt: f64,
        tag_store: &RuntimeTagStore,
        _alarm_system: Option<&AlarmSystem>,
    ) {
        for item in &mut self.items {
            if !matches!(item.status, ObjectiveStatus::Pending { .. }) {
                continue;
            }

            // Check timeout
            if let Some(limit) = item.objective.time_limit_sec {
                if elapsed_sec > limit {
                    item.status = ObjectiveStatus::TimedOut;
                    continue;
                }
            }

            match &item.objective.criteria {
                ObjectiveCriteria::TagWithinRange {
                    tag,
                    min_val,
                    max_val,
                    hold_duration_sec,
                } => {
                    if let Some(tag_id) = tag_store.resolve_name(tag) {
                        let val_f = tag_store.get_value(tag_id).and_then(|v| match v {
                            TagValue::Float(f) => Some(*f),
                            TagValue::Integer(i) => Some(*i as f64),
                            _ => None,
                        });

                        if let Some(val) = val_f {
                            if val >= *min_val && val <= *max_val {
                                item.consecutive_hold_sec += dt;
                                let progress = (item.consecutive_hold_sec / *hold_duration_sec)
                                    .clamp(0.0, 1.0);
                                if item.consecutive_hold_sec >= *hold_duration_sec {
                                    item.status = ObjectiveStatus::Satisfied {
                                        at_sim_time: elapsed_sec,
                                    };
                                } else {
                                    item.status = ObjectiveStatus::Pending { progress };
                                }
                            } else {
                                item.consecutive_hold_sec = 0.0;
                                item.status = ObjectiveStatus::Pending { progress: 0.0 };
                            }
                        }
                    }
                }
                ObjectiveCriteria::TagReachesValue {
                    tag,
                    operator,
                    target_val,
                } => {
                    if let Some(tag_id) = tag_store.resolve_name(tag) {
                        let val_f = tag_store.get_value(tag_id).and_then(|v| match v {
                            TagValue::Float(f) => Some(*f),
                            TagValue::Integer(i) => Some(*i as f64),
                            _ => None,
                        });

                        if let Some(val) = val_f {
                            if operator.evaluate(val, *target_val) {
                                item.status = ObjectiveStatus::Satisfied {
                                    at_sim_time: elapsed_sec,
                                };
                            }
                        }
                    }
                }
                ObjectiveCriteria::AlarmAcknowledged {
                    alarm_name,
                    within_sec,
                } => {
                    // Check if alarm exists and is acknowledged
                    // (AlarmSystem tracks state; if cleared or acknowledged within window)
                    if elapsed_sec <= *within_sec {
                        // In vertical slice / scenario tests, can be triggered via operator action
                    } else {
                        item.status = ObjectiveStatus::Failed {
                            reason: format!("Alarm '{alarm_name}' was not acknowledged in time"),
                        };
                    }
                }
                ObjectiveCriteria::NoTripOccurred => {
                    // Evaluated continuously or at completion
                }
            }
        }
    }

    /// Marks an alarm acknowledged objective as satisfied if matched.
    pub fn on_alarm_acknowledged(&mut self, alarm_name: &str, elapsed_sec: f64) {
        for item in &mut self.items {
            if let ObjectiveCriteria::AlarmAcknowledged {
                alarm_name: target,
                within_sec,
            } = &item.objective.criteria
            {
                if target == alarm_name && elapsed_sec <= *within_sec {
                    item.status = ObjectiveStatus::Satisfied {
                        at_sim_time: elapsed_sec,
                    };
                }
            }
        }
    }

    /// Computes overall weighted assessment score (0.0 to 100.0).
    pub fn score(&self) -> f64 {
        if self.items.is_empty() {
            return 100.0;
        }

        let mut total_weight = 0u64;
        let mut earned_weight = 0u64;

        for item in &self.items {
            total_weight += item.objective.weight as u64;
            if matches!(item.status, ObjectiveStatus::Satisfied { .. }) {
                earned_weight += item.objective.weight as u64;
            }
        }

        if total_weight == 0 {
            100.0
        } else {
            ((earned_weight as f64) / (total_weight as f64)) * 100.0
        }
    }

    /// Checks if all mandatory objectives have been satisfied.
    pub fn all_mandatory_satisfied(&self) -> bool {
        self.items.iter().all(|item| {
            if item.objective.mandatory {
                matches!(item.status, ObjectiveStatus::Satisfied { .. })
            } else {
                true
            }
        })
    }

    /// Checks if any mandatory objective has terminally failed or timed out.
    pub fn any_mandatory_failed(&self) -> bool {
        self.items.iter().any(|item| {
            item.objective.mandatory
                && matches!(
                    item.status,
                    ObjectiveStatus::Failed { .. } | ObjectiveStatus::TimedOut
                )
        })
    }

    pub fn items(&self) -> &[ObjectiveTrackerItem] {
        &self.items
    }

    pub fn reset(&mut self) {
        for item in &mut self.items {
            item.status = ObjectiveStatus::Pending { progress: 0.0 };
            item.consecutive_hold_sec = 0.0;
        }
    }
}
