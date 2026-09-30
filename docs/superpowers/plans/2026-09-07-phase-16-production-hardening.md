# Phase 16: Production Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform NebuDCS into an enterprise-grade, production-ready platform across performance, security, reliability, observability, backup/restore, migration, and deployment while strictly preserving all Phase 0–15 architecture guarantees and invariants.

**Architecture:** Harden existing server, domain, and storage subsystems using a constant-time cryptographic auth layer with server-side RBAC, atomic disk-backed persistence with crash recovery, streaming backup/restore with SHA-256 checksums, semantic schema migration engine, rate limiting, structured audit observability, standard health/readiness/liveness probes, and Linux-native systemd deployment assets.

**Tech Stack:** Rust (Axum, Tokio, Rusqlite, Tracing, Tower, Ring/SHA256, Serde), TypeScript / Web Components, Linux systemd.

## Global Constraints

- Rust simulation runtime remains the sole authoritative process authority; no second simulation engine.
- Zero hardcoded refinery unit assumptions (`K-101`, `V-101`, `LIC-101`, `CCR`, `NHT`, `CDU` must never be hardcoded in production logic).
- Frontend remains strictly presentation, orchestration, and operator interaction.
- Phase 12 scientific capabilities, Phase 13 engineering studio, Phase 14 dynamic HMI, and Phase 15 training studio must not regress.
- Deterministic simulation repeatability and state isolation must be preserved.
- All quality gates must pass: `cargo test --workspace`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --all -- --check`, `npm test`, `npm run build`.

---

## File Structure & Module Decomposition

### New Core Modules & Files
1. `apps/nebudcs-server/src/security/mod.rs` & `roles.rs`:
   - Constant-time token verification (preventing timing side-channel attacks).
   - Server-side Role-Based Access Control (`Role::Admin`, `Role::Engineer`, `Role::Instructor`, `Role::Operator`, `Role::Trainee`).
   - Rate limiting and input sanitization (path traversal defense).
   - Security audit logging with structured security events.
2. `apps/nebudcs-server/src/persistence/mod.rs` & `disk_store.rs`:
   - Persistent, atomic storage for projects, scenarios, and training session records.
   - Atomic write-temp-then-rename to prevent partial write corruption.
   - Automatic restart recovery on server boot.
3. `apps/nebudcs-server/src/backup/mod.rs` & `migration.rs`:
   - Backup bundle generator and restore engine with SHA-256 manifest integrity verification.
   - Schema version migration engine with forward compatibility checks.
4. `apps/nebudcs-server/src/observability/mod.rs` & `metrics.rs`:
   - Structured metrics collectors (tick rate, queue depth, memory, active sessions).
   - Cloud/Kubernetes/systemd standard Liveness (`/api/v1/health/live`), Readiness (`/api/v1/health/ready`), and Metrics (`/api/v1/metrics`) endpoints.
5. `deploy/`:
   - `nebudcs.service`: Systemd service definition with security sandboxing (`ProtectSystem=strict`, `NoNewPrivileges=true`).
   - `nebudcs.env`: Production environment configuration template.
   - `production-operations.md`: Runbook for installation, operations, Tailscale remote mesh, backups, and rollback.
6. `apps/nebudcs-server/tests/production_hardening_tests.rs`:
   - Comprehensive adversarial and integration tests covering security, persistence, restart recovery, backup/restore, migration, and performance limits.

---

### Task 1: Security Hardening — Constant-Time Auth, Server-Side RBAC, Path Traversal Defense & Rate Limiting

**Files:**
- Create: `apps/nebudcs-server/src/security/mod.rs`
- Create: `apps/nebudcs-server/src/security/roles.rs`
- Modify: `apps/nebudcs-server/src/auth.rs`
- Modify: `apps/nebudcs-server/src/config.rs`
- Modify: `apps/nebudcs-server/src/lib.rs`

- [ ] **Step 1: Implement constant-time string comparison and role hierarchy**
  - Implement `subtle::constant_time_eq` (or constant-time XOR comparison in pure Rust) for bearer token validation to eliminate timing attack vectors.
  - Implement `UserRole` enum (`Admin`, `Engineer`, `Instructor`, `Operator`, `Trainee`) with explicit permission checks (`can_modify_topology()`, `can_inject_faults()`, `can_operate_dcs()`, `can_administer_sessions()`).
  - Implement token-to-role resolution supporting multi-role configuration via env or config (e.g. `ADMIN_TOKEN`, `INSTRUCTOR_TOKEN`, `OPERATOR_TOKEN`, `TRAINEE_TOKEN`, or legacy single `AUTH_TOKEN` mapping to Admin).
- [ ] **Step 2: Implement Path Traversal & Input Sanitization**
  - Create `sanitize_entity_id(id: &str) -> Result<String, ServerError>` rejecting `..`, `/`, `\`, null bytes, and non-whitelisted characters.
- [ ] **Step 3: Implement In-Memory Token-Bucket Rate Limiter**
  - Implement lightweight, non-blocking rate limiter per IP/client for high-frequency endpoints (`/api/v1/control/`, `/api/v1/tags/write`, `/api/v1/training/sessions/*/action`).
- [ ] **Step 4: Wire security middleware into server router**
  - Connect security validation, role verification, and CORS configuration in `apps/nebudcs-server/src/lib.rs`.

---

### Task 2: Reliability & Persistence Hardening — Atomic Disk Storage & Crash Recovery

**Files:**
- Create: `apps/nebudcs-server/src/persistence/mod.rs`
- Create: `apps/nebudcs-server/src/persistence/disk_store.rs`
- Modify: `apps/nebudcs-server/src/project_manager.rs`
- Modify: `apps/nebudcs-server/src/scenario_manager.rs`
- Modify: `apps/nebudcs-server/src/training_manager.rs`
- Modify: `apps/nebudcs-server/src/host.rs`

- [ ] **Step 1: Implement `DiskStore` with atomic file writes**
  - Writes data to `.tmp` file in the configured storage directory (e.g. `data/projects`, `data/scenarios`, `data/training`), syncs to disk, and atomically renames to final `.json` target.
- [ ] **Step 2: Integrate `DiskStore` into `ProjectManager`, `ScenarioManager`, and `TrainingManager`**
  - On save / update / delete: atomically update disk storage.
  - On server boot: automatically load existing saved projects, scenarios, and sessions from disk before falling back to default seed projects.
- [ ] **Step 3: Implement graceful server shutdown drain**
  - In `apps/nebudcs-server/src/main.rs`, handle SIGINT/SIGTERM by signaling the host to stop, waiting for the simulation thread to exit cleanly, flushing historian writes, and closing connections.

---

### Task 3: Backup, Restore & Schema Migration Engine

**Files:**
- Create: `apps/nebudcs-server/src/backup/mod.rs`
- Create: `apps/nebudcs-server/src/backup/migration.rs`
- Create: `apps/nebudcs-server/src/routes/admin.rs`
- Modify: `apps/nebudcs-server/src/routes/mod.rs`

- [ ] **Step 1: Implement Backup Package Generation**
  - Gathers all projects, scenarios, completed training sessions, and historian database runs into a self-contained JSON/TAR bundle.
  - Computes SHA-256 checksum manifest of all bundled artifacts.
- [ ] **Step 2: Implement Secure Restore Engine**
  - Validates bundle schema version and verifies SHA-256 manifest.
  - Restores projects and scenarios atomically, rejecting corrupted or tampered payloads.
- [ ] **Step 3: Implement Schema Migration Engine**
  - Detects outdated project or scenario schema versions (e.g. `0.1.0` or `1.0.0` missing newly added metadata or unit parameters) and applies forward-compatible migrations to `CURRENT` version.
- [ ] **Step 4: Expose Admin REST Endpoints**
  - `POST /api/v1/admin/backup`
  - `POST /api/v1/admin/restore`
  - `POST /api/v1/admin/migrate`
  - Protected strictly by `UserRole::Admin`.

---

### Task 4: Observability, Metrics & Production Health Probes

**Files:**
- Create: `apps/nebudcs-server/src/observability/mod.rs`
- Create: `apps/nebudcs-server/src/observability/metrics.rs`
- Modify: `apps/nebudcs-server/src/routes/health.rs`
- Modify: `apps/nebudcs-server/src/routes/mod.rs`

- [ ] **Step 1: Implement Server Metrics Collector**
  - Collects uptime, process memory (via `/proc/self/statm` on Linux or fallback), total simulation ticks, average tick execution time, active WebSocket clients, active training sessions, and historian sample backlog.
- [ ] **Step 2: Implement Standard Health Endpoints**
  - `GET /api/v1/health`: Detailed operational health report (existing, enhanced with metrics).
  - `GET /api/v1/health/live`: Lightweight liveness probe returning HTTP 200 `{"status": "alive"}`.
  - `GET /api/v1/health/ready`: Readiness probe returning HTTP 200 `{"status": "ready"}` if simulation engine and storage are healthy, HTTP 503 if starting or degraded.
  - `GET /api/v1/metrics`: Structured Prometheus/JSON metrics endpoint.
- [ ] **Step 3: Implement Security & Lifecycle Audit Log Traces**
  - Emit structured `tracing::warn!` and `tracing::info!` events for auth failures, rate limit breaches, administrative commands, and project mutations.

---

### Task 5: Packaging, Systemd Service & Production Deployment Assets

**Files:**
- Create: `deploy/nebudcs.service`
- Create: `deploy/nebudcs.env`
- Create: `deploy/backup.sh`
- Create: `docs/deployment/production-operations.md`

- [ ] **Step 1: Create systemd service unit (`deploy/nebudcs.service`)**
  - Define production unit with `Type=simple`, `Restart=always`, `RestartSec=5`, `LimitNOFILE=65536`, and Linux security isolation (`ProtectSystem=strict`, `ProtectHome=true`, `PrivateTmp=true`, `NoNewPrivileges=true`).
- [ ] **Step 2: Create environment configuration template (`deploy/nebudcs.env`)**
  - Environment variable declarations for `PORT`, `BIND_ADDR`, `AUTH_TOKEN`, `DATA_DIR`, `LOG_LEVEL`, `RATE_LIMIT_RPM`, `CORS_ORIGINS`.
- [ ] **Step 3: Create automated backup script (`deploy/backup.sh`)**
  - Shell script to trigger `/api/v1/admin/backup`, timestamp the archive, and retain daily/weekly rotations.
- [ ] **Step 4: Create Production Operations Runbook (`docs/deployment/production-operations.md`)**
  - Step-by-step instructions for Linux deployment, systemd installation, Tailscale mesh network configuration for remote multi-station operations, disaster recovery, and zero-downtime rolling restart.

---

### Task 6: Production Hardening Test Suite & Adversarial Verification

**Files:**
- Create: `apps/nebudcs-server/tests/production_hardening_tests.rs`

- [ ] **Step 1: Write security & adversarial tests**
  - Test constant-time auth verification against valid and forged tokens.
  - Test role boundary enforcement: verify Trainee token is rejected from Admin and Instructor endpoints.
  - Test path traversal injection (`../../etc/passwd` in project/scenario IDs) is strictly rejected.
  - Test rate limiting triggers HTTP 429 when burst limit is exceeded.
- [ ] **Step 2: Write persistence & crash recovery tests**
  - Test project saved to disk survives server instance termination and reloads on new server startup.
- [ ] **Step 3: Write backup, restore & migration tests**
  - Create project and scenario $\to$ trigger backup $\to$ wipe storage $\to$ trigger restore $\to$ verify all entities match bit-for-bit and can be loaded into simulation runtime.
  - Test migration of legacy project schema.
- [ ] **Step 4: Write health, liveness, and readiness tests**
  - Test `/api/v1/health/live`, `/api/v1/health/ready`, and `/api/v1/metrics`.
- [ ] **Step 5: Run full regression and quality gates**
  - Execute `cargo test --workspace`.
  - Execute `cargo clippy --all-targets --all-features -- -D warnings`.
  - Execute `cargo fmt --all -- --check`.
  - Execute `npm test` and `npm run build` in `web/`.

---

### Task 7: Final Independent Verification & Reports

**Files:**
- Create: `docs/phase_16_implementation_report.md`
- Create: `docs/phase_16_final_verification_report.md`

- [ ] **Step 1: Execute complete independent verification audit**
  - Verify all 14 checklist items from user prompt (Security, Persistence, Backup/Restore, Migration, Recovery, Performance, Observability, Deployment, Upgrades, Phase 0–15 regression, Determinism, Custom projects, Training, Replay).
- [ ] **Step 2: Execute hardcode scan**
  - Prove zero hardcoded refinery assumptions or unit IDs in production code.
- [ ] **Step 3: Produce `phase_16_implementation_report.md`**
- [ ] **Step 4: Produce `phase_16_final_verification_report.md` with explicit verdict (`PASS`)**
- [ ] **Step 5: Run `graphify update .`**
