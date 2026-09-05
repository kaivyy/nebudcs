//! Piping network topology representation.
//!
//! Physical piping connects equipment ports in a process plant.
//! A [`PipeSegment`] has its own identity, nominal diameter, design specifications,
//! and two distinct endpoints (`endpoint_a` and `endpoint_b`).
//!
//! Importantly, connection direction (`A` to `B`) is distinct from the future
//! actual process flow direction (`A` → `B` or `B` → `A`), allowing bidirectional
//! or reverse flow without modifying the topology identity.

use serde::{Deserialize, Serialize};

use nebudcs_model::stream::PortRef;
use nebudcs_types::{EquipmentId, PipeSegmentId, PortId, StreamId};

/// Endpoint specification of a pipe segment.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PipeEndpoint {
    pub equipment_id: EquipmentId,
    pub port_id: PortId,
}

impl From<PortRef> for PipeEndpoint {
    fn from(r: PortRef) -> Self {
        Self {
            equipment_id: r.equipment_id,
            port_id: r.port_id,
        }
    }
}

impl From<PipeEndpoint> for PortRef {
    fn from(p: PipeEndpoint) -> Self {
        Self {
            equipment_id: p.equipment_id,
            port_id: p.port_id,
        }
    }
}

/// A physical pipe segment in the plant topology.
///
/// This represents a discrete section of piping connecting two equipment ports
/// (or joining other pipe segments at tees/manifolds).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PipeSegment {
    pub id: PipeSegmentId,
    pub name: String,
    /// Connection endpoint A.
    pub endpoint_a: PipeEndpoint,
    /// Connection endpoint B.
    pub endpoint_b: PipeEndpoint,
    /// Associated stream identity if this pipe carries a defined stream.
    pub stream_id: Option<StreamId>,
    /// Nominal diameter in meters (canonical SI unit).
    pub nominal_diameter: Option<f64>,
    /// Pipe length in meters (canonical SI unit).
    pub length: Option<f64>,
    /// Pipe schedule or specification (e.g., "Sch 40", "Sch 80").
    pub specification: Option<String>,
}

impl PipeSegment {
    /// Create a new pipe segment connecting two endpoints.
    pub fn new(
        name: impl Into<String>,
        endpoint_a: PipeEndpoint,
        endpoint_b: PipeEndpoint,
    ) -> Self {
        Self {
            id: PipeSegmentId::new(),
            name: name.into(),
            endpoint_a,
            endpoint_b,
            stream_id: None,
            nominal_diameter: None,
            length: None,
            specification: None,
        }
    }

    /// Attach a stream to this pipe segment.
    pub fn with_stream(mut self, stream_id: StreamId) -> Self {
        self.stream_id = Some(stream_id);
        self
    }

    /// Check if this pipe connects to the specified port on equipment.
    pub fn connects_to(&self, eq_id: EquipmentId, port_id: PortId) -> bool {
        (self.endpoint_a.equipment_id == eq_id && self.endpoint_a.port_id == port_id)
            || (self.endpoint_b.equipment_id == eq_id && self.endpoint_b.port_id == port_id)
    }

    /// Given one endpoint, get the opposite endpoint of this pipe.
    pub fn opposite_endpoint(&self, eq_id: EquipmentId, port_id: PortId) -> Option<&PipeEndpoint> {
        if self.endpoint_a.equipment_id == eq_id && self.endpoint_a.port_id == port_id {
            Some(&self.endpoint_b)
        } else if self.endpoint_b.equipment_id == eq_id && self.endpoint_b.port_id == port_id {
            Some(&self.endpoint_a)
        } else {
            None
        }
    }
}
