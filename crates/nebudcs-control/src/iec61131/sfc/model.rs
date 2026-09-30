//! Sequential Function Chart (SFC) AST and Structural Model.
//!
//! Models steps, initial steps, transitions, action qualifiers (N, S, R, P, L, D),
//! and state machine sequence networks.

use serde::{Deserialize, Serialize};

use crate::iec61131::storage::VariableDeclaration;

/// Action block qualifier defining execution timing relative to step activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SfcActionQualifier {
    /// Non-stored: active continuously while step is active (`N`).
    N,
    /// Set / Stored: activated on step entry, remains active until Reset (`S`).
    S,
    /// Reset: deactivated on step entry (`R`).
    R,
    /// Pulse: active for exactly 1 scan cycle on step entry (`P`).
    P,
    /// Time Limited: active while step is active, up to maximum duration T (`L`).
    L,
    /// Time Delayed: active while step is active, after minimum duration T (`D`).
    D,
}

/// Action block bound to a step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SfcAction {
    pub step_name: String,
    pub qualifier: SfcActionQualifier,
    pub target_var: String,
    pub duration: Option<f64>,
}

impl SfcAction {
    pub fn new(
        step_name: impl Into<String>,
        qualifier: SfcActionQualifier,
        target_var: impl Into<String>,
    ) -> Self {
        Self {
            step_name: step_name.into(),
            qualifier,
            target_var: target_var.into(),
            duration: None,
        }
    }

    pub fn with_duration(mut self, seconds: f64) -> Self {
        self.duration = Some(seconds);
        self
    }
}

/// Step element within an SFC sequence.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SfcStep {
    pub name: String,
    pub is_initial: bool,
}

impl SfcStep {
    pub fn new(name: impl Into<String>, is_initial: bool) -> Self {
        Self {
            name: name.into(),
            is_initial,
        }
    }
}

/// Transition between steps guarded by a condition expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SfcTransition {
    pub id: String,
    pub from_step: String,
    pub to_step: String,
    pub condition_expr: String,
    pub priority: usize,
}

impl SfcTransition {
    pub fn new(
        id: impl Into<String>,
        from_step: impl Into<String>,
        to_step: impl Into<String>,
        condition_expr: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            from_step: from_step.into(),
            to_step: to_step.into(),
            condition_expr: condition_expr.into(),
            priority: 0,
        }
    }

    pub fn with_priority(mut self, priority: usize) -> Self {
        self.priority = priority;
        self
    }
}

/// Complete Sequential Function Chart specification.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SfcChart {
    pub steps: Vec<SfcStep>,
    pub transitions: Vec<SfcTransition>,
    pub actions: Vec<SfcAction>,
    pub variables: Vec<VariableDeclaration>,
}

impl SfcChart {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_step(&mut self, step: SfcStep) {
        self.steps.push(step);
    }

    pub fn add_transition(&mut self, transition: SfcTransition) {
        self.transitions.push(transition);
    }

    pub fn add_action(&mut self, action: SfcAction) {
        self.actions.push(action);
    }

    pub fn add_variable(&mut self, var: VariableDeclaration) {
        self.variables.push(var);
    }
}
