//! Function Block Diagram (FBD) AST and Structural Model.
//!
//! Models graphical signal-flow networks: logic gates, arithmetic blocks,
//! comparison blocks, standard function blocks, inputs, outputs, and feedback delays.

use serde::{Deserialize, Serialize};

use crate::iec61131::storage::VariableDeclaration;
use crate::iec61131::types::IecValue;

/// Classification of a node in a Function Block Diagram.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FbdNodeKind {
    /// Reads from a named variable or physical `%I` address.
    InputVar { name: String },
    /// Writes to a named variable or physical `%Q` address.
    OutputVar { name: String },
    /// Constant literal value.
    Constant { value: IecValue },
    /// Boolean logic gate (AND, OR, NOT, XOR, NAND, NOR).
    LogicGate { gate_type: String },
    /// Arithmetic / Math function (ADD, SUB, MUL, DIV, MOD, ABS, SQRT, MIN, MAX, LIMIT).
    Arithmetic { op: String },
    /// Relational comparison (EQ, NE, LT, LE, GT, GE).
    Comparison { op: String },
    /// Function Block instance (TON, TOF, TP, CTU, CTD, SR, RS, PID).
    FunctionBlock { instance: String, fb_type: String },
    /// Single-scan feedback delay block (`DELAY_1T`) to safely break feedback loops.
    Delay1T { memory_var: String },
}

/// A node within an FBD signal graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FbdNode {
    pub id: String,
    pub kind: FbdNodeKind,
    pub label: Option<String>,
}

impl FbdNode {
    pub fn new(id: impl Into<String>, kind: FbdNodeKind) -> Self {
        Self {
            id: id.into(),
            kind,
            label: None,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// A directed connection wire connecting a source pin to a target pin.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FbdConnection {
    pub from_node: String,
    pub from_pin: String,
    pub to_node: String,
    pub to_pin: String,
}

impl FbdConnection {
    pub fn new(
        from_node: impl Into<String>,
        from_pin: impl Into<String>,
        to_node: impl Into<String>,
        to_pin: impl Into<String>,
    ) -> Self {
        Self {
            from_node: from_node.into(),
            from_pin: from_pin.into(),
            to_node: to_node.into(),
            to_pin: to_pin.into(),
        }
    }
}

/// Complete Function Block Diagram specification.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FbdGraph {
    pub nodes: Vec<FbdNode>,
    pub connections: Vec<FbdConnection>,
    pub variables: Vec<VariableDeclaration>,
}

impl FbdGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: FbdNode) {
        self.nodes.push(node);
    }

    pub fn add_connection(&mut self, conn: FbdConnection) {
        self.connections.push(conn);
    }

    pub fn add_variable(&mut self, var: VariableDeclaration) {
        self.variables.push(var);
    }
}
