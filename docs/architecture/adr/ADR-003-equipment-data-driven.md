---
title: ADR-003: Data-Driven Equipment Models
status: accepted
date: 2026-09-05
author: NebuDCS Architecture Team
---

# ADR-003: Data-Driven Equipment Models

## Status

Accepted

## Context

NebuDCS must support a vast array of process equipment (various pumps, compressors, columns, reactors). Hardcoding a Rust `struct` and implementation block for every single specific variant (e.g., `TwoStageCompressor`, `ThreeStageCompressor`, `MainPump`, `AuxPump`) would lead to explosion of source code, making the engine inflexible and difficult to maintain.

## Problem

How should equipment models be architected to allow maximum flexibility without modifying Rust source code for every variation?

## Decision

We will use a **Data-Driven Equipment Framework**.
- There will be a strict separation between `EquipmentDefinition` (the template), `EquipmentInstance` (the configuration), and `RuntimeState` (the dynamic simulation state).
- Generic primitive models (e.g., `GenericPump`, `GenericVessel`, `GenericCompressor`) will be implemented in Rust.
- Specific equipment variants (e.g., "H2 Compressor") are constructed by configuring parameters, ports, and attached logic on a generic primitive.

## Alternatives Considered

- **Code-First (Class/Struct per variant)**: Creating `struct H2Compressor`, `struct NASHCompressor`. Strong type safety but requires recompilation for new variants and is highly coupled.
- **ECS (Entity Component System)**: Using a strict ECS (like `bevy_ecs`). Good for games, but process simulation requires strict directed graphs (flow) and complex algebraic solvers that don't always map well to pure ECS.

## Rationale

A data-driven approach aligns with the requirement that "NebuDCS must NOT be hardcoded around a single refinery configuration." It allows process engineers to define new equipment templates via configuration files (JSON/YAML) and scripts without touching the Rust core.

## Trade-offs

- Slightly lower performance due to dynamic dispatch or generic parameter lookups compared to highly optimized specific structs.
- Validation logic becomes more complex, as errors (e.g., missing ports) are caught at runtime (project load) rather than compile time.

## Consequences

- The core simulation engine remains small and stable.
- Unit templates (NHT, RFCC) can be distributed as purely data packages.
- The UI can automatically generate configuration faceplates based on `EquipmentDefinition` metadata.

## Rejected Alternatives

- Code-first was rejected as it violates the core requirement of being a generic platform.
- Strict ECS was rejected as it complicates iterative numerical solvers that require traversing the process topology graph.

## Future Reconsideration Criteria

If the dynamic lookups and configuration parsing cause unacceptable performance overhead in the simulation loop, we may reconsider code-generation (e.g., generating Rust code from configuration files during a build step).
