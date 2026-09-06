//! Central simulation host and deterministic execution thread.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{Builder, JoinHandle};
use std::time::{Duration, Instant};

use crossbeam_channel::{bounded, Receiver, Sender};
use tokio::sync::{broadcast, RwLock};

use nebudcs_alarm::AlarmSystem;
use nebudcs_control::pid::PidMode;
use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_control::ControlSystem;
use nebudcs_model::tag::{TagQuality, TagValue};
use nebudcs_protocol::*;
use nebudcs_sim::{ClockState, SimulationEngine};

use crate::config::ServerConfig;
use crate::error::{ServerError, ServerResult};

/// In-flight command message with optional acknowledgment one-shot.
#[derive(Debug)]
pub struct EnqueuedCommand {
    pub command: RuntimeCommand,
    pub ack_sender: Option<tokio::sync::oneshot::Sender<ServerResult<()>>>,
}

/// Handle to interact with the running simulation host from API/WebSocket tasks.
#[derive(Clone)]
pub struct HostHandle {
    command_tx: Sender<EnqueuedCommand>,
    broadcast_tx: broadcast::Sender<ServerMessage>,
    snapshot_cache: Arc<RwLock<StateSnapshotDto>>,
    health_cache: Arc<RwLock<ServerHealthDto>>,
    scenario_cache: Arc<RwLock<ScenarioStatusDto>>,
    connected_clients: Arc<AtomicUsize>,
    historian_reader: Option<Arc<dyn nebudcs_historian::HistorianReader>>,
    config: ServerConfig,
}

impl HostHandle {
    /// Submits a typed runtime command to be applied at the next deterministic simulation boundary.
    pub async fn dispatch_command(&self, command: RuntimeCommand) -> ServerResult<()> {
        let (ack_tx, ack_rx) = tokio::sync::oneshot::channel();
        let enqueued = EnqueuedCommand {
            command,
            ack_sender: Some(ack_tx),
        };

        self.command_tx
            .try_send(enqueued)
            .map_err(|_| ServerError::CommandRejected("Command queue full".to_string()))?;

        match ack_rx.await {
            Ok(result) => result,
            Err(_) => Err(ServerError::Internal(
                "Command acknowledgment dropped".to_string(),
            )),
        }
    }

    /// Submits a command asynchronously without waiting for tick application (e.g. from WebSockets).
    pub fn try_send_command_nowait(&self, command: RuntimeCommand) -> ServerResult<()> {
        let enqueued = EnqueuedCommand {
            command,
            ack_sender: None,
        };
        self.command_tx
            .try_send(enqueued)
            .map_err(|_| ServerError::CommandRejected("Command queue full".to_string()))
    }

    /// Obtains a coherent point-in-time full state snapshot.
    pub async fn get_snapshot(&self) -> StateSnapshotDto {
        self.snapshot_cache.read().await.clone()
    }

    /// Obtains current server and engine health status.
    pub async fn get_health(&self) -> ServerHealthDto {
        let mut health = self.health_cache.read().await.clone();
        health.connected_clients = self.connected_clients.load(Ordering::Relaxed);
        health
    }

    /// Obtains current scenario and fault status.
    pub async fn get_scenario_status(&self) -> ScenarioStatusDto {
        self.scenario_cache.read().await.clone()
    }

    /// Subscribes to the broadcast channel for real-time WebSocket state updates.
    pub fn subscribe_broadcast(&self) -> broadcast::Receiver<ServerMessage> {
        self.broadcast_tx.subscribe()
    }

    /// Direct access to the historian reader if configured.
    pub fn historian_reader(&self) -> Option<&Arc<dyn nebudcs_historian::HistorianReader>> {
        self.historian_reader.as_ref()
    }

    /// Tracks a new connected WebSocket client.
    pub fn inc_clients(&self) -> usize {
        self.connected_clients.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Tracks a disconnected WebSocket client.
    pub fn dec_clients(&self) -> usize {
        self.connected_clients
            .fetch_sub(1, Ordering::Relaxed)
            .saturating_sub(1)
    }

    pub fn config(&self) -> &ServerConfig {
        &self.config
    }
}

/// Production Simulation Host managing the isolated background runner thread.
pub struct SimulationHost {
    handle: HostHandle,
    shutdown_flag: Arc<AtomicBool>,
    runner_handle: Option<JoinHandle<()>>,
}

impl SimulationHost {
    /// Initializes and starts the simulation host on a dedicated background thread.
    pub fn start(
        config: ServerConfig,
        engine: SimulationEngine,
        tag_store: RuntimeTagStore,
        alarm_system: Option<AlarmSystem>,
        control_system: Option<ControlSystem>,
        historian_reader: Option<Arc<dyn nebudcs_historian::HistorianReader>>,
    ) -> ServerResult<Self> {
        Self::start_with_scenario(
            config,
            engine,
            tag_store,
            alarm_system,
            control_system,
            Some(nebudcs_scenario::ScenarioEngine::new()),
            historian_reader,
            None,
        )
    }

    /// Initializes and starts the simulation host with optional scenario engine and historian sender.
    #[allow(clippy::too_many_arguments)]
    pub fn start_with_scenario(
        config: ServerConfig,
        engine: SimulationEngine,
        tag_store: RuntimeTagStore,
        alarm_system: Option<AlarmSystem>,
        control_system: Option<ControlSystem>,
        scenario_engine: Option<nebudcs_scenario::ScenarioEngine>,
        historian_reader: Option<Arc<dyn nebudcs_historian::HistorianReader>>,
        historian_sender: Option<nebudcs_historian::HistorianSender>,
    ) -> ServerResult<Self> {
        let (command_tx, command_rx) = bounded::<EnqueuedCommand>(1024);
        let (broadcast_tx, _) = broadcast::channel::<ServerMessage>(config.broadcast_capacity);

        // Initial snapshot and health
        let initial_sim_time = engine.clock().time().seconds();
        let initial_tick = engine.clock().tick_count();
        let initial_snapshot = build_snapshot_dto(
            "init",
            initial_sim_time,
            initial_tick,
            &tag_store,
            alarm_system.as_ref(),
        );

        let initial_health = ServerHealthDto {
            status: "running".to_string(),
            simulation_running: engine.clock().state() == ClockState::Running,
            sim_time_sec: initial_sim_time,
            tick_count: initial_tick,
            simulation_dt: config.simulation_dt,
            speed_multiplier: config.speed_multiplier,
            connected_clients: 0,
            protocol_version: PROTOCOL_VERSION.to_string(),
            historian_healthy: historian_reader.is_some(),
        };

        let initial_scenario_status = match scenario_engine.as_ref() {
            Some(sc) => build_scenario_status_dto(sc),
            None => ScenarioStatusDto {
                scenario_id: None,
                scenario_name: None,
                state: "IDLE".to_string(),
                elapsed_sec: 0.0,
                duration_limit_sec: None,
                active_faults: Vec::new(),
                active_disturbances_count: 0,
                events_total: 0,
                events_fired: 0,
                score: 0.0,
                mandatory_completed: false,
            },
        };

        let snapshot_cache = Arc::new(RwLock::new(initial_snapshot));
        let health_cache = Arc::new(RwLock::new(initial_health));
        let scenario_cache = Arc::new(RwLock::new(initial_scenario_status));
        let connected_clients = Arc::new(AtomicUsize::new(0));
        let shutdown_flag = Arc::new(AtomicBool::new(false));

        let handle = HostHandle {
            command_tx,
            broadcast_tx: broadcast_tx.clone(),
            snapshot_cache: Arc::clone(&snapshot_cache),
            health_cache: Arc::clone(&health_cache),
            scenario_cache: Arc::clone(&scenario_cache),
            connected_clients: Arc::clone(&connected_clients),
            historian_reader,
            config: config.clone(),
        };

        let runner_shutdown = Arc::clone(&shutdown_flag);
        let runner_snapshot = Arc::clone(&snapshot_cache);
        let runner_health = Arc::clone(&health_cache);
        let runner_scenario = Arc::clone(&scenario_cache);
        let runner_config = config;

        let runner_handle = Builder::new()
            .name("nebudcs-sim-runner".to_string())
            .spawn(move || {
                run_simulation_loop(
                    runner_config,
                    engine,
                    tag_store,
                    alarm_system,
                    control_system,
                    scenario_engine,
                    historian_sender,
                    command_rx,
                    broadcast_tx,
                    runner_snapshot,
                    runner_health,
                    runner_scenario,
                    runner_shutdown,
                );
            })
            .map_err(|e| {
                ServerError::Internal(format!("Failed to spawn simulation runner thread: {e}"))
            })?;

        Ok(Self {
            handle,
            shutdown_flag,
            runner_handle: Some(runner_handle),
        })
    }

    /// Access the thread-safe API handle.
    pub fn handle(&self) -> HostHandle {
        self.handle.clone()
    }

    /// Gracefully stops the background simulation runner thread.
    pub fn stop(&mut self) {
        self.shutdown_flag.store(true, Ordering::SeqCst);
        if let Some(h) = self.runner_handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for SimulationHost {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The isolated, deterministic simulation runner loop.
#[allow(clippy::too_many_arguments)]
fn run_simulation_loop(
    config: ServerConfig,
    mut engine: SimulationEngine,
    mut tag_store: RuntimeTagStore,
    mut alarm_system: Option<AlarmSystem>,
    mut control_system: Option<ControlSystem>,
    mut scenario_engine: Option<nebudcs_scenario::ScenarioEngine>,
    historian_sender: Option<nebudcs_historian::HistorianSender>,
    command_rx: Receiver<EnqueuedCommand>,
    broadcast_tx: broadcast::Sender<ServerMessage>,
    snapshot_cache: Arc<RwLock<StateSnapshotDto>>,
    health_cache: Arc<RwLock<ServerHealthDto>>,
    scenario_cache: Arc<RwLock<ScenarioStatusDto>>,
    shutdown_flag: Arc<AtomicBool>,
) {
    let mut seq: u64 = 0;
    let mut is_running = engine.clock().state() == ClockState::Running;
    let mut speed_multiplier = config.speed_multiplier;
    let base_dt = config.simulation_dt;
    let mut last_scenario_state = String::new();
    let mut last_faults_count: usize = 0;
    let mut last_events_fired: usize = 0;

    // Cache previous values to evaluate deadband deltas
    let mut last_values: Vec<f64> = (0..tag_store.len())
        .map(|i| {
            tag_store
                .get_value_by_index(i)
                .and_then(|v| match v {
                    TagValue::Float(f) => Some(*f),
                    TagValue::Integer(int) => Some(*int as f64),
                    TagValue::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
                    _ => None,
                })
                .unwrap_or(0.0)
        })
        .collect();

    let mut last_qualities: Vec<TagQuality> = (0..tag_store.len())
        .map(|i| {
            tag_store
                .get_quality_by_index(i)
                .unwrap_or(TagQuality::Good)
        })
        .collect();

    let mut next_tick_instant = Instant::now();

    while !shutdown_flag.load(Ordering::Relaxed) {
        let mut force_step = false;

        let mut any_commands_applied = false;
        let mut acks_to_send = Vec::new();

        // 1. Drain and apply pending commands at deterministic tick boundary
        while let Ok(cmd_enqueued) = command_rx.try_recv() {
            let result = apply_command(
                &cmd_enqueued.command,
                &mut engine,
                &mut tag_store,
                &mut alarm_system,
                &mut control_system,
                &mut scenario_engine,
                &mut is_running,
                &mut speed_multiplier,
                &mut force_step,
            );
            any_commands_applied = true;
            if let Some(ack) = cmd_enqueued.ack_sender {
                acks_to_send.push((ack, result));
            }
        }

        // 2. Execute tick if running or single-step requested
        let should_tick = is_running || force_step;
        if should_tick {
            if let Some(sc) = scenario_engine.as_mut() {
                if sc.state() == nebudcs_scenario::ScenarioState::Running {
                    let _ = sc.step(
                        engine.clock().time(),
                        base_dt,
                        &mut tag_store,
                        control_system.as_mut(),
                        alarm_system.as_ref(),
                        historian_sender.as_ref(),
                    );
                }
            }

            if let Err(e) = engine.step() {
                eprintln!("[nebudcs-server] Simulation tick failed: {e}");
            }
        }

        if should_tick || any_commands_applied {
            let sim_time_sec = engine.clock().time().seconds();
            let tick_count = engine.clock().tick_count();

            // Check changed tags beyond deadband
            let mut changed_tags = Vec::new();
            for (idx, tag_ref) in tag_store.tags().iter().enumerate() {
                let curr_val = tag_store.get_value_by_index(idx);
                let curr_qual = tag_store
                    .get_quality_by_index(idx)
                    .unwrap_or(TagQuality::Good);

                let Some(val) = curr_val else { continue };
                let val_dto = match val {
                    TagValue::Float(f) => TagValueDto::Float(*f),
                    TagValue::Integer(i) => TagValueDto::Integer(*i),
                    TagValue::Bool(b) => TagValueDto::Bool(*b),
                    TagValue::BadQuality => TagValueDto::Float(f64::NAN),
                    TagValue::Text(_) => continue,
                };

                let val_f = val_dto.as_f64().unwrap_or(0.0);
                let prev_f = last_values.get(idx).copied().unwrap_or(0.0);
                let prev_q = last_qualities.get(idx).copied().unwrap_or(TagQuality::Good);

                let delta = (val_f - prev_f).abs();
                if delta >= config.delta_deadband || curr_qual != prev_q {
                    changed_tags.push(TagValueSampleDto {
                        tag: tag_ref.name.clone(),
                        value: val_dto,
                        quality: match curr_qual {
                            TagQuality::Good => TagQualityDto::Good,
                            TagQuality::Uncertain => TagQualityDto::Uncertain,
                            TagQuality::Bad => TagQualityDto::Bad,
                            TagQuality::NotConnected => TagQualityDto::NotConnected,
                        },
                        unit: None,
                    });

                    if idx < last_values.len() {
                        last_values[idx] = val_f;
                        last_qualities[idx] = curr_qual;
                    }
                }
            }

            // Broadcast delta update if any tags changed
            if !changed_tags.is_empty() {
                seq += 1;
                let delta = DeltaUpdateDto {
                    seq,
                    sim_time_sec,
                    tick_count,
                    changed_tags,
                };
                let _ = broadcast_tx.send(ServerMessage::Delta(delta));
            }

            // Broadcast alarm updates if alarms exist
            if let Some(alarms) = alarm_system.as_ref() {
                if alarms.engine().has_unacknowledged() {
                    let active_alarms = build_alarm_snapshots(alarms);
                    let _ = broadcast_tx.send(ServerMessage::AlarmUpdate(active_alarms));
                }
            }

            // Update scenario status cache and broadcast updates if present
            if let Some(sc) = scenario_engine.as_ref() {
                let sc_dto = build_scenario_status_dto(sc);
                let state_changed = sc_dto.state != last_scenario_state
                    || sc_dto.active_faults.len() != last_faults_count
                    || sc_dto.events_fired != last_events_fired;

                if state_changed
                    || (is_running && tick_count.is_multiple_of(10))
                    || any_commands_applied
                {
                    last_scenario_state = sc_dto.state.clone();
                    last_faults_count = sc_dto.active_faults.len();
                    last_events_fired = sc_dto.events_fired;
                    let _ = broadcast_tx.send(ServerMessage::ScenarioUpdate(sc_dto.clone()));
                }

                if let Ok(mut cache) = scenario_cache.try_write() {
                    *cache = sc_dto;
                }
            }

            // Update snapshot and health cache (try_write avoids blocking sim thread)
            if let Ok(mut snap) = snapshot_cache.try_write() {
                *snap = build_snapshot_dto(
                    &format!("snap-{}", tick_count),
                    sim_time_sec,
                    tick_count,
                    &tag_store,
                    alarm_system.as_ref(),
                );
            }

            if let Ok(mut health) = health_cache.try_write() {
                health.simulation_running = is_running;
                health.sim_time_sec = sim_time_sec;
                health.tick_count = tick_count;
                health.speed_multiplier = speed_multiplier;
            }
        }

        // Send acknowledgments after cache is updated
        for (ack, res) in acks_to_send {
            let _ = ack.send(res);
        }

        // 3. Deterministic pacing sleep
        let pacing_dt = if speed_multiplier > 0.0 {
            base_dt / speed_multiplier
        } else {
            base_dt
        };

        let pace_duration = Duration::from_secs_f64(pacing_dt);
        next_tick_instant += pace_duration;

        let now = Instant::now();
        if next_tick_instant > now {
            std::thread::sleep(next_tick_instant - now);
        } else {
            // Pacing fell behind real-time; resynchronize target instant
            next_tick_instant = now;
            if !is_running {
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
}

/// Applies a validated runtime command to the simulation systems.
#[allow(clippy::too_many_arguments)]
fn apply_command(
    command: &RuntimeCommand,
    engine: &mut SimulationEngine,
    tag_store: &mut RuntimeTagStore,
    alarm_system: &mut Option<AlarmSystem>,
    control_system: &mut Option<ControlSystem>,
    scenario_engine: &mut Option<nebudcs_scenario::ScenarioEngine>,
    is_running: &mut bool,
    speed_multiplier: &mut f64,
    force_step: &mut bool,
) -> ServerResult<()> {
    match command {
        RuntimeCommand::SimControl(SimControlCommand::Play) => {
            *is_running = true;
            engine.clock_mut().play();
            Ok(())
        }
        RuntimeCommand::SimControl(SimControlCommand::Pause) => {
            *is_running = false;
            engine.clock_mut().pause();
            Ok(())
        }
        RuntimeCommand::SimControl(SimControlCommand::Step) => {
            *force_step = true;
            Ok(())
        }
        RuntimeCommand::SimControl(SimControlCommand::SetSpeedMultiplier { multiplier }) => {
            if !multiplier.is_finite() || *multiplier <= 0.0 {
                return Err(ServerError::InvalidRequest(
                    "Speed multiplier must be positive and finite".to_string(),
                ));
            }
            engine
                .clock_mut()
                .set_speed_multiplier(*multiplier)
                .map_err(|e| ServerError::InvalidRequest(e.to_string()))?;
            *speed_multiplier = *multiplier;
            Ok(())
        }
        RuntimeCommand::SimControl(SimControlCommand::Reset) => {
            *is_running = false;
            engine.clock_mut().pause();
            Ok(())
        }
        RuntimeCommand::WriteTag { tag, value } => {
            if let Some(tag_id) = tag_store.resolve_name(tag) {
                match value {
                    TagValueDto::Float(f) => {
                        if !f.is_finite() {
                            return Err(ServerError::InvalidRequest(
                                "Float tag value must be finite".to_string(),
                            ));
                        }
                        tag_store.set_tag_float(tag_id, *f, TagQuality::Good);
                    }
                    TagValueDto::Integer(i) => {
                        tag_store.set_tag(tag_id, TagValue::Integer(*i), TagQuality::Good);
                    }
                    TagValueDto::Bool(b) => {
                        tag_store.set_tag(tag_id, TagValue::Bool(*b), TagQuality::Good);
                    }
                }
                Ok(())
            } else {
                Err(ServerError::NotFound(format!("Tag '{tag}' not found")))
            }
        }
        RuntimeCommand::SetSetpoint { loop_name, sp } => {
            if !sp.is_finite() {
                return Err(ServerError::InvalidRequest(
                    "Setpoint must be finite".to_string(),
                ));
            }
            if let Some(tag_id) = tag_store.resolve_name(loop_name) {
                if let Some(ctrl) = control_system.as_mut() {
                    if let Some(pid) = ctrl.pid_mut(tag_id) {
                        pid.set_sp(*sp);
                        return Ok(());
                    }
                }
            }
            Err(ServerError::NotFound(format!(
                "Controller loop '{loop_name}' not found"
            )))
        }
        RuntimeCommand::SetMode { loop_name, mode } => {
            if let Some(tag_id) = tag_store.resolve_name(loop_name) {
                if let Some(ctrl) = control_system.as_mut() {
                    if let Some(pid) = ctrl.pid_mut(tag_id) {
                        let internal_mode = match mode {
                            PidModeDto::Manual => PidMode::Manual,
                            PidModeDto::Auto => PidMode::Auto,
                            PidModeDto::Cascade => PidMode::Cascade,
                            PidModeDto::RemoteCascade => PidMode::RemoteCascade,
                        };
                        pid.set_mode(internal_mode);
                        return Ok(());
                    }
                }
            }
            Err(ServerError::NotFound(format!(
                "Controller loop '{loop_name}' not found"
            )))
        }
        RuntimeCommand::SetManualOutput { loop_name, output } => {
            if !output.is_finite() {
                return Err(ServerError::InvalidRequest(
                    "Manual output must be finite".to_string(),
                ));
            }
            if let Some(tag_id) = tag_store.resolve_name(loop_name) {
                if let Some(ctrl) = control_system.as_mut() {
                    if let Some(pid) = ctrl.pid_mut(tag_id) {
                        pid.set_manual_output(*output);
                        return Ok(());
                    }
                }
            }
            Err(ServerError::NotFound(format!(
                "Controller loop '{loop_name}' not found"
            )))
        }
        RuntimeCommand::AcknowledgeAlarm { alarm_name } => {
            if let Some(alarms) = alarm_system.as_mut() {
                alarms.engine_mut().acknowledge_alarm(alarm_name);
                Ok(())
            } else {
                Err(ServerError::NotFound("Alarm system not active".to_string()))
            }
        }
        RuntimeCommand::ShelveAlarm {
            alarm_name,
            duration_seconds,
        } => {
            if !duration_seconds.is_finite() || *duration_seconds < 0.0 {
                return Err(ServerError::InvalidRequest(
                    "Shelve duration must be non-negative and finite".to_string(),
                ));
            }
            if let Some(alarms) = alarm_system.as_mut() {
                alarms
                    .engine_mut()
                    .shelve_alarm(alarm_name, *duration_seconds);
                Ok(())
            } else {
                Err(ServerError::NotFound("Alarm system not active".to_string()))
            }
        }
        RuntimeCommand::ResetProtection { block_name } => {
            if let Some(ctrl) = control_system.as_mut() {
                if ctrl.request_protection_reset(block_name) {
                    Ok(())
                } else {
                    Err(ServerError::NotFound(format!(
                        "Protection block '{block_name}' not found"
                    )))
                }
            } else {
                Err(ServerError::NotFound(
                    "Control/protection system not active".to_string(),
                ))
            }
        }
        RuntimeCommand::Scenario(scenario_cmd) => match scenario_cmd {
            ScenarioCommand::Start => {
                if let Some(sc) = scenario_engine.as_mut() {
                    let current_sim_time = engine.clock().time().seconds();
                    sc.arm_and_initialize(current_sim_time, tag_store, control_system.as_mut())
                        .map_err(|e| ServerError::InvalidRequest(e.to_string()))?;
                    sc.start()
                        .map_err(|e| ServerError::InvalidRequest(e.to_string()))?;
                    Ok(())
                } else {
                    Err(ServerError::NotFound(
                        "Scenario engine not active".to_string(),
                    ))
                }
            }
            ScenarioCommand::Pause => {
                if let Some(sc) = scenario_engine.as_mut() {
                    sc.pause()
                        .map_err(|e| ServerError::InvalidRequest(e.to_string()))?;
                    Ok(())
                } else {
                    Err(ServerError::NotFound(
                        "Scenario engine not active".to_string(),
                    ))
                }
            }
            ScenarioCommand::Resume => {
                if let Some(sc) = scenario_engine.as_mut() {
                    sc.resume()
                        .map_err(|e| ServerError::InvalidRequest(e.to_string()))?;
                    Ok(())
                } else {
                    Err(ServerError::NotFound(
                        "Scenario engine not active".to_string(),
                    ))
                }
            }
            ScenarioCommand::Abort { reason } => {
                if let Some(sc) = scenario_engine.as_mut() {
                    sc.abort(reason, tag_store);
                    Ok(())
                } else {
                    Err(ServerError::NotFound(
                        "Scenario engine not active".to_string(),
                    ))
                }
            }
            ScenarioCommand::Reset => {
                if let Some(sc) = scenario_engine.as_mut() {
                    sc.reset(tag_store);
                    Ok(())
                } else {
                    Err(ServerError::NotFound(
                        "Scenario engine not active".to_string(),
                    ))
                }
            }
            ScenarioCommand::Load { scenario_json } => {
                let defn: nebudcs_scenario::ScenarioDefinition =
                    serde_json::from_str(scenario_json).map_err(|e| {
                        ServerError::InvalidRequest(format!("Invalid scenario JSON: {e}"))
                    })?;
                if let Some(sc) = scenario_engine.as_mut() {
                    sc.load_scenario(defn)
                        .map_err(|e| ServerError::InvalidRequest(e.to_string()))?;
                    Ok(())
                } else {
                    Err(ServerError::NotFound(
                        "Scenario engine not active".to_string(),
                    ))
                }
            }
            ScenarioCommand::InjectFault { fault_json } => {
                let fault: nebudcs_scenario::FaultDefinition = serde_json::from_str(fault_json)
                    .map_err(|e| ServerError::InvalidRequest(format!("Invalid fault JSON: {e}")))?;
                if let Some(sc) = scenario_engine.as_mut() {
                    let current_sim_time = engine.clock().time().seconds();
                    sc.inject_fault(fault, current_sim_time, tag_store)
                        .map_err(|e| ServerError::InvalidRequest(e.to_string()))?;
                    Ok(())
                } else {
                    Err(ServerError::NotFound(
                        "Scenario engine not active".to_string(),
                    ))
                }
            }
            ScenarioCommand::ClearFault { fault_id } => {
                if let Some(sc) = scenario_engine.as_mut() {
                    if sc.clear_fault(fault_id, tag_store) {
                        Ok(())
                    } else {
                        Err(ServerError::NotFound(format!(
                            "Fault '{fault_id}' not found"
                        )))
                    }
                } else {
                    Err(ServerError::NotFound(
                        "Scenario engine not active".to_string(),
                    ))
                }
            }
        },
    }
}

/// Constructs a scenario status DTO from internal scenario engine state.
fn build_scenario_status_dto(sc: &nebudcs_scenario::ScenarioEngine) -> ScenarioStatusDto {
    let s = sc.status();
    ScenarioStatusDto {
        scenario_id: s.scenario_id,
        scenario_name: s.scenario_name,
        state: format!("{:?}", s.state).to_uppercase(),
        elapsed_sec: s.elapsed_sec,
        duration_limit_sec: s.duration_limit_sec,
        active_faults: s
            .active_faults
            .into_iter()
            .map(|f| ActiveFaultDto {
                id: f.id,
                target: f.target,
                category: f.category,
                elapsed_sec: f.elapsed_sec,
                duration_sec: f.duration_sec,
            })
            .collect(),
        active_disturbances_count: s.active_disturbances_count,
        events_total: s.events_total,
        events_fired: s.events_fired,
        score: s.score,
        mandatory_completed: s.mandatory_completed,
    }
}

/// Constructs a full state snapshot from current tag store and alarm states.
fn build_snapshot_dto(
    snapshot_id: &str,
    sim_time_sec: f64,
    tick_count: u64,
    tag_store: &RuntimeTagStore,
    alarm_system: Option<&AlarmSystem>,
) -> StateSnapshotDto {
    let mut tags = Vec::with_capacity(tag_store.len());
    for (idx, tag_ref) in tag_store.tags().iter().enumerate() {
        let val = tag_store.get_value_by_index(idx);
        let qual = tag_store
            .get_quality_by_index(idx)
            .unwrap_or(TagQuality::Good);

        let Some(v) = val else { continue };
        let val_dto = match v {
            TagValue::Float(f) => TagValueDto::Float(*f),
            TagValue::Integer(i) => TagValueDto::Integer(*i),
            TagValue::Bool(b) => TagValueDto::Bool(*b),
            TagValue::BadQuality => TagValueDto::Float(f64::NAN),
            TagValue::Text(_) => continue,
        };

        tags.push(TagValueSampleDto {
            tag: tag_ref.name.clone(),
            value: val_dto,
            quality: match qual {
                TagQuality::Good => TagQualityDto::Good,
                TagQuality::Uncertain => TagQualityDto::Uncertain,
                TagQuality::Bad => TagQualityDto::Bad,
                TagQuality::NotConnected => TagQualityDto::NotConnected,
            },
            unit: None,
        });
    }

    let active_alarms = alarm_system.map(build_alarm_snapshots).unwrap_or_default();

    StateSnapshotDto {
        snapshot_id: snapshot_id.to_string(),
        sim_time_sec,
        tick_count,
        tags,
        active_alarms,
    }
}

/// Builds an array of alarm snapshots from the active alarm blocks.
fn build_alarm_snapshots(alarms: &AlarmSystem) -> Vec<AlarmSnapshotDto> {
    alarms
        .engine()
        .alarms()
        .map(|(name, block)| AlarmSnapshotDto {
            alarm_name: name.clone(),
            state: match block.state() {
                nebudcs_alarm::AlarmState::Normal => AlarmStateDto::Normal,
                nebudcs_alarm::AlarmState::ActiveUnack => AlarmStateDto::ActiveUnack,
                nebudcs_alarm::AlarmState::ActiveAck => AlarmStateDto::ActiveAck,
                nebudcs_alarm::AlarmState::Cleared => AlarmStateDto::Cleared,
                nebudcs_alarm::AlarmState::Shelved => AlarmStateDto::Shelved,
            },
            severity: match block.config().severity {
                nebudcs_alarm::AlarmSeverity::Diagnostic => AlarmSeverityDto::Diagnostic,
                nebudcs_alarm::AlarmSeverity::Advisory => AlarmSeverityDto::Advisory,
                nebudcs_alarm::AlarmSeverity::Warning => AlarmSeverityDto::Warning,
                nebudcs_alarm::AlarmSeverity::Critical => AlarmSeverityDto::Critical,
            },
            pv: block.last_pv(),
            trip_limit: block.config().limit,
        })
        .collect()
}
