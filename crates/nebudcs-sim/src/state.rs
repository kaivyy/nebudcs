//! Double-buffered runtime simulation state representation.
//!
//! # Double-Buffering Invariant
//!
//! In tick $N$:
//! - Systems have read-only access to `State[N]` via [`StateReadView`].
//! - Systems have write-only access to `State[N+1]` via [`StateWriteView`].
//! - `State[N+1]` becomes `State[N]` only when the tick pipeline explicitly commits.
//! - Accidental read-after-write hazards in tick $N$ are structurally impossible
//!   because `StateReadView` and `StateWriteView` are separate handles referencing separate buffers.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use nebudcs_model::tag::{TagQuality, TagSnapshot, TagValue};
use nebudcs_types::{EquipmentId, SimulationTime, TagId};

use crate::error::SimError;

/// Runtime state for a single equipment instance.
///
/// Generic and equipment-definition driven: holds named process variables
/// and operational states without hardcoding PumpState, CompressorState, etc.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct EquipmentState {
    /// Active state name in the equipment state machine (e.g. "stopped", "running", "tripped").
    pub active_state: String,
    /// Process variables and continuous scalar parameters.
    pub variables: BTreeMap<String, f64>,
    /// Discrete states or boolean status flags (e.g. "is_energized", "permissive_ok").
    pub flags: BTreeMap<String, bool>,
}

impl EquipmentState {
    /// Creates a new equipment state with an initial state machine state.
    pub fn new(initial_state: impl Into<String>) -> Self {
        Self {
            active_state: initial_state.into(),
            variables: BTreeMap::new(),
            flags: BTreeMap::new(),
        }
    }

    /// Sets a continuous process variable.
    pub fn set_var(&mut self, name: impl Into<String>, val: f64) {
        self.variables.insert(name.into(), val);
    }

    /// Gets a continuous process variable.
    pub fn get_var(&self, name: &str) -> Option<f64> {
        self.variables.get(name).copied()
    }

    /// Sets a discrete boolean status flag.
    pub fn set_flag(&mut self, name: impl Into<String>, val: bool) {
        self.flags.insert(name.into(), val);
    }

    /// Gets a discrete boolean status flag.
    pub fn get_flag(&self, name: &str) -> Option<bool> {
        self.flags.get(name).copied()
    }
}

/// A single discrete state buffer containing values at a specific simulation point.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StateBuffer {
    /// Simulation time associated with this buffer.
    pub time: SimulationTime,
    /// Equipment states keyed by EquipmentId in deterministic BTreeMap.
    pub equipment: BTreeMap<EquipmentId, EquipmentState>,
    /// Tag values keyed by TagId in deterministic BTreeMap.
    pub tags: BTreeMap<TagId, TagValue>,
    /// Tag qualities keyed by TagId.
    pub qualities: BTreeMap<TagId, TagQuality>,
}

impl StateBuffer {
    /// Creates an empty state buffer at time $t = 0.0$.
    pub fn new() -> Self {
        Self::default()
    }

    /// Retrieves an equipment state.
    pub fn get_equipment(&self, id: EquipmentId) -> Option<&EquipmentState> {
        self.equipment.get(&id)
    }

    /// Retrieves a tag value.
    pub fn get_tag(&self, id: TagId) -> Option<&TagValue> {
        self.tags.get(&id)
    }

    /// Retrieves tag quality.
    pub fn get_quality(&self, id: TagId) -> Option<TagQuality> {
        self.qualities.get(&id).copied()
    }
}

/// Double-buffered runtime state container managing `current` ($N$) and `next` ($N+1$) buffers.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DoubleBufferedState {
    /// State[N]: the committed state of the current tick, exposed for reads.
    current: StateBuffer,
    /// State[N+1]: the working buffer receiving writes during the current tick.
    next: StateBuffer,
}

impl DoubleBufferedState {
    /// Creates a new double-buffered state initialized at time zero.
    pub fn new() -> Self {
        Self::default()
    }

    /// Initializes both current and next buffers with a matching baseline buffer.
    pub fn initialize(&mut self, initial: StateBuffer) {
        self.current = initial.clone();
        self.next = initial;
    }

    /// Direct reference to the committed current buffer State[N].
    #[inline]
    pub fn current(&self) -> &StateBuffer {
        &self.current
    }

    /// Prepares `next` buffer for tick $N$ execution: copies current into next
    /// so unwritten state persists across ticks, and updates `next.time`.
    pub fn prepare_next(&mut self, next_time: SimulationTime) {
        self.next = self.current.clone();
        self.next.time = next_time;
    }

    /// Returns a read-only view of State[N].
    #[inline]
    pub fn read_view(&self) -> StateReadView<'_> {
        StateReadView {
            buffer: &self.current,
        }
    }

    /// Returns a write-only view into State[N+1].
    #[inline]
    pub fn write_view(&mut self) -> StateWriteView<'_> {
        StateWriteView {
            buffer: &mut self.next,
        }
    }

    /// Returns simultaneous read view of State[N] and write view of State[N+1] via disjoint borrow.
    #[inline]
    pub fn rw_views(&mut self) -> (StateReadView<'_>, StateWriteView<'_>) {
        (
            StateReadView {
                buffer: &self.current,
            },
            StateWriteView {
                buffer: &mut self.next,
            },
        )
    }

    /// Commits State[N+1] into State[N], advancing the double-buffer generation.
    pub fn commit(&mut self) {
        self.current = self.next.clone();
    }
}

/// Read-only view into State[N].
///
/// Systems executing during tick $N$ read exclusively from this view.
pub struct StateReadView<'a> {
    buffer: &'a StateBuffer,
}

impl<'a> StateReadView<'a> {
    /// Simulation time for this state.
    #[inline]
    pub fn time(&self) -> SimulationTime {
        self.buffer.time
    }

    /// Looks up equipment state in State[N].
    pub fn equipment(&self, id: EquipmentId) -> Result<&'a EquipmentState, SimError> {
        self.buffer
            .equipment
            .get(&id)
            .ok_or(SimError::EquipmentNotFound { equipment_id: id })
    }

    /// Looks up tag value in State[N].
    pub fn tag_value(&self, id: TagId) -> Result<&'a TagValue, SimError> {
        self.buffer
            .tags
            .get(&id)
            .ok_or(SimError::TagNotFound { tag_id: id })
    }

    /// Looks up tag quality in State[N].
    pub fn tag_quality(&self, id: TagId) -> Result<TagQuality, SimError> {
        self.buffer
            .qualities
            .get(&id)
            .copied()
            .ok_or(SimError::TagNotFound { tag_id: id })
    }

    /// Creates a [`TagSnapshot`] from State[N].
    pub fn tag_snapshot(&self, id: TagId) -> Result<TagSnapshot, SimError> {
        let val = self.tag_value(id)?.clone();
        let qual = self.tag_quality(id)?;
        Ok(TagSnapshot {
            tag_id: id,
            value: val,
            quality: qual,
            timestamp: self.buffer.time,
        })
    }
}

/// Write-only view into State[N+1].
///
/// Systems executing during tick $N$ write their outputs exclusively to this view.
pub struct StateWriteView<'a> {
    buffer: &'a mut StateBuffer,
}

impl<'a> StateWriteView<'a> {
    /// Updates or inserts equipment state for State[N+1].
    pub fn set_equipment_state(&mut self, id: EquipmentId, state: EquipmentState) {
        self.buffer.equipment.insert(id, state);
    }

    /// Mutates an existing equipment state in State[N+1], creating a default state if not yet present.
    pub fn mutate_equipment<F>(&mut self, id: EquipmentId, f: F) -> Result<(), SimError>
    where
        F: FnOnce(&mut EquipmentState),
    {
        let eq = self.buffer.equipment.entry(id).or_default();
        f(eq);
        Ok(())
    }

    /// Updates tag value and quality for State[N+1].
    pub fn set_tag(&mut self, id: TagId, value: TagValue, quality: TagQuality) {
        self.buffer.tags.insert(id, value);
        self.buffer.qualities.insert(id, quality);
    }

    /// Convenience for setting a good float tag value in State[N+1].
    pub fn set_tag_float(&mut self, id: TagId, value: f64) {
        self.set_tag(id, TagValue::Float(value), TagQuality::Good);
    }

    /// Reads equipment state from State[N+1] (for intra-tick stage observation).
    #[inline]
    pub fn get_equipment(&self, id: EquipmentId) -> Option<&EquipmentState> {
        self.buffer.get_equipment(id)
    }

    /// Reads tag value from State[N+1] (for intra-tick stage observation).
    #[inline]
    pub fn get_tag(&self, id: TagId) -> Option<&TagValue> {
        self.buffer.get_tag(id)
    }

    /// Reads tag quality from State[N+1] (for intra-tick stage observation).
    #[inline]
    pub fn get_quality(&self, id: TagId) -> Option<TagQuality> {
        self.buffer.get_quality(id)
    }
}
