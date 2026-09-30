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
    /// Mechanical shaft power transmission between rotating machinery.
    MechanicalShaft,
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
    /// Optional waypoints for Manhattan / orthogonal visual layout on P&ID canvas.
    #[serde(default)]
    pub route_points: Vec<crate::equipment::Position2D>,
    /// Pipe inner diameter in meters (e.g. 0.1 m for a 4-inch pipe).
    #[serde(default)]
    pub diameter: Option<f64>,
    /// Pipe length in meters.
    #[serde(default)]
    pub length: Option<f64>,
    /// Pipe wall absolute roughness in meters (e.g. 0.000045 m for commercial steel).
    #[serde(default)]
    pub roughness: Option<f64>,
    /// Elevation change from source to sink in meters (delta_z = z_sink - z_source).
    #[serde(default)]
    pub elevation_change: Option<f64>,
    /// Minor / fitting loss factor K (dimensionless).
    #[serde(default)]
    pub loss_k: Option<f64>,
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
            route_points: Vec::new(),
            diameter: None,
            length: None,
            roughness: None,
            elevation_change: None,
            loss_k: None,
        }
    }

    /// Builder to configure pipe hydraulic parameters.
    pub fn with_hydraulics(
        mut self,
        diameter: f64,
        length: f64,
        roughness: f64,
        elevation_change: f64,
        loss_k: f64,
    ) -> Self {
        self.diameter = Some(diameter);
        self.length = Some(length);
        self.roughness = Some(roughness);
        self.elevation_change = Some(elevation_change);
        self.loss_k = Some(loss_k);
        self
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

    #[test]
    fn test_stream_hydraulic_parameters_serde() {
        let stream = make_stream().with_hydraulics(0.15, 45.0, 0.000045, 2.5, 3.2);
        assert_eq!(stream.diameter, Some(0.15));
        assert_eq!(stream.length, Some(45.0));
        assert_eq!(stream.roughness, Some(0.000045));
        assert_eq!(stream.elevation_change, Some(2.5));
        assert_eq!(stream.loss_k, Some(3.2));

        let json = serde_json::to_string(&stream).expect("serialize");
        let decoded: StreamDefinition = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.diameter, Some(0.15));
        assert_eq!(decoded.length, Some(45.0));
        assert_eq!(decoded.roughness, Some(0.000045));
        assert_eq!(decoded.elevation_change, Some(2.5));
        assert_eq!(decoded.loss_k, Some(3.2));
    }

    #[test]
    fn test_stream_legacy_deserialization_backward_compatible() {
        let stream = make_stream();
        let mut value = serde_json::to_value(&stream).expect("to_value");
        let obj = value.as_object_mut().unwrap();
        obj.remove("diameter");
        obj.remove("length");
        obj.remove("roughness");
        obj.remove("elevation_change");
        obj.remove("loss_k");

        let legacy_json = serde_json::to_string(&value).expect("serialize legacy");
        let decoded: StreamDefinition =
            serde_json::from_str(&legacy_json).expect("deserialize legacy");
        assert_eq!(decoded.name, stream.name);
        assert_eq!(decoded.diameter, None);
        assert_eq!(decoded.length, None);
        assert_eq!(decoded.roughness, None);
        assert_eq!(decoded.elevation_change, None);
        assert_eq!(decoded.loss_k, None);
    }
}
