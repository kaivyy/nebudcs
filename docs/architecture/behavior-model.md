---
title: Behavior Model Architecture
version: 0.1.0
status: draft
date: 2026-09-05
---

# Behavior Model Architecture

The behavior model defines how equipment dynamically acts, reacts, and fails.

## 1. Behavioral Definitions

Behaviors are defined through:
- Hardcoded robust physical equations (for standard fidelity).
- Lookup tables and performance maps.
- Configurable DSLs or sandboxed scripting for custom logic.

## 2. Failure Mode Architecture

Failures (mechanical, electrical, process, instrumentation, degradation) are first-class configurations. They alter the parameters of the behavioral definitions during the simulation tick rather than being mere UI animations.

```mermaid
stateDiagram-v2
    Healthy --> Degraded : Degradation Trigger
    Degraded --> Failed : Threshold Exceeded
    Healthy --> Failed : Critical Event
    Failed --> Healthy : Maintenance/Reset
```

## 3. Causal & Failure Propagation

Propagation emerges from physical relationships, not hardcoded chains.

```mermaid
graph TD
    A[Tank Level Decrease] --> B[Suction Pressure Drop]
    B --> C[NPSHA < NPSHR]
    C --> D[Cavitation]
    D --> E[Vibration Increase]
    E --> F[Vibration Alarm/Trip]
    F --> G[Downstream Flow Loss]
```

## 4. Failure Propagation through Utility Networks

Utility failures propagate naturally into process variations.

```mermaid
graph LR
    IA[Instrument Air Drop] --> V[Control Valve Closes]
    V --> P[Process Flow Decrease]
```

## 5. Auxiliary System Dependencies

Equipment depends on utility and auxiliary networks. 

```mermaid
graph LR
    LO[Lube Oil Pump Failure] --> P[Pressure Drop]
    P --> B[Bearing Temp Rise]
    B --> T[Turbine Trip]
    T --> P2[Driven Equipment Shutdown]
```

## 6. Dynamic Equipment Behavior

Dynamic relationships are non-linear. For a compressor, speed impacts flow, pressure, power, efficiency, vibration, and surge margin simultaneously based on performance maps.
