//! Structured channel addressing for the I/O Hardware Abstraction Layer.
//!
//! Canonical formats:
//! - Short: `R01-C01-AI01` (Rack 1, Chassis 1, Analog Input Channel 1)
//! - Extended: `R01-C01-M02-AO03` (Rack 1, Chassis 1, Module 2, Analog Output Channel 3)

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::hal::error::HalError;

/// Signal classification of an I/O channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ChannelKind {
    /// Analog Input (e.g. 4–20 mA transmitter).
    #[serde(rename = "AI")]
    AI,
    /// Analog Output (e.g. 4–20 mA valve positioner).
    #[serde(rename = "AO")]
    AO,
    /// Digital / Discrete Input (e.g. limit switch, contact).
    #[serde(rename = "DI")]
    DI,
    /// Digital / Discrete Output (e.g. solenoid valve, motor starter relay).
    #[serde(rename = "DO")]
    DO,
}

impl ChannelKind {
    /// Short 2-letter abbreviation for channel kind.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AI => "AI",
            Self::AO => "AO",
            Self::DI => "DI",
            Self::DO => "DO",
        }
    }

    /// Whether this channel represents an input to the controller.
    pub fn is_input(&self) -> bool {
        matches!(self, Self::AI | Self::DI)
    }

    /// Whether this channel represents an output from the controller.
    pub fn is_output(&self) -> bool {
        matches!(self, Self::AO | Self::DO)
    }

    /// Whether this channel is analog.
    pub fn is_analog(&self) -> bool {
        matches!(self, Self::AI | Self::AO)
    }

    /// Whether this channel is digital.
    pub fn is_digital(&self) -> bool {
        matches!(self, Self::DI | Self::DO)
    }
}

impl fmt::Display for ChannelKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for ChannelKind {
    type Err = HalError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "AI" => Ok(Self::AI),
            "AO" => Ok(Self::AO),
            "DI" => Ok(Self::DI),
            "DO" => Ok(Self::DO),
            _ => Err(HalError::InvalidAddress {
                address: s.to_string(),
                detail: format!("unknown channel kind '{}'; expected AI, AO, DI, or DO", s),
            }),
        }
    }
}

/// Strongly-typed deterministic hardware address for an I/O channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct IoChannelAddress {
    /// Rack index (1..255).
    pub rack: u8,
    /// Chassis index within the rack (1..255).
    pub chassis: u8,
    /// Module slot index within the chassis (1..255).
    pub module: u8,
    /// Channel index on the module (1..255).
    pub channel: u8,
    /// Functional kind of the channel (AI, AO, DI, DO).
    pub kind: ChannelKind,
}

impl IoChannelAddress {
    /// Creates a new channel address.
    pub fn new(rack: u8, chassis: u8, module: u8, channel: u8, kind: ChannelKind) -> Self {
        Self {
            rack,
            chassis,
            module,
            channel,
            kind,
        }
    }

    /// Creates a standard compact address where module slot defaults to 1.
    pub fn compact(rack: u8, chassis: u8, channel: u8, kind: ChannelKind) -> Self {
        Self::new(rack, chassis, 1, channel, kind)
    }

    /// Formats as standard canonical short string: `R01-C01-AI01`.
    pub fn to_canonical_string(&self) -> String {
        if self.module == 1 {
            format!(
                "R{:02}-C{:02}-{}{:02}",
                self.rack,
                self.chassis,
                self.kind.as_str(),
                self.channel
            )
        } else {
            format!(
                "R{:02}-C{:02}-M{:02}-{}{:02}",
                self.rack,
                self.chassis,
                self.module,
                self.kind.as_str(),
                self.channel
            )
        }
    }
}

impl fmt::Display for IoChannelAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_canonical_string())
    }
}

impl FromStr for IoChannelAddress {
    type Err = HalError;

    /// Parses address strings such as:
    /// - `R01-C01-AI01`
    /// - `R01-C01-AO03`
    /// - `R01-C02-DI08`
    /// - `R01-C01-M02-AI04`
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('-').map(|p| p.trim()).collect();
        if parts.len() < 3 || parts.len() > 4 {
            return Err(HalError::InvalidAddress {
                address: s.to_string(),
                detail: format!(
                    "expected 3 or 4 segments delimited by '-', got {}",
                    parts.len()
                ),
            });
        }

        // Segment 1: Rack (e.g. "R01" or "1")
        let rack_str = parts[0]
            .strip_prefix(|c| c == 'R' || c == 'r')
            .unwrap_or(parts[0]);
        let rack = rack_str
            .parse::<u8>()
            .map_err(|_| HalError::InvalidAddress {
                address: s.to_string(),
                detail: format!("invalid rack index '{}'", parts[0]),
            })?;

        // Segment 2: Chassis (e.g. "C01" or "1")
        let chassis_str = parts[1]
            .strip_prefix(|c| c == 'C' || c == 'c')
            .unwrap_or(parts[1]);
        let chassis = chassis_str
            .parse::<u8>()
            .map_err(|_| HalError::InvalidAddress {
                address: s.to_string(),
                detail: format!("invalid chassis index '{}'", parts[1]),
            })?;

        let (module, channel_part) = if parts.len() == 4 {
            // Module slot explicitly specified: "M02"
            let mod_str = parts[2]
                .strip_prefix(|c| c == 'M' || c == 'm')
                .unwrap_or(parts[2]);
            let module = mod_str
                .parse::<u8>()
                .map_err(|_| HalError::InvalidAddress {
                    address: s.to_string(),
                    detail: format!("invalid module index '{}'", parts[2]),
                })?;
            (module, parts[3])
        } else {
            (1u8, parts[2])
        };

        // Channel segment: e.g. "AI01" or "DO04" or "CH01"
        if channel_part.len() < 3 {
            return Err(HalError::InvalidAddress {
                address: s.to_string(),
                detail: format!("channel segment '{}' is too short", channel_part),
            });
        }

        let kind_str = &channel_part[..2];
        let num_str = &channel_part[2..];

        let kind = ChannelKind::from_str(kind_str)?;
        let channel = num_str
            .parse::<u8>()
            .map_err(|_| HalError::InvalidAddress {
                address: s.to_string(),
                detail: format!("invalid channel number in '{}'", channel_part),
            })?;

        if channel == 0 {
            return Err(HalError::InvalidAddress {
                address: s.to_string(),
                detail: "channel index must be 1-based (>= 1)".to_string(),
            });
        }

        Ok(Self::new(rack, chassis, module, channel, kind))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_parse_and_display() {
        let addr: IoChannelAddress = "R01-C01-AI03".parse().unwrap();
        assert_eq!(addr.rack, 1);
        assert_eq!(addr.chassis, 1);
        assert_eq!(addr.module, 1);
        assert_eq!(addr.channel, 3);
        assert_eq!(addr.kind, ChannelKind::AI);
        assert_eq!(addr.to_string(), "R01-C01-AI03");

        let ext_addr: IoChannelAddress = "R02-C03-M04-DO12".parse().unwrap();
        assert_eq!(ext_addr.rack, 2);
        assert_eq!(ext_addr.chassis, 3);
        assert_eq!(ext_addr.module, 4);
        assert_eq!(ext_addr.channel, 12);
        assert_eq!(ext_addr.kind, ChannelKind::DO);
        assert_eq!(ext_addr.to_string(), "R02-C03-M04-DO12");
    }

    #[test]
    fn test_address_invalid_rejection() {
        assert!("INVALID".parse::<IoChannelAddress>().is_err());
        assert!("R01-C01-XX01".parse::<IoChannelAddress>().is_err());
        assert!("R01-C01-AI00".parse::<IoChannelAddress>().is_err()); // 0-based rejected
    }
}
