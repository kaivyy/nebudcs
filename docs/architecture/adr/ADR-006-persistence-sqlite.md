---
title: ADR-006: Persistence with SQLite
status: accepted
date: 2026-09-05
author: NebuDCS Architecture Team
---

# ADR-006: Persistence with SQLite

## Status

Accepted

## Context

NebuDCS requires a format for saving and loading project files. Projects contain plant topology, equipment instances, tags, HMI layouts, scenarios, and historian data. The format must be portable (easy to share as a single file) and support schema versioning. PostgreSQL is targeted as an optional future server backend.

## Problem

What technology should be used for local project storage and the embedded historian?

## Decision

We will use **SQLite** as the primary storage format for NebuDCS projects. We will use **rusqlite** for synchronous, deterministic database operations where necessary, and **sqlx** for async historian reads/writes outside the hot simulation loop.

*Technology Version:* SQLite 3.45+, rusqlite 0.31+, sqlx 0.8+ (as of 2026).
*Compile Cost:* Moderate (compiling C SQLite via `libsqlite3-sys` or using bundled).
*Binary Size Impact:* Adds a few megabytes.
*Runtime/Memory Overhead:* Low; highly optimized.

## Alternatives Considered

- **Custom Binary Format (e.g., Bincode + Zip)**: Fast, but hard to inspect, query, or migrate. A database is much better for structured relations like tags and topology.
- **JSON / YAML**: Good for configuration, but terrible for historian data (too large, slow to parse, requires loading entire file into memory).
- **PostgreSQL**: Excellent for a multi-user server deployment, but unacceptable as a requirement for a standalone desktop application.

## Rationale

SQLite provides a single-file, highly portable database that can handle both relational configuration data (topology) and time-series data (historian) efficiently. It supports ACID transactions, ensuring project files don't corrupt on crash. Using standard SQL allows us to easily write migration scripts (e.g., using `dbmate` or `sqlx` migrations) for schema upgrades.

## Trade-offs

- SQLite can become a bottleneck for extremely high-frequency historian writes if not batched properly (WAL mode and batch inserts are required).
- Slightly more overhead than a pure binary format for simple loads.

## Consequences

- Project files are just `.sqlite` or `.nebu` files that can be inspected with standard tools (DBeaver, SQLite browser).
- Historian data is stored directly in the project file, making scenarios completely self-contained.
- We must maintain schema migration scripts for every release.

## Rejected Alternatives

- JSON was rejected because it cannot handle historical data.
- Custom binary was rejected due to lack of standard querying and migration tooling.

## Future Reconsideration Criteria

If the local historian write volume exceeds SQLite's capabilities even with WAL and batching, we may split the project into two files: SQLite for configuration, and a specialized time-series format (like Parquet or a custom ring buffer) for the historian.
