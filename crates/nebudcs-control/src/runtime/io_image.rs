//! Controller Input and Output Image Tables (%I and %Q memory model).
//!
//! Provides a decoupled signal memory model bridging field instrumentation,
//! transmitters, controllers, and actuators without coupling to hardware racks.

use nebudcs_model::tag::TagQuality;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Transport/Storage representation of an individual I/O signal value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IoSignalValue {
    /// Numerical or boolean value (booleans represented as 0.0 or 1.0).
    pub value: f64,
    /// Quality indicator (Good, Uncertain, Bad, etc.).
    pub quality: TagQuality,
    /// Authoritative simulation time when the sample was captured or written.
    pub timestamp_sim: f64,
}

impl IoSignalValue {
    /// Creates a new I/O signal with specified value, quality, and simulation timestamp.
    pub fn new(value: f64, quality: TagQuality, timestamp_sim: f64) -> Self {
        Self {
            value,
            quality,
            timestamp_sim,
        }
    }

    /// Convenience helper for a good quality signal.
    pub fn good(value: f64, timestamp_sim: f64) -> Self {
        Self::new(value, TagQuality::Good, timestamp_sim)
    }

    /// Returns boolean representation (true if value >= 0.5 and finite).
    pub fn as_bool(&self) -> bool {
        self.value.is_finite() && self.value >= 0.5
    }
}

/// Dual-table memory model holding Input Image (%I) and Output Image (%Q).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct IoImageTable {
    /// Controller Input Image Table (%I) populated at the start of a scan cycle.
    inputs: BTreeMap<String, IoSignalValue>,
    /// Controller Output Image Table (%Q) populated during logic execution and
    /// transferred to physical/virtual actuators upon scan cycle completion.
    outputs: BTreeMap<String, IoSignalValue>,
}

impl IoImageTable {
    /// Creates an empty I/O image table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reads an input point from the Input Image Table (%I).
    pub fn read_input(&self, key: &str) -> Option<&IoSignalValue> {
        self.inputs.get(key)
    }

    /// Reads the float value of an input point, if present.
    pub fn read_input_value(&self, key: &str) -> Option<f64> {
        self.inputs.get(key).map(|s| s.value)
    }

    /// Writes an input point into the Input Image Table (%I).
    pub fn write_input(
        &mut self,
        key: impl Into<String>,
        value: f64,
        quality: TagQuality,
        timestamp_sim: f64,
    ) {
        self.inputs.insert(
            key.into(),
            IoSignalValue::new(value, quality, timestamp_sim),
        );
    }

    /// Reads an output point from the Output Image Table (%Q).
    pub fn read_output(&self, key: &str) -> Option<&IoSignalValue> {
        self.outputs.get(key)
    }

    /// Reads the float value of an output point, if present.
    pub fn read_output_value(&self, key: &str) -> Option<f64> {
        self.outputs.get(key).map(|s| s.value)
    }

    /// Writes an output point into the Output Image Table (%Q).
    pub fn write_output(
        &mut self,
        key: impl Into<String>,
        value: f64,
        quality: TagQuality,
        timestamp_sim: f64,
    ) {
        self.outputs.insert(
            key.into(),
            IoSignalValue::new(value, quality, timestamp_sim),
        );
    }

    /// Clears all entries in the Input Image Table.
    pub fn clear_inputs(&mut self) {
        self.inputs.clear();
    }

    /// Clears all entries in the Output Image Table.
    pub fn clear_outputs(&mut self) {
        self.outputs.clear();
    }

    /// Access immutable reference to the full input map.
    pub fn inputs(&self) -> &BTreeMap<String, IoSignalValue> {
        &self.inputs
    }

    /// Access immutable reference to the full output map.
    pub fn outputs(&self) -> &BTreeMap<String, IoSignalValue> {
        &self.outputs
    }

    /// Access mutable reference to the full input map.
    pub fn inputs_mut(&mut self) -> &mut BTreeMap<String, IoSignalValue> {
        &mut self.inputs
    }

    /// Access mutable reference to the full output map.
    pub fn outputs_mut(&mut self) -> &mut BTreeMap<String, IoSignalValue> {
        &mut self.outputs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_image_table_crud() {
        let mut table = IoImageTable::new();
        table.write_input("%IW0", 12.34, TagQuality::Good, 1.0);
        table.write_output("%QW0", 56.78, TagQuality::Good, 1.0);

        assert_eq!(table.read_input_value("%IW0"), Some(12.34));
        assert_eq!(table.read_output_value("%QW0"), Some(56.78));
        assert_eq!(table.read_input_value("%IW1"), None);

        let input_sig = table.read_input("%IW0").unwrap();
        assert_eq!(input_sig.quality, TagQuality::Good);
        assert_eq!(input_sig.timestamp_sim, 1.0);
    }
}
