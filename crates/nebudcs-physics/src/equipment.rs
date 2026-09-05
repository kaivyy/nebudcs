//! Generic equipment physics capability models.
//!
//! # Architecture Rule
//!
//! Equipment behavior in NebuDCS is **never** implemented as rigid, hardcoded
//! types like `Pump`, `Compressor`, or `DistillationColumn`.
//!
//! Instead, behavior is modeled by composable generic physics capabilities:
//! - [`HeadSourceModel`]: Pressure/head generation (pumps, fans, compressors)
//! - [`FlowResistanceModel`]: Pressure drop from restriction (valves, orifices, pipes)
//! - [`AccumulatorVolumeModel`]: Dynamic mass holdup, level, species, and enthalpy accumulation (tanks, vessels, drums)
//! - [`HeatExchangeModel`]: Thermal duty transfer between thermal reservoirs or streams
//! - Custom implementations of [`EquipmentCapability`]: arbitrary user-defined physics

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use nebudcs_types::{EquipmentId, TagId};
use nebudcs_units::{Density, MassFlow, Pressure, SpecificEnthalpy, Temperature};

use crate::conservation::{EnergyBalance, MassBalance, SpeciesBalance};
use crate::error::PhysicsError;
use crate::hydraulics::ControlValveHydraulics;
use crate::state::ComponentComposition;

/// Standard acceleration due to gravity ($g$) in $\text{m/s}^2$.
pub const STANDARD_GRAVITY: f64 = 9.80665;

/// Execution context provided to equipment capability evaluations during a simulation step.
#[derive(Debug, Clone)]
pub struct EquipmentExecutionContext<'a> {
    /// Simulation timestep $\Delta t$ in seconds.
    pub dt: f64,
    /// Inlet mass flows to the equipment.
    pub inlet_flows: &'a [MassFlow],
    /// Outlet mass flows from the equipment.
    pub outlet_flows: &'a [MassFlow],
    /// External heat duty added to the equipment in Watts ($Q$).
    pub heat_duty_watts: f64,
    /// Shaft work input to the equipment in Watts ($W_s$).
    pub shaft_work_watts: f64,
}

/// Composable physical equipment capability trait.
///
/// NebuDCS models all equipment (pumps, vessels, exchangers, valves, columns)
/// as collections of modular capabilities implementing this trait.
pub trait EquipmentCapability: std::fmt::Debug + Send + Sync {
    /// Unique capability kind identifier (e.g. `"AccumulatorVolume"`, `"HeadSource"`).
    fn capability_name(&self) -> &str;

    /// Evaluates or dynamically integrates the capability over fixed timestep $\Delta t$.
    fn evaluate(&mut self, ctx: &EquipmentExecutionContext<'_>) -> Result<(), PhysicsError>;

    /// Queries a scalar state variable from this capability (e.g. `"level"`, `"bottom_pressure"`, `"delta_p"`).
    fn get_variable(&self, name: &str) -> Option<f64>;

    /// Sets an operating parameter or setpoint on this capability (e.g. `"speed"`, `"opening"`, `"headspace_pressure"`).
    fn set_variable(&mut self, name: &str, value: f64) -> Result<(), PhysicsError>;
}

/// Generic pressure/head generating capability.
///
/// Implements pump and compressor affinity laws:
///
/// $\Delta P(Q, \omega) = \omega^2 \cdot a_0 + a_2 \cdot Q^2$
///
/// where $\omega$ is relative rotational speed fraction ($\omega = \frac{N}{N_{rated}}$)
/// and $Q$ is volumetric flow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadSourceModel {
    /// Shutoff differential pressure at rated speed ($\omega = 1.0$) in Pascal.
    pub shutoff_delta_p: f64,
    /// Flow coefficient $a_2$ (negative for typical drooping centrifugal head curve).
    pub flow_coeff: f64,
    /// Relative rotational speed fraction ($N / N_{rated}$).
    pub speed_fraction: f64,
    /// Last computed differential pressure in Pascal.
    pub last_delta_p: f64,
}

impl HeadSourceModel {
    /// Creates a quadratic head curve model: $\Delta P(Q) = a_0 + a_2 Q^2$.
    pub fn new_quadratic(shutoff_delta_p: f64, flow_coeff: f64) -> Self {
        Self {
            shutoff_delta_p,
            flow_coeff,
            speed_fraction: 1.0,
            last_delta_p: shutoff_delta_p.max(0.0),
        }
    }

    /// Calculates differential pressure generated across the equipment at a given flow and speed.
    ///
    /// Obeying affinity laws:
    /// $\Delta P = \omega^2 \cdot a_0 + a_2 \cdot Q^2$
    pub fn calculate_delta_p(&self, volumetric_flow: f64, speed_fraction: f64) -> Pressure {
        let speed = speed_fraction.clamp(0.0, 2.0);
        let speed_sq = speed * speed;

        let dp =
            speed_sq * self.shutoff_delta_p + self.flow_coeff * volumetric_flow * volumetric_flow;
        let clamped_dp = dp.max(0.0);

        Pressure::new_unchecked(clamped_dp)
    }
}

impl EquipmentCapability for HeadSourceModel {
    fn capability_name(&self) -> &str {
        "HeadSource"
    }

    fn evaluate(&mut self, ctx: &EquipmentExecutionContext<'_>) -> Result<(), PhysicsError> {
        let total_flow: f64 = ctx.inlet_flows.iter().map(|f| f.value()).sum();
        let dp = self.calculate_delta_p(total_flow, self.speed_fraction);
        self.last_delta_p = dp.value();
        Ok(())
    }

    fn get_variable(&self, name: &str) -> Option<f64> {
        match name {
            "delta_p" => Some(self.last_delta_p),
            "speed" | "speed_fraction" => Some(self.speed_fraction),
            "shutoff_delta_p" => Some(self.shutoff_delta_p),
            _ => None,
        }
    }

    fn set_variable(&mut self, name: &str, value: f64) -> Result<(), PhysicsError> {
        match name {
            "speed" | "speed_fraction" => {
                self.speed_fraction = value.clamp(0.0, 2.0);
                Ok(())
            }
            "shutoff_delta_p" => {
                self.shutoff_delta_p = value.max(0.0);
                Ok(())
            }
            _ => Err(PhysicsError::InvalidOperatingCondition {
                message: format!("unknown variable {name} for HeadSourceModel"),
            }),
        }
    }
}

/// Generic flow resistance capability (valves, pipes, orifices).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlowResistanceModel {
    valve_hydraulics: ControlValveHydraulics,
    pub opening: f64,
    pub last_flow: f64,
}

impl FlowResistanceModel {
    /// Creates a resistance model based on valve $C_v$.
    pub fn new_from_cv(cv: f64) -> Self {
        Self {
            valve_hydraulics: ControlValveHydraulics::new(cv),
            opening: 1.0,
            last_flow: 0.0,
        }
    }

    /// Returns the underlying valve hydraulics.
    pub fn valve_hydraulics(&self) -> &ControlValveHydraulics {
        &self.valve_hydraulics
    }

    /// Calculates mass flow given inlet and outlet pressures, density, and opening fraction.
    pub fn calculate_flow(
        &self,
        inlet_p: Pressure,
        outlet_p: Pressure,
        density: Density,
        opening: f64,
    ) -> Result<MassFlow, PhysicsError> {
        self.valve_hydraulics
            .calculate_mass_flow(inlet_p, outlet_p, density, opening)
    }

    /// Calculates differential pressure given mass flow, density, and opening fraction.
    pub fn calculate_pressure_drop(
        &self,
        flow: MassFlow,
        density: Density,
        opening: f64,
    ) -> Result<nebudcs_units::DifferentialPressure, PhysicsError> {
        self.valve_hydraulics
            .calculate_pressure_drop(flow, density, opening)
    }
}

impl EquipmentCapability for FlowResistanceModel {
    fn capability_name(&self) -> &str {
        "FlowResistance"
    }

    fn evaluate(&mut self, ctx: &EquipmentExecutionContext<'_>) -> Result<(), PhysicsError> {
        let total_flow: f64 = ctx.inlet_flows.iter().map(|f| f.value()).sum();
        self.last_flow = total_flow;
        Ok(())
    }

    fn get_variable(&self, name: &str) -> Option<f64> {
        match name {
            "cv" => Some(self.valve_hydraulics.rated_cv),
            "opening" => Some(self.opening),
            "flow" => Some(self.last_flow),
            _ => None,
        }
    }

    fn set_variable(&mut self, name: &str, value: f64) -> Result<(), PhysicsError> {
        match name {
            "opening" => {
                self.opening = value.clamp(0.0, 1.0);
                Ok(())
            }
            _ => Err(PhysicsError::InvalidOperatingCondition {
                message: format!("unknown variable {name} for FlowResistanceModel"),
            }),
        }
    }
}

/// Generic dynamic accumulator volume capability (vessels, drums, tank sumps).
///
/// Tracks dynamic mass holdup $m(t)$, liquid fractional level $L(t)$, hydrostatic
/// bottom pressure $P(t) = P_{headspace} + \rho g L H$, species mass composition,
/// and accumulated enthalpy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccumulatorVolumeModel {
    total_volume: f64,            // m³
    cross_sectional_area: f64,    // m²
    height: f64,                  // m
    holdup_mass: f64,             // kg
    liquid_level: f64,            // 0.0 to 1.0
    headspace_pressure: Pressure, // Pa
    density: Density,             // kg/m³
    composition: Option<ComponentComposition>,
    specific_enthalpy: Option<SpecificEnthalpy>,
}

impl AccumulatorVolumeModel {
    /// Creates an accumulator volume with initial geometric and holdup state.
    pub fn new(
        total_volume: f64,
        cross_sectional_area: f64,
        initial_mass: f64,
        density: Density,
    ) -> Self {
        let height = (total_volume / cross_sectional_area.max(1e-6)).max(0.01);
        let liquid_volume = (initial_mass / density.value().max(1e-3)).max(0.0);
        let liquid_level = (liquid_volume / total_volume).clamp(0.0, 1.0);

        Self {
            total_volume,
            cross_sectional_area,
            height,
            holdup_mass: initial_mass,
            liquid_level,
            headspace_pressure: Pressure::new_unchecked(101_325.0),
            density,
            composition: None,
            specific_enthalpy: None,
        }
    }

    /// Sets the holdup component composition.
    pub fn set_composition(&mut self, comp: ComponentComposition) {
        self.composition = Some(comp);
    }

    /// Returns the holdup component composition if tracked.
    pub fn composition(&self) -> Option<&ComponentComposition> {
        self.composition.as_ref()
    }

    /// Sets the holdup specific enthalpy.
    pub fn set_specific_enthalpy(&mut self, h: SpecificEnthalpy) {
        self.specific_enthalpy = Some(h);
    }

    /// Returns the holdup specific enthalpy if tracked.
    pub fn specific_enthalpy(&self) -> Option<SpecificEnthalpy> {
        self.specific_enthalpy
    }

    /// Returns the baseline fluid density.
    pub fn density(&self) -> Density {
        self.density
    }

    /// Sets the baseline fluid density.
    pub fn set_density(&mut self, density: Density) {
        self.density = density;
    }

    /// Returns total vessel volume in m³.
    pub fn total_volume(&self) -> f64 {
        self.total_volume
    }

    /// Sets the headspace gas pressure in Pascal.
    pub fn set_headspace_pressure(&mut self, p: Pressure) {
        self.headspace_pressure = p;
    }

    /// Current headspace gas pressure in Pascal.
    pub fn headspace_pressure(&self) -> Pressure {
        self.headspace_pressure
    }

    /// Current holdup mass in kg.
    pub fn holdup_mass(&self) -> f64 {
        self.holdup_mass
    }

    /// Current liquid fractional level in range [0.0, 1.0].
    pub fn liquid_level(&self) -> f64 {
        self.liquid_level
    }

    /// Returns the hydrostatic bottom pressure given a fluid density.
    pub fn bottom_pressure(&self, density: Density) -> Pressure {
        let liquid_height = self.liquid_level * self.height;
        let hydrostatic_p = density.value() * STANDARD_GRAVITY * liquid_height;
        Pressure::new_unchecked(self.headspace_pressure.value() + hydrostatic_p)
    }

    /// Steps the accumulator volume dynamic holdup over fixed timestep $\Delta t$.
    pub fn step(
        &mut self,
        dt: f64,
        inflows: &[MassFlow],
        outflows: &[MassFlow],
        density: Density,
    ) -> Result<(), PhysicsError> {
        let new_mass = MassBalance::accumulate(self.holdup_mass, inflows, outflows, dt)?;
        self.holdup_mass = new_mass;
        self.density = density;

        let liquid_volume = (new_mass / density.value().max(1e-3)).max(0.0);
        self.liquid_level = (liquid_volume / self.total_volume).clamp(0.0, 1.0);

        Ok(())
    }

    /// Steps dynamic mass, species composition, and enthalpy accumulation simultaneously.
    pub fn step_detailed(
        &mut self,
        dt: f64,
        inflow_streams: &[(MassFlow, ComponentComposition, SpecificEnthalpy)],
        outflows: &[MassFlow],
        heat_duty_watts: f64,
        shaft_work_watts: f64,
    ) -> Result<(), PhysicsError> {
        let old_mass = self.holdup_mass;
        let inflows: Vec<MassFlow> = inflow_streams.iter().map(|(m, _, _)| *m).collect();
        let new_mass = MassBalance::accumulate(old_mass, &inflows, outflows, dt)?;

        let final_mass = if let Some(ref current_comp) = self.composition {
            let stream_comps: Vec<(MassFlow, ComponentComposition)> = inflow_streams
                .iter()
                .map(|(m, c, _)| (*m, c.clone()))
                .collect();
            let (accum_mass, new_comp) =
                SpeciesBalance::accumulate(old_mass, current_comp, &stream_comps, outflows, dt)?;
            self.composition = Some(new_comp);
            accum_mass
        } else {
            new_mass
        };

        let liquid_volume = (final_mass / self.density.value().max(1e-3)).max(0.0);
        self.liquid_level = (liquid_volume / self.total_volume).clamp(0.0, 1.0);
        self.holdup_mass = final_mass;

        if let Some(current_h) = self.specific_enthalpy {
            let stream_enthalpies: Vec<(MassFlow, SpecificEnthalpy)> =
                inflow_streams.iter().map(|(m, _, h)| (*m, *h)).collect();
            let inputs = crate::conservation::EnthalpyAccumulationInputs {
                initial_mass: old_mass,
                initial_enthalpy: current_h,
                heat_duty_watts,
                shaft_work_watts,
                dt,
                new_mass: final_mass,
            };
            let new_h = EnergyBalance::accumulate_enthalpy(inputs, &stream_enthalpies, outflows)?;
            self.specific_enthalpy = Some(new_h);
        }

        Ok(())
    }
}

impl EquipmentCapability for AccumulatorVolumeModel {
    fn capability_name(&self) -> &str {
        "AccumulatorVolume"
    }

    fn evaluate(&mut self, ctx: &EquipmentExecutionContext<'_>) -> Result<(), PhysicsError> {
        self.step(ctx.dt, ctx.inlet_flows, ctx.outlet_flows, self.density)
    }

    fn get_variable(&self, name: &str) -> Option<f64> {
        match name {
            "mass" => Some(self.holdup_mass),
            "level" => Some(self.liquid_level),
            "bottom_pressure" => Some(self.bottom_pressure(self.density).value()),
            "headspace_pressure" => Some(self.headspace_pressure.value()),
            "total_volume" => Some(self.total_volume),
            "enthalpy" => self.specific_enthalpy.map(|h| h.value()),
            _ => None,
        }
    }

    fn set_variable(&mut self, name: &str, value: f64) -> Result<(), PhysicsError> {
        match name {
            "headspace_pressure" => {
                let p = Pressure::new(value).map_err(|e| PhysicsError::NegativePressure {
                    value,
                    context: e.to_string(),
                })?;
                self.set_headspace_pressure(p);
                Ok(())
            }
            "level" => {
                self.liquid_level = value.clamp(0.0, 1.0);
                self.holdup_mass = self.liquid_level * self.total_volume * self.density.value();
                Ok(())
            }
            "mass" => {
                if value < 0.0 {
                    return Err(PhysicsError::InvalidOperatingCondition {
                        message: "mass cannot be negative".to_string(),
                    });
                }
                self.holdup_mass = value;
                let liquid_vol = value / self.density.value().max(1e-3);
                self.liquid_level = (liquid_vol / self.total_volume).clamp(0.0, 1.0);
                Ok(())
            }
            _ => Err(PhysicsError::InvalidOperatingCondition {
                message: format!("unknown variable {name} for AccumulatorVolumeModel"),
            }),
        }
    }
}

/// Generic heat exchange capability.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatExchangeModel {
    /// Overall heat transfer coefficient area product $UA$ in W/K.
    pub ua: f64,
    /// Last calculated duty in Watts.
    pub last_duty: f64,
}

impl HeatExchangeModel {
    /// Creates a heat exchange model with specified $UA$ (W/K).
    pub fn new_with_ua(ua: f64) -> Self {
        Self { ua, last_duty: 0.0 }
    }

    /// Calculates thermal duty $Q = UA \cdot (T_{hot} - T_{cold})$ in Watts.
    pub fn calculate_heat_duty(
        &self,
        t_hot: Temperature,
        t_cold: Temperature,
    ) -> Result<f64, PhysicsError> {
        let th = t_hot.value();
        let tc = t_cold.value();

        if !th.is_finite() || !tc.is_finite() {
            return Err(PhysicsError::NonFiniteValue {
                field: "temperature".to_string(),
                value: th,
            });
        }

        let delta_t = th - tc;
        let duty = self.ua * delta_t;
        Ok(duty)
    }

    /// Calculates log mean temperature difference (LMTD) for counter-current flow.
    pub fn lmtd(
        &self,
        t_h_in: Temperature,
        t_h_out: Temperature,
        t_c_in: Temperature,
        t_c_out: Temperature,
    ) -> Result<f64, PhysicsError> {
        let dt1 = t_h_in.value() - t_c_out.value();
        let dt2 = t_h_out.value() - t_c_in.value();

        if dt1 <= 0.0 || dt2 <= 0.0 {
            return Err(PhysicsError::InvalidOperatingCondition {
                message: "temperature crossover in heat exchanger".to_string(),
            });
        }

        let diff = dt1 - dt2;
        if diff.abs() < 1e-4 {
            Ok(0.5 * (dt1 + dt2))
        } else {
            Ok(diff / (dt1 / dt2).ln())
        }
    }
}

impl EquipmentCapability for HeatExchangeModel {
    fn capability_name(&self) -> &str {
        "HeatExchange"
    }

    fn evaluate(&mut self, ctx: &EquipmentExecutionContext<'_>) -> Result<(), PhysicsError> {
        self.last_duty = ctx.heat_duty_watts;
        Ok(())
    }

    fn get_variable(&self, name: &str) -> Option<f64> {
        match name {
            "ua" => Some(self.ua),
            "duty" => Some(self.last_duty),
            _ => None,
        }
    }

    fn set_variable(&mut self, name: &str, value: f64) -> Result<(), PhysicsError> {
        match name {
            "ua" => {
                if value < 0.0 {
                    return Err(PhysicsError::InvalidOperatingCondition {
                        message: "UA cannot be negative".to_string(),
                    });
                }
                self.ua = value;
                Ok(())
            }
            "duty" => {
                self.last_duty = value;
                Ok(())
            }
            _ => Err(PhysicsError::InvalidOperatingCondition {
                message: format!("unknown variable {name} for HeatExchangeModel"),
            }),
        }
    }
}

/// A generic equipment instance composed of modular physics capabilities.
#[derive(Debug)]
pub struct GenericEquipmentPhysics {
    id: EquipmentId,
    capabilities: Vec<Box<dyn EquipmentCapability>>,
    tag_bindings: BTreeMap<String, TagId>,
    inlet_flows: Vec<MassFlow>,
    outlet_flows: Vec<MassFlow>,
    heat_duty_watts: f64,
    shaft_work_watts: f64,
}

impl GenericEquipmentPhysics {
    /// Creates a new generic equipment container with the given ID.
    pub fn new(id: EquipmentId) -> Self {
        Self {
            id,
            capabilities: Vec::new(),
            tag_bindings: BTreeMap::new(),
            inlet_flows: Vec::new(),
            outlet_flows: Vec::new(),
            heat_duty_watts: 0.0,
            shaft_work_watts: 0.0,
        }
    }

    /// Returns the equipment ID.
    pub fn id(&self) -> EquipmentId {
        self.id
    }

    /// Adds a capability to this equipment instance.
    pub fn add_capability<C: EquipmentCapability + 'static>(&mut self, capability: C) {
        self.capabilities.push(Box::new(capability));
    }

    /// Binds an exported variable name (e.g. `"level"`, `"bottom_pressure"`, `"delta_p"`) to a DCS tag.
    pub fn bind_tag(&mut self, variable_name: impl Into<String>, tag_id: TagId) {
        self.tag_bindings.insert(variable_name.into(), tag_id);
    }

    /// Sets the current inlet mass flows.
    pub fn set_inlet_flows(&mut self, flows: Vec<MassFlow>) {
        self.inlet_flows = flows;
    }

    /// Sets the current outlet mass flows.
    pub fn set_outlet_flows(&mut self, flows: Vec<MassFlow>) {
        self.outlet_flows = flows;
    }

    /// Sets the heat duty in Watts.
    pub fn set_heat_duty(&mut self, watts: f64) {
        self.heat_duty_watts = watts;
    }

    /// Sets the shaft work in Watts.
    pub fn set_shaft_work(&mut self, watts: f64) {
        self.shaft_work_watts = watts;
    }

    /// Queries a variable from the equipment by checking its capabilities in order.
    pub fn get_variable(&self, name: &str) -> Option<f64> {
        for cap in &self.capabilities {
            if let Some(val) = cap.get_variable(name) {
                return Some(val);
            }
        }
        None
    }

    /// Sets a variable on the first capability that accepts it.
    pub fn set_variable(&mut self, name: &str, value: f64) -> Result<(), PhysicsError> {
        for cap in &mut self.capabilities {
            if cap.set_variable(name, value).is_ok() {
                return Ok(());
            }
        }
        Err(PhysicsError::InvalidOperatingCondition {
            message: format!("variable {name} not recognized on equipment {}", self.id),
        })
    }

    /// Evaluates all capabilities belonging to this equipment over timestep $\Delta t$.
    pub fn evaluate(&mut self, dt: f64) -> Result<(), PhysicsError> {
        let ctx = EquipmentExecutionContext {
            dt,
            inlet_flows: &self.inlet_flows,
            outlet_flows: &self.outlet_flows,
            heat_duty_watts: self.heat_duty_watts,
            shaft_work_watts: self.shaft_work_watts,
        };

        for cap in &mut self.capabilities {
            cap.evaluate(&ctx)?;
        }
        Ok(())
    }

    /// Returns the tag bindings map.
    pub fn tag_bindings(&self) -> &BTreeMap<String, TagId> {
        &self.tag_bindings
    }

    /// Returns the slice of registered capabilities.
    pub fn capabilities(&self) -> &[Box<dyn EquipmentCapability>] {
        &self.capabilities
    }
}
