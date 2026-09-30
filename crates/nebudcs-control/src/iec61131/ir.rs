//! Typed Intermediate Representation (IR) for IEC 61131-3 Languages.
//!
//! All four industrial logic languages (ST, LD, FBD, SFC) compile into this
//! shared, deterministic intermediate representation.

use serde::{Deserialize, Serialize};

use crate::iec61131::types::IecValue;

/// Typed expression tree in the Common IEC Intermediate Representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IecExpr {
    /// Literal constant value (e.g. `12.5`, `TRUE`, `T#500ms`).
    Literal(IecValue),
    /// Variable reference (e.g. `SPEED_SP`, `PUMP_RUNNING`).
    Var(String),
    /// Controller Input Image read (e.g. `%IW100`, `%IX0.0`).
    IoInput(String),
    /// Controller Output Image readback (e.g. `%QW100`, `%QX0.1`).
    IoOutput(String),
    /// Unary operator (`NOT`, `-`, `+`).
    Unary { op: String, expr: Box<IecExpr> },
    /// Binary operator (`+`, `-`, `*`, `/`, `MOD`, `AND`, `OR`, `XOR`, `=`, `<>`, `<`, `>`, `<=`, `>=`).
    Binary {
        op: String,
        left: Box<IecExpr>,
        right: Box<IecExpr>,
    },
    /// Function call (`ABS(x)`, `SQRT(x)`, `MIN(a,b)`, `MAX(a,b)`, `LIMIT(mn,in,mx)`).
    FunctionCall { name: String, args: Vec<IecExpr> },
    /// Array element index access (`BUFFER[i]`).
    ArrayAccess { array: String, index: Box<IecExpr> },
    /// Structure field access (`MOTOR_DATA.STATUS`).
    FieldAccess { struct_var: String, field: String },
}

/// Executable statement in the Common IEC Intermediate Representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IecStatement {
    /// Variable assignment: `target := expr;`.
    Assign { target: String, expr: IecExpr },
    /// Direct Output Image Table assignment: `%QW100 := expr;`.
    AssignIoOutput { key: String, expr: IecExpr },
    /// Array element assignment: `array[index] := expr;`.
    AssignArray {
        array: String,
        index: IecExpr,
        expr: IecExpr,
    },
    /// Structure field assignment: `struct_var.field := expr;`.
    AssignField {
        struct_var: String,
        field: String,
        expr: IecExpr,
    },
    /// Conditional branch: `IF ... THEN ... ELSIF ... ELSE ... END_IF;`.
    If {
        condition: IecExpr,
        then_branch: Vec<IecStatement>,
        elsif_branches: Vec<(IecExpr, Vec<IecStatement>)>,
        else_branch: Option<Vec<IecStatement>>,
    },
    /// Multi-way selection: `CASE selector OF ... ELSE ... END_CASE;`.
    Case {
        selector: IecExpr,
        cases: Vec<(Vec<IecValue>, Vec<IecStatement>)>,
        else_branch: Option<Vec<IecStatement>>,
    },
    /// Iterative counting loop: `FOR counter := from TO to BY step DO ... END_FOR;`.
    For {
        counter: String,
        from: IecExpr,
        to: IecExpr,
        by: Option<IecExpr>,
        body: Vec<IecStatement>,
    },
    /// Pre-tested loop: `WHILE condition DO ... END_WHILE;`.
    While {
        condition: IecExpr,
        body: Vec<IecStatement>,
    },
    /// Post-tested loop: `REPEAT ... UNTIL condition END_REPEAT;`.
    Repeat {
        body: Vec<IecStatement>,
        until: IecExpr,
    },
    /// Function Block instance execution: `TON_01(IN := start, PT := T#5s);`.
    FbCall {
        instance: String,
        fb_type: String,
        params: Vec<(String, IecExpr)>,
    },
    /// Early routine termination.
    Return,
    /// Loop break.
    Exit,
}
