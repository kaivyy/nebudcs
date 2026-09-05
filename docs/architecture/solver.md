---
title: Solver Architecture
version: 0.1.0
status: draft
date: 2026-09-05
---

# Solver Architecture

Numerical solvers execute the simulation dynamically.

## 1. Abstractions

The system decouples the domain model from the mathematical solver (equations, variables, constraints, residuals, solver, convergence, tolerances, diagnostics), allowing DAE and non-linear solvers to be introduced later.

```mermaid
graph TD
    Domain[Plant Topology & Equations] --> Var[Variables & Constraints]
    Var --> Solver[Numerical Solver]
    Solver --> Update[State Update]
```

## 2. Convergence State Machine

Tracks initialization and simulation validity.

```mermaid
stateDiagram-v2
    INVALID_MODEL --> VALID_UNCONVERGED
    VALID_UNCONVERGED --> PARTIALLY_INITIALIZED
    PARTIALLY_INITIALIZED --> CONVERGED
    CONVERGED --> VALID_UNCONVERGED : Disturbance
```

## 3. Simulation Execution Loop

Tick-based execution loop updating control, equipment, and checking convergence.

```mermaid
graph TD
    Tick --> ReadInputs[Read Operator Inputs]
    ReadInputs --> Control[Update Control Logic]
    Control --> Equipment[Evaluate Equipment]
    Equipment --> Solver[Solver / Recycle Iteration]
    Solver --> Write[Update Historian & Output]
```

## 4. Recycle & Initialization

- **Recycle loops**: Successive substitution, Wegstein, Broyden (configurable).
- **Initialization**: Steady-state solvers initialize the dynamic condition before transient simulation begins.
- **Simulation Modes**: Steady-state initialization, dynamic simulation, transient operation, startup, shutdown, emergency, trip.
- **Future Extensibility**: Domain model exposes equations generically so future DAE solvers can solve them implicitly.
