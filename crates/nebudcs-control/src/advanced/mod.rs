//! Stage 5: Advanced Process Control Subsystem.
//!
//! Provides deterministic industrial control strategies and composable primitives:
//! - Dynamic signal conditioning and lead-lag compensation ([`filter`])
//! - Wild/controlled ratio regulation ([`ratio`])
//! - High/Low override selectors with anti-windup external reset feedback ([`selector`])
//! - Multi-actuator split-range control ([`split_range`])
//! - Equal-percentage, linear, and quick-opening valve trim characterization ([`valve_char`])
//! - Compressor dynamic anti-surge protection ([`anti_surge`])
//! - Composable strategy orchestration ([`strategy`])

pub mod anti_surge;
pub mod filter;
pub mod ratio;
pub mod selector;
pub mod split_range;
pub mod strategy;
pub mod valve_char;

pub use anti_surge::{AntiSurgeAlarmState, AntiSurgeController};
pub use filter::{DeadTimeBuffer, FirstOrderFilter, LeadLagCompensator};
pub use ratio::RatioController;
pub use selector::{HighSelector, LowSelector, OverrideSelector, SelectedSource, SelectorType};
pub use split_range::{SplitRangeController, SplitRangeSegment};
pub use strategy::{AdvancedControlStrategy, StrategyKind};
pub use valve_char::{ValveCharacteristic, ValveCharacterizer};
