//! Hydraulic network node representation.
//!
//! Nodes represent junctions, vessels, boundaries, or equipment connection ports
//! in a process hydraulic network. Each node has a defined pressure, elevation,
//! temperature, composition, and physical phase.

use serde::{Deserialize, Serialize};

use nebudcs_types::{EquipmentId, PortId};
use nebudcs_units::{Density, Pressure, Temperature};

use crate::state::{ComponentComposition, Phase};

/// Classification of a hydraulic node within the network topology.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeKind {
    /// Dirichlet boundary with fixed or externally driven pressure
    /// (e.g. atmospheric storage tank, supply battery limit, flare header).
    PressureBoundary { pressure: Pressure },
    /// Internal junction (tee, manifold, intermediate piping node) where pressure
    /// is an unknown solved by mass conservation: $\sum \dot{m}_{in} = \sum \dot{m}_{out}$.
    InternalJunction,
    /// Connection port on a process equipment instance (e.g. pump nozzle, vessel nozzle).
    EquipmentPort {
        equipment_id: EquipmentId,
        port_id: PortId,
    },
}

/// A node in the hydraulic network.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HydraulicNode {
    /// Unique node identifier.
    pub id: String,
    /// Human-readable label or tag.
    pub name: String,
    /// Classification of the node.
    pub kind: NodeKind,
    /// Static pressure at this node in Pascal.
    pub pressure: Pressure,
    /// Temperature at this node in Kelvin.
    pub temperature: Temperature,
    /// Elevation above plant reference plane in meters.
    pub elevation: f64,
    /// Fluid density at this node in kg/m³.
    pub density: Density,
    /// Specific enthalpy in J/kg.
    pub enthalpy: f64,
    /// Fluid phase state.
    pub phase: Phase,
    /// Multicomponent composition (mole or mass fractions).
    pub composition: Option<ComponentComposition>,
}

impl HydraulicNode {
    /// Creates a new internal junction node with initial guesses.
    pub fn new_junction(id: impl Into<String>, name: impl Into<String>, elevation: f64) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind: NodeKind::InternalJunction,
            pressure: Pressure::new_unchecked(101_325.0),
            temperature: Temperature::new_unchecked(293.15),
            elevation,
            density: Density::new_unchecked(1000.0),
            enthalpy: 84_000.0,
            phase: Phase::Liquid,
            composition: None,
        }
    }

    /// Creates a fixed pressure boundary node.
    pub fn new_boundary(
        id: impl Into<String>,
        name: impl Into<String>,
        pressure: Pressure,
        elevation: f64,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind: NodeKind::PressureBoundary { pressure },
            pressure,
            temperature: Temperature::new_unchecked(293.15),
            elevation,
            density: Density::new_unchecked(1000.0),
            enthalpy: 84_000.0,
            phase: Phase::Liquid,
            composition: None,
        }
    }

    /// Creates an equipment port node.
    pub fn new_equipment_port(
        id: impl Into<String>,
        name: impl Into<String>,
        equipment_id: EquipmentId,
        port_id: PortId,
        elevation: f64,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind: NodeKind::EquipmentPort {
                equipment_id,
                port_id,
            },
            pressure: Pressure::new_unchecked(101_325.0),
            temperature: Temperature::new_unchecked(293.15),
            elevation,
            density: Density::new_unchecked(1000.0),
            enthalpy: 84_000.0,
            phase: Phase::Liquid,
            composition: None,
        }
    }

    /// Returns `true` if this node is an unknown internal junction to be solved.
    pub fn is_internal(&self) -> bool {
        matches!(self.kind, NodeKind::InternalJunction)
    }

    /// Returns `true` if this node is a fixed pressure boundary.
    pub fn is_boundary(&self) -> bool {
        matches!(self.kind, NodeKind::PressureBoundary { .. })
    }

    /// Sets fluid physical properties.
    pub fn with_properties(
        mut self,
        density: Density,
        temperature: Temperature,
        enthalpy: f64,
        phase: Phase,
    ) -> Self {
        self.density = density;
        self.temperature = temperature;
        self.enthalpy = enthalpy;
        self.phase = phase;
        self
    }
}
