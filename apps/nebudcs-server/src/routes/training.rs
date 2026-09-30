//! Training studio, instructor console, trainee station, evaluation & replay REST endpoints.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

use nebudcs_protocol::{
    AbortSessionRequest, CreateTrainingSessionRequest, EvaluationReportDto,
    LoadSessionProjectRequest, PidModeDto, RecordTraineeActionRequest, ReplayFrameDto,
    ReplaySessionSummaryDto, RuntimeCommand, ScenarioAuthoringDto, ScenarioBranchDto,
    ScenarioSummaryDto, ScenarioValidationResultDto, SimControlCommand, TagValueDto,
    TraineeActionDto, TrainingDebriefDto, TrainingSessionDetailDto, TrainingSessionSummaryDto,
};
use nebudcs_scenario::training::action::{ActionStatus, TraineeActionType};

use crate::error::{ServerError, ServerResult};
use crate::host::HostHandle;
use crate::security::{audit_log, sanitize_identifier, UserRole};

// ── Query parameter structs ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ReplayFrameQuery {
    pub sim_time: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct DuplicateScenarioQuery {
    pub new_id: String,
    pub new_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ValidateScenarioQuery {
    pub project_id: Option<String>,
}

// ── Scenario Catalog & Authoring ────────────────────────────────────────────

pub async fn list_scenarios(
    State(host): State<HostHandle>,
) -> ServerResult<Json<Vec<ScenarioSummaryDto>>> {
    let mgr = host.scenario_manager().read().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire scenario manager lock: {e}"))
    })?;
    Ok(Json(mgr.list_scenarios()))
}

pub async fn get_scenario(
    State(host): State<HostHandle>,
    Path(id): Path<String>,
) -> ServerResult<Json<ScenarioAuthoringDto>> {
    let safe_id = sanitize_identifier(&id)?;
    let mgr = host.scenario_manager().read().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire scenario manager lock: {e}"))
    })?;
    let scn = mgr
        .get_scenario(&safe_id)
        .ok_or_else(|| ServerError::NotFound(format!("Scenario '{safe_id}' not found")))?;
    Ok(Json(scn))
}

pub async fn save_scenario(
    State(host): State<HostHandle>,
    role: UserRole,
    Json(mut dto): Json<ScenarioAuthoringDto>,
) -> ServerResult<Json<ScenarioAuthoringDto>> {
    if !role.can_instruct() {
        audit_log(
            "SCENARIO_SAVE",
            &format!("{role}"),
            "/api/v1/training/scenarios",
            "Non-instructor attempted scenario save",
            false,
        );
        return Err(ServerError::Forbidden(
            "Instructor or Admin role required to create or modify scenarios".to_string(),
        ));
    }

    dto.id = sanitize_identifier(&dto.id)?;
    let mut mgr = host.scenario_manager().write().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire scenario manager lock: {e}"))
    })?;
    let saved = mgr
        .save_scenario(dto)
        .map_err(ServerError::InvalidRequest)?;
    audit_log(
        "SCENARIO_SAVE",
        &format!("{role}"),
        &format!("/api/v1/training/scenarios/{}", saved.id),
        "Scenario saved",
        true,
    );
    Ok(Json(saved))
}

pub async fn update_scenario(
    State(host): State<HostHandle>,
    role: UserRole,
    Path(id): Path<String>,
    Json(mut dto): Json<ScenarioAuthoringDto>,
) -> ServerResult<Json<ScenarioAuthoringDto>> {
    if !role.can_instruct() {
        audit_log(
            "SCENARIO_UPDATE",
            &format!("{role}"),
            &format!("/api/v1/training/scenarios/{id}"),
            "Non-instructor attempted scenario update",
            false,
        );
        return Err(ServerError::Forbidden(
            "Instructor or Admin role required to modify scenarios".to_string(),
        ));
    }

    let safe_id = sanitize_identifier(&id)?;
    dto.id = safe_id.clone();
    let mut mgr = host.scenario_manager().write().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire scenario manager lock: {e}"))
    })?;
    let saved = mgr
        .save_scenario(dto)
        .map_err(ServerError::InvalidRequest)?;
    audit_log(
        "SCENARIO_UPDATE",
        &format!("{role}"),
        &format!("/api/v1/training/scenarios/{safe_id}"),
        "Scenario updated",
        true,
    );
    Ok(Json(saved))
}

pub async fn delete_scenario(
    State(host): State<HostHandle>,
    role: UserRole,
    Path(id): Path<String>,
) -> ServerResult<Json<serde_json::Value>> {
    if !role.can_instruct() {
        audit_log(
            "SCENARIO_DELETE",
            &format!("{role}"),
            &format!("/api/v1/training/scenarios/{id}"),
            "Non-instructor attempted scenario deletion",
            false,
        );
        return Err(ServerError::Forbidden(
            "Instructor or Admin role required to delete scenarios".to_string(),
        ));
    }

    let safe_id = sanitize_identifier(&id)?;
    let mut mgr = host.scenario_manager().write().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire scenario manager lock: {e}"))
    })?;
    mgr.delete_scenario(&safe_id)
        .map_err(ServerError::NotFound)?;
    audit_log(
        "SCENARIO_DELETE",
        &format!("{role}"),
        &format!("/api/v1/training/scenarios/{safe_id}"),
        "Scenario deleted",
        true,
    );
    Ok(Json(
        serde_json::json!({ "status": "deleted", "id": safe_id }),
    ))
}

pub async fn duplicate_scenario(
    State(host): State<HostHandle>,
    role: UserRole,
    Path(id): Path<String>,
    Query(query): Query<DuplicateScenarioQuery>,
) -> ServerResult<Json<ScenarioAuthoringDto>> {
    if !role.can_instruct() {
        audit_log(
            "SCENARIO_DUPLICATE",
            &format!("{role}"),
            &format!("/api/v1/training/scenarios/{id}/duplicate"),
            "Non-instructor attempted scenario duplication",
            false,
        );
        return Err(ServerError::Forbidden(
            "Instructor or Admin role required to duplicate scenarios".to_string(),
        ));
    }

    let safe_id = sanitize_identifier(&id)?;
    let safe_new_id = sanitize_identifier(&query.new_id)?;
    let mut mgr = host.scenario_manager().write().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire scenario manager lock: {e}"))
    })?;
    let duplicated = mgr
        .duplicate_scenario(
            &safe_id,
            &safe_new_id,
            query.new_name.as_deref().unwrap_or(""),
        )
        .map_err(ServerError::InvalidRequest)?;
    audit_log(
        "SCENARIO_DUPLICATE",
        &format!("{role}"),
        &format!("/api/v1/training/scenarios/{safe_new_id}"),
        "Scenario duplicated",
        true,
    );
    Ok(Json(duplicated))
}

pub async fn validate_scenario_endpoint(
    State(host): State<HostHandle>,
    Path(id): Path<String>,
    Query(query): Query<ValidateScenarioQuery>,
) -> ServerResult<Json<ScenarioValidationResultDto>> {
    let safe_id = sanitize_identifier(&id)?;
    let project_id = match query.project_id {
        Some(ref pid) => sanitize_identifier(pid)?,
        None => crate::project_manager::DEFAULT_PROJECT_ID.to_string(),
    };

    let project = {
        let pm = host.project_manager().read().map_err(|e| {
            ServerError::Internal(format!("Failed to acquire project manager lock: {e}"))
        })?;
        pm.get_project(&project_id)
            .ok_or_else(|| ServerError::NotFound(format!("Project '{project_id}' not found")))?
    };

    let sm = host.scenario_manager().read().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire scenario manager lock: {e}"))
    })?;

    Ok(Json(
        sm.validate_scenario_against_project(&safe_id, &project),
    ))
}

// ── Training Session Lifecycle ───────────────────────────────────────────────

pub async fn list_sessions(
    State(host): State<HostHandle>,
) -> ServerResult<Json<Vec<TrainingSessionSummaryDto>>> {
    let tm = host.training_manager().read().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire training manager lock: {e}"))
    })?;
    Ok(Json(tm.list_sessions()))
}

pub async fn create_session(
    State(host): State<HostHandle>,
    role: UserRole,
    Json(mut req): Json<CreateTrainingSessionRequest>,
) -> ServerResult<Json<TrainingSessionDetailDto>> {
    if !role.can_instruct() {
        audit_log(
            "SESSION_CREATE",
            &format!("{role}"),
            "/api/v1/training/sessions",
            "Non-instructor attempted session creation",
            false,
        );
        return Err(ServerError::Forbidden(
            "Instructor or Admin role required to create training sessions".to_string(),
        ));
    }

    if let Some(ref pid) = req.project_id {
        req.project_id = Some(sanitize_identifier(pid)?);
    }
    if let Some(ref sid) = req.scenario_id {
        req.scenario_id = Some(sanitize_identifier(sid)?);
    }

    let mut tm = host.training_manager().write().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire training manager lock: {e}"))
    })?;
    let created = tm
        .create_session(req)
        .map_err(ServerError::InvalidRequest)?;
    audit_log(
        "SESSION_CREATE",
        &format!("{role}"),
        &format!("/api/v1/training/sessions/{}", created.id),
        "Training session created",
        true,
    );
    Ok(Json(created))
}

pub async fn get_session(
    State(host): State<HostHandle>,
    Path(id): Path<String>,
) -> ServerResult<Json<TrainingSessionDetailDto>> {
    let safe_id = sanitize_identifier(&id)?;
    let tm = host.training_manager().read().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire training manager lock: {e}"))
    })?;
    let detail = tm
        .get_session(&safe_id)
        .ok_or_else(|| ServerError::NotFound(format!("Session '{safe_id}' not found")))?;
    Ok(Json(detail))
}

pub async fn load_session_project(
    State(host): State<HostHandle>,
    role: UserRole,
    Path(id): Path<String>,
    Json(mut req): Json<LoadSessionProjectRequest>,
) -> ServerResult<Json<TrainingSessionDetailDto>> {
    if !role.can_instruct() {
        return Err(ServerError::Forbidden(
            "Instructor or Admin role required to load session project".to_string(),
        ));
    }

    let safe_id = sanitize_identifier(&id)?;
    req.project_id = sanitize_identifier(&req.project_id)?;
    req.scenario_id = sanitize_identifier(&req.scenario_id)?;

    // 1. Load project into authoritative sim engine
    host.load_project_into_simulation(&req.project_id).await?;

    // 2. Retrieve authored scenario if available
    let scenario_dto = {
        let sm = host.scenario_manager().read().map_err(|e| {
            ServerError::Internal(format!("Failed to acquire scenario manager lock: {e}"))
        })?;
        sm.get_scenario(&req.scenario_id)
    };

    // 3. Load into training manager
    let mut tm = host.training_manager().write().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire training manager lock: {e}"))
    })?;
    let detail = tm
        .load_session_project(&safe_id, &req.project_id, &req.scenario_id, scenario_dto)
        .map_err(ServerError::InvalidRequest)?;

    audit_log(
        "SESSION_LOAD",
        &format!("{role}"),
        &format!("/api/v1/training/sessions/{safe_id}/load-project"),
        "Session project loaded",
        true,
    );
    Ok(Json(detail))
}

pub async fn start_session(
    State(host): State<HostHandle>,
    role: UserRole,
    Path(id): Path<String>,
) -> ServerResult<Json<TrainingSessionDetailDto>> {
    if !role.can_instruct() {
        return Err(ServerError::Forbidden(
            "Instructor or Admin role required to start training sessions".to_string(),
        ));
    }

    let safe_id = sanitize_identifier(&id)?;
    let snap = host.get_snapshot().await;
    let sim_time = snap.sim_time_sec;

    host.dispatch_command(RuntimeCommand::SimControl(SimControlCommand::Play))
        .await?;

    let mut tm = host.training_manager().write().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire training manager lock: {e}"))
    })?;
    let detail = tm
        .start_session(&safe_id, sim_time)
        .map_err(ServerError::InvalidRequest)?;

    let mut initial_samples = Vec::with_capacity(snap.tags.len());
    for t in &snap.tags {
        if let Some(v) = t.value.as_f64() {
            initial_samples.push((t.tag.clone(), v));
        }
    }
    tm.ingest_tick(sim_time, &initial_samples);

    audit_log(
        "SESSION_START",
        &format!("{role}"),
        &format!("/api/v1/training/sessions/{safe_id}/start"),
        "Training session commenced",
        true,
    );
    Ok(Json(detail))
}

pub async fn pause_session(
    State(host): State<HostHandle>,
    role: UserRole,
    Path(id): Path<String>,
) -> ServerResult<Json<TrainingSessionDetailDto>> {
    if !role.can_instruct() {
        return Err(ServerError::Forbidden(
            "Instructor or Admin role required to pause training sessions".to_string(),
        ));
    }

    let safe_id = sanitize_identifier(&id)?;
    let sim_time = host.get_snapshot().await.sim_time_sec;

    host.dispatch_command(RuntimeCommand::SimControl(SimControlCommand::Pause))
        .await?;

    let mut tm = host.training_manager().write().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire training manager lock: {e}"))
    })?;
    let detail = tm
        .pause_session(&safe_id, sim_time)
        .map_err(ServerError::InvalidRequest)?;

    Ok(Json(detail))
}

pub async fn resume_session(
    State(host): State<HostHandle>,
    role: UserRole,
    Path(id): Path<String>,
) -> ServerResult<Json<TrainingSessionDetailDto>> {
    if !role.can_instruct() {
        return Err(ServerError::Forbidden(
            "Instructor or Admin role required to resume training sessions".to_string(),
        ));
    }

    let safe_id = sanitize_identifier(&id)?;
    let sim_time = host.get_snapshot().await.sim_time_sec;

    host.dispatch_command(RuntimeCommand::SimControl(SimControlCommand::Play))
        .await?;

    let mut tm = host.training_manager().write().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire training manager lock: {e}"))
    })?;
    let detail = tm
        .resume_session(&safe_id, sim_time)
        .map_err(ServerError::InvalidRequest)?;

    Ok(Json(detail))
}

pub async fn reset_session(
    State(host): State<HostHandle>,
    role: UserRole,
    Path(id): Path<String>,
) -> ServerResult<Json<TrainingSessionDetailDto>> {
    if !role.can_instruct() {
        return Err(ServerError::Forbidden(
            "Instructor or Admin role required to reset training sessions".to_string(),
        ));
    }

    let safe_id = sanitize_identifier(&id)?;
    host.dispatch_command(RuntimeCommand::SimControl(SimControlCommand::Reset))
        .await?;

    let mut tm = host.training_manager().write().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire training manager lock: {e}"))
    })?;
    let detail = tm
        .reset_session(&safe_id)
        .map_err(ServerError::InvalidRequest)?;

    Ok(Json(detail))
}

pub async fn complete_session(
    State(host): State<HostHandle>,
    role: UserRole,
    Path(id): Path<String>,
) -> ServerResult<Json<EvaluationReportDto>> {
    if !role.can_instruct() {
        return Err(ServerError::Forbidden(
            "Instructor or Admin role required to complete training sessions".to_string(),
        ));
    }

    let safe_id = sanitize_identifier(&id)?;
    let snap = host.get_snapshot().await;
    let sim_time = snap.sim_time_sec;

    let _ = host
        .dispatch_command(RuntimeCommand::SimControl(SimControlCommand::Pause))
        .await;

    let mut tm = host.training_manager().write().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire training manager lock: {e}"))
    })?;

    let mut final_samples = Vec::with_capacity(snap.tags.len());
    for t in &snap.tags {
        if let Some(v) = t.value.as_f64() {
            final_samples.push((t.tag.clone(), v));
        }
    }
    tm.ingest_tick(sim_time, &final_samples);

    let report = tm
        .complete_session(&safe_id, sim_time)
        .map_err(ServerError::InvalidRequest)?;

    audit_log(
        "SESSION_COMPLETE",
        &format!("{role}"),
        &format!("/api/v1/training/sessions/{safe_id}/complete"),
        "Training session completed and evaluated",
        true,
    );
    Ok(Json(report))
}

pub async fn abort_session(
    State(host): State<HostHandle>,
    role: UserRole,
    Path(id): Path<String>,
    Json(req): Json<AbortSessionRequest>,
) -> ServerResult<Json<TrainingSessionDetailDto>> {
    if !role.can_instruct() {
        return Err(ServerError::Forbidden(
            "Instructor or Admin role required to abort training sessions".to_string(),
        ));
    }

    let safe_id = sanitize_identifier(&id)?;
    let sim_time = host.get_snapshot().await.sim_time_sec;

    let _ = host
        .dispatch_command(RuntimeCommand::SimControl(SimControlCommand::Pause))
        .await;

    let mut tm = host.training_manager().write().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire training manager lock: {e}"))
    })?;
    let detail = tm
        .abort_session(&safe_id, &req.reason, sim_time)
        .map_err(ServerError::InvalidRequest)?;

    audit_log(
        "SESSION_ABORT",
        &format!("{role}"),
        &format!("/api/v1/training/sessions/{safe_id}/abort"),
        "Training session aborted",
        true,
    );
    Ok(Json(detail))
}

// ── Trainee Action Recording ────────────────────────────────────────────────

pub async fn record_trainee_action(
    State(host): State<HostHandle>,
    Path(id): Path<String>,
    Json(req): Json<RecordTraineeActionRequest>,
) -> ServerResult<Json<TraineeActionDto>> {
    let safe_id = sanitize_identifier(&id)?;
    let sim_time = host.get_snapshot().await.sim_time_sec;
    let action_upper = req.action_type.to_uppercase();

    // Role guard: trainees cannot execute administrative commands
    let forbidden = [
        "INJECT_FAULT",
        "CLEAR_FAULT",
        "LOAD_PROJECT",
        "RESET_SIMULATION",
    ];
    if forbidden.contains(&action_upper.as_str()) {
        let action = TraineeActionType::EmergencyStop; // placeholder — won't be dispatched
        let mut tm = host.training_manager().write().map_err(|e| {
            ServerError::Internal(format!("Failed to acquire training manager lock: {e}"))
        })?;
        return tm
            .record_trainee_action(
                &safe_id,
                req.actor,
                action,
                ActionStatus::Rejected {
                    reason: "Trainee lacks permission for administrative commands".to_string(),
                },
                sim_time,
            )
            .map(Json)
            .map_err(ServerError::InvalidRequest);
    }

    // Build strongly-typed action and corresponding runtime command
    let (action_type_val, runtime_cmd) = match action_upper.as_str() {
        "SET_SP" | "SET_SETPOINT" => {
            let sp = req.requested_value.unwrap_or(0.0);
            (
                TraineeActionType::SetSetpoint {
                    loop_name: req.target_entity.clone(),
                    sp,
                },
                Some(RuntimeCommand::SetSetpoint {
                    loop_name: req.target_entity.clone(),
                    sp,
                }),
            )
        }
        "SET_MODE" => {
            let mode_str = req
                .command_payload
                .as_ref()
                .and_then(|p| p.get("mode"))
                .and_then(|m| m.as_str())
                .unwrap_or("MAN");
            let (mode_dto, mode_str_canon) = match mode_str.to_uppercase().as_str() {
                "AUTO" => (PidModeDto::Auto, "AUTO"),
                "CAS" | "CASCADE" => (PidModeDto::Cascade, "CAS"),
                _ => (PidModeDto::Manual, "MAN"),
            };
            (
                TraineeActionType::SetMode {
                    loop_name: req.target_entity.clone(),
                    mode: mode_str_canon.to_string(),
                },
                Some(RuntimeCommand::SetMode {
                    loop_name: req.target_entity.clone(),
                    mode: mode_dto,
                }),
            )
        }
        "SET_OUTPUT" | "MANUAL_OUTPUT" | "SET_MANUAL_OUTPUT" => {
            let output = req.requested_value.unwrap_or(0.0);
            (
                TraineeActionType::SetManualOutput {
                    loop_name: req.target_entity.clone(),
                    output,
                },
                Some(RuntimeCommand::SetManualOutput {
                    loop_name: req.target_entity.clone(),
                    output,
                }),
            )
        }
        "WRITE_TAG" => {
            let val = req.requested_value.unwrap_or(0.0);
            (
                TraineeActionType::WriteTag {
                    tag: req.target_entity.clone(),
                    value: val,
                },
                Some(RuntimeCommand::WriteTag {
                    tag: req.target_entity.clone(),
                    value: TagValueDto::Float(val),
                }),
            )
        }
        "ACK_ALARM" => (
            TraineeActionType::AcknowledgeAlarm {
                alarm_name: req.target_entity.clone(),
            },
            Some(RuntimeCommand::AcknowledgeAlarm {
                alarm_name: req.target_entity.clone(),
            }),
        ),
        "SHELVE_ALARM" => {
            let dur = req.requested_value.unwrap_or(60.0);
            (
                TraineeActionType::ShelveAlarm {
                    alarm_name: req.target_entity.clone(),
                    duration_sec: dur,
                },
                Some(RuntimeCommand::ShelveAlarm {
                    alarm_name: req.target_entity.clone(),
                    duration_seconds: dur,
                }),
            )
        }
        "RESET_PROTECTION" | "RESET_TRIP" => (
            TraineeActionType::ResetProtection,
            Some(RuntimeCommand::ResetProtection {
                block_name: req.target_entity.clone(),
            }),
        ),
        "EMERGENCY_STOP" | "ESD" => (TraineeActionType::EmergencyStop, None),
        cmd => (
            TraineeActionType::EquipmentCommand {
                equipment_name: req.target_entity.clone(),
                command: cmd.to_string(),
            },
            None,
        ),
    };

    // Dispatch command to authoritative simulation engine if applicable
    let status = if let Some(cmd) = runtime_cmd {
        match host.dispatch_command(cmd).await {
            Ok(()) => ActionStatus::Accepted,
            Err(e) => ActionStatus::Failed {
                error: format!("{e}"),
            },
        }
    } else {
        ActionStatus::Accepted
    };

    // Record action into TrainingManager
    let mut tm = host.training_manager().write().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire training manager lock: {e}"))
    })?;
    let recorded = tm
        .record_trainee_action(&safe_id, req.actor, action_type_val, status, sim_time)
        .map_err(ServerError::InvalidRequest)?;

    Ok(Json(recorded))
}

// ── Evaluation & Replay ─────────────────────────────────────────────────────

pub async fn get_evaluation(
    State(host): State<HostHandle>,
    Path(id): Path<String>,
) -> ServerResult<Json<EvaluationReportDto>> {
    let safe_id = sanitize_identifier(&id)?;
    let tm = host.training_manager().read().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire training manager lock: {e}"))
    })?;
    let eval = tm.get_evaluation(&safe_id).ok_or_else(|| {
        ServerError::NotFound(format!("Evaluation for session '{safe_id}' not found"))
    })?;
    Ok(Json(eval))
}

pub async fn get_replay_summary(
    State(host): State<HostHandle>,
    Path(id): Path<String>,
) -> ServerResult<Json<ReplaySessionSummaryDto>> {
    let safe_id = sanitize_identifier(&id)?;
    let tm = host.training_manager().read().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire training manager lock: {e}"))
    })?;
    let summary = tm.get_replay_summary(&safe_id).ok_or_else(|| {
        ServerError::NotFound(format!("Replay data for session '{safe_id}' not found"))
    })?;
    Ok(Json(summary))
}

pub async fn get_replay_frame(
    State(host): State<HostHandle>,
    Path(id): Path<String>,
    Query(query): Query<ReplayFrameQuery>,
) -> ServerResult<Json<ReplayFrameDto>> {
    let safe_id = sanitize_identifier(&id)?;
    let sim_time = query.sim_time.unwrap_or(0.0);
    let tm = host.training_manager().read().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire training manager lock: {e}"))
    })?;
    let frame = tm.get_replay_frame(&safe_id, sim_time).ok_or_else(|| {
        ServerError::NotFound(format!("Replay frame for session '{safe_id}' not found"))
    })?;
    Ok(Json(frame))
}

pub async fn prepare_session(
    State(host): State<HostHandle>,
    role: UserRole,
    Path(id): Path<String>,
) -> ServerResult<Json<TrainingSessionDetailDto>> {
    if !role.can_instruct() {
        return Err(ServerError::Forbidden(
            "Instructor or Admin role required to prepare training sessions".to_string(),
        ));
    }

    let safe_id = sanitize_identifier(&id)?;
    let mut tm = host.training_manager().write().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire training manager lock: {e}"))
    })?;
    let detail = tm
        .prepare_session(&safe_id)
        .map_err(ServerError::InvalidRequest)?;

    audit_log(
        "SESSION_PREPARE",
        &format!("{role}"),
        &format!("/api/v1/training/sessions/{safe_id}/prepare"),
        "Training session prepared",
        true,
    );
    Ok(Json(detail))
}

pub async fn get_debrief(
    State(host): State<HostHandle>,
    Path(id): Path<String>,
) -> ServerResult<Json<TrainingDebriefDto>> {
    let safe_id = sanitize_identifier(&id)?;
    let tm = host.training_manager().read().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire training manager lock: {e}"))
    })?;
    let debrief = tm.get_debrief(&safe_id).ok_or_else(|| {
        ServerError::NotFound(format!("Debrief for session '{safe_id}' not found"))
    })?;
    Ok(Json(debrief))
}

pub async fn get_branches(
    State(host): State<HostHandle>,
    Path(id): Path<String>,
) -> ServerResult<Json<Vec<ScenarioBranchDto>>> {
    let safe_id = sanitize_identifier(&id)?;
    let tm = host.training_manager().read().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire training manager lock: {e}"))
    })?;
    Ok(Json(tm.get_branches(&safe_id)))
}

pub async fn trigger_branch(
    State(host): State<HostHandle>,
    role: UserRole,
    Path((id, branch_id)): Path<(String, String)>,
) -> ServerResult<Json<ScenarioBranchDto>> {
    if !role.can_instruct() {
        return Err(ServerError::Forbidden(
            "Instructor or Admin role required to trigger scenario branches".to_string(),
        ));
    }

    let safe_id = sanitize_identifier(&id)?;
    let safe_branch_id = sanitize_identifier(&branch_id)?;
    let mut tm = host.training_manager().write().map_err(|e| {
        ServerError::Internal(format!("Failed to acquire training manager lock: {e}"))
    })?;
    let branch = tm
        .trigger_branch(&safe_id, &safe_branch_id)
        .map_err(ServerError::InvalidRequest)?;

    audit_log(
        "SCENARIO_BRANCH_TRIGGER",
        &format!("{role}"),
        &format!("/api/v1/training/sessions/{safe_id}/branches/{safe_branch_id}/trigger"),
        "Scenario branch triggered manually",
        true,
    );
    Ok(Json(branch))
}
