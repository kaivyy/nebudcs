---
title: ADR-007: Scripting with Rhai
status: accepted
date: 2026-09-05
author: NebuDCS Architecture Team
---

# ADR-007: Scripting with Rhai

## Status

Accepted

## Context

NebuDCS needs a way to define custom equipment behavior, complex scenario logic, and derived tags without requiring users to recompile the Rust simulation core. The scripting environment must be secure, sandboxed, and strictly deterministic.

## Problem

Which scripting language and engine should be embedded into the NebuDCS core?

## Decision

We will use **Rhai**, a safe, lightweight, embedded scripting language for Rust.

*Technology Version:* Rhai 1.19+ (as of 2026).
*Compile Cost:* Moderate (uses macros and AST evaluation).
*Binary Size Impact:* Low.
*Runtime/Memory Overhead:* Low (AST interpreter, no JIT).

## Alternatives Considered

- **Lua (via mlua)**: Extremely fast, industry standard for embedding. However, C-bindings are required, introducing `unsafe` code and potential security/memory vulnerabilities.
- **WebAssembly (Wasmtime/Wasmer)**: Extremely secure and fast, supports multiple languages (Rust, C, AssemblyScript). However, compiling Wasm requires external toolchains, which is too heavy for simple scenario logic.
- **Starlark**: Deterministic and safe, but syntax is heavily optimized for configuration rather than dynamic simulation logic loops.
- **Pure Declarative (No Scripting)**: Secure and fast, but not flexible enough for edge-case training scenarios or novel equipment types.

## Rationale

Rhai is written in pure Rust, ensuring memory safety. It integrates seamlessly with Rust types, allowing us to expose the `PlantModel` API safely. Most importantly, Rhai provides native mechanisms for strict sandboxing: we can limit the number of operations (preventing infinite loops) and memory usage. It does not have access to the OS, file system, or network by default.

## Trade-offs

- Rhai is an AST interpreter, so it is slower than LuaJIT or WebAssembly.
- The syntax is specific to Rhai (though very similar to Rust/JavaScript), meaning users have to learn a slightly new language.

## Consequences

- Script execution must be carefully budgeted within the 10ms simulation tick.
- Process engineers can write custom reactor logic or scenario triggers directly in the UI.
- Scripts cannot crash the simulation engine via memory corruption.

## Rejected Alternatives

- Lua was rejected due to the requirement to avoid C-FFI and `unsafe` where possible.
- WebAssembly was rejected because it requires a complex compilation toolchain for users.

## Future Reconsideration Criteria

If Rhai performance becomes a bottleneck for complex custom thermodynamics scripts, we will evaluate integrating WebAssembly specifically for computationally intensive custom equipment models, while keeping Rhai for high-level scenario logic.
