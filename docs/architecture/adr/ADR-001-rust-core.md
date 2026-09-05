---
title: ADR-001: Rust as the Core Language
status: accepted
date: 2026-09-05
author: NebuDCS Architecture Team
---

# ADR-001: Rust as the Core Language

## Status

Accepted

## Context

NebuDCS requires a high-performance, safe, and portable language to serve as the simulation core. The engine must support desktop, headless server, and web (via WebAssembly) deployment. It requires deterministic execution, tight memory control, and must avoid the pitfalls of garbage collection pauses in a real-time simulation loop.

## Problem

Which programming language should serve as the foundation for the NebuDCS core simulation engine?

## Decision

We will use **Stable Rust** for the core simulation engine.
- We will organize the project as a Cargo workspace.
- `unsafe` code is strictly prohibited unless extensively justified and isolated (e.g., for direct hardware or C-FFI interactions not present in the core).
- Asynchrony (`async`/`await`) is permitted for IO, networking, and UI, but is **strictly prohibited** inside the deterministic simulation loop.

*Technology Version:* Rust Stable 1.102+ (as of late 2026).
*Compile Cost:* High, but mitigated by modular crate architecture and incremental compilation.
*Binary Size Impact:* Low to medium.
*Runtime/Memory Overhead:* Minimal.

## Alternatives Considered

- **C++**: High performance, industry standard for simulations, but suffers from memory safety issues, complex build systems (CMake), and difficult dependency management.
- **Go**: Fast compilation, good concurrency, but garbage collection introduces unpredictable pauses that break real-time deterministic simulation constraints.
- **Python**: Excellent ecosystem for data science/thermodynamics, but too slow for a real-time core loop; GIL makes threading difficult; hard to distribute as a standalone desktop binary.
- **Julia**: Great for mathematical modeling, but high JIT warmup time, difficult standalone distribution, and heavy runtime overhead.

## Rationale

Rust provides zero-cost abstractions, deterministic memory management without a garbage collector, and strong thread safety guarantees. Its build system (Cargo) and module system natively support the large-scale workspace architecture required for NebuDCS.

## Trade-offs

- Slower compilation times compared to Go.
- Steeper learning curve for domain engineers who might be more familiar with Python or C++.
- Ecosystem for specialized thermodynamics is smaller than Python's, requiring us to build or port models.

## Consequences

- The simulation loop will remain predictable with no GC pauses.
- Memory leaks and data races are largely eliminated by the compiler.
- We must invest upfront in defining clear crate boundaries to manage compile times.

## Rejected Alternatives

- C++ was rejected due to memory safety risks and tooling complexity.
- Go was rejected due to GC pauses.
- Python/Julia were rejected due to runtime performance and distribution challenges.

## Future Reconsideration Criteria

If Rust compilation times become an insurmountable bottleneck for the engineering team, or if a critical thermodynamics library cannot be ported and requires a different host language, we would reconsider the architecture (potentially wrapping C++ or Python via FFI, though highly undesirable).
