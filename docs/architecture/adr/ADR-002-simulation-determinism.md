---
title: ADR-002: Deterministic Simulation Engine
status: accepted
date: 2026-09-05
author: NebuDCS Architecture Team
---

# ADR-002: Deterministic Simulation Engine

## Status

Accepted

## Context

NebuDCS functions as a generic process simulation and training environment. Features such as scenario replay, automated testing, and synchronization between multiple clients require the simulation to produce exactly the same results given the same initial state and inputs.

## Problem

How should the simulation execution model be designed to guarantee determinism?

## Decision

The core simulation loop will be **strictly deterministic** and run on a single thread (or a tightly controlled deterministic worker pool).
- System time (`std::time::SystemTime`, `Instant::now()`) cannot be used to drive simulation logic; a virtual simulation clock must be used.
- Thread scheduling, asynchronous task yielding, and network latency must not affect the simulation math.
- Floating-point math must be handled carefully (using deterministic libraries or strict IEEE-754 adherence) to ensure cross-platform consistency where possible.

## Alternatives Considered

- **Wall-clock-driven simulation**: Using system time to calculate deltas. Easy to implement, but impossible to reliably replay or test.
- **Async-driven simulation**: Using `async`/`await` for equipment updates. Incurs non-deterministic task scheduling overhead.
- **Multi-threaded lock-step**: Complex to implement safely without introducing race conditions that break determinism.

## Rationale

Determinism is the foundation of scenario replay, reliable automated testing (especially CI), and state synchronization. If the simulation diverges due to thread scheduling or clock jitter, users cannot trust training scenarios, and developers cannot reproduce bugs.

## Trade-offs

- Performance is bounded by single-core speed (unless using deterministic parallelism, which is complex).
- All external inputs (operator actions, script events) must be explicitly queued and synchronized to the simulation tick.

## Consequences

- We can implement a robust "historian replay" feature that only records inputs and re-simulates the plant.
- Automated tests will never be flaky due to timing issues.
- `async` Rust cannot be used in the hot loop.

## Rejected Alternatives

- Wall-clock and async-driven approaches were rejected because they fundamentally break reproducibility, which is a core requirement for a DCS training simulator.

## Future Reconsideration Criteria

If a single thread cannot meet the 10ms tick budget for extremely large plant topologies, we will investigate deterministic multi-threading (e.g., using dependency graphs to safely parallelize non-interacting equipment), but we will not abandon the determinism requirement.
