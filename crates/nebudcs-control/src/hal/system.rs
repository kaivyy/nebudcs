use nebudcs_model::tag::TagValue;
use nebudcs_sim::state::{StateReadView, StateWriteView};
use nebudcs_types::{ControllerId, EquipmentId, IoSystemId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::actuator::ActuatorCommand;
use crate::hal::address::IoChannelAddress;
use crate::hal::channel::IoChannel;
use crate::hal::error::HalError;
use crate::hal::mapping::{validate_mappings, ControllerIoMapping, MappingDirection};
use crate::hal::rack::IoRack;
use crate::hal::signal::IoSignalQuality;
use crate::runtime::controller::VirtualController;

/// High-resolution diagnostic counters and performance profiling for the HAL.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HalDiagnostics {
    /// Total number of channel samples dispatched across simulation steps.
    pub channels_scanned: u64,
    /// Number of channels currently in a faulted state (wire-break, short-circuit, etc.).
    pub faulted_channels: usize,
    /// Total number of rack power loss incidents detected.
    pub power_faults: usize,
    /// Total number of backplane communication bus failures detected.
    pub bus_faults: usize,
    /// Duration of the most recent I/O scan in microseconds.
    pub last_scan_duration_us: u64,
    /// Exponential moving average of I/O scan duration in microseconds.
    pub avg_scan_duration_us: f64,
    /// Worst-case peak I/O scan duration in microseconds.
    pub max_scan_duration_us: u64,
}

/// The top-level I/O Subsystem managing physical hardware hierarchy and controller mappings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IoSystem {
    pub id: IoSystemId,
    pub name: String,
    pub racks: BTreeMap<u8, IoRack>,
    pub mappings: Vec<ControllerIoMapping>,
    pub diagnostics: HalDiagnostics,
}

impl Default for IoSystem {
    fn default() -> Self {
        Self::new("Default-HAL")
    }
}

impl IoSystem {
    /// Creates a new I/O System with a unique ID.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: IoSystemId::new(),
            name: name.into(),
            racks: BTreeMap::new(),
            mappings: Vec::new(),
            diagnostics: HalDiagnostics::default(),
        }
    }

    /// Creates a new I/O System with explicit ID.
    pub fn with_id(id: IoSystemId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            racks: BTreeMap::new(),
            mappings: Vec::new(),
            diagnostics: HalDiagnostics::default(),
        }
    }

    /// Adds a rack to this I/O system.
    pub fn add_rack(&mut self, rack: IoRack) -> Result<(), HalError> {
        let num = rack.rack_number;
        if self.racks.contains_key(&num) {
            return Err(HalError::DuplicateAddress {
                address: format!("Rack {}", num),
            });
        }
        self.racks.insert(num, rack);
        Ok(())
    }

    /// Accesses an immutable rack by number.
    pub fn rack(&self, num: u8) -> Option<&IoRack> {
        self.racks.get(&num)
    }

    /// Accesses a mutable rack by number.
    pub fn rack_mut(&mut self, num: u8) -> Option<&mut IoRack> {
        self.racks.get_mut(&num)
    }

    /// Accesses an immutable channel by structured address.
    pub fn channel(&self, addr: &IoChannelAddress) -> Option<&IoChannel> {
        self.racks
            .get(&addr.rack)?
            .chassis
            .get(&addr.chassis)?
            .modules
            .get(&addr.module)?
            .channel(addr.channel)
    }

    /// Accesses a mutable channel by structured address.
    pub fn channel_mut(&mut self, addr: &IoChannelAddress) -> Option<&mut IoChannel> {
        self.racks
            .get_mut(&addr.rack)?
            .chassis
            .get_mut(&addr.chassis)?
            .modules
            .get_mut(&addr.module)?
            .channel_mut(addr.channel)
    }

    /// Accesses an immutable channel by string address (e.g. "R01-C01-AI01").
    pub fn channel_by_str(&self, s: &str) -> Option<&IoChannel> {
        let addr: IoChannelAddress = s.parse().ok()?;
        self.channel(&addr)
    }

    /// Accesses a mutable channel by string address.
    pub fn channel_mut_by_str(&mut self, s: &str) -> Option<&mut IoChannel> {
        let addr: IoChannelAddress = s.parse().ok()?;
        self.channel_mut(&addr)
    }

    /// Registers a controller-to-I/O mapping, performing integrity and conflict validation.
    pub fn add_mapping(&mut self, mapping: ControllerIoMapping) -> Result<(), HalError> {
        // 1. Verify that the target channel actually exists
        if self.channel(&mapping.channel_address).is_none() {
            return Err(HalError::ChannelNotFound {
                address: mapping.channel_address.to_string(),
            });
        }

        // 2. Validate proposed mapping set
        let mut test_mappings = self.mappings.clone();
        test_mappings.push(mapping.clone());
        validate_mappings(&test_mappings)?;

        self.mappings.push(mapping);
        Ok(())
    }

    /// Authoritative validation of the full I/O hardware hierarchy and mappings.
    pub fn validate(&self) -> Result<(), HalError> {
        // Validate mappings
        validate_mappings(&self.mappings)?;

        // Ensure all mapped channels exist in the hardware tree
        for m in &self.mappings {
            if self.channel(&m.channel_address).is_none() {
                return Err(HalError::ChannelNotFound {
                    address: m.channel_address.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Samples field variables from simulation state, applies signal conditioning,
    /// updates HAL channel states, and commits values into controller `%I` tables.
    pub fn sample_inputs(
        &mut self,
        read_state: &StateReadView<'_>,
        write_state: &mut StateWriteView<'_>,
        controllers: &mut BTreeMap<ControllerId, VirtualController>,
        sim_time: f64,
    ) {
        let start_time = std::time::Instant::now();
        let mut scanned_count = 0u64;
        let mut faulted_count = 0usize;
        let mappings = self.mappings.clone();
        for mapping in &mappings {
            if mapping.direction != MappingDirection::Input {
                continue;
            }

            let addr = mapping.channel_address;

            // Check hardware health status along the hierarchy path
            let rack_ok = self
                .racks
                .get(&addr.rack)
                .map(|r| r.power_ok)
                .unwrap_or(false);
            let bus_ok = self
                .racks
                .get(&addr.rack)
                .and_then(|r| r.chassis.get(&addr.chassis))
                .map(|c| c.bus_ok)
                .unwrap_or(false);

            if let Some(channel) = self.channel_mut(&addr) {
                scanned_count += 1;

                if !rack_ok || !bus_ok {
                    // Power or backplane failure
                    match channel {
                        IoChannel::Ai(c) => {
                            c.quality = IoSignalQuality::CommunicationFault;
                            c.last_updated_sim = sim_time;
                        }
                        IoChannel::Di(c) => {
                            c.quality = IoSignalQuality::CommunicationFault;
                            c.last_updated_sim = sim_time;
                        }
                        _ => {}
                    }
                    faulted_count += 1;
                } else if let Some(tag_id) = mapping.target_tag {
                    // Read physical variable from simulation state
                    let field_val = match write_state.get_tag(tag_id) {
                        Some(TagValue::Float(v)) => *v,
                        _ => match read_state.tag_value(tag_id) {
                            Ok(TagValue::Float(v)) => *v,
                            _ => 0.0,
                        },
                    };

                    match channel {
                        IoChannel::Ai(c) => {
                            c.sample(field_val, sim_time);
                            if c.quality.is_bad() {
                                faulted_count += 1;
                            }
                        }
                        IoChannel::Di(c) => {
                            c.sample(field_val >= 0.5, sim_time);
                            if c.quality.is_bad() {
                                faulted_count += 1;
                            }
                        }
                        _ => {}
                    }
                }

                // Write into target controller's Input Image Table (%I)
                let val = channel.value_as_float();
                let quality = channel.quality().into();

                if let Some(ctrl) = controllers.get_mut(&mapping.controller_id) {
                    ctrl.io_image_mut()
                        .write_input(&mapping.signal_key, val, quality, sim_time);
                }
            }
        }

        let elapsed_us = start_time.elapsed().as_micros() as u64;
        self.diagnostics.channels_scanned += scanned_count;
        self.diagnostics.faulted_channels = faulted_count;
        self.diagnostics.last_scan_duration_us = elapsed_us;
        self.diagnostics.max_scan_duration_us =
            self.diagnostics.max_scan_duration_us.max(elapsed_us);
        self.diagnostics.avg_scan_duration_us =
            0.95 * self.diagnostics.avg_scan_duration_us + 0.05 * (elapsed_us as f64);
    }

    /// Dispatches output values from controller `%Q` tables through HAL output channels,
    /// applying output scaling, fallback behaviors, and forwarding to actuator arbiters.
    pub fn dispatch_outputs<F>(
        &mut self,
        controllers: &BTreeMap<ControllerId, VirtualController>,
        mut submit_actuator_cmd: F,
        write_state: &mut StateWriteView<'_>,
        sim_time: f64,
    ) where
        F: FnMut(EquipmentId, ActuatorCommand),
    {
        let mappings = self.mappings.clone();
        for mapping in &mappings {
            if mapping.direction != MappingDirection::Output {
                continue;
            }

            let addr = mapping.channel_address;
            let rack_ok = self
                .racks
                .get(&addr.rack)
                .map(|r| r.power_ok)
                .unwrap_or(false);
            let bus_ok = self
                .racks
                .get(&addr.rack)
                .and_then(|r| r.chassis.get(&addr.chassis))
                .map(|c| c.bus_ok)
                .unwrap_or(false);

            let ctrl_opt = controllers.get(&mapping.controller_id);
            let ctrl_healthy = ctrl_opt
                .map(|c| c.lifecycle().is_running())
                .unwrap_or(false);

            if let Some(channel) = self.channel_mut(&addr) {
                if !rack_ok || !bus_ok || !ctrl_healthy {
                    // Controller stopped, faulted, or hardware unpowered: apply fallback behavior
                    match channel {
                        IoChannel::Ao(c) => c.apply_fallback(sim_time),
                        IoChannel::Do(c) => c.apply_fallback(sim_time),
                        _ => {}
                    }
                } else if let Some(ctrl) = ctrl_opt {
                    // Normal execution: read from %Q
                    if let Some(out_sig) = ctrl.io_image().read_output(&mapping.signal_key) {
                        match channel {
                            IoChannel::Ao(c) => c.write_command(out_sig.value, sim_time),
                            IoChannel::Do(c) => c.write_command(out_sig.as_bool(), sim_time),
                            _ => {}
                        }
                    }
                }

                // Dispatch conditioned channel output to actuator arbiter if configured
                let out_val = channel.value_as_float();
                let out_quality = channel.quality().into();

                if let Some(act_id) = mapping.target_actuator {
                    let source_name = ctrl_opt
                        .map(|c| c.name().to_string())
                        .unwrap_or_else(|| "HAL".to_string());
                    submit_actuator_cmd(
                        act_id,
                        ActuatorCommand::new(mapping.priority, out_val).with_source(source_name),
                    );
                }

                // Dispatch to simulation state tag if configured
                if let Some(target_tag) = mapping.target_tag {
                    write_state.set_tag(target_tag, TagValue::Float(out_val), out_quality);
                }
            }
        }
    }
}
