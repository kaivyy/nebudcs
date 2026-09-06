---
title: Implementation Blueprint & Execution Guide
version: 2.0.0
status: verified-baseline
date: 2026-09-06
---

# NebuDCS Implementation Blueprint & Execution Guide

## Executive Overview

This implementation blueprint serves as the authoritative, dependency-aware execution guide for the NebuDCS platform. It bridges the system architecture and PRD requirements into an actionable, strictly ordered implementation sequence.

### Current Implementation Baseline

- **Phase 0 (Foundation)**: COMPLETE
- **Phase 1 (Domain Model & Physical Types)**: COMPLETE
- **Phase 2 (Topology & Ports)**: COMPLETE
- **Phase 3 (Simulation Runtime & Double-Buffering)**: COMPLETE
- **Phase 4 (Process Physics & Conservation Foundation)**: **`PHASE 4 VERIFIED`**
- **Phase 5 (Control Engine & Automation)**: **`PHASE 5 VERIFIED`**
- **Phase 6 (Historian & Timeseries Persistence)**: **`PHASE 6 VERIFIED`**
- **Phase 7 (Realtime Protocol & API Server)**: **`PHASE 7 VERIFIED`**
- **Phase 8 (End-to-End Vertical Slice 1)**: **`PHASE 8 VERIFIED`**
- **Phase 9 (Scenario & Fault Injection Engine)**: **`PLANNED / NEXT`**

---

## Core Product Principles & Invariants

All development must preserve these non-negotiable architectural invariants:

1. **Simulation Engine as the Source of Truth**:
   The authoritative model flow is strictly unidirectional:
   $$\text{Project Model} \to \text{Topology} \to \text{Simulation} \to \text{Physical State} \to \text{Tags/Signals} \to \text{Protocol} \to \text{UI}$$
   Visual graphics, P&ID layouts, faceplates, and trends are purely views observing authoritative simulation state. Frontend code MUST NOT contain duplicate physics or simulation truth.
2. **Generic & Fully Configurable**:
   NebuDCS must **NOT** be hardcoded around any specific refinery unit (e.g., CDU, NHT, CCR, RFCC, Platforming, TDHT, AH Unibon) or vendor DCS/PLC. All refinery units and process configurations are reusable data packages constructed from generic primitives without recompiling the Rust simulation core.
3. **Deterministic, Fixed-Timestep Execution**:
   The core simulation loop runs single-threaded at a fixed timestep $\Delta t$. It uses strictly monotonic `SimulationTime`, zero system clock lookups, double-buffered state isolation, and zero heap allocations inside the tick hot loop.
4. **Three Strictly Decoupled Product Environments**:
   - **Engineering Studio**: Authoritative model authoring, equipment library, topology routing, parameter inspection, and design validation.
   - **Operator / DCS Station**: Runtime process monitoring, interactive graphics, alarm management, faceplate operation, and safe diagnostics (topology editing disabled).
   - **Training / Scenario Studio**: Data-driven scenario execution, instructor controls, dynamic physical fault injection, rubric-based evaluation, and deterministic session replay.
5. **Headless-First Architecture**:
   The simulation core and host server run entirely headless on Linux (e.g., Ubuntu Server). UI clients (Desktop via `egui/wgpu` or Web via TypeScript/Web Components) connect over local memory or lightweight WebSocket/REST protocols. Secure remote multi-station operation is achieved via network overlays (e.g., Tailscale) without heavy cloud infrastructure.

---

## Navigation & Implementation Document Index

| Document | Purpose |
|---|---|
| [Roadmap](./roadmap.md) | Comprehensive 17-phase lifecycle (Phases 0 through 16) with dependencies, scopes, and acceptance criteria. |
| [Architecture Traceability](./architecture-traceability.md) | Complete PRD ↔ architecture ↔ implementation phase traceability matrix and audit findings. |
| [Dependency Map](./dependency-map.md) | Concrete crate hierarchy, allowed/forbidden import directions, ownership boundaries, and cycle prevention. |
| [Phase 1: Foundation](./phase-01-foundation.md) | Foundation crates specification (`nebudcs-types`, `nebudcs-units`, `nebudcs-model`). |
| [Phase 4: Process Physics](./phase-04-process-physics.md) | Verified Phase 4 physics architecture, conservation equations, hydraulics, VLE, recycle solver, safeguards, and engineering limitations. |
| [Phase 5: Control Engine](./phase-05-control-engine.md) | Verified implementation specification for Phase 5 (PID, bumpless transfer, anti-windup, filters, execution order, signal quality, logic blocks, and explicit non-goals). |
| [Phase 6: Historian](./phase-06-historian.md) | Hardened implementation specification for Phase 6 (SQLite WAL, non-blocking sampling, bounded backpressure, event journal, downsampling, and verification matrix). |
| [Phase 7: Realtime Protocol](../architecture/protocol.md) | Verified implementation specification for Phase 7 (Axum host, WebSocket streaming, delta updates, REST API, session authentication). |
| [Vertical Slice 1](./vertical-slice-01.md) | Verified specification for the end-to-end full-stack proof (Equipment → Port → Instrument → Controller → Actuator → Physics → State → Tag → Protocol → Web/Desktop UI → P&ID/HMI → Faceplate → Alarm/Trend). |
| [Testing Strategy](./testing-strategy.md) | Multi-layered testing strategy (software correctness, numerical correctness, physical model verification, determinism, performance benchmarks). |
| [Coding Agent Rules](./coding-agent-rules.md) | Absolute rules, forbidden patterns, commit formats, and quality gates for coding agents. |

---

## How a Coding Agent Must Use These Documents

Before writing any code for a phase:
1. Verify the current phase status in [Roadmap](./roadmap.md) and confirm all prerequisite phases are verified.
2. Read [Coding Agent Rules](./coding-agent-rules.md) to ensure all workspace invariants are understood.
3. Check [Dependency Map](./dependency-map.md) to confirm exact crate boundaries and strictly forbidden imports.
4. Review [Testing Strategy](./testing-strategy.md) to adhere to Test-Driven Development (TDD) requirements: write failing tests first, ensure they fail for the right reason, write minimal clean code, verify all tests pass, and verify `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check`.
5. Check git status to confirm only relevant files were modified.

> [!IMPORTANT]
> **Implementation Order Rule**: Never implement features in architecture-document order or PRD section order. Always execute strictly according to the dependency-ordered phase plan defined in [Roadmap](./roadmap.md).
