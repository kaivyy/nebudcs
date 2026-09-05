//! Tag values, quality, and snapshots.
//!
//! Tags are the primary interface between the simulation engine and external
//! systems (HMI, historian, control, alarms).  A tag has a strongly typed
//! value — not a raw `f64`.

use serde::{Deserialize, Serialize};

use nebudcs_types::{SimulationTime, TagId};

/// The runtime value of a tag.
///
/// Tags are strongly typed — a `Float` tag cannot be confused with a `Bool`
/// tag at the application level.  The `BadQuality` variant represents any
/// state where the value is not usable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum TagValue {
    Float(f64),
    Bool(bool),
    Integer(i64),
    Text(String),
    /// The value cannot be trusted (sensor failure, communication loss, etc.).
    BadQuality,
}

/// Data quality associated with a tag value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TagQuality {
    /// The value is reliable and within sensor range.
    Good,
    /// The value may be correct but confidence is reduced.
    Uncertain,
    /// The value is not reliable.
    Bad,
    /// No source has provided a value for this tag.
    NotConnected,
}

/// A snapshot of a tag's value at a specific simulation time.
///
/// Historians record sequences of `TagSnapshot`s.  Protocol layers send
/// these to connected clients.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TagSnapshot {
    pub tag_id: TagId,
    pub value: TagValue,
    pub quality: TagQuality,
    /// The simulation clock time at which this value was valid.
    pub timestamp: SimulationTime,
}

impl TagSnapshot {
    /// Construct a good-quality float snapshot.
    pub fn good_float(tag_id: TagId, value: f64, timestamp: SimulationTime) -> Self {
        Self {
            tag_id,
            value: TagValue::Float(value),
            quality: TagQuality::Good,
            timestamp,
        }
    }

    /// Construct a bad-quality snapshot.
    pub fn bad(tag_id: TagId, timestamp: SimulationTime) -> Self {
        Self {
            tag_id,
            value: TagValue::BadQuality,
            quality: TagQuality::Bad,
            timestamp,
        }
    }

    /// Returns `true` if the quality is `Good`.
    pub fn is_good(&self) -> bool {
        self.quality == TagQuality::Good
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use nebudcs_types::{SimulationTime, TagId};

    fn t0() -> SimulationTime {
        SimulationTime::try_new(0.0).unwrap()
    }

    #[test]
    fn test_tag_value_variants() {
        let values = vec![
            TagValue::Float(42.5),
            TagValue::Bool(true),
            TagValue::Integer(-42),
            TagValue::Text("OK".to_owned()),
            TagValue::BadQuality,
        ];
        for v in &values {
            let json = serde_json::to_string(v).expect("serialize");
            let decoded: TagValue = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(v, &decoded);
        }
    }

    #[test]
    fn test_tag_quality_good_and_bad() {
        let id = TagId::new();
        let good = TagSnapshot::good_float(id, 42.0, t0());
        assert!(good.is_good());
        assert_eq!(good.quality, TagQuality::Good);

        let bad = TagSnapshot::bad(id, t0());
        assert!(!bad.is_good());
        assert_eq!(bad.quality, TagQuality::Bad);
        assert_eq!(bad.value, TagValue::BadQuality);
    }

    #[test]
    fn test_tag_snapshot_serde() {
        let id = TagId::new();
        let snap = TagSnapshot::good_float(id, 101_325.0, t0());
        let json = serde_json::to_string(&snap).expect("serialize");
        let decoded: TagSnapshot = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(snap.tag_id, decoded.tag_id);
        assert_eq!(snap.quality, decoded.quality);
    }
}
