//! Function Block Diagram (FBD) Topological Sort and IR Compiler.
//!
//! Converts graphical signal-flow networks into deterministic, cycle-checked
//! IR execution sequences.

use std::collections::{BTreeMap, VecDeque};

use crate::error::ControlError;
use crate::iec61131::fbd::model::{FbdConnection, FbdGraph, FbdNode, FbdNodeKind};
use crate::iec61131::ir::{IecExpr, IecStatement};
use crate::iec61131::storage::SymbolTable;
use crate::iec61131::types::IecValue;

/// Compiles an FBD graph into a SymbolTable and executable IR statements.
pub fn compile_fbd(graph: &FbdGraph) -> Result<(SymbolTable, Vec<IecStatement>), ControlError> {
    let mut symbols = SymbolTable::new();
    for var in &graph.variables {
        symbols.declare(var.clone())?;
    }

    // Index nodes by ID
    let mut node_map: BTreeMap<String, &FbdNode> = BTreeMap::new();
    for node in &graph.nodes {
        if node_map.contains_key(&node.id) {
            return Err(ControlError::InvalidParameter(format!(
                "Duplicate FBD node ID '{}'",
                node.id
            )));
        }
        node_map.insert(node.id.clone(), node);
    }

    // Topological sorting with Kahn's algorithm
    // Note: Delay1T incoming edges do NOT block scan order; Delay1T produces
    // its output from previous-scan memory, breaking algebraic cycles.
    let mut in_degree: BTreeMap<String, usize> = BTreeMap::new();
    let mut adj: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for node in &graph.nodes {
        in_degree.insert(node.id.clone(), 0);
        adj.insert(node.id.clone(), Vec::new());
    }

    for conn in &graph.connections {
        let to_node = node_map.get(&conn.to_node).ok_or_else(|| {
            ControlError::InvalidParameter(format!(
                "Connection target node '{}' not found",
                conn.to_node
            ))
        })?;

        // If target is Delay1T, the edge is a feedback delay and does not increment in-degree
        if !matches!(to_node.kind, FbdNodeKind::Delay1T { .. }) {
            *in_degree.entry(conn.to_node.clone()).or_insert(0) += 1;
            adj.entry(conn.from_node.clone())
                .or_default()
                .push(conn.to_node.clone());
        }
    }

    let mut queue: VecDeque<String> = VecDeque::new();
    for (id, &deg) in &in_degree {
        if deg == 0 {
            queue.push_back(id.clone());
        }
    }

    let mut execution_order: Vec<String> = Vec::new();
    while let Some(node_id) = queue.pop_front() {
        execution_order.push(node_id.clone());
        if let Some(neighbors) = adj.get(&node_id) {
            for neighbor in neighbors {
                let deg = in_degree.get_mut(neighbor).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    queue.push_back(neighbor.clone());
                }
            }
        }
    }

    if execution_order.len() != graph.nodes.len() {
        return Err(ControlError::InvalidParameter(
            "FBD graph contains an unbuffered algebraic feedback cycle. Feedback paths must include a DELAY_1T block.".to_string(),
        ));
    }

    // Map: (node_id, pin_name) -> IecExpr
    let mut pin_outputs: BTreeMap<(String, String), IecExpr> = BTreeMap::new();
    let mut statements: Vec<IecStatement> = Vec::new();
    let mut delay_updates: Vec<IecStatement> = Vec::new();

    // Group incoming connections by target: to_node -> Vec<FbdConnection>
    let mut incoming_conns: BTreeMap<String, Vec<&FbdConnection>> = BTreeMap::new();
    for conn in &graph.connections {
        incoming_conns
            .entry(conn.to_node.clone())
            .or_default()
            .push(conn);
    }

    for node_id in &execution_order {
        let node = node_map.get(node_id).unwrap();
        let in_conns = incoming_conns.get(node_id).cloned().unwrap_or_default();

        match &node.kind {
            FbdNodeKind::InputVar { name } => {
                let expr = if name.starts_with("%I") || name.starts_with("%i") {
                    IecExpr::IoInput(name.clone())
                } else if name.starts_with("%Q") || name.starts_with("%q") {
                    IecExpr::IoOutput(name.clone())
                } else {
                    IecExpr::Var(name.clone())
                };
                pin_outputs.insert((node.id.clone(), "OUT".to_string()), expr.clone());
                pin_outputs.insert((node.id.clone(), String::new()), expr);
            }
            FbdNodeKind::Constant { value } => {
                let expr = IecExpr::Literal(value.clone());
                pin_outputs.insert((node.id.clone(), "OUT".to_string()), expr.clone());
                pin_outputs.insert((node.id.clone(), String::new()), expr);
            }
            FbdNodeKind::Delay1T { memory_var } => {
                // Out pin produces current value of memory_var
                let out_expr = IecExpr::Var(memory_var.clone());
                pin_outputs.insert((node.id.clone(), "OUT".to_string()), out_expr.clone());
                pin_outputs.insert((node.id.clone(), String::new()), out_expr);

                // Find input to delay
                if let Some(conn) = in_conns.first() {
                    let in_expr = lookup_source_pin(&pin_outputs, &conn.from_node, &conn.from_pin)?;
                    delay_updates.push(IecStatement::Assign {
                        target: memory_var.clone(),
                        expr: in_expr,
                    });
                }
            }
            FbdNodeKind::LogicGate { gate_type } => {
                let in_exprs = collect_inputs(&pin_outputs, &in_conns)?;
                let gate_expr = build_logic_gate(gate_type, in_exprs)?;
                let temp_var = format!("_FBD_{}", node.id.replace('-', "_"));
                statements.push(IecStatement::Assign {
                    target: temp_var.clone(),
                    expr: gate_expr,
                });
                pin_outputs.insert(
                    (node.id.clone(), "OUT".to_string()),
                    IecExpr::Var(temp_var.clone()),
                );
                pin_outputs.insert((node.id.clone(), String::new()), IecExpr::Var(temp_var));
            }
            FbdNodeKind::Arithmetic { op } => {
                let in_exprs = collect_inputs(&pin_outputs, &in_conns)?;
                let arith_expr = build_arithmetic(op, in_exprs)?;
                let temp_var = format!("_FBD_{}", node.id.replace('-', "_"));
                statements.push(IecStatement::Assign {
                    target: temp_var.clone(),
                    expr: arith_expr,
                });
                pin_outputs.insert(
                    (node.id.clone(), "OUT".to_string()),
                    IecExpr::Var(temp_var.clone()),
                );
                pin_outputs.insert((node.id.clone(), String::new()), IecExpr::Var(temp_var));
            }
            FbdNodeKind::Comparison { op } => {
                let in_exprs = collect_inputs(&pin_outputs, &in_conns)?;
                if in_exprs.len() < 2 {
                    return Err(ControlError::InvalidParameter(format!(
                        "Comparison node '{}' requires 2 inputs",
                        node.id
                    )));
                }
                let comp_expr = IecExpr::Binary {
                    op: op.clone(),
                    left: Box::new(in_exprs[0].clone()),
                    right: Box::new(in_exprs[1].clone()),
                };
                let temp_var = format!("_FBD_{}", node.id.replace('-', "_"));
                statements.push(IecStatement::Assign {
                    target: temp_var.clone(),
                    expr: comp_expr,
                });
                pin_outputs.insert(
                    (node.id.clone(), "OUT".to_string()),
                    IecExpr::Var(temp_var.clone()),
                );
                pin_outputs.insert((node.id.clone(), String::new()), IecExpr::Var(temp_var));
            }
            FbdNodeKind::FunctionBlock { instance, fb_type } => {
                let mut params = Vec::new();
                for conn in &in_conns {
                    let expr = lookup_source_pin(&pin_outputs, &conn.from_node, &conn.from_pin)?;
                    params.push((conn.to_pin.clone(), expr));
                }

                statements.push(IecStatement::FbCall {
                    instance: instance.clone(),
                    fb_type: fb_type.clone(),
                    params,
                });

                // Standard outputs accessible from this FB
                for pin in &["Q", "ET", "CV", "OUT", "PV", "SP", "MAN_OUT"] {
                    let out_expr = IecExpr::FieldAccess {
                        struct_var: instance.clone(),
                        field: pin.to_string(),
                    };
                    pin_outputs.insert((node.id.clone(), pin.to_string()), out_expr);
                }
                // Default OUT pin mapped to Q or OUT
                let default_out = if fb_type.eq_ignore_ascii_case("PID") {
                    IecExpr::FieldAccess {
                        struct_var: instance.clone(),
                        field: "OUT".to_string(),
                    }
                } else {
                    IecExpr::FieldAccess {
                        struct_var: instance.clone(),
                        field: "Q".to_string(),
                    }
                };
                pin_outputs.insert((node.id.clone(), String::new()), default_out);
            }
            FbdNodeKind::OutputVar { name } => {
                if let Some(conn) = in_conns.first() {
                    let in_expr = lookup_source_pin(&pin_outputs, &conn.from_node, &conn.from_pin)?;
                    if name.starts_with("%Q") || name.starts_with("%q") {
                        statements.push(IecStatement::AssignIoOutput {
                            key: name.clone(),
                            expr: in_expr,
                        });
                    } else {
                        statements.push(IecStatement::Assign {
                            target: name.clone(),
                            expr: in_expr,
                        });
                    }
                }
            }
        }
    }

    // Append delay updates at the end of the scan
    statements.extend(delay_updates);

    Ok((symbols, statements))
}

fn lookup_source_pin(
    pin_outputs: &BTreeMap<(String, String), IecExpr>,
    from_node: &str,
    from_pin: &str,
) -> Result<IecExpr, ControlError> {
    if let Some(expr) = pin_outputs.get(&(from_node.to_string(), from_pin.to_string())) {
        return Ok(expr.clone());
    }
    // Fallback to default output pin "" or "OUT"
    if let Some(expr) = pin_outputs.get(&(from_node.to_string(), String::new())) {
        return Ok(expr.clone());
    }
    if let Some(expr) = pin_outputs.get(&(from_node.to_string(), "OUT".to_string())) {
        return Ok(expr.clone());
    }
    Err(ControlError::InvalidParameter(format!(
        "Source pin '{from_pin}' on node '{from_node}' not found or not yet evaluated"
    )))
}

fn collect_inputs(
    pin_outputs: &BTreeMap<(String, String), IecExpr>,
    in_conns: &[&FbdConnection],
) -> Result<Vec<IecExpr>, ControlError> {
    let mut sorted_conns = in_conns.to_vec();
    sorted_conns.sort_by_key(|c| c.to_pin.clone());

    let mut exprs = Vec::new();
    for conn in sorted_conns {
        exprs.push(lookup_source_pin(
            pin_outputs,
            &conn.from_node,
            &conn.from_pin,
        )?);
    }
    Ok(exprs)
}

fn build_logic_gate(gate: &str, inputs: Vec<IecExpr>) -> Result<IecExpr, ControlError> {
    match gate.to_ascii_uppercase().as_str() {
        "NOT" => {
            let first = inputs.first().ok_or_else(|| {
                ControlError::InvalidParameter("NOT gate requires 1 input".into())
            })?;
            Ok(IecExpr::Unary {
                op: "NOT".to_string(),
                expr: Box::new(first.clone()),
            })
        }
        "AND" => {
            if inputs.is_empty() {
                return Ok(IecExpr::Literal(IecValue::Bool(true)));
            }
            let mut it = inputs.into_iter();
            let mut acc = it.next().unwrap();
            for item in it {
                acc = IecExpr::Binary {
                    op: "AND".to_string(),
                    left: Box::new(acc),
                    right: Box::new(item),
                };
            }
            Ok(acc)
        }
        "OR" => {
            if inputs.is_empty() {
                return Ok(IecExpr::Literal(IecValue::Bool(false)));
            }
            let mut it = inputs.into_iter();
            let mut acc = it.next().unwrap();
            for item in it {
                acc = IecExpr::Binary {
                    op: "OR".to_string(),
                    left: Box::new(acc),
                    right: Box::new(item),
                };
            }
            Ok(acc)
        }
        "XOR" => {
            if inputs.len() < 2 {
                return Err(ControlError::InvalidParameter(
                    "XOR gate requires 2 inputs".into(),
                ));
            }
            Ok(IecExpr::Binary {
                op: "XOR".to_string(),
                left: Box::new(inputs[0].clone()),
                right: Box::new(inputs[1].clone()),
            })
        }
        "NAND" => {
            let and_expr = build_logic_gate("AND", inputs)?;
            Ok(IecExpr::Unary {
                op: "NOT".to_string(),
                expr: Box::new(and_expr),
            })
        }
        "NOR" => {
            let or_expr = build_logic_gate("OR", inputs)?;
            Ok(IecExpr::Unary {
                op: "NOT".to_string(),
                expr: Box::new(or_expr),
            })
        }
        other => Err(ControlError::InvalidParameter(format!(
            "Unknown logic gate '{other}'"
        ))),
    }
}

fn build_arithmetic(op: &str, inputs: Vec<IecExpr>) -> Result<IecExpr, ControlError> {
    match op.to_ascii_uppercase().as_str() {
        "ADD" | "+" => {
            if inputs.len() < 2 {
                return Err(ControlError::InvalidParameter(
                    "ADD requires 2 inputs".into(),
                ));
            }
            Ok(IecExpr::Binary {
                op: "+".to_string(),
                left: Box::new(inputs[0].clone()),
                right: Box::new(inputs[1].clone()),
            })
        }
        "SUB" | "-" => {
            if inputs.len() < 2 {
                return Err(ControlError::InvalidParameter(
                    "SUB requires 2 inputs".into(),
                ));
            }
            Ok(IecExpr::Binary {
                op: "-".to_string(),
                left: Box::new(inputs[0].clone()),
                right: Box::new(inputs[1].clone()),
            })
        }
        "MUL" | "*" => {
            if inputs.len() < 2 {
                return Err(ControlError::InvalidParameter(
                    "MUL requires 2 inputs".into(),
                ));
            }
            Ok(IecExpr::Binary {
                op: "*".to_string(),
                left: Box::new(inputs[0].clone()),
                right: Box::new(inputs[1].clone()),
            })
        }
        "DIV" | "/" => {
            if inputs.len() < 2 {
                return Err(ControlError::InvalidParameter(
                    "DIV requires 2 inputs".into(),
                ));
            }
            Ok(IecExpr::Binary {
                op: "/".to_string(),
                left: Box::new(inputs[0].clone()),
                right: Box::new(inputs[1].clone()),
            })
        }
        func @ ("ABS" | "SQRT" | "MIN" | "MAX" | "LIMIT" | "LN" | "EXP" | "SIN" | "COS"
        | "TRUNC") => Ok(IecExpr::FunctionCall {
            name: func.to_string(),
            args: inputs,
        }),
        other => Err(ControlError::InvalidParameter(format!(
            "Unknown arithmetic operation '{other}'"
        ))),
    }
}
