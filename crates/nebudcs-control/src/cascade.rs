//! Cascade control loop coordination (Master/Primary and Slave/Secondary loops).

use serde::{Deserialize, Serialize};

use crate::pid::{PidConfig, PidController, PidMode};

/// Coordinated primary (master) and secondary (slave) cascade control pair.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CascadePair {
    master: PidController,
    slave: PidController,
}

impl CascadePair {
    /// Creates a new cascade pair from master and slave configurations.
    pub fn new(master_config: PidConfig, slave_config: PidConfig) -> Self {
        Self {
            master: PidController::new(master_config),
            slave: PidController::new(slave_config),
        }
    }

    /// Read-only reference to master controller.
    pub fn master(&self) -> &PidController {
        &self.master
    }

    /// Mutable reference to master controller.
    pub fn master_mut(&mut self) -> &mut PidController {
        &mut self.master
    }

    /// Read-only reference to slave controller.
    pub fn slave(&self) -> &PidController {
        &self.slave
    }

    /// Mutable reference to slave controller.
    pub fn slave_mut(&mut self) -> &mut PidController {
        &mut self.slave
    }

    /// Executes one coordinated cascade step.
    ///
    /// - If slave is in `Cascade` or `RemoteCascade`:
    ///   - Master steps with `master_pv`.
    ///   - Master output is passed as slave's cascade setpoint.
    ///   - Slave steps with `slave_pv`.
    /// - If slave is in `Manual` or `Auto`:
    ///   - Master enters tracking mode tracking slave's active setpoint.
    ///   - Master steps with `master_pv`.
    ///   - Slave steps with `slave_pv` using its local SP or manual output.
    pub fn step(&mut self, dt: f64, master_pv: f64, slave_pv: f64) -> f64 {
        if self.slave.mode() == PidMode::Cascade || self.slave.mode() == PidMode::RemoteCascade {
            if self.master.mode() != PidMode::Auto {
                self.master.set_mode(PidMode::Auto);
            }
            self.master.exit_tracking();
            let master_out = self.master.step(dt, master_pv);
            self.slave.set_cascade_sp(master_out);
            self.slave.step(dt, slave_pv)
        } else {
            // Slave is local (Auto or Manual)
            // Master continuously tracks slave setpoint so when slave is transferred
            // to cascade, the master output matches the current slave setpoint bumplessly!
            self.master.enter_tracking(self.slave.sp());
            let _ = self.master.step(dt, master_pv);
            self.slave.step(dt, slave_pv)
        }
    }
}
