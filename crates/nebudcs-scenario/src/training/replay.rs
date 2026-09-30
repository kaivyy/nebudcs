//! Read-only training session replay engine and chronological state reconstruction.

use nebudcs_historian::HistoricalEvent;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::action::TraineeActionRecord;
use super::session::TrainingSession;

/// Categorized replay marker types for timeline visualization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReplayMarkerKind {
    Alarm,
    Trip,
    Fault,
    OperatorAction,
    ScenarioEvent,
    Lifecycle,
}

/// Discrete chronological marker positioned along the session replay timeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayMarker {
    pub marker_id: String,
    pub sim_time_sec: f64,
    pub kind: ReplayMarkerKind,
    pub source_id: String,
    pub label: String,
    pub details: String,
    pub severity: u8,
}

/// Point-in-time state reconstruction frame for presentation on replay console.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayFrame {
    pub sim_time_sec: f64,
    pub tags: BTreeMap<String, f64>,
    pub active_alarms: Vec<String>,
    pub active_trips: Vec<String>,
    pub recent_actions: Vec<TraineeActionRecord>,
}

/// Consolidated, read-only session data package for historical playback.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplaySessionData {
    pub session_id: String,
    pub project_id: String,
    pub scenario_id: String,
    pub start_sim_time_sec: f64,
    pub end_sim_time_sec: f64,
    pub duration_sec: f64,
    pub tag_trajectories: BTreeMap<String, Vec<(f64, f64)>>,
    pub events: Vec<HistoricalEvent>,
    pub actions: Vec<TraineeActionRecord>,
    pub markers: Vec<ReplayMarker>,
}

impl ReplaySessionData {
    /// Builds an immutable replay session dataset from recorded simulation evidence.
    pub fn build(
        session: &TrainingSession,
        samples: &[(f64, String, f64)], // (sim_time, tag, value)
        events: &[HistoricalEvent],
        actions: &[TraineeActionRecord],
    ) -> Self {
        let start_sim_time_sec = session.simulation_start_time_sec;
        let end_sim_time_sec = session.current_simulation_time_sec.max(start_sim_time_sec);
        let duration_sec = end_sim_time_sec - start_sim_time_sec;

        // 1. Group samples by tag and ensure sorted by simulation timestamp
        let mut tag_trajectories: BTreeMap<String, Vec<(f64, f64)>> = BTreeMap::new();
        for (time, tag, val) in samples {
            tag_trajectories
                .entry(tag.clone())
                .or_default()
                .push((*time, *val));
        }
        for traj in tag_trajectories.values_mut() {
            traj.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        }

        // 2. Synthesize replay markers from events and operator actions
        let mut markers = Vec::new();
        let mut marker_counter = 0;

        for evt in events {
            marker_counter += 1;
            let kind = match evt.event_type.as_str() {
                "ALARM" => ReplayMarkerKind::Alarm,
                "TRIP" => ReplayMarkerKind::Trip,
                "FAULT" => ReplayMarkerKind::Fault,
                "MARKER" => ReplayMarkerKind::ScenarioEvent,
                _ => ReplayMarkerKind::Lifecycle,
            };

            markers.push(ReplayMarker {
                marker_id: format!("marker-evt-{}", marker_counter),
                sim_time_sec: evt.sim_time.seconds(),
                kind,
                source_id: evt.source_id.clone(),
                label: format!("{}: {}", evt.event_type, evt.source_id),
                details: format!(
                    "State: {} -> {}",
                    evt.old_state.as_deref().unwrap_or("INIT"),
                    evt.new_state
                ),
                severity: evt.severity.unwrap_or(1),
            });
        }

        for act in actions {
            marker_counter += 1;
            markers.push(ReplayMarker {
                marker_id: format!("marker-act-{}", marker_counter),
                sim_time_sec: act.sim_time_sec,
                kind: ReplayMarkerKind::OperatorAction,
                source_id: act.target_entity.clone(),
                label: format!("Action: {}", act.action_type),
                details: format!("Trainee {} commanded {}", act.actor, act.target_entity),
                severity: 2,
            });
        }

        markers.sort_by(|a, b| {
            a.sim_time_sec
                .partial_cmp(&b.sim_time_sec)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Self {
            session_id: session.id.clone(),
            project_id: session.project_id.clone(),
            scenario_id: session.scenario_id.clone(),
            start_sim_time_sec,
            end_sim_time_sec,
            duration_sec,
            tag_trajectories,
            events: events.to_vec(),
            actions: actions.to_vec(),
            markers,
        }
    }

    /// Reconstructs the plant telemetry, alarms, trips, and recent actions at requested simulation time.
    ///
    /// This query is strictly read-only and does NOT interact with running simulation state.
    pub fn frame_at(&self, query_sim_time: f64) -> ReplayFrame {
        let t = query_sim_time.clamp(self.start_sim_time_sec, self.end_sim_time_sec);

        // 1. Reconstruct tag values via nearest/piecewise interpolation
        let mut tags = BTreeMap::new();
        for (tag_name, points) in &self.tag_trajectories {
            if points.is_empty() {
                continue;
            }
            if points.len() == 1 || t <= points[0].0 {
                tags.insert(tag_name.clone(), points[0].1);
            } else if t >= points.last().unwrap().0 {
                tags.insert(tag_name.clone(), points.last().unwrap().1);
            } else {
                // Binary search for interval
                let idx = match points.binary_search_by(|p| p.0.partial_cmp(&t).unwrap()) {
                    Ok(exact) => exact,
                    Err(insertion) => insertion.saturating_sub(1),
                };
                let p1 = &points[idx];
                let p2 = &points[(idx + 1).min(points.len() - 1)];

                if (p2.0 - p1.0).abs() < 1e-6 {
                    tags.insert(tag_name.clone(), p1.1);
                } else {
                    let fraction = (t - p1.0) / (p2.0 - p1.0);
                    let val = p1.1 + fraction * (p2.1 - p1.1);
                    tags.insert(tag_name.clone(), val);
                }
            }
        }

        // 2. Active alarms at time t: track state transitions chronologically up to t
        let mut alarm_states: BTreeMap<String, String> = BTreeMap::new();
        for evt in &self.events {
            if evt.event_type == "ALARM" && evt.sim_time.seconds() <= t {
                alarm_states.insert(evt.source_id.clone(), evt.new_state.clone());
            }
        }
        let active_alarms: Vec<String> = alarm_states
            .into_iter()
            .filter(|(_, state)| state == "ACTIVE" || state == "UNACK")
            .map(|(name, _)| name)
            .collect();

        // 3. Active trips at time t
        let mut trip_states: BTreeMap<String, bool> = BTreeMap::new();
        for evt in &self.events {
            if evt.sim_time.seconds() <= t {
                if evt.event_type == "TRIP" {
                    trip_states.insert(evt.source_id.clone(), true);
                } else if evt.event_type == "RESET" {
                    trip_states.insert(evt.source_id.clone(), false);
                }
            }
        }
        let active_trips: Vec<String> = trip_states
            .into_iter()
            .filter(|(_, active)| *active)
            .map(|(name, _)| name)
            .collect();

        // 4. Actions occurring within a 5-second lookback window [t - 5.0, t]
        let window_start = (t - 5.0).max(0.0);
        let recent_actions: Vec<TraineeActionRecord> = self
            .actions
            .iter()
            .filter(|a| a.sim_time_sec >= window_start && a.sim_time_sec <= t)
            .cloned()
            .collect();

        ReplayFrame {
            sim_time_sec: t,
            tags,
            active_alarms,
            active_trips,
            recent_actions,
        }
    }

    /// Queries all timeline markers falling within a time window.
    pub fn markers_in_range(&self, start: f64, end: f64) -> Vec<&ReplayMarker> {
        self.markers
            .iter()
            .filter(|m| m.sim_time_sec >= start && m.sim_time_sec <= end)
            .collect()
    }
}
