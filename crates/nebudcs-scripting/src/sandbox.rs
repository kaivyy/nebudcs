//! Sandboxed Rhai scripting environment for Level 3 custom equipment equations.
//!
//! Enforces:
//! - Complete denial of host filesystem, network, process, OS, and unsafe access.
//! - Hard instruction limit of 50,000 operations per execution step.
//! - Memory allocation and recursion depth constraints.
//! - Finite float validation preventing NaN or Infinity leakage into the simulation.

use rhai::{Dynamic, Engine, Scope, AST};
use std::collections::BTreeMap;

use crate::error::{ScriptingError, ScriptingResult};

/// Maximum allowed script operations per step (architectural invariant: 50,000).
pub const MAX_SCRIPT_OPERATIONS: u64 = 50_000;

/// Execution context passed into a custom equipment script.
#[derive(Debug, Clone, Default)]
pub struct ScriptExecutionContext {
    pub dt: f64,
    pub inputs: BTreeMap<String, f64>,
    pub parameters: BTreeMap<String, f64>,
    pub state_variables: BTreeMap<String, f64>,
    pub state_flags: BTreeMap<String, bool>,
}

/// Output produced by a custom equipment script execution.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScriptExecutionOutput {
    pub variables: BTreeMap<String, f64>,
    pub flags: BTreeMap<String, bool>,
    pub trip_commanded: bool,
    pub trip_reason: Option<String>,
}

/// Secure Rhai sandbox engine.
pub struct RhaiSandbox {
    engine: Engine,
}

impl Default for RhaiSandbox {
    fn default() -> Self {
        Self::new()
    }
}

impl RhaiSandbox {
    /// Creates a new strictly sandboxed Rhai engine.
    pub fn new() -> Self {
        let mut engine = Engine::new();

        // 1. Enforce strict instruction and resource limits
        engine.set_max_operations(MAX_SCRIPT_OPERATIONS);
        engine.set_max_call_levels(16);
        engine.set_max_expr_depths(32, 16);
        engine.set_max_string_size(256);
        engine.set_max_array_size(64);
        engine.set_max_map_size(64);

        // 2. Register safe mathematical primitives for process equations
        engine.register_fn("sqrt", |x: f64| -> f64 {
            if x < 0.0 {
                0.0
            } else {
                x.sqrt()
            }
        });
        engine.register_fn("pow", |x: f64, y: f64| -> f64 { x.powf(y) });
        engine.register_fn("exp", |x: f64| -> f64 { x.exp() });
        engine.register_fn("ln", |x: f64| -> f64 {
            if x <= 0.0 {
                -1e9
            } else {
                x.ln()
            }
        });
        engine.register_fn("log10", |x: f64| -> f64 {
            if x <= 0.0 {
                -1e9
            } else {
                x.log10()
            }
        });
        engine.register_fn("sin", |x: f64| -> f64 { x.sin() });
        engine.register_fn("cos", |x: f64| -> f64 { x.cos() });
        engine.register_fn("tan", |x: f64| -> f64 { x.tan() });
        engine.register_fn("abs", |x: f64| -> f64 { x.abs() });
        engine.register_fn("min", |x: f64, y: f64| -> f64 { x.min(y) });
        engine.register_fn("max", |x: f64, y: f64| -> f64 { x.max(y) });
        engine.register_fn("clamp", |x: f64, min: f64, max: f64| -> f64 {
            x.clamp(min, max)
        });
        engine.register_fn("floor", |x: f64| -> f64 { x.floor() });
        engine.register_fn("ceil", |x: f64| -> f64 { x.ceil() });
        engine.register_fn("round", |x: f64| -> f64 { x.round() });

        // 3. Register fallback operations for missing map entries (which evaluate to `()`)
        engine.register_fn("+", |_: (), y: f64| -> f64 { y });
        engine.register_fn("+", |x: f64, _: ()| -> f64 { x });
        engine.register_fn("-", |_: (), y: f64| -> f64 { -y });
        engine.register_fn("-", |x: f64, _: ()| -> f64 { x });
        engine.register_fn("*", |_: (), _: f64| -> f64 { 0.0 });
        engine.register_fn("*", |_: f64, _: ()| -> f64 { 0.0 });
        engine.register_fn("/", |_: (), y: f64| -> f64 {
            if y.abs() < 1e-12 {
                0.0
            } else {
                0.0 / y
            }
        });
        engine.register_fn("/", |x: f64, _: ()| -> f64 { x });
        engine.register_fn("pow", |_: (), y: f64| -> f64 { 0.0f64.powf(y) });
        engine.register_fn("pow", |x: f64, _: ()| -> f64 { x });
        engine.register_fn("exp", |_: ()| -> f64 { 1.0 });
        engine.register_fn("sqrt", |_: ()| -> f64 { 0.0 });
        engine.register_fn("clamp", |_: (), min: f64, max: f64| -> f64 {
            min.clamp(min, max)
        });
        engine.register_fn("max", |_: (), y: f64| -> f64 { y.max(0.0) });
        engine.register_fn("max", |x: f64, _: ()| -> f64 { x.max(0.0) });
        engine.register_fn("min", |_: (), y: f64| -> f64 { y.min(0.0) });
        engine.register_fn("min", |x: f64, _: ()| -> f64 { x.min(0.0) });
        engine.register_fn(">", |_: (), y: f64| -> bool { 0.0 > y });
        engine.register_fn(">", |x: f64, _: ()| -> bool { x > 0.0 });
        engine.register_fn("<", |_: (), y: f64| -> bool { 0.0 < y });
        engine.register_fn("<", |x: f64, _: ()| -> bool { x < 0.0 });
        engine.register_fn(">=", |_: (), y: f64| -> bool { 0.0 >= y });
        engine.register_fn(">=", |x: f64, _: ()| -> bool { x >= 0.0 });
        engine.register_fn("<=", |_: (), y: f64| -> bool { 0.0 <= y });
        engine.register_fn("<=", |x: f64, _: ()| -> bool { x <= 0.0 });
        engine.register_fn("+", |_: (), _: ()| -> f64 { 0.0 });
        engine.register_fn("-", |_: (), _: ()| -> f64 { 0.0 });
        engine.register_fn("*", |_: (), _: ()| -> f64 { 0.0 });
        engine.register_fn("/", |_: (), _: ()| -> f64 { 0.0 });
        engine.register_fn("==", |_: (), _: ()| -> bool { true });
        engine.register_fn("!=", |_: (), _: ()| -> bool { false });

        Self { engine }
    }

    /// Compiles and validates a script into an immutable AST.
    pub fn compile_script(&self, script: &str) -> ScriptingResult<AST> {
        // Pre-scan for obviously malicious or forbidden patterns
        let forbidden_keywords = ["eval", "import", "module", "curry"];
        for kw in forbidden_keywords {
            if script.contains(kw) {
                return Err(ScriptingError::Security(format!(
                    "Script contains forbidden keyword: '{kw}'"
                )));
            }
        }

        self.engine
            .compile(script)
            .map_err(|e| ScriptingError::Compilation(format!("Compilation error: {e}")))
    }

    /// Executes the script step function within the sandbox.
    pub fn execute_step(
        &self,
        ast: &AST,
        entry_point: &str,
        ctx: &ScriptExecutionContext,
    ) -> ScriptingResult<ScriptExecutionOutput> {
        // Prepare scope with input structures
        let mut scope = Scope::new();

        let mut inputs_map = rhai::Map::new();
        for (k, &v) in &ctx.inputs {
            inputs_map.insert(k.clone().into(), Dynamic::from(v));
        }

        let mut params_map = rhai::Map::new();
        for (k, &v) in &ctx.parameters {
            params_map.insert(k.clone().into(), Dynamic::from(v));
        }

        let mut state_map = rhai::Map::new();
        for (k, &v) in &ctx.state_variables {
            state_map.insert(k.clone().into(), Dynamic::from(v));
        }
        for (k, &v) in &ctx.state_flags {
            state_map.insert(k.clone().into(), Dynamic::from(v));
        }

        let dt_val = Dynamic::from(ctx.dt);

        // Call the entry function step(inputs, params, state, dt)
        let result: Dynamic = self
            .engine
            .call_fn(
                &mut scope,
                ast,
                entry_point,
                (inputs_map, params_map, state_map, dt_val),
            )
            .map_err(|e| match *e {
                rhai::EvalAltResult::ErrorTooManyOperations(_) => {
                    ScriptingError::InstructionQuotaExceeded(format!(
                        "Exceeded {MAX_SCRIPT_OPERATIONS} operation quota"
                    ))
                }
                rhai::EvalAltResult::ErrorStackOverflow(_) => {
                    ScriptingError::ResourceLimit("Call stack overflow".to_string())
                }
                rhai::EvalAltResult::ErrorDataTooLarge(msg, _) => {
                    ScriptingError::ResourceLimit(format!("Data size limit exceeded: {msg}"))
                }
                other => ScriptingError::Execution(format!("Script runtime error: {other}")),
            })?;

        // Extract outputs from the returned map
        let mut output = ScriptExecutionOutput::default();

        if let Some(map) = result.try_cast::<rhai::Map>() {
            for (key, val) in map {
                let key_str = key.to_string();
                if key_str == "trip" {
                    if let Ok(b) = val.as_bool() {
                        output.trip_commanded = b;
                    }
                } else if key_str == "trip_reason" {
                    output.trip_reason = Some(val.to_string());
                } else if let Ok(b) = val.as_bool() {
                    output.flags.insert(key_str, b);
                } else if let Ok(f) = val.as_float() {
                    if !f.is_finite() {
                        return Err(ScriptingError::Execution(format!(
                            "Script output variable '{key_str}' is non-finite: {f}"
                        )));
                    }
                    output.variables.insert(key_str, f);
                } else if let Ok(i) = val.as_int() {
                    output.variables.insert(key_str, i as f64);
                }
            }
        } else {
            return Err(ScriptingError::Execution(
                "Script entry point must return a map containing updated state".to_string(),
            ));
        }

        Ok(output)
    }
}
