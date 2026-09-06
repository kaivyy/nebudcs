//! Central deterministic scenario engine orchestrator.

use nebudcs_alarm::AlarmSystem;
use nebudcs_control::pid::PidMode;
use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_control::ControlSystem;
use nebudcs_historian::{HistorianRecord, HistorianSender, HistoricalEvent};
use nebudcs_model::tag::{TagQuality, TagValue};
use nebudcs_types::SimulationTime;

use crate::disturbance::DisturbanceManager;
use crate::error::{ScenarioError, ScenarioResult};
use crate::fault::{ActiveFaultSummary, FaultManager};
use crate::model::*;
use crate::objective::ObjectiveEvaluator;
use crate::timeline::TimelineScheduler;
use crate::validation::{validate_fault, validate_scenario};

/// Comprehensive runtime scenario status DTO for telemetry and UI.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ScenarioStatus {
    pub scenario_id: Option<String>,
    pub scenario_name: Option<String>,
    pub state: ScenarioState,
    pub elapsed_sec: f64,
    pub duration_limit_sec: Option<f64>,
    pub active_faults: Vec<ActiveFaultSummary>,
    pub active_disturbances_count: usize,
    pub events_total: usize,
    pub events_fired: usize,
    pub score: f64,
    pub mandatory_completed: bool,
}

/// The deterministic scenario execution engine.
#[derive(Debug, Clone)]
pub struct ScenarioEngine {
    scenario: Option<ScenarioDefinition>,
    state: ScenarioState,
    start_sim_time_sec: f64,
    elapsed_scenario_sec: f64,
    timeline: TimelineScheduler,
    faults: FaultManager,
    disturbances: DisturbanceManager,
    objectives: ObjectiveEvaluator,
    event_seq: u64,
}

impl Default for ScenarioEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ScenarioEngine {
    /// Creates a new, uninitialized scenario engine.
    pub fn new() -> Self {
        Self {
            scenario: None,
            state: ScenarioState::Idle,
            start_sim_time_sec: 0.0,
            elapsed_scenario_sec: 0.0,
            timeline: TimelineScheduler::default(),
            faults: FaultManager::new(0x42_1337_cafe),
            disturbances: DisturbanceManager::new(0x42_1337_cafe),
            objectives: ObjectiveEvaluator::default(),
            event_seq: 0,
        }
    }

    /// Loads and validates a declarative scenario definition.
    pub fn load_scenario(&mut self, scenario: ScenarioDefinition) -> ScenarioResult<()> {
        validate_scenario(&scenario)?;

        let seed = scenario.random_seed;
        self.timeline = TimelineScheduler::new(scenario.timeline.clone());
        self.faults = FaultManager::new(seed);
        self.disturbances = DisturbanceManager::new(seed);
        self.objectives = ObjectiveEvaluator::new(scenario.objectives.clone());
        self.scenario = Some(scenario);
        self.state = ScenarioState::Loaded;
        self.elapsed_scenario_sec = 0.0;

        Ok(())
    }

    /// Arms the scenario and applies initial conditions to runtime stores.
    pub fn arm_and_initialize(
        &mut self,
        current_sim_time: f64,
        tag_store: &mut RuntimeTagStore,
        mut control_system: Option<&mut ControlSystem>,
    ) -> ScenarioResult<()> {
        let Some(ref scn) = self.scenario else {
            return Err(ScenarioError::InvalidState(
                "No scenario loaded to arm".into(),
            ));
        };

        self.start_sim_time_sec = current_sim_time;
        self.elapsed_scenario_sec = 0.0;
        self.timeline.reset();
        self.faults.reset(scn.random_seed, tag_store);
        self.disturbances.reset(scn.random_seed);
        self.objectives.reset();

        // Apply initial tag conditions
        for tag_init in &scn.initial_conditions.tags {
            if let Some(tag_id) = tag_store.resolve_name(&tag_init.tag) {
                tag_store.set_tag_float(tag_id, tag_init.value, TagQuality::Good);
            }
        }

        // Apply initial controller conditions
        for ctrl_init in &scn.initial_conditions.controllers {
            if let Some(tag_id) = tag_store.resolve_name(&ctrl_init.loop_name) {
                if let Some(ref mut ctrl) = control_system {
                    if let Some(pid) = ctrl.pid_mut(tag_id) {
                        if let Some(ref mode_str) = ctrl_init.mode {
                            let mode = match mode_str.to_uppercase().as_str() {
                                "MAN" | "MANUAL" => PidMode::Manual,
                                "AUTO" => PidMode::Auto,
                                "CAS" | "CASCADE" => PidMode::Cascade,
                                "RCAS" | "REMOTE_CASCADE" => PidMode::RemoteCascade,
                                _ => PidMode::Manual,
                            };
                            pid.set_mode(mode);
                        }
                        if let Some(sp) = ctrl_init.sp {
                            pid.set_sp(sp);
                        }
                        if let Some(man_out) = ctrl_init.manual_output {
                            pid.set_manual_output(man_out);
                        }
                    }
                }
            }
        }

        // Apply initial equipment conditions
        for eq_init in &scn.initial_conditions.equipment {
            if let Some(speed) = eq_init.speed_rpm {
                let speed_tag = format!("{}.SPEED", eq_init.equipment_name);
                if let Some(tag_id) = tag_store.resolve_name(&speed_tag) {
                    tag_store.set_tag_float(tag_id, speed, TagQuality::Good);
                }
            }
            if let Some(is_running) = eq_init.is_running {
                let status_tag = format!("{}.STATUS", eq_init.equipment_name);
                if let Some(tag_id) = tag_store.resolve_name(&status_tag) {
                    tag_store.set_tag(
                        tag_id,
                        TagValue::Integer(if is_running { 1 } else { 0 }),
                        TagQuality::Good,
                    );
                }
            }
        }

        self.state = ScenarioState::Armed;
        Ok(())
    }

    /// Transitions an armed or loaded scenario into active execution.
    pub fn start(&mut self) -> ScenarioResult<()> {
        match self.state {
            ScenarioState::Armed | ScenarioState::Loaded => {
                self.state = ScenarioState::Running;
                Ok(())
            }
            ScenarioState::Running => Ok(()),
            other => Err(ScenarioError::InvalidState(format!(
                "Cannot start scenario from state {other:?}"
            ))),
        }
    }

    /// Pauses scenario progression.
    pub fn pause(&mut self) -> ScenarioResult<()> {
        if self.state == ScenarioState::Running {
            self.state = ScenarioState::Paused;
            Ok(())
        } else {
            Err(ScenarioError::InvalidState(format!(
                "Cannot pause scenario from state {:?}",
                self.state
            )))
        }
    }

    /// Resumes a paused scenario.
    pub fn resume(&mut self) -> ScenarioResult<()> {
        if self.state == ScenarioState::Paused {
            self.state = ScenarioState::Running;
            Ok(())
        } else {
            Err(ScenarioError::InvalidState(format!(
                "Cannot resume scenario from state {:?}",
                self.state
            )))
        }
    }

    /// Aborts the running or paused scenario.
    pub fn abort(&mut self, reason: &str, tag_store: &mut RuntimeTagStore) {
        if let Some(ref scn) = self.scenario {
            self.faults.reset(scn.random_seed, tag_store);
            self.disturbances.reset(scn.random_seed);
        }
        self.state = ScenarioState::Aborted;
        let _ = reason;
    }

    /// Resets the scenario to its loaded baseline.
    pub fn reset(&mut self, tag_store: &mut RuntimeTagStore) {
        if let Some(ref scn) = self.scenario {
            self.timeline.reset();
            self.faults.reset(scn.random_seed, tag_store);
            self.disturbances.reset(scn.random_seed);
            self.objectives.reset();
            self.elapsed_scenario_sec = 0.0;
            self.state = ScenarioState::Loaded;
        } else {
            self.state = ScenarioState::Idle;
        }
    }

    /// Injects an ad-hoc fault directly into the running simulation.
    pub fn inject_fault(
        &mut self,
        fault: FaultDefinition,
        current_sim_time: f64,
        tag_store: &mut RuntimeTagStore,
    ) -> ScenarioResult<()> {
        validate_fault(&fault)?;
        self.faults
            .inject_fault(fault, current_sim_time, tag_store)?;
        let _ = self.faults.step(0.0, tag_store);
        Ok(())
    }

    /// Clears an active fault by ID.
    pub fn clear_fault(&mut self, fault_id: &str, tag_store: &mut RuntimeTagStore) -> bool {
        self.faults.clear_fault(fault_id, tag_store)
    }

    /// Deterministic execution step invoked at simulation tick boundaries.
    pub fn step(
        &mut self,
        sim_time: SimulationTime,
        dt: f64,
        tag_store: &mut RuntimeTagStore,
        mut control_system: Option<&mut ControlSystem>,
        alarm_system: Option<&AlarmSystem>,
        historian: Option<&HistorianSender>,
    ) -> ScenarioStatus {
        let is_running = self.state == ScenarioState::Running;
        let current_sim_sec = sim_time.seconds();

        if is_running {
            self.elapsed_scenario_sec += dt;

            // 1. Evaluate timeline triggers
            let actions = self.timeline.step(self.elapsed_scenario_sec, tag_store);
            for (event_id, action) in actions {
                self.log_historian_event(
                    historian,
                    sim_time,
                    "TIMELINE_EVENT",
                    &event_id,
                    None,
                    "FIRED",
                    None,
                );

                match action {
                    ScenarioAction::InjectFault(fault) => {
                        let fault_id = fault.id.clone();
                        let _ = self.faults.inject_fault(fault, current_sim_sec, tag_store);
                        let _ = self.faults.step(0.0, tag_store);
                        self.log_historian_event(
                            historian,
                            sim_time,
                            "FAULT_INJECTED",
                            &fault_id,
                            None,
                            "ACTIVE",
                            None,
                        );
                    }
                    ScenarioAction::ClearFault { fault_id } => {
                        self.faults.clear_fault(&fault_id, tag_store);
                        self.log_historian_event(
                            historian,
                            sim_time,
                            "FAULT_CLEARED",
                            &fault_id,
                            Some("ACTIVE"),
                            "CLEARED",
                            None,
                        );
                    }
                    ScenarioAction::ApplyDisturbance(dist) => {
                        let dist_id = dist.id.clone();
                        self.disturbances
                            .apply_disturbance(dist, current_sim_sec, tag_store);
                        self.log_historian_event(
                            historian,
                            sim_time,
                            "DISTURBANCE_APPLIED",
                            &dist_id,
                            None,
                            "ACTIVE",
                            None,
                        );
                    }
                    ScenarioAction::ExecuteOperatorAction(op) => {
                        self.apply_operator_action(op, tag_store, control_system.as_deref_mut());
                    }
                    ScenarioAction::EmitMarker { label, message } => {
                        self.log_historian_event(
                            historian,
                            sim_time,
                            "SCENARIO_MARKER",
                            &label,
                            None,
                            &message,
                            None,
                        );
                    }
                }
            }
        }

        // 2. Step active disturbances
        let _ = self.disturbances.step(dt, tag_store);

        // 3. Step active faults
        let expired_faults = self.faults.step(dt, tag_store);
        for fault_id in expired_faults {
            self.log_historian_event(
                historian,
                sim_time,
                "FAULT_EXPIRED",
                &fault_id,
                Some("ACTIVE"),
                "EXPIRED",
                None,
            );
        }

        // 4. Step objective evaluation
        if is_running {
            self.objectives
                .step(self.elapsed_scenario_sec, dt, tag_store, alarm_system);

            // 5. Evaluate completion / termination conditions
            let scenario_id_and_limit = self
                .scenario
                .as_ref()
                .map(|s| (s.id.clone(), s.duration_limit_sec));

            if let Some((scenario_id, duration_limit)) = scenario_id_and_limit {
                if let Some(limit) = duration_limit {
                    if self.elapsed_scenario_sec >= limit {
                        self.state = ScenarioState::Completed;
                        let score = self.objectives.score();
                        self.log_historian_event(
                            historian,
                            sim_time,
                            "SCENARIO_COMPLETED",
                            &scenario_id,
                            Some("RUNNING"),
                            "COMPLETED",
                            Some(score),
                        );
                    }
                } else if self.objectives.all_mandatory_satisfied()
                    && self.timeline.fired_count() == self.timeline.len()
                    && self.faults.is_empty()
                    && self.disturbances.is_empty()
                {
                    self.state = ScenarioState::Completed;
                    let score = self.objectives.score();
                    self.log_historian_event(
                        historian,
                        sim_time,
                        "SCENARIO_COMPLETED",
                        &scenario_id,
                        Some("RUNNING"),
                        "COMPLETED",
                        Some(score),
                    );
                }
            }
        }

        self.status()
    }

    fn apply_operator_action(
        &mut self,
        action: OperatorAction,
        tag_store: &mut RuntimeTagStore,
        control_system: Option<&mut ControlSystem>,
    ) {
        match action {
            OperatorAction::SetSetpoint { loop_name, sp } => {
                if let Some(tag_id) = tag_store.resolve_name(&loop_name) {
                    if let Some(ctrl) = control_system {
                        if let Some(pid) = ctrl.pid_mut(tag_id) {
                            pid.set_sp(sp);
                        }
                    }
                }
            }
            OperatorAction::SetMode { loop_name, mode } => {
                if let Some(tag_id) = tag_store.resolve_name(&loop_name) {
                    if let Some(ctrl) = control_system {
                        if let Some(pid) = ctrl.pid_mut(tag_id) {
                            let m = match mode.to_uppercase().as_str() {
                                "MAN" | "MANUAL" => PidMode::Manual,
                                "AUTO" => PidMode::Auto,
                                "CAS" | "CASCADE" => PidMode::Cascade,
                                "RCAS" => PidMode::RemoteCascade,
                                _ => PidMode::Manual,
                            };
                            pid.set_mode(m);
                        }
                    }
                }
            }
            OperatorAction::SetManualOutput { loop_name, output } => {
                if let Some(tag_id) = tag_store.resolve_name(&loop_name) {
                    if let Some(ctrl) = control_system {
                        if let Some(pid) = ctrl.pid_mut(tag_id) {
                            pid.set_manual_output(output);
                        }
                    }
                }
            }
            OperatorAction::WriteTag { tag, value } => {
                if let Some(tag_id) = tag_store.resolve_name(&tag) {
                    tag_store.set_tag_float(tag_id, value, TagQuality::Good);
                }
            }
            OperatorAction::AcknowledgeAlarm { alarm_name } => {
                self.objectives
                    .on_alarm_acknowledged(&alarm_name, self.elapsed_scenario_sec);
            }
            OperatorAction::ShelveAlarm { .. } => {}
            OperatorAction::ResetProtection => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn log_historian_event(
        &mut self,
        historian: Option<&HistorianSender>,
        sim_time: SimulationTime,
        event_type: &str,
        source_id: &str,
        old_state: Option<&str>,
        new_state: &str,
        value: Option<f64>,
    ) {
        if let Some(h) = historian {
            self.event_seq += 1;
            let run_id = self
                .scenario
                .as_ref()
                .map(|s| s.id.clone())
                .unwrap_or_else(|| "default_run".into());

            let event = HistoricalEvent {
                run_id,
                sim_time,
                seq: self.event_seq,
                event_type: event_type.to_string(),
                source_id: source_id.to_string(),
                old_state: old_state.map(|s| s.to_string()),
                new_state: new_state.to_string(),
                value,
                severity: Some(2),
                details_json: None,
                persisted_wall_clock_ms: 0,
            };

            let _ = h.try_send(HistorianRecord::Event(event));
        }
    }

    /// Obtains current consolidated scenario status for protocol publication.
    pub fn status(&self) -> ScenarioStatus {
        ScenarioStatus {
            scenario_id: self.scenario.as_ref().map(|s| s.id.clone()),
            scenario_name: self.scenario.as_ref().map(|s| s.name.clone()),
            state: self.state,
            elapsed_sec: self.elapsed_scenario_sec,
            duration_limit_sec: self.scenario.as_ref().and_then(|s| s.duration_limit_sec),
            active_faults: self.faults.summaries(),
            active_disturbances_count: self.disturbances.len(),
            events_total: self.timeline.len(),
            events_fired: self.timeline.fired_count(),
            score: self.objectives.score(),
            mandatory_completed: self.objectives.all_mandatory_satisfied(),
        }
    }

    pub fn state(&self) -> ScenarioState {
        self.state
    }

    pub fn scenario(&self) -> Option<&ScenarioDefinition> {
        self.scenario.as_ref()
    }
}
