//! Advanced pump cavitation, Net Positive Suction Head (NPSH), and hydraulic degradation models.
//!
//! Provides [`PumpCavitationModel`], implementing [`crate::equipment::EquipmentCapability`]
//! to dynamically evaluate NPSHA, NPSHR, cavitation margin, head degradation, and vibration.

use serde::{Deserialize, Serialize};

use nebudcs_units::{Density, Pressure};

use crate::equipment::{EquipmentCapability, EquipmentExecutionContext, STANDARD_GRAVITY};
use crate::error::PhysicsError;

/// Cavitation regime classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CavitationRegime {
    /// Safe operating regime: NPSHA exceeds NPSHR with comfortable margin (> 1.0 m).
    Safe,
    /// Incipient cavitation: NPSHA is close to NPSHR (0.0 < margin <= 1.0 m).
    Incipient,
    /// Severe cavitation: NPSHA < NPSHR, causing acoustic noise, vibration, and head drop.
    Cavitation,
}

/// Advanced pump hydraulic model with dynamic NPSH and cavitation degradation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PumpCavitationModel {
    /// Rated pump flow in m³/s.
    pub rated_flow_m3_s: f64,
    /// Rated pump speed in RPM.
    pub rated_rpm: f64,
    /// Rated NPSH required ($NPSH_{R, rated}$) in meters of liquid head at rated conditions.
    pub rated_npshr_m: f64,
    /// Baseline vibration level (RMS) in mm/s.
    pub baseline_vibration_mm_s: f64,
    /// Suction static nozzle elevation in meters relative to reference.
    pub suction_elevation_m: f64,
    /// Suction line piping hydraulic loss head in meters.
    pub suction_loss_head_m: f64,

    /// Current relative rotational speed fraction $N / N_{rated}$.
    pub current_speed_fraction: f64,
    /// Current suction static pressure in Pascal.
    pub suction_pressure: Pressure,
    /// Fluid vapor pressure at pumping temperature in Pascal.
    pub fluid_vapor_pressure: Pressure,
    /// Fluid density in kg/m³.
    pub fluid_density: Density,

    /// Last calculated Net Positive Suction Head Available ($NPSH_A$) in meters.
    pub last_npsha_m: f64,
    /// Last calculated Net Positive Suction Head Required ($NPSH_R$) in meters.
    pub last_npshr_m: f64,
    /// Last calculated cavitation margin ($NPSH_A - NPSH_R$) in meters.
    pub last_cavitation_margin_m: f64,
    /// Head degradation multiplier $\psi \in [0.0, 1.0]$ applied to discharge head.
    pub last_head_degradation: f64,
    /// Dynamic vibration velocity in mm/s (increases sharply under cavitation).
    pub last_vibration_mm_s: f64,
    /// Current cavitation regime.
    pub last_regime: CavitationRegime,
}

impl PumpCavitationModel {
    /// Creates a pump cavitation model with rated parameters.
    pub fn new(
        rated_flow_m3_s: f64,
        rated_rpm: f64,
        rated_npshr_m: f64,
        fluid_density: f64,
    ) -> Result<Self, PhysicsError> {
        if rated_flow_m3_s <= 0.0 || rated_rpm <= 0.0 || rated_npshr_m <= 0.0 {
            return Err(PhysicsError::InvalidOperatingCondition {
                message: "pump rated parameters must be strictly positive".to_string(),
            });
        }

        let density = Density::new(fluid_density).map_err(|_| PhysicsError::InvalidDensity {
            value: fluid_density,
            context: "PumpCavitationModel::new".to_string(),
        })?;

        Ok(Self {
            rated_flow_m3_s,
            rated_rpm,
            rated_npshr_m,
            baseline_vibration_mm_s: 1.5,
            suction_elevation_m: 0.0,
            suction_loss_head_m: 0.0,
            current_speed_fraction: 1.0,
            suction_pressure: Pressure::new_unchecked(101_325.0),
            fluid_vapor_pressure: Pressure::new_unchecked(3_169.0), // Water at 25 °C
            fluid_density: density,
            last_npsha_m: 10.0,
            last_npshr_m: rated_npshr_m,
            last_cavitation_margin_m: 10.0 - rated_npshr_m,
            last_head_degradation: 1.0,
            last_vibration_mm_s: 1.5,
            last_regime: CavitationRegime::Safe,
        })
    }

    /// Evaluates Net Positive Suction Head Available ($NPSH_A$) in meters:
    ///
    /// $NPSH_A = \frac{P_{suction} - P_{vapor}}{\rho \cdot g} + z_{suction} - h_{losses} + \frac{v_{suction}^2}{2g}$
    pub fn calculate_npsha(&self, suction_velocity_m_s: f64) -> f64 {
        let rho = self.fluid_density.value().max(1.0);
        let g = STANDARD_GRAVITY;

        let delta_p = self.suction_pressure.value() - self.fluid_vapor_pressure.value();
        let static_head = delta_p / (rho * g);
        let vel_head = (suction_velocity_m_s * suction_velocity_m_s) / (2.0 * g);

        let npsha = static_head + self.suction_elevation_m - self.suction_loss_head_m + vel_head;
        npsha.max(0.0)
    }

    /// Evaluates Net Positive Suction Head Required ($NPSH_R$) in meters:
    ///
    /// $NPSH_R(Q, N) = NPSH_{R, rated} \cdot \left(\frac{Q}{Q_{rated}}\right)^2 \cdot \left(\frac{N}{N_{rated}}\right)^2$
    pub fn calculate_npshr(&self, volumetric_flow_m3_s: f64, speed_fraction: f64) -> f64 {
        let q_ratio = (volumetric_flow_m3_s / self.rated_flow_m3_s.max(1e-6)).max(0.0);
        let n_ratio = speed_fraction.clamp(0.1, 2.0);

        // Standard hydraulic scaling: NPSHR scales approximately with flow squared and speed squared
        let npshr = self.rated_npshr_m * (0.2 + 0.8 * q_ratio * q_ratio) * (n_ratio * n_ratio);
        npshr.max(0.1)
    }

    /// Evaluates dynamic cavitation metrics given mass flow and speed fraction.
    pub fn evaluate_cavitation(&mut self, mass_flow: f64, speed_fraction: f64) {
        let rho = self.fluid_density.value().max(1.0);
        let q_m3_s = (mass_flow / rho).max(0.0);

        // Approximate suction nozzle area based on rated flow at 2 m/s
        let suction_area = (self.rated_flow_m3_s / 2.0).max(1e-4);
        let suction_velocity = q_m3_s / suction_area;

        let npsha = self.calculate_npsha(suction_velocity);
        let npshr = self.calculate_npshr(q_m3_s, speed_fraction);
        let margin = npsha - npshr;

        self.last_npsha_m = npsha;
        self.last_npshr_m = npshr;
        self.last_cavitation_margin_m = margin;

        if margin > 1.0 {
            self.last_regime = CavitationRegime::Safe;
            self.last_head_degradation = 1.0;
            self.last_vibration_mm_s = self.baseline_vibration_mm_s;
        } else if margin > 0.0 {
            self.last_regime = CavitationRegime::Incipient;
            // Mild degradation: 1.0 down to 0.97 (3% drop is standard definition of NPSHR)
            self.last_head_degradation = 0.97 + 0.03 * margin;
            self.last_vibration_mm_s = self.baseline_vibration_mm_s * (1.0 + 1.5 * (1.0 - margin));
        } else {
            self.last_regime = CavitationRegime::Cavitation;
            // Severe degradation when margin is negative: head drops toward zero
            let severe_deficit = (-margin).min(npshr);
            self.last_head_degradation =
                (0.97 - 0.90 * (severe_deficit / npshr.max(0.1))).max(0.05);
            self.last_vibration_mm_s =
                self.baseline_vibration_mm_s * (2.5 + 4.0 * (severe_deficit / npshr.max(0.1)));
        }
    }
}

impl EquipmentCapability for PumpCavitationModel {
    fn capability_name(&self) -> &str {
        "PumpCavitation"
    }

    fn evaluate(&mut self, ctx: &EquipmentExecutionContext<'_>) -> Result<(), PhysicsError> {
        let total_flow: f64 = ctx.inlet_flows.iter().map(|f| f.value()).sum();
        self.evaluate_cavitation(total_flow, self.current_speed_fraction);
        Ok(())
    }

    fn get_variable(&self, name: &str) -> Option<f64> {
        match name {
            "npsha" | "npsh_a" => Some(self.last_npsha_m),
            "npshr" | "npsh_r" => Some(self.last_npshr_m),
            "cavitation_margin" => Some(self.last_cavitation_margin_m),
            "head_degradation" => Some(self.last_head_degradation),
            "vibration" => Some(self.last_vibration_mm_s),
            "is_cavitating" => Some(if self.last_regime == CavitationRegime::Cavitation {
                1.0
            } else {
                0.0
            }),
            "speed" | "speed_fraction" => Some(self.current_speed_fraction),
            "suction_pressure" => Some(self.suction_pressure.value()),
            "vapor_pressure" => Some(self.fluid_vapor_pressure.value()),
            _ => None,
        }
    }

    fn set_variable(&mut self, name: &str, value: f64) -> Result<(), PhysicsError> {
        match name {
            "speed" | "speed_fraction" => {
                self.current_speed_fraction = value.clamp(0.0, 2.0);
                Ok(())
            }
            "suction_pressure" => {
                self.suction_pressure =
                    Pressure::new(value).map_err(|e| PhysicsError::NegativePressure {
                        value,
                        context: e.to_string(),
                    })?;
                Ok(())
            }
            "vapor_pressure" => {
                self.fluid_vapor_pressure =
                    Pressure::new(value).map_err(|e| PhysicsError::NegativePressure {
                        value,
                        context: e.to_string(),
                    })?;
                Ok(())
            }
            _ => Err(PhysicsError::InvalidOperatingCondition {
                message: format!("unknown variable '{name}' on PumpCavitationModel"),
            }),
        }
    }
}
