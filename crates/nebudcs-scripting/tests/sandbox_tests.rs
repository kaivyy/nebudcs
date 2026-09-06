//! Security, determinism, and resource quota tests for the Rhai scripting sandbox.

use nebudcs_scripting::error::ScriptingError;
use nebudcs_scripting::sandbox::{RhaiSandbox, ScriptExecutionContext};

#[test]
fn test_sandbox_safe_math_execution() {
    let sandbox = RhaiSandbox::new();
    let script = r#"
        fn step(inputs, params, state, dt) {
            let flow = inputs["flow"];
            let base_head = params["head_base"];
            let droop = params["droop"];
            
            let dp = base_head - droop * pow(flow, 2.0);
            state["delta_p"] = max(dp, 0.0);
            state["eff"] = clamp(0.85 - 0.001 * flow, 0.1, 0.95);
            state["sqrt_flow"] = sqrt(flow);
            return state;
        }
    "#;

    let ast = sandbox.compile_script(script).expect("compile succeeds");
    let mut ctx = ScriptExecutionContext {
        dt: 0.05,
        ..Default::default()
    };
    ctx.inputs.insert("flow".into(), 16.0);
    ctx.parameters.insert("head_base".into(), 100.0);
    ctx.parameters.insert("droop".into(), 0.2);

    let out = sandbox.execute_step(&ast, "step", &ctx).expect("executes");
    assert!((out.variables.get("delta_p").unwrap() - (100.0 - 0.2 * 256.0)).abs() < 1e-6);
    assert_eq!(*out.variables.get("sqrt_flow").unwrap(), 4.0);
}

#[test]
fn test_sandbox_deterministic_execution() {
    let sandbox = RhaiSandbox::new();
    let script = r#"
        fn step(inputs, params, state, dt) {
            let x = inputs["in_val"];
            state["out"] = sin(x) + cos(x) * exp(0.01 * x);
            return state;
        }
    "#;
    let ast = sandbox.compile_script(script).unwrap();

    let mut ctx = ScriptExecutionContext {
        dt: 0.05,
        ..Default::default()
    };
    ctx.inputs.insert("in_val".into(), 42.12345);

    let first = sandbox.execute_step(&ast, "step", &ctx).unwrap();

    for _ in 0..100 {
        let current = sandbox.execute_step(&ast, "step", &ctx).unwrap();
        assert_eq!(
            first, current,
            "Script output must be bit-identical across runs"
        );
    }
}

#[test]
fn test_sandbox_instruction_quota_aborts_infinite_loop() {
    let sandbox = RhaiSandbox::new();
    let script = r#"
        fn step(inputs, params, state, dt) {
            let counter = 0;
            while true {
                counter += 1;
            }
            state["counter"] = counter;
            return state;
        }
    "#;

    let ast = sandbox.compile_script(script).expect("syntax compiles");
    let ctx = ScriptExecutionContext::default();

    let res = sandbox.execute_step(&ast, "step", &ctx);
    assert!(
        matches!(res, Err(ScriptingError::InstructionQuotaExceeded(_))),
        "Infinite loop must be cleanly terminated by the 50,000 instruction limit"
    );
}

#[test]
fn test_sandbox_stack_recursion_limit() {
    let sandbox = RhaiSandbox::new();
    let script = r#"
        fn recursive_bomb(n) {
            return recursive_bomb(n + 1);
        }

        fn step(inputs, params, state, dt) {
            recursive_bomb(1);
            return state;
        }
    "#;

    let ast = sandbox.compile_script(script).unwrap();
    let ctx = ScriptExecutionContext::default();

    let res = sandbox.execute_step(&ast, "step", &ctx);
    assert!(
        res.is_err(),
        "Deep recursion must be caught before causing stack overflow"
    );
}

#[test]
fn test_sandbox_forbidden_keywords_rejected() {
    let sandbox = RhaiSandbox::new();

    // Rejects eval keyword
    let script_eval = "fn step(inputs, params, state, dt) { eval(\"1+1\"); return state; }";
    assert!(matches!(
        sandbox.compile_script(script_eval),
        Err(ScriptingError::Security(_))
    ));

    // Rejects import keyword
    let script_import = "import \"something\" as mod; fn step(i,p,s,dt) { return s; }";
    assert!(matches!(
        sandbox.compile_script(script_import),
        Err(ScriptingError::Security(_))
    ));
}

#[test]
fn test_sandbox_system_io_denied() {
    let sandbox = RhaiSandbox::new();

    // Trying to call non-existent host/file/OS functions
    let script_fs = r#"
        fn step(inputs, params, state, dt) {
            open_file("/etc/passwd");
            return state;
        }
    "#;
    let ast = sandbox.compile_script(script_fs).unwrap();
    let ctx = ScriptExecutionContext::default();

    let res = sandbox.execute_step(&ast, "step", &ctx);
    assert!(
        matches!(res, Err(ScriptingError::Execution(_))),
        "Arbitrary system/file calls must fail as undefined functions"
    );
}

#[test]
fn test_sandbox_rejects_non_finite_float_output() {
    let sandbox = RhaiSandbox::new();
    let script_inf = r#"
        fn step(inputs, params, state, dt) {
            state["bad_val"] = 1.0 / 0.0;
            return state;
        }
    "#;

    let ast = sandbox.compile_script(script_inf).unwrap();
    let ctx = ScriptExecutionContext::default();

    let res = sandbox.execute_step(&ast, "step", &ctx);
    assert!(
        matches!(res, Err(ScriptingError::Execution(_))),
        "Non-finite output (Infinity) must be rejected"
    );
}
