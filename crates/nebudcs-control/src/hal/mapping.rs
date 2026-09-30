//! Controller-to-I/O mapping and arbitration rules.

use nebudcs_types::{ControllerId, EquipmentId, TagId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::actuator::CommandPriority;
use crate::hal::address::IoChannelAddress;
use crate::hal::error::HalError;

/// Direction of signal data flow relative to the virtual controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MappingDirection {
    /// Field / Sensor -> Channel -> %I -> Controller
    Input,
    /// Controller -> %Q -> Channel -> Actuator
    Output,
}

/// Binding configuration connecting a Virtual Controller tag/%I/%Q point to a physical HAL channel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControllerIoMapping {
    /// Target virtual controller instance.
    pub controller_id: ControllerId,
    /// Signal identifier within the controller's I/O image table (e.g. "%IW0", "FLOW_IN", "VALVE_CMD").
    pub signal_key: String,
    /// Canonical hardware channel address (e.g. "R01-C01-AI01").
    pub channel_address: IoChannelAddress,
    /// Direction of signal transfer.
    pub direction: MappingDirection,
    /// Optional target actuator equipment when this is an output channel.
    pub target_actuator: Option<EquipmentId>,
    /// Optional field simulation tag providing physical variable when this is an input channel.
    pub target_tag: Option<TagId>,
    /// Command arbitration priority when driving actuators.
    pub priority: CommandPriority,
}

impl ControllerIoMapping {
    /// Creates a new input mapping binding a field tag to a controller %I point via a HAL channel.
    pub fn new_input(
        controller_id: ControllerId,
        signal_key: impl Into<String>,
        channel_address: IoChannelAddress,
        target_tag: Option<TagId>,
    ) -> Self {
        Self {
            controller_id,
            signal_key: signal_key.into(),
            channel_address,
            direction: MappingDirection::Input,
            target_actuator: None,
            target_tag,
            priority: CommandPriority::Auto,
        }
    }

    /// Creates a new output mapping binding a controller %Q point to an actuator via a HAL channel.
    pub fn new_output(
        controller_id: ControllerId,
        signal_key: impl Into<String>,
        channel_address: IoChannelAddress,
        target_actuator: Option<EquipmentId>,
        priority: CommandPriority,
    ) -> Self {
        Self {
            controller_id,
            signal_key: signal_key.into(),
            channel_address,
            direction: MappingDirection::Output,
            target_actuator,
            target_tag: None,
            priority,
        }
    }
}

/// Validates an array of controller I/O mappings.
///
/// # Rules Enforced:
/// 1. Directional compatibility: AI/DI must be `Input`; AO/DO must be `Output`.
/// 2. Output Ownership: An output channel address cannot be driven by multiple controllers simultaneously
///    without explicit arbitration. Conflicting output ownership is rejected.
/// 3. Input Multicast: Multiple controllers are explicitly permitted to subscribe to the same input channel.
pub fn validate_mappings(mappings: &[ControllerIoMapping]) -> Result<(), HalError> {
    let mut output_owners: HashMap<IoChannelAddress, ControllerId> = HashMap::new();

    for m in mappings {
        // Rule 1: Direction compatibility
        match m.direction {
            MappingDirection::Input => {
                if !m.channel_address.kind.is_input() {
                    return Err(HalError::IncompatibleSignalType {
                        address: m.channel_address.to_string(),
                        expected: "Input (AI or DI)".to_string(),
                        found: m.channel_address.kind.to_string(),
                    });
                }
            }
            MappingDirection::Output => {
                if !m.channel_address.kind.is_output() {
                    return Err(HalError::IncompatibleSignalType {
                        address: m.channel_address.to_string(),
                        expected: "Output (AO or DO)".to_string(),
                        found: m.channel_address.kind.to_string(),
                    });
                }

                // Rule 2: Single controller output ownership check
                if let Some(existing_ctrl) = output_owners.get(&m.channel_address) {
                    if *existing_ctrl != m.controller_id {
                        return Err(HalError::ConflictingOutputOwnership {
                            address: m.channel_address.to_string(),
                            existing_controller: existing_ctrl.to_string(),
                            new_controller: m.controller_id.to_string(),
                        });
                    }
                } else {
                    output_owners.insert(m.channel_address, m.controller_id);
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hal::address::ChannelKind;

    #[test]
    fn test_valid_multicast_inputs_allowed() {
        let ctrl_1 = ControllerId::new();
        let ctrl_2 = ControllerId::new();
        let addr = IoChannelAddress::compact(1, 1, 1, ChannelKind::AI);

        let mappings = vec![
            ControllerIoMapping::new_input(ctrl_1, "PV", addr, None),
            ControllerIoMapping::new_input(ctrl_2, "PV_COPY", addr, None),
        ];

        assert!(validate_mappings(&mappings).is_ok());
    }

    #[test]
    fn test_conflicting_output_ownership_rejected() {
        let ctrl_1 = ControllerId::new();
        let ctrl_2 = ControllerId::new();
        let addr = IoChannelAddress::compact(1, 1, 1, ChannelKind::AO);

        let mappings = vec![
            ControllerIoMapping::new_output(ctrl_1, "CV", addr, None, CommandPriority::Auto),
            ControllerIoMapping::new_output(ctrl_2, "CV", addr, None, CommandPriority::Auto),
        ];

        let err = validate_mappings(&mappings).unwrap_err();
        assert!(matches!(err, HalError::ConflictingOutputOwnership { .. }));
    }

    #[test]
    fn test_direction_mismatch_rejected() {
        let ctrl_1 = ControllerId::new();
        let ai_addr = IoChannelAddress::compact(1, 1, 1, ChannelKind::AI);

        let mappings = vec![ControllerIoMapping::new_output(
            ctrl_1,
            "CV",
            ai_addr,
            None,
            CommandPriority::Auto,
        )];

        let err = validate_mappings(&mappings).unwrap_err();
        assert!(matches!(err, HalError::IncompatibleSignalType { .. }));
    }
}
