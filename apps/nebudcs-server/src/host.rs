//! Central simulation host and deterministic execution thread.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{Builder, JoinHandle};
use std::time::{Duration, Instant};

use crossbeam_channel::{bounded, Receiver, Sender};
use tokio::sync::{broadcast, RwLock};

use nebudcs_alarm::{AlarmBlockConfig, AlarmSeverity, AlarmSystem, AlarmType};
use nebudcs_control::actuator::{ActuatorConfig, CommandPriority};
use nebudcs_control::advanced::{
    AdvancedControlStrategy, AntiSurgeController, LeadLagCompensator, OverrideSelector,
    RatioController, SelectorType, SplitRangeController, StrategyKind, ValveCharacteristic,
    ValveCharacterizer,
};
use nebudcs_control::hal::{IoChannel, IoSystem};
use nebudcs_control::instrument::InstrumentConfig;
use nebudcs_control::pid::{PidConfig, PidController, PidDirection, PidMode};
use nebudcs_control::protection::TripDirection;
use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_control::ControlSystem;
use nebudcs_model::tag::{TagQuality, TagValue};
use nebudcs_physics::equipment::{
    AccumulatorVolumeModel, CheckValveModel, DistillationColumnModel, FlowResistanceModel,
    GenericEquipmentPhysics, HeadSourceModel, HeatExchangeModel, ProcessThermalModel,
    TurbineExpansionModel,
};
use nebudcs_physics::system::{CouplingKind, GenericPhysicsSystem, ProcessCoupling};
use nebudcs_protocol::*;
use nebudcs_sim::{ClockState, SimulationEngine, SimulationSystem};
use nebudcs_types::{ControllerId, EquipmentId, TagId, TaskId};
use nebudcs_units::Density;

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
    equipment_manager: Arc<std::sync::RwLock<nebudcs_scripting::CustomEquipmentManager>>,
    project_manager: Arc<std::sync::RwLock<crate::project_manager::ProjectManager>>,
    scenario_manager: Arc<std::sync::RwLock<crate::scenario_manager::ScenarioManager>>,
    training_manager: Arc<std::sync::RwLock<crate::training_manager::TrainingManager>>,
    rate_limiter: Arc<crate::security::RateLimiter>,
    disk_store: Arc<crate::persistence::DiskStore>,
    controllers_cache: Arc<RwLock<Vec<ControllerDetailDto>>>,
    hal_cache: Arc<RwLock<Option<HalSystemSummaryDto>>>,
    network_cache: Arc<RwLock<nebudcs_physics::network::NetworkDiagnostics>>,
    communications_manager:
        Arc<std::sync::RwLock<crate::communications_manager::CommunicationsManager>>,
    sis_cache: Arc<RwLock<Vec<SifDetailDto>>>,
    cause_effect_cache: Arc<RwLock<CauseEffectMatrixDto>>,
    sis_summary_cache: Arc<RwLock<SisStatusSummaryDto>>,
    validation_manager: Arc<std::sync::RwLock<crate::validation_manager::ValidationManager>>,
    cluster_manager: Arc<std::sync::RwLock<crate::cluster_manager::ClusterManager>>,
    audit_manager: Arc<std::sync::RwLock<crate::audit_manager::AuditManager>>,
    diagnostics_manager: Arc<std::sync::RwLock<crate::diagnostics_manager::DiagnosticsManager>>,
    pub security_manager: Arc<std::sync::RwLock<crate::security::CyberSecurityManager>>,
}

impl HostHandle {
    /// Returns current hydraulic network diagnostics.
    pub async fn get_network_diagnostics(&self) -> nebudcs_physics::network::NetworkDiagnostics {
        self.network_cache.read().await.clone()
    }

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

    /// Obtains a summary list of all configured virtual controllers.
    pub async fn get_controllers(&self) -> Vec<ControllerSummaryDto> {
        let lock = self.controllers_cache.read().await;
        lock.iter()
            .map(|c| ControllerSummaryDto {
                id: c.id.clone(),
                name: c.name.clone(),
                lifecycle: c.lifecycle.clone(),
                task_count: c.tasks.len(),
                scan_count: c.scan_count,
                avg_scan_duration_us: c.avg_scan_duration_us,
                max_scan_duration_us: c.max_scan_duration_us,
                watchdog_trips: c.watchdog_trips,
                overrun_count: c.overrun_count,
                is_faulted: c.fault.is_some(),
            })
            .collect()
    }

    /// Obtains detailed state and diagnostics for a specific controller by ID or name.
    pub async fn get_controller(&self, id_or_name: &str) -> Option<ControllerDetailDto> {
        let lock = self.controllers_cache.read().await;
        lock.iter()
            .find(|c| c.id == id_or_name || c.name == id_or_name)
            .cloned()
    }

    /// Obtains summary and diagnostics for all configured I/O HAL systems.
    pub async fn get_hal_systems(&self) -> Vec<HalSystemSummaryDto> {
        let lock = self.hal_cache.read().await;
        if let Some(ref hal) = *lock {
            vec![hal.clone()]
        } else {
            Vec::new()
        }
    }

    /// Obtains detailed channel status and diagnostics for a specific I/O channel address.
    pub async fn get_hal_channel(&self, address: &str) -> Option<HalChannelDto> {
        let parsed: Option<nebudcs_control::hal::IoChannelAddress> = address.parse().ok();
        let lock = self.hal_cache.read().await;
        let hal = lock.as_ref()?;
        for rack in &hal.racks {
            for chassis in &rack.chassis {
                for module in &chassis.modules {
                    for channel in &module.channels {
                        if channel.address == address {
                            return Some(channel.clone());
                        }
                        if let Some(ref target) = parsed {
                            if let Ok(ch_addr) = channel
                                .address
                                .parse::<nebudcs_control::hal::IoChannelAddress>()
                            {
                                if ch_addr == *target {
                                    return Some(channel.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Lists all active controller-to-I/O mappings.
    pub async fn get_hal_mappings(&self) -> Vec<HalMappingDto> {
        let lock = self.hal_cache.read().await;
        if let Some(ref hal) = *lock {
            hal.mappings.clone()
        } else {
            Vec::new()
        }
    }

    /// Subscribes to the broadcast channel for real-time WebSocket state updates.
    pub fn subscribe_broadcast(&self) -> broadcast::Receiver<ServerMessage> {
        self.broadcast_tx.subscribe()
    }

    /// Obtains live telemetry and diagnostics for all configured Safety Instrumented Functions.
    pub async fn get_sifs(&self) -> Vec<SifDetailDto> {
        self.sis_cache.read().await.clone()
    }

    /// Obtains detailed telemetry for a specific SIF by ID.
    pub async fn get_sif(&self, id: &str) -> Option<SifDetailDto> {
        let sifs = self.sis_cache.read().await;
        sifs.iter().find(|s| s.id == id).cloned()
    }

    /// Obtains the dynamically generated Cause & Effect Matrix for all SIFs.
    pub async fn get_cause_effect_matrix(&self) -> CauseEffectMatrixDto {
        self.cause_effect_cache.read().await.clone()
    }

    /// Obtains high-level SIS executive status summary.
    pub async fn get_sis_summary(&self) -> SisStatusSummaryDto {
        self.sis_summary_cache.read().await.clone()
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

    /// Access to the custom equipment manager.
    pub fn equipment_manager(
        &self,
    ) -> &Arc<std::sync::RwLock<nebudcs_scripting::CustomEquipmentManager>> {
        &self.equipment_manager
    }

    /// Access to the engineering studio project manager.
    pub fn project_manager(
        &self,
    ) -> &Arc<std::sync::RwLock<crate::project_manager::ProjectManager>> {
        &self.project_manager
    }

    /// Access to the Training Studio scenario manager.
    pub fn scenario_manager(
        &self,
    ) -> &Arc<std::sync::RwLock<crate::scenario_manager::ScenarioManager>> {
        &self.scenario_manager
    }

    /// Access to the Training Studio training session manager.
    pub fn training_manager(
        &self,
    ) -> &Arc<std::sync::RwLock<crate::training_manager::TrainingManager>> {
        &self.training_manager
    }

    /// Access to the Scientific Validation & Certification manager.
    pub fn validation_manager(
        &self,
    ) -> &Arc<std::sync::RwLock<crate::validation_manager::ValidationManager>> {
        &self.validation_manager
    }

    /// Access to the server-wide token-bucket rate limiter.
    pub fn rate_limiter(&self) -> &Arc<crate::security::RateLimiter> {
        &self.rate_limiter
    }

    /// Access to the persistent atomic DiskStore.
    pub fn disk_store(&self) -> &Arc<crate::persistence::DiskStore> {
        &self.disk_store
    }

    /// Access to the industrial communications manager.
    pub fn communications_manager(
        &self,
    ) -> &Arc<std::sync::RwLock<crate::communications_manager::CommunicationsManager>> {
        &self.communications_manager
    }

    /// Access to the enterprise cluster & HA manager.
    pub fn cluster_manager(
        &self,
    ) -> &Arc<std::sync::RwLock<crate::cluster_manager::ClusterManager>> {
        &self.cluster_manager
    }

    /// Access to the unified platform audit manager.
    pub fn audit_manager(
        &self,
    ) -> &Arc<std::sync::RwLock<crate::audit_manager::AuditManager>> {
        &self.audit_manager
    }

    /// Access to the global unified diagnostics manager.
    pub fn diagnostics_manager(
        &self,
    ) -> &Arc<std::sync::RwLock<crate::diagnostics_manager::DiagnosticsManager>> {
        &self.diagnostics_manager
    }

    /// Access to the industrial cybersecurity manager (IEC 62443 / PERA).
    pub fn security_manager(
        &self,
    ) -> &Arc<std::sync::RwLock<crate::security::CyberSecurityManager>> {
        &self.security_manager
    }

    /// Loads and synchronizes an authored project into the live simulation host.
    pub async fn load_project_into_simulation(&self, project_id: &str) -> ServerResult<()> {
        let project_dto = {
            let mgr = self.project_manager.read().map_err(|e| {
                ServerError::Internal(format!("Failed to acquire project manager lock: {e}"))
            })?;
            mgr.get_project(project_id)
                .ok_or_else(|| ServerError::NotFound(format!("Project '{project_id}' not found")))?
        };

        if let Ok(mut comms) = self.communications_manager.write() {
            comms.load_project_communications(&project_dto.communications);
        }

        self.dispatch_command(RuntimeCommand::LoadProject {
            project: Box::new(project_dto),
        })
        .await?;

        Ok(())
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
        Self::start_with_all(
            config,
            engine,
            tag_store,
            alarm_system,
            control_system,
            Some(nebudcs_scenario::ScenarioEngine::new()),
            Some(Arc::new(std::sync::RwLock::new(
                nebudcs_scripting::CustomEquipmentManager::new(),
            ))),
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
        Self::start_with_all(
            config,
            engine,
            tag_store,
            alarm_system,
            control_system,
            scenario_engine,
            Some(Arc::new(std::sync::RwLock::new(
                nebudcs_scripting::CustomEquipmentManager::new(),
            ))),
            historian_reader,
            historian_sender,
        )
    }

    /// Initializes and starts the simulation host with all optional subsystems.
    #[allow(clippy::too_many_arguments)]
    pub fn start_with_all(
        config: ServerConfig,
        engine: SimulationEngine,
        tag_store: RuntimeTagStore,
        alarm_system: Option<AlarmSystem>,
        control_system: Option<ControlSystem>,
        scenario_engine: Option<nebudcs_scenario::ScenarioEngine>,
        equipment_manager: Option<
            Arc<std::sync::RwLock<nebudcs_scripting::CustomEquipmentManager>>,
        >,
        historian_reader: Option<Arc<dyn nebudcs_historian::HistorianReader>>,
        historian_sender: Option<nebudcs_historian::HistorianSender>,
    ) -> ServerResult<Self> {
        let (command_tx, command_rx) = bounded::<EnqueuedCommand>(1024);
        let (broadcast_tx, _) = broadcast::channel::<ServerMessage>(config.broadcast_capacity);

        let equipment_manager = equipment_manager.unwrap_or_else(|| {
            Arc::new(std::sync::RwLock::new(
                nebudcs_scripting::CustomEquipmentManager::new(),
            ))
        });

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

        let disk_store = Arc::new(
            crate::persistence::DiskStore::new(&config.data_dir)
                .unwrap_or_else(|_| crate::persistence::DiskStore::new("data").unwrap()),
        );

        let rate_limiter = Arc::new(crate::security::RateLimiter::new(
            config.rate_limit_burst,
            config.rate_limit_rps,
        ));

        let project_manager = Arc::new(std::sync::RwLock::new(
            crate::project_manager::ProjectManager::new().with_disk_store((*disk_store).clone()),
        ));

        let scenario_manager = Arc::new(std::sync::RwLock::new(
            crate::scenario_manager::ScenarioManager::new().with_disk_store((*disk_store).clone()),
        ));

        let training_manager = Arc::new(std::sync::RwLock::new(
            crate::training_manager::TrainingManager::new().with_disk_store((*disk_store).clone()),
        ));

        let validation_manager = Arc::new(std::sync::RwLock::new(
            crate::validation_manager::ValidationManager::new()
                .with_data_dir(config.data_dir.clone()),
        ));

        let communications_manager = Arc::new(std::sync::RwLock::new(
            crate::communications_manager::CommunicationsManager::new(),
        ));

        let mut control_system = control_system;
        if let Some(ref mut cs) = control_system {
            if cs.io_system().racks.is_empty() {
                cs.set_io_system(build_default_hal_system());
            }
        } else {
            let mut cs = ControlSystem::new();
            cs.set_io_system(build_default_hal_system());
            control_system = Some(cs);
        }

        let initial_controllers = if let Some(ctrl) = control_system.as_ref() {
            build_controllers_dtos(ctrl)
        } else {
            Vec::new()
        };
        let controllers_cache = Arc::new(RwLock::new(initial_controllers));

        let initial_hal = control_system
            .as_ref()
            .map(|ctrl| build_hal_dto(ctrl.io_system()));
        let hal_cache = Arc::new(RwLock::new(initial_hal));
        let network_cache = Arc::new(RwLock::new(
            nebudcs_physics::network::NetworkDiagnostics::default(),
        ));

        let initial_sifs = if let Some(ctrl) = control_system.as_ref() {
            build_sis_dtos(ctrl)
        } else {
            Vec::new()
        };
        let sis_cache = Arc::new(RwLock::new(initial_sifs));

        let initial_ce = if let Some(ctrl) = control_system.as_ref() {
            build_cause_effect_dto(ctrl)
        } else {
            CauseEffectMatrixDto {
                causes: Vec::new(),
                effects: Vec::new(),
                cells: Vec::new(),
            }
        };
        let cause_effect_cache = Arc::new(RwLock::new(initial_ce));

        let initial_sis_summary = if let Some(ctrl) = control_system.as_ref() {
            build_sis_summary_dto(ctrl)
        } else {
            SisStatusSummaryDto {
                enabled: true,
                sif_count: 0,
                armed_count: 0,
                tripped_count: 0,
                bypassed_count: 0,
                degraded_count: 0,
                proof_tests_count: 0,
                active_overrides_count: 0,
            }
        };
        let sis_summary_cache = Arc::new(RwLock::new(initial_sis_summary));

        let cluster_manager = Arc::new(std::sync::RwLock::new(crate::cluster_manager::ClusterManager::new(
            "nebudcs-cluster-alpha",
            "node-sim-01",
            "Control/Simulation Primary",
            nebudcs_protocol::NodeRole::ControlSimulation,
            nebudcs_protocol::HaState::Primary,
        )));
        let audit_manager = Arc::new(std::sync::RwLock::new(crate::audit_manager::AuditManager::new()));
        let diagnostics_manager = Arc::new(std::sync::RwLock::new(crate::diagnostics_manager::DiagnosticsManager::new()));
        let security_manager = Arc::new(std::sync::RwLock::new(crate::security::CyberSecurityManager::new()));

        let handle = HostHandle {
            command_tx,
            broadcast_tx: broadcast_tx.clone(),
            snapshot_cache: Arc::clone(&snapshot_cache),
            health_cache: Arc::clone(&health_cache),
            scenario_cache: Arc::clone(&scenario_cache),
            connected_clients: Arc::clone(&connected_clients),
            historian_reader,
            config: config.clone(),
            equipment_manager: Arc::clone(&equipment_manager),
            project_manager,
            scenario_manager,
            training_manager: Arc::clone(&training_manager),
            rate_limiter,
            disk_store,
            controllers_cache: Arc::clone(&controllers_cache),
            hal_cache: Arc::clone(&hal_cache),
            network_cache: Arc::clone(&network_cache),
            communications_manager: Arc::clone(&communications_manager),
            sis_cache: Arc::clone(&sis_cache),
            cause_effect_cache: Arc::clone(&cause_effect_cache),
            sis_summary_cache: Arc::clone(&sis_summary_cache),
            validation_manager: Arc::clone(&validation_manager),
            cluster_manager,
            audit_manager,
            diagnostics_manager,
            security_manager,
        };

        let runner_shutdown = Arc::clone(&shutdown_flag);
        let runner_snapshot = Arc::clone(&snapshot_cache);
        let runner_health = Arc::clone(&health_cache);
        let runner_scenario = Arc::clone(&scenario_cache);
        let runner_equipment = Arc::clone(&equipment_manager);
        let runner_training = Arc::clone(&training_manager);
        let runner_controllers = Arc::clone(&controllers_cache);
        let runner_hal = Arc::clone(&hal_cache);
        let runner_network = Arc::clone(&network_cache);
        let runner_comms = Arc::clone(&communications_manager);
        let runner_sis = Arc::clone(&sis_cache);
        let runner_cause_effect = Arc::clone(&cause_effect_cache);
        let runner_sis_summary = Arc::clone(&sis_summary_cache);
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
                    runner_equipment,
                    historian_sender,
                    command_rx,
                    broadcast_tx,
                    runner_snapshot,
                    runner_health,
                    runner_scenario,
                    runner_shutdown,
                    runner_training,
                    runner_controllers,
                    runner_hal,
                    runner_network,
                    runner_comms,
                    runner_sis,
                    runner_cause_effect,
                    runner_sis_summary,
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
    equipment_manager: Arc<std::sync::RwLock<nebudcs_scripting::CustomEquipmentManager>>,
    historian_sender: Option<nebudcs_historian::HistorianSender>,
    command_rx: Receiver<EnqueuedCommand>,
    broadcast_tx: broadcast::Sender<ServerMessage>,
    snapshot_cache: Arc<RwLock<StateSnapshotDto>>,
    health_cache: Arc<RwLock<ServerHealthDto>>,
    scenario_cache: Arc<RwLock<ScenarioStatusDto>>,
    shutdown_flag: Arc<AtomicBool>,
    training_manager: Arc<std::sync::RwLock<crate::training_manager::TrainingManager>>,
    controllers_cache: Arc<RwLock<Vec<ControllerDetailDto>>>,
    hal_cache: Arc<RwLock<Option<HalSystemSummaryDto>>>,
    network_cache: Arc<RwLock<nebudcs_physics::network::NetworkDiagnostics>>,
    runner_comms: Arc<std::sync::RwLock<crate::communications_manager::CommunicationsManager>>,
    sis_cache: Arc<RwLock<Vec<SifDetailDto>>>,
    cause_effect_cache: Arc<RwLock<CauseEffectMatrixDto>>,
    sis_summary_cache: Arc<RwLock<SisStatusSummaryDto>>,
) {
    let mut seq: u64 = 0;
    let mut is_running = engine.clock().state() == ClockState::Running;
    let mut speed_multiplier = config.speed_multiplier;
    let base_dt = config.simulation_dt;
    let mut last_scenario_state = String::new();
    let mut last_faults_count: usize = 0;
    let mut last_events_fired: usize = 0;

    let mut physics_system = GenericPhysicsSystem::new();
    physics_system.set_utility("STEAM_PRESSURE", 40.0);
    physics_system.set_utility("COOLING_WATER", 3.0);
    physics_system.set_utility("INSTRUMENT_AIR", 7.0);

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
                &mut physics_system,
                &mut alarm_system,
                &mut control_system,
                &mut scenario_engine,
                &equipment_manager,
                &mut is_running,
                &mut speed_multiplier,
                &mut force_step,
                &mut last_values,
                &mut last_qualities,
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

            if let Ok(mut eq_mgr) = equipment_manager.write() {
                let empty_inputs = std::collections::HashMap::new();
                let _ = eq_mgr.step(base_dt, &empty_inputs, &mut tag_store);
            }

            let tick_res = engine.step_with_callback(|ctx, read_state, write_state| {
                for (idx, tag_ref) in tag_store.tags().iter().enumerate() {
                    if let Some(val) = tag_store.get_value_by_index(idx) {
                        let qual = tag_store
                            .get_quality_by_index(idx)
                            .unwrap_or(TagQuality::Good);
                        write_state.set_tag(tag_ref.id, val.clone(), qual);
                    }
                }

                physics_system.step(ctx, read_state, write_state)?;
                if let Some(ctrl) = control_system.as_mut() {
                    let air_ok = physics_system
                        .get_utility("INSTRUMENT_AIR")
                        .map(|p| p >= 2.0)
                        .unwrap_or(true);
                    ctrl.set_all_actuators_power_air(air_ok);

                    ctrl.step(ctx, read_state, write_state)?;
                    for (name, block) in ctrl.protections() {
                        let status_tag = format!("{name}.TRIPPED");
                        if let Some(tid) = tag_store.resolve_name(&status_tag) {
                            write_state.set_tag(
                                tid,
                                TagValue::Bool(block.is_tripped()),
                                TagQuality::Good,
                            );
                        }
                    }
                }
                if let Some(alarms) = alarm_system.as_mut() {
                    alarms.step(ctx, read_state, write_state)?;
                }
                Ok(())
            });

            if let Err(e) = tick_res {
                eprintln!("[nebudcs-server] Simulation tick failed: {e}");
            } else {
                for (tid, val) in &engine.committed_buffer().tags {
                    let qual = engine
                        .committed_buffer()
                        .get_quality(*tid)
                        .unwrap_or(TagQuality::Good);
                    tag_store.set_tag(*tid, val.clone(), qual);
                }
            }
        }

        if should_tick || any_commands_applied {
            let sim_time_sec = engine.clock().time().seconds();
            let tick_count = engine.clock().tick_count();

            // Continuously stream simulation telemetry into active training sessions
            if should_tick {
                if let Ok(mut cm) = runner_comms.try_write() {
                    cm.sync_external_values(&mut tag_store, (sim_time_sec * 1000.0) as u64);
                }

                if let Ok(mut tm) = training_manager.try_write() {
                    let mut tag_samples = Vec::with_capacity(tag_store.len());
                    for (idx, tag_ref) in tag_store.tags().iter().enumerate() {
                        if let Some(val) = tag_store.get_value_by_index(idx) {
                            let val_f = match val {
                                TagValue::Float(f) => *f,
                                TagValue::Integer(i) => *i as f64,
                                TagValue::Bool(b) => {
                                    if *b {
                                        1.0
                                    } else {
                                        0.0
                                    }
                                }
                                _ => continue,
                            };
                            tag_samples.push((tag_ref.name.clone(), val_f));
                        }
                    }
                    tm.ingest_tick(sim_time_sec, &tag_samples);
                }
            }

            if tag_store.len() != last_values.len() {
                last_values.resize(tag_store.len(), 0.0);
                last_qualities.resize(tag_store.len(), TagQuality::Good);
            }

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

            // Broadcast delta update if any tags changed, or periodically (every 25 ticks / ~500ms)
            // or when commands were applied, ensuring client clock and state advancement.
            let should_broadcast_delta = !changed_tags.is_empty()
                || (is_running && tick_count.is_multiple_of(25))
                || any_commands_applied;

            if should_broadcast_delta {
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

            if let Some(ctrl_sys) = control_system.as_ref() {
                if let Ok(mut cache) = controllers_cache.try_write() {
                    *cache = build_controllers_dtos(ctrl_sys);
                }
                if let Ok(mut cache) = hal_cache.try_write() {
                    *cache = Some(build_hal_dto(ctrl_sys.io_system()));
                }
                if let Ok(mut cache) = sis_cache.try_write() {
                    *cache = build_sis_dtos(ctrl_sys);
                }
                if let Ok(mut cache) = cause_effect_cache.try_write() {
                    *cache = build_cause_effect_dto(ctrl_sys);
                }
                if let Ok(mut cache) = sis_summary_cache.try_write() {
                    *cache = build_sis_summary_dto(ctrl_sys);
                }
            }

            if let Ok(mut cache) = network_cache.try_write() {
                *cache = physics_system.network_diagnostics().clone();
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

fn build_advanced_control_strategy(
    strat_dto: &AdvancedControlStrategyDto,
    tag_store: &mut RuntimeTagStore,
    eq_id_map: &std::collections::HashMap<String, EquipmentId>,
) -> Option<AdvancedControlStrategy> {
    let mut get_or_create_tag = |tag_name: &str| -> TagId {
        tag_store.resolve_name(tag_name).unwrap_or_else(|| {
            let id = TagId::new();
            tag_store.register_tag(tag_name, id, TagValue::Float(0.0), TagQuality::Good);
            id
        })
    };

    let parse_f64 = |key: &str, default: f64| -> f64 {
        strat_dto
            .parameters
            .get(key)
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(default)
    };

    let resolve_actuator = |key: &str| -> Option<EquipmentId> {
        strat_dto.parameters.get(key).and_then(|name| {
            eq_id_map
                .get(name)
                .copied()
                .or_else(|| name.parse::<EquipmentId>().ok())
        })
    };

    let kind = match strat_dto.strategy_type.to_ascii_uppercase().as_str() {
        "CASCADE" => {
            let p_pv_name = strat_dto.parameters.get("primary_pv")?;
            let s_pv_name = strat_dto.parameters.get("secondary_pv")?;
            let primary_pv = get_or_create_tag(p_pv_name);
            let secondary_pv = get_or_create_tag(s_pv_name);
            let primary_sp = parse_f64("primary_sp", 50.0);

            let primary_dir = if strat_dto
                .parameters
                .get("primary_direction")
                .map(|d| d.eq_ignore_ascii_case("direct"))
                .unwrap_or(false)
            {
                PidDirection::Direct
            } else {
                PidDirection::Reverse
            };

            let secondary_dir = if strat_dto
                .parameters
                .get("secondary_direction")
                .map(|d| d.eq_ignore_ascii_case("direct"))
                .unwrap_or(false)
            {
                PidDirection::Direct
            } else {
                PidDirection::Reverse
            };

            let mut primary_pid = PidController::new(PidConfig {
                kp: parse_f64("primary_kp", 1.5),
                ti: parse_f64("primary_ti", 10.0),
                td: parse_f64("primary_td", 0.0),
                out_min: parse_f64("secondary_sp_min", 0.0),
                out_max: parse_f64("secondary_sp_max", 100.0),
                direction: primary_dir,
                ..Default::default()
            });
            primary_pid.set_mode(PidMode::Auto);
            primary_pid.set_sp(primary_sp);

            let mut secondary_pid = PidController::new(PidConfig {
                kp: parse_f64("secondary_kp", 1.0),
                ti: parse_f64("secondary_ti", 2.0),
                td: parse_f64("secondary_td", 0.1),
                out_min: parse_f64("output_min", 0.0),
                out_max: parse_f64("output_max", 100.0),
                direction: secondary_dir,
                ..Default::default()
            });
            secondary_pid.set_mode(PidMode::Cascade);
            secondary_pid.set_sp_limits(
                parse_f64("secondary_sp_min", 0.0),
                parse_f64("secondary_sp_max", 100.0),
            );

            let output_tag = strat_dto
                .parameters
                .get("output_tag")
                .map(|t| get_or_create_tag(t));
            let target_actuator = resolve_actuator("target_actuator");

            StrategyKind::Cascade {
                primary_pv,
                primary_sp,
                primary_pid,
                secondary_pv,
                secondary_pid,
                output_tag,
                target_actuator,
            }
        }
        "RATIO" => {
            let wild_name = strat_dto.parameters.get("wild_pv")?;
            let ctrl_name = strat_dto.parameters.get("controlled_pv")?;
            let wild_pv = get_or_create_tag(wild_name);
            let controlled_pv = get_or_create_tag(ctrl_name);

            let mut ratio_controller = RatioController::new(parse_f64("ratio", 1.0));
            ratio_controller.set_bias(parse_f64("bias", 0.0));
            ratio_controller
                .set_ratio_limits(parse_f64("ratio_min", 0.01), parse_f64("ratio_max", 10.0));

            let ratio_dir = if strat_dto
                .parameters
                .get("direction")
                .map(|d| d.eq_ignore_ascii_case("direct"))
                .unwrap_or(false)
            {
                PidDirection::Direct
            } else {
                PidDirection::Reverse
            };

            let mut pid = PidController::new(PidConfig {
                kp: parse_f64("kp", 1.2),
                ti: parse_f64("ti", 5.0),
                td: parse_f64("td", 0.0),
                out_min: parse_f64("output_min", 0.0),
                out_max: parse_f64("output_max", 100.0),
                direction: ratio_dir,
                ..Default::default()
            });
            pid.set_mode(PidMode::Auto);

            let output_tag = strat_dto
                .parameters
                .get("output_tag")
                .map(|t| get_or_create_tag(t));
            let target_actuator = resolve_actuator("target_actuator");

            StrategyKind::Ratio {
                wild_pv,
                controlled_pv,
                ratio_controller,
                pid,
                output_tag,
                target_actuator,
            }
        }
        "FEEDFORWARD" => {
            let pv_name = strat_dto.parameters.get("pv_tag")?;
            let dist_name = strat_dto.parameters.get("disturbance_tag")?;
            let pv_tag = get_or_create_tag(pv_name);
            let disturbance_tag = get_or_create_tag(dist_name);
            let sp = parse_f64("sp", 50.0);

            let ff_dir = if strat_dto
                .parameters
                .get("direction")
                .map(|d| d.eq_ignore_ascii_case("direct"))
                .unwrap_or(false)
            {
                PidDirection::Direct
            } else {
                PidDirection::Reverse
            };

            let mut pid = PidController::new(PidConfig {
                kp: parse_f64("kp", 1.5),
                ti: parse_f64("ti", 5.0),
                td: parse_f64("td", 0.0),
                out_min: parse_f64("output_min", 0.0),
                out_max: parse_f64("output_max", 100.0),
                direction: ff_dir,
                ..Default::default()
            });
            pid.set_mode(PidMode::Auto);
            pid.set_sp(sp);

            let lead_lag = LeadLagCompensator::with_gain(
                parse_f64("t_lead", 2.0),
                parse_f64("t_lag", 1.0),
                parse_f64("gain", 1.0),
            );

            let output_tag = strat_dto
                .parameters
                .get("output_tag")
                .map(|t| get_or_create_tag(t));
            let target_actuator = resolve_actuator("target_actuator");

            StrategyKind::FeedForward {
                pv_tag,
                sp,
                pid,
                disturbance_tag,
                lead_lag,
                output_tag,
                target_actuator,
            }
        }
        "OVERRIDE" => {
            let prim_name = strat_dto.parameters.get("primary_pv")?;
            let constr_name = strat_dto.parameters.get("constraint_pv")?;
            let primary_pv = get_or_create_tag(prim_name);
            let constraint_pv = get_or_create_tag(constr_name);
            let primary_sp = parse_f64("primary_sp", 50.0);
            let constraint_limit = parse_f64("constraint_limit", 80.0);

            let mut primary_pid = PidController::new(PidConfig {
                kp: parse_f64("primary_kp", 1.0),
                ti: parse_f64("primary_ti", 5.0),
                out_min: parse_f64("output_min", 0.0),
                out_max: parse_f64("output_max", 100.0),
                ..Default::default()
            });
            primary_pid.set_mode(PidMode::Auto);
            primary_pid.set_sp(primary_sp);

            let is_reverse = strat_dto
                .parameters
                .get("constraint_direction")
                .map(|d| d.eq_ignore_ascii_case("reverse"))
                .unwrap_or(true);
            let mut constraint_pid = PidController::new(PidConfig {
                kp: parse_f64("constraint_kp", 2.0),
                ti: parse_f64("constraint_ti", 3.0),
                out_min: parse_f64("output_min", 0.0),
                out_max: parse_f64("output_max", 100.0),
                direction: if is_reverse {
                    PidDirection::Reverse
                } else {
                    PidDirection::Direct
                },
                ..Default::default()
            });
            constraint_pid.set_mode(PidMode::Auto);
            constraint_pid.set_sp(constraint_limit);

            let sel_type = if strat_dto
                .parameters
                .get("selector_type")
                .map(|s| s.eq_ignore_ascii_case("high"))
                .unwrap_or(false)
            {
                SelectorType::High
            } else {
                SelectorType::Low
            };
            let selector = OverrideSelector::new(sel_type);

            let output_tag = strat_dto
                .parameters
                .get("output_tag")
                .map(|t| get_or_create_tag(t));
            let target_actuator = resolve_actuator("target_actuator");

            StrategyKind::Override {
                primary_pv,
                primary_sp,
                primary_pid,
                constraint_pv,
                constraint_limit,
                constraint_pid,
                selector,
                output_tag,
                target_actuator,
            }
        }
        "SPLITRANGE" => {
            let pv_name = strat_dto.parameters.get("pv_tag")?;
            let pv_tag = get_or_create_tag(pv_name);
            let sp = parse_f64("sp", 50.0);

            let mut master_pid = PidController::new(PidConfig {
                kp: parse_f64("kp", 1.5),
                ti: parse_f64("ti", 5.0),
                out_min: 0.0,
                out_max: 100.0,
                ..Default::default()
            });
            master_pid.set_mode(PidMode::Auto);
            master_pid.set_sp(sp);

            let name_a = strat_dto
                .parameters
                .get("split_a_name")
                .cloned()
                .unwrap_or_else(|| "ValveA".to_string());
            let name_b = strat_dto
                .parameters
                .get("split_b_name")
                .cloned()
                .unwrap_or_else(|| "ValveB".to_string());
            let split_range = SplitRangeController::standard_complementary(&name_a, &name_b);

            let split_a_output_tag = strat_dto
                .parameters
                .get("split_a_output_tag")
                .map(|t| get_or_create_tag(t));
            let split_a_target_actuator = resolve_actuator("split_a_target_actuator");
            let split_b_output_tag = strat_dto
                .parameters
                .get("split_b_output_tag")
                .map(|t| get_or_create_tag(t));
            let split_b_target_actuator = resolve_actuator("split_b_target_actuator");

            StrategyKind::SplitRange {
                pv_tag,
                sp,
                master_pid,
                split_range,
                split_a_output_tag,
                split_a_target_actuator,
                split_b_output_tag,
                split_b_target_actuator,
            }
        }
        "ANTISURGE" => {
            let flow_name = strat_dto.parameters.get("flow_tag")?;
            let p_suc_name = strat_dto.parameters.get("p_suction_tag")?;
            let p_dis_name = strat_dto.parameters.get("p_discharge_tag")?;
            let flow_tag = get_or_create_tag(flow_name);
            let p_suction_tag = get_or_create_tag(p_suc_name);
            let p_discharge_tag = get_or_create_tag(p_dis_name);

            let mut controller = AntiSurgeController::new(
                parse_f64("sll_slope_a", 0.35),
                parse_f64("sll_intercept_b", 0.5),
            );
            controller.safety_margin = parse_f64("safety_margin", 0.10);
            controller.kp = parse_f64("kp", 2.0);
            controller.ti = parse_f64("ti", 1.0);
            controller.derivative_kick_gain = parse_f64("derivative_kick_gain", 1.5);
            controller.kick_threshold_rate = parse_f64("kick_threshold_rate", 2.0);

            let recycle_valve_tag = strat_dto
                .parameters
                .get("recycle_valve_tag")
                .map(|t| get_or_create_tag(t));
            let target_actuator = resolve_actuator("target_actuator");

            StrategyKind::AntiSurge {
                flow_tag,
                p_suction_tag,
                p_discharge_tag,
                controller,
                recycle_valve_tag,
                target_actuator,
            }
        }
        "VALVECHARACTERIZATION" => {
            let in_name = strat_dto.parameters.get("input_tag")?;
            let input_tag = get_or_create_tag(in_name);

            let char_type = strat_dto
                .parameters
                .get("characteristic")
                .map(|s| s.as_str())
                .unwrap_or("Linear");
            let characterizer = match char_type.to_ascii_uppercase().as_str() {
                "EQUALPERCENTAGE" | "EQ_PCT" => {
                    ValveCharacterizer::new(ValveCharacteristic::EqualPercentage {
                        rangeability: parse_f64("rangeability", 50.0),
                    })
                }
                "QUICKOPENING" | "QUICK" => {
                    ValveCharacterizer::new(ValveCharacteristic::QuickOpening)
                }
                _ => ValveCharacterizer::new(ValveCharacteristic::Linear),
            };

            let output_tag = strat_dto
                .parameters
                .get("output_tag")
                .map(|t| get_or_create_tag(t));
            let target_actuator = resolve_actuator("target_actuator");

            StrategyKind::ValveCharacterization {
                input_tag,
                characterizer,
                output_tag,
                target_actuator,
            }
        }
        _ => return None,
    };

    let mut strategy = AdvancedControlStrategy::new(
        strat_dto.id.clone(),
        strat_dto.name.clone(),
        strat_dto.description.clone(),
        kind,
    );
    strategy.enabled = strat_dto.enabled;
    Some(strategy)
}

/// Loads and instantiates an authored project into the live simulation host.
/// Performs total state isolation: all previous equipment, tags, connections,
/// alarms, controllers, and buffers are wiped before building the new plant.
#[allow(clippy::too_many_arguments)]
fn apply_load_project(
    project: &ProjectDetailDto,
    engine: &mut SimulationEngine,
    tag_store: &mut RuntimeTagStore,
    physics_system: &mut GenericPhysicsSystem,
    alarm_system: &mut Option<AlarmSystem>,
    control_system: &mut Option<ControlSystem>,
    scenario_engine: &mut Option<nebudcs_scenario::ScenarioEngine>,
    equipment_manager: &Arc<std::sync::RwLock<nebudcs_scripting::CustomEquipmentManager>>,
    last_values: &mut Vec<f64>,
    last_qualities: &mut Vec<TagQuality>,
) -> ServerResult<()> {
    // 1. Total state reset & isolation
    engine.reset();
    engine.clear_systems();
    tag_store.clear();
    physics_system.clear();

    for (util_name, util_val) in &[
        ("STEAM_PRESSURE", 40.0),
        ("COOLING_WATER", 3.0),
        ("INSTRUMENT_AIR", 7.0),
    ] {
        physics_system.set_utility(*util_name, *util_val);
        let tid = tag_store.resolve_name(util_name).unwrap_or_else(|| {
            let id = TagId::new();
            tag_store.register_tag(*util_name, id, TagValue::Float(*util_val), TagQuality::Good);
            id
        });
        tag_store.set_tag_float(tid, *util_val, TagQuality::Good);
    }

    if let Some(ctrl) = control_system.as_mut() {
        ctrl.clear();
        ctrl.set_io_system(build_default_hal_system());
    } else {
        let mut cs = ControlSystem::new();
        cs.set_io_system(build_default_hal_system());
        *control_system = Some(cs);
    }

    if let Some(alarms) = alarm_system.as_mut() {
        alarms.clear();
    } else {
        *alarm_system = Some(AlarmSystem::new());
    }

    if let Ok(mut eq_mgr) = equipment_manager.write() {
        eq_mgr.clear_instances();
    }
    if let Some(sc) = scenario_engine.as_mut() {
        sc.reset(tag_store);
    }

    // 2. Equipment registration
    let mut eq_id_map: std::collections::HashMap<String, EquipmentId> =
        std::collections::HashMap::new();

    for eq in &project.equipment {
        let eq_id = eq
            .id
            .parse::<EquipmentId>()
            .unwrap_or_else(|_| EquipmentId::new());
        eq_id_map.insert(eq.id.clone(), eq_id);
        eq_id_map.insert(eq.name.clone(), eq_id);

        let mut phys_eq = GenericEquipmentPhysics::new(eq_id);
        let def_lower = eq.definition_id.to_lowercase();
        let name_lower = eq.name.to_lowercase();

        if def_lower.contains("column")
            || def_lower.contains("fractionator")
            || name_lower.starts_with("col")
        {
            let trays = eq
                .parameters
                .get("trays")
                .map(|v| *v as usize)
                .unwrap_or(20);
            let col = DistillationColumnModel::new(trays);
            phys_eq.add_capability(col);
            let _ = phys_eq.set_variable("bottom_level", 0.5);
            let _ = phys_eq.set_variable("overhead_purity", 0.985);
            let _ = phys_eq.set_variable("temperature", 85.0);
            let _ = phys_eq.set_variable("distillate_flow", 8.0);
            let _ = phys_eq.set_variable("bottoms_flow", 12.0);
        } else if def_lower.contains("check_valve")
            || def_lower.contains("check")
            || name_lower.starts_with("cv")
            || (name_lower.starts_with("ck") && !name_lower.starts_with("ctrl"))
        {
            let cv = eq.parameters.get("cv").copied().unwrap_or(75.0);
            let chk = CheckValveModel::new(cv, 5000.0);
            phys_eq.add_capability(chk);
            let _ = phys_eq.set_variable("is_open", 1.0);
            let _ = phys_eq.set_variable("flow", 15.0);
        } else if def_lower.contains("separator")
            || def_lower.contains("vessel")
            || def_lower.contains("drum")
            || def_lower.contains("tank")
            || name_lower.starts_with('v')
            || (name_lower.starts_with('t') && !name_lower.starts_with("tt"))
        {
            let volume = eq.parameters.get("volume").copied().unwrap_or(25.0);
            let area = eq
                .parameters
                .get("cross_section_area")
                .copied()
                .unwrap_or(4.0);
            let initial_mass = eq
                .parameters
                .get("initial_mass")
                .copied()
                .unwrap_or(10_000.0);
            let density = eq
                .parameters
                .get("fluid_density")
                .copied()
                .unwrap_or(1000.0);
            let accum = AccumulatorVolumeModel::new(
                volume,
                area,
                initial_mass,
                Density::new_unchecked(density),
            );
            phys_eq.add_capability(accum);
            if let Some(init_lvl) = eq.parameters.get("level").copied() {
                let lvl = if init_lvl > 1.0 { init_lvl / 100.0 } else { init_lvl };
                let _ = phys_eq.set_variable("level", lvl);
            }
            let _ = phys_eq.set_variable("pressure", 1.013);
        } else if def_lower.contains("pump")
            || name_lower.starts_with('p')
            || name_lower.starts_with('k')
        {
            let head = eq
                .parameters
                .get("design_head")
                .or_else(|| eq.parameters.get("rated_head"))
                .copied()
                .unwrap_or(120.0);
            let flow = eq
                .parameters
                .get("design_flow")
                .or_else(|| eq.parameters.get("rated_flow"))
                .or_else(|| eq.parameters.get("rated_mass_flow"))
                .copied()
                .unwrap_or(50.0);
            let _power = eq
                .parameters
                .get("rated_power")
                .copied()
                .unwrap_or(150_000.0);
            let head_source = HeadSourceModel::new_quadratic(head * 1000.0, -0.05);
            phys_eq.add_capability(head_source);
            let initial_speed = eq
                .parameters
                .get("speed")
                .or_else(|| eq.parameters.get("speed_rpm"))
                .copied()
                .unwrap_or(100.0);
            let _ = phys_eq.set_variable("speed", initial_speed);
            let _ = phys_eq.set_variable("flow", flow);
            let _ = phys_eq.set_variable("rated_flow", flow);
            let _ = phys_eq.set_variable("head", head);
            let _ = phys_eq.set_variable("vibration", 1.2);
            let _ = phys_eq.set_variable("power", 120_000.0);
            let _ = phys_eq.set_variable("npsha", 15.0);
            let _ = phys_eq.set_variable("npshr", 3.2);

            if let Some(ctrl) = control_system.as_mut() {
                if !ctrl.has_actuator(eq_id) {
                    ctrl.register_actuator(
                        eq_id,
                        "speed",
                        ActuatorConfig::default(),
                        initial_speed,
                        None,
                    );
                }
            }
        } else if def_lower.contains("valve")
            || name_lower.starts_with("fv")
            || (name_lower.starts_with('v') && name_lower.contains("valve"))
        {
            let cv = eq
                .parameters
                .get("cv")
                .or_else(|| eq.parameters.get("cv_max"))
                .copied()
                .unwrap_or(85.0);
            let res = FlowResistanceModel::new_from_cv(cv);
            phys_eq.add_capability(res);
            let init_pos = eq
                .parameters
                .get("position")
                .or_else(|| eq.parameters.get("opening"))
                .copied()
                .unwrap_or(50.0);
            let _ = phys_eq.set_variable("opening", (init_pos / 100.0).clamp(0.0, 1.0));

            if let Some(ctrl) = control_system.as_mut() {
                if !ctrl.has_actuator(eq_id) {
                    ctrl.register_actuator(
                        eq_id,
                        "opening",
                        ActuatorConfig::default(),
                        init_pos,
                        None,
                    );
                }
            }
        } else if def_lower.contains("exchanger")
            || def_lower.contains("heater")
            || def_lower.contains("cooler")
            || name_lower.starts_with('e')
        {
            let area = eq.parameters.get("area").copied().unwrap_or(50.0);
            let u = eq.parameters.get("overall_u").copied().unwrap_or(500.0);
            let t_in = eq
                .parameters
                .get("process_inlet_t")
                .copied()
                .unwrap_or(25.0);
            let duty = eq
                .parameters
                .get("design_duty")
                .copied()
                .unwrap_or(500_000.0);
            let exch = HeatExchangeModel::new_with_ua(area * u);
            let thermal = ProcessThermalModel::new(duty, 2.0);
            phys_eq.add_capability(exch);
            phys_eq.add_capability(thermal);
            let _ = phys_eq.set_variable("temperature", t_in);
            let _ = phys_eq.set_variable("duty", duty);
            let _ = phys_eq.set_variable("flow", 20.0);
        } else if def_lower.contains("reactor") || name_lower.starts_with('r') {
            let duty = eq
                .parameters
                .get("thermal_duty")
                .copied()
                .unwrap_or(100_000.0);
            let tau = eq.parameters.get("tau").copied().unwrap_or(3.0);
            let thermal = ProcessThermalModel::new(duty, tau);
            phys_eq.add_capability(thermal);
            let _ = phys_eq.set_variable("temperature", 180.0);
            let _ = phys_eq.set_variable("pressure", 25.0);
            let _ = phys_eq.set_variable("conversion", 0.85);
            let _ = phys_eq.set_variable("flow", 20.0);
        } else if def_lower.contains("turbine") || name_lower.starts_with("st") {
            let power = eq
                .parameters
                .get("rated_power")
                .copied()
                .unwrap_or(250_000.0);
            let turb = TurbineExpansionModel::new(power);
            phys_eq.add_capability(turb);
            let init_speed = eq.parameters.get("speed").copied().unwrap_or(3000.0);
            let _ = phys_eq.set_variable("speed", init_speed);
            let _ = phys_eq.set_variable("power", power * 0.8);
            let _ = phys_eq.set_variable("steam_flow", 1.7);
            let _ = phys_eq.set_variable("governor_opening", 0.85);
        } else if def_lower.contains("compressor") || name_lower.starts_with('c') {
            let head = eq.parameters.get("rated_head").copied().unwrap_or(80.0);
            let flow = eq.parameters.get("rated_flow").copied().unwrap_or(40.0);
            let power = eq
                .parameters
                .get("rated_power")
                .copied()
                .unwrap_or(200_000.0);
            let head_source = HeadSourceModel::new_quadratic(head * 1000.0, -0.05);
            phys_eq.add_capability(head_source);
            let _ = phys_eq.set_variable("speed", 3000.0);
            let _ = phys_eq.set_variable("flow", flow);
            let _ = phys_eq.set_variable("head", head);
            let _ = phys_eq.set_variable("power", power * 0.75);
            let _ = phys_eq.set_variable("vibration", 1.2);
        } else {
            let res = FlowResistanceModel::new_from_cv(50.0);
            phys_eq.add_capability(res);
        }

        // Register default equipment state variables as DCS tags for operator access
        for (var_name, fallback_val) in &[
            ("level", 50.0),
            ("pressure", 1.013),
            ("temperature", 25.0),
            ("flow", 0.0),
            ("speed", 100.0),
            ("opening", 50.0),
            ("duty", 0.0),
            ("conversion", 0.0),
            ("vibration", 1.2),
            ("power", 0.0),
            ("npsha", 15.0),
            ("purity", 0.98),
            ("bottom_level", 50.0),
            ("overhead_purity", 0.985),
            ("distillate_flow", 8.0),
            ("bottoms_flow", 12.0),
            ("steam_flow", 10.0),
        ] {
            if let Some(val) = phys_eq.get_variable(var_name) {
                let tag_name = format!("{}.{}", eq.name.to_uppercase(), var_name.to_uppercase());
                let tid = tag_store.resolve_name(&tag_name).unwrap_or_else(|| {
                    let id = TagId::new();
                    tag_store.register_tag(&tag_name, id, TagValue::Float(val), TagQuality::Good);
                    id
                });
                phys_eq.bind_tag(*var_name, tid);
            } else if *var_name == "opening" {
                let tag_name = format!("{}.POS", eq.name.to_uppercase());
                let tid = tag_store.resolve_name(&tag_name).unwrap_or_else(|| {
                    let id = TagId::new();
                    tag_store.register_tag(
                        &tag_name,
                        id,
                        TagValue::Float(*fallback_val),
                        TagQuality::Good,
                    );
                    id
                });
                phys_eq.bind_tag("opening", tid);
            }
        }

        physics_system.register_equipment(phys_eq);
    }

    // 3. Instruments registration
    for inst in &project.instruments {
        let tag_id = inst.tag.parse::<TagId>().unwrap_or_else(|_| TagId::new());
        tag_store.register_tag(
            &inst.tag,
            tag_id,
            TagValue::Float(inst.range_min),
            TagQuality::Good,
        );

        let kind = inst.measurement_kind.to_lowercase();
        let tag_upper = inst.tag.to_uppercase();
        let var_name = if kind.contains("level") || tag_upper.starts_with("LT") {
            "level"
        } else if kind.contains("temp") || tag_upper.starts_with("TT") {
            "temperature"
        } else if kind.contains("press") || tag_upper.starts_with("PT") {
            "pressure"
        } else if kind.contains("vib") || tag_upper.starts_with("VT") {
            "vibration"
        } else if kind.contains("speed") || tag_upper.starts_with("ST") {
            "speed"
        } else if kind.contains("power") || kind.contains("watt") || tag_upper.starts_with("WT") {
            "power"
        } else if kind.contains("pos") || kind.contains("opening") || tag_upper.starts_with("ZT") {
            "opening"
        } else if kind.contains("pur") || kind.contains("comp") || tag_upper.starts_with("AT") {
            "purity"
        } else {
            "flow"
        };

        if let Some(target_str) = &inst.equipment_id {
            if let Some(&target_eq_id) = eq_id_map.get(target_str) {
                let has_primary_binding = if let Some(eq) = physics_system.equipment(target_eq_id) {
                    eq.tag_bindings().contains_key(var_name)
                } else {
                    false
                };

                let effective_var = if !has_primary_binding {
                    if let Some(eq) = physics_system.equipment_mut(target_eq_id) {
                        eq.bind_tag(var_name, tag_id);
                    }
                    var_name.to_string()
                } else {
                    let custom_var = inst.tag.to_lowercase();
                    if let Some(eq) = physics_system.equipment_mut(target_eq_id) {
                        eq.bind_tag(&custom_var, tag_id);
                    }
                    custom_var
                };

                if let Some(ctrl) = control_system.as_mut() {
                    let inst_cfg = InstrumentConfig {
                        lrv: inst.range_min,
                        urv: if (inst.range_max - inst.range_min).abs() < 1e-6 {
                            if var_name == "vibration" {
                                10.0
                            } else if var_name == "speed" {
                                5000.0
                            } else if var_name == "power" {
                                500_000.0
                            } else {
                                inst.range_min + 100.0
                            }
                        } else {
                            inst.range_max
                        },
                        ..Default::default()
                    };
                    let init_val = if inst.tag == "TI-101-CIT" {
                        287.0
                    } else if inst.tag == "TI-101-SK1" {
                        395.0
                    } else if inst.tag == "TI-101-SK2" {
                        398.0
                    } else if inst.tag == "TI-101-RIT" {
                        308.5
                    } else if inst.tag == "TI-101-ROT" {
                        332.0
                    } else if inst.tag == "PT-104" {
                        4.2
                    } else if inst.tag == "FT-105" {
                        1250.0
                    } else if inst.tag == "PT-105" {
                        7.5
                    } else if inst.tag == "PDT-101" {
                        -4.8
                    } else if inst.tag == "TI-104" {
                        180.0
                    } else if inst.tag == "AI-101" {
                        3.2
                    } else if inst.tag == "AI-102" {
                        15.0
                    } else if inst.tag == "FT-Air" {
                        14250.0
                    } else if inst.tag == "PT-Air" {
                        24.5
                    } else if inst.tag == "FZ-101" {
                        45.0
                    } else if inst.tag == "FFC-101" {
                        1.15
                    } else if inst.tag == "PT-107" {
                        6.0
                    } else if inst.tag == "FT-107" {
                        120.0
                    } else {
                        physics_system
                            .equipment(target_eq_id)
                            .and_then(|eq| eq.get_variable(&effective_var))
                            .unwrap_or(inst.range_min)
                    };
                    if has_primary_binding {
                        if let Some(eq) = physics_system.equipment_mut(target_eq_id) {
                            let _ = eq.set_variable(&effective_var, init_val);
                        }
                    }
                    tag_store.set_tag_float(tag_id, init_val, TagQuality::Good);
                    ctrl.register_instrument(target_eq_id, effective_var, tag_id, inst_cfg, init_val);
                }
            }
        }
    }

    // 4. Streams / Couplings
    for stream in &project.streams {
        let src_id = eq_id_map
            .get(&stream.source_equipment_id)
            .copied()
            .unwrap_or_else(|| {
                stream
                    .source_equipment_id
                    .parse::<EquipmentId>()
                    .unwrap_or_else(|_| EquipmentId::new())
            });
        let snk_id = eq_id_map
            .get(&stream.sink_equipment_id)
            .copied()
            .unwrap_or_else(|| {
                stream
                    .sink_equipment_id
                    .parse::<EquipmentId>()
                    .unwrap_or_else(|_| EquipmentId::new())
            });

        let st_lower = stream.stream_type.to_lowercase();
        let name_lower = stream.name.to_lowercase();

        let coupling_kind = if st_lower.contains("mechanical") || name_lower.contains("shaft") {
            CouplingKind::MechanicalShaft { speed_ratio: 1.0 }
        } else if st_lower.contains("thermal") || name_lower.contains("heat") {
            CouplingKind::ThermalDuty {
                heat_capacity_cp: 2000.0,
            }
        } else if st_lower.contains("utility") || name_lower.contains("steam") {
            CouplingKind::UtilitySupply {
                utility_kind: "STEAM_PRESSURE".to_string(),
                nominal_value: 40.0,
            }
        } else {
            CouplingKind::MaterialFlow {
                nominal_density: 1000.0,
            }
        };

        let flow_tag = format!("{}.FLOW", stream.name.to_uppercase());
        let flow_tid = tag_store.resolve_name(&flow_tag).unwrap_or_else(|| {
            let id = TagId::new();
            tag_store.register_tag(&flow_tag, id, TagValue::Float(0.0), TagQuality::Good);
            id
        });
        let temp_tag = format!("{}.TEMP", stream.name.to_uppercase());
        if tag_store.resolve_name(&temp_tag).is_none() {
            tag_store.register_tag(&temp_tag, TagId::new(), TagValue::Float(25.0), TagQuality::Good);
        }
        let press_tag = format!("{}.PRESS", stream.name.to_uppercase());
        if tag_store.resolve_name(&press_tag).is_none() {
            tag_store.register_tag(&press_tag, TagId::new(), TagValue::Float(1.013), TagQuality::Good);
        }

        physics_system.add_coupling(
            ProcessCoupling::new(src_id, snk_id, coupling_kind.clone())
                .with_stream(stream.name.clone(), Some(flow_tid)),
        );

        if matches!(coupling_kind, CouplingKind::MaterialFlow { .. })
            && (stream.diameter.is_some() || stream.length.is_some())
        {
            let src_str = src_id.to_string();
            let snk_str = snk_id.to_string();

            if physics_system.network().get_node(&src_str).is_none() {
                let is_boundary = project.equipment.iter().any(|e| {
                    (e.id == stream.source_equipment_id || e.name == stream.source_equipment_id)
                        && (e.definition_id.to_lowercase().contains("tank")
                            || e.definition_id.to_lowercase().contains("vessel")
                            || e.name.to_lowercase().contains("tk")
                            || e.name.to_lowercase().contains("boundary"))
                });
                let src_node = if is_boundary {
                    let init_p = physics_system
                        .equipment(src_id)
                        .and_then(|eq| {
                            eq.get_variable("bottom_pressure")
                                .or_else(|| eq.get_variable("pressure"))
                        })
                        .unwrap_or(101_325.0);
                    nebudcs_physics::network::HydraulicNode::new_boundary(
                        src_str.clone(),
                        stream.source_equipment_id.clone(),
                        nebudcs_units::Pressure::new_unchecked(init_p),
                        0.0,
                    )
                } else {
                    nebudcs_physics::network::HydraulicNode::new_junction(
                        src_str.clone(),
                        stream.source_equipment_id.clone(),
                        0.0,
                    )
                };
                physics_system.network_mut().add_node(src_node);
            }

            if physics_system.network().get_node(&snk_str).is_none() {
                let is_boundary = project.equipment.iter().any(|e| {
                    (e.id == stream.sink_equipment_id || e.name == stream.sink_equipment_id)
                        && (e.definition_id.to_lowercase().contains("tank")
                            || e.definition_id.to_lowercase().contains("vessel")
                            || e.name.to_lowercase().contains("tk")
                            || e.name.to_lowercase().contains("boundary"))
                });
                let snk_node = if is_boundary {
                    let init_p = physics_system
                        .equipment(snk_id)
                        .and_then(|eq| {
                            eq.get_variable("bottom_pressure")
                                .or_else(|| eq.get_variable("pressure"))
                        })
                        .unwrap_or(101_325.0);
                    nebudcs_physics::network::HydraulicNode::new_boundary(
                        snk_str.clone(),
                        stream.sink_equipment_id.clone(),
                        nebudcs_units::Pressure::new_unchecked(init_p),
                        0.0,
                    )
                } else {
                    nebudcs_physics::network::HydraulicNode::new_junction(
                        snk_str.clone(),
                        stream.sink_equipment_id.clone(),
                        0.0,
                    )
                };
                physics_system.network_mut().add_node(snk_node);
            }

            let edge = nebudcs_physics::network::HydraulicEdge::new_pipe(
                stream.id.clone(),
                stream.name.clone(),
                src_str,
                snk_str,
                stream.length.unwrap_or(10.0),
                stream.diameter.unwrap_or(0.1),
                stream.roughness.unwrap_or(0.000045),
                stream.loss_k.unwrap_or(0.0),
            );
            physics_system.network_mut().add_edge(edge);
        }
    }

    // 5. Control Loops
    for cl in &project.control_loops {
        let pv_tag_id = tag_store.resolve_name(&cl.pv_tag).unwrap_or_else(|| {
            let tid = cl.pv_tag.parse::<TagId>().unwrap_or_else(|_| TagId::new());
            tag_store.register_tag(&cl.pv_tag, tid, TagValue::Float(cl.sp), TagQuality::Good);
            tid
        });

        let out_tag_id = tag_store.resolve_name(&cl.out_tag).unwrap_or_else(|| {
            let tid = cl.out_tag.parse::<TagId>().unwrap_or_else(|_| TagId::new());
            tag_store.register_tag(&cl.out_tag, tid, TagValue::Float(50.0), TagQuality::Good);
            tid
        });

        if tag_store.resolve_name(&cl.name).is_none() {
            tag_store.register_tag(
                &cl.name,
                out_tag_id,
                TagValue::Float(cl.sp),
                TagQuality::Good,
            );
        }

        let target_eq_id = project
            .equipment
            .iter()
            .find(|e| cl.out_tag.starts_with(&e.name) || cl.out_tag.starts_with(&e.id))
            .and_then(|e| eq_id_map.get(&e.id).copied())
            .unwrap_or_else(|| {
                cl.out_tag
                    .parse::<EquipmentId>()
                    .unwrap_or_else(|_| EquipmentId::new())
            });

        if let Some(ctrl) = control_system.as_mut() {
            if !ctrl.has_actuator(target_eq_id) {
                ctrl.register_actuator(
                    target_eq_id,
                    "opening",
                    ActuatorConfig::default(),
                    50.0,
                    None,
                );
            }
            let pid_cfg = PidConfig {
                kp: cl.kp,
                ti: cl.ti,
                td: cl.td,
                out_min: cl.output_min,
                out_max: cl.output_max,
                direction: if cl.reverse_acting {
                    PidDirection::Reverse
                } else {
                    PidDirection::Direct
                },
                ..Default::default()
            };
            ctrl.register_pid(pv_tag_id, out_tag_id, target_eq_id, pid_cfg);

            if let Some(p) = ctrl.pid_mut(out_tag_id) {
                p.set_sp(cl.sp);
                p.set_mode(match cl.mode.to_uppercase().as_str() {
                    "MANUAL" | "MAN" => PidMode::Manual,
                    "CASCADE" | "CAS" => PidMode::Cascade,
                    _ => PidMode::Auto,
                });
            }
        }
    }

    // 6. Alarms
    for al in &project.alarms {
        let pv_tag_id = tag_store.resolve_name(&al.pv_tag).unwrap_or_else(|| {
            let tid = al.pv_tag.parse::<TagId>().unwrap_or_else(|_| TagId::new());
            tag_store.register_tag(&al.pv_tag, tid, TagValue::Float(0.0), TagQuality::Good);
            tid
        });

        let alarm_type = match al.alarm_type.to_uppercase().as_str() {
            "HIHI" => AlarmType::HiHi,
            "HI" => AlarmType::Hi,
            "LOLO" => AlarmType::LoLo,
            "LO" => AlarmType::Lo,
            _ => AlarmType::Hi,
        };

        let severity = match al.severity.to_uppercase().as_str() {
            "CRITICAL" => AlarmSeverity::Critical,
            "HIGH" | "WARNING" => AlarmSeverity::Warning,
            _ => AlarmSeverity::Advisory,
        };

        let cfg = AlarmBlockConfig {
            name: al.name.clone(),
            tag_id: pv_tag_id,
            alarm_type,
            severity,
            limit: al.trip_limit,
            deadband: al.deadband,
            default_shelve_duration: 60.0,
        };

        if let Some(alarms) = alarm_system.as_mut() {
            alarms.add_alarm(cfg);
        }
    }

    // 7. Interlocks
    for il in &project.interlocks {
        let src_tag_id = tag_store.resolve_name(&il.source_tag).unwrap_or_else(|| {
            let tid = il
                .source_tag
                .parse::<TagId>()
                .unwrap_or_else(|_| TagId::new());
            tag_store.register_tag(&il.source_tag, tid, TagValue::Float(0.0), TagQuality::Good);
            tid
        });

        let target_eq_id = il
            .target_equipment
            .as_ref()
            .and_then(|id_str| {
                eq_id_map
                    .get(id_str)
                    .copied()
                    .or_else(|| id_str.parse::<EquipmentId>().ok())
            })
            .unwrap_or_else(EquipmentId::new);

        let direction = match il.condition.to_lowercase().as_str() {
            "lt" | "lte" | "<" | "<=" => TripDirection::Low,
            _ => TripDirection::High,
        };

        let safe_pos = match il.target_action.to_uppercase().as_str() {
            "OPEN" => 100.0,
            _ => 0.0,
        };

        if let Some(ctrl) = control_system.as_mut() {
            if !ctrl.has_actuator(target_eq_id) {
                ctrl.register_actuator(
                    target_eq_id,
                    "opening",
                    ActuatorConfig::default(),
                    50.0,
                    None,
                );
            }
            let reset_deadband = if il.threshold.abs() <= 1.0 {
                0.02
            } else if il.threshold.abs() <= 100.0 {
                0.5
            } else {
                10.0
            };
            ctrl.register_protection_with_direction(
                &il.name,
                src_tag_id,
                target_eq_id,
                CommandPriority::Interlock,
                safe_pos,
                il.threshold,
                reset_deadband,
                direction,
            );
        }

        let trip_tag_name = format!("{}.TRIPPED", il.name);
        let trip_tag_id = tag_store.resolve_name(&trip_tag_name).unwrap_or_else(|| {
            let tid = TagId::new();
            tag_store.register_tag(&trip_tag_name, tid, TagValue::Bool(false), TagQuality::Good);
            tid
        });
        let _ = trip_tag_id;
    }

    // 7.1 Canonical Project Tags Registration
    for tag_dto in &project.tags {
        if tag_store.resolve_name(&tag_dto.name).is_none() {
            let tid = tag_dto.id.parse::<TagId>().unwrap_or_else(|_| TagId::new());
            let val = match tag_dto.data_type.to_ascii_uppercase().as_str() {
                "BOOL" => TagValue::Bool(false),
                "INT" | "DINT" | "INT32" => TagValue::Integer(0),
                _ => TagValue::Float(0.0),
            };
            tag_store.register_tag(&tag_dto.name, tid, val, TagQuality::Good);
        }
    }

    // 7.2 I/O Hardware Abstraction Layer (HAL) & Mappings
    if let Some(ctrl) = control_system.as_mut() {
        let mut hal_system = if let Some(sys_dto) = project.io_systems.first() {
            build_hal_system_from_dto(sys_dto).unwrap_or_else(|_| build_default_hal_system())
        } else {
            build_default_hal_system()
        };

        for map in &project.io_mappings {
            if let Ok(addr) = map
                .channel_address
                .parse::<nebudcs_control::hal::IoChannelAddress>()
            {
                let ctrl_id = map
                    .controller_id
                    .parse::<ControllerId>()
                    .unwrap_or_else(|_| ControllerId::new());
                let target_tag_id = map
                    .target_tag
                    .as_ref()
                    .and_then(|t| tag_store.resolve_name(t));
                let target_act_id = map.target_actuator.as_ref().and_then(|a| {
                    eq_id_map
                        .get(a)
                        .copied()
                        .or_else(|| a.parse::<EquipmentId>().ok())
                });
                let priority = match map.priority.unwrap_or(5) {
                    1 => CommandPriority::Esd,
                    2 => CommandPriority::Interlock,
                    3 => CommandPriority::Permissive,
                    4 => CommandPriority::Manual,
                    _ => CommandPriority::Auto,
                };

                let mapping = if map.direction.eq_ignore_ascii_case("output") {
                    nebudcs_control::hal::ControllerIoMapping::new_output(
                        ctrl_id,
                        &map.signal_key,
                        addr,
                        target_act_id,
                        priority,
                    )
                } else {
                    nebudcs_control::hal::ControllerIoMapping::new_input(
                        ctrl_id,
                        &map.signal_key,
                        addr,
                        target_tag_id,
                    )
                };
                let _ = hal_system.add_mapping(mapping);
            }
        }
        ctrl.set_io_system(hal_system);

        // 7.3 Virtual Controllers, Tasks, and IEC Routines
        for ctrl_dto in &project.controllers {
            let ctrl_id = ctrl_dto
                .id
                .parse::<ControllerId>()
                .unwrap_or_else(|_| ControllerId::new());
            let ctrl_cfg = nebudcs_control::runtime::ControllerConfig {
                name: ctrl_dto.name.clone(),
                description: ctrl_dto.description.clone(),
                watchdog_timeout_seconds: ctrl_dto.watchdog_timeout_seconds,
                enabled: ctrl_dto.enabled,
            };
            let mut vctrl =
                nebudcs_control::runtime::VirtualController::new(ctrl_id, &ctrl_dto.name, ctrl_cfg);

            for task_dto in &ctrl_dto.tasks {
                let task_id = task_dto
                    .id
                    .parse::<TaskId>()
                    .unwrap_or_else(|_| TaskId::new());
                let mut task = match task_dto.task_type.to_lowercase().as_str() {
                    "event" => nebudcs_control::runtime::ControllerTask::new_event(
                        task_id,
                        &task_dto.name,
                        "EVENT_TRIGGER",
                        task_dto.priority,
                    ),
                    "continuous" => nebudcs_control::runtime::ControllerTask::new_continuous(
                        task_id,
                        &task_dto.name,
                        task_dto.priority,
                    ),
                    _ => nebudcs_control::runtime::ControllerTask::new_periodic(
                        task_id,
                        &task_dto.name,
                        task_dto.period_seconds,
                        task_dto.priority,
                    ),
                };
                task = task.with_watchdog_limit(task_dto.watchdog_seconds);
                task.set_enabled(task_dto.enabled);

                for prog_dto in &task_dto.programs {
                    let mut iec_prog = nebudcs_control::iec61131::IecProgram::new(&prog_dto.name);
                    for routine_dto in &prog_dto.routines {
                        if !routine_dto.enabled {
                            continue;
                        }
                        let routine = match routine_dto.language.to_ascii_uppercase().as_str() {
                            "ST" => nebudcs_control::iec61131::IecRoutine::from_st(
                                &routine_dto.name,
                                &routine_dto.source_text,
                            )
                            .map_err(|e| {
                                ServerError::InvalidRequest(format!(
                                    "Failed to compile ST routine '{}': {e}",
                                    routine_dto.name
                                ))
                            })?,
                            "LD" => {
                                let diag: nebudcs_control::iec61131::LdDiagram =
                                    serde_json::from_str(&routine_dto.source_text).map_err(
                                        |e| {
                                            ServerError::InvalidRequest(format!(
                                                "Failed to deserialize LD diagram '{}': {e}",
                                                routine_dto.name
                                            ))
                                        },
                                    )?;
                                nebudcs_control::iec61131::IecRoutine::from_ld(
                                    &routine_dto.name,
                                    &diag,
                                )
                                .map_err(|e| {
                                    ServerError::InvalidRequest(format!(
                                        "Failed to compile LD routine '{}': {e}",
                                        routine_dto.name
                                    ))
                                })?
                            }
                            "FBD" => {
                                let graph: nebudcs_control::iec61131::FbdGraph =
                                    serde_json::from_str(&routine_dto.source_text).map_err(
                                        |e| {
                                            ServerError::InvalidRequest(format!(
                                                "Failed to deserialize FBD graph '{}': {e}",
                                                routine_dto.name
                                            ))
                                        },
                                    )?;
                                nebudcs_control::iec61131::IecRoutine::from_fbd(
                                    &routine_dto.name,
                                    &graph,
                                )
                                .map_err(|e| {
                                    ServerError::InvalidRequest(format!(
                                        "Failed to compile FBD routine '{}': {e}",
                                        routine_dto.name
                                    ))
                                })?
                            }
                            "SFC" => {
                                let chart: nebudcs_control::iec61131::SfcChart =
                                    serde_json::from_str(&routine_dto.source_text).map_err(
                                        |e| {
                                            ServerError::InvalidRequest(format!(
                                                "Failed to deserialize SFC chart '{}': {e}",
                                                routine_dto.name
                                            ))
                                        },
                                    )?;
                                nebudcs_control::iec61131::IecRoutine::from_sfc(
                                    &routine_dto.name,
                                    &chart,
                                )
                                .map_err(|e| {
                                    ServerError::InvalidRequest(format!(
                                        "Failed to compile SFC routine '{}': {e}",
                                        routine_dto.name
                                    ))
                                })?
                            }
                            other => {
                                return Err(ServerError::InvalidRequest(format!(
                                    "Unsupported routine language '{other}'"
                                )))
                            }
                        };
                        iec_prog.add_routine(routine).map_err(|e| {
                            ServerError::InvalidRequest(format!("Failed to add routine: {e}"))
                        })?;
                    }
                    task.add_program(Box::new(iec_prog));
                }
                vctrl.add_task(task);
            }

            if let Some(cpu) = &ctrl_dto.cpu_profile {
                vctrl.set_cpu_profile(nebudcs_control::runtime::CpuProfile {
                    execution_budget_us: cpu.execution_budget_us,
                    max_scan_frequency_hz: cpu.max_scan_frequency_hz,
                    utilization_threshold_pct: cpu.utilization_threshold_pct,
                    architecture: cpu.architecture.clone(),
                });
            }

            if let Some(mem) = &ctrl_dto.memory_profile {
                vctrl.set_memory_profile(nebudcs_control::runtime::MemoryProfile {
                    program_memory_bytes: mem.program_memory_bytes,
                    data_memory_bytes: mem.data_memory_bytes,
                    retentive_memory_bytes: mem.retentive_memory_bytes,
                    program_memory_used_bytes: mem.program_memory_used_bytes,
                    data_memory_used_bytes: mem.data_memory_used_bytes,
                    retentive_memory_used_bytes: mem.retentive_memory_used_bytes,
                });
            }

            if let Some(fw) = &ctrl_dto.firmware_profile {
                vctrl.set_firmware_profile(nebudcs_control::runtime::FirmwareProfile {
                    version: fw.version.clone(),
                    build: fw.build.clone(),
                    iec_standard: fw.iec_standard.clone(),
                });
            }

            if let Some(red) = &ctrl_dto.redundancy {
                let role = match red.role.to_lowercase().as_str() {
                    "primary" => nebudcs_control::runtime::RedundancyRole::Primary,
                    "standby" => nebudcs_control::runtime::RedundancyRole::Standby,
                    _ => nebudcs_control::runtime::RedundancyRole::Standalone,
                };
                vctrl.set_redundancy(nebudcs_control::runtime::RedundancyState {
                    role,
                    status: nebudcs_control::runtime::RedundancyStatus::Synchronizing,
                    partner_id: red.partner_controller_id.clone(),
                    sync_frequency_hz: red.sync_frequency_hz,
                    switchover_timeout_seconds: red.switchover_timeout_seconds,
                    last_sync_sim_time: 0.0,
                    sync_cycle_count: 0,
                });
            }

            for cm_dto in &ctrl_dto.control_modules {
                let pid_cfg = nebudcs_control::pid::PidConfig {
                    kp: cm_dto.kp,
                    ti: cm_dto.ti,
                    td: cm_dto.td,
                    derivative_filter_n: 10.0,
                    out_min: cm_dto.output_min,
                    out_max: cm_dto.output_max,
                    direction: nebudcs_control::pid::PidDirection::Direct,
                    sp_tracking_in_manual: true,
                    scan_divisor: 1,
                };
                let mut cm = nebudcs_control::runtime::DcsControlModule::new(
                    &cm_dto.id,
                    &cm_dto.name,
                    &cm_dto.pv_signal,
                    &cm_dto.out_signal,
                    cm_dto.sp,
                    pid_cfg,
                )
                .with_description(&cm_dto.description)
                .with_execution_rate(cm_dto.execution_rate_seconds);

                for p in &cm_dto.permissives {
                    cm.add_permissive(p);
                }
                for i in &cm_dto.interlocks {
                    cm.add_interlock(i);
                }
                cm.set_safe_position(cm_dto.safe_position);

                let target_mode = match cm_dto.mode.to_uppercase().as_str() {
                    "MAN" | "MANUAL" => nebudcs_control::runtime::ControlModuleMode::Manual,
                    "CAS" | "CASCADE" => nebudcs_control::runtime::ControlModuleMode::Cascade,
                    "RCAS" | "REMOTECASCADE" => {
                        nebudcs_control::runtime::ControlModuleMode::RemoteCascade
                    }
                    _ => nebudcs_control::runtime::ControlModuleMode::Auto,
                };
                let _ = cm.set_mode(target_mode);

                vctrl.add_control_module(cm);
            }

            if ctrl_dto.enabled {
                let _ = vctrl.start();
            }
            ctrl.register_controller(vctrl);
        }

        // 7.4 Advanced Control Strategies
        for strat_dto in &project.advanced_control {
            if let Some(strat) = build_advanced_control_strategy(strat_dto, tag_store, &eq_id_map) {
                ctrl.register_strategy(strat);
            }
        }

        // 7.5 Safety Instrumented System (SIS)
        if let Some(sis_cfg) = &project.sis {
            let mut sis = nebudcs_control::sis::SafetyInstrumentedSystem::new();
            sis.set_enabled(sis_cfg.enabled);

            for sif_def in &sis_cfg.sifs {
                let dir = match sif_def.direction.to_uppercase().as_str() {
                    "LOW" => nebudcs_control::sis::voting::TripDirection::Low,
                    _ => nebudcs_control::sis::voting::TripDirection::High,
                };
                let arch = match sif_def.architecture.to_lowercase().as_str() {
                    "1oo1" => nebudcs_control::sis::voting::VotingArchitecture::OneOutOfOne,
                    "1oo2" => nebudcs_control::sis::voting::VotingArchitecture::OneOutOfTwo,
                    "2oo2" => nebudcs_control::sis::voting::VotingArchitecture::TwoOutOfTwo,
                    "2oo4" => nebudcs_control::sis::voting::VotingArchitecture::TwoOutOfFour,
                    _ => nebudcs_control::sis::voting::VotingArchitecture::TwoOutOfThree,
                };
                let deg = match sif_def.degradation_policy.to_lowercase().as_str() {
                    "degradeto2oo2" | "2oo2" => {
                        nebudcs_control::sis::voting::DegradationPolicy::DegradeTo2oo2
                    }
                    "degradeto1oo1" | "1oo1" => {
                        nebudcs_control::sis::voting::DegradationPolicy::DegradeTo1oo1
                    }
                    "triponfault" | "trip" => {
                        nebudcs_control::sis::voting::DegradationPolicy::TripOnFault
                    }
                    "freezestate" | "freeze" => {
                        nebudcs_control::sis::voting::DegradationPolicy::FreezeState
                    }
                    _ => nebudcs_control::sis::voting::DegradationPolicy::DegradeTo1oo2,
                };

                let final_elements = sif_def
                    .final_elements
                    .iter()
                    .map(|fe| {
                        let safe_st = match fe.safe_state.to_uppercase().as_str() {
                            "VALVEOPEN" | "OPEN" => {
                                nebudcs_control::sis::sif::SifSafeState::ValveOpen
                            }
                            "DRIVERTRIP" | "TRIP" => {
                                nebudcs_control::sis::sif::SifSafeState::DriverTrip
                            }
                            _ => nebudcs_control::sis::sif::SifSafeState::ValveClosed,
                        };
                        let act_type = match fe.action_type.to_lowercase().as_str() {
                            "feedisolation" => {
                                nebudcs_control::sis::sif::SifActionType::FeedIsolation
                            }
                            "emergencydepressuring" => {
                                nebudcs_control::sis::sif::SifActionType::EmergencyDepressuring
                            }
                            "drivertrip" => nebudcs_control::sis::sif::SifActionType::DriverTrip,
                            "heatingcutoff" => {
                                nebudcs_control::sis::sif::SifActionType::HeatingCutoff
                            }
                            "emergencycooling" => {
                                nebudcs_control::sis::sif::SifActionType::EmergencyCooling
                            }
                            _ => nebudcs_control::sis::sif::SifActionType::GeneralSafetyTrip,
                        };
                        nebudcs_control::sis::sif::SifFinalElement {
                            target_equipment: fe.target_equipment.clone(),
                            actuator_var: fe.actuator_var.clone(),
                            safe_state: safe_st,
                            de_energize_to_trip: fe.de_energize_to_trip,
                            action_type: act_type,
                            feedback_tag: fe.feedback_tag.clone(),
                        }
                    })
                    .collect();

                let config = nebudcs_control::sis::sif::SifConfig {
                    id: sif_def.id.clone(),
                    name: sif_def.name.clone(),
                    description: sif_def.description.clone(),
                    hazard_id: sif_def.hazard_id.clone(),
                    hazard_scenario: sif_def.hazard_scenario.clone(),
                    target_sil: sif_def.target_sil.clone(),
                    direction: dir,
                    threshold: sif_def.threshold,
                    reset_deadband: sif_def.reset_deadband,
                    trip_delay_seconds: sif_def.trip_delay_seconds,
                    architecture: arch,
                    degradation_policy: deg,
                    sensor_tags: sif_def.sensor_tags.clone(),
                    final_elements,
                    latched: sif_def.latched,
                    reset_permissives: sif_def.reset_permissives.clone(),
                    pfd_avg_estimate: sif_def.pfd_avg_estimate,
                };

                let sif = nebudcs_control::sis::sif::SafetyInstrumentedFunction::new(config);
                sis.register_sif(sif);
            }

            *ctrl.sis_mut() = sis;
        }
    }

    // 7.5 Process Boundary Conditions & Initial Conditions
    for bc in &project.boundary_conditions {
        let density = bc.density.unwrap_or(1000.0);
        let mw = bc.molecular_weight.unwrap_or(0.018);
        let (mass_flow, vol_flow) = nebudcs_units::conversion::parse_flow(
            bc.flow_value,
            &bc.flow_unit,
            density,
            Some(mw),
        )
        .unwrap_or_else(|_| {
            (
                nebudcs_units::MassFlow::new_unchecked(bc.flow_value),
                nebudcs_units::VolumetricFlow::new_unchecked(bc.flow_value / density),
            )
        });

        let pressure = nebudcs_units::conversion::parse_pressure(
            bc.pressure_value,
            &bc.pressure_unit,
        )
        .unwrap_or_else(|_| nebudcs_units::Pressure::new_unchecked(bc.pressure_value));

        let temperature = nebudcs_units::conversion::parse_temperature(
            bc.temperature_value,
            &bc.temperature_unit,
        )
        .unwrap_or_else(|_| {
            nebudcs_units::Temperature::new_unchecked(bc.temperature_value + 273.15)
        });

        let target_eq_id = bc.target_equipment_id.as_ref().and_then(|id_str| {
            eq_id_map
                .get(id_str)
                .copied()
                .or_else(|| id_str.parse::<EquipmentId>().ok())
        });

        let mut comp = std::collections::BTreeMap::new();
        for (k, v) in &bc.composition {
            comp.insert(k.clone(), *v);
        }

        let pbc = nebudcs_physics::system::ProcessBoundaryCondition {
            id: bc.id.clone(),
            name: bc.name.clone(),
            condition_type: bc.condition_type.clone(),
            target_equipment: target_eq_id,
            target_port: bc.target_port.clone(),
            target_stream: bc.target_stream_id.clone(),
            mass_flow_kg_s: mass_flow.value(),
            volumetric_flow_m3_s: vol_flow.value(),
            pressure_pa: pressure.value(),
            temperature_c: temperature.to_celsius(),
            phase: bc.phase.clone(),
            density,
            molecular_weight: mw,
            composition: comp,
            enabled: bc.enabled,
        };

        physics_system.add_boundary_condition(pbc);

        // Register boundary condition tags so external inputs are observable via WebSocket
        let bc_tag_base = bc.name.to_uppercase().replace(' ', "_");
        let bc_flow_tag = format!("{}.FLOW", bc_tag_base);
        if tag_store.resolve_name(&bc_flow_tag).is_none() {
            tag_store.register_tag(
                &bc_flow_tag,
                TagId::new(),
                TagValue::Float(bc.flow_value),
                TagQuality::Good,
            );
        }
        let bc_temp_tag = format!("{}.TEMP", bc_tag_base);
        if tag_store.resolve_name(&bc_temp_tag).is_none() {
            tag_store.register_tag(
                &bc_temp_tag,
                TagId::new(),
                TagValue::Float(bc.temperature_value),
                TagQuality::Good,
            );
        }
        let bc_press_tag = format!("{}.PRESS", bc_tag_base);
        if tag_store.resolve_name(&bc_press_tag).is_none() {
            tag_store.register_tag(
                &bc_press_tag,
                TagId::new(),
                TagValue::Float(bc.pressure_value),
                TagQuality::Good,
            );
        }
    }

    for ic in &project.initial_conditions {
        let target_eq_id = eq_id_map
            .get(&ic.entity_id)
            .copied()
            .or_else(|| ic.entity_id.parse::<EquipmentId>().ok());
        let norm_value = if ic.variable == "level" && ic.value > 1.0 {
            ic.value / 100.0
        } else {
            ic.value
        };
        if let Some(eq_id) = target_eq_id {
            if let Some(eq) = physics_system.equipment_mut(eq_id) {
                let _ = eq.set_variable(&ic.variable, norm_value);
            }
        }
        if let Some(tid) = tag_store.resolve_name(&ic.entity_id) {
            tag_store.set_tag_float(tid, norm_value, TagQuality::Good);
        }
        let tag_var_name =
            format!("{}.{}", ic.entity_id.to_uppercase(), ic.variable.to_uppercase());
        if let Some(tid) = tag_store.resolve_name(&tag_var_name) {
            tag_store.set_tag_float(tid, norm_value, TagQuality::Good);
        }
    }

    // 8. Re-seed cache vectors
    *last_values = (0..tag_store.len())
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

    *last_qualities = (0..tag_store.len())
        .map(|i| {
            tag_store
                .get_quality_by_index(i)
                .unwrap_or(TagQuality::Good)
        })
        .collect();

    Ok(())
}

/// Applies a validated runtime command to the simulation systems.
#[allow(clippy::too_many_arguments)]
fn apply_command(
    command: &RuntimeCommand,
    engine: &mut SimulationEngine,
    tag_store: &mut RuntimeTagStore,
    physics_system: &mut GenericPhysicsSystem,
    alarm_system: &mut Option<AlarmSystem>,
    control_system: &mut Option<ControlSystem>,
    scenario_engine: &mut Option<nebudcs_scenario::ScenarioEngine>,
    equipment_manager: &Arc<std::sync::RwLock<nebudcs_scripting::CustomEquipmentManager>>,
    is_running: &mut bool,
    speed_multiplier: &mut f64,
    force_step: &mut bool,
    last_values: &mut Vec<f64>,
    last_qualities: &mut Vec<TagQuality>,
) -> ServerResult<()> {
    match command {
        RuntimeCommand::LoadProject { project } => apply_load_project(
            project,
            engine,
            tag_store,
            physics_system,
            alarm_system,
            control_system,
            scenario_engine,
            equipment_manager,
            last_values,
            last_qualities,
        ),
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
            engine.clock_mut().reset();
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
                        for eq in physics_system.equipment_map_mut().values_mut() {
                            let matching_var = eq
                                .tag_bindings()
                                .iter()
                                .find(|(_, tid)| **tid == tag_id)
                                .map(|(var, _)| var.clone());
                            if let Some(var) = matching_var {
                                let _ = eq.set_variable(&var, *f);
                            }
                        }
                        if tag.to_uppercase().contains("STEAM") {
                            physics_system.set_utility("STEAM_PRESSURE", *f);
                        }
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
        RuntimeCommand::Equipment(eq_cmd) => match eq_cmd {
            EquipmentCommand::Define { definition_json } => {
                let def: nebudcs_scripting::CustomEquipmentDefinition =
                    serde_json::from_str(definition_json).map_err(|e| {
                        ServerError::InvalidRequest(format!("Invalid equipment JSON: {e}"))
                    })?;
                nebudcs_scripting::validation::validate_definition(&def)
                    .map_err(|e| ServerError::InvalidRequest(e.to_string()))?;
                let mut eq_mgr = equipment_manager.write().map_err(|_| {
                    ServerError::Internal("Equipment manager lock poisoned".to_string())
                })?;
                eq_mgr
                    .register_definition(def)
                    .map_err(|e| ServerError::InvalidRequest(e.to_string()))?;
                Ok(())
            }
            EquipmentCommand::Instantiate {
                definition_id,
                name,
                parameter_overrides,
                instance_id,
            } => {
                let eq_id = if let Some(id_str) = instance_id {
                    id_str.parse::<EquipmentId>().map_err(|e| {
                        ServerError::InvalidRequest(format!("Invalid EquipmentId: {e}"))
                    })?
                } else {
                    EquipmentId::new()
                };

                let mut eq_mgr = equipment_manager.write().map_err(|_| {
                    ServerError::Internal("Equipment manager lock poisoned".to_string())
                })?;

                eq_mgr
                    .instantiate(eq_id, definition_id, name.clone(), parameter_overrides)
                    .map_err(|e| ServerError::InvalidRequest(e.to_string()))?;

                // Register and bind standard tags for this equipment
                let tag_prefix = name.to_uppercase();
                let status_tag_name = format!("{tag_prefix}.STATUS");
                let status_tag_id = tag_store.resolve_name(&status_tag_name).unwrap_or_else(|| {
                    let tid = TagId::new();
                    tag_store.register_tag(
                        &status_tag_name,
                        tid,
                        TagValue::Integer(0),
                        TagQuality::Good,
                    );
                    tid
                });
                eq_mgr.bind_tag(eq_id, "STATUS", status_tag_id);

                let speed_tag_name = format!("{tag_prefix}.SPEED");
                let speed_tag_id = tag_store.resolve_name(&speed_tag_name).unwrap_or_else(|| {
                    let tid = TagId::new();
                    tag_store.register_tag(
                        &speed_tag_name,
                        tid,
                        TagValue::Float(0.0),
                        TagQuality::Good,
                    );
                    tid
                });
                eq_mgr.bind_tag(eq_id, "speed", speed_tag_id);

                let dp_tag_name = format!("{tag_prefix}.DELTA_P");
                let dp_tag_id = tag_store.resolve_name(&dp_tag_name).unwrap_or_else(|| {
                    let tid = TagId::new();
                    tag_store.register_tag(
                        &dp_tag_name,
                        tid,
                        TagValue::Float(0.0),
                        TagQuality::Good,
                    );
                    tid
                });
                eq_mgr.bind_tag(eq_id, "delta_p", dp_tag_id);

                let flow_tag_name = format!("{tag_prefix}.FLOW");
                let flow_tag_id = tag_store.resolve_name(&flow_tag_name).unwrap_or_else(|| {
                    let tid = TagId::new();
                    tag_store.register_tag(
                        &flow_tag_name,
                        tid,
                        TagValue::Float(0.0),
                        TagQuality::Good,
                    );
                    tid
                });
                eq_mgr.bind_tag(eq_id, "flow", flow_tag_id);

                let trip_tag_name = format!("{tag_prefix}.TRIP");
                let trip_tag_id = tag_store.resolve_name(&trip_tag_name).unwrap_or_else(|| {
                    let tid = TagId::new();
                    tag_store.register_tag(
                        &trip_tag_name,
                        tid,
                        TagValue::Bool(false),
                        TagQuality::Good,
                    );
                    tid
                });
                eq_mgr.bind_tag(eq_id, "is_tripped", trip_tag_id);

                Ok(())
            }
            EquipmentCommand::Start { instance_id } => {
                let eq_id = instance_id.parse::<EquipmentId>().map_err(|e| {
                    ServerError::InvalidRequest(format!("Invalid EquipmentId: {e}"))
                })?;
                let mut eq_mgr = equipment_manager.write().map_err(|_| {
                    ServerError::Internal("Equipment manager lock poisoned".to_string())
                })?;
                eq_mgr
                    .start(eq_id)
                    .map_err(|e| ServerError::InvalidRequest(e.to_string()))?;
                Ok(())
            }
            EquipmentCommand::Stop { instance_id } => {
                let eq_id = instance_id.parse::<EquipmentId>().map_err(|e| {
                    ServerError::InvalidRequest(format!("Invalid EquipmentId: {e}"))
                })?;
                let mut eq_mgr = equipment_manager.write().map_err(|_| {
                    ServerError::Internal("Equipment manager lock poisoned".to_string())
                })?;
                eq_mgr
                    .stop(eq_id)
                    .map_err(|e| ServerError::InvalidRequest(e.to_string()))?;
                Ok(())
            }
            EquipmentCommand::ResetTrip { instance_id } => {
                let eq_id = instance_id.parse::<EquipmentId>().map_err(|e| {
                    ServerError::InvalidRequest(format!("Invalid EquipmentId: {e}"))
                })?;
                let mut eq_mgr = equipment_manager.write().map_err(|_| {
                    ServerError::Internal("Equipment manager lock poisoned".to_string())
                })?;
                let ok = eq_mgr
                    .reset_trip(eq_id)
                    .map_err(|e| ServerError::InvalidRequest(e.to_string()))?;
                if ok {
                    Ok(())
                } else {
                    Err(ServerError::InvalidRequest(
                        "Cannot reset trip: process condition still violated".to_string(),
                    ))
                }
            }
            EquipmentCommand::SetVariable {
                instance_id,
                var_name,
                value,
            } => {
                let eq_id = instance_id.parse::<EquipmentId>().map_err(|e| {
                    ServerError::InvalidRequest(format!("Invalid EquipmentId: {e}"))
                })?;
                let mut eq_mgr = equipment_manager.write().map_err(|_| {
                    ServerError::Internal("Equipment manager lock poisoned".to_string())
                })?;
                if let Some(inst) = eq_mgr.get_instance_mut(eq_id) {
                    inst.set_var(var_name, *value);
                    Ok(())
                } else {
                    Err(ServerError::NotFound(format!(
                        "Equipment instance '{instance_id}' not found"
                    )))
                }
            }
        },
        RuntimeCommand::Library(lib_cmd) => match lib_cmd {
            LibraryCommand::RegisterTemplate { template_json } => {
                let template: nebudcs_scripting::EquipmentTemplate =
                    serde_json::from_str(template_json).map_err(|e| {
                        ServerError::InvalidRequest(format!("Invalid template JSON: {e}"))
                    })?;
                let mut eq_mgr = equipment_manager.write().map_err(|_| {
                    ServerError::Internal("Equipment manager lock poisoned".to_string())
                })?;
                eq_mgr
                    .library_mut()
                    .register_template(template)
                    .map_err(|e| ServerError::InvalidRequest(e.to_string()))?;
                Ok(())
            }
            LibraryCommand::InstantiateTemplate {
                template_id,
                version,
                name,
                instance_id,
                parameter_overrides,
            } => {
                let eq_id = if let Some(id_str) = instance_id {
                    id_str.parse::<EquipmentId>().map_err(|e| {
                        ServerError::InvalidRequest(format!("Invalid EquipmentId: {e}"))
                    })?
                } else {
                    EquipmentId::new()
                };

                let ver_parsed = match version {
                    Some(v) => Some(v.parse::<nebudcs_types::SchemaVersion>().map_err(|e| {
                        ServerError::InvalidRequest(format!("Invalid version format: {e}"))
                    })?),
                    None => None,
                };

                let mut eq_mgr = equipment_manager.write().map_err(|_| {
                    ServerError::Internal("Equipment manager lock poisoned".to_string())
                })?;

                eq_mgr
                    .instantiate_template(
                        template_id,
                        ver_parsed.as_ref(),
                        eq_id,
                        name.clone(),
                        parameter_overrides,
                    )
                    .map_err(|e| ServerError::InvalidRequest(e.to_string()))?;

                // Register and bind standard tags for this equipment
                let tag_prefix = name.to_uppercase();
                let status_tag_name = format!("{tag_prefix}.STATUS");
                let status_tag_id = tag_store.resolve_name(&status_tag_name).unwrap_or_else(|| {
                    let tid = TagId::new();
                    tag_store.register_tag(
                        &status_tag_name,
                        tid,
                        TagValue::Integer(0),
                        TagQuality::Good,
                    );
                    tid
                });
                eq_mgr.bind_tag(eq_id, "STATUS", status_tag_id);

                let speed_tag_name = format!("{tag_prefix}.SPEED");
                let speed_tag_id = tag_store.resolve_name(&speed_tag_name).unwrap_or_else(|| {
                    let tid = TagId::new();
                    tag_store.register_tag(
                        &speed_tag_name,
                        tid,
                        TagValue::Float(0.0),
                        TagQuality::Good,
                    );
                    tid
                });
                eq_mgr.bind_tag(eq_id, "speed", speed_tag_id);

                let dp_tag_name = format!("{tag_prefix}.DELTA_P");
                let dp_tag_id = tag_store.resolve_name(&dp_tag_name).unwrap_or_else(|| {
                    let tid = TagId::new();
                    tag_store.register_tag(
                        &dp_tag_name,
                        tid,
                        TagValue::Float(0.0),
                        TagQuality::Good,
                    );
                    tid
                });
                eq_mgr.bind_tag(eq_id, "delta_p", dp_tag_id);

                let flow_tag_name = format!("{tag_prefix}.FLOW");
                let flow_tag_id = tag_store.resolve_name(&flow_tag_name).unwrap_or_else(|| {
                    let tid = TagId::new();
                    tag_store.register_tag(
                        &flow_tag_name,
                        tid,
                        TagValue::Float(0.0),
                        TagQuality::Good,
                    );
                    tid
                });
                eq_mgr.bind_tag(eq_id, "flow", flow_tag_id);

                Ok(())
            }
            LibraryCommand::RegisterPackage { package_json } => {
                let package: nebudcs_scripting::EquipmentPackage =
                    serde_json::from_str(package_json).map_err(|e| {
                        ServerError::InvalidRequest(format!("Invalid package JSON: {e}"))
                    })?;
                let mut eq_mgr = equipment_manager.write().map_err(|_| {
                    ServerError::Internal("Equipment manager lock poisoned".to_string())
                })?;
                eq_mgr
                    .library_mut()
                    .register_package(package)
                    .map_err(|e| ServerError::InvalidRequest(e.to_string()))?;
                Ok(())
            }
            LibraryCommand::InstantiatePackage {
                package_id,
                version,
                instance_prefix,
                parameter_overrides,
            } => {
                let ver_parsed = match version {
                    Some(v) => Some(v.parse::<nebudcs_types::SchemaVersion>().map_err(|e| {
                        ServerError::InvalidRequest(format!("Invalid version format: {e}"))
                    })?),
                    None => None,
                };

                let mut eq_mgr = equipment_manager.write().map_err(|_| {
                    ServerError::Internal("Equipment manager lock poisoned".to_string())
                })?;

                let res = eq_mgr
                    .instantiate_package(
                        package_id,
                        ver_parsed.as_ref(),
                        instance_prefix,
                        parameter_overrides,
                    )
                    .map_err(|e| ServerError::InvalidRequest(e.to_string()))?;

                for child in &res.instances {
                    let tag_prefix = child.name.to_uppercase();
                    let status_tag_name = format!("{tag_prefix}.STATUS");
                    let status_tag_id =
                        tag_store.resolve_name(&status_tag_name).unwrap_or_else(|| {
                            let tid = TagId::new();
                            tag_store.register_tag(
                                &status_tag_name,
                                tid,
                                TagValue::Integer(0),
                                TagQuality::Good,
                            );
                            tid
                        });
                    eq_mgr.bind_tag(child.id, "STATUS", status_tag_id);
                }

                Ok(())
            }
        },
        RuntimeCommand::ControllerAction {
            controller_id,
            action,
        } => {
            if let Some(ctrl_sys) = control_system.as_mut() {
                let target = ctrl_sys
                    .controllers_mut()
                    .values_mut()
                    .find(|c| c.id().to_string() == *controller_id || c.name() == *controller_id);
                if let Some(c) = target {
                    match action.to_lowercase().as_str() {
                        "start" => c
                            .start()
                            .map_err(|e| ServerError::InvalidRequest(e.to_string()))?,
                        "stop" => c
                            .stop()
                            .map_err(|e| ServerError::InvalidRequest(e.to_string()))?,
                        "pause" => c
                            .pause()
                            .map_err(|e| ServerError::InvalidRequest(e.to_string()))?,
                        "resume" => c
                            .resume()
                            .map_err(|e| ServerError::InvalidRequest(e.to_string()))?,
                        "reset" => c
                            .reset()
                            .map_err(|e| ServerError::InvalidRequest(e.to_string()))?,
                        "power_off" => c
                            .power_off()
                            .map_err(|e| ServerError::InvalidRequest(e.to_string()))?,
                        "boot" => c
                            .boot()
                            .map_err(|e| ServerError::InvalidRequest(e.to_string()))?,
                        "shutdown" => c
                            .shutdown()
                            .map_err(|e| ServerError::InvalidRequest(e.to_string()))?,
                        "switchover" => {
                            let next_role = if c.redundancy().role
                                == nebudcs_control::runtime::RedundancyRole::Primary
                            {
                                nebudcs_control::runtime::RedundancyRole::Standby
                            } else {
                                nebudcs_control::runtime::RedundancyRole::Primary
                            };
                            c.trigger_switchover(next_role);
                        }
                        "clear_fault" => {
                            let _ = c.clear_fault();
                        }
                        other => {
                            return Err(ServerError::InvalidRequest(format!(
                                "Unknown controller action: {other}"
                            )))
                        }
                    }
                    Ok(())
                } else {
                    Err(ServerError::NotFound(format!(
                        "Controller '{controller_id}' not found"
                    )))
                }
            } else {
                Err(ServerError::NotFound(
                    "Control system not active".to_string(),
                ))
            }
        }
        RuntimeCommand::ControlModuleAction {
            controller_id,
            module_id,
            mode,
            sp,
            manual_out,
        } => {
            if let Some(ctrl_sys) = control_system.as_mut() {
                let target = ctrl_sys
                    .controllers_mut()
                    .values_mut()
                    .find(|c| c.id().to_string() == *controller_id || c.name() == *controller_id);
                if let Some(c) = target {
                    if let Some(cm) = c.control_module_mut(module_id) {
                        if let Some(sp_val) = sp {
                            cm.set_sp(*sp_val);
                        }
                        if let Some(out_val) = manual_out {
                            cm.set_manual_out(*out_val);
                        }
                        if let Some(mode_str) = mode {
                            let new_mode = match mode_str.to_uppercase().as_str() {
                                "MAN" | "MANUAL" => {
                                    nebudcs_control::runtime::ControlModuleMode::Manual
                                }
                                "CAS" | "CASCADE" => {
                                    nebudcs_control::runtime::ControlModuleMode::Cascade
                                }
                                "RCAS" | "REMOTECASCADE" => {
                                    nebudcs_control::runtime::ControlModuleMode::RemoteCascade
                                }
                                _ => nebudcs_control::runtime::ControlModuleMode::Auto,
                            };
                            cm.set_mode(new_mode)
                                .map_err(|e| ServerError::InvalidRequest(e.to_string()))?;
                        }
                        Ok(())
                    } else {
                        Err(ServerError::NotFound(format!(
                            "Control module '{module_id}' not found"
                        )))
                    }
                } else {
                    Err(ServerError::NotFound(format!(
                        "Controller '{controller_id}' not found"
                    )))
                }
            } else {
                Err(ServerError::NotFound(
                    "Control system not active".to_string(),
                ))
            }
        }
        RuntimeCommand::ControllerFaultAction {
            controller_id,
            fault_type,
            description,
            recoverable,
        } => {
            if let Some(ctrl_sys) = control_system.as_mut() {
                let target = ctrl_sys
                    .controllers_mut()
                    .values_mut()
                    .find(|c| c.id().to_string() == *controller_id || c.name() == *controller_id);
                if let Some(c) = target {
                    let sim_time = engine.clock().time().seconds();
                    match fault_type.to_lowercase().as_str() {
                        "clear" => {
                            let _ = c.clear_fault();
                        }
                        "watchdog" | "watchdogtimeout" => {
                            c.inject_watchdog_timeout(sim_time);
                        }
                        "overrun" | "taskoverrun" => {
                            let task_id =
                                c.tasks().keys().next().cloned().unwrap_or_else(TaskId::new);
                            c.inject_task_overrun(task_id, sim_time);
                        }
                        "cpu_overload" | "cpuoverload" => {
                            c.inject_cpu_overload(sim_time);
                        }
                        "memory_exhaustion" | "memoryexhaustion" => {
                            c.inject_memory_exhaustion(sim_time);
                        }
                        "power_loss" | "powerloss" => {
                            c.inject_power_loss(sim_time);
                        }
                        _ => {
                            c.inject_fault(nebudcs_control::runtime::ControllerFault {
                                code: "CUSTOM_FAULT".to_string(),
                                message: description.clone(),
                                timestamp_sim: sim_time,
                                recoverable: *recoverable,
                            });
                        }
                    }
                    Ok(())
                } else {
                    Err(ServerError::NotFound(format!(
                        "Controller '{controller_id}' not found"
                    )))
                }
            } else {
                Err(ServerError::NotFound(
                    "Control system not active".to_string(),
                ))
            }
        }
        RuntimeCommand::HalChannelFault {
            channel_address,
            fault,
            forced_value,
        } => {
            if let Some(ctrl_sys) = control_system.as_mut() {
                let parsed_addr: Result<nebudcs_control::hal::IoChannelAddress, _> =
                    channel_address.parse();
                let addr = parsed_addr
                    .map_err(|e| ServerError::InvalidRequest(format!("Invalid address: {e}")))?;
                let ch = ctrl_sys.io_system_mut().channel_mut(&addr).ok_or_else(|| {
                    ServerError::NotFound(format!("Channel '{channel_address}' not found"))
                })?;

                match ch {
                    nebudcs_control::hal::IoChannel::Ai(c) => {
                        c.forced_value = *forced_value;
                        if let Some(f_str) = fault {
                            match f_str.to_lowercase().as_str() {
                                "normal" | "clear" => {
                                    c.forced_fault = None;
                                    c.forced_value = None;
                                    c.fault_state = nebudcs_control::hal::AnalogFaultState::Normal;
                                    c.quality = nebudcs_control::hal::IoSignalQuality::Good;
                                }
                                "wirebreak" => {
                                    c.forced_fault =
                                        Some(nebudcs_control::hal::AnalogFaultState::WireBreak);
                                    c.fault_state =
                                        nebudcs_control::hal::AnalogFaultState::WireBreak;
                                    c.quality = nebudcs_control::hal::IoSignalQuality::Bad;
                                }
                                "shortcircuit" => {
                                    c.forced_fault =
                                        Some(nebudcs_control::hal::AnalogFaultState::ShortCircuit);
                                    c.fault_state =
                                        nebudcs_control::hal::AnalogFaultState::ShortCircuit;
                                    c.quality = nebudcs_control::hal::IoSignalQuality::Bad;
                                }
                                "underrange" => {
                                    c.forced_fault =
                                        Some(nebudcs_control::hal::AnalogFaultState::UnderRange);
                                    c.fault_state =
                                        nebudcs_control::hal::AnalogFaultState::UnderRange;
                                    c.quality = nebudcs_control::hal::IoSignalQuality::Uncertain;
                                }
                                "overrange" => {
                                    c.forced_fault =
                                        Some(nebudcs_control::hal::AnalogFaultState::OverRange);
                                    c.fault_state =
                                        nebudcs_control::hal::AnalogFaultState::OverRange;
                                    c.quality = nebudcs_control::hal::IoSignalQuality::Uncertain;
                                }
                                "frozen" => {
                                    c.forced_fault =
                                        Some(nebudcs_control::hal::AnalogFaultState::Frozen);
                                    c.fault_state = nebudcs_control::hal::AnalogFaultState::Frozen;
                                    c.quality = nebudcs_control::hal::IoSignalQuality::Uncertain;
                                }
                                other => {
                                    return Err(ServerError::InvalidRequest(format!(
                                        "Unknown analog fault: {other}"
                                    )))
                                }
                            }
                        }
                    }
                    nebudcs_control::hal::IoChannel::Di(c) => {
                        if let Some(val) = forced_value {
                            c.forced_fault = Some(if *val >= 0.5 {
                                nebudcs_control::hal::DigitalFaultState::StuckTrue
                            } else {
                                nebudcs_control::hal::DigitalFaultState::StuckFalse
                            });
                        }
                        if let Some(f_str) = fault {
                            match f_str.to_lowercase().as_str() {
                                "normal" | "clear" => c.forced_fault = None,
                                "stucktrue" => {
                                    c.forced_fault =
                                        Some(nebudcs_control::hal::DigitalFaultState::StuckTrue)
                                }
                                "stuckfalse" => {
                                    c.forced_fault =
                                        Some(nebudcs_control::hal::DigitalFaultState::StuckFalse)
                                }
                                "inverted" => {
                                    c.forced_fault =
                                        Some(nebudcs_control::hal::DigitalFaultState::Inverted)
                                }
                                other => {
                                    return Err(ServerError::InvalidRequest(format!(
                                        "Unknown digital fault: {other}"
                                    )))
                                }
                            }
                        }
                    }
                    nebudcs_control::hal::IoChannel::Ao(c) => {
                        if let Some(val) = forced_value {
                            c.write_command(*val, 0.0);
                        }
                    }
                    nebudcs_control::hal::IoChannel::Do(c) => {
                        if let Some(val) = forced_value {
                            c.write_command(*val >= 0.5, 0.0);
                        }
                    }
                }
                Ok(())
            } else {
                Err(ServerError::NotFound(
                    "Control system not active".to_string(),
                ))
            }
        }
        RuntimeCommand::SifResetAction { sif_id, actor } => {
            if let Some(ctrl) = control_system.as_mut() {
                let sim_time = engine.clock().time().seconds();
                let io_image = ctrl.io_image();
                ctrl.sis_mut()
                    .reset_sif(sif_id, actor.clone(), sim_time, &io_image)
                    .map_err(ServerError::InvalidRequest)?;
                Ok(())
            } else {
                Err(ServerError::NotFound(
                    "Control system not active".to_string(),
                ))
            }
        }
        RuntimeCommand::SifBypassAction {
            sif_id,
            bypass_type,
            authorized_by,
            reason,
            duration_seconds,
            active,
        } => {
            if let Some(ctrl) = control_system.as_mut() {
                let sim_time = engine.clock().time().seconds();
                let bypass = if *active {
                    Some(nebudcs_control::sis::sif::SifBypass {
                        bypass_type: bypass_type.clone(),
                        authorized_by: authorized_by.clone(),
                        reason: reason.clone(),
                        start_sim_time: sim_time,
                        duration_seconds: *duration_seconds,
                        active: true,
                    })
                } else {
                    None
                };
                ctrl.sis_mut()
                    .set_bypass(sif_id, bypass, sim_time)
                    .map_err(ServerError::InvalidRequest)?;
                Ok(())
            } else {
                Err(ServerError::NotFound(
                    "Control system not active".to_string(),
                ))
            }
        }
        RuntimeCommand::SifProofTestAction { sif_id, technician } => {
            if let Some(ctrl) = control_system.as_mut() {
                let sim_time = engine.clock().time().seconds();
                ctrl.sis_mut()
                    .run_proof_test(sif_id, technician.clone(), sim_time)
                    .map_err(ServerError::InvalidRequest)?;
                Ok(())
            } else {
                Err(ServerError::NotFound(
                    "Control system not active".to_string(),
                ))
            }
        }
    }
}

/// Constructs live SIF detail DTOs from the safety instrumented system.
fn build_sis_dtos(ctrl_sys: &ControlSystem) -> Vec<SifDetailDto> {
    let sis = ctrl_sys.sis();
    let io_image = ctrl_sys.io_image();

    sis.sifs()
        .values()
        .map(|sif| {
            let sensor_channels = sif
                .config()
                .sensor_tags
                .iter()
                .enumerate()
                .map(|(idx, tag)| {
                    let sig = io_image.read_input(tag);
                    let (val, quality, faulted) = if let Some(s) = sig {
                        (
                            s.value,
                            format!("{:?}", s.quality),
                            s.quality == TagQuality::Bad,
                        )
                    } else {
                        (0.0, "Good".to_string(), false)
                    };
                    SifSensorChannelDto {
                        id: format!("CH-{}", idx + 1),
                        tag: tag.clone(),
                        value: val,
                        quality,
                        unit: None,
                        faulted,
                    }
                })
                .collect();

            let final_elements = sif
                .config()
                .final_elements
                .iter()
                .map(|fe| SifFinalElementDto {
                    target_equipment: fe.target_equipment.clone(),
                    actuator_var: fe.actuator_var.clone(),
                    safe_state: format!("{:?}", fe.safe_state),
                    de_energize_to_trip: fe.de_energize_to_trip,
                    action_type: format!("{:?}", fe.action_type),
                    feedback_tag: fe.feedback_tag.clone(),
                })
                .collect();

            let diag = sif.diagnostics();
            let diagnostics = SifDiagnosticsDto {
                total_sensors: diag.total_sensors,
                healthy_sensors: diag.healthy_sensors,
                faulted_sensors: diag.faulted_sensors,
                final_elements_count: diag.final_elements_count,
                trip_count: diag.trip_count,
                last_trip_sim_time: diag.last_trip_sim_time,
                last_trip_cause: diag.last_trip_cause.clone(),
                last_reset_sim_time: diag.last_reset_sim_time,
                voting_scheme: diag.voting_scheme.clone(),
                delay_elapsed_seconds: diag.delay_elapsed_seconds,
            };

            let bypass = sif.bypass_info().map(|b| SifBypassDto {
                bypass_type: b.bypass_type.clone(),
                authorized_by: b.authorized_by.clone(),
                reason: b.reason.clone(),
                start_sim_time: b.start_sim_time,
                duration_seconds: b.duration_seconds,
                active: b.active,
            });

            SifDetailDto {
                id: sif.id().to_string(),
                name: sif.name().to_string(),
                description: sif.config().description.clone(),
                hazard_id: sif.config().hazard_id.clone(),
                hazard_scenario: sif.config().hazard_scenario.clone(),
                target_sil: sif.config().target_sil.clone(),
                direction: format!("{:?}", sif.config().direction),
                threshold: sif.config().threshold,
                reset_deadband: sif.config().reset_deadband,
                trip_delay_seconds: sif.config().trip_delay_seconds,
                architecture: format!("{:?}", sif.config().architecture),
                degradation_policy: format!("{:?}", sif.config().degradation_policy),
                state: format!("{:?}", sif.state()),
                trip_latched: sif.is_tripped(),
                sensor_channels,
                final_elements,
                diagnostics,
                bypass,
                pfd_avg_estimate: sif.config().pfd_avg_estimate,
            }
        })
        .collect()
}

/// Constructs the Cause & Effect matrix DTO from the safety instrumented system.
fn build_cause_effect_dto(ctrl_sys: &ControlSystem) -> CauseEffectMatrixDto {
    let ce = ctrl_sys.sis().cause_effect_matrix();
    CauseEffectMatrixDto {
        causes: ce
            .causes
            .into_iter()
            .map(|c| CauseEntryDto {
                sif_id: c.sif_id,
                description: c.description,
                hazard_id: c.hazard_id,
                threshold: c.threshold,
                voting: c.voting,
                status: c.status,
            })
            .collect(),
        effects: ce
            .effects
            .into_iter()
            .map(|e| EffectEntryDto {
                equipment_id: e.equipment_id,
                variable: e.variable,
                safe_state: e.safe_state,
                action_type: format!("{:?}", e.action_type),
            })
            .collect(),
        cells: ce
            .cells
            .into_iter()
            .map(|cell| CauseEffectCellDto {
                cause_index: cell.cause_index,
                effect_index: cell.effect_index,
                active_link: cell.active_link,
                commanded_state: cell.commanded_state.map(|s| format!("{:?}", s)),
                is_asserted: cell.is_asserted,
            })
            .collect(),
    }
}

/// Constructs the high-level SIS executive status summary DTO.
fn build_sis_summary_dto(ctrl_sys: &ControlSystem) -> SisStatusSummaryDto {
    let sis = ctrl_sys.sis();
    let mut armed_count = 0;
    let mut tripped_count = 0;
    let mut bypassed_count = 0;
    let mut degraded_count = 0;

    for sif in sis.sifs().values() {
        match sif.state() {
            nebudcs_control::sis::sif::SifState::Armed => armed_count += 1,
            nebudcs_control::sis::sif::SifState::Tripped => tripped_count += 1,
            nebudcs_control::sis::sif::SifState::Bypassed => bypassed_count += 1,
            nebudcs_control::sis::sif::SifState::Degraded => degraded_count += 1,
            _ => armed_count += 1,
        }
    }

    let active_overrides_count = ctrl_sys.active_safety_overrides_count();

    SisStatusSummaryDto {
        enabled: sis.is_enabled(),
        sif_count: sis.sifs().len(),
        armed_count,
        tripped_count,
        bypassed_count,
        degraded_count,
        proof_tests_count: sis.proof_tests().len(),
        active_overrides_count,
    }
}

/// Constructs controller detail DTOs from active virtual controllers in the control system.
fn build_controllers_dtos(ctrl_sys: &ControlSystem) -> Vec<ControllerDetailDto> {
    ctrl_sys
        .controllers()
        .values()
        .map(|ctrl| {
            let tasks = ctrl
                .tasks()
                .values()
                .map(|t| {
                    let (task_type_str, period_sec) = match t.task_type() {
                        nebudcs_control::runtime::TaskType::Periodic { period_seconds } => {
                            ("PERIODIC".to_string(), Some(*period_seconds))
                        }
                        nebudcs_control::runtime::TaskType::Event { .. } => {
                            ("EVENT".to_string(), None)
                        }
                        nebudcs_control::runtime::TaskType::Continuous => {
                            ("CONTINUOUS".to_string(), None)
                        }
                    };
                    ControllerTaskDto {
                        id: t.id().to_string(),
                        name: t.name().to_string(),
                        task_type: task_type_str,
                        period_seconds: period_sec,
                        priority: t.priority(),
                        enabled: t.is_enabled(),
                        execution_count: t.execution_count(),
                        overrun_count: t.overrun_count(),
                        last_duration_us: t.last_duration_us(),
                        max_duration_us: t.max_duration_us(),
                        program_count: t.programs().len(),
                    }
                })
                .collect();

            let diag = ctrl.diagnostics();
            ControllerDetailDto {
                id: ctrl.id().to_string(),
                name: ctrl.name().to_string(),
                description: ctrl.config().description.clone(),
                lifecycle: ctrl.lifecycle().to_string(),
                watchdog_timeout_seconds: ctrl.config().watchdog_timeout_seconds,
                tasks,
                input_points: ctrl.io_image().inputs().keys().cloned().collect(),
                output_points: ctrl.io_image().outputs().keys().cloned().collect(),
                scan_count: diag.scan_count,
                task_execution_count: diag.task_execution_count,
                avg_scan_duration_us: diag.avg_scan_duration_us,
                max_scan_duration_us: diag.max_scan_duration_us,
                last_scan_duration_us: diag.last_scan_duration_us,
                watchdog_trips: diag.watchdog_trips,
                overrun_count: diag.overrun_count,
                fault: ctrl.fault().map(|f| ControllerFaultDto {
                    code: f.code.clone(),
                    message: f.message.clone(),
                    timestamp_sim: f.timestamp_sim,
                    recoverable: f.recoverable,
                }),
                cpu_profile: Some(CpuProfileDto {
                    execution_budget_us: ctrl.cpu_profile().execution_budget_us,
                    max_scan_frequency_hz: ctrl.cpu_profile().max_scan_frequency_hz,
                    utilization_threshold_pct: ctrl.cpu_profile().utilization_threshold_pct,
                    architecture: ctrl.cpu_profile().architecture.clone(),
                }),
                memory_profile: Some(MemoryProfileDto {
                    program_memory_bytes: ctrl.memory_profile().program_memory_bytes,
                    data_memory_bytes: ctrl.memory_profile().data_memory_bytes,
                    retentive_memory_bytes: ctrl.memory_profile().retentive_memory_bytes,
                    program_memory_used_bytes: ctrl.memory_profile().program_memory_used_bytes,
                    data_memory_used_bytes: ctrl.memory_profile().data_memory_used_bytes,
                    retentive_memory_used_bytes: ctrl.memory_profile().retentive_memory_used_bytes,
                }),
                firmware_profile: Some(FirmwareProfileDto {
                    version: ctrl.firmware_profile().version.clone(),
                    build: ctrl.firmware_profile().build.clone(),
                    iec_standard: ctrl.firmware_profile().iec_standard.clone(),
                }),
                redundancy: Some(RedundancyStatusDto {
                    role: format!("{:?}", ctrl.redundancy().role),
                    status: format!("{:?}", ctrl.redundancy().status),
                    partner_id: ctrl.redundancy().partner_id.clone(),
                    sync_frequency_hz: ctrl.redundancy().sync_frequency_hz,
                    switchover_timeout_seconds: ctrl.redundancy().switchover_timeout_seconds,
                    last_sync_sim_time: ctrl.redundancy().last_sync_sim_time,
                    sync_cycle_count: ctrl.redundancy().sync_cycle_count,
                }),
                control_modules: ctrl
                    .control_modules()
                    .values()
                    .map(|cm| DcsControlModuleDto {
                        id: cm.id().to_string(),
                        name: cm.name().to_string(),
                        description: cm.description().to_string(),
                        mode: cm.mode().as_str().to_string(),
                        execution_rate_seconds: cm.execution_rate_seconds(),
                        pv_signal: cm.pv_key().to_string(),
                        sp: cm.sp(),
                        out_signal: cm.out_key().to_string(),
                        strategy_type: format!("{:?}", cm.strategy()),
                        kp: cm.pid().config().kp,
                        ti: cm.pid().config().ti,
                        td: cm.pid().config().td,
                        output_min: cm.pid().config().out_min,
                        output_max: cm.pid().config().out_max,
                        permissives: cm.permissives().to_vec(),
                        interlocks: cm.interlocks().to_vec(),
                        safe_position: cm.safe_position(),
                        current_pv: cm.current_pv(),
                        current_out: cm.current_out(),
                        is_interlocked: cm.is_interlocked(),
                        permissives_ok: cm.permissives_ok(),
                        tracking_active: cm.status().tracking_active,
                    })
                    .collect(),
            }
        })
        .collect()
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

/// Constructs a comprehensive HalSystemSummaryDto from the active I/O HAL subsystem.
pub fn build_hal_dto(hal: &IoSystem) -> HalSystemSummaryDto {
    let mut racks_dto = Vec::new();
    let mut total_modules = 0;
    let mut total_channels = 0;

    for rack in hal.racks.values() {
        let mut chassis_dto = Vec::new();
        for chassis in rack.chassis.values() {
            let mut modules_dto = Vec::new();
            for module in chassis.modules.values() {
                total_modules += 1;
                let mut channels_dto = Vec::new();
                for channel in &module.channels {
                    total_channels += 1;
                    channels_dto.push(match channel {
                        IoChannel::Ai(c) => HalChannelDto {
                            id: c.id.to_string(),
                            channel_number: c.channel_number,
                            address: c.address.to_string(),
                            name: c.name.clone(),
                            kind: "AI".to_string(),
                            value: channel.value_as_float(),
                            raw_counts: c.raw_counts,
                            quality: channel.quality().to_string(),
                            fault_state: format!("{:?}", c.fault_state),
                            status: format!("{:?}", c.status),
                            engineering_unit: c.scaling.unit.clone().unwrap_or_default(),
                            lrv: c.scaling.lrv,
                            urv: c.scaling.urv,
                            fallback_behavior: None,
                            forced_value: c.forced_value,
                        },
                        IoChannel::Ao(c) => HalChannelDto {
                            id: c.id.to_string(),
                            channel_number: c.channel_number,
                            address: c.address.to_string(),
                            name: c.name.clone(),
                            kind: "AO".to_string(),
                            value: channel.value_as_float(),
                            raw_counts: c.raw_counts,
                            quality: channel.quality().to_string(),
                            fault_state: "Normal".to_string(),
                            status: format!("{:?}", c.status),
                            engineering_unit: c.scaling.unit.clone().unwrap_or_default(),
                            lrv: c.scaling.lrv,
                            urv: c.scaling.urv,
                            fallback_behavior: Some(format!("{:?}", c.fallback_behavior)),
                            forced_value: None,
                        },
                        IoChannel::Di(c) => HalChannelDto {
                            id: c.id.to_string(),
                            channel_number: c.channel_number,
                            address: c.address.to_string(),
                            name: c.name.clone(),
                            kind: "DI".to_string(),
                            value: channel.value_as_float(),
                            raw_counts: if channel.value_as_bool() { 1 } else { 0 },
                            quality: channel.quality().to_string(),
                            fault_state: c
                                .forced_fault
                                .map(|f| format!("{:?}", f))
                                .unwrap_or_else(|| "Normal".to_string()),
                            status: format!("{:?}", c.status),
                            engineering_unit: "bool".to_string(),
                            lrv: 0.0,
                            urv: 1.0,
                            fallback_behavior: None,
                            forced_value: None,
                        },
                        IoChannel::Do(c) => HalChannelDto {
                            id: c.id.to_string(),
                            channel_number: c.channel_number,
                            address: c.address.to_string(),
                            name: c.name.clone(),
                            kind: "DO".to_string(),
                            value: channel.value_as_float(),
                            raw_counts: if channel.value_as_bool() { 1 } else { 0 },
                            quality: channel.quality().to_string(),
                            fault_state: "Normal".to_string(),
                            status: format!("{:?}", c.status),
                            engineering_unit: "bool".to_string(),
                            lrv: 0.0,
                            urv: 1.0,
                            fallback_behavior: Some(format!("{:?}", c.fallback_behavior)),
                            forced_value: None,
                        },
                    });
                }
                modules_dto.push(HalModuleDto {
                    slot: module.slot,
                    name: module.name.clone(),
                    module_type: format!("{:?}", module.module_type),
                    status: if module.is_healthy {
                        "Healthy".to_string()
                    } else {
                        "Faulted".to_string()
                    },
                    channels: channels_dto,
                });
            }
            chassis_dto.push(HalChassisDto {
                chassis_number: chassis.chassis_number,
                name: chassis.name.clone(),
                bus_ok: chassis.bus_ok,
                modules: modules_dto,
            });
        }
        racks_dto.push(HalRackDto {
            rack_number: rack.rack_number,
            name: rack.name.clone(),
            power_ok: rack.power_ok,
            chassis: chassis_dto,
        });
    }

    let mappings_dto = hal
        .mappings
        .iter()
        .map(|m| HalMappingDto {
            controller_id: m.controller_id.to_string(),
            signal_key: m.signal_key.clone(),
            channel_address: m.channel_address.to_string(),
            direction: match m.direction {
                nebudcs_control::hal::MappingDirection::Input => "Input".to_string(),
                nebudcs_control::hal::MappingDirection::Output => "Output".to_string(),
            },
            target_tag: m.target_tag.map(|t| t.to_string()),
            target_actuator: m.target_actuator.map(|a| a.to_string()),
            priority: Some(m.priority as u8),
        })
        .collect();

    HalSystemSummaryDto {
        id: hal.id.to_string(),
        name: hal.name.clone(),
        rack_count: hal.racks.len(),
        module_count: total_modules,
        channel_count: total_channels,
        mapping_count: hal.mappings.len(),
        diagnostics: HalDiagnosticsDto {
            channels_scanned: hal.diagnostics.channels_scanned,
            faulted_channels: hal.diagnostics.faulted_channels,
            last_scan_duration_us: hal.diagnostics.last_scan_duration_us,
            max_scan_duration_us: hal.diagnostics.max_scan_duration_us,
            avg_scan_duration_us: hal.diagnostics.avg_scan_duration_us,
        },
        racks: racks_dto,
        mappings: mappings_dto,
    }
}

/// Constructs a default industrial I/O HAL system with Rack 1, Chassis 1, and AI8/AO8/DI16/DO16 modules.
pub fn build_default_hal_system() -> IoSystem {
    let mut hal = IoSystem::new("HAL-MAIN");
    let mut rack = nebudcs_control::hal::IoRack::new(1, "RACK-01");
    let mut chassis = nebudcs_control::hal::IoChassis::new(1, "CHASSIS-01");

    let mut ai_mod = nebudcs_control::hal::IoModule::new(
        1,
        "AI-01",
        nebudcs_control::hal::IoModuleType::AnalogInput8,
    );
    let mut ao_mod = nebudcs_control::hal::IoModule::new(
        2,
        "AO-01",
        nebudcs_control::hal::IoModuleType::AnalogOutput8,
    );
    let mut di_mod = nebudcs_control::hal::IoModule::new(
        3,
        "DI-01",
        nebudcs_control::hal::IoModuleType::DigitalInput16,
    );
    let mut do_mod = nebudcs_control::hal::IoModule::new(
        4,
        "DO-01",
        nebudcs_control::hal::IoModuleType::DigitalOutput16,
    );

    for ch_num in 1..=8 {
        let addr_ai = nebudcs_control::hal::IoChannelAddress::new(
            1,
            1,
            1,
            ch_num,
            nebudcs_control::hal::ChannelKind::AI,
        );
        let scaling =
            nebudcs_control::hal::AnalogScalingConfig::new_4_20ma(0.0, 100.0, "%").unwrap();
        let ai_ch = nebudcs_control::hal::AnalogInputChannel::new(
            ch_num,
            addr_ai,
            format!("AI-{:02}", ch_num),
            scaling,
        );
        let _ = ai_mod.add_channel(IoChannel::Ai(ai_ch));

        let addr_ao = nebudcs_control::hal::IoChannelAddress::new(
            1,
            1,
            2,
            ch_num,
            nebudcs_control::hal::ChannelKind::AO,
        );
        let scaling_ao =
            nebudcs_control::hal::AnalogScalingConfig::new_4_20ma(0.0, 100.0, "%").unwrap();
        let ao_ch = nebudcs_control::hal::AnalogOutputChannel::new(
            ch_num,
            addr_ao,
            format!("AO-{:02}", ch_num),
            scaling_ao,
            nebudcs_control::hal::OutputFallbackBehavior::FailClosed,
        );
        let _ = ao_mod.add_channel(IoChannel::Ao(ao_ch));
    }

    for ch_num in 1..=16 {
        let addr_di = nebudcs_control::hal::IoChannelAddress::new(
            1,
            1,
            3,
            ch_num,
            nebudcs_control::hal::ChannelKind::DI,
        );
        let di_ch = nebudcs_control::hal::DigitalInputChannel::new(
            ch_num,
            addr_di,
            format!("DI-{:02}", ch_num),
            false,
        );
        let _ = di_mod.add_channel(IoChannel::Di(di_ch));

        let addr_do = nebudcs_control::hal::IoChannelAddress::new(
            1,
            1,
            4,
            ch_num,
            nebudcs_control::hal::ChannelKind::DO,
        );
        let do_ch = nebudcs_control::hal::DigitalOutputChannel::new(
            ch_num,
            addr_do,
            format!("DO-{:02}", ch_num),
            nebudcs_control::hal::DigitalFallbackBehavior::FailOpen,
        );
        let _ = do_mod.add_channel(IoChannel::Do(do_ch));
    }

    let _ = chassis.add_module(ai_mod);
    let _ = chassis.add_module(ao_mod);
    let _ = chassis.add_module(di_mod);
    let _ = chassis.add_module(do_mod);
    let _ = rack.add_chassis(chassis);
    let _ = hal.add_rack(rack);

    hal
}

/// Builds an authoritative `IoSystem` from an imported or project `HalSystemSummaryDto`.
pub fn build_hal_system_from_dto(dto: &HalSystemSummaryDto) -> Result<IoSystem, String> {
    let mut hal = IoSystem::new(&dto.name);
    for r in &dto.racks {
        let mut rack = nebudcs_control::hal::IoRack::new(r.rack_number, &r.name);
        rack.power_ok = r.power_ok;
        for c in &r.chassis {
            let mut chassis = nebudcs_control::hal::IoChassis::new(c.chassis_number, &c.name);
            chassis.bus_ok = c.bus_ok;
            for m in &c.modules {
                let mod_type = match m.module_type.as_str() {
                    "AnalogOutput8" => nebudcs_control::hal::IoModuleType::AnalogOutput8,
                    "DigitalInput16" => nebudcs_control::hal::IoModuleType::DigitalInput16,
                    "DigitalOutput16" => nebudcs_control::hal::IoModuleType::DigitalOutput16,
                    _ => nebudcs_control::hal::IoModuleType::AnalogInput8,
                };
                let mut module = nebudcs_control::hal::IoModule::new(m.slot, &m.name, mod_type);
                for ch in &m.channels {
                    let kind = match ch.kind.to_uppercase().as_str() {
                        "AO" => nebudcs_control::hal::ChannelKind::AO,
                        "DI" => nebudcs_control::hal::ChannelKind::DI,
                        "DO" => nebudcs_control::hal::ChannelKind::DO,
                        _ => nebudcs_control::hal::ChannelKind::AI,
                    };
                    let addr = nebudcs_control::hal::IoChannelAddress::new(
                        r.rack_number,
                        c.chassis_number,
                        m.slot,
                        ch.channel_number,
                        kind,
                    );
                    match kind {
                        nebudcs_control::hal::ChannelKind::AI => {
                            let scaling = nebudcs_control::hal::AnalogScalingConfig::new_4_20ma(
                                ch.lrv,
                                ch.urv,
                                &ch.engineering_unit,
                            )
                            .unwrap_or_else(|_| {
                                nebudcs_control::hal::AnalogScalingConfig::new_4_20ma(
                                    0.0, 100.0, "%",
                                )
                                .unwrap()
                            });
                            let ai = nebudcs_control::hal::AnalogInputChannel::new(
                                ch.channel_number,
                                addr,
                                &ch.name,
                                scaling,
                            );
                            let _ = module.add_channel(IoChannel::Ai(ai));
                        }
                        nebudcs_control::hal::ChannelKind::AO => {
                            let scaling = nebudcs_control::hal::AnalogScalingConfig::new_4_20ma(
                                ch.lrv,
                                ch.urv,
                                &ch.engineering_unit,
                            )
                            .unwrap_or_else(|_| {
                                nebudcs_control::hal::AnalogScalingConfig::new_4_20ma(
                                    0.0, 100.0, "%",
                                )
                                .unwrap()
                            });
                            let fallback = match ch.fallback_behavior.as_deref() {
                                Some("HoldLast") => {
                                    nebudcs_control::hal::OutputFallbackBehavior::HoldLast
                                }
                                Some("ForcedValue") => {
                                    nebudcs_control::hal::OutputFallbackBehavior::ForcedValue(0.0)
                                }
                                _ => nebudcs_control::hal::OutputFallbackBehavior::FailClosed,
                            };
                            let ao = nebudcs_control::hal::AnalogOutputChannel::new(
                                ch.channel_number,
                                addr,
                                &ch.name,
                                scaling,
                                fallback,
                            );
                            let _ = module.add_channel(IoChannel::Ao(ao));
                        }
                        nebudcs_control::hal::ChannelKind::DI => {
                            let di = nebudcs_control::hal::DigitalInputChannel::new(
                                ch.channel_number,
                                addr,
                                &ch.name,
                                false,
                            );
                            let _ = module.add_channel(IoChannel::Di(di));
                        }
                        nebudcs_control::hal::ChannelKind::DO => {
                            let fallback = match ch.fallback_behavior.as_deref() {
                                Some("HoldLast") => {
                                    nebudcs_control::hal::DigitalFallbackBehavior::HoldLast
                                }
                                _ => nebudcs_control::hal::DigitalFallbackBehavior::FailOpen,
                            };
                            let do_ch = nebudcs_control::hal::DigitalOutputChannel::new(
                                ch.channel_number,
                                addr,
                                &ch.name,
                                fallback,
                            );
                            let _ = module.add_channel(IoChannel::Do(do_ch));
                        }
                    }
                }
                let _ = chassis.add_module(module);
            }
            let _ = rack.add_chassis(chassis);
        }
        let _ = hal.add_rack(rack);
    }
    Ok(hal)
}
