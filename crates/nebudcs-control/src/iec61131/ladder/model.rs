//! Ladder Diagram (LD) AST and Structural Model.
//!
//! Models rungs, power rails, contacts (NO, NC, P, N), parallel branches,
//! coils (Output, Negated, Set, Reset), and inline function block execution.

use serde::{Deserialize, Serialize};

use crate::iec61131::storage::VariableDeclaration;

/// Classification of a Ladder Diagram contact element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LdContactKind {
    /// Normally Open contact: passes power when operand is TRUE (`--[ ]--`).
    NormallyOpen,
    /// Normally Closed contact: passes power when operand is FALSE (`--[/]--`).
    NormallyClosed,
    /// Positive Transition contact: passes power for 1 scan on rising edge (`--[P]--`).
    PositiveTransition,
    /// Negative Transition contact: passes power for 1 scan on falling edge (`--[N]--`).
    NegativeTransition,
}

/// Contact element evaluating an input or memory variable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LdContact {
    pub kind: LdContactKind,
    pub variable: String,
}

impl LdContact {
    pub fn no(variable: impl Into<String>) -> Self {
        Self {
            kind: LdContactKind::NormallyOpen,
            variable: variable.into(),
        }
    }

    pub fn nc(variable: impl Into<String>) -> Self {
        Self {
            kind: LdContactKind::NormallyClosed,
            variable: variable.into(),
        }
    }

    pub fn pos_edge(variable: impl Into<String>) -> Self {
        Self {
            kind: LdContactKind::PositiveTransition,
            variable: variable.into(),
        }
    }

    pub fn neg_edge(variable: impl Into<String>) -> Self {
        Self {
            kind: LdContactKind::NegativeTransition,
            variable: variable.into(),
        }
    }
}

/// Classification of a Ladder Diagram coil element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LdCoilKind {
    /// Standard Output coil: energized when rung power is TRUE (`--( )--`).
    Output,
    /// Negated coil: energized when rung power is FALSE (`--(/)--`).
    Negated,
    /// Set / Latch coil: sets variable TRUE when rung power is TRUE (`--(S)--`).
    Set,
    /// Reset / Unlatch coil: resets variable FALSE when rung power is TRUE (`--(R)--`).
    Reset,
}

/// Coil element writing to an output or internal variable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LdCoil {
    pub kind: LdCoilKind,
    pub variable: String,
}

impl LdCoil {
    pub fn output(variable: impl Into<String>) -> Self {
        Self {
            kind: LdCoilKind::Output,
            variable: variable.into(),
        }
    }

    pub fn negated(variable: impl Into<String>) -> Self {
        Self {
            kind: LdCoilKind::Negated,
            variable: variable.into(),
        }
    }

    pub fn set(variable: impl Into<String>) -> Self {
        Self {
            kind: LdCoilKind::Set,
            variable: variable.into(),
        }
    }

    pub fn reset(variable: impl Into<String>) -> Self {
        Self {
            kind: LdCoilKind::Reset,
            variable: variable.into(),
        }
    }
}

/// Parallel branch container (OR branch between alternative series paths).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LdParallelBranch {
    pub paths: Vec<Vec<LdElement>>,
}

/// Rung element: single contact, parallel branch network, or inline FB execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LdElement {
    Contact(LdContact),
    Parallel(LdParallelBranch),
    FbCall {
        instance: String,
        fb_type: String,
        inputs: Vec<(String, String)>,
    },
}

/// A single Ladder rung spanning from left power rail to right rail.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LdRung {
    pub number: usize,
    pub comment: Option<String>,
    pub elements: Vec<LdElement>,
    pub coils: Vec<LdCoil>,
}

impl LdRung {
    pub fn new(number: usize) -> Self {
        Self {
            number,
            comment: None,
            elements: Vec::new(),
            coils: Vec::new(),
        }
    }

    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }

    pub fn add_element(&mut self, element: LdElement) -> &mut Self {
        self.elements.push(element);
        self
    }

    pub fn add_coil(&mut self, coil: LdCoil) -> &mut Self {
        self.coils.push(coil);
        self
    }
}

/// Complete Ladder Diagram specification.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LdDiagram {
    pub rungs: Vec<LdRung>,
    pub variables: Vec<VariableDeclaration>,
}

impl LdDiagram {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_rung(&mut self, rung: LdRung) {
        self.rungs.push(rung);
    }

    pub fn add_variable(&mut self, var: VariableDeclaration) {
        self.variables.push(var);
    }
}
