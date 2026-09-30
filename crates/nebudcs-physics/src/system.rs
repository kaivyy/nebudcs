use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use nebudcs_sim::{SimError, SimulationSystem, StateReadView, StateWriteView, StepContext};
use nebudcs_types::{EquipmentId, TagId};
use nebudcs_units::Density;

use crate::equipment::{AccumulatorVolumeModel, GenericEquipmentPhysics};
use crate::network::{HydraulicNetwork, HydraulicNetworkSolver, NetworkDiagnostics};

/// A material, mechanical, thermal, or utility connection transferring state between equipment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessCoupling {
    pub source_id: EquipmentId,
    pub sink_id: EquipmentId,
    pub kind: CouplingKind,
    #[serde(default)]
    pub stream_name: Option<String>,
    #[serde(default)]
    pub flow_tag_id: Option<TagId>,
}

impl ProcessCoupling {
    pub fn new(source_id: EquipmentId, sink_id: EquipmentId, kind: CouplingKind) -> Self {
        Self {
            source_id,
            sink_id,
            kind,
            stream_name: None,
            flow_tag_id: None,
        }
    }

    pub fn with_stream(mut self, name: impl Into<String>, tag_id: Option<TagId>) -> Self {
        self.stream_name = Some(name.into());
        self.flow_tag_id = tag_id;
        self
    }
}

/// Generic coupling mechanisms connecting equipment nodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CouplingKind {
    /// Fluid flow from source to sink.
    MaterialFlow { nominal_density: f64 },
    /// Rotational shaft coupling: driver speed drives driven equipment speed.
    MechanicalShaft { speed_ratio: f64 },
    /// Thermal heat transfer from source to sink.
    ThermalDuty { heat_capacity_cp: f64 },
    /// Utility feed (e.g. steam, cooling water, instrument air).
    UtilitySupply {
        utility_kind: String,
        nominal_value: f64,
    },
}

/// Canonical dynamic boundary condition representing external feeds, utility sources, and export sinks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessBoundaryCondition {
    pub id: String,
    pub name: String,
    pub condition_type: String, // "feed", "utility", "product_export", "environmental"
    pub target_equipment: Option<EquipmentId>,
    pub target_port: Option<String>,
    pub target_stream: Option<String>,
    pub mass_flow_kg_s: f64,
    pub volumetric_flow_m3_s: f64,
    pub pressure_pa: f64,
    pub temperature_c: f64,
    pub phase: String,
    pub density: f64,
    pub molecular_weight: f64,
    pub composition: BTreeMap<String, f64>,
    pub enabled: bool,
}

/// Generic physics simulation system implementing [`nebudcs_sim::SimulationSystem`].
///
/// Dispatches dynamic fixed-step updates for registered equipment models (accumulators,
/// resistances, head sources, heat exchangers, and reactors) during engine tick cycles
/// and evaluates multi-equipment process, mechanical, thermal, and utility couplings.
#[derive(Debug, Default)]
pub struct GenericPhysicsSystem {
    equipment: BTreeMap<EquipmentId, GenericEquipmentPhysics>,
    couplings: Vec<ProcessCoupling>,
    utility_states: BTreeMap<String, f64>,
    boundary_conditions: BTreeMap<String, ProcessBoundaryCondition>,
    network: HydraulicNetwork,
    network_solver: HydraulicNetworkSolver,
    last_diagnostics: NetworkDiagnostics,
}

impl GenericPhysicsSystem {
    /// Creates an empty generic physics system.
    pub fn new() -> Self {
        Self {
            equipment: BTreeMap::new(),
            couplings: Vec::new(),
            utility_states: BTreeMap::new(),
            boundary_conditions: BTreeMap::new(),
            network: HydraulicNetwork::new(),
            network_solver: HydraulicNetworkSolver::new(),
            last_diagnostics: NetworkDiagnostics::default(),
        }
    }

    /// Clears all registered equipment, couplings, and utility states.
    pub fn clear(&mut self) {
        self.equipment.clear();
        self.couplings.clear();
        self.utility_states.clear();
        self.boundary_conditions.clear();
        self.network.clear();
        self.last_diagnostics = NetworkDiagnostics::default();
    }

    /// Returns a reference to the integrated hydraulic network.
    pub fn network(&self) -> &HydraulicNetwork {
        &self.network
    }

    /// Returns a mutable reference to the integrated hydraulic network.
    pub fn network_mut(&mut self) -> &mut HydraulicNetwork {
        &mut self.network
    }

    /// Sets or replaces the integrated hydraulic network.
    pub fn set_network(&mut self, network: HydraulicNetwork) {
        self.network = network;
    }

    /// Returns the latest hydraulic network diagnostics.
    pub fn network_diagnostics(&self) -> &NetworkDiagnostics {
        &self.last_diagnostics
    }

    /// Sets custom solver configuration for the integrated hydraulic network.
    pub fn set_network_solver(&mut self, solver: HydraulicNetworkSolver) {
        self.network_solver = solver;
    }

    /// Registers a generic equipment container with arbitrary capabilities.
    pub fn register_equipment(&mut self, equipment: GenericEquipmentPhysics) {
        self.equipment.insert(equipment.id(), equipment);
    }

    /// Adds a process or driver coupling between equipment nodes.
    pub fn add_coupling(&mut self, coupling: ProcessCoupling) {
        self.couplings.push(coupling);
    }

    /// Registers a process boundary condition (external feed, utility supply, or product export).
    pub fn add_boundary_condition(&mut self, bc: ProcessBoundaryCondition) {
        self.boundary_conditions.insert(bc.id.clone(), bc);
    }

    /// Returns a reference to all registered boundary conditions.
    pub fn boundary_conditions(&self) -> &BTreeMap<String, ProcessBoundaryCondition> {
        &self.boundary_conditions
    }

    /// Returns a mutable reference to all registered boundary conditions.
    pub fn boundary_conditions_mut(&mut self) -> &mut BTreeMap<String, ProcessBoundaryCondition> {
        &mut self.boundary_conditions
    }

    /// Enables or disables a registered boundary condition.
    pub fn set_boundary_condition_enabled(&mut self, id: &str, enabled: bool) {
        if let Some(bc) = self.boundary_conditions.get_mut(id) {
            bc.enabled = enabled;
        }
    }

    /// Updates the flow rate of a registered boundary condition.
    pub fn set_boundary_condition_flow(
        &mut self,
        id: &str,
        mass_flow_kg_s: f64,
        volumetric_flow_m3_s: f64,
    ) {
        if let Some(bc) = self.boundary_conditions.get_mut(id) {
            bc.mass_flow_kg_s = mass_flow_kg_s;
            bc.volumetric_flow_m3_s = volumetric_flow_m3_s;
        }
    }

    /// Sets an authoritative utility state (e.g. "STEAM_PRESSURE", "INSTRUMENT_AIR").
    pub fn set_utility(&mut self, name: impl Into<String>, value: f64) {
        self.utility_states.insert(name.into(), value);
    }

    /// Queries an authoritative utility state.
    pub fn get_utility(&self, name: &str) -> Option<f64> {
        self.utility_states.get(name).copied()
    }

    /// Convenience registration for a tank or accumulator volume with level tag publishing.
    pub fn register_tank(
        &mut self,
        equipment_id: EquipmentId,
        level_tag: TagId,
        total_volume: f64,
        cross_sectional_area: f64,
        initial_mass: f64,
        fluid_density: f64,
    ) {
        let density = Density::new_unchecked(fluid_density);
        let model =
            AccumulatorVolumeModel::new(total_volume, cross_sectional_area, initial_mass, density);

        let mut eq = GenericEquipmentPhysics::new(equipment_id);
        eq.add_capability(model);
        eq.bind_tag("level", level_tag);

        self.equipment.insert(equipment_id, eq);
    }

    /// Returns a reference to an equipment instance by ID.
    pub fn equipment(&self, id: EquipmentId) -> Option<&GenericEquipmentPhysics> {
        self.equipment.get(&id)
    }

    /// Returns a mutable reference to an equipment instance by ID.
    pub fn equipment_mut(&mut self, id: EquipmentId) -> Option<&mut GenericEquipmentPhysics> {
        self.equipment.get_mut(&id)
    }

    /// Returns a reference to all registered equipment instances.
    pub fn equipment_map(&self) -> &BTreeMap<EquipmentId, GenericEquipmentPhysics> {
        &self.equipment
    }

    /// Returns a mutable reference to all registered equipment instances.
    pub fn equipment_map_mut(&mut self) -> &mut BTreeMap<EquipmentId, GenericEquipmentPhysics> {
        &mut self.equipment
    }

    /// Returns the number of registered equipment instances.
    pub fn len(&self) -> usize {
        self.equipment.len()
    }

    /// Returns true if no equipment is registered.
    pub fn is_empty(&self) -> bool {
        self.equipment.is_empty()
    }
}

impl SimulationSystem for GenericPhysicsSystem {
    fn name(&self) -> &str {
        "GenericPhysicsSystem"
    }

    fn step(
        &mut self,
        ctx: StepContext,
        read_state: &StateReadView<'_>,
        write_state: &mut StateWriteView<'_>,
    ) -> Result<(), SimError> {
        let dt = ctx.dt;

        // 1. Synchronize actuator-driven and control variables from write_state / read_state
        for (eq_id, eq) in &mut self.equipment {
            if let Some(eq_state) = write_state.get_equipment(*eq_id) {
                if let Some(pos) = eq_state.get_var("opening") {
                    let frac = if pos > 1.0 { pos / 100.0 } else { pos };
                    let _ = eq.set_variable("opening", frac.clamp(0.0, 1.0));
                }
                if let Some(spd) = eq_state.get_var("speed") {
                    let _ = eq.set_variable("speed", spd);
                }
                if let Some(duty) = eq_state.get_var("duty") {
                    let _ = eq.set_variable("duty", duty);
                }
            } else if let Ok(eq_state) = read_state.equipment(*eq_id) {
                if let Some(pos) = eq_state.get_var("opening") {
                    let frac = if pos > 1.0 { pos / 100.0 } else { pos };
                    let _ = eq.set_variable("opening", frac.clamp(0.0, 1.0));
                }
                if let Some(spd) = eq_state.get_var("speed") {
                    let _ = eq.set_variable("speed", spd);
                }
                if let Some(duty) = eq_state.get_var("duty") {
                    let _ = eq.set_variable("duty", duty);
                }
            }

            let has_actuator_opening = write_state
                .get_equipment(*eq_id)
                .and_then(|s| s.get_var("opening"))
                .is_some()
                || read_state
                    .equipment(*eq_id)
                    .map(|s| s.get_var("opening").is_some())
                    .unwrap_or(false);

            // Also check any bound tag updated in write_state or read_state for unactuated variables
            let bindings: Vec<(String, TagId)> = eq
                .tag_bindings()
                .iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect();
            for (var_name, tag_id) in bindings {
                let tag_val = write_state
                    .get_tag(tag_id)
                    .or_else(|| read_state.tag_value(tag_id).ok());
                if let Some(val) = tag_val {
                    let float_val = match val {
                        nebudcs_model::tag::TagValue::Float(f) => Some(*f),
                        nebudcs_model::tag::TagValue::Integer(i) => Some(*i as f64),
                        _ => None,
                    };
                    if let Some(f) = float_val {
                        if var_name == "speed" {
                            let _ = eq.set_variable("speed", f);
                        } else if var_name == "opening" && !has_actuator_opening {
                            let frac = if f > 1.0 { f / 100.0 } else { f };
                            let _ = eq.set_variable("opening", frac.clamp(0.0, 1.0));
                        } else if var_name == "duty" {
                            let _ = eq.set_variable("duty", f);
                        }
                    }
                }
            }
        }

        // 1b. Synchronize boundary conditions to target equipment & utilities
        for bc in self.boundary_conditions.values() {
            if !bc.enabled {
                continue;
            }
            if bc.condition_type == "utility" {
                self.utility_states.insert(bc.name.clone(), bc.pressure_pa / 100_000.0);
            }
            if let Some(target_id) = bc.target_equipment {
                if let Some(eq) = self.equipment.get_mut(&target_id) {
                    let _ = eq.set_variable("boundary_flow", bc.mass_flow_kg_s);
                    let _ = eq.set_variable("boundary_vol_flow", bc.volumetric_flow_m3_s * 3600.0);
                    let _ = eq.set_variable("inlet_temperature", bc.temperature_c);
                    let _ = eq.set_variable("inlet_pressure", bc.pressure_pa);

                    if bc.condition_type == "feed" {
                        let _ = eq.set_variable("flow", bc.volumetric_flow_m3_s * 3600.0);
                        let _ = eq.set_variable("temperature", bc.temperature_c);
                        let _ = eq.set_variable("pressure", bc.pressure_pa / 100_000.0);
                    } else if bc.name.to_uppercase().contains("WASH")
                        || bc.name.to_uppercase().contains("WATER")
                    {
                        let _ = eq.set_variable("flow", bc.volumetric_flow_m3_s * 3600.0);
                        let _ = eq.set_variable("temperature", bc.temperature_c);
                    }
                }
            }
        }

        // 2. Evaluate Mechanical Shaft Couplings (Driver -> Shaft -> Driven Equipment)
        for c in &self.couplings {
            if let CouplingKind::MechanicalShaft { speed_ratio } = c.kind {
                let driver_speed = self
                    .equipment
                    .get(&c.source_id)
                    .and_then(|eq| eq.get_variable("speed"))
                    .unwrap_or(0.0);

                // Driver speed is influenced by steam pressure utility if configured
                let steam_factor = if let Some(steam_p) = self.utility_states.get("STEAM_PRESSURE")
                {
                    (steam_p / 40.0).clamp(0.0, 2.0).sqrt()
                } else {
                    1.0
                };

                let effective_speed = driver_speed * steam_factor * speed_ratio;
                if let Some(driven) = self.equipment.get_mut(&c.sink_id) {
                    let _ = driven.set_variable("speed", effective_speed);
                }
            }
        }

        // 3. Evaluate Hydraulic Network or fallback Material Flow Couplings
        let has_network =
            !self.network.edges.is_empty() && self.network.nodes.values().any(|n| n.is_boundary());
        if has_network {
            // (a) Sync equipment dynamic state into hydraulic network nodes and edges
            for (eq_id, eq) in &self.equipment {
                let eq_id_str = eq_id.to_string();

                // Update node pressure/temperature if node matches equipment
                if let Some(node) = self.network.get_node_mut(&eq_id_str) {
                    if let Some(p) = eq
                        .get_variable("bottom_pressure")
                        .or_else(|| eq.get_variable("pressure"))
                    {
                        node.pressure = nebudcs_units::Pressure::new_unchecked(p.max(0.0));
                    }
                    if let Some(t) = eq.get_variable("temperature") {
                        node.temperature =
                            nebudcs_units::Temperature::new_unchecked((t + 273.15).max(1.0));
                    }
                }

                // Update edge opening / pump speed if edge corresponds to equipment
                for edge in &mut self.network.edges {
                    if edge.id == eq_id_str
                        || edge.from_node == eq_id_str
                        || edge.to_node == eq_id_str
                    {
                        if let Some(opening) = eq.get_variable("opening") {
                            edge.set_opening(opening);
                        }
                        if let Some(speed) = eq.get_variable("speed") {
                            let spd_frac = if speed > 10.0 { speed / 3000.0 } else { speed };
                            edge.set_speed(spd_frac);
                        } else if let Some(speed_frac) = eq.get_variable("speed_fraction") {
                            edge.set_speed(speed_frac);
                        }
                    }
                }
            }

            // (b) Solve the hydraulic network
            if let Ok(diags) = self.network_solver.solve(&mut self.network) {
                self.last_diagnostics = diags;
            }

            // (c) Distribute computed network flows, pressure drops, and suction pressures back to equipment
            for edge in &self.network.edges {
                let flow_val = edge.mass_flow.value().abs();
                let mflow = nebudcs_units::MassFlow::new_unchecked(flow_val);

                for (eq_id, eq) in &mut self.equipment {
                    let eq_id_str = eq_id.to_string();
                    if edge.id == eq_id_str {
                        let _ = eq.set_variable("flow", flow_val);
                        let _ = eq.set_variable("delta_p", edge.pressure_drop.value().abs());
                    }
                    if edge.from_node == eq_id_str {
                        eq.set_outlet_flows(vec![mflow]);
                        let _ = eq.set_variable("flow", flow_val);
                    }
                    if edge.to_node == eq_id_str {
                        eq.set_inlet_flows(vec![mflow]);
                        let _ = eq.set_variable("flow", flow_val);
                        if let Some(from_node) = self.network.get_node(&edge.from_node) {
                            let _ = eq.set_variable("suction_pressure", from_node.pressure.value());
                        }
                    }
                }
            }
        } else {
            // Fallback: evaluate physical material flow couplings
            let mut pump_speeds: BTreeMap<EquipmentId, f64> = BTreeMap::new();
            let mut valve_openings: BTreeMap<EquipmentId, f64> = BTreeMap::new();
            for (eq_id, eq) in &self.equipment {
                if let Some(spd) = eq.get_variable("speed") {
                    pump_speeds.insert(*eq_id, spd);
                }
                if let Some(op) = eq.get_variable("opening") {
                    valve_openings.insert(*eq_id, op);
                }
            }

            for c in &self.couplings {
                if let CouplingKind::MaterialFlow { .. } = c.kind {
                    if let Some(src_eq) = self.equipment.get(&c.source_id) {
                        let upstream_p = src_eq
                            .get_variable("bottom_pressure")
                            .or_else(|| src_eq.get_variable("pressure"))
                            .unwrap_or(101_325.0);
                        if let Some(snk_eq) = self.equipment.get_mut(&c.sink_id) {
                            let _ = snk_eq.set_variable("suction_pressure", upstream_p);
                        }
                    }
                }
            }

            let mut flows_to_apply: Vec<(EquipmentId, EquipmentId, f64, Option<TagId>)> = Vec::new();
            for c in &self.couplings {
                if let CouplingKind::MaterialFlow { .. } = c.kind {
                    let source_flow = if let Some(src_eq) = self.equipment.get(&c.source_id) {
                        if let Some(speed) = src_eq.get_variable("speed") {
                            let rated_speed = if speed > 5000.0 {
                                10450.0
                            } else if speed > 1000.0 {
                                2950.0
                            } else {
                                100.0
                            };
                            let spd_frac = (speed / rated_speed).clamp(0.0, 2.0);
                            if speed < 50.0 {
                                0.0
                            } else {
                                let valve_factor =
                                    valve_openings.get(&c.sink_id).copied().unwrap_or(1.0);
                                let base_q = src_eq
                                    .get_variable("rated_flow")
                                    .or_else(|| src_eq.get_variable("design_flow"))
                                    .or_else(|| src_eq.get_variable("flow"))
                                    .unwrap_or(199.0);
                                (base_q * spd_frac * (valve_factor / 0.55).clamp(0.0, 2.0))
                                    .max(0.0)
                            }
                        } else if let Some(opening) = src_eq.get_variable("opening") {
                            let up_flow = src_eq.get_variable("flow").unwrap_or(199.0);
                            up_flow * (opening / 0.55).clamp(0.0, 2.0)
                        } else {
                            src_eq.get_variable("flow").unwrap_or(199.0)
                        }
                    } else {
                        10.0
                    };
                    flows_to_apply.push((c.source_id, c.sink_id, source_flow, c.flow_tag_id));
                }
            }

            for (src_id, sink_id, flow_val, tag_id_opt) in flows_to_apply {
                let mflow = nebudcs_units::MassFlow::new_unchecked(flow_val);
                if let Some(src) = self.equipment.get_mut(&src_id) {
                    src.set_outlet_flows(vec![mflow]);
                    let _ = src.set_variable("flow", flow_val);
                }
                if let Some(sink) = self.equipment.get_mut(&sink_id) {
                    sink.set_inlet_flows(vec![mflow]);
                    if sink.get_variable("speed").is_none() {
                        let _ = sink.set_variable("flow", flow_val);
                    }
                }
                if let Some(tid) = tag_id_opt {
                    write_state.set_tag_float(tid, flow_val);
                }
            }
        }

        // 4. Evaluate Thermal Duty Couplings
        for c in &self.couplings {
            if let CouplingKind::ThermalDuty { heat_capacity_cp } = c.kind {
                let heat_duty = self
                    .equipment
                    .get(&c.source_id)
                    .and_then(|eq| eq.get_variable("duty"))
                    .unwrap_or(50_000.0);

                if let Some(sink) = self.equipment.get_mut(&c.sink_id) {
                    sink.set_heat_duty(heat_duty);
                    let flow = sink.get_variable("flow").unwrap_or(10.0).max(0.001);
                    let delta_t = heat_duty / (flow * heat_capacity_cp);
                    let base_temp = 25.0;
                    let target_temp = base_temp + delta_t;
                    let _ = sink.set_variable("temperature", target_temp);
                    let press = 15.0 + 0.01 * flow * flow + 0.02 * (target_temp - 25.0).max(0.0);
                    let _ = sink.set_variable("pressure", press);
                }
            }
        }

        // 5. Evaluate Capabilities per equipment
        for (eq_id, eq) in &mut self.equipment {
            if let Err(e) = eq.evaluate(dt) {
                return Err(SimError::SystemExecutionFailed {
                    system_name: "GenericPhysicsSystem".to_string(),
                    detail: format!("physics step failed for equipment {eq_id}: {e}"),
                });
            }

            // Update State[N+1] equipment variables for all known exported variables
            let _ = write_state.mutate_equipment(*eq_id, |eq_state| {
                for (var_name, val) in eq.extra_variables() {
                    eq_state.set_var(var_name, *val);
                }
                for var_name in &[
                    "mass",
                    "level",
                    "bottom_pressure",
                    "headspace_pressure",
                    "flow",
                    "delta_p",
                    "duty",
                    "speed",
                    "opening",
                    "temperature",
                    "pressure",
                    "conversion",
                    "vibration",
                    "power",
                    "npsha",
                    "npshr",
                    "purity",
                    "steam_flow",
                    "distillate_flow",
                    "bottoms_flow",
                ] {
                    if let Some(val) = eq.get_variable(var_name) {
                        eq_state.set_var(*var_name, val);
                    }
                }
            });

            // Update State[N+1] DCS tags bound to variables
            for (var_name, tag_id) in eq.tag_bindings() {
                if let Some(val) = eq.get_variable(var_name) {
                    write_state.set_tag_float(*tag_id, val);
                }
            }
        }

        Ok(())
    }
}
