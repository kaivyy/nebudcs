---
title: Coding Agent Rules
version: 1.0.0
status: draft
date: 2026-09-05
---

# PREAMBLE
These rules apply to every agent that implements NebuDCS code. Violation of any rule is grounds to reject the implementation.

# BEFORE STARTING ANY IMPLEMENTATION TASK
1. Read the relevant architecture document(s) listed in the phase plan.
2. Read this file completely.
3. Read the dependency map (`dependency-map.md`).
4. Read the phase plan for the phase you are implementing.
5. Confirm you understand which crate you are working in and its allowed dependencies.
6. Confirm you understand the exit criteria for the phase.

# ARCHITECTURAL INVARIANTS
1. The simulation engine is the source of truth. HMI/P&ID are views only. No UI code may write to simulation state.
2. `EquipmentDefinition` is immutable and versioned once in use by a project. Never mutate it at runtime.
3. `RuntimeState` is strictly separate from `EquipmentDefinition` and from `EquipmentInstance` (configuration). Never conflate them.
4. Process-specific logic MUST NOT enter the simulation core. These are data packages.
5. Internal simulation state is strongly typed. No `HashMap<String, f64>` as authoritative process state.
6. The simulation tick loop is single-threaded and deterministic. No system time in the tick. No async/await in CPU-bound calculations.
7. Custom equipment must not require recompiling the simulation core.
8. Engineering units are internally canonical SI. Never store display units as canonical values. Never mix dimensions.
9. Topology is separate from P&ID layout.
10. Alarm limits reference the authoritative simulation state only.
11. Historian writes are asynchronous and non-blocking to the simulation tick.
12. Serialization boundaries are explicit. Use DTO boundaries.
13. Project versions must be reproducible.
14. Security boundaries are enforced. Scripts run in a sandbox.

# CODING RULES
1. Never import a crate that creates a dependency cycle.
2. Never add an external dependency without justification.
3. Never use `unwrap()` in production code. Use `?` or explicit error handling.
4. Never use `unsafe` without documented justification.
5. Never use `clone()` in the simulation hot loop. Pre-allocate.
6. Never allocate `String` or `Vec` in the simulation tick without justification.
7. Never use `HashMap<String, ...>` for internal process state.
8. Never use `f32` for simulation state values. Use `f64`.
9. Never make calculations time-dependent on system clocks. Use `SimulationTime`.
10. Never allow a lower-layer crate to import from a higher-layer crate.
11. Never store display units as canonical values.
12. Never duplicate domain types between crates.
13. Always write tests before or alongside implementation (TDD).
14. Always add `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]` to domain types unless justified.
15. Always handle all enum arms in `match` expressions.
16. Never change simulation behavior without updating tests and docs.
17. Each commit must be minimal and focused.

# WHEN ADDING A NEW DEPENDENCY
- State name and version.
- State why it is needed.
- State alternatives considered.
- State if it is dev-dependency.
- Verify no security advisories.
- Verify no feature flag conflicts.

# WHEN MAKING AN ARCHITECTURAL DEVIATION
- Document in a new ADR.
- Update architecture doc.
- Update traceability matrix.
- Get acknowledgment.

# WHEN A TEST FAILS
- Fix implementation, not the test (unless test is wrong).
- Never delete tests to pass builds.
- Never add `#[allow(...)]` globally.

# COMMIT MESSAGE FORMAT
```
feat(crate-name): brief description of what was added

Longer description if needed.

Phase: 1
Architecture: domain-model.md
Tests: test_pump_definition_fixture
```

# FORBIDDEN PATTERNS
- `HashMap<String, f64>` as process state.
- `HashMap<String, Box<dyn Any>>` as equipment state.
- `impl Trait for dyn AnyEquipment` with no type safety.
- Refinery-specific branches in core.
- `use tokio::...` in tick function.
- `use std::time::SystemTime` in physics.
- `unwrap()` in non-test code.
- `.clone()` inside hot loop.
- Direct mutation of `EquipmentDefinition` at runtime.
- GlobalState/AppState/WorldState conflating layers.
- CSS/HTML/JS in Rust logic.
- Crates importing from apps/.
