---
title: ADR-009: Crate Architecture and Workspace Boundaries
status: accepted
date: 2026-09-05
author: NebuDCS Architecture Team
---

# ADR-009: Crate Architecture and Workspace Boundaries

## Status

Accepted (Updated to align with verified workspace baseline)

## Context

NebuDCS is a high-performance, deterministic process simulation, DCS automation, and training platform. A monolithic crate would lead to slow compilation cycles, tight coupling, and violation of the boundary between domain logic, numerical physics, and presentation layers. Microservices would introduce unacceptable network serialization latency, non-deterministic scheduling, and high deployment complexity for a synchronous 10ms simulation tick.

## Decision

We organize the Rust codebase into a **Cargo Workspace** with strictly unidirectional, layered dependency boundaries:

```mermaid
graph TD
    subgraph "Level 0: Types & Errors"
        Types[nebudcs-types]
    end

    subgraph "Level 1: Engineering Units"
        Units[nebudcs-units]
    end

    subgraph "Level 2: Domain Model & Hierarchy"
        Model[nebudcs-model]
    end

    subgraph "Level 3: Graph Topology"
        Topology[nebudcs-topology]
    end

    subgraph "Level 4: Simulation Runtime"
        Sim[nebudcs-sim]
    end

    subgraph "Level 5: Process Physics"
        Physics[nebudcs-physics]
    end

    subgraph "Level 6: Control & Alarms"
        Control[nebudcs-control]
        Alarm[nebudcs-alarm]
    end

    subgraph "Level 7: Persistence, Transport & Extension"
        Historian[nebudcs-historian]
        Protocol[nebudcs-protocol]
        Scenario[nebudcs-scenario]
        Scripting[nebudcs-scripting]
    end

    subgraph "Level 8: Applications & Hosts"
        Server[apps/nebudcs-server]
        Desktop[apps/nebudcs-desktop]
        CLI[apps/nebudcs-cli]
    end

    Types --> Units
    Types --> Model
    Units --> Model

    Types --> Topology
    Units --> Topology
    Model --> Topology

    Types --> Sim
    Units --> Sim
    Model --> Sim
    Topology --> Sim

    Types --> Physics
    Units --> Physics
    Model --> Physics
    Topology --> Physics
    Sim --> Physics

    Types --> Control
    Units --> Control
    Model --> Control
    Topology --> Control
    Sim --> Control
    Physics --> Control

    Types --> Alarm
    Units --> Alarm
    Model --> Alarm
    Control --> Alarm

    Types --> Historian
    Units --> Historian
    Sim --> Historian
    Alarm --> Historian

    Types --> Protocol
    Units --> Protocol
    Model --> Protocol
    Sim --> Protocol
    Control --> Protocol
    Alarm --> Protocol

    Types --> Scenario
    Model --> Scenario
    Sim --> Scenario
    Physics --> Scenario
    Control --> Scenario
    Alarm --> Scenario

    Types --> Scripting
    Units --> Scripting
    Model --> Scripting

    Sim --> Server
    Protocol --> Server
    Historian --> Server

    Sim --> Desktop
    Protocol --> Desktop
    Model --> Desktop

    Sim --> CLI
    Historian --> CLI
```

### Crate Responsibilities & Invariants
- **`nebudcs-types`**: Core primitive newtypes (`EntityId`, `TagId`, `SimulationTime`, `SchemaVersion`, `NebuError`). **Zero workspace dependencies.**
- **`nebudcs-units`**: Strongly typed physical quantities (`Pressure`, `Temperature`, `MassFlow`, etc.), canonical SI internal representation, and display unit conversions. Depends only on `nebudcs-types`.
- **`nebudcs-model`**: Pure data schemas and DTOs (`EquipmentDefinition`, `EquipmentInstance`, `StreamDefinition`, `Project`). Immutable at runtime. Depends on `types` and `units`.
- **`nebudcs-topology`**: Graph-based process/utility/signal topology engine, port connection validation, and Tarjan's SCC cycle analysis.
- **`nebudcs-sim`**: Deterministic simulation runtime, fixed-step clock, double-buffered state isolation, and execution planning.
- **`nebudcs-physics`**: Generic conservation laws (mass, species, enthalpy), dynamic inventory accumulator, fluid hydraulics, property packages, and nonlinear recycle solver.
- **`nebudcs-control`**: DCS/PLC automation primitives, PID controllers, bumpless transfer, anti-windup, and actuator dynamics.
- **`nebudcs-alarm`**: Process limit alarms, shelving, deadbands, and alarm lifecycle state machine.
- **`nebudcs-historian`**: Asynchronous SQLite persistence, sample downsampling, and replay log storage.
- **`nebudcs-protocol`**: Serialization DTOs, WebSocket delta-update streaming, and REST API.
- **`nebudcs-scenario`**: Data-driven scenario runner, timeline triggers, and dynamic fault injection.
- **`nebudcs-scripting`**: Sandboxed Rhai scripting runtime for Level 3 custom equipment behavior.
- **Applications (`apps/*`)**: Headless server (`nebudcs-server`), desktop engineering app (`nebudcs-desktop`), and command-line harness (`nebudcs-cli`).

## Alternatives Considered

- **Single Monolithic Crate**: Rejected due to compile times and the high risk of architectural coupling between UI rendering, network protocols, and core physics.
- **Microservices Architecture**: Rejected due to IPC/network serialization latency breaking the sub-10ms simulation tick budget.

## Rationale

This layered workspace architecture guarantees strict separation of concerns, enables clean parallel testing, drastically reduces incremental compilation times, and prevents illegal dependency cycles at compile time.

## Consequences

- Domain logic and physics remain 100% headless and testable without UI or network servers.
- Adding dependencies requires conscious architectural review.
