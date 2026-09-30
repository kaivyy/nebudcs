# NebuDCS Production Deployment & End-to-End Smoke Test Report

- **Date:** 2026-09-07
- **Platform:** Linux (x86_64)
- **Service Unit:** `nebudcs.service` (systemd)
- **Deployment Mode:** Native Linux Daemon with Systemd Security Sandboxing & Axum Single-Port Static Serving
- **Verdict:** **PASS**

---

## 1. Executive Summary

This report documents the official production deployment and end-to-end smoke verification of NebuDCS (Phases 0–16). The verification proved that the authoritative Rust simulation runtime (`nebudcs-server`) and the web frontend (`web/dist`) can be installed, started, accessed, and operated together under real Linux systemd execution with full security hardening.

All twelve operational criteria defined in the deployment specifications passed with 100% success:
1. Production release compilation (`nebudcs-server`, 14 MB ELF 64-bit executable).
2. Production frontend compilation (`web/dist`, 159 kB JS, 22 kB CSS).
3. Linux system user (`nebudcs`) and state directories (`/var/lib/nebudcs/data`, `/var/lib/nebudcs/dist`, `/var/backups/nebudcs`, `/etc/nebudcs`).
4. Systemd unit installation and zero-warning verification (`systemd-analyze verify`).
5. Live health and telemetry probes (`/api/v1/health/live`, `/api/v1/health/ready`, `/api/v1/metrics`).
6. Axum single-port static asset serving (`GET /`, `/assets/index-sKNF8HLV.js`, `/assets/index-Dj99W0jr.css`).
7. Multi-Role RBAC enforcement (`Admin`, `Instructor`, `Operator`, `Trainee`), path traversal rejection (`../../etc/passwd`), and token-bucket rate limiting (250-request burst test).
8. Arbitrary non-refinery plant flowsheet (`smoke-test-plant`: `PUMP-A1`, `TANK-B7`, `VALVE-C3`, `PT-900`, `FT-901`, `LT-902`, `LIC-902`) through project creation, topological validation, simulation loading, continuous ticking, faceplate setpoint adjustment, alarm generation, and alarm acknowledgement.
9. Training studio workflow: instructor scenario authoring (`scn-smoke-recovery`), session loading, simulation execution, trainee intervention recording, forbidden fault rejection, completion, deterministic evaluation (100%), and replay timeline scrubbing.
10. Persistence and crash recovery: service restart (`systemctl restart nebudcs.service`) with 100% data fidelity preservation across projects, scenarios, and sessions.
11. Cryptographic backup and restore (`/usr/local/bin/nebudcs-backup.sh`, SHA-256 manifest verification, REST restore execution).
12. WebSocket realtime streaming (`ws://127.0.0.1:8088/ws`) and full frontend test suite (79/79 passing across 10 test files).

---

## 2. Host & Environment Specification

| Component | Detail |
|---|---|
| **Operating System** | Linux 6.6+ x86_64 |
| **Init System** | systemd (PID 1) |
| **Service User** | `nebudcs` (`system`, nologin) |
| **Binary Path** | `/usr/local/bin/nebudcs-server` (0755) |
| **Config Path** | `/etc/nebudcs/nebudcs.env` (0640, `root:nebudcs`) |
| **Data Directory** | `/var/lib/nebudcs/data` (0750, `nebudcs:nebudcs`) |
| **Static Assets** | `/var/lib/nebudcs/dist` (0750, `nebudcs:nebudcs`) |
| **Backup Storage** | `/var/backups/nebudcs` (0750, `nebudcs:nebudcs`) |
| **Backup Tool** | `/usr/local/bin/nebudcs-backup.sh` (0750) |
| **Production Port** | TCP 8088 (`0.0.0.0:8088`) |

---

## 3. Production Artifact Compilation

### 3.1 Backend Release Binary
```bash
cargo build --release -p nebudcs-server
```
- **Profile:** `release` (opt-level = 3, LTO enabled)
- **Compile Duration:** 4m 47s
- **Output Binary:** `target/release/nebudcs-server`
- **Binary Size:** 14 MB (14,642,888 bytes)
- **Binary Type:** ELF 64-bit LSB pie executable, x86-64, dynamically linked

### 3.2 Web Frontend Production Bundle
```bash
npm --prefix web run build
```
- **Builder:** Vite v8.2.2 + TypeScript 5.7+
- **Build Duration:** 152 ms
- **Output Directory:** `web/dist/`
  - `dist/index.html`: 0.41 kB (gzip: 0.29 kB)
  - `dist/assets/index-Dj99W0jr.css`: 22.64 kB (gzip: 5.20 kB)
  - `dist/assets/index-sKNF8HLV.js`: 159.51 kB (gzip: 35.25 kB)

---

## 4. Systemd Sandboxing & Installation

### 4.1 Unit File (`/etc/systemd/system/nebudcs.service`)
The production service was verified using `systemd-analyze verify`:
```bash
systemd-analyze verify /etc/systemd/system/nebudcs.service
# Exit code: 0 (No warnings or syntax errors)
```

### 4.2 Sandboxing & Hardening Directives Confirmed Active
Inspected via `systemctl show nebudcs.service`:
```ini
MainPID=4189198
LimitNOFILE=65536
ReadWritePaths=/var/lib/nebudcs
PrivateTmp=yes
ProtectHome=yes
ProtectSystem=strict
NoNewPrivileges=yes
ActiveState=active
SubState=running
```
- `ProtectSystem=strict`: The host root filesystem is mounted strictly read-only except `/var/lib/nebudcs`.
- `ProtectHome=yes`: User `/home` directories are completely inaccessible.
- `PrivateTmp=yes`: Process receives an isolated `/tmp` namespace.
- `NoNewPrivileges=yes`: Prevents privilege escalation via `setuid`/`setgid`.
- `LimitNOFILE=65536`: High file-descriptor headroom for concurrent WebSockets.

---

## 5. End-to-End Smoke Test Execution Results

The automated smoke test suite (`tests/smoke_test_runner.py`) was executed against the live systemd daemon (`http://127.0.0.1:8088`).

### 5.1 Health Probes & Static Asset Delivery
| Probe / Endpoint | Request | HTTP Status | Response Payload | Verdict |
|---|---|---|---|---|
| Liveness Probe | `GET /api/v1/health/live` | 200 OK | `{"status":"alive"}` | **PASS** |
| Readiness Probe | `GET /api/v1/health/ready` | 200 OK | `{"simulation":"running","status":"ready","tick_count":0}` | **PASS** |
| Metrics Telemetry | `GET /api/v1/metrics` | 200 OK | `{"process_memory_bytes":12673024,"tick_rate_hz":20.0,...}` | **PASS** |
| SPA Index Document | `GET /` | 200 OK | `<!DOCTYPE html><html lang="en">...<nebu-app></nebu-app>` | **PASS** |
| Compiled JS Module | `GET /assets/index-sKNF8HLV.js` | 200 OK | `Content-Type: text/javascript`, 159,513 bytes | **PASS** |
| Compiled Stylesheet | `GET /assets/index-Dj99W0jr.css` | 200 OK | `Content-Type: text/css`, 22,648 bytes | **PASS** |

### 5.2 Security & Multi-Role RBAC Enforcement
| Test Scenario | Caller Role | Endpoint / Payload | Expected | Actual Result | Verdict |
|---|---|---|---|---|---|
| Unauthenticated Admin Access | None | `POST /api/v1/admin/backup` | 401 Unauthorized | HTTP 401 `{"code":"UNAUTHORIZED","message":"Missing Authorization header"}` | **PASS** |
| Trainee Privilege Boundary | Trainee Token | `POST /api/v1/admin/backup` | 403 Forbidden | HTTP 403 `{"code":"UNAUTHORIZED","message":"Permission denied: Administrator privileges required for backup"}` | **PASS** |
| Operator Privilege Boundary | Operator Token | `POST /api/v1/admin/backup` | 403 Forbidden | HTTP 403 `{"code":"UNAUTHORIZED","message":"Permission denied: Administrator privileges required for backup"}` | **PASS** |
| Trainee Scenario Authoring | Trainee Token | `POST /api/v1/training/scenarios` | 403 Forbidden | HTTP 403 `{"code":"UNAUTHORIZED","message":"Permission denied: Instructor or Admin role required to create or modify scenarios"}` | **PASS** |
| Path Traversal in Project ID | Admin Token | `GET /api/v1/projects/..%2F..%2Fetc%2Fpasswd` | 403 Forbidden | HTTP 403 `{"code":"UNAUTHORIZED","message":"Permission denied: Path traversal attack detected in identifier: '../../etc/passwd'"}` | **PASS** |
| Path Traversal in Scenario ID | Admin Token | `GET /api/v1/training/scenarios/..%2F..%2Fetc%2Fpasswd` | 403 Forbidden | HTTP 403 `{"code":"UNAUTHORIZED","message":"Permission denied: Path traversal attack detected in identifier: '../../etc/passwd'"}` | **PASS** |
| Token Bucket Rate Limiting | Any Token | 250 requests in rapid burst | 429 Too Many Requests | Requests 1–229: HTTP 200; Requests 230–250: HTTP 429 | **PASS** |

### 5.3 Arbitrary Non-Refinery Plant E2E Smoke Test
- **Project Name:** `Smoke Test Plant - Generic Water Cooling & Buffer`
- **Topology:**
  - `PUMP-A1` (`standard.pump.centrifugal`, design flow: 3,000 kg/h, head: 80 m, power: 150 kW)
  - `VALVE-C3` (`standard.valve.control_globe`, Cv: 50.0, pos: 50.0%)
  - `TANK-B7` (`standard.vessel.vertical_separator`, volume: 25.0 m³, level: 45.0%)
  - Streams: `S-901` (`PUMP-A1.discharge` $\to$ `VALVE-C3.inlet`), `S-902` (`VALVE-C3.outlet` $\to$ `TANK-B7.feed`)
  - Instruments: `PT-900` (discharge pressure, 0–40 bar), `FT-901` (mass flow, 0–5,000 kg/h), `LT-902` (level, 0–100%)
  - Control Loop: `LIC-902` (PV: `TANK-B7.LEVEL`, SP: 50.0, OUT: `VALVE-C3.POS`, Kp: 1.5, Ti: 20s, Td: 0.5s)
  - Alarm: `LAH-902` (PV: `TANK-B7.LEVEL`, Hi threshold: 80.0%)
  - Interlock: `TRIP-PUMP-A1` (PV: `TANK-B7.LEVEL` < 5.0% trips `PUMP-A1`)

| Stage | Command / Endpoint | Request / Payload | Observed Result | Verdict |
|---|---|---|---|---|
| Project Creation | `POST /api/v1/projects` | `name: "Smoke Test Plant..."` | HTTP 200, Project ID `proj-01M1VZBF4RKJ5KGY0P5WHCY8Y8` | **PASS** |
| Entity Configuration | `PUT /api/v1/projects/{id}` | 3 equipment, 2 streams, 3 instruments, 1 loop, 1 alarm, 1 interlock | HTTP 200, Persisted to disk | **PASS** |
| Topological Validation | `POST /api/v1/projects/{id}/validate` | Validation request | HTTP 200, `valid: true`, `error_count: 0` | **PASS** |
| Simulation Initialization | `POST /api/v1/projects/{id}/load-sim` | Synchronize model to sim engine | HTTP 200, `status: "ok"`, 15 runtime tags registered | **PASS** |
| Clock Lifecycle Control | `POST /api/v1/sim/control` | `{"action": "play"}` | HTTP 200, Clock started, 20 Hz deterministic tick thread active | **PASS** |
| Telemetry Verification | `GET /api/v1/sim/snapshot` | Point-in-time snapshot query | HTTP 200, SimTime: 0.85s, Ticks: 17, Tags: 15, Active alarms: 0 | **PASS** |
| Tag Store Inspection | `GET /api/v1/tags` | List all live runtime tags | HTTP 200, Confirmed `TANK-B7.LEVEL`, `PUMP-A1.FLOW`, `VALVE-C3.POS` | **PASS** |
| Faceplate Tuning | `POST /api/v1/control/pid/setpoint` | `{"loop_name": "LIC-902", "sp": 65.0}` | HTTP 200, SP updated to 65.0 | **PASS** |
| Upset & Alarm Trigger | `POST /api/v1/tags/write` | `{"tag": "TANK-B7.LEVEL", "value": 85.0}` | HTTP 200, Tag written; `LAH-902` tripped | **PASS** |
| Alarm Verification | `GET /api/v1/alarms` | Query active alarm summary | HTTP 200, Active alarms: 1 (`LAH-902`, severity Warning) | **PASS** |
| Alarm Acknowledgment | `POST /api/v1/alarms/acknowledge` | `{"alarm_name": "LAH-902"}` | HTTP 200, Alarm transitioned to `Acknowledged` | **PASS** |

### 5.4 Training Studio E2E Smoke Test
- **Scenario:** `scn-smoke-recovery` ("Plant Cooling Upset & Recovery Drill", duration: 600s, random seed: 12345678)
- **Session:** `tsess-01m1vzbgfthh3dj7d05h2y30m5` (Instructor: `INST-LEAD-1`, Trainee: `TRAINEE-OP-9`)

| Step | Endpoint | Role | Action / Result | Verdict |
|---|---|---|---|---|
| Author Scenario | `POST /api/v1/training/scenarios` | Instructor | Scenario saved with initial conditions, timeline marker, objective | **PASS** |
| Create Session | `POST /api/v1/training/sessions` | Instructor | Session created, initial status `Configuring` | **PASS** |
| Load Session Project | `POST /api/v1/training/sessions/{id}/load` | Instructor | Plant model synchronized with scenario baseline | **PASS** |
| Start Session | `POST /api/v1/training/sessions/{id}/start` | Instructor | Clock starts; session status becomes `Running` | **PASS** |
| Operator Action 1 | `POST /api/v1/training/sessions/{id}/action` | Trainee | `SET_SP` on `LIC-902` to 52.0 $\to$ `status: "Accepted"` | **PASS** |
| Operator Action 2 | `POST /api/v1/training/sessions/{id}/action` | Trainee | `ACK_ALARM` on `LAH-902` $\to$ `status: "Accepted"` | **PASS** |
| Forbidden Action | `POST /api/v1/training/sessions/{id}/action` | Trainee | `INJECT_FAULT` on `PUMP-A1` $\to$ `status: "Rejected: Trainee lacks permission..."` | **PASS** |
| Complete & Grade | `POST /api/v1/training/sessions/{id}/complete` | Instructor | Session ended, evaluated: `percentage: 100.0`, `passed: true` | **PASS** |
| Fetch Evaluation | `GET /api/v1/training/sessions/{id}/evaluation` | Instructor | Deterministic report fetched: verified 100% score | **PASS** |
| Replay Timeline | `GET /api/v1/training/sessions/{id}/replay` | Instructor | Replay summary fetched: 3 event markers verified | **PASS** |
| Replay Frame | `GET /api/v1/training/sessions/{id}/replay/frame?sim_time=0.05` | Instructor | Frame reconstructed deterministically at sim_time 0.05s | **PASS** |

### 5.5 Persistence & Service Crash Recovery Test
1. Verified atomic disk files in `/var/lib/nebudcs/data`:
   - `/var/lib/nebudcs/data/projects/proj-01M1VZBF4RKJ5KGY0P5WHCY8Y8.json` (7,295 bytes)
   - `/var/lib/nebudcs/data/scenarios/scn-smoke-recovery.json` (1,020 bytes)
   - `/var/lib/nebudcs/data/training/tsess-01m1vzbgfthh3dj7d05h2y30m5.json` (6,337 bytes)
2. Terminated and restarted systemd service:
   ```bash
   systemctl restart nebudcs.service
   ```
3. Polled readiness probe: recovered and reported ready within 300 ms.
4. Re-queried entities via REST API:
   - Project: `proj-01M1VZBF4RKJ5KGY0P5WHCY8Y8` survived with all 3 equipment, 2 streams, and controllers intact.
   - Scenario: `scn-smoke-recovery` survived intact.
   - Session: `tsess-01m1vzbgfthh3dj7d05h2y30m5` survived intact.
- **Verdict:** **PASS**

### 5.6 Cryptographic Backup & Restore Test
1. Executed automated backup tool:
   ```bash
   /usr/local/bin/nebudcs-backup.sh
   ```
   Output:
   ```
   [Sun Sep 6 06:23:10 PM UTC 2026] Requesting backup bundle from http://127.0.0.1:8088...
   [Sun Sep 6 06:23:10 PM UTC 2026] Backup successful: /var/backups/nebudcs/nebudcs_backup_20260906T182310Z.json (51225 bytes)
   [Sun Sep 6 06:23:10 PM UTC 2026] Rotating backups older than 30 days...
   [Sun Sep 6 06:23:10 PM UTC 2026] Backup and rotation complete.
   ```
2. Validated backup bundle structure:
   - `bundle_version`: `"1.0.0"`
   - `server_version`: `"0.1.0"`
   - `manifest`: 14 entries (each with SHA-256 digest, filename, size, category)
   - `projects`: 7 projects
   - `scenarios`: 3 scenarios
   - `training`: 4 sessions
3. Executed restore against `/api/v1/admin/restore` with Admin token:
   - Server verified cryptographic SHA-256 hash for all 14 files before writing.
   - Result: `projects_restored: 7`, `scenarios_restored: 3`, `training_restored: 4`.
   - In-memory managers reloaded state from disk.
   - Re-queried project via `GET /api/v1/projects/{id}`: verified bit-for-bit restoration.
- **Verdict:** **PASS**

### 5.7 WebSocket Realtime Telemetry Verification
Executed live client handshake test:
```bash
node -e '...' # ws://127.0.0.1:8088/ws
```
- Handshake request: `protocol_version: "1.0.0"`, `client_id: "test-smoke-client"`
- Handshake ack received: `protocol_version: "1.0.0"`, `session_id: "01M1VZCE9FH52T3W2N658TV8KZ"`
- Subscription request dispatched: `subscribe_all: true`, `include_alarms: true`
- Initial snapshot received: coherent state snapshot delivered over WebSocket frame.
- Session ended cleanly.
- **Verdict:** **PASS**

### 5.8 Frontend Regression Suite
```bash
npm --prefix web run test
```
- **Test Files:** 10 passed (10 total)
- **Tests:** 79 passed (79 total)
- **Suites:** `training.test.ts`, `operator.test.ts`, `components.test.ts`, `studio.test.ts`, `scenario.test.ts`, `websocket.test.ts`, `protocol.test.ts`, `project-switching.test.ts`, `generic-runtime.test.ts`, `store.test.ts`
- **Verdict:** **PASS**

---

## 6. Security Audit Trail Logs

Journal logs confirm all operational and administrative actions generated structured security audit events:
```
Sep 07 01:23:09 openclaw nebudcs-server: Security Event: /api/v1/training/sessions/.../load permitted for Instructor event="SESSION_LOAD" actor="Instructor" action="/api/v1/training/sessions/.../load-project" details="Session project loaded" permitted=true
Sep 07 01:23:09 openclaw nebudcs-server: Security Event: /api/v1/training/sessions/.../start permitted for Instructor event="SESSION_START" actor="Instructor" action="/api/v1/training/sessions/.../start" details="Training session commenced" permitted=true
Sep 07 01:23:10 openclaw nebudcs-server: Security Event: /api/v1/training/sessions/.../complete permitted for Instructor event="SESSION_COMPLETE" actor="Instructor" action="/api/v1/training/sessions/.../complete" details="Training session completed and evaluated" permitted=true
Sep 07 01:23:10 openclaw nebudcs-server: Security Event: /api/v1/admin/backup permitted for Admin event="ADMIN_BACKUP" actor="Admin" action="/api/v1/admin/backup" details="Initiating platform backup" permitted=true
Sep 07 01:23:10 openclaw nebudcs-server: Security Event: /api/v1/admin/restore permitted for Admin event="ADMIN_RESTORE" actor="Admin" action="/api/v1/admin/restore" details="Initiating platform restore" permitted=true
```

---

## 7. Operational Verdict

```
╔═════════════════════════════════════════════════════════════════════════════╗
║                                                                             ║
║   NEBUDCS PRODUCTION DEPLOYMENT & END-TO-END SMOKE TEST: PASS               ║
║                                                                             ║
║   - Release binary and web assets installed and active under systemd.       ║
║   - Linux security sandboxing (Strict, PrivateTmp, ProtectHome) active.     ║
║   - Multi-role RBAC, path traversal guard, and rate limiter verified.       ║
║   - Arbitrary generic flowsheet simulation, faceplate, and alarms verified. ║
║   - Training lifecycle, deterministic evaluation, and replay verified.      ║
║   - Crash recovery and cryptographic backup/restore verified.               ║
║   - Zero hardcoded refinery entities; Rust runtime sole process authority.  ║
║                                                                             ║
╚═════════════════════════════════════════════════════════════════════════════╝
```
