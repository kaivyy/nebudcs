---
title: Phase 1 Foundation
version: 1.0.0
status: implemented
date: 2026-09-05
---

# GOAL
The smallest justified Rust workspace that makes all subsequent phases possible without architectural redesign.

# WORKSPACE STRUCTURE
```toml
[workspace]
resolver = "2"
members = [
    "crates/nebudcs-types",
    "crates/nebudcs-units",
    "crates/nebudcs-model",
]
```
Justification: These three crates represent the pure data and type foundation. No behavior (simulation, topology) is introduced yet, ensuring absolute stability of the core types.

# CRATE-BY-CRATE SPECIFICATION

## `nebudcs-types`
- **Purpose:** Provide the fundamental strongly-typed IDs, time representation, and errors.
- **Forbidden imports:** No workspace dependencies.
- **External dependencies:** `serde`, `uuid` (or `ulid`), `thiserror`.
- **Types to implement:**
  - `EntityId(Ulid)`
  - Newtypes: `ProjectId`, `PlantId`, `AreaId`, `UnitId`, `EquipmentId`, `StreamId`, `TagId`, `PortId`
  - `SimulationTime(f64)`
  - `SchemaVersion(u32, u32, u32)`
  - `NebuError`, `NebuResult<T>`
- **File layout:** `src/ids.rs`, `src/time.rs`, `src/error.rs`, `src/lib.rs`
- **Tests:** ID uniqueness, type non-interchangeability.

## `nebudcs-units`
- **Purpose:** Strongly typed physical quantities preventing dimension mixing.
- **Depends on:** `nebudcs-types` only.
- **External dependencies:** `serde`, `thiserror`.
- **Implementation approach:** Newtype-per-quantity. Not a generic framework to keep it simple and compile-time fast for Phase 1.
- **Types:**
  - `Pressure(f64)`, `Temperature(f64)`, `MassFlow(f64)`, `VolumetricFlow(f64)`, `Power(f64)`, `Level(f64)`, `Volume(f64)`, `Speed(f64)`, `Vibration(f64)`, `DifferentialPressure(f64)`.
  - `DisplayUnit` enums (e.g., `PressureUnit`).
  - `to_display`, `from_display`.
  - `UnitAwareValue`, `QuantityKind`.
- **File layout:** `src/quantities.rs`, `src/display.rs`, `src/conversion.rs`, `src/lib.rs`
- **Tests:** Round-trip conversions.

## `nebudcs-model`
- **Purpose:** Project hierarchy DTOs, EquipmentDefinition schema.
- **Depends on:** `nebudcs-types`, `nebudcs-units`.
- **External dependencies:** `serde`, `serde_json`, `toml`, `indexmap`.
- **Types:**
  - `Project`, `Plant`, `Area`, `Unit`.
  - `EquipmentDefinition` with fields: `id`, `name`, `version`, `category`, `capabilities`, `parameters`, `ports`, `tags`, `operating_envelope`, `alarm_definitions`, `failure_modes`, `state_machine`, `metadata`.
  - Enums: `EquipmentCategory`, `Capability`, `PortDomain`, `PortDirection`, `PortMedia`.
  - Support structs: `ParameterDefinition`, `PortDefinition`, `TagDefinition`, `OperatingEnvelope`, `AlarmDefinition`, `FailureModeDefinition`, `StateMachineDefinition`.
  - `EquipmentInstance`, `StreamDefinition`, `StreamType`, `StreamComposition`.
  - `TagValue`, `TagQuality`, `TagSnapshot`.
  - `ValidationResult`, `ValidationSeverity`.
- **File layout:** `src/project.rs`, `src/equipment.rs`, `src/stream.rs`, `src/tag.rs`, `src/validation.rs`, `src/lib.rs`
- **Tests:** Serde round-trip, project hierarchy construction.

# CARGO.TOML CONTENT
(Standard boilerplate with specified dependencies and exact versions will be created by the coding agent.)

# PHASE 1 FILE TREE
```
/root/NebuDCS/
  Cargo.toml
  .gitignore
  .github/workflows/ci.yml
  crates/
    nebudcs-types/
      Cargo.toml
      src/
        lib.rs
        ids.rs
        time.rs
        error.rs
    nebudcs-units/
      Cargo.toml
      src/
        lib.rs
        quantities.rs
        display.rs
        conversion.rs
    nebudcs-model/
      Cargo.toml
      src/
        lib.rs
        project.rs
        equipment.rs
        stream.rs
        tag.rs
        validation.rs
```

# EXPLICITLY EXCLUDED
- `nebudcs-topology`: Belongs in Phase 2.
- `nebudcs-sim`: Belongs in Phase 3.
- `nebudcs-physics`: Belongs in Phase 4.

# EXIT CRITERIA
- `cargo build --workspace` passes with zero errors/warnings.
- `cargo test --workspace` passes.
- `cargo clippy --workspace -- -D warnings` passes.
- `cargo fmt --check --workspace` passes.
- All types serialize/deserialize properly.
- IDs are strongly typed and not interchangeable.
