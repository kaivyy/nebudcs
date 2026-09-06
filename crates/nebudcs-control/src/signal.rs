//! Runtime signal representation and pre-allocated typed tag buffer.
//!
//! # Separation of Concerns
//!
//! - **Configuration Time**: String tag names (e.g. `"FIC-101.PV"`) are mapped once to [`TagId`] and vector offsets.
//! - **Runtime Hot Path**: Zero string operations, zero dictionary queries, zero allocations. Access is direct via [`TagId`] or integer indices.

use std::collections::BTreeMap;

use nebudcs_model::tag::{TagQuality, TagValue};
use nebudcs_types::TagId;
use serde::{Deserialize, Serialize};

/// Pre-resolved tag handle combining a human-readable name, strongly typed [`TagId`], and array index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedTag {
    pub name: String,
    pub id: TagId,
    pub index: usize,
}

/// Pre-allocated contiguous runtime store for tag values and data qualities.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RuntimeTagStore {
    name_to_id: BTreeMap<String, TagId>,
    id_to_index: BTreeMap<TagId, usize>,
    tags: Vec<ResolvedTag>,
    values: Vec<TagValue>,
    qualities: Vec<TagQuality>,
}

impl RuntimeTagStore {
    /// Creates an empty runtime tag store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a tag at configuration time, pre-allocating its runtime slot.
    pub fn register_tag(
        &mut self,
        name: impl Into<String>,
        id: TagId,
        initial_val: TagValue,
        initial_qual: TagQuality,
    ) -> usize {
        let name_str = name.into();
        let index = self.tags.len();

        self.name_to_id.insert(name_str.clone(), id);
        self.id_to_index.insert(id, index);
        self.tags.push(ResolvedTag {
            name: name_str,
            id,
            index,
        });
        self.values.push(initial_val);
        self.qualities.push(initial_qual);

        index
    }

    /// Resolves a string tag name to its [`TagId`] at configuration time.
    pub fn resolve_name(&self, name: &str) -> Option<TagId> {
        self.name_to_id.get(name).copied()
    }

    /// Resolves a [`TagId`] to its flat vector index.
    #[inline]
    pub fn get_index(&self, id: TagId) -> Option<usize> {
        self.id_to_index.get(&id).copied()
    }

    /// Number of registered tags.
    #[inline]
    pub fn len(&self) -> usize {
        self.tags.len()
    }

    /// True if store is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }

    /// Direct index read for tag value (zero allocation, O(1)).
    #[inline]
    pub fn get_value_by_index(&self, index: usize) -> Option<&TagValue> {
        self.values.get(index)
    }

    /// Direct index read for tag quality (zero allocation, O(1)).
    #[inline]
    pub fn get_quality_by_index(&self, index: usize) -> Option<TagQuality> {
        self.qualities.get(index).copied()
    }

    /// Direct index write for tag value and quality (zero allocation, O(1)).
    #[inline]
    pub fn set_by_index(&mut self, index: usize, val: TagValue, qual: TagQuality) {
        if index < self.values.len() {
            self.values[index] = val;
            self.qualities[index] = qual;
        }
    }

    /// Direct index write for float value.
    #[inline]
    pub fn set_float_by_index(&mut self, index: usize, val: f64, qual: TagQuality) {
        self.set_by_index(index, TagValue::Float(val), qual);
    }

    /// Read tag value by [`TagId`].
    #[inline]
    pub fn get_value(&self, id: TagId) -> Option<&TagValue> {
        let idx = self.get_index(id)?;
        self.get_value_by_index(idx)
    }

    /// Read tag value as a numeric float if float, integer, or bool.
    #[inline]
    pub fn get_value_float(&self, id: TagId) -> Option<f64> {
        self.get_value(id).and_then(|v| match v {
            TagValue::Float(f) => Some(*f),
            TagValue::Integer(i) => Some(*i as f64),
            TagValue::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            _ => None,
        })
    }

    /// Read tag quality by [`TagId`].
    #[inline]
    pub fn get_quality(&self, id: TagId) -> Option<TagQuality> {
        let idx = self.get_index(id)?;
        self.get_quality_by_index(idx)
    }

    /// Sets tag value and quality by [`TagId`].
    #[inline]
    pub fn set_tag(&mut self, id: TagId, val: TagValue, qual: TagQuality) {
        if let Some(idx) = self.get_index(id) {
            self.set_by_index(idx, val, qual);
        }
    }

    /// Sets float tag value and quality by [`TagId`].
    #[inline]
    pub fn set_tag_float(&mut self, id: TagId, val: f64, qual: TagQuality) {
        self.set_tag(id, TagValue::Float(val), qual);
    }

    /// Slice of all registered tags.
    pub fn tags(&self) -> &[ResolvedTag] {
        &self.tags
    }
}
