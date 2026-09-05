//! Deterministic simulation execution plan.
//!
//! Converts the raw SCC analysis from [`nebudcs_topology`] into an actionable,
//! topologically ordered sequence of execution blocks (acyclic and cyclic).
//!
//! # Upstream-to-Downstream Order
//!
//! Tarjan's SCC post-order returns downstream/sink components first.
//! [`ExecutionPlan::from_topology`] reverses this order so that execution
//! proceeds deterministically from upstream (feed/sources) to downstream (products/sinks).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use nebudcs_topology::TopologyGraph;
use nebudcs_types::EquipmentId;

/// A discrete unit of execution within the simulation schedule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionBlock {
    /// An acyclic block containing a single equipment node (or a set of independent nodes).
    Acyclic {
        /// Equipment nodes to execute in this stage in deterministic order.
        equipment: Vec<EquipmentId>,
    },
    /// A cyclic block (recycle loop) requiring iterative solving across multiple nodes.
    RecycleLoop {
        /// Equipment nodes participating in the recycle loop.
        equipment: Vec<EquipmentId>,
        /// Suggested maximum solver iterations for this loop.
        max_iterations: usize,
    },
}

impl ExecutionBlock {
    /// Returns the equipment IDs in this block in deterministic order.
    pub fn equipment(&self) -> &[EquipmentId] {
        match self {
            ExecutionBlock::Acyclic { equipment } => equipment,
            ExecutionBlock::RecycleLoop { equipment, .. } => equipment,
        }
    }

    /// Returns `true` if this block represents a cyclic recycle loop.
    pub fn is_recycle(&self) -> bool {
        matches!(self, ExecutionBlock::RecycleLoop { .. })
    }
}

/// Ordered execution schedule for an entire unit or plant.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// Ordered stages of execution blocks.
    blocks: Vec<ExecutionBlock>,
}

impl ExecutionPlan {
    /// Creates an empty execution plan.
    pub fn new() -> Self {
        Self::default()
    }

    /// Constructs a deterministic execution plan from a [`TopologyGraph`].
    ///
    /// Algorithm:
    /// 1. Runs `topology.analyze_scc()` to detect all components and recycle loops.
    /// 2. Tarjan's algorithm produces components in reverse topological order (sinks first).
    /// 3. Reverses the component vector to obtain forward upstream $\rightarrow$ downstream order.
    /// 4. Maps each component to either an `Acyclic` or `RecycleLoop` block.
    pub fn from_topology(topology: &TopologyGraph) -> Self {
        let scc_result = topology.analyze_scc();

        // Convert recycle_loops list to a fast lookup set of sorted vectors
        let recycle_set: BTreeSet<Vec<EquipmentId>> =
            scc_result.recycle_loops.into_iter().collect();

        // Reverse Tarjan post-order to get upstream -> downstream topological order
        let mut ordered_components = scc_result.components;
        ordered_components.reverse();

        let mut blocks = Vec::with_capacity(ordered_components.len());
        for mut comp in ordered_components {
            // Guarantee inner determinism
            comp.sort_by_key(|a| a.to_string());

            if recycle_set.contains(&comp) {
                blocks.push(ExecutionBlock::RecycleLoop {
                    equipment: comp,
                    max_iterations: 50, // Default safe convergence bound
                });
            } else {
                blocks.push(ExecutionBlock::Acyclic { equipment: comp });
            }
        }

        Self { blocks }
    }

    /// Returns the ordered blocks in this execution plan.
    pub fn blocks(&self) -> &[ExecutionBlock] {
        &self.blocks
    }

    /// Returns the total number of execution blocks.
    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    /// Returns `true` if the plan has no blocks.
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// Returns all equipment IDs in the plan in final flattened execution order.
    pub fn flattened_execution_order(&self) -> Vec<EquipmentId> {
        let mut list = Vec::new();
        for block in &self.blocks {
            list.extend_from_slice(block.equipment());
        }
        list
    }
}
