//! Semantic and numerical validation for scenario definitions.

use crate::error::{ScenarioError, ScenarioResult};
use crate::model::*;

/// Validates a complete scenario definition prior to storage, loading, or execution.
pub fn validate_scenario(scenario: &ScenarioDefinition) -> ScenarioResult<()> {
    if scenario.id.trim().is_empty() {
        return Err(ScenarioError::Validation(
            "Scenario ID cannot be empty".into(),
        ));
    }
    if scenario.name.trim().is_empty() {
        return Err(ScenarioError::Validation(
            "Scenario name cannot be empty".into(),
        ));
    }

    if !scenario
        .schema_version
        .is_compatible_with(SCENARIO_SCHEMA_VERSION)
    {
        return Err(ScenarioError::IncompatibleVersion {
            expected: SCENARIO_SCHEMA_VERSION.to_string(),
            actual: scenario.schema_version.to_string(),
        });
    }

    if let Some(dur) = scenario.duration_limit_sec {
        if !dur.is_finite() || dur <= 0.0 {
            return Err(ScenarioError::Validation(
                "Duration limit must be positive and finite".into(),
            ));
        }
    }

    // Validate initial conditions
    validate_initial_conditions(&scenario.initial_conditions)?;

    // Validate timeline events
    for (idx, event) in scenario.timeline.iter().enumerate() {
        validate_event(event).map_err(|e| {
            ScenarioError::Validation(format!("Event #{idx} ('{}'): {e}", event.id))
        })?;
    }

    // Validate objectives
    for (idx, obj) in scenario.objectives.iter().enumerate() {
        validate_objective(obj).map_err(|e| {
            ScenarioError::Validation(format!("Objective #{idx} ('{}'): {e}", obj.id))
        })?;
    }

    Ok(())
}

fn validate_initial_conditions(ic: &InitialConditions) -> ScenarioResult<()> {
    for tag_init in &ic.tags {
        if tag_init.tag.trim().is_empty() {
            return Err(ScenarioError::Validation(
                "Initial tag name cannot be empty".into(),
            ));
        }
        if !tag_init.value.is_finite() {
            return Err(ScenarioError::Validation(format!(
                "Initial value for tag '{}' must be finite",
                tag_init.tag
            )));
        }
    }

    for ctrl in &ic.controllers {
        if ctrl.loop_name.trim().is_empty() {
            return Err(ScenarioError::Validation(
                "Controller loop name cannot be empty".into(),
            ));
        }
        if let Some(sp) = ctrl.sp {
            if !sp.is_finite() {
                return Err(ScenarioError::Validation(format!(
                    "Setpoint for loop '{}' must be finite",
                    ctrl.loop_name
                )));
            }
        }
        if let Some(out) = ctrl.manual_output {
            if !out.is_finite() {
                return Err(ScenarioError::Validation(format!(
                    "Manual output for loop '{}' must be finite",
                    ctrl.loop_name
                )));
            }
        }
    }

    for eq in &ic.equipment {
        if eq.equipment_name.trim().is_empty() {
            return Err(ScenarioError::Validation(
                "Equipment name cannot be empty".into(),
            ));
        }
        if let Some(speed) = eq.speed_rpm {
            if !speed.is_finite() || speed < 0.0 {
                return Err(ScenarioError::Validation(format!(
                    "Equipment '{}' speed must be non-negative and finite",
                    eq.equipment_name
                )));
            }
        }
    }

    Ok(())
}

fn validate_event(event: &ScenarioEvent) -> ScenarioResult<()> {
    if event.id.trim().is_empty() {
        return Err(ScenarioError::Validation("Event ID cannot be empty".into()));
    }

    match &event.trigger {
        EventTrigger::Time { elapsed_sec } => {
            if !elapsed_sec.is_finite() || *elapsed_sec < 0.0 {
                return Err(ScenarioError::Validation(
                    "Event trigger elapsed_sec must be non-negative and finite".into(),
                ));
            }
        }
        EventTrigger::TagCondition { tag, value, .. } => {
            if tag.trim().is_empty() {
                return Err(ScenarioError::Validation(
                    "Trigger tag name cannot be empty".into(),
                ));
            }
            if !value.is_finite() {
                return Err(ScenarioError::Validation(
                    "Trigger comparison value must be finite".into(),
                ));
            }
        }
    }

    match &event.action {
        ScenarioAction::InjectFault(fault) => validate_fault(fault)?,
        ScenarioAction::ClearFault { fault_id } => {
            if fault_id.trim().is_empty() {
                return Err(ScenarioError::Validation(
                    "ClearFault ID cannot be empty".into(),
                ));
            }
        }
        ScenarioAction::ApplyDisturbance(dist) => validate_disturbance(dist)?,
        ScenarioAction::ExecuteOperatorAction(action) => validate_operator_action(action)?,
        ScenarioAction::EmitMarker { label, .. } => {
            if label.trim().is_empty() {
                return Err(ScenarioError::Validation(
                    "Marker label cannot be empty".into(),
                ));
            }
        }
    }

    Ok(())
}

pub fn validate_fault(fault: &FaultDefinition) -> ScenarioResult<()> {
    if fault.id.trim().is_empty() {
        return Err(ScenarioError::Validation("Fault ID cannot be empty".into()));
    }
    if fault.target.trim().is_empty() {
        return Err(ScenarioError::Validation(
            "Fault target cannot be empty".into(),
        ));
    }

    if let Some(dur) = fault.duration_sec {
        if !dur.is_finite() || dur <= 0.0 {
            return Err(ScenarioError::Validation(
                "Fault duration must be positive and finite".into(),
            ));
        }
    }

    match &fault.fault_type {
        FaultType::SensorBias { offset } => {
            if !offset.is_finite() {
                return Err(ScenarioError::Validation(
                    "Sensor bias offset must be finite".into(),
                ));
            }
        }
        FaultType::SensorDrift { rate_per_sec } => {
            if !rate_per_sec.is_finite() {
                return Err(ScenarioError::Validation(
                    "Sensor drift rate must be finite".into(),
                ));
            }
        }
        FaultType::SensorFrozen { value } => {
            if let Some(val) = value {
                if !val.is_finite() {
                    return Err(ScenarioError::Validation(
                        "Frozen sensor value must be finite".into(),
                    ));
                }
            }
        }
        FaultType::SensorLossOfSignal => {}
        FaultType::SensorDegradation { noise_amplitude } => {
            if !noise_amplitude.is_finite() || *noise_amplitude < 0.0 {
                return Err(ScenarioError::Validation(
                    "Noise amplitude must be non-negative and finite".into(),
                ));
            }
        }
        FaultType::ActuatorStuck { position } => {
            if !position.is_finite() || *position < 0.0 || *position > 100.0 {
                return Err(ScenarioError::Validation(
                    "Actuator stuck position must be within [0.0, 100.0]".into(),
                ));
            }
        }
        FaultType::ActuatorSluggish { max_rate_per_sec } => {
            if !max_rate_per_sec.is_finite() || *max_rate_per_sec <= 0.0 {
                return Err(ScenarioError::Validation(
                    "Actuator max rate must be positive and finite".into(),
                ));
            }
        }
        FaultType::ActuatorFailSafeTrip => {}
        FaultType::EquipmentMechanicalTrip => {}
        FaultType::EquipmentEfficiencyLoss { factor } => {
            if !factor.is_finite() || *factor < 0.0 || *factor > 1.0 {
                return Err(ScenarioError::Validation(
                    "Efficiency loss factor must be within [0.0, 1.0]".into(),
                ));
            }
        }
        FaultType::ProcessBlockage {
            resistance_multiplier,
        } => {
            if !resistance_multiplier.is_finite() || *resistance_multiplier < 1.0 {
                return Err(ScenarioError::Validation(
                    "Process blockage resistance multiplier must be >= 1.0".into(),
                ));
            }
        }
        FaultType::ProcessLeak { leak_rate_kg_s } => {
            if !leak_rate_kg_s.is_finite() || *leak_rate_kg_s < 0.0 {
                return Err(ScenarioError::Validation(
                    "Leak rate must be non-negative and finite".into(),
                ));
            }
        }
    }

    Ok(())
}

fn validate_disturbance(dist: &DisturbanceDefinition) -> ScenarioResult<()> {
    if dist.id.trim().is_empty() {
        return Err(ScenarioError::Validation(
            "Disturbance ID cannot be empty".into(),
        ));
    }
    if dist.target_tag.trim().is_empty() {
        return Err(ScenarioError::Validation(
            "Disturbance target tag cannot be empty".into(),
        ));
    }

    if let Some(dur) = dist.duration_sec {
        if !dur.is_finite() || dur <= 0.0 {
            return Err(ScenarioError::Validation(
                "Disturbance duration must be positive and finite".into(),
            ));
        }
    }

    match &dist.disturbance_type {
        DisturbanceType::Step { value, .. } => {
            if !value.is_finite() {
                return Err(ScenarioError::Validation(
                    "Step value must be finite".into(),
                ));
            }
        }
        DisturbanceType::Ramp {
            target_value,
            duration_sec,
        } => {
            if !target_value.is_finite() {
                return Err(ScenarioError::Validation(
                    "Ramp target value must be finite".into(),
                ));
            }
            if !duration_sec.is_finite() || *duration_sec <= 0.0 {
                return Err(ScenarioError::Validation(
                    "Ramp duration must be positive and finite".into(),
                ));
            }
        }
        DisturbanceType::Pulse {
            value,
            pulse_width_sec,
        } => {
            if !value.is_finite() {
                return Err(ScenarioError::Validation(
                    "Pulse value must be finite".into(),
                ));
            }
            if !pulse_width_sec.is_finite() || *pulse_width_sec <= 0.0 {
                return Err(ScenarioError::Validation(
                    "Pulse width must be positive and finite".into(),
                ));
            }
        }
        DisturbanceType::Sinusoidal {
            amplitude,
            frequency_hz,
            offset,
        } => {
            if !amplitude.is_finite() || *amplitude < 0.0 {
                return Err(ScenarioError::Validation(
                    "Sinusoidal amplitude must be non-negative and finite".into(),
                ));
            }
            if !frequency_hz.is_finite() || *frequency_hz <= 0.0 {
                return Err(ScenarioError::Validation(
                    "Sinusoidal frequency must be positive and finite".into(),
                ));
            }
            if !offset.is_finite() {
                return Err(ScenarioError::Validation(
                    "Sinusoidal offset must be finite".into(),
                ));
            }
        }
        DisturbanceType::BoundedNoise { max_amplitude } => {
            if !max_amplitude.is_finite() || *max_amplitude < 0.0 {
                return Err(ScenarioError::Validation(
                    "Noise amplitude must be non-negative and finite".into(),
                ));
            }
        }
    }

    Ok(())
}

fn validate_operator_action(action: &OperatorAction) -> ScenarioResult<()> {
    match action {
        OperatorAction::SetSetpoint { loop_name, sp } => {
            if loop_name.trim().is_empty() {
                return Err(ScenarioError::Validation(
                    "Loop name cannot be empty".into(),
                ));
            }
            if !sp.is_finite() {
                return Err(ScenarioError::Validation("Setpoint must be finite".into()));
            }
        }
        OperatorAction::SetMode { loop_name, mode } => {
            if loop_name.trim().is_empty() {
                return Err(ScenarioError::Validation(
                    "Loop name cannot be empty".into(),
                ));
            }
            if mode.trim().is_empty() {
                return Err(ScenarioError::Validation("Mode cannot be empty".into()));
            }
        }
        OperatorAction::SetManualOutput { loop_name, output } => {
            if loop_name.trim().is_empty() {
                return Err(ScenarioError::Validation(
                    "Loop name cannot be empty".into(),
                ));
            }
            if !output.is_finite() {
                return Err(ScenarioError::Validation(
                    "Manual output must be finite".into(),
                ));
            }
        }
        OperatorAction::WriteTag { tag, value } => {
            if tag.trim().is_empty() {
                return Err(ScenarioError::Validation("Tag name cannot be empty".into()));
            }
            if !value.is_finite() {
                return Err(ScenarioError::Validation("Tag value must be finite".into()));
            }
        }
        OperatorAction::AcknowledgeAlarm { alarm_name } => {
            if alarm_name.trim().is_empty() {
                return Err(ScenarioError::Validation(
                    "Alarm name cannot be empty".into(),
                ));
            }
        }
        OperatorAction::ShelveAlarm {
            alarm_name,
            duration_sec,
        } => {
            if alarm_name.trim().is_empty() {
                return Err(ScenarioError::Validation(
                    "Alarm name cannot be empty".into(),
                ));
            }
            if !duration_sec.is_finite() || *duration_sec <= 0.0 {
                return Err(ScenarioError::Validation(
                    "Shelve duration must be positive and finite".into(),
                ));
            }
        }
        OperatorAction::ResetProtection => {}
    }

    Ok(())
}

fn validate_objective(obj: &ScenarioObjective) -> ScenarioResult<()> {
    if obj.id.trim().is_empty() {
        return Err(ScenarioError::Validation(
            "Objective ID cannot be empty".into(),
        ));
    }
    if obj.description.trim().is_empty() {
        return Err(ScenarioError::Validation(
            "Objective description cannot be empty".into(),
        ));
    }

    if let Some(limit) = obj.time_limit_sec {
        if !limit.is_finite() || limit <= 0.0 {
            return Err(ScenarioError::Validation(
                "Objective time limit must be positive and finite".into(),
            ));
        }
    }

    match &obj.criteria {
        ObjectiveCriteria::TagWithinRange {
            tag,
            min_val,
            max_val,
            hold_duration_sec,
        } => {
            if tag.trim().is_empty() {
                return Err(ScenarioError::Validation(
                    "Objective tag name cannot be empty".into(),
                ));
            }
            if !min_val.is_finite() || !max_val.is_finite() {
                return Err(ScenarioError::Validation(
                    "Range bounds must be finite".into(),
                ));
            }
            if min_val > max_val {
                return Err(ScenarioError::Validation(
                    "min_val cannot exceed max_val".into(),
                ));
            }
            if !hold_duration_sec.is_finite() || *hold_duration_sec < 0.0 {
                return Err(ScenarioError::Validation(
                    "Hold duration must be non-negative and finite".into(),
                ));
            }
        }
        ObjectiveCriteria::TagReachesValue {
            tag, target_val, ..
        } => {
            if tag.trim().is_empty() {
                return Err(ScenarioError::Validation(
                    "Objective tag name cannot be empty".into(),
                ));
            }
            if !target_val.is_finite() {
                return Err(ScenarioError::Validation(
                    "Target value must be finite".into(),
                ));
            }
        }
        ObjectiveCriteria::AlarmAcknowledged {
            alarm_name,
            within_sec,
        } => {
            if alarm_name.trim().is_empty() {
                return Err(ScenarioError::Validation(
                    "Alarm name cannot be empty".into(),
                ));
            }
            if !within_sec.is_finite() || *within_sec <= 0.0 {
                return Err(ScenarioError::Validation(
                    "Within seconds must be positive and finite".into(),
                ));
            }
        }
        ObjectiveCriteria::NoTripOccurred => {}
    }

    Ok(())
}
