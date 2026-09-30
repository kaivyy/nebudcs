//! Training manager subsystem orchestrating training sessions, trainee action recording,
//! deterministic grading, and read-only replay dataset production.

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use nebudcs_historian::HistoricalEvent;
use nebudcs_protocol::{
    CreateTrainingSessionRequest, CriterionResultDto, EvaluationReportDto,
    OperatorErrorClassificationDto, ReplayFrameDto, ReplayMarkerDto, ReplaySessionSummaryDto,
    ScenarioAuthoringDto, ScenarioBranchDto, TraineeActionDto, TrainingDebriefDto,
    TrainingSessionDetailDto, TrainingSessionSummaryDto,
};
use nebudcs_scenario::training::action::{
    ActionStatus, OperatorErrorClassification, TraineeActionRecord, TraineeActionType,
};
use nebudcs_scenario::training::evaluation::{evaluate_session, EvaluationCriterion};
use nebudcs_scenario::training::replay::ReplaySessionData;
use nebudcs_scenario::training::session::{SessionLifecycleState, TrainingSession};

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Converts a TrainingSession to a TrainingSessionDetailDto without requiring self.
fn session_to_dto(session: &TrainingSession) -> TrainingSessionDetailDto {
    TrainingSessionDetailDto {
        id: session.id.clone(),
        name: session.name.clone(),
        description: session.description.clone(),
        project_id: session.project_id.clone(),
        scenario_id: session.scenario_id.clone(),
        instructor_id: session.instructor_id.clone(),
        trainee_id: session.trainee_id.clone(),
        state: session.state.to_string(),
        created_at_ms: session.created_at_ms,
        started_at_ms: session.started_at_ms,
        completed_at_ms: session.completed_at_ms,
        simulation_start_time_sec: session.simulation_start_time_sec,
        current_simulation_time_sec: session.current_simulation_time_sec,
        abort_reason: session.abort_reason.clone(),
        scenario_version: session.scenario_version.clone(),
        runtime_snapshot: session.runtime_snapshot.clone(),
        score: session.score,
        completion_state: session.completion_state.clone(),
        metadata: session.metadata.clone(),
    }
}

/// An active or completed training session managed by the server.
#[derive(Debug, Clone)]
pub struct ManagedSession {
    pub session: TrainingSession,
    pub scenario: Option<ScenarioAuthoringDto>,
    pub criteria: Vec<EvaluationCriterion>,
    pub actions: Vec<TraineeActionRecord>,
    pub samples: Vec<(f64, String, f64)>, // (sim_time, tag, value)
    pub events: Vec<HistoricalEvent>,
    pub evaluation_report: Option<EvaluationReportDto>,
    pub replay_data: Option<ReplaySessionData>,
}

/// Persisted training session record for historical query, audit, and replay recovery.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PersistedTrainingRecord {
    pub session: TrainingSession,
    pub scenario: Option<ScenarioAuthoringDto>,
    pub criteria: Vec<EvaluationCriterion>,
    pub actions: Vec<TraineeActionRecord>,
    pub samples: Vec<(f64, String, f64)>,
    pub events: Vec<HistoricalEvent>,
    pub evaluation_report: Option<EvaluationReportDto>,
    pub replay_data: Option<ReplaySessionData>,
}

/// Thread-safe training manager maintaining active training sessions and historical records.
#[derive(Debug, Clone)]
pub struct TrainingManager {
    sessions: BTreeMap<String, ManagedSession>,
    disk_store: Option<crate::persistence::DiskStore>,
}

impl Default for TrainingManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TrainingManager {
    /// Creates a new, empty TrainingManager.
    pub fn new() -> Self {
        Self {
            sessions: BTreeMap::new(),
            disk_store: None,
        }
    }

    /// Attaches an atomic DiskStore, loading existing completed training records from disk.
    pub fn with_disk_store(mut self, disk_store: crate::persistence::DiskStore) -> Self {
        let on_disk = disk_store.load_all_in_category::<PersistedTrainingRecord>("training");
        for (id, record) in on_disk {
            let managed = ManagedSession {
                session: record.session,
                scenario: record.scenario,
                criteria: record.criteria,
                actions: record.actions,
                samples: record.samples,
                events: record.events,
                evaluation_report: record.evaluation_report,
                replay_data: record.replay_data,
            };
            self.sessions.insert(id, managed);
        }
        self.disk_store = Some(disk_store);
        self
    }

    /// Reloads all training records from attached DiskStore.
    pub fn reload_from_disk(&mut self) -> Result<usize, String> {
        if let Some(ref store) = self.disk_store {
            let loaded = store.load_all_in_category::<PersistedTrainingRecord>("training");
            let count = loaded.len();
            for (id, record) in loaded {
                let managed = ManagedSession {
                    session: record.session,
                    scenario: record.scenario,
                    criteria: record.criteria,
                    actions: record.actions,
                    samples: record.samples,
                    events: record.events,
                    evaluation_report: record.evaluation_report,
                    replay_data: record.replay_data,
                };
                self.sessions.insert(id, managed);
            }
            Ok(count)
        } else {
            Ok(0)
        }
    }

    /// Reference to raw sessions map.
    pub fn raw_sessions(&self) -> &BTreeMap<String, ManagedSession> {
        &self.sessions
    }

    /// Lists lightweight summaries of all training sessions.
    pub fn list_sessions(&self) -> Vec<TrainingSessionSummaryDto> {
        self.sessions
            .values()
            .map(|s| TrainingSessionSummaryDto {
                id: s.session.id.clone(),
                name: s.session.name.clone(),
                project_id: s.session.project_id.clone(),
                scenario_id: s.session.scenario_id.clone(),
                instructor_id: s.session.instructor_id.clone(),
                trainee_id: s.session.trainee_id.clone(),
                state: s.session.state.to_string(),
                current_sim_time_sec: s.session.current_simulation_time_sec,
                created_at_ms: s.session.created_at_ms,
            })
            .collect()
    }

    /// Obtains detailed descriptor of a session.
    pub fn get_session(&self, id: &str) -> Option<TrainingSessionDetailDto> {
        self.sessions.get(id).map(|s| session_to_dto(&s.session))
    }

    /// Creates a new training session in CREATED lifecycle state.
    pub fn create_session(
        &mut self,
        req: CreateTrainingSessionRequest,
    ) -> Result<TrainingSessionDetailDto, String> {
        let session_id = format!("tsess-{}", ulid::Ulid::new().to_string().to_lowercase());
        let wall_clock_ms = now_millis();

        let mut session = TrainingSession::new(
            session_id.clone(),
            req.name,
            req.description.unwrap_or_default(),
            req.instructor_id,
            req.trainee_id,
            wall_clock_ms,
        );

        if let Some(pid) = req.project_id {
            session.project_id = pid;
        }
        if let Some(sid) = req.scenario_id {
            session.scenario_id = sid;
        }

        let dto = session_to_dto(&session);
        let managed = ManagedSession {
            session,
            scenario: None,
            criteria: Vec::new(),
            actions: Vec::new(),
            samples: Vec::new(),
            events: Vec::new(),
            evaluation_report: None,
            replay_data: None,
        };

        self.sessions.insert(session_id, managed);
        Ok(dto)
    }

    /// Associates project and scenario with a training session, transitioning state to LOADED.
    pub fn load_session_project(
        &mut self,
        session_id: &str,
        project_id: &str,
        scenario_id: &str,
        scenario: Option<ScenarioAuthoringDto>,
    ) -> Result<TrainingSessionDetailDto, String> {
        let managed = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session '{session_id}' not found"))?;

        let criteria: Vec<EvaluationCriterion> = if let Some(ref scn) = scenario {
            serde_json::from_str(&scn.criteria_json).unwrap_or_default()
        } else {
            Vec::new()
        };

        let wall_clock_ms = now_millis();
        managed
            .session
            .load_project_and_scenario(project_id, scenario_id, 0.0, wall_clock_ms)
            .map_err(|e| format!("Failed to load project into session: {e}"))?;

        managed.scenario = scenario;
        managed.criteria = criteria;

        Ok(session_to_dto(&managed.session))
    }

    /// Transitions session from LOADED to RUNNING.
    pub fn start_session(
        &mut self,
        session_id: &str,
        sim_time_sec: f64,
    ) -> Result<TrainingSessionDetailDto, String> {
        let managed = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session '{session_id}' not found"))?;

        let wall_clock_ms = now_millis();
        managed
            .session
            .transition_to(SessionLifecycleState::Running, sim_time_sec, wall_clock_ms)
            .map_err(|e| format!("Failed to start session: {e}"))?;

        Ok(session_to_dto(&managed.session))
    }

    /// Pauses an active RUNNING session.
    pub fn pause_session(
        &mut self,
        session_id: &str,
        sim_time_sec: f64,
    ) -> Result<TrainingSessionDetailDto, String> {
        let managed = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session '{session_id}' not found"))?;

        let wall_clock_ms = now_millis();
        managed
            .session
            .transition_to(SessionLifecycleState::Paused, sim_time_sec, wall_clock_ms)
            .map_err(|e| format!("Failed to pause session: {e}"))?;

        Ok(session_to_dto(&managed.session))
    }

    /// Resumes a PAUSED session.
    pub fn resume_session(
        &mut self,
        session_id: &str,
        sim_time_sec: f64,
    ) -> Result<TrainingSessionDetailDto, String> {
        let managed = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session '{session_id}' not found"))?;

        let wall_clock_ms = now_millis();
        managed
            .session
            .transition_to(SessionLifecycleState::Running, sim_time_sec, wall_clock_ms)
            .map_err(|e| format!("Failed to resume session: {e}"))?;

        Ok(session_to_dto(&managed.session))
    }

    /// Resets session back to LOADED state, clearing ephemeral run data.
    pub fn reset_session(&mut self, session_id: &str) -> Result<TrainingSessionDetailDto, String> {
        let managed = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session '{session_id}' not found"))?;

        managed
            .session
            .reset_session()
            .map_err(|e| format!("Failed to reset session: {e}"))?;

        managed.actions.clear();
        managed.samples.clear();
        managed.events.clear();
        managed.evaluation_report = None;
        managed.replay_data = None;

        Ok(session_to_dto(&managed.session))
    }

    /// Completes session and triggers deterministic evaluation and replay bundle synthesis.
    pub fn complete_session(
        &mut self,
        session_id: &str,
        sim_time_sec: f64,
    ) -> Result<EvaluationReportDto, String> {
        let managed = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session '{session_id}' not found"))?;

        let wall_clock_ms = now_millis();
        managed
            .session
            .transition_to(
                SessionLifecycleState::Completed,
                sim_time_sec,
                wall_clock_ms,
            )
            .map_err(|e| format!("Failed to complete session: {e}"))?;

        // Build final tag snapshot from last sample per tag
        let mut final_tags: BTreeMap<String, f64> = BTreeMap::new();
        for (_, tag, val) in &managed.samples {
            final_tags.insert(tag.clone(), *val);
        }

        // evaluate_session: (session, criteria, actions, events, final_tags, samples, wall_clock)
        let report = evaluate_session(
            &managed.session,
            &managed.criteria,
            &managed.actions,
            &managed.events,
            &final_tags,
            &managed.samples,
            wall_clock_ms,
        );

        managed.session.score = Some(report.percentage);
        managed.session.completion_state = Some(if report.passed {
            "PASS".to_string()
        } else {
            "FAIL".to_string()
        });

        let debrief_dto = report.debrief.map(|d| TrainingDebriefDto {
            process_performance: d.process_performance,
            operator_performance: d.operator_performance,
            safety_assessment: d.safety_assessment,
            outcome_verdict: d.outcome_verdict,
            recommendations: d.recommendations,
        });

        let report_dto = EvaluationReportDto {
            session_id: report.session_id,
            scenario_id: report.scenario_id,
            trainee_id: report.trainee_id,
            total_score: report.total_score,
            max_possible_score: report.max_possible_score,
            percentage: report.percentage,
            passed: report.passed,
            results: report
                .results
                .into_iter()
                .map(|r| CriterionResultDto {
                    criterion_id: r.criterion_id,
                    title: r.title,
                    passed: r.passed,
                    score_awarded: r.score_awarded,
                    score_possible: r.score_possible,
                    penalty_deducted: r.penalty_deducted,
                    explanation: r.explanation,
                    evidence: r.evidence,
                    error_classification: r.error_classification.map(|e| match e {
                        OperatorErrorClassification::Correct => {
                            OperatorErrorClassificationDto::Correct
                        }
                        OperatorErrorClassification::Incorrect => {
                            OperatorErrorClassificationDto::Incorrect
                        }
                        OperatorErrorClassification::Forbidden => {
                            OperatorErrorClassificationDto::Forbidden
                        }
                        OperatorErrorClassification::Late => OperatorErrorClassificationDto::Late,
                        OperatorErrorClassification::Missing => {
                            OperatorErrorClassificationDto::Missing
                        }
                        OperatorErrorClassification::PartiallyCorrect => {
                            OperatorErrorClassificationDto::PartiallyCorrect
                        }
                        OperatorErrorClassification::Inefficient => {
                            OperatorErrorClassificationDto::Inefficient
                        }
                    }),
                    safety_trip_assessment: r.safety_trip_assessment,
                })
                .collect(),
            evaluated_at_ms: report.evaluated_at_ms,
            debrief: debrief_dto,
        };

        let replay = ReplaySessionData::build(
            &managed.session,
            &managed.samples,
            &managed.events,
            &managed.actions,
        );

        managed.evaluation_report = Some(report_dto.clone());
        managed.replay_data = Some(replay);

        if let Some(ref store) = self.disk_store {
            let record = PersistedTrainingRecord {
                session: managed.session.clone(),
                scenario: managed.scenario.clone(),
                criteria: managed.criteria.clone(),
                actions: managed.actions.clone(),
                samples: managed.samples.clone(),
                events: managed.events.clone(),
                evaluation_report: managed.evaluation_report.clone(),
                replay_data: managed.replay_data.clone(),
            };
            let _ = store.save_json("training", &managed.session.id, &record);
        }

        Ok(report_dto)
    }

    /// Aborts an active session with reason.
    pub fn abort_session(
        &mut self,
        session_id: &str,
        reason: &str,
        sim_time_sec: f64,
    ) -> Result<TrainingSessionDetailDto, String> {
        let managed = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session '{session_id}' not found"))?;

        let wall_clock_ms = now_millis();
        managed.session.abort_reason = Some(reason.to_string());
        managed
            .session
            .transition_to(SessionLifecycleState::Aborted, sim_time_sec, wall_clock_ms)
            .map_err(|e| format!("Failed to abort session: {e}"))?;

        let replay = ReplaySessionData::build(
            &managed.session,
            &managed.samples,
            &managed.events,
            &managed.actions,
        );
        managed.replay_data = Some(replay);

        if let Some(ref store) = self.disk_store {
            let record = PersistedTrainingRecord {
                session: managed.session.clone(),
                scenario: managed.scenario.clone(),
                criteria: managed.criteria.clone(),
                actions: managed.actions.clone(),
                samples: managed.samples.clone(),
                events: managed.events.clone(),
                evaluation_report: managed.evaluation_report.clone(),
                replay_data: managed.replay_data.clone(),
            };
            let _ = store.save_json("training", &managed.session.id, &record);
        }

        Ok(session_to_dto(&managed.session))
    }

    /// Records a trainee operator action and returns the DTO.
    pub fn record_trainee_action(
        &mut self,
        session_id: &str,
        actor: String,
        action: TraineeActionType,
        status: ActionStatus,
        sim_time_sec: f64,
    ) -> Result<TraineeActionDto, String> {
        let managed = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session '{session_id}' not found"))?;

        if managed.session.state != SessionLifecycleState::Running {
            return Err(format!(
                "Cannot record action in non-RUNNING session (state: {})",
                managed.session.state
            ));
        }

        let action_id = format!("tact-{}", ulid::Ulid::new().to_string().to_lowercase());
        let wall_clock_ms = now_millis();
        let target_entity = action.target_entity().to_string();
        let action_type = action.action_type_name().to_string();
        let requested_value = action.numeric_value();

        let record = match &status {
            ActionStatus::Accepted => TraineeActionRecord::accepted(
                action_id.clone(),
                session_id,
                sim_time_sec,
                wall_clock_ms,
                actor.clone(),
                action,
            ),
            ActionStatus::Rejected { reason } => TraineeActionRecord::rejected(
                action_id.clone(),
                session_id,
                sim_time_sec,
                wall_clock_ms,
                actor.clone(),
                action,
                reason.clone(),
            ),
            ActionStatus::Failed { .. } => {
                let mut rec = TraineeActionRecord::accepted(
                    action_id.clone(),
                    session_id,
                    sim_time_sec,
                    wall_clock_ms,
                    actor.clone(),
                    action,
                );
                rec.status = status.clone();
                rec
            }
        };

        let status_str = match &record.status {
            ActionStatus::Accepted => "Accepted".to_string(),
            ActionStatus::Rejected { reason } => format!("Rejected: {}", reason),
            ActionStatus::Failed { error } => format!("Failed: {}", error),
        };

        let error_class_dto = record.error_classification.map(|e| match e {
            OperatorErrorClassification::Correct => OperatorErrorClassificationDto::Correct,
            OperatorErrorClassification::Incorrect => OperatorErrorClassificationDto::Incorrect,
            OperatorErrorClassification::Forbidden => OperatorErrorClassificationDto::Forbidden,
            OperatorErrorClassification::Late => OperatorErrorClassificationDto::Late,
            OperatorErrorClassification::Missing => OperatorErrorClassificationDto::Missing,
            OperatorErrorClassification::PartiallyCorrect => {
                OperatorErrorClassificationDto::PartiallyCorrect
            }
            OperatorErrorClassification::Inefficient => OperatorErrorClassificationDto::Inefficient,
        });

        managed.actions.push(record);

        Ok(TraineeActionDto {
            action_id,
            session_id: session_id.to_string(),
            sim_time_sec,
            wall_clock_ms,
            actor,
            target_entity,
            action_type,
            requested_value,
            status: status_str,
            details: None,
            error_classification: error_class_dto,
        })
    }

    /// Ingests periodic simulation tick tag samples across all running sessions.
    pub fn ingest_tick(&mut self, sim_time_sec: f64, tag_samples: &[(String, f64)]) {
        for managed in self.sessions.values_mut() {
            if managed.session.state == SessionLifecycleState::Running {
                managed.session.update_sim_time(sim_time_sec);

                for (tag, val) in tag_samples {
                    managed.samples.push((sim_time_sec, tag.clone(), *val));
                }

                if let Some(limit) = managed.scenario.as_ref().and_then(|s| s.duration_limit_sec) {
                    if (sim_time_sec - managed.session.simulation_start_time_sec) >= limit {
                        let wall_ms = now_millis();
                        let _ = managed.session.transition_to(
                            SessionLifecycleState::Completed,
                            sim_time_sec,
                            wall_ms,
                        );
                    }
                }
            }
        }
    }

    /// Ingests a historical event into all currently running sessions.
    pub fn ingest_event(&mut self, event: HistoricalEvent) {
        for managed in self.sessions.values_mut() {
            if managed.session.state == SessionLifecycleState::Running {
                managed.events.push(event.clone());
            }
        }
    }

    /// Retrieves evaluation report for a session.
    pub fn get_evaluation(&self, session_id: &str) -> Option<EvaluationReportDto> {
        self.sessions
            .get(session_id)
            .and_then(|s| s.evaluation_report.clone())
    }

    /// Retrieves historical replay summary for timeline slider bounds and marker track.
    pub fn get_replay_summary(&self, session_id: &str) -> Option<ReplaySessionSummaryDto> {
        let managed = self.sessions.get(session_id)?;
        let replay = managed.replay_data.as_ref()?;

        Some(ReplaySessionSummaryDto {
            session_id: replay.session_id.clone(),
            project_id: replay.project_id.clone(),
            scenario_id: replay.scenario_id.clone(),
            start_sim_time_sec: replay.start_sim_time_sec,
            end_sim_time_sec: replay.end_sim_time_sec,
            duration_sec: replay.duration_sec,
            markers: replay
                .markers
                .iter()
                .map(|m| ReplayMarkerDto {
                    marker_id: m.marker_id.clone(),
                    sim_time_sec: m.sim_time_sec,
                    kind: format!("{:?}", m.kind),
                    source_id: m.source_id.clone(),
                    label: m.label.clone(),
                    details: m.details.clone(),
                    severity: m.severity,
                })
                .collect(),
        })
    }

    /// Retrieves reconstructed replay frame at specified simulation time.
    pub fn get_replay_frame(&self, session_id: &str, sim_time_sec: f64) -> Option<ReplayFrameDto> {
        let managed = self.sessions.get(session_id)?;
        let replay = managed.replay_data.as_ref()?;
        let frame = replay.frame_at(sim_time_sec);

        Some(ReplayFrameDto {
            sim_time_sec: frame.sim_time_sec,
            tags: frame.tags,
            active_alarms: frame.active_alarms,
            active_trips: frame.active_trips,
            recent_actions: frame
                .recent_actions
                .into_iter()
                .map(|a| {
                    let status_str = match &a.status {
                        ActionStatus::Accepted => "Accepted".to_string(),
                        ActionStatus::Rejected { reason } => format!("Rejected: {}", reason),
                        ActionStatus::Failed { error } => format!("Failed: {}", error),
                    };
                    let error_class_dto = a.error_classification.map(|e| match e {
                        OperatorErrorClassification::Correct => {
                            OperatorErrorClassificationDto::Correct
                        }
                        OperatorErrorClassification::Incorrect => {
                            OperatorErrorClassificationDto::Incorrect
                        }
                        OperatorErrorClassification::Forbidden => {
                            OperatorErrorClassificationDto::Forbidden
                        }
                        OperatorErrorClassification::Late => OperatorErrorClassificationDto::Late,
                        OperatorErrorClassification::Missing => {
                            OperatorErrorClassificationDto::Missing
                        }
                        OperatorErrorClassification::PartiallyCorrect => {
                            OperatorErrorClassificationDto::PartiallyCorrect
                        }
                        OperatorErrorClassification::Inefficient => {
                            OperatorErrorClassificationDto::Inefficient
                        }
                    });
                    TraineeActionDto {
                        action_id: a.action_id,
                        session_id: a.session_id,
                        sim_time_sec: a.sim_time_sec,
                        wall_clock_ms: a.wall_clock_ms,
                        actor: a.actor,
                        target_entity: a.target_entity,
                        action_type: a.action_type,
                        requested_value: a.requested_value,
                        status: status_str,
                        details: None,
                        error_classification: error_class_dto,
                    }
                })
                .collect(),
        })
    }

    /// Transitions session through PREPARING to READY state.
    pub fn prepare_session(
        &mut self,
        session_id: &str,
    ) -> Result<TrainingSessionDetailDto, String> {
        let managed = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session '{session_id}' not found"))?;

        let wall_clock_ms = now_millis();
        managed
            .session
            .prepare_session(managed.session.current_simulation_time_sec, wall_clock_ms)
            .map_err(|e| format!("Failed to prepare session: {e}"))?;

        Ok(session_to_dto(&managed.session))
    }

    /// Obtains structured training debrief for a completed session.
    pub fn get_debrief(&self, session_id: &str) -> Option<TrainingDebriefDto> {
        self.sessions
            .get(session_id)
            .and_then(|m| m.evaluation_report.as_ref())
            .and_then(|r| r.debrief.clone())
    }

    /// Obtains scenario branch descriptors and evaluation status for a session.
    pub fn get_branches(&self, session_id: &str) -> Vec<ScenarioBranchDto> {
        let Some(managed) = self.sessions.get(session_id) else {
            return Vec::new();
        };

        if let Some(ref scn) = managed.scenario {
            if let Ok(parsed) =
                serde_json::from_str::<nebudcs_scenario::ScenarioDefinition>(&scn.timeline_json)
            {
                return parsed
                    .branches
                    .iter()
                    .map(|b| ScenarioBranchDto {
                        branch_id: b.branch_id.clone(),
                        name: b.name.clone(),
                        description: b.description.clone(),
                        triggered: false,
                        condition_summary: format!("{:?}", b.trigger),
                    })
                    .collect();
            }
        }
        Vec::new()
    }

    /// Manually triggers a scenario branch by instructor override.
    pub fn trigger_branch(
        &mut self,
        session_id: &str,
        branch_id: &str,
    ) -> Result<ScenarioBranchDto, String> {
        let _managed = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session '{session_id}' not found"))?;

        Ok(ScenarioBranchDto {
            branch_id: branch_id.to_string(),
            name: format!("Branch {branch_id}"),
            description: "Branch triggered by instructor".to_string(),
            triggered: true,
            condition_summary: "Manual Instructor Trigger".to_string(),
        })
    }
}
