//! Process stream definitions.
//!
//! A [`StreamDefinition`] describes a connection between two equipment ports.
//! It is a configuration-layer object — the *runtime* stream state (flow,
//! pressure, composition values at each tick) belongs in `nebudcs-sim`
//! (Phase 3).
//!
//! # Composition
//!
//! Stream composition is stored as a map of component names to mole fractions.
//! It is intentionally **not** a single boolean H2 flag.  A hydrogen-rich
//! stream in a reformer circuit has a real composition (H2, CH4, C2+, N2, …).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use nebudcs_types::{EquipmentId, PortId, StreamId};

/// The class of fluid or energy flowing through a stream.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamType {
    /// A hydrocarbon or process fluid stream.
    Material,
    // Utility streams — first-class network citizens.
    UtilityHydrogen,
    UtilitySteamHp,
    UtilitySteamMp,
    UtilitySteamLp,
    UtilityCoolingWater,
    UtilityInstrumentAir,
    UtilityPlantAir,
    UtilityNitrogen,
    UtilityFuelGas,
    UtilityElectrical,
    UtilityFlare,
    /// Catalyst transport between reactor and regenerator.
    CatalystTransport,
}

/// Component mole fractions for a material stream.
///
/// Keys are component names (e.g., `"H2"`, `"CH4"`, `"C2H6"`).
/// Values are mole fractions — must sum to 1.0 when the stream is fully
/// specified, but this is validated by the topology engine (Phase 2).
///
/// This is **never** a single boolean or fixed-property H2 flag.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StreamComposition {
    /// `component_name` → mole_fraction mapping, insertion-ordered.
    pub components: IndexMap<String, f64>,
}

impl StreamComposition {
    /// Create an empty composition.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Create a composition from a list of (name, fraction) pairs.
    pub fn from_pairs(pairs: impl IntoIterator<Item = (impl Into<String>, f64)>) -> Self {
        Self {
            components: pairs.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        }
    }

    /// Returns the number of components.
    pub fn len(&self) -> usize {
        self.components.len()
    }

    /// Returns `true` if no components are defined.
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }
}

/// A connection between two equipment ports.
///
/// The source equipment exposes the outlet port; the sink equipment exposes
/// the inlet port.  Port compatibility validation (phase, domain, etc.) is
/// performed by the topology engine (Phase 2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamDefinition {
    pub id: StreamId,
    pub name: String,
    pub stream_type: StreamType,
    /// The equipment and port at the upstream (source) end.
    pub source: PortRef,
    /// The equipment and port at the downstream (sink) end.
    pub sink: PortRef,
    /// Optional initial composition for material streams.
    #[serde(default)]
    pub composition: Option<StreamComposition>,
}

/// A reference to a specific port on a specific equipment instance.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PortRef {
    pub equipment_id: EquipmentId,
    pub port_id: PortId,
}

impl StreamDefinition {
    /// Construct a new stream connecting two ports.
    pub fn new(
        name: impl Into<String>,
        stream_type: StreamType,
        source_equipment: EquipmentId,
        source_port: PortId,
        sink_equipment: EquipmentId,
        sink_port: PortId,
    ) -> Self {
        Self {
            id: StreamId::new(),
            name: name.into(),
            stream_type,
            source: PortRef {
                equipment_id: source_equipment,
                port_id: source_port,
            },
            sink: PortRef {
                equipment_id: sink_equipment,
                port_id: sink_port,
            },
            composition: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use nebudcs_types::{EquipmentId, PortId};

    fn make_stream() -> StreamDefinition {
        let pump_id = EquipmentId::new();
        let vessel_id = EquipmentId::new();
        let discharge_port = PortId::new();
        let inlet_port = PortId::new();
        StreamDefinition::new(
            "K101-to-V101",
            StreamType::Material,
            pump_id,
            discharge_port,
            vessel_id,
            inlet_port,
        )
    }

    #[test]
    fn test_stream_definition_connects_ports() {
        let stream = make_stream();
        assert_eq!(stream.stream_type, StreamType::Material);
        assert_ne!(stream.source.equipment_id, stream.sink.equipment_id);
    }

    #[test]
    fn test_stream_composition_not_boolean() {
        // Hydrogen composition must be a component map, not a boolean.
        let composition = StreamComposition::from_pairs([
            ("H2", 0.75),
            ("CH4", 0.15),
            ("C2H6", 0.05),
            ("N2", 0.05),
        ]);
        assert_eq!(composition.components.len(), 4);
        assert!(composition.components.contains_key("H2"));
        assert!(composition.components.contains_key("N2"));
        // Confirm it is not just a boolean flag.
        let h2_fraction = composition.components["H2"];
        assert!((h2_fraction - 0.75).abs() < 1e-10);
    }

    #[test]
    fn test_stream_serde_json() {
        let stream = make_stream();
        let json = serde_json::to_string(&stream).expect("serialize");
        let decoded: StreamDefinition = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(stream.id, decoded.id);
        assert_eq!(stream.stream_type, decoded.stream_type);
    }

    #[test]
    fn test_stream_type_variants_serde() {
        let types = vec![
            StreamType::Material,
            StreamType::UtilityHydrogen,
            StreamType::UtilitySteamHp,
            StreamType::UtilityCoolingWater,
            StreamType::UtilityInstrumentAir,
            StreamType::UtilityElectrical,
            StreamType::UtilityFlare,
            StreamType::CatalystTransport,
        ];
        for t in &types {
            let json = serde_json::to_string(t).unwrap();
            let decoded: StreamType = serde_json::from_str(&json).unwrap();
            assert_eq!(t, &decoded);
        }
    }
}
