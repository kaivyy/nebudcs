//! Deterministic Execution Virtual Machine for IEC 61131-3 Logic.
//!
//! Executes the Common Intermediate Representation (IR) with strict
//! bounds checking, iteration guards, and integration with the I/O Image Table.

use std::collections::BTreeMap;

use nebudcs_model::tag::TagQuality;

use crate::error::ControlError;
use crate::iec61131::fb::FunctionBlockInstance;
use crate::iec61131::ir::{IecExpr, IecStatement};
use crate::iec61131::storage::MemoryStore;
use crate::iec61131::types::IecValue;
use crate::runtime::io_image::IoImageTable;

/// Maximum iterations allowed per loop to prevent infinite scan freezes.
pub const MAX_LOOP_ITERATIONS: usize = 1000;

/// Flow control signal during statement evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowControl {
    Continue,
    Exit,
    Return,
}

/// Deterministic Virtual Machine executing IEC 61131-3 IR.
pub struct IecVm<'a> {
    pub dt: f64,
    pub sim_time: f64,
    pub memory: &'a mut MemoryStore,
    pub io_image: &'a mut IoImageTable,
    pub fb_instances: &'a mut BTreeMap<String, Box<dyn FunctionBlockInstance>>,
    pub step_counter: usize,
}

impl<'a> IecVm<'a> {
    /// Creates a new VM execution context for the current scan cycle.
    pub fn new(
        dt: f64,
        sim_time: f64,
        memory: &'a mut MemoryStore,
        io_image: &'a mut IoImageTable,
        fb_instances: &'a mut BTreeMap<String, Box<dyn FunctionBlockInstance>>,
    ) -> Self {
        Self {
            dt,
            sim_time,
            memory,
            io_image,
            fb_instances,
            step_counter: 0,
        }
    }

    /// Evaluates an expression to an `IecValue`.
    pub fn eval_expr(&mut self, expr: &IecExpr) -> Result<IecValue, ControlError> {
        self.step_counter += 1;
        match expr {
            IecExpr::Literal(val) => Ok(val.clone()),
            IecExpr::Var(name) => {
                if name.eq_ignore_ascii_case("_DT") {
                    return Ok(IecValue::Time(self.dt));
                }
                if name.eq_ignore_ascii_case("_TIME") {
                    return Ok(IecValue::Time(self.sim_time));
                }
                // Check local/program memory first
                if let Some(val) = self.memory.get(name) {
                    Ok(val.clone())
                } else if name.starts_with("%I") || name.starts_with("%i") {
                    // Implicit input point
                    let val = self.io_image.read_input_value(name).unwrap_or(0.0);
                    Ok(IecValue::LReal(val))
                } else if name.starts_with("%Q") || name.starts_with("%q") {
                    // Implicit output readback
                    let val = self.io_image.read_output_value(name).unwrap_or(0.0);
                    Ok(IecValue::LReal(val))
                } else {
                    // Unknown variable defaults to zero/false rather than crashing scan
                    Ok(IecValue::LReal(0.0))
                }
            }
            IecExpr::IoInput(key) => {
                let val = self.io_image.read_input_value(key).unwrap_or(0.0);
                Ok(IecValue::LReal(val))
            }
            IecExpr::IoOutput(key) => {
                let val = self.io_image.read_output_value(key).unwrap_or(0.0);
                Ok(IecValue::LReal(val))
            }
            IecExpr::Unary { op, expr } => {
                let v = self.eval_expr(expr)?;
                v.unary_op(op)
            }
            IecExpr::Binary { op, left, right } => {
                let l = self.eval_expr(left)?;
                let r = self.eval_expr(right)?;
                match op.as_str() {
                    "=" | "==" | "<>" | "!=" | "<" | "<=" | ">" | ">=" => {
                        Ok(IecValue::Bool(l.compare_op(op, &r)?))
                    }
                    "AND" | "&" | "OR" | "XOR" => l.boolean_op(op, &r),
                    _ => l.binary_op(op, &r),
                }
            }
            IecExpr::FunctionCall { name, args } => {
                let evaluated_args: Result<Vec<IecValue>, ControlError> =
                    args.iter().map(|a| self.eval_expr(a)).collect();
                self.call_built_in_function(name, &evaluated_args?)
            }
            IecExpr::ArrayAccess { array, index } => {
                let idx_val = self.eval_expr(index)?;
                let idx = idx_val.as_i64()? as usize;
                if let Some(IecValue::Array(elems)) = self.memory.get(array) {
                    if idx < elems.len() {
                        Ok(elems[idx].clone())
                    } else {
                        Err(ControlError::InvalidParameter(format!(
                            "Array index {} out of bounds (size {})",
                            idx,
                            elems.len()
                        )))
                    }
                } else {
                    Err(ControlError::InvalidParameter(format!(
                        "Variable '{}' is not an array",
                        array
                    )))
                }
            }
            IecExpr::FieldAccess { struct_var, field } => {
                // If referencing a Function Block output (e.g. `TON_01.Q` or `TON_01.ET`)
                if let Some(fb) = self.fb_instances.get(struct_var) {
                    if let Some(out_val) = fb.get_output(field) {
                        return Ok(out_val);
                    }
                }
                // Check memory store struct
                if let Some(IecValue::Struct(fields)) = self.memory.get(struct_var) {
                    if let Some(v) = fields.get(field) {
                        Ok(v.clone())
                    } else {
                        Err(ControlError::InvalidParameter(format!(
                            "Struct '{}' has no field '{}'",
                            struct_var, field
                        )))
                    }
                } else {
                    Err(ControlError::InvalidParameter(format!(
                        "Variable or FB '{}' not found for field access",
                        struct_var
                    )))
                }
            }
        }
    }

    /// Evaluates a statement or block of statements, returning flow control indicators.
    pub fn execute_statement(&mut self, stmt: &IecStatement) -> Result<FlowControl, ControlError> {
        self.step_counter += 1;
        match stmt {
            IecStatement::Assign { target, expr } => {
                let val = self.eval_expr(expr)?;
                if target.starts_with("%Q") || target.starts_with("%q") {
                    // Direct assign to output image
                    let num = val.as_f64()?;
                    self.io_image
                        .write_output(target, num, TagQuality::Good, self.sim_time);
                } else {
                    self.memory.set(target, val);
                }
                Ok(FlowControl::Continue)
            }
            IecStatement::AssignIoOutput { key, expr } => {
                let val = self.eval_expr(expr)?;
                let num = val.as_f64()?;
                self.io_image
                    .write_output(key, num, TagQuality::Good, self.sim_time);
                Ok(FlowControl::Continue)
            }
            IecStatement::AssignArray { array, index, expr } => {
                let idx = self.eval_expr(index)?.as_i64()? as usize;
                let val = self.eval_expr(expr)?;
                if let Some(IecValue::Array(mut elems)) = self.memory.get(array).cloned() {
                    if idx < elems.len() {
                        elems[idx] = val;
                        self.memory.set(array, IecValue::Array(elems));
                        Ok(FlowControl::Continue)
                    } else {
                        Err(ControlError::InvalidParameter(format!(
                            "Array index {} out of bounds",
                            idx
                        )))
                    }
                } else {
                    Err(ControlError::InvalidParameter(format!(
                        "Variable '{}' is not an array",
                        array
                    )))
                }
            }
            IecStatement::AssignField {
                struct_var,
                field,
                expr,
            } => {
                let val = self.eval_expr(expr)?;
                if let Some(IecValue::Struct(mut fields)) = self.memory.get(struct_var).cloned() {
                    fields.insert(field.clone(), val);
                    self.memory.set(struct_var, IecValue::Struct(fields));
                    Ok(FlowControl::Continue)
                } else {
                    Err(ControlError::InvalidParameter(format!(
                        "Variable '{}' is not a struct",
                        struct_var
                    )))
                }
            }
            IecStatement::If {
                condition,
                then_branch,
                elsif_branches,
                else_branch,
            } => {
                if self.eval_expr(condition)?.as_bool()? {
                    return self.execute_block(then_branch);
                }
                for (elsif_cond, elsif_body) in elsif_branches {
                    if self.eval_expr(elsif_cond)?.as_bool()? {
                        return self.execute_block(elsif_body);
                    }
                }
                if let Some(else_body) = else_branch {
                    return self.execute_block(else_body);
                }
                Ok(FlowControl::Continue)
            }
            IecStatement::Case {
                selector,
                cases,
                else_branch,
            } => {
                let sel_val = self.eval_expr(selector)?;
                for (match_vals, body) in cases {
                    for m in match_vals {
                        if sel_val.compare_op("=", m)? {
                            return self.execute_block(body);
                        }
                    }
                }
                if let Some(else_body) = else_branch {
                    return self.execute_block(else_body);
                }
                Ok(FlowControl::Continue)
            }
            IecStatement::For {
                counter,
                from,
                to,
                by,
                body,
            } => {
                let start_val = self.eval_expr(from)?.as_i64()?;
                let end_val = self.eval_expr(to)?.as_i64()?;
                let step = if let Some(by_expr) = by {
                    self.eval_expr(by_expr)?.as_i64()?
                } else {
                    1
                };

                if step == 0 {
                    return Err(ControlError::InvalidParameter(
                        "FOR loop BY step cannot be zero".to_string(),
                    ));
                }

                let mut current = start_val;
                let mut iterations = 0;
                while (step > 0 && current <= end_val) || (step < 0 && current >= end_val) {
                    if iterations >= MAX_LOOP_ITERATIONS {
                        return Err(ControlError::InvalidParameter(format!(
                            "FOR loop exceeded maximum allowed iterations ({MAX_LOOP_ITERATIONS})"
                        )));
                    }
                    iterations += 1;
                    self.memory.set(counter, IecValue::Dint(current as i32));
                    match self.execute_block(body)? {
                        FlowControl::Continue => {}
                        FlowControl::Exit => break,
                        FlowControl::Return => return Ok(FlowControl::Return),
                    }
                    current += step;
                }
                Ok(FlowControl::Continue)
            }
            IecStatement::While { condition, body } => {
                let mut iterations = 0;
                while self.eval_expr(condition)?.as_bool()? {
                    if iterations >= MAX_LOOP_ITERATIONS {
                        return Err(ControlError::InvalidParameter(format!(
                            "WHILE loop exceeded maximum allowed iterations ({MAX_LOOP_ITERATIONS})"
                        )));
                    }
                    iterations += 1;
                    match self.execute_block(body)? {
                        FlowControl::Continue => {}
                        FlowControl::Exit => break,
                        FlowControl::Return => return Ok(FlowControl::Return),
                    }
                }
                Ok(FlowControl::Continue)
            }
            IecStatement::Repeat { body, until } => {
                let mut iterations = 0;
                loop {
                    if iterations >= MAX_LOOP_ITERATIONS {
                        return Err(ControlError::InvalidParameter(format!(
                            "REPEAT loop exceeded maximum allowed iterations ({MAX_LOOP_ITERATIONS})"
                        )));
                    }
                    iterations += 1;
                    match self.execute_block(body)? {
                        FlowControl::Continue => {}
                        FlowControl::Exit => break,
                        FlowControl::Return => return Ok(FlowControl::Return),
                    }
                    if self.eval_expr(until)?.as_bool()? {
                        break;
                    }
                }
                Ok(FlowControl::Continue)
            }
            IecStatement::FbCall {
                instance,
                fb_type,
                params,
            } => {
                let mut evaluated_inputs = BTreeMap::new();
                for (param_name, expr) in params {
                    evaluated_inputs.insert(param_name.clone(), self.eval_expr(expr)?);
                }

                // If instance does not exist in registry, instantiate default instance
                if !self.fb_instances.contains_key(instance) {
                    let new_fb: Box<dyn FunctionBlockInstance> = match fb_type
                        .to_ascii_uppercase()
                        .as_str()
                    {
                        "TON" => Box::new(crate::iec61131::fb::TonBlock::new(instance)),
                        "TOF" => Box::new(crate::iec61131::fb::TofBlock::new(instance)),
                        "TP" => Box::new(crate::iec61131::fb::TpBlock::new(instance)),
                        "CTU" => Box::new(crate::iec61131::fb::CtuBlock::new(instance)),
                        "CTD" => Box::new(crate::iec61131::fb::CtdBlock::new(instance)),
                        "R_TRIG" => Box::new(crate::iec61131::fb::RTrigBlock::new(instance)),
                        "F_TRIG" => Box::new(crate::iec61131::fb::FTrigBlock::new(instance)),
                        "SR" => Box::new(crate::iec61131::fb::SrBlock::new(instance)),
                        "RS" => Box::new(crate::iec61131::fb::RsBlock::new(instance)),
                        "PID" => Box::new(crate::iec61131::fb::PidIecBlock::new(
                            instance,
                            crate::pid::PidConfig::default(),
                        )),
                        "LEAD_LAG" => Box::new(crate::iec61131::fb::LeadLagBlock::new(instance)),
                        "FILTER_1ST" => {
                            Box::new(crate::iec61131::fb::FilterFirstOrderBlock::new(instance))
                        }
                        "RATIO" => Box::new(crate::iec61131::fb::RatioBlock::new(instance)),
                        "SEL_HIGH" => {
                            Box::new(crate::iec61131::fb::HighSelectorBlock::new(instance))
                        }
                        "SEL_LOW" => Box::new(crate::iec61131::fb::LowSelectorBlock::new(instance)),
                        "SPLIT_RANGE" => {
                            Box::new(crate::iec61131::fb::SplitRangeBlock::new(instance))
                        }
                        "VALVE_CHAR" => {
                            Box::new(crate::iec61131::fb::ValveCharBlock::new(instance))
                        }
                        "ANTI_SURGE" => {
                            Box::new(crate::iec61131::fb::AntiSurgeBlock::new(instance))
                        }
                        other => {
                            return Err(ControlError::InvalidParameter(format!(
                                "Unknown Function Block type '{other}'"
                            )))
                        }
                    };
                    self.fb_instances.insert(instance.clone(), new_fb);
                }

                if let Some(fb) = self.fb_instances.get_mut(instance) {
                    let outputs = fb.step(self.dt, self.sim_time, &evaluated_inputs)?;
                    // Also mirror outputs into memory store under `instance.param`
                    for (out_name, val) in outputs {
                        self.memory.set(format!("{instance}.{out_name}"), val);
                    }
                }
                Ok(FlowControl::Continue)
            }
            IecStatement::Return => Ok(FlowControl::Return),
            IecStatement::Exit => Ok(FlowControl::Exit),
        }
    }

    /// Executes a sequential block of statements.
    pub fn execute_block(&mut self, block: &[IecStatement]) -> Result<FlowControl, ControlError> {
        for stmt in block {
            let flow = self.execute_statement(stmt)?;
            if flow != FlowControl::Continue {
                return Ok(flow);
            }
        }
        Ok(FlowControl::Continue)
    }

    /// Executes standard IEC 61131-3 mathematical built-in functions.
    fn call_built_in_function(
        &self,
        name: &str,
        args: &[IecValue],
    ) -> Result<IecValue, ControlError> {
        match name.to_ascii_uppercase().as_str() {
            "ABS" => {
                let val = args
                    .first()
                    .ok_or_else(|| {
                        ControlError::InvalidParameter("ABS requires 1 argument".into())
                    })?
                    .as_f64()?;
                Ok(IecValue::LReal(val.abs()))
            }
            "SQRT" => {
                let val = args
                    .first()
                    .ok_or_else(|| {
                        ControlError::InvalidParameter("SQRT requires 1 argument".into())
                    })?
                    .as_f64()?;
                if val < 0.0 {
                    return Err(ControlError::InvalidParameter(
                        "SQRT of negative number".to_string(),
                    ));
                }
                Ok(IecValue::LReal(val.sqrt()))
            }
            "MIN" => {
                if args.len() < 2 {
                    return Err(ControlError::InvalidParameter(
                        "MIN requires at least 2 arguments".into(),
                    ));
                }
                let a = args[0].as_f64()?;
                let b = args[1].as_f64()?;
                Ok(IecValue::LReal(a.min(b)))
            }
            "MAX" => {
                if args.len() < 2 {
                    return Err(ControlError::InvalidParameter(
                        "MAX requires at least 2 arguments".into(),
                    ));
                }
                let a = args[0].as_f64()?;
                let b = args[1].as_f64()?;
                Ok(IecValue::LReal(a.max(b)))
            }
            "LIMIT" => {
                if args.len() != 3 {
                    return Err(ControlError::InvalidParameter(
                        "LIMIT requires 3 arguments: (MN, IN, MX)".into(),
                    ));
                }
                let mn = args[0].as_f64()?;
                let input = args[1].as_f64()?;
                let mx = args[2].as_f64()?;
                Ok(IecValue::LReal(input.clamp(mn, mx)))
            }
            "LN" => {
                let val = args
                    .first()
                    .ok_or_else(|| ControlError::InvalidParameter("LN requires 1 argument".into()))?
                    .as_f64()?;
                Ok(IecValue::LReal(val.ln()))
            }
            "EXP" => {
                let val = args
                    .first()
                    .ok_or_else(|| {
                        ControlError::InvalidParameter("EXP requires 1 argument".into())
                    })?
                    .as_f64()?;
                Ok(IecValue::LReal(val.exp()))
            }
            "SIN" => {
                let val = args
                    .first()
                    .ok_or_else(|| {
                        ControlError::InvalidParameter("SIN requires 1 argument".into())
                    })?
                    .as_f64()?;
                Ok(IecValue::LReal(val.sin()))
            }
            "COS" => {
                let val = args
                    .first()
                    .ok_or_else(|| {
                        ControlError::InvalidParameter("COS requires 1 argument".into())
                    })?
                    .as_f64()?;
                Ok(IecValue::LReal(val.cos()))
            }
            "TRUNC" => {
                let val = args
                    .first()
                    .ok_or_else(|| {
                        ControlError::InvalidParameter("TRUNC requires 1 argument".into())
                    })?
                    .as_f64()?;
                Ok(IecValue::Dint(val.trunc() as i32))
            }
            other => Err(ControlError::InvalidParameter(format!(
                "Unknown built-in function '{other}'"
            ))),
        }
    }
}
