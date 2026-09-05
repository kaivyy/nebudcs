//! Topology graph representing process, utility, piping, and signal connectivity.
//!
//! Maintains transactional validity via validate-then-mutate semantics.
//! All query operations return deterministic ordering.

use std::collections::{BTreeMap, BTreeSet};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use nebudcs_model::equipment::{EquipmentDefinition, PortDefinition};
use nebudcs_model::stream::StreamDefinition;
use nebudcs_types::{
    EquipmentDefinitionId, EquipmentId, PipeSegmentId, PortId, SignalConnectionId, StreamId,
};

use crate::error::TopologyError;
use crate::pipe::PipeSegment;
use crate::port::validate_port_compatibility;
use crate::scc::{find_strongly_connected_components, SccAnalysisResult};
use crate::signal::SignalConnection;

/// The central domain topology graph for a unit or plant.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TopologyGraph {
    /// Equipment definitions registered in this topology context.
    definitions: IndexMap<EquipmentDefinitionId, EquipmentDefinition>,
    /// Equipment instances in the topology and their definition reference.
    equipment: IndexMap<EquipmentId, EquipmentDefinitionId>,
    /// Streams connecting equipment ports (process and utilities).
    streams: IndexMap<StreamId, StreamDefinition>,
    /// Physical piping segments connecting equipment ports.
    pipes: IndexMap<PipeSegmentId, PipeSegment>,
    /// Instrument/control signal connections.
    signals: IndexMap<SignalConnectionId, SignalConnection>,

    /// Graph adjacencies for process and utility streams: source_eq -> set of sink_eq.
    #[serde(skip)]
    adjacency: BTreeMap<EquipmentId, BTreeSet<EquipmentId>>,
    /// Reverse adjacencies: sink_eq -> set of source_eq.
    #[serde(skip)]
    reverse_adjacency: BTreeMap<EquipmentId, BTreeSet<EquipmentId>>,
    /// Map of (EquipmentId, PortId) -> set of StreamIds attached.
    #[serde(skip)]
    port_streams: BTreeMap<(EquipmentId, PortId), BTreeSet<StreamId>>,
    /// Map of (EquipmentId, PortId) -> set of PipeSegmentIds attached.
    #[serde(skip)]
    port_pipes: BTreeMap<(EquipmentId, PortId), BTreeSet<PipeSegmentId>>,
}

impl TopologyGraph {
    /// Creates a new empty topology graph.
    pub fn new() -> Self {
        Self::default()
    }

    // -----------------------------------------------------------------------
    // Definition and Equipment Registration
    // -----------------------------------------------------------------------

    /// Registers an immutable equipment definition template.
    pub fn register_definition(&mut self, definition: EquipmentDefinition) {
        self.definitions.insert(definition.id, definition);
    }

    /// Looks up an equipment definition by its ID.
    pub fn get_definition(&self, def_id: EquipmentDefinitionId) -> Option<&EquipmentDefinition> {
        self.definitions.get(&def_id)
    }

    /// Adds an equipment instance referencing a registered definition.
    pub fn add_equipment(
        &mut self,
        equipment_id: EquipmentId,
        definition_id: EquipmentDefinitionId,
    ) -> Result<(), TopologyError> {
        if !self.definitions.contains_key(&definition_id) {
            return Err(TopologyError::DefinitionNotFound { equipment_id });
        }
        self.equipment.insert(equipment_id, definition_id);
        self.adjacency.entry(equipment_id).or_default();
        self.reverse_adjacency.entry(equipment_id).or_default();
        Ok(())
    }

    /// Returns true if an equipment instance exists in the graph.
    pub fn contains_equipment(&self, equipment_id: EquipmentId) -> bool {
        self.equipment.contains_key(&equipment_id)
    }

    /// Returns the number of equipment instances in the graph.
    pub fn equipment_count(&self) -> usize {
        self.equipment.len()
    }

    /// Returns all equipment IDs in deterministic order.
    pub fn equipment_ids(&self) -> Vec<EquipmentId> {
        let mut ids: Vec<EquipmentId> = self.equipment.keys().copied().collect();
        ids.sort_by_key(|a| a.to_string());
        ids
    }

    /// Looks up the PortDefinition for a specific equipment instance and port ID.
    pub fn get_port_definition(
        &self,
        equipment_id: EquipmentId,
        port_id: PortId,
    ) -> Result<&PortDefinition, TopologyError> {
        let def_id = self
            .equipment
            .get(&equipment_id)
            .ok_or(TopologyError::EquipmentNotFound { equipment_id })?;
        let definition = self
            .definitions
            .get(def_id)
            .ok_or(TopologyError::DefinitionNotFound { equipment_id })?;
        definition
            .ports
            .iter()
            .find(|p| p.id == port_id)
            .ok_or(TopologyError::PortNotFound {
                equipment_id,
                port_id,
            })
    }

    // -----------------------------------------------------------------------
    // Stream Connections (Process & Utilities)
    // -----------------------------------------------------------------------

    /// Validates and adds a stream connection to the topology graph.
    ///
    /// Adheres strictly to validate-then-mutate semantics: if any validation rule fails,
    /// the topology graph remains completely unmodified.
    pub fn add_stream(&mut self, stream: StreamDefinition) -> Result<(), TopologyError> {
        if self.streams.contains_key(&stream.id) {
            return Err(TopologyError::DuplicateStream {
                stream_id: stream.id,
            });
        }

        let source_eq = stream.source.equipment_id;
        let source_port_id = stream.source.port_id;
        let sink_eq = stream.sink.equipment_id;
        let sink_port_id = stream.sink.port_id;

        // Verify entities exist and fetch their definitions
        let source_port_def = self.get_port_definition(source_eq, source_port_id)?;
        let sink_port_def = self.get_port_definition(sink_eq, sink_port_id)?;

        let source_conn_count = self
            .port_streams
            .get(&(source_eq, source_port_id))
            .map_or(0, |s| s.len());
        let sink_conn_count = self
            .port_streams
            .get(&(sink_eq, sink_port_id))
            .map_or(0, |s| s.len());

        // Compatibility validation
        validate_port_compatibility(
            source_eq,
            source_port_def,
            source_conn_count,
            sink_eq,
            sink_port_def,
            sink_conn_count,
            Some(&stream.stream_type),
        )?;

        // Check for duplicate stream connection between exactly the same source and sink ports
        if let Some(existing_streams) = self.port_streams.get(&(source_eq, source_port_id)) {
            for &existing_id in existing_streams {
                if let Some(existing_stream) = self.streams.get(&existing_id) {
                    if existing_stream.sink.equipment_id == sink_eq
                        && existing_stream.sink.port_id == sink_port_id
                    {
                        return Err(TopologyError::DuplicateConnection {
                            source_eq,
                            source_port: source_port_id,
                            sink_eq,
                            sink_port: sink_port_id,
                        });
                    }
                }
            }
        }

        // --- All checks passed: perform mutations ---
        self.adjacency.entry(source_eq).or_default().insert(sink_eq);
        self.reverse_adjacency
            .entry(sink_eq)
            .or_default()
            .insert(source_eq);
        self.port_streams
            .entry((source_eq, source_port_id))
            .or_default()
            .insert(stream.id);
        self.port_streams
            .entry((sink_eq, sink_port_id))
            .or_default()
            .insert(stream.id);

        self.streams.insert(stream.id, stream);
        Ok(())
    }

    /// Retrieves a reference to a stream by ID.
    pub fn get_stream(&self, stream_id: StreamId) -> Option<&StreamDefinition> {
        self.streams.get(&stream_id)
    }

    /// Returns the number of streams in the topology.
    pub fn stream_count(&self) -> usize {
        self.streams.len()
    }

    /// Returns all streams connected to a specific equipment port.
    pub fn streams_at_port(
        &self,
        equipment_id: EquipmentId,
        port_id: PortId,
    ) -> Vec<&StreamDefinition> {
        let mut results = Vec::new();
        if let Some(stream_ids) = self.port_streams.get(&(equipment_id, port_id)) {
            for id in stream_ids {
                if let Some(s) = self.streams.get(id) {
                    results.push(s);
                }
            }
        }
        results.sort_by_key(|a| a.id.to_string());
        results
    }

    // -----------------------------------------------------------------------
    // Piping Topology
    // -----------------------------------------------------------------------

    /// Adds a physical pipe segment between two equipment ports.
    pub fn add_pipe(&mut self, pipe: PipeSegment) -> Result<(), TopologyError> {
        // Verify endpoints exist
        self.get_port_definition(pipe.endpoint_a.equipment_id, pipe.endpoint_a.port_id)?;
        self.get_port_definition(pipe.endpoint_b.equipment_id, pipe.endpoint_b.port_id)?;

        let a_key = (pipe.endpoint_a.equipment_id, pipe.endpoint_a.port_id);
        let b_key = (pipe.endpoint_b.equipment_id, pipe.endpoint_b.port_id);

        self.port_pipes.entry(a_key).or_default().insert(pipe.id);
        self.port_pipes.entry(b_key).or_default().insert(pipe.id);

        self.pipes.insert(pipe.id, pipe);
        Ok(())
    }

    /// Retrieves a pipe segment by ID.
    pub fn get_pipe(&self, pipe_id: PipeSegmentId) -> Option<&PipeSegment> {
        self.pipes.get(&pipe_id)
    }

    /// Returns all pipe segments connected to a given port.
    pub fn pipes_at_port(&self, equipment_id: EquipmentId, port_id: PortId) -> Vec<&PipeSegment> {
        let mut results = Vec::new();
        if let Some(pipe_ids) = self.port_pipes.get(&(equipment_id, port_id)) {
            for id in pipe_ids {
                if let Some(p) = self.pipes.get(id) {
                    results.push(p);
                }
            }
        }
        results.sort_by_key(|a| a.id.to_string());
        results
    }

    // -----------------------------------------------------------------------
    // Signal / Instrument Connections
    // -----------------------------------------------------------------------

    /// Adds an instrument/control signal connection to the topology.
    pub fn add_signal(&mut self, signal: SignalConnection) -> Result<(), TopologyError> {
        // If source is a port, verify it exists and is a signal port
        if let crate::signal::SignalEndpoint::Port {
            equipment_id,
            port_id,
        } = &signal.source
        {
            let port_def = self.get_port_definition(*equipment_id, *port_id)?;
            if port_def.domain != nebudcs_model::equipment::PortDomain::Signal {
                return Err(TopologyError::SignalPortMismatch {
                    reason: format!(
                        "signal connection source port {port_id} domain is {:?}, expected Signal",
                        port_def.domain
                    ),
                });
            }
        }

        // If destination is a port, verify it exists and is a signal port
        if let crate::signal::SignalEndpoint::Port {
            equipment_id,
            port_id,
        } = &signal.destination
        {
            let port_def = self.get_port_definition(*equipment_id, *port_id)?;
            if port_def.domain != nebudcs_model::equipment::PortDomain::Signal {
                return Err(TopologyError::SignalPortMismatch {
                    reason: format!(
                        "signal connection destination port {port_id} domain is {:?}, expected Signal",
                        port_def.domain
                    ),
                });
            }
        }

        self.signals.insert(signal.id, signal);
        Ok(())
    }

    /// Retrieves a signal connection by ID.
    pub fn get_signal(&self, signal_id: SignalConnectionId) -> Option<&SignalConnection> {
        self.signals.get(&signal_id)
    }

    /// Returns all signal connections in the topology.
    pub fn signals(&self) -> Vec<&SignalConnection> {
        let mut results: Vec<&SignalConnection> = self.signals.values().collect();
        results.sort_by_key(|a| a.id.to_string());
        results
    }

    // -----------------------------------------------------------------------
    // Graph Traversal Queries
    // -----------------------------------------------------------------------

    /// Returns immediate downstream equipment connected via outgoing streams.
    pub fn downstream_equipment(&self, equipment_id: EquipmentId) -> Vec<EquipmentId> {
        self.adjacency
            .get(&equipment_id)
            .map(|neighbors| neighbors.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Returns immediate upstream equipment connected via incoming streams.
    pub fn upstream_equipment(&self, equipment_id: EquipmentId) -> Vec<EquipmentId> {
        self.reverse_adjacency
            .get(&equipment_id)
            .map(|neighbors| neighbors.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Returns all neighboring equipment (both upstream and downstream).
    pub fn equipment_neighbors(&self, equipment_id: EquipmentId) -> Vec<EquipmentId> {
        let mut set = BTreeSet::new();
        if let Some(down) = self.adjacency.get(&equipment_id) {
            set.extend(down);
        }
        if let Some(up) = self.reverse_adjacency.get(&equipment_id) {
            set.extend(up);
        }
        set.into_iter().collect()
    }

    /// Returns the number of physical pipe segments in the topology.
    pub fn pipe_count(&self) -> usize {
        self.pipes.len()
    }

    /// Returns all pipe segments in deterministic ID order.
    pub fn pipes(&self) -> Vec<&PipeSegment> {
        let mut results: Vec<&PipeSegment> = self.pipes.values().collect();
        results.sort_by_key(|a| a.id.to_string());
        results
    }

    /// Returns all equipment connected to the given equipment via pipe segments.
    pub fn piping_neighbors(&self, equipment_id: EquipmentId) -> Vec<EquipmentId> {
        let mut neighbors = BTreeSet::new();
        for pipe in self.pipes.values() {
            if pipe.endpoint_a.equipment_id == equipment_id {
                neighbors.insert(pipe.endpoint_b.equipment_id);
            } else if pipe.endpoint_b.equipment_id == equipment_id {
                neighbors.insert(pipe.endpoint_a.equipment_id);
            }
        }
        neighbors.into_iter().collect()
    }

    /// Returns the number of signal connections.
    pub fn signal_count(&self) -> usize {
        self.signals.len()
    }

    /// Returns all signal connections originating from a given equipment or port.
    pub fn signals_from_equipment(&self, equipment_id: EquipmentId) -> Vec<&SignalConnection> {
        let mut results: Vec<&SignalConnection> = self
            .signals
            .values()
            .filter(|sig| match &sig.source {
                crate::signal::SignalEndpoint::Port {
                    equipment_id: eq_id,
                    ..
                } => *eq_id == equipment_id,
                crate::signal::SignalEndpoint::Tag(_) => false,
            })
            .collect();
        results.sort_by_key(|a| a.id.to_string());
        results
    }

    /// Returns all signal connections targeting a given equipment or port.
    pub fn signals_to_equipment(&self, equipment_id: EquipmentId) -> Vec<&SignalConnection> {
        let mut results: Vec<&SignalConnection> = self
            .signals
            .values()
            .filter(|sig| match &sig.destination {
                crate::signal::SignalEndpoint::Port {
                    equipment_id: eq_id,
                    ..
                } => *eq_id == equipment_id,
                crate::signal::SignalEndpoint::Tag(_) => false,
            })
            .collect();
        results.sort_by_key(|a| a.id.to_string());
        results
    }

    /// Finds all equipment reachable from `start` via directed stream traversal.
    pub fn reachable_equipment(&self, start: EquipmentId) -> Vec<EquipmentId> {
        let mut visited = BTreeSet::new();
        let mut queue = vec![start];

        while let Some(current) = queue.pop() {
            if visited.insert(current) {
                if let Some(next_nodes) = self.adjacency.get(&current) {
                    for &next in next_nodes {
                        if !visited.contains(&next) {
                            queue.push(next);
                        }
                    }
                }
            }
        }

        visited.into_iter().collect()
    }

    /// Checks if a directed path exists from `source` to `destination`.
    pub fn path_exists(&self, source: EquipmentId, destination: EquipmentId) -> bool {
        if source == destination {
            return true;
        }
        self.reachable_equipment(source).contains(&destination)
    }

    /// Computes weakly connected components (treating stream edges as undirected).
    pub fn connected_components(&self) -> Vec<Vec<EquipmentId>> {
        let mut unvisited: BTreeSet<EquipmentId> = self.equipment.keys().copied().collect();
        let mut components = Vec::new();

        while let Some(&start) = unvisited.iter().next() {
            let mut comp = Vec::new();
            let mut queue = vec![start];
            unvisited.remove(&start);

            while let Some(current) = queue.pop() {
                comp.push(current);
                for neighbor in self.equipment_neighbors(current) {
                    if unvisited.remove(&neighbor) {
                        queue.push(neighbor);
                    }
                }
            }

            comp.sort_by_key(|a| a.to_string());
            components.push(comp);
        }

        components.sort_by(|a, b| {
            let a_first = a.first().map(|x| x.to_string()).unwrap_or_default();
            let b_first = b.first().map(|x| x.to_string()).unwrap_or_default();
            a_first.cmp(&b_first)
        });
        components
    }

    // -----------------------------------------------------------------------
    // Strongly Connected Components / Recycle Loop Detection
    // -----------------------------------------------------------------------

    /// Analyzes the process/utility stream graph to identify strongly connected
    /// components (SCC) and recycle loops.
    pub fn analyze_scc(&self) -> SccAnalysisResult {
        let nodes = self.equipment_ids();
        find_strongly_connected_components(&nodes, &self.adjacency)
    }
}
