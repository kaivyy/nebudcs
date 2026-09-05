---
title: Implementation Blueprint
version: 1.0.0
status: draft
date: 2026-09-05
---

# NebuDCS Implementation Blueprint

## What this is and how to use it
This implementation blueprint serves as the definitive execution guide for the NebuDCS platform. It translates the architectural decisions and PRD requirements into an ordered, dependency-aware, and actionable plan for coding agents and human developers.

## Relation to Architecture Docs and PRD
While the PRD defines *what* to build and the architecture docs define *how* it should be structured, this blueprint defines *when* and *in what order* to build it. It provides the concrete step-by-step path to realizing the architecture without violating invariants.

## How a coding agent should use these documents
Coding agents must read this `README.md`, the `dependency-map.md`, and the `coding-agent-rules.md` before starting any task. They must strictly follow the dependency order and phase requirements, reading the specific phase plan (e.g., `phase-01-foundation.md`) for their current task.

## Navigation Table
| Document | Purpose |
|----------|---------|
| [Architecture Traceability](./architecture-traceability.md) | Maps PRD requirements to architecture and phases. |
| [Dependency Map](./dependency-map.md) | Concrete dependencies, allowed imports, and crate boundaries. |
| [Roadmap](./roadmap.md) | The 15-phase implementation plan. |
| [Phase 1: Foundation](./phase-01-foundation.md) | Detailed specification for the core workspace. |
| [Vertical Slice 1](./vertical-slice-01.md) | Specification for the first end-to-end working system. |
| [Testing Strategy](./testing-strategy.md) | Required tests and quality gates. |
| [Coding Agent Rules](./coding-agent-rules.md) | Invariants and rules that must never be broken. |

## Implementation Philosophy
1. **Dependency-First:** Lower layers must be complete before higher layers depend on them.
2. **Test-First:** Write tests before or alongside implementation.
3. **Vertical Slices:** Prove the architecture works end-to-end as early as possible.

**WARNING:** Do not implement features in the order they appear in the architecture documents. Follow the dependency order defined in this blueprint.
