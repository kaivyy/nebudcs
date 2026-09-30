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
use std::sync::Arc;

use nebudcs_types::{EquipmentId, TagId};
use nebudcs_units::{Density, MassFlow, Pressure, SpecificEnthalpy, Temperature};

use crate::conservation::{EnergyBalance, MassBalance, SpeciesBalance};
use crate::error::PhysicsError;
use crate::hydraulics::ControlValveHydraulics;
use crate::properties::PropertyPackage;
use crate::state::{ComponentComposition, PhysicalState};

/// Standard acceleration due to gravity ($g$) in $\text{m/s}^2$.
pub const STANDARD_GRAVITY: f64 = 9.80665;

/// Execution context provided to equipment capability evaluations during a simulation step.
#[derive(Clone)]
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
    /// Upstream physical states for each inlet stream.
    pub inlet_states: &'a [PhysicalState],
    /// Pluggable thermodynamic property package context, if available.
    pub property_package: Option<&'a dyn PropertyPackage>,
}

impl<'a> std::fmt::Debug for EquipmentExecutionContext<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EquipmentExecutionContext")
            .field("dt", &self.dt)
            .field("inlet_flows", &self.inlet_flows)
            .field("outlet_flows", &self.outlet_flows)
            .field("heat_duty_watts", &self.heat_duty_watts)
            .field("shaft_work_watts", &self.shaft_work_watts)
            .field("inlet_states", &self.inlet_states)
            .field("property_package", &self.property_package.map(|p| p.name()))
            .finish()
    }
}

impl<'a> EquipmentExecutionContext<'a> {
    /// Creates a basic execution context with hydraulic flows.
    pub fn new(dt: f64, inlet_flows: &'a [MassFlow], outlet_flows: &'a [MassFlow]) -> Self {
        Self {
            dt,
            inlet_flows,
            outlet_flows,
            heat_duty_watts: 0.0,
            shaft_work_watts: 0.0,
            inlet_states: &[],
            property_package: None,
        }
    }

    /// Builder to provide thermodynamic context.
    pub fn with_thermodynamics(
        mut self,
        inlet_states: &'a [PhysicalState],
        property_package: Option<&'a dyn PropertyPackage>,
    ) -> Self {
        self.inlet_states = inlet_states;
        self.property_package = property_package;
        self
    }
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
    /// Rotational speed (e.g. in RPM or native units).
    pub speed: f64,
    /// Relative rotational speed fraction ($N / N_{rated}$).
    pub speed_fraction: f64,
    /// Last computed differential pressure in Pascal.
    pub last_delta_p: f64,
    /// Last measured inlet mass flow.
    pub last_flow: f64,
    /// Upstream suction pressure in Pascal.
    pub suction_pressure: f64,
    /// Net Positive Suction Head Required in meters (NPSHR).
    pub npsh_required: f64,
    /// Net Positive Suction Head Available in meters (NPSHA).
    pub npsh_available: f64,
    /// RMS vibration velocity in mm/s.
    pub vibration: f64,
    /// Shaft power consumed in Watts.
    pub power: f64,
}

impl HeadSourceModel {
    /// Creates a quadratic head curve model: $\Delta P(Q) = a_0 + a_2 Q^2$.
    pub fn new_quadratic(shutoff_delta_p: f64, flow_coeff: f64) -> Self {
        Self {
            shutoff_delta_p,
            flow_coeff,
            speed: 3000.0,
            speed_fraction: 1.0,
            last_delta_p: shutoff_delta_p.max(0.0),
            last_flow: 0.0,
            suction_pressure: 101_325.0,
            npsh_required: 3.2,
            npsh_available: 10.0,
            vibration: 1.2,
            power: 0.0,
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
        self.last_flow = total_flow;
        let dp = self.calculate_delta_p(total_flow, self.speed_fraction);
        self.last_delta_p = dp.value();

        // Calculate shaft power: W = (dp * Q) / eta + mechanical idle loss
        let rho = 1000.0;
        let vol_flow = total_flow / rho;
        let hydraulic_power = self.last_delta_p * vol_flow;
        let mech_idle = 1500.0 * self.speed_fraction * self.speed_fraction;
        self.power = (hydraulic_power / 0.75) + mech_idle;

        // Calculate NPSHA: (P_suction - P_vap) / (rho * g)
        let p_vap = 2340.0; // Pa at ~20°C water
        let g = STANDARD_GRAVITY;
        self.npsh_available = ((self.suction_pressure - p_vap) / (rho * g)).max(0.0);

        // NPSH margin and Cavitation / Vibration evaluation
        let speed_norm = self.speed_fraction.clamp(0.0, 2.0);
        let base_vib = 1.2 * speed_norm;
        if self.speed_fraction > 0.05 && self.npsh_available < self.npsh_required {
            let deficit = self.npsh_required - self.npsh_available;
            // Severe vibration surge due to cavitation collapse
            self.vibration = base_vib + 2.5 * deficit + 0.8 * (deficit * deficit);
        } else {
            self.vibration = base_vib;
        }

        Ok(())
    }

    fn get_variable(&self, name: &str) -> Option<f64> {
        match name {
            "flow" => Some(self.last_flow),
            "delta_p" => Some(self.last_delta_p),
            "speed" => Some(self.speed),
            "speed_fraction" => Some(self.speed_fraction),
            "shutoff_delta_p" => Some(self.shutoff_delta_p),
            "suction_pressure" => Some(self.suction_pressure),
            "npsha" => Some(self.npsh_available),
            "npshr" => Some(self.npsh_required),
            "vibration" => Some(self.vibration),
            "power" => Some(self.power),
            _ => None,
        }
    }

    fn set_variable(&mut self, name: &str, value: f64) -> Result<(), PhysicsError> {
        match name {
            "flow" => {
                self.last_flow = value;
                Ok(())
            }
            "speed" => {
                self.speed = value;
                if value > 10.0 {
                    self.speed_fraction = (value / 3000.0).clamp(0.0, 2.0);
                } else {
                    self.speed_fraction = value.clamp(0.0, 2.0);
                }
                Ok(())
            }
            "speed_fraction" => {
                self.speed_fraction = value.clamp(0.0, 2.0);
                self.speed = self.speed_fraction * 3000.0;
                Ok(())
            }
            "shutoff_delta_p" => {
                self.shutoff_delta_p = value.max(0.0);
                Ok(())
            }
            "suction_pressure" => {
                self.suction_pressure = value.max(0.0);
                Ok(())
            }
            "npsh_required" | "npshr" => {
                self.npsh_required = value.max(0.1);
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

/// Generic thermal and reaction process state capability (heaters, reactors, columns).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessThermalModel {
    pub temperature: f64,
    pub pressure: f64,
    pub heat_duty: f64,
    pub conversion: f64,
    pub reaction_heat: f64,
}

impl ProcessThermalModel {
    /// Creates a thermal and pressure state capability with initial temperature (°C) and pressure (bar or Pa).
    pub fn new(initial_temperature: f64, initial_pressure: f64) -> Self {
        Self {
            temperature: initial_temperature,
            pressure: initial_pressure,
            heat_duty: 0.0,
            conversion: 0.0,
            reaction_heat: 0.0,
        }
    }
}

impl EquipmentCapability for ProcessThermalModel {
    fn capability_name(&self) -> &str {
        "ProcessThermal"
    }

    fn evaluate(&mut self, ctx: &EquipmentExecutionContext<'_>) -> Result<(), PhysicsError> {
        let flow: f64 = ctx.inlet_flows.iter().map(|f| f.value()).sum();
        if flow > 0.001 {
            let cp = 2500.0;
            let delta_t_duty = ctx.heat_duty_watts / (flow * cp);
            let target_temp = 25.0 + delta_t_duty + self.reaction_heat;
            let tau = 5.0;
            let alpha = (ctx.dt / tau).clamp(0.0, 1.0);
            self.temperature += alpha * (target_temp - self.temperature);
        }
        Ok(())
    }

    fn get_variable(&self, name: &str) -> Option<f64> {
        match name {
            "temperature" => Some(self.temperature),
            "pressure" => Some(self.pressure),
            "duty" | "heat_duty" => Some(self.heat_duty),
            "conversion" => Some(self.conversion),
            _ => None,
        }
    }

    fn set_variable(&mut self, name: &str, value: f64) -> Result<(), PhysicsError> {
        match name {
            "temperature" => {
                self.temperature = value;
                self.reaction_heat = (value - 25.0).max(0.0);
                Ok(())
            }
            "pressure" => {
                self.pressure = value;
                Ok(())
            }
            "duty" | "heat_duty" => {
                self.heat_duty = value;
                Ok(())
            }
            "conversion" => {
                self.conversion = value.clamp(0.0, 1.0);
                Ok(())
            }
            "reaction_heat" => {
                self.reaction_heat = value;
                Ok(())
            }
            _ => Err(PhysicsError::InvalidOperatingCondition {
                message: format!("unknown variable {name} for ProcessThermalModel"),
            }),
        }
    }
}

/// Generic steam or gas expansion turbine capability.
///
/// Converts thermodynamic fluid expansion across an enthalpy drop into shaft power and rotational speed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurbineExpansionModel {
    /// Rated shaft power in Watts at nominal steam flow.
    pub rated_power_watts: f64,
    /// Isentropic expansion efficiency fraction (0.0 to 1.0).
    pub isentropic_efficiency: f64,
    /// Live throttle steam inlet pressure in Pascal.
    pub inlet_pressure_pa: f64,
    /// Live exhaust condenser/header pressure in Pascal.
    pub exhaust_pressure_pa: f64,
    /// Rotational shaft speed in RPM.
    pub speed: f64,
    /// Live mechanical shaft power produced in Watts.
    pub power_output_watts: f64,
    /// Steam consumption mass flow in kg/s.
    pub steam_flow: f64,
    /// Governor valve opening fraction (0.0 to 1.0).
    pub governor_opening: f64,
}

impl TurbineExpansionModel {
    pub fn new(rated_power_watts: f64) -> Self {
        Self {
            rated_power_watts,
            isentropic_efficiency: 0.78,
            inlet_pressure_pa: 4_000_000.0, // 40 bar HP steam
            exhaust_pressure_pa: 300_000.0, // 3 bar LP steam
            speed: 3000.0,
            power_output_watts: 0.0,
            steam_flow: 0.0,
            governor_opening: 1.0,
        }
    }
}

impl EquipmentCapability for TurbineExpansionModel {
    fn capability_name(&self) -> &str {
        "TurbineExpansion"
    }

    fn evaluate(&mut self, ctx: &EquipmentExecutionContext<'_>) -> Result<(), PhysicsError> {
        let nominal_delta_h = 188_000.0;
        let rated_steam_flow =
            self.rated_power_watts / (self.isentropic_efficiency.max(0.1) * nominal_delta_h);
        let flow: f64 = ctx.inlet_flows.iter().map(|f| f.value()).sum();
        self.steam_flow = if flow > 0.001 {
            flow
        } else {
            self.governor_opening * rated_steam_flow
        };

        let pr =
            (self.exhaust_pressure_pa / self.inlet_pressure_pa.max(100_000.0)).clamp(0.01, 1.0);
        let delta_h_is = 420_000.0 * (1.0 - pr.powf(0.23));
        self.power_output_watts = self.steam_flow * self.isentropic_efficiency * delta_h_is;

        if self.speed < 1.0 {
            let p_ratio =
                (self.power_output_watts / self.rated_power_watts.max(1000.0)).clamp(0.0, 2.5);
            self.speed = 3000.0 * p_ratio.sqrt();
        }
        Ok(())
    }

    fn get_variable(&self, name: &str) -> Option<f64> {
        match name {
            "speed" => Some(self.speed),
            "power" => Some(self.power_output_watts),
            "flow" | "steam_flow" => Some(self.steam_flow),
            "inlet_pressure" => Some(self.inlet_pressure_pa),
            "exhaust_pressure" => Some(self.exhaust_pressure_pa),
            "governor_opening" | "opening" => Some(self.governor_opening),
            _ => None,
        }
    }

    fn set_variable(&mut self, name: &str, value: f64) -> Result<(), PhysicsError> {
        match name {
            "speed" => {
                self.speed = value.max(0.0);
                Ok(())
            }
            "governor_opening" | "opening" => {
                self.governor_opening = value.clamp(0.0, 1.0);
                Ok(())
            }
            "inlet_pressure" => {
                self.inlet_pressure_pa = value.max(101_325.0);
                Ok(())
            }
            "exhaust_pressure" => {
                self.exhaust_pressure_pa = value.max(10_000.0);
                Ok(())
            }
            "flow" | "steam_flow" => {
                self.steam_flow = value.max(0.0);
                Ok(())
            }
            _ => Err(PhysicsError::InvalidOperatingCondition {
                message: format!("unknown variable {name} for TurbineExpansionModel"),
            }),
        }
    }
}

/// Generic directional flow check valve capability.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckValveModel {
    pub cracking_pressure_pa: f64,
    pub forward_cv: f64,
    pub is_open: bool,
    pub last_flow: f64,
}

impl CheckValveModel {
    pub fn new(forward_cv: f64, cracking_pressure_pa: f64) -> Self {
        Self {
            cracking_pressure_pa,
            forward_cv,
            is_open: true,
            last_flow: 0.0,
        }
    }
}

impl EquipmentCapability for CheckValveModel {
    fn capability_name(&self) -> &str {
        "CheckValve"
    }

    fn evaluate(&mut self, ctx: &EquipmentExecutionContext<'_>) -> Result<(), PhysicsError> {
        let flow: f64 = ctx.inlet_flows.iter().map(|f| f.value()).sum();
        self.is_open = flow >= 0.0;
        self.last_flow = if self.is_open { flow } else { 0.0 };
        Ok(())
    }

    fn get_variable(&self, name: &str) -> Option<f64> {
        match name {
            "flow" => Some(self.last_flow),
            "is_open" => Some(if self.is_open { 1.0 } else { 0.0 }),
            "cv" => Some(if self.is_open { self.forward_cv } else { 0.0 }),
            _ => None,
        }
    }

    fn set_variable(&mut self, name: &str, value: f64) -> Result<(), PhysicsError> {
        match name {
            "is_open" => {
                self.is_open = value > 0.5;
                Ok(())
            }
            _ => Err(PhysicsError::InvalidOperatingCondition {
                message: format!("unknown variable {name} for CheckValveModel"),
            }),
        }
    }
}

/// Generic multi-stage distillation/fractionation column capability.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DistillationColumnModel {
    pub tray_count: usize,
    pub top_pressure_pa: f64,
    pub bottom_pressure_pa: f64,
    pub top_temperature_c: f64,
    pub bottom_temperature_c: f64,
    pub reflux_ratio: f64,
    pub boilup_duty_watts: f64,
    pub bottom_level: f64,
    pub distillate_flow: f64,
    pub bottoms_flow: f64,
    pub overhead_purity: f64,
}

impl DistillationColumnModel {
    pub fn new(tray_count: usize) -> Self {
        Self {
            tray_count,
            top_pressure_pa: 150_000.0,
            bottom_pressure_pa: 190_000.0,
            top_temperature_c: 65.0,
            bottom_temperature_c: 110.0,
            reflux_ratio: 2.5,
            boilup_duty_watts: 500_000.0,
            bottom_level: 0.50,
            distillate_flow: 8.0,
            bottoms_flow: 12.0,
            overhead_purity: 0.985,
        }
    }
}

impl EquipmentCapability for DistillationColumnModel {
    fn capability_name(&self) -> &str {
        "DistillationColumn"
    }

    fn evaluate(&mut self, ctx: &EquipmentExecutionContext<'_>) -> Result<(), PhysicsError> {
        let feed_flow: f64 = ctx.inlet_flows.iter().map(|f| f.value()).sum();
        let effective_feed = if feed_flow > 0.001 { feed_flow } else { 20.0 };

        let boilup_factor = (self.boilup_duty_watts / 500_000.0).clamp(0.1, 3.0);
        let vapor_rate = effective_feed * 0.6 * boilup_factor;
        self.distillate_flow = vapor_rate / (1.0 + self.reflux_ratio);
        self.bottoms_flow = (effective_feed - self.distillate_flow).max(0.1);

        let net_accumulation = effective_feed - self.distillate_flow - self.bottoms_flow;
        self.bottom_level =
            (self.bottom_level + net_accumulation * (ctx.dt / 10_000.0)).clamp(0.0, 1.0);

        let n_stages = self.tray_count as f64;
        self.overhead_purity =
            (1.0 - (-0.08 * n_stages * self.reflux_ratio / boilup_factor).exp()).clamp(0.70, 0.999);

        self.top_temperature_c = 60.0 + 15.0 * (1.0 - self.overhead_purity);
        self.bottom_temperature_c = 105.0 + 10.0 * boilup_factor;
        Ok(())
    }

    fn get_variable(&self, name: &str) -> Option<f64> {
        match name {
            "level" => Some(self.bottom_level),
            "top_temperature" | "temperature" => Some(self.top_temperature_c),
            "bottom_temperature" => Some(self.bottom_temperature_c),
            "distillate_flow" => Some(self.distillate_flow),
            "bottoms_flow" => Some(self.bottoms_flow),
            "purity" | "conversion" => Some(self.overhead_purity),
            "reflux_ratio" => Some(self.reflux_ratio),
            "boilup_duty" | "duty" => Some(self.boilup_duty_watts),
            "top_pressure" | "pressure" => Some(self.top_pressure_pa / 100_000.0),
            _ => None,
        }
    }

    fn set_variable(&mut self, name: &str, value: f64) -> Result<(), PhysicsError> {
        match name {
            "level" => {
                self.bottom_level = value.clamp(0.0, 1.0);
                Ok(())
            }
            "reflux_ratio" => {
                self.reflux_ratio = value.clamp(0.1, 50.0);
                Ok(())
            }
            "boilup_duty" | "duty" => {
                self.boilup_duty_watts = value.max(0.0);
                Ok(())
            }
            "top_pressure" | "pressure" => {
                self.top_pressure_pa = (value * 100_000.0).max(10_000.0);
                Ok(())
            }
            _ => Err(PhysicsError::InvalidOperatingCondition {
                message: format!("unknown variable {name} for DistillationColumnModel"),
            }),
        }
    }
}

/// A generic equipment instance composed of modular physics capabilities.
pub struct GenericEquipmentPhysics {
    id: EquipmentId,
    capabilities: Vec<Box<dyn EquipmentCapability>>,
    tag_bindings: BTreeMap<String, TagId>,
    extra_variables: BTreeMap<String, f64>,
    inlet_flows: Vec<MassFlow>,
    outlet_flows: Vec<MassFlow>,
    heat_duty_watts: f64,
    shaft_work_watts: f64,
    inlet_states: Vec<PhysicalState>,
    property_package: Option<Arc<dyn PropertyPackage>>,
}

impl std::fmt::Debug for GenericEquipmentPhysics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GenericEquipmentPhysics")
            .field("id", &self.id)
            .field("capabilities_count", &self.capabilities.len())
            .field("tag_bindings", &self.tag_bindings)
            .field("extra_variables", &self.extra_variables)
            .field("inlet_flows", &self.inlet_flows)
            .field("outlet_flows", &self.outlet_flows)
            .field("heat_duty_watts", &self.heat_duty_watts)
            .field("shaft_work_watts", &self.shaft_work_watts)
            .field("inlet_states", &self.inlet_states)
            .field(
                "property_package",
                &self.property_package.as_ref().map(|p| p.name()),
            )
            .finish()
    }
}

impl GenericEquipmentPhysics {
    /// Creates a new generic equipment container with the given ID.
    pub fn new(id: EquipmentId) -> Self {
        Self {
            id,
            capabilities: Vec::new(),
            tag_bindings: BTreeMap::new(),
            extra_variables: BTreeMap::new(),
            inlet_flows: Vec::new(),
            outlet_flows: Vec::new(),
            heat_duty_watts: 0.0,
            shaft_work_watts: 0.0,
            inlet_states: Vec::new(),
            property_package: None,
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

    /// Sets upstream inlet physical states.
    pub fn set_inlet_states(&mut self, states: Vec<PhysicalState>) {
        self.inlet_states = states;
    }

    /// Sets the active thermodynamic property package.
    pub fn set_property_package(&mut self, package: Arc<dyn PropertyPackage>) {
        self.property_package = Some(package);
    }

    /// Sets the heat duty in Watts.
    pub fn set_heat_duty(&mut self, watts: f64) {
        self.heat_duty_watts = watts;
    }

    /// Sets the shaft work in Watts.
    pub fn set_shaft_work(&mut self, watts: f64) {
        self.shaft_work_watts = watts;
    }

    /// Queries a variable from the equipment by checking its capabilities in order,
    /// falling back to custom dynamic variables if not present on capabilities.
    pub fn get_variable(&self, name: &str) -> Option<f64> {
        for cap in &self.capabilities {
            if let Some(val) = cap.get_variable(name) {
                return Some(val);
            }
        }
        self.extra_variables.get(name).copied()
    }

    /// Sets a variable on the first capability that accepts it, falling back to
    /// storing the variable directly in extra_variables for arbitrary equipment state.
    pub fn set_variable(&mut self, name: &str, value: f64) -> Result<(), PhysicsError> {
        for cap in &mut self.capabilities {
            if cap.set_variable(name, value).is_ok() {
                return Ok(());
            }
        }
        self.extra_variables.insert(name.to_string(), value);
        Ok(())
    }

    /// Returns a reference to all custom dynamic variables registered on this equipment.
    pub fn extra_variables(&self) -> &BTreeMap<String, f64> {
        &self.extra_variables
    }

    /// Evaluates all capabilities belonging to this equipment over timestep $\Delta t$.
    pub fn evaluate(&mut self, dt: f64) -> Result<(), PhysicsError> {
        let ctx = EquipmentExecutionContext {
            dt,
            inlet_flows: &self.inlet_flows,
            outlet_flows: &self.outlet_flows,
            heat_duty_watts: self.heat_duty_watts,
            shaft_work_watts: self.shaft_work_watts,
            inlet_states: &self.inlet_states,
            property_package: self.property_package.as_deref(),
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
