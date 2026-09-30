//! Sequential Function Chart (SFC) Compiler to Common IEC Intermediate Representation.
//!
//! Transforms steps, transitions, action qualifiers (N, S, R, P, L, D), and sequence
//! networks into deterministic IR execution statements and symbol declarations.

use crate::error::ControlError;
use crate::iec61131::ir::{IecExpr, IecStatement};
use crate::iec61131::sfc::model::{SfcActionQualifier, SfcChart};
use crate::iec61131::st::lexer::Lexer;
use crate::iec61131::st::parser::Parser;
use crate::iec61131::storage::{SymbolTable, VariableDeclaration};
use crate::iec61131::types::{IecType, IecValue};

/// Compiles an SFC chart into a SymbolTable and executable IR statements.
pub fn compile_sfc(chart: &SfcChart) -> Result<(SymbolTable, Vec<IecStatement>), ControlError> {
    let mut symbols = SymbolTable::new();

    // Register user variables
    for var in &chart.variables {
        symbols.declare(var.clone())?;
    }

    // Register internal SFC state variables
    for step in &chart.steps {
        // Step active flag: _SFC_X_<step_name>
        let x_name = format!("_SFC_X_{}", step.name);
        if symbols.lookup(&x_name).is_none() {
            symbols.declare(
                VariableDeclaration::new(&x_name, IecType::Bool)
                    .with_initial_value(IecValue::Bool(step.is_initial)),
            )?;
        }

        // Step elapsed timer: _SFC_T_<step_name>
        let t_name = format!("_SFC_T_{}", step.name);
        if symbols.lookup(&t_name).is_none() {
            symbols.declare(
                VariableDeclaration::new(&t_name, IecType::Time)
                    .with_initial_value(IecValue::Time(0.0)),
            )?;
        }

        // Step pulse entry flag: _SFC_P_<step_name>
        let p_name = format!("_SFC_P_{}", step.name);
        if symbols.lookup(&p_name).is_none() {
            symbols.declare(
                VariableDeclaration::new(&p_name, IecType::Bool)
                    .with_initial_value(IecValue::Bool(step.is_initial)),
            )?;
        }

        // Next-scan scratch flag: _SFC_NEXT_X_<step_name>
        let next_x_name = format!("_SFC_NEXT_X_{}", step.name);
        if symbols.lookup(&next_x_name).is_none() {
            symbols.declare(
                VariableDeclaration::new(&next_x_name, IecType::Bool)
                    .with_initial_value(IecValue::Bool(step.is_initial)),
            )?;
        }
    }

    // Register any undeclared action target variables as BOOL
    for act in &chart.actions {
        if !act.target_var.starts_with("%") && symbols.lookup(&act.target_var).is_none() {
            symbols.declare(VariableDeclaration::new(&act.target_var, IecType::Bool))?;
        }
    }

    let mut statements: Vec<IecStatement> = Vec::new();

    // 1. Accumulate step timers
    for step in &chart.steps {
        let x_var = format!("_SFC_X_{}", step.name);
        let t_var = format!("_SFC_T_{}", step.name);
        let p_var = format!("_SFC_P_{}", step.name);

        // Reset pulse entry flag from previous scan
        statements.push(IecStatement::Assign {
            target: p_var,
            expr: IecExpr::Literal(IecValue::Bool(false)),
        });

        // Update elapsed timer: IF _SFC_X_step THEN _SFC_T_step := _SFC_T_step + _DT ELSE 0.0; END_IF;
        statements.push(IecStatement::If {
            condition: IecExpr::Var(x_var),
            then_branch: vec![IecStatement::Assign {
                target: t_var.clone(),
                expr: IecExpr::Binary {
                    op: "+".to_string(),
                    left: Box::new(IecExpr::Var(t_var.clone())),
                    right: Box::new(IecExpr::Var("_DT".to_string())),
                },
            }],
            elsif_branches: Vec::new(),
            else_branch: Some(vec![IecStatement::Assign {
                target: t_var,
                expr: IecExpr::Literal(IecValue::Time(0.0)),
            }]),
        });
    }

    // 2. Initialize scratch next flags with current step active states
    for step in &chart.steps {
        statements.push(IecStatement::Assign {
            target: format!("_SFC_NEXT_X_{}", step.name),
            expr: IecExpr::Var(format!("_SFC_X_{}", step.name)),
        });
    }

    // 3. Evaluate transitions in priority order
    let mut sorted_transitions = chart.transitions.clone();
    sorted_transitions.sort_by_key(|t| t.priority);

    for trans in sorted_transitions {
        let cond_expr = parse_condition_expression(&trans.condition_expr)?;
        let fire_condition = IecExpr::Binary {
            op: "AND".to_string(),
            left: Box::new(IecExpr::Var(format!("_SFC_X_{}", trans.from_step))),
            right: Box::new(cond_expr),
        };

        let transition_actions = vec![
            IecStatement::Assign {
                target: format!("_SFC_NEXT_X_{}", trans.from_step),
                expr: IecExpr::Literal(IecValue::Bool(false)),
            },
            IecStatement::Assign {
                target: format!("_SFC_NEXT_X_{}", trans.to_step),
                expr: IecExpr::Literal(IecValue::Bool(true)),
            },
            IecStatement::Assign {
                target: format!("_SFC_P_{}", trans.to_step),
                expr: IecExpr::Literal(IecValue::Bool(true)),
            },
            IecStatement::Assign {
                target: format!("_SFC_T_{}", trans.to_step),
                expr: IecExpr::Literal(IecValue::Time(0.0)),
            },
        ];

        statements.push(IecStatement::If {
            condition: fire_condition,
            then_branch: transition_actions,
            elsif_branches: Vec::new(),
            else_branch: None,
        });
    }

    // 4. Commit next-state scratch flags to current step state
    for step in &chart.steps {
        statements.push(IecStatement::Assign {
            target: format!("_SFC_X_{}", step.name),
            expr: IecExpr::Var(format!("_SFC_NEXT_X_{}", step.name)),
        });
    }

    // 5. Execute step actions
    for action in &chart.actions {
        let x_var = format!("_SFC_X_{}", action.step_name);
        let p_var = format!("_SFC_P_{}", action.step_name);
        let t_var = format!("_SFC_T_{}", action.step_name);

        match action.qualifier {
            SfcActionQualifier::N => {
                // Non-stored: active continuously while step is active
                statements.push(create_assignment(&action.target_var, IecExpr::Var(x_var)));
            }
            SfcActionQualifier::S => {
                // Set / Stored: latched true on pulse
                statements.push(IecStatement::If {
                    condition: IecExpr::Var(p_var),
                    then_branch: vec![create_assignment(
                        &action.target_var,
                        IecExpr::Literal(IecValue::Bool(true)),
                    )],
                    elsif_branches: Vec::new(),
                    else_branch: None,
                });
            }
            SfcActionQualifier::R => {
                // Reset: unlatched false on pulse
                statements.push(IecStatement::If {
                    condition: IecExpr::Var(p_var),
                    then_branch: vec![create_assignment(
                        &action.target_var,
                        IecExpr::Literal(IecValue::Bool(false)),
                    )],
                    elsif_branches: Vec::new(),
                    else_branch: None,
                });
            }
            SfcActionQualifier::P => {
                // Pulse: active for 1 scan on entry
                statements.push(create_assignment(&action.target_var, IecExpr::Var(p_var)));
            }
            SfcActionQualifier::L => {
                // Time Limited: active while step is active, up to duration
                let duration = action.duration.unwrap_or(1.0);
                let active_expr = IecExpr::Binary {
                    op: "AND".to_string(),
                    left: Box::new(IecExpr::Var(x_var)),
                    right: Box::new(IecExpr::Binary {
                        op: "<=".to_string(),
                        left: Box::new(IecExpr::Var(t_var)),
                        right: Box::new(IecExpr::Literal(IecValue::Time(duration))),
                    }),
                };
                statements.push(create_assignment(&action.target_var, active_expr));
            }
            SfcActionQualifier::D => {
                // Time Delayed: active while step is active, after duration
                let duration = action.duration.unwrap_or(1.0);
                let active_expr = IecExpr::Binary {
                    op: "AND".to_string(),
                    left: Box::new(IecExpr::Var(x_var)),
                    right: Box::new(IecExpr::Binary {
                        op: ">=".to_string(),
                        left: Box::new(IecExpr::Var(t_var)),
                        right: Box::new(IecExpr::Literal(IecValue::Time(duration))),
                    }),
                };
                statements.push(create_assignment(&action.target_var, active_expr));
            }
        }
    }

    Ok((symbols, statements))
}

fn create_assignment(target: &str, expr: IecExpr) -> IecStatement {
    if target.starts_with("%Q") || target.starts_with("%q") {
        IecStatement::AssignIoOutput {
            key: target.to_string(),
            expr,
        }
    } else {
        IecStatement::Assign {
            target: target.to_string(),
            expr,
        }
    }
}

fn parse_condition_expression(cond_str: &str) -> Result<IecExpr, ControlError> {
    let trimmed = cond_str.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("TRUE") || trimmed == "1" {
        return Ok(IecExpr::Literal(IecValue::Bool(true)));
    }
    if trimmed.eq_ignore_ascii_case("FALSE") || trimmed == "0" {
        return Ok(IecExpr::Literal(IecValue::Bool(false)));
    }

    let mut lexer = Lexer::new(trimmed);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    parser.parse_expr()
}
