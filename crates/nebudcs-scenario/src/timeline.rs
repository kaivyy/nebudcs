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

            let triggered = match &event.trigger {
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
            };

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
