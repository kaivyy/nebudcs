//! Advanced data-driven compressor map physics, surge dynamics, and thermodynamic enthalpy coupling.
//!
//! Provides [`CompressorMapModel`], implementing [`crate::equipment::EquipmentCapability`]
//! with multi-dimensional speed lines, surge/choke margin tracking, and rigorous thermodynamic
//! shaft power evaluation.

use serde::{Deserialize, Serialize};

use nebudcs_units::{Pressure, Temperature};

use crate::equipment::{EquipmentCapability, EquipmentExecutionContext};
use crate::error::PhysicsError;
use crate::properties::UNIVERSAL_GAS_CONSTANT;
use crate::thermo::component::ComponentDatabase;

/// Operating regime classification of a compressor on its performance map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompressorOperatingState {
    /// Normal stable operating zone well away from surge and choke.
    Normal,
    /// Operating near the surge line (surge margin < 10%).
    NearSurge,
    /// Operating at or beyond the surge line (flow below minimum stable flow).
    Surge,
    /// Operating near the choke limit (flow > 90% of choke capacity).
    NearChoke,
    /// Operating at sonic stone wall / aerodynamic choke limit.
    Choke,
    /// Operating in an extrapolated region outside validated map points.
    MapExtrapolation,
    /// Operating condition invalid (e.g. non-positive speed or pressure).
    Invalid,
}

/// A discrete performance operating point on a constant-speed line.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CompressorMapPoint {
    /// Mass flow rate $\dot{m}$ in kg/s.
    pub mass_flow: f64,
    /// Total-to-total pressure ratio $r_p = P_{out} / P_{in}$.
    pub pressure_ratio: f64,
    /// Isentropic efficiency $\eta_{is} \in (0.0, 1.0]$.
    pub isentropic_efficiency: f64,
}

/// A constant-speed operating curve consisting of ordered performance points from surge to choke.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompressorSpeedLine {
    /// Relative rotational speed fraction ($N / N_{rated}$, typically 0.5 to 1.15).
    pub speed_fraction: f64,
    /// Surge limit flow at this speed in kg/s.
    pub surge_flow: f64,
    /// Choke limit flow at this speed in kg/s.
    pub choke_flow: f64,
    /// Ordered map points from lowest flow (surge) to highest flow (choke).
    pub points: Vec<CompressorMapPoint>,
}

impl CompressorSpeedLine {
    /// Creates and validates a speed line.
    pub fn new(
        speed_fraction: f64,
        surge_flow: f64,
        choke_flow: f64,
        points: Vec<CompressorMapPoint>,
    ) -> Result<Self, PhysicsError> {
        if speed_fraction <= 0.0 || !speed_fraction.is_finite() {
            return Err(PhysicsError::InvalidOperatingCondition {
                message: format!("speed_fraction must be positive: {speed_fraction}"),
            });
        }
        if surge_flow >= choke_flow || surge_flow <= 0.0 {
            return Err(PhysicsError::InvalidOperatingCondition {
                message: format!("invalid flow limits: surge {surge_flow} >= choke {choke_flow}"),
            });
        }
        if points.len() < 2 {
            return Err(PhysicsError::InvalidOperatingCondition {
                message: "speed line must contain at least 2 points".to_string(),
            });
        }

        // Validate flow ordering
        for i in 1..points.len() {
            if points[i].mass_flow <= points[i - 1].mass_flow {
                return Err(PhysicsError::InvalidOperatingCondition {
                    message: format!(
                        "compressor map points must be strictly increasing in mass flow: {} <= {}",
                        points[i].mass_flow,
                        points[i - 1].mass_flow
                    ),
                });
            }
        }

        Ok(Self {
            speed_fraction,
            surge_flow,
            choke_flow,
            points,
        })
    }

    /// Interpolates pressure ratio and efficiency along this speed line for a given flow.
    pub fn interpolate(&self, flow: f64) -> (f64, f64, CompressorOperatingState) {
        if flow <= 0.0 || !flow.is_finite() {
            return (1.0, 0.5, CompressorOperatingState::Invalid);
        }

        // Check surge and choke states
        let state = if flow < self.surge_flow {
            CompressorOperatingState::Surge
        } else if flow < self.surge_flow * 1.10 {
            CompressorOperatingState::NearSurge
        } else if flow > self.choke_flow {
            CompressorOperatingState::Choke
        } else if flow > self.choke_flow * 0.90 {
            CompressorOperatingState::NearChoke
        } else {
            CompressorOperatingState::Normal
        };

        // Linear interpolation between points
        if flow <= self.points[0].mass_flow {
            let p0 = &self.points[0];
            return (p0.pressure_ratio, p0.isentropic_efficiency, state);
        }

        let last_idx = self.points.len() - 1;
        if flow >= self.points[last_idx].mass_flow {
            let plast = &self.points[last_idx];
            return (plast.pressure_ratio, plast.isentropic_efficiency, state);
        }

        for i in 0..last_idx {
            let p1 = &self.points[i];
            let p2 = &self.points[i + 1];
            if flow >= p1.mass_flow && flow <= p2.mass_flow {
                let frac = (flow - p1.mass_flow) / (p2.mass_flow - p1.mass_flow).max(1e-9);
                let pr = p1.pressure_ratio + frac * (p2.pressure_ratio - p1.pressure_ratio);
                let eff = p1.isentropic_efficiency
                    + frac * (p2.isentropic_efficiency - p1.isentropic_efficiency);
                return (pr, eff, state);
            }
        }

        let plast = &self.points[last_idx];
        (plast.pressure_ratio, plast.isentropic_efficiency, state)
    }
}

/// Advanced compressor performance map model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompressorMapModel {
    /// Rated rotational speed in RPM.
    pub rated_rpm: f64,
    /// Rated inlet volume or mass flow in kg/s.
    pub rated_mass_flow: f64,
    /// Set of speed lines defining the performance characteristics.
    pub speed_lines: Vec<CompressorSpeedLine>,
    /// Current rotational speed fraction $N / N_{rated}$.
    pub current_speed_fraction: f64,
    /// Current inlet pressure in Pascal.
    pub inlet_pressure: Pressure,
    /// Current outlet pressure in Pascal.
    pub outlet_pressure: Pressure,
    /// Last calculated pressure ratio.
    pub last_pressure_ratio: f64,
    /// Last calculated isentropic efficiency.
    pub last_efficiency: f64,
    /// Last evaluated surge margin ($(\dot{m} - \dot{m}_{surge}) / \dot{m}_{surge}$).
    pub last_surge_margin: f64,
    /// Last calculated shaft power in Watts.
    pub last_power_watts: f64,
    /// Current operating regime.
    pub last_operating_state: CompressorOperatingState,
}

impl CompressorMapModel {
    /// Creates a generic compressor map model with default calibrated speed lines.
    pub fn new_standard(rated_rpm: f64, rated_mass_flow: f64) -> Self {
        // Construct standard speed lines for 70%, 80%, 90%, 100%, 105% speed
        let mut speed_lines = Vec::new();
        let speeds = [0.70, 0.80, 0.90, 1.00, 1.05];

        for &spd in &speeds {
            let surge_flow = rated_mass_flow * spd * 0.45;
            let choke_flow = rated_mass_flow * spd * 1.35;

            let n_pts = 6;
            let mut points = Vec::with_capacity(n_pts);
            for i in 0..n_pts {
                let frac = i as f64 / (n_pts - 1) as f64;
                let flow = surge_flow + frac * (choke_flow - surge_flow);
                // Drooping pressure ratio curve
                let pr_max = 1.0 + 2.5 * spd * spd;
                let pr = pr_max - 1.2 * spd * (frac * frac);
                // Peak efficiency near mid-flow
                let eff = 0.82 - 0.25 * (frac - 0.5).powi(2);

                points.push(CompressorMapPoint {
                    mass_flow: flow,
                    pressure_ratio: pr.max(1.01),
                    isentropic_efficiency: eff.clamp(0.40, 0.88),
                });
            }

            let sl = CompressorSpeedLine::new(spd, surge_flow, choke_flow, points)
                .expect("valid speed line");
            speed_lines.push(sl);
        }

        Self {
            rated_rpm,
            rated_mass_flow,
            speed_lines,
            current_speed_fraction: 1.0,
            inlet_pressure: Pressure::new_unchecked(101_325.0),
            outlet_pressure: Pressure::new_unchecked(250_000.0),
            last_pressure_ratio: 2.5,
            last_efficiency: 0.80,
            last_surge_margin: 0.35,
            last_power_watts: 0.0,
            last_operating_state: CompressorOperatingState::Normal,
        }
    }

    /// Evaluates operating state, pressure ratio, efficiency, and surge margin at given flow and speed.
    pub fn evaluate_performance(&mut self, mass_flow: f64, speed_fraction: f64) {
        let spd = speed_fraction.clamp(0.1, 1.5);
        let flow = mass_flow.max(1e-6);

        if self.speed_lines.is_empty() {
            self.last_operating_state = CompressorOperatingState::Invalid;
            return;
        }

        // Find surrounding speed lines
        let mut lower_idx = 0;
        let mut upper_idx = self.speed_lines.len() - 1;

        for i in 0..self.speed_lines.len() {
            if self.speed_lines[i].speed_fraction <= spd {
                lower_idx = i;
            }
            if self.speed_lines[i].speed_fraction >= spd {
                upper_idx = i;
                break;
            }
        }

        let (pr, eff, state, surge_m) = if lower_idx == upper_idx {
            let sl = &self.speed_lines[lower_idx];
            let (pr, eff, state) = sl.interpolate(flow);
            let sm = (flow - sl.surge_flow) / sl.surge_flow.max(1e-6);
            (pr, eff, state, sm)
        } else {
            let sl1 = &self.speed_lines[lower_idx];
            let sl2 = &self.speed_lines[upper_idx];
            let w =
                (spd - sl1.speed_fraction) / (sl2.speed_fraction - sl1.speed_fraction).max(1e-6);

            let (pr1, eff1, state1) = sl1.interpolate(flow);
            let (pr2, eff2, state2) = sl2.interpolate(flow);

            let sm1 = (flow - sl1.surge_flow) / sl1.surge_flow.max(1e-6);
            let sm2 = (flow - sl2.surge_flow) / sl2.surge_flow.max(1e-6);

            let pr = pr1 + w * (pr2 - pr1);
            let eff = eff1 + w * (eff2 - eff1);
            let sm = sm1 + w * (sm2 - sm1);

            let state = if state1 == CompressorOperatingState::Surge
                || state2 == CompressorOperatingState::Surge
            {
                CompressorOperatingState::Surge
            } else if state1 == CompressorOperatingState::NearSurge
                || state2 == CompressorOperatingState::NearSurge
            {
                CompressorOperatingState::NearSurge
            } else if state1 == CompressorOperatingState::Choke
                || state2 == CompressorOperatingState::Choke
            {
                CompressorOperatingState::Choke
            } else {
                CompressorOperatingState::Normal
            };

            (pr, eff, state, sm)
        };

        self.last_pressure_ratio = pr;
        self.last_efficiency = eff;
        self.last_operating_state = state;
        self.last_surge_margin = surge_m;
        self.outlet_pressure = Pressure::new_unchecked(self.inlet_pressure.value() * pr);
    }

    /// Calculates rigorous shaft power $W_s = \frac{\dot{m} \Delta h_{is}}{\eta_{is}}$
    /// using fluid heat capacities and gas constant.
    pub fn calculate_shaft_power(
        &mut self,
        mass_flow: f64,
        cp_over_cv: f64,
        gas_constant_spec: f64,
        t_inlet_k: f64,
    ) -> f64 {
        if mass_flow <= 1e-6 || self.last_efficiency <= 1e-6 {
            self.last_power_watts = 0.0;
            return 0.0;
        }

        let k = cp_over_cv.clamp(1.1, 1.67);
        let pr = self.last_pressure_ratio.max(1.0);
        let eff = self.last_efficiency.clamp(0.1, 1.0);

        // Isentropic head: H_is = [k / (k-1)] * R_spec * T_in * [ (P_out/P_in)^((k-1)/k) - 1 ]
        let k_factor = k / (k - 1.0);
        let pr_exp = pr.powf((k - 1.0) / k);
        let head_is = k_factor * gas_constant_spec * t_inlet_k * (pr_exp - 1.0);

        let power = (mass_flow * head_is) / eff;
        self.last_power_watts = power.max(0.0);
        self.last_power_watts
    }
}

impl EquipmentCapability for CompressorMapModel {
    fn capability_name(&self) -> &str {
        "CompressorMap"
    }

    fn evaluate(&mut self, ctx: &EquipmentExecutionContext<'_>) -> Result<(), PhysicsError> {
        let total_flow: f64 = ctx.inlet_flows.iter().map(|f| f.value()).sum();
        self.evaluate_performance(total_flow, self.current_speed_fraction);

        if total_flow <= 1e-6 {
            self.last_power_watts = 0.0;
            return Ok(());
        }

        // F-05: Derive thermodynamic properties rigorously from upstream stream state / property package
        if ctx.inlet_states.is_empty() {
            return Err(PhysicsError::MissingProperty {
                name:
                    "inlet_stream_state (required for rigorous compressor shaft power calculation)"
                        .to_string(),
            });
        }

        let inlet_state = &ctx.inlet_states[0];
        let t_in = inlet_state.temperature().value();
        let p_in = inlet_state.pressure().value();
        let mw = inlet_state.molecular_weight(); // in kg/mol

        if mw <= 0.0 || !mw.is_finite() {
            return Err(PhysicsError::InvalidComposition {
                reason: format!("invalid inlet molecular weight for compressor: {mw}"),
            });
        }
        if t_in <= 0.0 || !t_in.is_finite() {
            return Err(PhysicsError::NonPositiveTemperature {
                value: t_in,
                context: "compressor inlet stream temperature".to_string(),
            });
        }

        // Specific gas constant: R_spec = R_universal / M_w in J/(kg*K)
        let r_spec = UNIVERSAL_GAS_CONSTANT / mw;

        // Synchronize inlet and outlet pressures based on actual upstream stream pressure
        self.inlet_pressure = inlet_state.pressure();
        self.outlet_pressure = Pressure::new_unchecked(p_in * self.last_pressure_ratio);

        // Derive Cp (specific isobaric heat capacity in J/(kg*K))
        let cp = if let Some(pkg) = ctx.property_package {
            let t_plus = Temperature::new(t_in + 1.0).map_err(|e| {
                PhysicsError::InvalidOperatingCondition {
                    message: e.to_string(),
                }
            })?;
            let t_minus = Temperature::new(t_in - 1.0).map_err(|e| {
                PhysicsError::InvalidOperatingCondition {
                    message: e.to_string(),
                }
            })?;
            let h_plus = pkg
                .specific_enthalpy(inlet_state.pressure(), t_plus, inlet_state.composition())?
                .value();
            let h_minus = pkg
                .specific_enthalpy(inlet_state.pressure(), t_minus, inlet_state.composition())?
                .value();
            (h_plus - h_minus) / 2.0
        } else {
            // Compute from component database standard library
            let db = ComponentDatabase::standard_library();
            let mut cp_molar_mix = 0.0;
            for (name, &frac) in inlet_state.composition().fractions() {
                let comp = db.get(name).ok_or_else(|| PhysicsError::MissingProperty {
                    name: format!("thermodynamic Cp correlation or property package for component '{name}' in compressor evaluation"),
                })?;
                cp_molar_mix += frac * comp.cp_ideal(t_in);
            }
            // Specific Cp = Molar Cp / M_w (J/(kg*K))
            cp_molar_mix / mw
        };

        let cv = cp - r_spec;
        if cv <= 1e-3 {
            return Err(PhysicsError::InvalidOperatingCondition {
                message: format!("computed non-physical heat capacity difference: Cp={cp}, R_spec={r_spec}, Cv={cv}"),
            });
        }

        let k = cp / cv;
        self.calculate_shaft_power(total_flow, k, r_spec, t_in);
        Ok(())
    }

    fn get_variable(&self, name: &str) -> Option<f64> {
        match name {
            "speed" | "speed_fraction" => Some(self.current_speed_fraction),
            "pressure_ratio" => Some(self.last_pressure_ratio),
            "efficiency" => Some(self.last_efficiency),
            "surge_margin" => Some(self.last_surge_margin),
            "power" | "shaft_power" => Some(self.last_power_watts),
            "outlet_pressure" => Some(self.outlet_pressure.value()),
            "inlet_pressure" => Some(self.inlet_pressure.value()),
            "is_surging" => Some(
                if self.last_operating_state == CompressorOperatingState::Surge {
                    1.0
                } else {
                    0.0
                },
            ),
            "is_choked" => Some(
                if self.last_operating_state == CompressorOperatingState::Choke {
                    1.0
                } else {
                    0.0
                },
            ),
            _ => None,
        }
    }

    fn set_variable(&mut self, name: &str, value: f64) -> Result<(), PhysicsError> {
        match name {
            "speed" | "speed_fraction" => {
                self.current_speed_fraction = value.clamp(0.0, 2.0);
                Ok(())
            }
            "inlet_pressure" => {
                self.inlet_pressure =
                    Pressure::new(value).map_err(|e| PhysicsError::NegativePressure {
                        value,
                        context: e.to_string(),
                    })?;
                Ok(())
            }
            _ => Err(PhysicsError::InvalidOperatingCondition {
                message: format!("unknown variable '{name}' on CompressorMapModel"),
            }),
        }
    }
}
