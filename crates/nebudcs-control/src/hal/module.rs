//! I/O Module abstraction representing a card or slice in a chassis slot.

use nebudcs_types::ModuleId;
use serde::{Deserialize, Serialize};

use crate::hal::address::ChannelKind;
use crate::hal::channel::IoChannel;
use crate::hal::error::HalError;

/// Classification of physical I/O card models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IoModuleType {
    /// 8-Channel 4–20 mA Analog Input Card.
    AnalogInput8,
    /// 8-Channel 4–20 mA Analog Output Card.
    AnalogOutput8,
    /// 16-Channel 24 VDC Discrete Input Card.
    DigitalInput16,
    /// 16-Channel 24 VDC Solid-State / Relay Output Card.
    DigitalOutput16,
    /// Universal Mixed I/O Card (4 AI, 4 AO, 8 DI, 8 DO).
    UniversalMixed,
}

impl IoModuleType {
    /// Maximum number of channels supported by this module type.
    pub fn max_channels(&self) -> u8 {
        match self {
            Self::AnalogInput8 => 8,
            Self::AnalogOutput8 => 8,
            Self::DigitalInput16 => 16,
            Self::DigitalOutput16 => 16,
            Self::UniversalMixed => 24,
        }
    }

    /// Whether this module type supports the given channel kind.
    pub fn supports_kind(&self, kind: ChannelKind) -> bool {
        match self {
            Self::AnalogInput8 => kind == ChannelKind::AI,
            Self::AnalogOutput8 => kind == ChannelKind::AO,
            Self::DigitalInput16 => kind == ChannelKind::DI,
            Self::DigitalOutput16 => kind == ChannelKind::DO,
            Self::UniversalMixed => true,
        }
    }

    /// User-friendly module model designation.
    pub fn model_name(&self) -> &'static str {
        match self {
            Self::AnalogInput8 => "AI-08-420MA",
            Self::AnalogOutput8 => "AO-08-420MA",
            Self::DigitalInput16 => "DI-16-24VDC",
            Self::DigitalOutput16 => "DO-16-RELAY",
            Self::UniversalMixed => "MIX-24-UNIV",
        }
    }
}

/// Representation of an I/O module in a chassis slot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IoModule {
    pub id: ModuleId,
    /// 1-based slot index in the parent chassis.
    pub slot: u8,
    pub name: String,
    pub module_type: IoModuleType,
    pub channels: Vec<IoChannel>,
    pub is_healthy: bool,
    pub is_enabled: bool,
}

impl IoModule {
    /// Creates a new I/O module.
    pub fn new(slot: u8, name: impl Into<String>, module_type: IoModuleType) -> Self {
        Self {
            id: ModuleId::new(),
            slot,
            name: name.into(),
            module_type,
            channels: Vec::new(),
            is_healthy: true,
            is_enabled: true,
        }
    }

    /// Adds a channel to this module, verifying channel index and type compatibility.
    pub fn add_channel(&mut self, channel: IoChannel) -> Result<(), HalError> {
        let max = self.module_type.max_channels();
        let num = channel.channel_number();
        if num == 0 || num > max {
            return Err(HalError::ChannelOutOfBounds {
                module_type: self.module_type.model_name().to_string(),
                channel: num,
                max_channels: max,
            });
        }

        let kind = channel.address().kind;
        if !self.module_type.supports_kind(kind) {
            return Err(HalError::IncompatibleSignalType {
                address: channel.address().to_string(),
                expected: self.module_type.model_name().to_string(),
                found: kind.to_string(),
            });
        }

        if self.channels.iter().any(|c| c.channel_number() == num) {
            return Err(HalError::DuplicateAddress {
                address: channel.address().to_string(),
            });
        }

        self.channels.push(channel);
        Ok(())
    }

    /// Retrieves an immutable reference to a channel by its 1-based index.
    pub fn channel(&self, num: u8) -> Option<&IoChannel> {
        self.channels.iter().find(|c| c.channel_number() == num)
    }

    /// Retrieves a mutable reference to a channel by its 1-based index.
    pub fn channel_mut(&mut self, num: u8) -> Option<&mut IoChannel> {
        self.channels.iter_mut().find(|c| c.channel_number() == num)
    }
}
