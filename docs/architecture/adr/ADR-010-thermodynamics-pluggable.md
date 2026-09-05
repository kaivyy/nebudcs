---
title: ADR-010: Pluggable Thermodynamics and Properties
status: accepted
date: 2026-09-05
author: NebuDCS Architecture Team
---

# ADR-010: Pluggable Thermodynamics and Properties

## Status

Accepted

## Context

Simulating process plants requires calculating fluid properties (density, enthalpy, phase equilibrium) based on pressure, temperature, and composition. High-fidelity equations of state (like Peng-Robinson) are computationally expensive and complex to implement. A training simulator often requires fast, stable models rather than perfect chemical engineering accuracy.

## Problem

How should the thermodynamics calculations be integrated into the simulation engine?

## Decision

We will use a **Pluggable Property Package Architecture**.
- The simulation core will interact with thermodynamics strictly through a generic `PropertyPackage` trait.
- Initial development will use a **simplified empirical model** (ideal gas, constant heat capacity, simple liquid density) to establish the simulation framework and UI.
- The architecture will explicitly support swapping in high-fidelity libraries (like a Rust port of CoolProp or custom Peng-Robinson implementations) at the project or stream level.

## Alternatives Considered

- **Embed a Full Process Simulator Library (e.g., DWSIM/CAPE-OPEN)**: Extremely accurate, but heavy, slow, and often written in C# or C++, requiring unsafe FFI and breaking determinism/performance constraints.
- **Hardcode Peng-Robinson**: Difficult to implement robustly, computationally heavy, and often fails to converge during extreme transient states common in training simulators (e.g., total pressure collapse).

## Rationale

A training simulator requires stability above all else. If an operator opens a valve to flare and pressure drops to atmospheric instantly, a rigorous Equation of State solver might fail to converge and crash the simulation. A pluggable architecture allows us to use simplified, unconditionally stable models for general training, and high-fidelity models only when the specific scenario requires it (and where we can bound the inputs).

## Trade-offs

- The generic `PropertyPackage` trait might require frequent adjustments as we discover the needs of more complex EOS models.
- Maintaining multiple thermodynamic models increases testing burden.

## Consequences

- The platform can scale from simple water/air hydraulic simulations up to complex hydrocarbon distillation.
- We avoid getting bogged down in complex thermodynamic math during Phase 1-5 of the roadmap.

## Rejected Alternatives

- Embedding existing heavy simulators was rejected due to performance, language barriers (C++ FFI), and stability issues during transients.

## Future Reconsideration Criteria

If industry partners require CAPE-OPEN compatibility for their proprietary property packages, we will have to build an FFI bridge, but it will remain isolated behind the `PropertyPackage` trait and likely run in a separate worker thread to protect the core tick budget.
