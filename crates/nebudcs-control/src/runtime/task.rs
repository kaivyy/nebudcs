//! Controller Task Scheduling and Execution Engine.
//!
//! Provides deterministic task scheduling with support for periodic execution,
//! priority ordering, scan counters, and watchdog / overrun monitoring.

use std::time::Instant;

use nebudcs_types::TaskId;
use serde::{Deserialize, Serialize};

use crate::error::ControlError;
use crate::runtime::io_image::IoImageTable;
use crate::runtime::program::ControllerProgram;

/// Type of controller execution task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskType {
    /// Periodic task scheduled at a fixed interval (in seconds).
    Periodic {
        /// Recurrence interval in seconds (e.g. 0.01 for 10ms, 0.05 for 50ms, 0.1 for 100ms).
        period_seconds: f64,
    },
    /// Event-driven task triggered on boolean transition of a monitored tag.
    Event {
        /// Monitored tag trigger name.
        trigger_tag: String,
    },
    /// Continuous task running during spare scan cycles.
    Continuous,
}

/// A scheduled controller execution task holding an ordered sequence of programs.
#[derive(Debug)]
pub struct ControllerTask {
    id: TaskId,
    name: String,
    task_type: TaskType,
    /// Task priority (1 = highest urgency, 15 = lowest).
    priority: u8,
    /// Whether this task is active and scheduled for execution.
    enabled: bool,
    /// Watchdog execution time limit in seconds (if exceeded, flags overrun).
    watchdog_limit_seconds: Option<f64>,
    /// Accumulated simulation time since last task scan.
    accumulated_dt: f64,
    /// Total number of times this task has executed.
    execution_count: u64,
    /// Total number of times execution duration exceeded configured watchdog/period limit.
    overrun_count: u64,
    /// Simulation time of last successful execution.
    last_execution_sim_time: f64,
    /// Wall-clock duration of last execution in microseconds (for diagnostics only).
    last_duration_us: u64,
    /// Peak execution duration in microseconds.
    max_duration_us: u64,
    /// Ordered programs/routines executed by this task.
    programs: Vec<Box<dyn ControllerProgram>>,
}

impl Clone for ControllerTask {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            name: self.name.clone(),
            task_type: self.task_type.clone(),
            priority: self.priority,
            enabled: self.enabled,
            watchdog_limit_seconds: self.watchdog_limit_seconds,
            accumulated_dt: self.accumulated_dt,
            execution_count: self.execution_count,
            overrun_count: self.overrun_count,
            last_execution_sim_time: self.last_execution_sim_time,
            last_duration_us: self.last_duration_us,
            max_duration_us: self.max_duration_us,
            programs: self.programs.iter().map(|p| p.clone_box()).collect(),
        }
    }
}

impl ControllerTask {
    /// Creates a new periodic controller task.
    pub fn new_periodic(
        id: TaskId,
        name: impl Into<String>,
        period_seconds: f64,
        priority: u8,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            task_type: TaskType::Periodic { period_seconds },
            priority: priority.clamp(1, 15),
            enabled: true,
            watchdog_limit_seconds: Some(period_seconds * 0.8),
            accumulated_dt: 0.0,
            execution_count: 0,
            overrun_count: 0,
            last_execution_sim_time: 0.0,
            last_duration_us: 0,
            max_duration_us: 0,
            programs: Vec::new(),
        }
    }

    /// Creates a new event-triggered controller task.
    pub fn new_event(
        id: TaskId,
        name: impl Into<String>,
        trigger_tag: impl Into<String>,
        priority: u8,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            task_type: TaskType::Event {
                trigger_tag: trigger_tag.into(),
            },
            priority: priority.clamp(1, 15),
            enabled: true,
            watchdog_limit_seconds: None,
            accumulated_dt: 0.0,
            execution_count: 0,
            overrun_count: 0,
            last_execution_sim_time: 0.0,
            last_duration_us: 0,
            max_duration_us: 0,
            programs: Vec::new(),
        }
    }

    /// Creates a new continuous controller task.
    pub fn new_continuous(id: TaskId, name: impl Into<String>, priority: u8) -> Self {
        Self {
            id,
            name: name.into(),
            task_type: TaskType::Continuous,
            priority: priority.clamp(1, 15),
            enabled: true,
            watchdog_limit_seconds: None,
            accumulated_dt: 0.0,
            execution_count: 0,
            overrun_count: 0,
            last_execution_sim_time: 0.0,
            last_duration_us: 0,
            max_duration_us: 0,
            programs: Vec::new(),
        }
    }

    /// Sets an explicit watchdog execution limit for this task.
    pub fn with_watchdog_limit(mut self, limit_seconds: f64) -> Self {
        self.watchdog_limit_seconds = Some(limit_seconds);
        self
    }

    pub fn id(&self) -> TaskId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn task_type(&self) -> &TaskType {
        &self.task_type
    }

    pub fn priority(&self) -> u8 {
        self.priority
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn execution_count(&self) -> u64 {
        self.execution_count
    }

    pub fn overrun_count(&self) -> u64 {
        self.overrun_count
    }

    pub fn last_execution_sim_time(&self) -> f64 {
        self.last_execution_sim_time
    }

    pub fn last_duration_us(&self) -> u64 {
        self.last_duration_us
    }

    pub fn max_duration_us(&self) -> u64 {
        self.max_duration_us
    }

    pub fn watchdog_limit_seconds(&self) -> Option<f64> {
        self.watchdog_limit_seconds
    }

    /// Adds a program to this task's execution sequence.
    pub fn add_program(&mut self, program: Box<dyn ControllerProgram>) {
        self.programs.push(program);
    }

    /// Access immutable list of programs.
    pub fn programs(&self) -> &[Box<dyn ControllerProgram>] {
        &self.programs
    }

    /// Access mutable list of programs.
    pub fn programs_mut(&mut self) -> &mut [Box<dyn ControllerProgram>] {
        &mut self.programs
    }

    /// Evaluates whether the task is due for execution in the current simulation tick.
    pub fn is_due(&self, dt: f64) -> bool {
        if !self.enabled {
            return false;
        }
        match self.task_type {
            TaskType::Periodic { period_seconds } => {
                // Task is due if accumulated simulation time + current dt reaches period
                (self.accumulated_dt + dt) >= (period_seconds - 1e-9)
            }
            TaskType::Event { .. } => false, // Event triggers evaluated explicitly via trigger
            TaskType::Continuous => true,
        }
    }

    /// Advances the internal simulation clock accumulator without executing.
    pub fn accumulate(&mut self, dt: f64) {
        self.accumulated_dt += dt;
    }

    /// Executes the task programs, consuming accumulated simulation time and updating statistics.
    pub fn execute(
        &mut self,
        dt: f64,
        sim_time: f64,
        io_image: &mut IoImageTable,
    ) -> Result<u64, ControlError> {
        let start_time = Instant::now();

        // Determine the effective execution dt for control algorithms
        let effective_dt = match self.task_type {
            TaskType::Periodic { period_seconds } => {
                let actual_elapsed = self.accumulated_dt + dt;
                // Reset accumulator modulo period or to 0.0
                self.accumulated_dt = (actual_elapsed - period_seconds).max(0.0);
                if actual_elapsed > 0.0 {
                    actual_elapsed
                } else {
                    period_seconds
                }
            }
            _ => {
                self.accumulated_dt = 0.0;
                dt
            }
        };

        for program in &mut self.programs {
            program.execute(effective_dt, sim_time, io_image)?;
        }

        let elapsed_dur = start_time.elapsed();
        let elapsed_us = elapsed_dur.as_micros() as u64;

        self.last_duration_us = elapsed_us;
        self.max_duration_us = self.max_duration_us.max(elapsed_us);
        self.last_execution_sim_time = sim_time;
        self.execution_count += 1;

        // Check if execution duration exceeded configured watchdog limit (in seconds)
        if let Some(limit_sec) = self.watchdog_limit_seconds {
            let limit_us = (limit_sec * 1_000_000.0) as u64;
            if elapsed_us > limit_us {
                self.overrun_count += 1;
            }
        }

        Ok(elapsed_us)
    }

    /// Resets execution counters, timers, and programs.
    pub fn reset(&mut self) {
        self.accumulated_dt = 0.0;
        self.execution_count = 0;
        self.overrun_count = 0;
        self.last_execution_sim_time = 0.0;
        self.last_duration_us = 0;
        self.max_duration_us = 0;
        for program in &mut self.programs {
            program.reset();
        }
    }
}
