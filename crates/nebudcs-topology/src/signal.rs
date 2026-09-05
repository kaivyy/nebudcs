//! Instrument and signal topology representation.
//!
//! Models signal connections between instruments (transmitters, sensors),
//! controllers (PID, logic, safety systems), and final control elements (actuators, valves).
//!
//! Signal connections are strictly directed (`source` transmitter/controller to `destination` controller/actuator)
//! and typed by signal carrier (e.g., 4-20mA, digital discrete, bus protocol, pneumatic).

use serde::{Deserialize, Serialize};

use nebudcs_types::{EquipmentId, PortId, SignalConnectionId, TagId};

/// Signal carrier medium.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalMedium {
    /// 4-20 mA DC current loop.
    AnalogCurrent,
    /// 0-10 V or 1-5 V DC analog voltage.
    AnalogVoltage,
    /// 24 V DC discrete binary signal.
    DigitalDiscrete,
    /// Pneumatic signal (e.g. 3-15 psi / 0.2-1.0 bar to control valve positioner).
    Pneumatic,
    /// Digital communication fieldbus (Modbus, Profibus, Foundation Fieldbus).
    Fieldbus,
    /// Software logical signal (internal tag-to-controller link).
    SoftwareTag,
}

/// Endpoint of a signal connection.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SignalEndpoint {
    /// Port on an equipment instance (e.g. transmitter signal port, valve actuator signal port).
    Port {
        equipment_id: EquipmentId,
        port_id: PortId,
    },
    /// A DCS tag reference (e.g. PV input, MV output).
    Tag(TagId),
}

/// A signal connection between instruments, controllers, or actuators.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignalConnection {
    pub id: SignalConnectionId,
    pub name: String,
    /// Source of the signal (e.g. Transmitter output port or Controller output).
    pub source: SignalEndpoint,
    /// Destination of the signal (e.g. Controller PV input or Actuator command port).
    pub destination: SignalEndpoint,
    /// Physical/logical transmission medium.
    pub medium: SignalMedium,
    /// Description or loop identifier (e.g., "LIC-101 measurement loop").
    pub description: Option<String>,
}

impl SignalConnection {
    /// Create a new directed signal connection.
    pub fn new(
        name: impl Into<String>,
        source: SignalEndpoint,
        destination: SignalEndpoint,
        medium: SignalMedium,
    ) -> Self {
        Self {
            id: SignalConnectionId::new(),
            name: name.into(),
            source,
            destination,
            medium,
            description: None,
        }
    }
}
