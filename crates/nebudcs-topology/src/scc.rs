//! Strongly Connected Component (SCC) and recycle loop detection.
//!
//! Utilizes Tarjan's strongly connected components algorithm on directed graphs.
//! All outputs are ordered deterministically to preserve simulation reproducibility.

use std::collections::{BTreeMap, BTreeSet};

use nebudcs_types::EquipmentId;

/// Result of a Strongly Connected Component (SCC) analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SccAnalysisResult {
    /// All strongly connected components in topological component order.
    /// Each component is a sorted list of equipment IDs.
    pub components: Vec<Vec<EquipmentId>>,
    /// Only the cyclic components (components containing > 1 node, or 1 node with a self-loop).
    pub recycle_loops: Vec<Vec<EquipmentId>>,
}

/// Computes the strongly connected components of an equipment graph using Tarjan's algorithm.
///
/// Nodes and edges are sorted deterministically before and during traversal
/// to guarantee invariant deterministic output regardless of memory layout.
pub fn find_strongly_connected_components(
    nodes: &[EquipmentId],
    adjacency: &BTreeMap<EquipmentId, BTreeSet<EquipmentId>>,
) -> SccAnalysisResult {
    struct TarjanState<'a> {
        adjacency: &'a BTreeMap<EquipmentId, BTreeSet<EquipmentId>>,
        index: usize,
        indices: BTreeMap<EquipmentId, usize>,
        lowlinks: BTreeMap<EquipmentId, usize>,
        on_stack: BTreeSet<EquipmentId>,
        stack: Vec<EquipmentId>,
        components: Vec<Vec<EquipmentId>>,
    }

    impl<'a> TarjanState<'a> {
        fn strongconnect(&mut self, v: EquipmentId) {
            self.indices.insert(v, self.index);
            self.lowlinks.insert(v, self.index);
            self.index += 1;
            self.stack.push(v);
            self.on_stack.insert(v);

            // Explore neighbors deterministically (BTreeSet is already sorted)
            if let Some(neighbors) = self.adjacency.get(&v) {
                for &w in neighbors {
                    if !self.indices.contains_key(&w) {
                        self.strongconnect(w);
                        let w_low = *self.lowlinks.get(&w).unwrap();
                        let v_low = self.lowlinks.get_mut(&v).unwrap();
                        if w_low < *v_low {
                            *v_low = w_low;
                        }
                    } else if self.on_stack.contains(&w) {
                        let w_idx = *self.indices.get(&w).unwrap();
                        let v_low = self.lowlinks.get_mut(&v).unwrap();
                        if w_idx < *v_low {
                            *v_low = w_idx;
                        }
                    }
                }
            }

            // Root of SCC found
            if self.lowlinks.get(&v) == self.indices.get(&v) {
                let mut component = Vec::new();
                while let Some(w) = self.stack.pop() {
                    self.on_stack.remove(&w);
                    component.push(w);
                    if w == v {
                        break;
                    }
                }
                // Deterministic sort within the component
                component.sort_by_key(|a| a.to_string());
                self.components.push(component);
            }
        }
    }

    let mut state = TarjanState {
        adjacency,
        index: 0,
        indices: BTreeMap::new(),
        lowlinks: BTreeMap::new(),
        on_stack: BTreeSet::new(),
        stack: Vec::new(),
        components: Vec::new(),
    };

    // Deterministic iteration over all given nodes
    let mut sorted_nodes = nodes.to_vec();
    sorted_nodes.sort_by_key(|a| a.to_string());

    for node in sorted_nodes {
        if !state.indices.contains_key(&node) {
            state.strongconnect(node);
        }
    }

    // Determine which components are cyclic / recycles
    let mut recycle_loops = Vec::new();
    for comp in &state.components {
        let is_recycle = if comp.len() > 1 {
            true
        } else if comp.len() == 1 {
            // Self loop check
            let single = comp[0];
            adjacency
                .get(&single)
                .is_some_and(|neighbors| neighbors.contains(&single))
        } else {
            false
        };

        if is_recycle {
            recycle_loops.push(comp.clone());
        }
    }

    SccAnalysisResult {
        components: state.components,
        recycle_loops,
    }
}
