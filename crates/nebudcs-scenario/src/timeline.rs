//! Deterministic event timeline scheduler.

use std::cmp::Ordering;
use std::collections::HashSet;

use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_model::tag::TagValue;

use crate::model::{EventTrigger, ScenarioAction, ScenarioEvent};

/// An event in the scheduler queue with deterministic sorting key.
#[derive(Debug, Clone, PartialEq)]
struct QueuedEvent {
    event: ScenarioEvent,
}

impl Eq for QueuedEvent {}

impl PartialOrd for QueuedEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueuedEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        let t1 = match &self.event.trigger {
            EventTrigger::Time { elapsed_sec } => *elapsed_sec,
            _ => f64::MAX,
        };
        let t2 = match &other.event.trigger {
            EventTrigger::Time { elapsed_sec } => *elapsed_sec,
            _ => f64::MAX,
        };

        // Deterministic ordering: time -> priority (descending) -> id
        t1.partial_cmp(&t2)
            .unwrap_or(Ordering::Equal)
            .then_with(|| other.event.priority.cmp(&self.event.priority))
            .then_with(|| self.event.id.cmp(&other.event.id))
    }
}

/// Manages and schedules timeline events deterministically against simulation time.
#[derive(Debug, Clone, Default)]
pub struct TimelineScheduler {
    all_events: Vec<ScenarioEvent>,
    fired_event_ids: HashSet<String>,
}

impl TimelineScheduler {
    /// Creates a new scheduler initialized with the scenario's events.
    pub fn new(events: Vec<ScenarioEvent>) -> Self {
        Self {
            all_events: events,
            fired_event_ids: HashSet::new(),
        }
    }

    /// Evaluates which events should fire at the current elapsed simulation time.
    ///
    /// Multiple events triggering at the exact same simulation step are returned
    /// in strictly deterministic order sorted by `(trigger_time, priority, event_id)`.
    pub fn step(
        &mut self,
        elapsed_sec: f64,
        tag_store: &RuntimeTagStore,
    ) -> Vec<(String, ScenarioAction)> {
        let mut due_events = Vec::new();

        for event in &self.all_events {
            if self.fired_event_ids.contains(&event.id) {
                continue;
            }

            let triggered = evaluate_event_trigger(&event.trigger, elapsed_sec, tag_store);

            if triggered {
                due_events.push(QueuedEvent {
                    event: event.clone(),
                });
            }
        }

        // Sort triggered events deterministically
        due_events.sort();

        let mut actions = Vec::with_capacity(due_events.len());
        for queued in due_events {
            self.fired_event_ids.insert(queued.event.id.clone());
            actions.push((queued.event.id, queued.event.action));
        }

        actions
    }

    /// Resets the scheduler, allowing all events to fire again from the beginning.
    pub fn reset(&mut self) {
        self.fired_event_ids.clear();
    }

    /// Returns the total number of events managed by this scheduler.
    pub fn len(&self) -> usize {
        self.all_events.len()
    }

    /// Returns the number of events that have already fired.
    pub fn fired_count(&self) -> usize {
        self.fired_event_ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.all_events.is_empty()
    }
}

/// Evaluates any supported EventTrigger against current simulation time and runtime tag store.
fn evaluate_event_trigger(
    trigger: &EventTrigger,
    elapsed_sec: f64,
    tag_store: &RuntimeTagStore,
) -> bool {
    match trigger {
        EventTrigger::Time { elapsed_sec: t } => elapsed_sec >= *t,
        EventTrigger::TagCondition {
            tag,
            operator,
            value,
        } => {
            if let Some(tag_id) = tag_store.resolve_name(tag) {
                if let Some(tag_val) = tag_store.get_value(tag_id) {
                    let curr_f = match tag_val {
                        TagValue::Float(f) => Some(*f),
                        TagValue::Integer(i) => Some(*i as f64),
                        TagValue::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
                        _ => None,
                    };
                    if let Some(val_f) = curr_f {
                        operator.evaluate(val_f, *value)
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        }
        EventTrigger::PvDeviation {
            tag,
            target,
            max_deviation,
        } => {
            if let Some(tag_id) = tag_store.resolve_name(tag) {
                if let Some(tag_val) = tag_store.get_value(tag_id) {
                    let curr_f = match tag_val {
                        TagValue::Float(f) => Some(*f),
                        TagValue::Integer(i) => Some(*i as f64),
                        _ => None,
                    };
                    if let Some(val_f) = curr_f {
                        (val_f - *target).abs() > *max_deviation
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        }
        EventTrigger::EquipmentState {
            equipment_name,
            is_running,
        } => {
            let status_tag = format!("{}.STATUS", equipment_name);
            let running_tag = format!("{}.RUNNING", equipment_name);
            let speed_tag = format!("{}.SPEED", equipment_name);

            let observed_running = [status_tag, running_tag, speed_tag].iter().any(|t| {
                if let Some(tag_id) = tag_store.resolve_name(t) {
                    if let Some(tag_val) = tag_store.get_value(tag_id) {
                        match tag_val {
                            TagValue::Bool(b) => *b,
                            TagValue::Float(f) => *f > 0.5,
                            TagValue::Integer(i) => *i > 0,
                            _ => false,
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            });

            observed_running == *is_running
        }
        EventTrigger::AlarmActive { alarm_name } => {
            let active_tag = format!("{}.ACTIVE", alarm_name);
            let state_tag = format!("{}.STATE", alarm_name);

            [active_tag, state_tag, alarm_name.clone()].iter().any(|t| {
                if let Some(tag_id) = tag_store.resolve_name(t) {
                    if let Some(tag_val) = tag_store.get_value(tag_id) {
                        match tag_val {
                            TagValue::Bool(b) => *b,
                            TagValue::Float(f) => *f > 0.5,
                            TagValue::Integer(i) => *i > 0,
                            _ => false,
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            })
        }
        EventTrigger::SisState { sif_id, is_tripped } => {
            let trip_tag = format!("{}.TRIPPED", sif_id);
            let status_tag = format!("{}.STATUS", sif_id);

            let observed_tripped = [trip_tag, status_tag].iter().any(|t| {
                if let Some(tag_id) = tag_store.resolve_name(t) {
                    if let Some(tag_val) = tag_store.get_value(tag_id) {
                        match tag_val {
                            TagValue::Bool(b) => *b,
                            TagValue::Float(f) => *f > 0.5,
                            TagValue::Integer(i) => *i > 0,
                            _ => false,
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            });

            observed_tripped == *is_tripped
        }
        EventTrigger::OperatorActionTrigger {
            target_entity,
            action_type,
        } => {
            let action_flag_tag = format!("{}.ACTION_{}", target_entity, action_type);
            if let Some(tag_id) = tag_store.resolve_name(&action_flag_tag) {
                if let Some(tag_val) = tag_store.get_value(tag_id) {
                    match tag_val {
                        TagValue::Bool(b) => *b,
                        TagValue::Float(f) => *f > 0.5,
                        _ => false,
                    }
                } else {
                    false
                }
            } else {
                false
            }
        }
        EventTrigger::CompoundAnd { triggers } => {
            !triggers.is_empty()
                && triggers
                    .iter()
                    .all(|tr| evaluate_event_trigger(tr, elapsed_sec, tag_store))
        }
        EventTrigger::CompoundOr { triggers } => triggers
            .iter()
            .any(|tr| evaluate_event_trigger(tr, elapsed_sec, tag_store)),
    }
}
