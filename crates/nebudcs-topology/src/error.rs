//! Strongly typed topology validation errors.

use thiserror::Error;

use nebudcs_model::equipment::{Cardinality, Phase, PortDirection, PortDomain};
use nebudcs_types::{EquipmentId, NebuError, PipeSegmentId, PortId, SignalConnectionId, StreamId};

/// Errors encountered when validating or mutating process, piping, or signal topology.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum TopologyError {
    #[error("equipment with id {equipment_id} not found in topology")]
    EquipmentNotFound { equipment_id: EquipmentId },

    #[error("equipment definition for equipment {equipment_id} not registered")]
    DefinitionNotFound { equipment_id: EquipmentId },

    #[error("port with id {port_id} not found on equipment {equipment_id}")]
    PortNotFound {
        equipment_id: EquipmentId,
        port_id: PortId,
    },

    #[error("stream with id {stream_id} not found in topology")]
    StreamNotFound { stream_id: StreamId },

    #[error("pipe segment with id {pipe_id} not found in topology")]
    PipeNotFound { pipe_id: PipeSegmentId },

    #[error("signal connection with id {connection_id} not found in topology")]
    SignalConnectionNotFound { connection_id: SignalConnectionId },

    #[error("stream {stream_id} already exists in topology")]
    DuplicateStream { stream_id: StreamId },

    #[error("connection between port {source_port} on {source_eq} and port {sink_port} on {sink_eq} already exists")]
    DuplicateConnection {
        source_eq: EquipmentId,
        source_port: PortId,
        sink_eq: EquipmentId,
        sink_port: PortId,
    },

    #[error("incompatible port domains: source is {source_domain:?}, sink is {sink_domain:?}")]
    DomainMismatch {
        source_domain: PortDomain,
        sink_domain: PortDomain,
    },

    #[error(
        "incompatible port directions: cannot connect source {source_dir:?} to sink {sink_dir:?}"
    )]
    DirectionMismatch {
        source_dir: PortDirection,
        sink_dir: PortDirection,
    },

    #[error(
        "incompatible phases: source supports {source_phases:?}, sink supports {sink_phases:?}"
    )]
    PhaseMismatch {
        source_phases: Vec<Phase>,
        sink_phases: Vec<Phase>,
    },

    #[error(
        "cardinality violation on port {port_id} of equipment {equipment_id}: limit is {cardinality:?}, current connection count is {current_count}"
    )]
    CardinalityExceeded {
        equipment_id: EquipmentId,
        port_id: PortId,
        cardinality: Cardinality,
        current_count: usize,
    },

    #[error("self-connection invalid: port cannot be connected to itself")]
    SelfPortConnection {
        equipment_id: EquipmentId,
        port_id: PortId,
    },

    #[error("stream type {stream_type:?} does not match port domain {port_domain:?}")]
    StreamTypeDomainMismatch {
        stream_type: String,
        port_domain: PortDomain,
    },

    #[error("signal port mismatch: {reason}")]
    SignalPortMismatch { reason: String },
}

impl From<TopologyError> for NebuError {
    fn from(err: TopologyError) -> Self {
        NebuError::InvalidConnection {
            detail: err.to_string(),
        }
    }
}
