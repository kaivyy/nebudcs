//! Virtual Controller Core Abstraction and Lifecycle State Machine.
//!
//! Provides an isolated, independent virtual controller engine managing tasks,
//! programs, I/O memory tables, execution diagnostics, CPU/memory profiles,
//! DCS control modules, redundancy replication, and watchdog timers.

use std::collections::BTreeMap;
use std::time::Instant;

use nebudcs_types::{ControllerId, TaskId};
use serde::{Deserialize, Serialize};

use crate::error::ControlError;
use crate::runtime::io_image::{IoImageTable, IoSignalValue};
use crate::runtime::module::DcsControlModule;
use crate::runtime::task::ControllerTask;

/// Controller lifecycle state conforming to IEC 61131 execution concepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ControllerLifecycle {
    /// Controller is stopped / powered off.
    Stopped,
    /// Controller is completely powered down (volatile memory erased).
    PowerOff,
    /// Controller is loading hardware and firmware images.
    Booting,
    /// Controller is initializing memory and state.
    Initializing,
    /// Controller is actively scheduling and executing tasks.
    Running,
    /// Controller execution is temporarily paused.
    Paused,
    /// Controller encountered a fatal fault and entered fail-safe state.
    Faulted,
    /// Controller is performing a graceful shutdown before power off.
    ShuttingDown,
}

impl ControllerLifecycle {
    /// Whether the controller is actively executing programs.
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running)
    }

    /// Whether the controller is powered on (in any operational or paused state).
    pub fn is_powered(&self) -> bool {
        !matches!(self, Self::PowerOff)
    }
}

impl std::fmt::Display for ControllerLifecycle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stopped => write!(f, "STOPPED"),
            Self::PowerOff => write!(f, "POWER_OFF"),
            Self::Booting => write!(f, "BOOTING"),
            Self::Initializing => write!(f, "INITIALIZING"),
            Self::Running => write!(f, "RUNNING"),
            Self::Paused => write!(f, "PAUSED"),
            Self::Faulted => write!(f, "FAULTED"),
            Self::ShuttingDown => write!(f, "SHUTTING_DOWN"),
        }
    }
}

/// Description of a runtime controller fault condition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControllerFault {
    /// Machine-readable fault code (e.g. "WATCHDOG_OVERRUN", "CPU_OVERLOAD", "MEMORY_EXHAUSTION").
    pub code: String,
    /// Human-readable fault description.
    pub message: String,
    /// Simulation timestamp when fault occurred.
    pub timestamp_sim: f64,
    /// Whether the fault is recoverable via a reset command.
    pub recoverable: bool,
}

/// Static configuration for a virtual controller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControllerConfig {
    pub name: String,
    pub description: String,
    /// Maximum allowable total scan cycle time in seconds before watchdog trips.
    pub watchdog_timeout_seconds: f64,
    pub enabled: bool,
}

impl Default for ControllerConfig {
    fn default() -> Self {
        Self {
            name: "VCPU-01".to_string(),
            description: "Default Virtual Controller".to_string(),
            watchdog_timeout_seconds: 0.1, // 100 ms watchdog limit
            enabled: true,
        }
    }
}

/// Virtual CPU Hardware Profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CpuProfile {
    pub execution_budget_us: u64,
    pub max_scan_frequency_hz: f64,
    pub utilization_threshold_pct: f64,
    pub architecture: String,
}

impl Default for CpuProfile {
    fn default() -> Self {
        Self {
            execution_budget_us: 50_000, // 50 ms budget
            max_scan_frequency_hz: 50.0,
            utilization_threshold_pct: 80.0,
            architecture: "VendorNeutral-VirtualVCPU-64".to_string(),
        }
    }
}

/// Virtual Memory Profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryProfile {
    pub program_memory_bytes: usize,
    pub data_memory_bytes: usize,
    pub retentive_memory_bytes: usize,
    pub program_memory_used_bytes: usize,
    pub data_memory_used_bytes: usize,
    pub retentive_memory_used_bytes: usize,
}

impl Default for MemoryProfile {
    fn default() -> Self {
        Self {
            program_memory_bytes: 1024 * 1024 * 16,  // 16 MB
            data_memory_bytes: 1024 * 1024 * 32,     // 32 MB
            retentive_memory_bytes: 1024 * 1024 * 4, // 4 MB
            program_memory_used_bytes: 0,
            data_memory_used_bytes: 0,
            retentive_memory_used_bytes: 0,
        }
    }
}

/// Virtual Controller Firmware Profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FirmwareProfile {
    pub version: String,
    pub build: String,
    pub iec_standard: String,
}

impl Default for FirmwareProfile {
    fn default() -> Self {
        Self {
            version: "v8.1.0".to_string(),
            build: "2026.09".to_string(),
            iec_standard: "IEC 61131-3:2013 Ed.3".to_string(),
        }
    }
}

/// Redundancy role in high-availability architectures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RedundancyRole {
    Standalone,
    Primary,
    Standby,
}

/// Dynamic operational status of redundant synchronization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RedundancyStatus {
    Isolated,
    Synchronizing,
    Synchronized,
    Failed,
    Switchover,
}

/// Redundancy configuration and state for high-availability controllers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedundancyState {
    pub role: RedundancyRole,
    pub status: RedundancyStatus,
    pub partner_id: Option<String>,
    pub sync_frequency_hz: f64,
    pub switchover_timeout_seconds: f64,
    pub last_sync_sim_time: f64,
    pub sync_cycle_count: u64,
}

impl Default for RedundancyState {
    fn default() -> Self {
        Self {
            role: RedundancyRole::Standalone,
            status: RedundancyStatus::Isolated,
            partner_id: None,
            sync_frequency_hz: 20.0,
            switchover_timeout_seconds: 0.1,
            last_sync_sim_time: 0.0,
            sync_cycle_count: 0,
        }
    }
}

/// Retentive memory storage that survives power cycling and reboots.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RetentiveStore {
    entries: BTreeMap<String, IoSignalValue>,
}

impl RetentiveStore {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    pub fn set(&mut self, key: impl Into<String>, value: IoSignalValue) {
        self.entries.insert(key.into(), value);
    }

    pub fn get(&self, key: &str) -> Option<&IoSignalValue> {
        self.entries.get(key)
    }

    pub fn entries(&self) -> &BTreeMap<String, IoSignalValue> {
        &self.entries
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// Dynamic runtime diagnostics and performance telemetry for a controller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControllerDiagnostics {
    pub scan_count: u64,
    pub task_execution_count: u64,
    pub overrun_count: u64,
    pub watchdog_trips: u64,
    pub last_scan_sim_time: f64,
    pub last_scan_duration_us: u64,
    pub max_scan_duration_us: u64,
    pub avg_scan_duration_us: f64,
    pub state: ControllerLifecycle,
}

impl Default for ControllerDiagnostics {
    fn default() -> Self {
        Self {
            scan_count: 0,
            task_execution_count: 0,
            overrun_count: 0,
            watchdog_trips: 0,
            last_scan_sim_time: 0.0,
            last_scan_duration_us: 0,
            max_scan_duration_us: 0,
            avg_scan_duration_us: 0.0,
            state: ControllerLifecycle::Stopped,
        }
    }
}

/// Virtual Industrial Controller managing tasks, I/O memory images, hardware profiles,
/// DCS control modules, and diagnostics.
#[derive(Debug, Clone)]
pub struct VirtualController {
    id: ControllerId,
    name: String,
    config: ControllerConfig,
    lifecycle: ControllerLifecycle,
    tasks: BTreeMap<TaskId, ControllerTask>,
    io_image: IoImageTable,
    diagnostics: ControllerDiagnostics,
    fault: Option<ControllerFault>,
    cpu_profile: CpuProfile,
    memory_profile: MemoryProfile,
    firmware_profile: FirmwareProfile,
    redundancy: RedundancyState,
    retentive_store: RetentiveStore,
    control_modules: BTreeMap<String, DcsControlModule>,
}

impl VirtualController {
    /// Creates a new virtual controller instance with the given ID and name.
    pub fn new(id: ControllerId, name: impl Into<String>, config: ControllerConfig) -> Self {
        let name_str = name.into();
        Self {
            id,
            name: name_str,
            config,
            lifecycle: ControllerLifecycle::Stopped,
            tasks: BTreeMap::new(),
            io_image: IoImageTable::new(),
            diagnostics: ControllerDiagnostics::default(),
            fault: None,
            cpu_profile: CpuProfile::default(),
            memory_profile: MemoryProfile::default(),
            firmware_profile: FirmwareProfile::default(),
            redundancy: RedundancyState::default(),
            retentive_store: RetentiveStore::new(),
            control_modules: BTreeMap::new(),
        }
    }

    /// Creates a new virtual controller instance with a generated ControllerId and given name.
    pub fn with_name(name: impl Into<String>, config: ControllerConfig) -> Self {
        Self::new(ControllerId::new(), name, config)
    }

    pub fn id(&self) -> ControllerId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn config(&self) -> &ControllerConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut ControllerConfig {
        &mut self.config
    }

    pub fn lifecycle(&self) -> ControllerLifecycle {
        self.lifecycle
    }

    pub fn is_running(&self) -> bool {
        self.lifecycle == ControllerLifecycle::Running
    }

    pub fn diagnostics(&self) -> &ControllerDiagnostics {
        &self.diagnostics
    }

    pub fn fault(&self) -> Option<&ControllerFault> {
        self.fault.as_ref()
    }

    pub fn io_image(&self) -> &IoImageTable {
        &self.io_image
    }

    pub fn io_image_mut(&mut self) -> &mut IoImageTable {
        &mut self.io_image
    }

    pub fn tasks(&self) -> &BTreeMap<TaskId, ControllerTask> {
        &self.tasks
    }

    pub fn tasks_mut(&mut self) -> &mut BTreeMap<TaskId, ControllerTask> {
        &mut self.tasks
    }

    pub fn cpu_profile(&self) -> &CpuProfile {
        &self.cpu_profile
    }

    pub fn cpu_profile_mut(&mut self) -> &mut CpuProfile {
        &mut self.cpu_profile
    }

    pub fn set_cpu_profile(&mut self, profile: CpuProfile) {
        self.cpu_profile = profile;
    }

    pub fn memory_profile(&self) -> &MemoryProfile {
        &self.memory_profile
    }

    pub fn memory_profile_mut(&mut self) -> &mut MemoryProfile {
        &mut self.memory_profile
    }

    pub fn set_memory_profile(&mut self, profile: MemoryProfile) {
        self.memory_profile = profile;
    }

    pub fn firmware_profile(&self) -> &FirmwareProfile {
        &self.firmware_profile
    }

    pub fn firmware_profile_mut(&mut self) -> &mut FirmwareProfile {
        &mut self.firmware_profile
    }

    pub fn set_firmware_profile(&mut self, profile: FirmwareProfile) {
        self.firmware_profile = profile;
    }

    pub fn redundancy(&self) -> &RedundancyState {
        &self.redundancy
    }

    pub fn redundancy_mut(&mut self) -> &mut RedundancyState {
        &mut self.redundancy
    }

    pub fn set_redundancy(&mut self, state: RedundancyState) {
        self.redundancy = state;
    }

    pub fn retentive_store(&self) -> &RetentiveStore {
        &self.retentive_store
    }

    pub fn retentive_store_mut(&mut self) -> &mut RetentiveStore {
        &mut self.retentive_store
    }

    pub fn add_control_module(&mut self, module: DcsControlModule) {
        self.control_modules.insert(module.id().to_string(), module);
    }

    pub fn remove_control_module(&mut self, id: &str) -> Option<DcsControlModule> {
        self.control_modules.remove(id)
    }

    pub fn control_module(&self, id: &str) -> Option<&DcsControlModule> {
        self.control_modules.get(id)
    }

    pub fn control_module_mut(&mut self, id: &str) -> Option<&mut DcsControlModule> {
        self.control_modules.get_mut(id)
    }

    pub fn control_modules(&self) -> &BTreeMap<String, DcsControlModule> {
        &self.control_modules
    }

    pub fn control_modules_mut(&mut self) -> &mut BTreeMap<String, DcsControlModule> {
        &mut self.control_modules
    }

    /// Adds a task to the controller.
    pub fn add_task(&mut self, task: ControllerTask) {
        self.tasks.insert(task.id(), task);
    }

    /// Removes a task from the controller by ID.
    pub fn remove_task(&mut self, id: TaskId) -> Option<ControllerTask> {
        self.tasks.remove(&id)
    }

    /// Retrieves an immutable reference to a task by ID.
    pub fn get_task(&self, id: TaskId) -> Option<&ControllerTask> {
        self.tasks.get(&id)
    }

    /// Retrieves a mutable reference to a task by ID.
    pub fn get_task_mut(&mut self, id: TaskId) -> Option<&mut ControllerTask> {
        self.tasks.get_mut(&id)
    }

    /// Alias for get_task.
    pub fn task(&self, id: TaskId) -> Option<&ControllerTask> {
        self.get_task(id)
    }

    /// Alias for get_task_mut.
    pub fn task_mut(&mut self, id: TaskId) -> Option<&mut ControllerTask> {
        self.get_task_mut(id)
    }

    // -----------------------------------------------------------------------
    // Lifecycle Transitions
    // -----------------------------------------------------------------------

    /// Transition directly to a target lifecycle state with validation.
    pub fn transition_to(&mut self, target: ControllerLifecycle) -> Result<(), ControlError> {
        match target {
            ControllerLifecycle::Stopped => self.stop(),
            ControllerLifecycle::PowerOff => self.power_off(),
            ControllerLifecycle::Booting => self.boot(),
            ControllerLifecycle::ShuttingDown => self.shutdown(),
            ControllerLifecycle::Initializing => {
                if self.lifecycle == ControllerLifecycle::Stopped
                    || self.lifecycle == ControllerLifecycle::Booting
                {
                    self.lifecycle = ControllerLifecycle::Initializing;
                    self.diagnostics.state = ControllerLifecycle::Initializing;
                    Ok(())
                } else {
                    Err(ControlError::InvalidLifecycleTransition {
                        from: self.lifecycle.to_string(),
                        to: "INITIALIZING".to_string(),
                        reason: "Can only transition to Initializing from Stopped or Booting state"
                            .to_string(),
                    })
                }
            }
            ControllerLifecycle::Running => self.start(),
            ControllerLifecycle::Paused => self.pause(),
            ControllerLifecycle::Faulted => {
                self.trip_fault(
                    "MANUAL_FAULT",
                    "Manual fault transition requested",
                    0.0,
                    true,
                );
                Ok(())
            }
        }
    }

    /// Starts controller execution.
    pub fn start(&mut self) -> Result<(), ControlError> {
        match self.lifecycle {
            ControllerLifecycle::Stopped
            | ControllerLifecycle::Initializing
            | ControllerLifecycle::Booting => {
                self.lifecycle = ControllerLifecycle::Running;
                self.diagnostics.state = ControllerLifecycle::Running;
                Ok(())
            }
            ControllerLifecycle::Running => Ok(()),
            ControllerLifecycle::Paused => {
                self.lifecycle = ControllerLifecycle::Running;
                self.diagnostics.state = ControllerLifecycle::Running;
                Ok(())
            }
            ControllerLifecycle::PowerOff => {
                // Must boot before running from power-off
                self.boot()?;
                self.start()
            }
            ControllerLifecycle::ShuttingDown => Err(ControlError::InvalidLifecycleTransition {
                from: "SHUTTING_DOWN".to_string(),
                to: "RUNNING".to_string(),
                reason: "Cannot start a controller that is currently shutting down".to_string(),
            }),
            ControllerLifecycle::Faulted => Err(ControlError::InvalidLifecycleTransition {
                from: "FAULTED".to_string(),
                to: "RUNNING".to_string(),
                reason: "Controller must be reset before starting from a faulted state".to_string(),
            }),
        }
    }

    /// Pauses controller execution.
    pub fn pause(&mut self) -> Result<(), ControlError> {
        match self.lifecycle {
            ControllerLifecycle::Running => {
                self.lifecycle = ControllerLifecycle::Paused;
                self.diagnostics.state = ControllerLifecycle::Paused;
                Ok(())
            }
            ControllerLifecycle::Paused => Ok(()),
            _ => Err(ControlError::InvalidLifecycleTransition {
                from: self.lifecycle.to_string(),
                to: "PAUSED".to_string(),
                reason: "Only a running controller can be paused".to_string(),
            }),
        }
    }

    /// Resumes controller execution from a paused state.
    pub fn resume(&mut self) -> Result<(), ControlError> {
        self.start()
    }

    /// Stops controller execution.
    pub fn stop(&mut self) -> Result<(), ControlError> {
        self.lifecycle = ControllerLifecycle::Stopped;
        self.diagnostics.state = ControllerLifecycle::Stopped;
        Ok(())
    }

    /// Powers off the virtual controller.
    pub fn power_off(&mut self) -> Result<(), ControlError> {
        self.lifecycle = ControllerLifecycle::PowerOff;
        self.diagnostics.state = ControllerLifecycle::PowerOff;
        // Volatile image tables are erased upon total power loss; retentive store is preserved
        self.io_image.clear_inputs();
        self.io_image.clear_outputs();
        for task in self.tasks.values_mut() {
            task.reset();
        }
        for module in self.control_modules.values_mut() {
            module.reset();
        }
        Ok(())
    }

    /// Initiates a hardware boot sequence.
    pub fn boot(&mut self) -> Result<(), ControlError> {
        match self.lifecycle {
            ControllerLifecycle::PowerOff
            | ControllerLifecycle::Stopped
            | ControllerLifecycle::Faulted => {
                self.lifecycle = ControllerLifecycle::Booting;
                self.diagnostics.state = ControllerLifecycle::Booting;
                // Transition to Initializing upon successful firmware check
                self.lifecycle = ControllerLifecycle::Initializing;
                self.diagnostics.state = ControllerLifecycle::Initializing;
                Ok(())
            }
            _ => Err(ControlError::InvalidLifecycleTransition {
                from: self.lifecycle.to_string(),
                to: "BOOTING".to_string(),
                reason: "Controller can only boot from PowerOff, Stopped, or Faulted states"
                    .to_string(),
            }),
        }
    }

    /// Initiates graceful shutdown and powers off.
    pub fn shutdown(&mut self) -> Result<(), ControlError> {
        self.lifecycle = ControllerLifecycle::ShuttingDown;
        self.diagnostics.state = ControllerLifecycle::ShuttingDown;
        self.power_off()
    }

    /// Resets the controller, clearing faults and resetting task accumulators.
    pub fn reset(&mut self) -> Result<(), ControlError> {
        self.fault = None;
        self.lifecycle = ControllerLifecycle::Stopped;
        self.diagnostics = ControllerDiagnostics::default();
        for task in self.tasks.values_mut() {
            task.reset();
        }
        for module in self.control_modules.values_mut() {
            module.reset();
        }
        self.io_image.clear_inputs();
        self.io_image.clear_outputs();
        Ok(())
    }

    /// Explicitly trip the controller into a faulted state.
    pub fn trip_fault(
        &mut self,
        code: impl Into<String>,
        message: impl Into<String>,
        timestamp_sim: f64,
        recoverable: bool,
    ) {
        let code_str = code.into();
        let msg_str = message.into();
        self.fault = Some(ControllerFault {
            code: code_str,
            message: msg_str,
            timestamp_sim,
            recoverable,
        });
        self.lifecycle = ControllerLifecycle::Faulted;
        self.diagnostics.state = ControllerLifecycle::Faulted;
    }

    // -----------------------------------------------------------------------
    // Fault Injection Model
    // -----------------------------------------------------------------------

    pub fn inject_fault(&mut self, fault: ControllerFault) {
        self.trip_fault(
            fault.code,
            fault.message,
            fault.timestamp_sim,
            fault.recoverable,
        );
    }

    pub fn inject_watchdog_timeout(&mut self, sim_time: f64) {
        self.trip_fault(
            "WATCHDOG_TIMEOUT",
            format!(
                "Hardware scan watchdog timeout (threshold: {:.3}s)",
                self.config.watchdog_timeout_seconds
            ),
            sim_time,
            true,
        );
        self.diagnostics.watchdog_trips += 1;
    }

    pub fn inject_task_overrun(&mut self, task_id: TaskId, sim_time: f64) {
        self.trip_fault(
            "TASK_OVERRUN",
            format!("Task '{task_id}' exceeded deterministic execution budget"),
            sim_time,
            true,
        );
        self.diagnostics.overrun_count += 1;
    }

    pub fn inject_cpu_overload(&mut self, sim_time: f64) {
        self.trip_fault(
            "CPU_OVERLOAD",
            format!(
                "Virtual CPU utilization exceeded limit: {:.1}%",
                self.cpu_profile.utilization_threshold_pct
            ),
            sim_time,
            true,
        );
    }

    pub fn inject_memory_exhaustion(&mut self, sim_time: f64) {
        self.trip_fault(
            "MEMORY_EXHAUSTION",
            format!(
                "Memory allocation exceeded profile capacity ({} bytes)",
                self.memory_profile.data_memory_bytes
            ),
            sim_time,
            false, // Non-recoverable without restart
        );
    }

    pub fn inject_power_loss(&mut self, _sim_time: f64) {
        let _ = self.power_off();
    }

    pub fn clear_fault(&mut self) -> Result<(), ControlError> {
        if let Some(ref f) = self.fault {
            if !f.recoverable {
                return Err(ControlError::InvalidParameter(
                    "Cannot clear non-recoverable fault; controller reboot required".to_string(),
                ));
            }
        }
        self.reset()
    }

    // -----------------------------------------------------------------------
    // Redundancy Replication
    // -----------------------------------------------------------------------

    /// Replicates state from this Primary controller to a designated Standby controller.
    pub fn replicate_to_standby(
        &self,
        standby: &mut VirtualController,
    ) -> Result<(), ControlError> {
        if standby.redundancy.role != RedundancyRole::Standby {
            return Err(ControlError::InvalidParameter(
                "Target controller is not configured as Standby".to_string(),
            ));
        }

        // 1. Replicate Retentive Storage
        standby.retentive_store = self.retentive_store.clone();

        // 2. Replicate I/O Image Table (%I and %Q)
        standby.io_image = self.io_image.clone();

        // 3. Replicate DCS Control Modules state
        for (id, module) in &self.control_modules {
            if let Some(st_mod) = standby.control_modules.get_mut(id) {
                st_mod.set_sp(module.sp());
                let _ = st_mod.set_mode(module.mode());
                st_mod.set_manual_out(module.out_manual());
                let status = module.status();
                st_mod.set_tracking(status.tracking_active, status.out);
            }
        }

        // 4. Update Standby sync telemetry
        standby.redundancy.status = RedundancyStatus::Synchronized;
        standby.redundancy.last_sync_sim_time = self.diagnostics.last_scan_sim_time;
        standby.redundancy.sync_cycle_count += 1;

        Ok(())
    }

    /// Triggers an immediate role switchover.
    pub fn trigger_switchover(&mut self, new_role: RedundancyRole) {
        self.redundancy.role = new_role;
        self.redundancy.status = RedundancyStatus::Switchover;
        if new_role == RedundancyRole::Primary {
            let _ = self.start();
            self.redundancy.status = RedundancyStatus::Synchronized;
        } else if new_role == RedundancyRole::Standby {
            self.redundancy.status = RedundancyStatus::Synchronized;
        }
    }

    // -----------------------------------------------------------------------
    // Execution Scan Cycle
    // -----------------------------------------------------------------------

    /// Executes one scan cycle step for the controller.
    pub fn step(&mut self, dt: f64, sim_time: f64) -> Result<(), ControlError> {
        if self.lifecycle == ControllerLifecycle::Faulted {
            let (code, msg) = self
                .fault
                .as_ref()
                .map(|f| (f.code.clone(), f.message.clone()))
                .unwrap_or_else(|| ("UNKNOWN_FAULT".into(), "Controller is faulted".into()));
            return Err(ControlError::ControllerFaulted {
                controller_id: self.name.clone(),
                code,
                detail: msg,
            });
        }

        if self.lifecycle != ControllerLifecycle::Running {
            return Ok(());
        }

        let scan_start = Instant::now();

        // 1. Execute DCS Control Modules
        for module in self.control_modules.values_mut() {
            if let Err(e) = module.step(dt, sim_time, &mut self.io_image) {
                self.trip_fault("CONTROL_MODULE_ERROR", e.to_string(), sim_time, true);
                return Err(e);
            }
        }

        // 2. Identify all tasks due in this simulation tick
        let mut due_task_ids: Vec<(u8, TaskId)> = Vec::new();
        let mut not_due_task_ids: Vec<TaskId> = Vec::new();

        for (task_id, task) in &self.tasks {
            if task.is_due(dt) {
                // Priority ascending: 1 is highest priority
                due_task_ids.push((task.priority(), *task_id));
            } else {
                not_due_task_ids.push(*task_id);
            }
        }

        // 3. Accumulate dt for tasks not due
        for task_id in not_due_task_ids {
            if let Some(task) = self.tasks.get_mut(&task_id) {
                task.accumulate(dt);
            }
        }

        // 4. Sort due tasks deterministically: by priority ascending, then TaskId
        due_task_ids.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

        // 5. Execute scheduled tasks in priority order
        let mut _total_task_dur_us: u64 = 0;
        let mut tasks_executed: u64 = 0;

        for (_priority, task_id) in due_task_ids {
            if let Some(task) = self.tasks.get_mut(&task_id) {
                match task.execute(dt, sim_time, &mut self.io_image) {
                    Ok(dur_us) => {
                        _total_task_dur_us += dur_us;
                        tasks_executed += 1;
                    }
                    Err(e) => {
                        self.trip_fault("TASK_EXECUTION_ERROR", e.to_string(), sim_time, true);
                        return Err(e);
                    }
                }
            }
        }

        let scan_dur = scan_start.elapsed();
        let scan_dur_us = scan_dur.as_micros() as u64;

        // 6. Watchdog and CPU budget monitoring
        let watchdog_limit_us = (self.config.watchdog_timeout_seconds * 1_000_000.0) as u64;
        if scan_dur_us > watchdog_limit_us && watchdog_limit_us > 0 {
            self.diagnostics.watchdog_trips += 1;
            self.diagnostics.overrun_count += 1;
        }

        if scan_dur_us > self.cpu_profile.execution_budget_us
            && self.cpu_profile.execution_budget_us > 0
        {
            self.diagnostics.overrun_count += 1;
        }

        // 7. Update diagnostics
        self.diagnostics.scan_count += 1;
        self.diagnostics.task_execution_count += tasks_executed;
        self.diagnostics.last_scan_sim_time = sim_time;
        self.diagnostics.last_scan_duration_us = scan_dur_us;
        self.diagnostics.max_scan_duration_us =
            self.diagnostics.max_scan_duration_us.max(scan_dur_us);

        // Exponential moving average for average scan duration
        let alpha = 0.05;
        self.diagnostics.avg_scan_duration_us = if self.diagnostics.scan_count == 1 {
            scan_dur_us as f64
        } else {
            (alpha * scan_dur_us as f64) + ((1.0 - alpha) * self.diagnostics.avg_scan_duration_us)
        };

        Ok(())
    }
}
