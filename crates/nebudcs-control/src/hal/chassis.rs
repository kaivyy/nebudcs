//! I/O Chassis backplane container holding module slots.

use nebudcs_types::ChassisId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::hal::error::HalError;
use crate::hal::module::IoModule;

/// Representation of an I/O chassis housing card slots on a high-speed backplane.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IoChassis {
    pub id: ChassisId,
    /// 1-based chassis index within the parent rack.
    pub chassis_number: u8,
    pub name: String,
    /// Backplane communication bus operational health.
    pub bus_ok: bool,
    /// Slot-indexed modules.
    pub modules: BTreeMap<u8, IoModule>,
}

impl IoChassis {
    /// Creates a new I/O chassis.
    pub fn new(chassis_number: u8, name: impl Into<String>) -> Self {
        Self {
            id: ChassisId::new(),
            chassis_number,
            name: name.into(),
            bus_ok: true,
            modules: BTreeMap::new(),
        }
    }

    /// Mounts an I/O module into a chassis slot.
    pub fn add_module(&mut self, module: IoModule) -> Result<(), HalError> {
        let slot = module.slot;
        if self.modules.contains_key(&slot) {
            return Err(HalError::ModuleSlotOccupied {
                chassis: self.chassis_number,
                slot,
            });
        }
        self.modules.insert(slot, module);
        Ok(())
    }

    /// Accesses a module by its slot index.
    pub fn module(&self, slot: u8) -> Option<&IoModule> {
        self.modules.get(&slot)
    }

    /// Accesses a mutable module by its slot index.
    pub fn module_mut(&mut self, slot: u8) -> Option<&mut IoModule> {
        self.modules.get_mut(&slot)
    }
}
