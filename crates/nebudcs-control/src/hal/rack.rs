//! I/O Rack container representing a physical cabinet or remote I/O drop.

use nebudcs_types::RackId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::hal::chassis::IoChassis;
use crate::hal::error::HalError;

/// Representation of an I/O rack supplying power and housing chassis units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IoRack {
    pub id: RackId,
    /// 1-based rack index within the I/O system.
    pub rack_number: u8,
    pub name: String,
    /// Main power supply operational health.
    pub power_ok: bool,
    /// Chassis units installed in this rack.
    pub chassis: BTreeMap<u8, IoChassis>,
}

impl IoRack {
    /// Creates a new I/O rack.
    pub fn new(rack_number: u8, name: impl Into<String>) -> Self {
        Self {
            id: RackId::new(),
            rack_number,
            name: name.into(),
            power_ok: true,
            chassis: BTreeMap::new(),
        }
    }

    /// Adds a chassis unit to this rack.
    pub fn add_chassis(&mut self, chassis: IoChassis) -> Result<(), HalError> {
        let num = chassis.chassis_number;
        if self.chassis.contains_key(&num) {
            return Err(HalError::DuplicateAddress {
                address: format!("Rack {} Chassis {}", self.rack_number, num),
            });
        }
        self.chassis.insert(num, chassis);
        Ok(())
    }

    /// Accesses a chassis unit by its 1-based index.
    pub fn chassis(&self, num: u8) -> Option<&IoChassis> {
        self.chassis.get(&num)
    }

    /// Accesses a mutable chassis unit by its 1-based index.
    pub fn chassis_mut(&mut self, num: u8) -> Option<&mut IoChassis> {
        self.chassis.get_mut(&num)
    }
}
