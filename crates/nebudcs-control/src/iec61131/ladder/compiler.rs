//! Ladder Diagram (LD) Compiler to Common IEC Intermediate Representation.
//!
//! Transforms power-flow rungs, contacts, parallel branches, and coils into
//! deterministic IR statements and symbol declarations.

use crate::error::ControlError;
use crate::iec61131::ir::{IecExpr, IecStatement};
use crate::iec61131::ladder::model::{
    LdCoilKind, LdContactKind, LdDiagram, LdElement, LdParallelBranch,
};
use crate::iec61131::storage::SymbolTable;
use crate::iec61131::types::IecValue;

/// Compiles a Ladder Diagram into a SymbolTable and executable IR statements.
pub fn compile_ladder(
    diagram: &LdDiagram,
) -> Result<(SymbolTable, Vec<IecStatement>), ControlError> {
    let mut symbols = SymbolTable::new();
    for var in &diagram.variables {
        symbols.declare(var.clone())?;
    }

    let mut statements = Vec::new();

    for rung in &diagram.rungs {
        let (mut prep_stmts, power_expr) = compile_rung_elements(rung.number, &rung.elements)?;
        statements.append(&mut prep_stmts);

        for coil in &rung.coils {
            match coil.kind {
                LdCoilKind::Output => {
                    if coil.variable.starts_with("%Q") || coil.variable.starts_with("%q") {
                        statements.push(IecStatement::AssignIoOutput {
                            key: coil.variable.clone(),
                            expr: power_expr.clone(),
                        });
                    } else {
                        statements.push(IecStatement::Assign {
                            target: coil.variable.clone(),
                            expr: power_expr.clone(),
                        });
                    }
                }
                LdCoilKind::Negated => {
                    let neg_expr = IecExpr::Unary {
                        op: "NOT".to_string(),
                        expr: Box::new(power_expr.clone()),
                    };
                    if coil.variable.starts_with("%Q") || coil.variable.starts_with("%q") {
                        statements.push(IecStatement::AssignIoOutput {
                            key: coil.variable.clone(),
                            expr: neg_expr,
                        });
                    } else {
                        statements.push(IecStatement::Assign {
                            target: coil.variable.clone(),
                            expr: neg_expr,
                        });
                    }
                }
                LdCoilKind::Set => {
                    let assign_stmt =
                        if coil.variable.starts_with("%Q") || coil.variable.starts_with("%q") {
                            IecStatement::AssignIoOutput {
                                key: coil.variable.clone(),
                                expr: IecExpr::Literal(IecValue::Bool(true)),
                            }
                        } else {
                            IecStatement::Assign {
                                target: coil.variable.clone(),
                                expr: IecExpr::Literal(IecValue::Bool(true)),
                            }
                        };
                    statements.push(IecStatement::If {
                        condition: power_expr.clone(),
                        then_branch: vec![assign_stmt],
                        elsif_branches: Vec::new(),
                        else_branch: None,
                    });
                }
                LdCoilKind::Reset => {
                    let assign_stmt =
                        if coil.variable.starts_with("%Q") || coil.variable.starts_with("%q") {
                            IecStatement::AssignIoOutput {
                                key: coil.variable.clone(),
                                expr: IecExpr::Literal(IecValue::Bool(false)),
                            }
                        } else {
                            IecStatement::Assign {
                                target: coil.variable.clone(),
                                expr: IecExpr::Literal(IecValue::Bool(false)),
                            }
                        };
                    statements.push(IecStatement::If {
                        condition: power_expr.clone(),
                        then_branch: vec![assign_stmt],
                        elsif_branches: Vec::new(),
                        else_branch: None,
                    });
                }
            }
        }
    }

    Ok((symbols, statements))
}

fn compile_rung_elements(
    rung_num: usize,
    elements: &[LdElement],
) -> Result<(Vec<IecStatement>, IecExpr), ControlError> {
    if elements.is_empty() {
        return Ok((Vec::new(), IecExpr::Literal(IecValue::Bool(true))));
    }

    let mut prep_stmts = Vec::new();
    let mut exprs = Vec::new();

    for (idx, elem) in elements.iter().enumerate() {
        match elem {
            LdElement::Contact(c) => {
                let var_expr = if c.variable.starts_with("%I") || c.variable.starts_with("%i") {
                    IecExpr::IoInput(c.variable.clone())
                } else if c.variable.starts_with("%Q") || c.variable.starts_with("%q") {
                    IecExpr::IoOutput(c.variable.clone())
                } else {
                    IecExpr::Var(c.variable.clone())
                };

                match c.kind {
                    LdContactKind::NormallyOpen => {
                        exprs.push(var_expr);
                    }
                    LdContactKind::NormallyClosed => {
                        exprs.push(IecExpr::Unary {
                            op: "NOT".to_string(),
                            expr: Box::new(var_expr),
                        });
                    }
                    LdContactKind::PositiveTransition => {
                        let trig_name = format!("_RTRIG_RUNG_{rung_num}_{idx}");
                        prep_stmts.push(IecStatement::FbCall {
                            instance: trig_name.clone(),
                            fb_type: "R_TRIG".to_string(),
                            params: vec![("CLK".to_string(), var_expr)],
                        });
                        exprs.push(IecExpr::FieldAccess {
                            struct_var: trig_name,
                            field: "Q".to_string(),
                        });
                    }
                    LdContactKind::NegativeTransition => {
                        let trig_name = format!("_FTRIG_RUNG_{rung_num}_{idx}");
                        prep_stmts.push(IecStatement::FbCall {
                            instance: trig_name.clone(),
                            fb_type: "F_TRIG".to_string(),
                            params: vec![("CLK".to_string(), var_expr)],
                        });
                        exprs.push(IecExpr::FieldAccess {
                            struct_var: trig_name,
                            field: "Q".to_string(),
                        });
                    }
                }
            }
            LdElement::Parallel(branch) => {
                let (branch_stmts, branch_expr) = compile_parallel_branch(rung_num, branch)?;
                prep_stmts.extend(branch_stmts);
                exprs.push(branch_expr);
            }
            LdElement::FbCall {
                instance,
                fb_type,
                inputs,
            } => {
                let params = inputs
                    .iter()
                    .map(|(k, v)| {
                        let expr = if v.starts_with("%I") || v.starts_with("%i") {
                            IecExpr::IoInput(v.clone())
                        } else if v.starts_with("%Q") || v.starts_with("%q") {
                            IecExpr::IoOutput(v.clone())
                        } else {
                            IecExpr::Var(v.clone())
                        };
                        (k.clone(), expr)
                    })
                    .collect();

                prep_stmts.push(IecStatement::FbCall {
                    instance: instance.clone(),
                    fb_type: fb_type.clone(),
                    params,
                });
                exprs.push(IecExpr::FieldAccess {
                    struct_var: instance.clone(),
                    field: "Q".to_string(),
                });
            }
        }
    }

    let combined_expr = combine_and(exprs);
    Ok((prep_stmts, combined_expr))
}

fn compile_parallel_branch(
    rung_num: usize,
    branch: &LdParallelBranch,
) -> Result<(Vec<IecStatement>, IecExpr), ControlError> {
    if branch.paths.is_empty() {
        return Ok((Vec::new(), IecExpr::Literal(IecValue::Bool(false))));
    }

    let mut prep_stmts = Vec::new();
    let mut path_exprs = Vec::new();

    for path in &branch.paths {
        let (path_stmts, p_expr) = compile_rung_elements(rung_num, path)?;
        prep_stmts.extend(path_stmts);
        path_exprs.push(p_expr);
    }

    let combined_or = combine_or(path_exprs);
    Ok((prep_stmts, combined_or))
}

fn combine_and(mut exprs: Vec<IecExpr>) -> IecExpr {
    if exprs.is_empty() {
        return IecExpr::Literal(IecValue::Bool(true));
    }
    let first = exprs.remove(0);
    exprs.into_iter().fold(first, |acc, item| IecExpr::Binary {
        op: "AND".to_string(),
        left: Box::new(acc),
        right: Box::new(item),
    })
}

fn combine_or(mut exprs: Vec<IecExpr>) -> IecExpr {
    if exprs.is_empty() {
        return IecExpr::Literal(IecValue::Bool(false));
    }
    let first = exprs.remove(0);
    exprs.into_iter().fold(first, |acc, item| IecExpr::Binary {
        op: "OR".to_string(),
        left: Box::new(acc),
        right: Box::new(item),
    })
}
