//! Signal selectors and override / selective control strategies.
//!
//! Provides:
//! - [`HighSelector`]: Selects the maximum value among candidate inputs.
//! - [`LowSelector`]: Selects the minimum value among candidate inputs.
//! - [`OverrideSelector`]: Selects between primary control and safety/constraint
//!   override with anti-windup external reset feedback.

use serde::{Deserialize, Serialize};

/// Type of selection criterion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectorType {
    /// High selection ($y = \max(u_1, u_2)$).
    High,
    /// Low selection ($y = \min(u_1, u_2)$).
    Low,
}

/// Identifies which input source was selected by an override block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectedSource {
    /// Primary controller output was selected.
    Primary,
    /// Constraint / safety override controller output was selected.
    Override,
}

/// High signal selector ($y = \max_i u_i$).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HighSelector;

impl HighSelector {
    /// Selects the maximum finite input among candidates.
    pub fn select(&self, inputs: &[f64]) -> f64 {
        inputs
            .iter()
            .copied()
            .filter(|v| v.is_finite())
            .fold(f64::NEG_INFINITY, f64::max)
    }
}

/// Low signal selector ($y = \min_i u_i$).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LowSelector;

impl LowSelector {
    /// Selects the minimum finite input among candidates.
    pub fn select(&self, inputs: &[f64]) -> f64 {
        inputs
            .iter()
            .copied()
            .filter(|v| v.is_finite())
            .fold(f64::INFINITY, f64::min)
    }
}

/// Override / Selective control block with anti-windup external reset feedback.
///
/// Compares a primary controller demand $u_{prim}$ against a constraint override demand
/// $u_{ovr}$ (e.g. suction pressure constraint overriding normal flow control).
///
/// Generates external reset feedback ($FB_{ext}$) equal to the selected winning command.
/// Passing this feedback to the unselected controller guarantees zero reset windup!
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverrideSelector {
    /// Selection criterion (High or Low).
    pub selector_type: SelectorType,
    /// Currently selected winning source.
    selected_source: SelectedSource,
    /// Current selected output value.
    output: f64,
}

impl OverrideSelector {
    /// Creates a new override selector with the given criterion.
    pub fn new(selector_type: SelectorType) -> Self {
        Self {
            selector_type,
            selected_source: SelectedSource::Primary,
            output: 0.0,
        }
    }

    /// Evaluates the override selector given primary and override inputs.
    ///
    /// Returns the selected winning output command.
    pub fn select(&mut self, u_primary: f64, u_override: f64) -> f64 {
        if !u_primary.is_finite() && !u_override.is_finite() {
            return self.output;
        }
        if !u_primary.is_finite() {
            self.selected_source = SelectedSource::Override;
            self.output = u_override;
            return self.output;
        }
        if !u_override.is_finite() {
            self.selected_source = SelectedSource::Primary;
            self.output = u_primary;
            return self.output;
        }

        match self.selector_type {
            SelectorType::High => {
                if u_override > u_primary {
                    self.selected_source = SelectedSource::Override;
                    self.output = u_override;
                } else {
                    self.selected_source = SelectedSource::Primary;
                    self.output = u_primary;
                }
            }
            SelectorType::Low => {
                if u_override < u_primary {
                    self.selected_source = SelectedSource::Override;
                    self.output = u_override;
                } else {
                    self.selected_source = SelectedSource::Primary;
                    self.output = u_primary;
                }
            }
        }

        self.output
    }

    /// Returns which controller is currently driving the manipulated variable.
    pub fn selected_source(&self) -> SelectedSource {
        self.selected_source
    }

    /// True if the safety or constraint override controller is currently in control.
    pub fn is_override_active(&self) -> bool {
        self.selected_source == SelectedSource::Override
    }

    /// Current output command.
    pub fn output(&self) -> f64 {
        self.output
    }

    /// External reset feedback to be fed to both primary and override controllers.
    ///
    /// In Foxboro / Fisher / Honeywell architectures, setting integral tracking or
    /// reset feedback to the selector output keeps the unselected loop ready to resume
    /// control instantly without bump or overshoot.
    pub fn external_reset_feedback(&self) -> f64 {
        self.output
    }
}
