/**
 * Phase 15: Training Studio Web Component.
 * Comprehensive, modular interface providing:
 *  - Instructor Console (Session lifecycle, ad-hoc faults, live monitoring)
 *  - Scenario Editor (CRUD, entity validation, versioning, authoring)
 *  - Trainee Station (Restricted operator interface, action recording, process graphics)
 *  - Evaluation Report (Deterministic grading breakdown, evidence, scoring)
 *  - Replay Station (Read-only historical scrubber, marker track, frame reconstruction)
 */

import { ApiClient, sharedApiClient } from "../api/client";
import { TrainingStore, trainingStore, TrainingStudioTab, TrainingState } from "../state/training-store";
import {
  CreateTrainingSessionRequest,
  ScenarioAuthoringDto,
} from "../protocol/types";
import { store } from "../state/store";
import { authStore } from "../state/auth-store";

export class NebuTrainingStudio extends HTMLElement {
  public apiClient: ApiClient;
  public store: TrainingStore;
  private unsubscribe?: () => void;
  private unsubscribeAuth?: () => void;
  private replayPlaying: boolean = false;
  private replayTimer: number | null = null;

  constructor() {
    super();
    this.apiClient = sharedApiClient;
    this.store = trainingStore;
  }

  connectedCallback() {
    this.unsubscribe = this.store.subscribe((state) => this.render(state));
    this.unsubscribeAuth = authStore.subscribe(() => this.render(this.store.getState()));
    this.store.loadSessions();
    this.store.loadScenarios();
    this.store.loadProjects();
    this.render(this.store.getState());
  }

  disconnectedCallback() {
    if (this.replayTimer) {
      clearInterval(this.replayTimer);
      this.replayTimer = null;
    }
    if (this.unsubscribe) {
      this.unsubscribe();
    }
    if (this.unsubscribeAuth) {
      this.unsubscribeAuth();
    }
  }

  private switchTab(tab: TrainingStudioTab) {
    this.store.setActiveTab(tab);
  }

  render(state: TrainingState) {
    const session = state.activeSession;
    const sessionState = session ? session.state : "NONE";
    const role = authStore.getRole();
    const isTrainee = role === "trainee";

    if (isTrainee && (state.activeTab === "instructor" || state.activeTab === "scenario-editor")) {
      state.activeTab = "trainee";
    }

    this.innerHTML = `
      <div class="training-studio-container">
        <!-- Training Studio Navigation Bar -->
        <div class="training-nav-header">
          <div class="training-tabs">
            ${
              !isTrainee
                ? `
                  <button class="t-tab ${state.activeTab === "instructor" ? "active" : ""}" data-tab="instructor">
                    <span class="tab-icon">🎓</span> Instructor Console
                  </button>
                  <button class="t-tab ${state.activeTab === "scenario-editor" ? "active" : ""}" data-tab="scenario-editor">
                    <span class="tab-icon">📝</span> Scenario Editor
                  </button>
                `
                : ""
            }
            <button class="t-tab ${state.activeTab === "trainee" ? "active" : ""}" data-tab="trainee">
              <span class="tab-icon">🕹️</span> Trainee Station
            </button>
            <button class="t-tab ${state.activeTab === "evaluation" ? "active" : ""}" data-tab="evaluation">
              <span class="tab-icon">📊</span> Evaluation Report
            </button>
            <button class="t-tab ${state.activeTab === "replay" ? "active" : ""}" data-tab="replay">
              <span class="tab-icon">📼</span> Replay Station
            </button>
          </div>

          <div class="training-session-badge mono">
            ${
              session
                ? `
              <span class="session-label">SESSION:</span>
              <strong class="session-name">${session.name}</strong>
              <span class="badge badge-state-${sessionState.toLowerCase()}">${sessionState}</span>
              <span class="session-timer">T=${session.current_simulation_time_sec.toFixed(1)}s</span>
            `
                : `<span class="text-dim">No active training session selected</span>`
            }
          </div>
        </div>

        ${
          state.error
            ? `<div class="training-error-banner"><span class="err-icon">⚠️</span> ${state.error}</div>`
            : ""
        }

        <!-- Tab Content Views -->
        <div class="training-content-area">
          ${this.renderActiveTab(state)}
        </div>
      </div>
    `;

    this.bindEvents(state);
  }

  private renderActiveTab(state: TrainingState): string {
    const role = authStore.getRole();
    if (role === "trainee" && (state.activeTab === "instructor" || state.activeTab === "scenario-editor")) {
      return this.renderTraineeStation(state);
    }

    switch (state.activeTab) {
      case "instructor":
        return this.renderInstructorConsole(state);
      case "scenario-editor":
        return this.renderScenarioEditor(state);
      case "trainee":
        return this.renderTraineeStation(state);
      case "evaluation":
        return this.renderEvaluationReport(state);
      case "replay":
        return this.renderReplayStation(state);
    }
  }

  // ── 1. Instructor Console ────────────────────────────────────────────────

  private renderInstructorConsole(state: TrainingState): string {
    const session = state.activeSession;

    return `
      <div class="instructor-grid">
        <!-- Left: Session Management & Execution Control -->
        <div class="card instructor-panel">
          <div class="card-header">
            <h3>Active Training Session Control</h3>
            <button class="btn btn-sm btn-primary" id="btnOpenCreateSession">➕ New Session</button>
          </div>
          <div class="card-body">
            ${
              session
                ? `
              <div class="session-details-card mono">
                <div class="detail-row"><span>Session ID:</span> <strong>${session.id}</strong></div>
                <div class="detail-row"><span>Project:</span> <span>${session.project_id || "Unassigned"}</span></div>
                <div class="detail-row"><span>Scenario:</span> <span>${session.scenario_id || "Unassigned"}</span></div>
                <div class="detail-row"><span>Instructor:</span> <span>${session.instructor_id}</span></div>
                <div class="detail-row"><span>Trainee:</span> <span>${session.trainee_id}</span></div>
                <div class="detail-row"><span>Status:</span> <span class="badge badge-state-${session.state.toLowerCase()}">${session.state}</span></div>
                <div class="detail-row"><span>Sim Time:</span> <span>${session.current_simulation_time_sec.toFixed(1)} s</span></div>
              </div>

              <div class="instructor-action-toolbar mt-3">
                <button class="btn btn-sm btn-info" id="btnLoadProject" ${session.state === "RUNNING" ? "disabled" : ""}>
                  📥 Load Project & Scenario
                </button>
                <button class="btn btn-sm btn-secondary" id="btnPrepareSession" ${session.state === "RUNNING" || session.state === "READY" ? "disabled" : ""}>
                  ⚙️ Prepare
                </button>
                <button class="btn btn-sm btn-success" id="btnStartSession" ${session.state === "RUNNING" ? "disabled" : ""}>
                  ▶ Start Session
                </button>
                <button class="btn btn-sm btn-warning" id="btnPauseSession" ${session.state !== "RUNNING" ? "disabled" : ""}>
                  ⏸ Pause
                </button>
                <button class="btn btn-sm btn-info" id="btnResumeSession" ${session.state !== "PAUSED" ? "disabled" : ""}>
                  ⏯ Resume
                </button>
                <button class="btn btn-sm btn-secondary" id="btnResetSession">
                  🔄 Reset
                </button>
                <button class="btn btn-sm btn-primary" id="btnCompleteSession" ${session.state !== "RUNNING" && session.state !== "PAUSED" ? "disabled" : ""}>
                  🏁 Complete & Grade
                </button>
                <button class="btn btn-sm btn-danger" id="btnAbortSession" ${session.state === "COMPLETED" || session.state === "ABORTED" ? "disabled" : ""}>
                  🛑 Abort
                </button>
              </div>
            `
                : `
              <div class="empty-state-notice text-dim">
                Select a session from the list below or create a new session to begin.
              </div>
            `
            }
          </div>

          <!-- Session List Table -->
          <div class="card-header mt-3">
            <h3>All Training Sessions</h3>
          </div>
          <div class="table-container">
            <table class="data-table mono">
              <thead>
                <tr>
                  <th>Name</th>
                  <th>State</th>
                  <th>Trainee</th>
                  <th>Sim Time</th>
                  <th>Action</th>
                </tr>
              </thead>
              <tbody>
                ${state.sessions
                  .map(
                    (s) => `
                  <tr class="${session?.id === s.id ? "selected-row" : ""}">
                    <td><strong>${s.name}</strong></td>
                    <td><span class="badge badge-state-${s.state.toLowerCase()}">${s.state}</span></td>
                    <td>${s.trainee_id}</td>
                    <td>${s.current_sim_time_sec.toFixed(1)}s</td>
                    <td>
                      <button class="btn btn-xs btn-outline btn-select-session" data-id="${s.id}">Select</button>
                    </td>
                  </tr>
                `
                  )
                  .join("")}
                ${state.sessions.length === 0 ? `<tr><td colspan="5" class="text-center text-dim">No sessions found</td></tr>` : ""}
              </tbody>
            </table>
          </div>
        </div>

        <!-- Right: Ad-hoc Fault Injection & Telemetry Overrides -->
        <div class="card fault-injection-panel">
          <div class="card-header">
            <h3>Ad-Hoc Fault Injection (Instructor)</h3>
            <span class="badge badge-warning">AUTHORITATIVE CORE</span>
          </div>
          <div class="card-body">
            <p class="text-dim text-sm">
              Inject live operational disturbances directly into the process model to assess trainee response.
            </p>

            <div class="fault-cards-grid">
              <div class="fault-card">
                <h4>🚰 Feed Loss</h4>
                <p class="text-xs text-dim">Cuts upstream supply to 0 kg/h, draining vertical separator vessel.</p>
                <button class="btn btn-sm btn-outline-danger btn-inject-feed-loss" ${session?.state !== "RUNNING" ? "disabled" : ""}>
                  Trigger Feed Loss
                </button>
              </div>

              <div class="fault-card">
                <h4>❄️ Cooling Water Failure</h4>
                <p class="text-xs text-dim">Reduces cooling pressure to 0.5 bar, inducing exotherm / thermal spike.</p>
                <button class="btn btn-sm btn-outline-danger btn-inject-cooling-loss" ${session?.state !== "RUNNING" ? "disabled" : ""}>
                  Trigger Cooling Loss
                </button>
              </div>

              <div class="fault-card">
                <h4>🕹️ Valve Sticking</h4>
                <p class="text-xs text-dim">Freezes FV-101 actuator position demand, disabling automatic throttling.</p>
                <button class="btn btn-sm btn-outline-danger btn-inject-valve-stick" ${session?.state !== "RUNNING" ? "disabled" : ""}>
                  Trigger Valve Freeze
                </button>
              </div>

              <div class="fault-card">
                <h4>📡 Sensor Signal Loss</h4>
                <p class="text-xs text-dim">Drops transmitter quality to BAD and zeros signal, tripping alarms.</p>
                <button class="btn btn-sm btn-outline-danger btn-inject-sensor-loss" ${session?.state !== "RUNNING" ? "disabled" : ""}>
                  Trigger Sensor Loss
                </button>
              </div>
            </div>

            <!-- Dynamic Scenario Branching Panel (ANSI/ISA-77.20 / ISA-18.2) -->
            <div class="scenario-branch-panel mt-3">
              <div class="card-header" style="display: flex; justify-content: space-between; align-items: center;">
                <h4>Dynamic Scenario Branches</h4>
                <span class="badge badge-info">${state.branches.length} Defined</span>
              </div>
              <div class="card-body">
                <p class="text-xs text-dim" style="margin-bottom: 8px;">
                  Branches evaluate against runtime plant states (ANSI/ISA-77.20). Instructors may trigger manual override.
                </p>
                ${
                  state.branches.length === 0
                    ? `<div class="text-xs text-dim" style="font-style: italic;">No dynamic branches configured for active scenario.</div>`
                    : `
                  <div class="branch-cards-grid">
                    ${state.branches
                      .map(
                        (b) => `
                      <div class="branch-card ${b.triggered ? "branch-triggered" : ""}">
                        <div class="branch-card-header">
                          <span class="mono" style="font-weight: 700;">${b.branch_id}</span>
                          <span class="badge ${b.triggered ? "badge-success" : "badge-secondary"}">
                            ${b.triggered ? "TRIGGERED" : "ARMED"}
                          </span>
                        </div>
                        <div class="text-xs mt-1">Target: <code>${b.target_step_id}</code></div>
                        <div class="text-xs text-dim mt-1">${b.description}</div>
                        <div class="text-xs mono text-warning mt-1">Cond: ${b.condition_summary}</div>
                        <button class="btn btn-xs btn-outline-warning mt-2 btn-trigger-branch" data-branch="${b.branch_id}" ${b.triggered || session?.state !== "RUNNING" ? "disabled" : ""}>
                          ⚡ Trigger Branch
                        </button>
                      </div>
                    `
                      )
                      .join("")}
                  </div>
                `
                }
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Modal: Create Training Session -->
      <div class="modal-backdrop" id="modalCreateSession" style="display: none;">
        <div class="modal-dialog">
          <div class="modal-header">
            <h3>Create Training Session</h3>
            <button class="modal-close" id="btnCloseCreateSession">✕</button>
          </div>
          <div class="modal-body">
            <form id="formCreateSession">
              <div class="form-group">
                <label>Session Name</label>
                <input type="text" id="inpSessionName" class="form-control" required placeholder="e.g. Unit 100 Surge Control Exam" />
              </div>
              <div class="form-group">
                <label>Description</label>
                <textarea id="inpSessionDesc" class="form-control" rows="2" placeholder="Objectives and instructions for trainee"></textarea>
              </div>
              <div class="form-row">
                <div class="form-group col">
                  <label>Instructor ID</label>
                  <input type="text" id="inpInstructorId" class="form-control" value="CHIEF-INSTRUCTOR-01" required />
                </div>
                <div class="form-group col">
                  <label>Trainee ID</label>
                  <input type="text" id="inpTraineeId" class="form-control" value="OPERATOR-TRAINEE-04" required />
                </div>
              </div>
              <div class="form-row">
                <div class="form-group col">
                  <label>Plant Project</label>
                  <select id="selProjectId" class="form-control" required>
                    ${
                      state.projects.length === 0
                        ? `<option value="">No projects available</option>`
                        : state.projects
                            .map(
                              (p) => `<option value="${p.id}">${p.name} (${p.id})</option>`
                            )
                            .join("")
                    }
                  </select>
                </div>
                <div class="form-group col">
                  <label>Scenario</label>
                  <select id="selScenarioId" class="form-control" required>
                    ${state.scenarios.map((s) => `<option value="${s.id}">${s.name} (${s.id})</option>`).join("")}
                  </select>
                </div>
              </div>
              <div class="modal-footer">
                <button type="button" class="btn btn-secondary" id="btnCancelCreateSession">Cancel</button>
                <button type="submit" class="btn btn-primary">Create Session</button>
              </div>
            </form>
          </div>
        </div>
      </div>
    `;
  }

  // ── 2. Scenario Editor ──────────────────────────────────────────────────

  private renderScenarioEditor(state: TrainingState): string {
    const scn = state.editingScenario;

    return `
      <div class="scenario-editor-layout">
        <!-- Left: Scenario Catalog -->
        <div class="card scenario-catalog-panel">
          <div class="card-header">
            <h3>Scenario Catalog</h3>
            <button class="btn btn-sm btn-primary" id="btnNewScenario">➕ New Scenario</button>
          </div>
          <div class="table-container">
            <table class="data-table mono">
              <thead>
                <tr>
                  <th>ID</th>
                  <th>Name</th>
                  <th>Duration</th>
                  <th>Actions</th>
                </tr>
              </thead>
              <tbody>
                ${state.scenarios
                  .map(
                    (s) => `
                  <tr class="${scn?.id === s.id ? "selected-row" : ""}">
                    <td><strong>${s.id}</strong></td>
                    <td>${s.name}</td>
                    <td>${s.duration_limit_sec ? `${s.duration_limit_sec}s` : "Unlimited"}</td>
                    <td>
                      <button class="btn btn-xs btn-outline btn-edit-scenario" data-id="${s.id}">Edit</button>
                      <button class="btn btn-xs btn-outline btn-dup-scenario" data-id="${s.id}">Copy</button>
                      <button class="btn btn-xs btn-outline-danger btn-del-scenario" data-id="${s.id}">Del</button>
                    </td>
                  </tr>
                `
                  )
                  .join("")}
              </tbody>
            </table>
          </div>
        </div>

        <!-- Right: Scenario Authoring Details Form -->
        <div class="card scenario-authoring-panel">
          <div class="card-header">
            <h3>${scn ? `Editing Scenario: ${scn.id}` : "Scenario Authoring & Validation"}</h3>
            ${
              scn
                ? `
              <div class="authoring-actions">
                <button class="btn btn-sm btn-info" id="btnValidateScenario">🔍 Validate Against Project</button>
                <button class="btn btn-sm btn-success" id="btnSaveScenario">💾 Save Scenario</button>
              </div>
            `
                : ""
            }
          </div>
          <div class="card-body">
            ${
              scn
                ? `
              <div id="validationResultBox"></div>

              <form id="formScenarioAuthoring">
                <div class="form-row">
                  <div class="form-group col">
                    <label>Scenario ID</label>
                    <input type="text" id="inpScnId" class="form-control mono" value="${scn.id}" required />
                  </div>
                  <div class="form-group col">
                    <label>Scenario Title</label>
                    <input type="text" id="inpScnName" class="form-control" value="${scn.name}" required />
                  </div>
                </div>

                <div class="form-group">
                  <label>Description & Objectives</label>
                  <textarea id="inpScnDesc" class="form-control" rows="2">${scn.description}</textarea>
                </div>

                <div class="form-row">
                  <div class="form-group col">
                    <label>Author</label>
                    <input type="text" id="inpScnAuthor" class="form-control" value="${scn.author}" />
                  </div>
                  <div class="form-group col">
                    <label>Version</label>
                    <input type="text" id="inpScnVersion" class="form-control mono" value="${scn.version}" />
                  </div>
                  <div class="form-group col">
                    <label>Duration Limit (seconds)</label>
                    <input type="number" id="inpScnDuration" class="form-control mono" value="${scn.duration_limit_sec || 300}" />
                  </div>
                </div>

                <div class="form-group mt-2">
                  <label>Evaluation Criteria Definition (JSON)</label>
                  <textarea id="inpScnCriteria" class="form-control mono code-editor" rows="6">${scn.criteria_json}</textarea>
                </div>

                <div class="form-group mt-2">
                  <label>Initial Conditions Overrides (JSON)</label>
                  <textarea id="inpScnInit" class="form-control mono code-editor" rows="3">${scn.initial_conditions_json}</textarea>
                </div>

                <div class="form-group mt-2">
                  <label>Scenario Timeline Events (JSON Array)</label>
                  <textarea id="inpScnTimeline" class="form-control mono code-editor" rows="3">${scn.timeline_json || "[]"}</textarea>
                </div>

                <div class="form-group mt-2">
                  <label>Training Objectives (JSON Array)</label>
                  <textarea id="inpScnObjectives" class="form-control mono code-editor" rows="3">${scn.objectives_json || "[]"}</textarea>
                </div>
              </form>
            `
                : `
              <div class="empty-state-notice text-dim">
                Select a scenario from the catalog or click "New Scenario" to start authoring.
              </div>
            `
            }
          </div>
        </div>
      </div>
    `;
  }

  // ── 3. Trainee Station ──────────────────────────────────────────────────

  private renderTraineeStation(state: TrainingState): string {
    const session = state.activeSession;

    return `
      <div class="trainee-station-container">
        <!-- Trainee Control Toolbar (ISA-101 Style Operator Controls) -->
        <div class="trainee-action-bar card">
          <div class="trainee-info">
            <span class="badge badge-info">TRAINEE OPERATOR CONSOLE</span>
            <span class="mono">OPERATOR: <strong>${session?.trainee_id || "ANONYMOUS"}</strong></span>
            <span class="mono">SESSION: <strong>${session?.id || "NO_ACTIVE_SESSION"}</strong></span>
          </div>

          <div class="trainee-controls-group">
            <div class="control-item">
              <label>Target Loop</label>
              <select id="selTraineeTarget" class="form-control-sm mono">
                ${
                  store.activeProject && store.activeProject.control_loops.length > 0
                    ? store.activeProject.control_loops
                        .map((cl) => `<option value="${cl.name}">${cl.name} (${cl.pv_tag})</option>`)
                        .join("")
                    : `<option value="LIC-101">LIC-101 (Level Control)</option>`
                }
              </select>
            </div>

            <div class="control-item">
              <label>Set SP</label>
              <input type="number" id="inpTraineeSp" class="form-control-sm mono" step="0.5" value="50.0" style="width: 70px;" />
              <button class="btn btn-xs btn-primary" id="btnTraineeSetSp" ${session?.state !== "RUNNING" ? "disabled" : ""}>Set</button>
            </div>

            <div class="control-item">
              <label>Mode</label>
              <button class="btn btn-xs btn-outline" id="btnModeMan" ${session?.state !== "RUNNING" ? "disabled" : ""}>MAN</button>
              <button class="btn btn-xs btn-outline" id="btnModeAuto" ${session?.state !== "RUNNING" ? "disabled" : ""}>AUTO</button>
              <button class="btn btn-xs btn-outline" id="btnModeCas" ${session?.state !== "RUNNING" ? "disabled" : ""}>CAS</button>
            </div>

            <div class="control-item">
              <label>Output %</label>
              <input type="number" id="inpTraineeOut" class="form-control-sm mono" step="1.0" min="0" max="100" value="45" style="width: 60px;" />
              <button class="btn btn-xs btn-primary" id="btnTraineeSetOut" ${session?.state !== "RUNNING" ? "disabled" : ""}>Apply</button>
            </div>

            <div class="control-item">
              <label>Alarm / Trip Action</label>
              <button class="btn btn-xs btn-warning" id="btnTraineeAckAlarm" ${session?.state !== "RUNNING" ? "disabled" : ""}>ACK Alarm</button>
              <button class="btn btn-xs btn-danger" id="btnTraineeResetTrip" ${session?.state !== "RUNNING" ? "disabled" : ""}>Reset Trip</button>
            </div>

            <div class="control-item">
              <label>RBAC Security</label>
              <button class="btn btn-xs btn-outline-danger" id="btnTraineeForbiddenFault" ${session?.state !== "RUNNING" ? "disabled" : ""}>⚠️ Test Admin Guard</button>
            </div>
          </div>
        </div>

        <!-- Trainee Action Feedback Banner -->
        <div id="traineeFeedbackBanner" class="mono text-xs mt-1 mb-1 alert" style="display: none; padding: 6px 12px;"></div>

        <!-- DCS Operator HMI Embedded Layout -->
        <div class="trainee-hmi-grid mt-2">
          <div class="trainee-process-column">
            <nebu-process-view></nebu-process-view>
            <div class="hmi-subgrid mt-2">
              <nebu-faceplate></nebu-faceplate>
              <nebu-trend></nebu-trend>
            </div>
          </div>
          <div class="trainee-alarm-column">
            <nebu-alarm-panel></nebu-alarm-panel>
          </div>
        </div>
      </div>
    `;
  }

  // ── 4. Evaluation Report ────────────────────────────────────────────────

  private renderEvaluationReport(state: TrainingState): string {
    const report = state.evaluationReport;

    if (!report) {
      return `
        <div class="card evaluation-card">
          <div class="card-header">
            <h3>Trainee Performance Assessment & Grading</h3>
          </div>
          <div class="card-body empty-state-notice text-dim">
            No evaluation report loaded. Complete an active session to generate explainable grading evidence.
          </div>
        </div>
      `;
    }

    const passed = report.passed;

    return `
      <div class="card evaluation-card">
        <!-- Evaluation Banner -->
        <div class="eval-header-banner ${passed ? "eval-pass" : "eval-fail"}">
          <div class="eval-badge-title">
            <span class="eval-icon">${passed ? "✅" : "❌"}</span>
            <div>
              <h2>ASSESSMENT OUTCOME: ${passed ? "PASSED" : "FAILED"}</h2>
              <span class="mono">Session ID: ${report.session_id} | Trainee: ${report.trainee_id}</span>
            </div>
          </div>
          <div class="eval-score-summary mono">
            <div class="score-number">${report.percentage.toFixed(1)}%</div>
            <div class="score-fraction">${report.total_score} / ${report.max_possible_score} Points</div>
          </div>
        </div>

        <!-- Criteria Evidence Table -->
        <div class="card-body">
          <h3>Deterministic Criteria Breakdown</h3>
          <div class="table-container mt-2">
            <table class="data-table mono">
              <thead>
                <tr>
                  <th>Criterion</th>
                  <th>Result</th>
                  <th>Classification</th>
                  <th>Score</th>
                  <th>Explanation</th>
                  <th>Evidence Details</th>
                </tr>
              </thead>
              <tbody>
                ${report.results
                  .map(
                    (r) => `
                  <tr class="${r.passed ? "pass-row" : "fail-row"}">
                    <td><strong>${r.title}</strong><br><span class="text-xs text-dim">${r.criterion_id}</span></td>
                    <td><span class="badge ${r.passed ? "badge-success" : "badge-danger"}">${r.passed ? "PASS" : "FAIL"}</span></td>
                    <td>
                      ${r.error_classification ? `<span class="badge badge-warning">${r.error_classification}</span>` : `<span class="text-dim">-</span>`}
                      ${r.safety_trip_assessment ? `<br><span class="text-xs text-danger">${r.safety_trip_assessment}</span>` : ""}
                    </td>
                    <td>+${r.score_awarded} / ${r.score_possible} pts ${r.penalty_deducted > 0 ? `(-${r.penalty_deducted})` : ""}</td>
                    <td>${r.explanation}</td>
                    <td class="text-xs">
                      ${Object.entries(r.evidence || {})
                        .map(([k, v]) => `<div><strong>${k}:</strong> ${v}</div>`)
                        .join("")}
                    </td>
                  </tr>
                `
                  )
                  .join("")}
              </tbody>
            </table>
          </div>

          <!-- Structured Debrief Panel -->
          ${(() => {
            const debrief = report.debrief || state.debrief;
            if (!debrief) return "";
            return `
              <div class="structured-debrief-container mt-4">
                <div class="card-header debrief-header">
                  <h3 style="margin: 0; color: #fff;">📋 Structured Post-Run Debrief (ANSI/ISA-77.20 / ISA-101)</h3>
                  <span class="badge badge-primary">${debrief.outcome_verdict}</span>
                </div>
                <div class="card-body">
                  <div class="debrief-grid">
                    <div class="debrief-section">
                      <h4>📈 Process Stability & KPIs</h4>
                      <p class="text-sm">${debrief.process_performance}</p>
                    </div>
                    <div class="debrief-section">
                      <h4>⏱️ Operator Timeliness & Actions</h4>
                      <p class="text-sm">${debrief.operator_performance}</p>
                    </div>
                    <div class="debrief-section">
                      <h4>🛡️ Safety & SIS Interlock Adherence</h4>
                      <p class="text-sm">${debrief.safety_assessment}</p>
                    </div>
                  </div>
                  <div class="debrief-recommendations mt-3">
                    <h4>💡 Instructor Recommendations</h4>
                    <ul class="recommendation-list">
                      ${debrief.recommendations.map((rec) => `<li>${rec}</li>`).join("")}
                    </ul>
                  </div>
                </div>
              </div>
            `;
          })()}

          <div class="eval-footer-actions mt-3">
            <button class="btn btn-primary" id="btnLoadReplayFromEval" data-session="${report.session_id}">
              📼 Load Replay for This Session
            </button>
          </div>
        </div>
      </div>
    `;
  }

  // ── 5. Replay Station ───────────────────────────────────────────────────

  private renderReplayStation(state: TrainingState): string {
    const summary = state.replaySummary;
    const frame = state.replayFrame;
    const currentTime = state.replaySimTime;

    if (!summary) {
      return `
        <div class="card replay-card">
          <div class="card-header">
            <h3>Historical Training Session Replay</h3>
          </div>
          <div class="card-body empty-state-notice text-dim">
            No replay session loaded. Complete a session or select one to inspect read-only playback.
          </div>
        </div>
      `;
    }

    const minTime = summary.start_sim_time_sec;
    const maxTime = summary.end_sim_time_sec;

    return `
      <div class="card replay-card">
        <!-- Scrubber and Timeline Header -->
        <div class="card-header">
          <div>
            <h3>Historical Session Playback: ${summary.session_id}</h3>
            <span class="text-xs mono text-dim">Duration: ${summary.duration_sec.toFixed(1)}s (T=${minTime.toFixed(1)}s to T=${maxTime.toFixed(1)}s)</span>
          </div>
          <div class="playback-controls">
            <button class="btn btn-sm btn-primary" id="btnReplayPlayPause">${this.replayPlaying ? "⏸ Pause" : "▶ Play"}</button>
            <button class="btn btn-sm btn-secondary" id="btnReplayStepBack">⏮ -1s</button>
            <button class="btn btn-sm btn-secondary" id="btnReplayStepFwd">⏭ +1s</button>
            <span class="badge badge-info mono" id="replayTimeBadge">T = ${currentTime.toFixed(2)} s</span>
          </div>
        </div>

        <div class="card-body">
          <!-- Timeline Scrubber Bar -->
          <div class="replay-timeline-container">
            <input type="range" min="${minTime}" max="${maxTime}" step="0.1" value="${currentTime}" id="replayScrubber" class="timeline-slider" />

            <!-- Chronological Event Marker Track -->
            <div class="marker-track">
              ${summary.markers
                .map((m) => {
                  const percent = maxTime > minTime ? ((m.sim_time_sec - minTime) / (maxTime - minTime)) * 100 : 0;
                  return `
                  <div class="marker-pin marker-${m.kind.toLowerCase()}" style="left: ${percent}%;" title="${m.label} at ${m.sim_time_sec.toFixed(1)}s: ${m.details}">
                    📍
                  </div>
                `;
                })
                .join("")}
            </div>
          </div>

          <!-- Replay Frame Inspection Grid -->
          <div class="replay-frame-grid mt-3">
            <!-- Left: Interpolated Tag Values -->
            <div class="frame-tags-card">
              <h4>Reconstructed Process State (T=${currentTime.toFixed(2)}s)</h4>
              <div class="table-container">
                <table class="data-table mono text-xs">
                  <thead>
                    <tr><th>Tag</th><th>Interpolated Value</th></tr>
                  </thead>
                  <tbody>
                    ${
                      frame
                        ? Object.entries(frame.tags)
                            .map(
                              ([t, v]) => `
                          <tr><td><strong>${t}</strong></td><td>${v.toFixed(3)}</td></tr>
                        `
                            )
                            .join("")
                        : `<tr><td colspan="2" class="text-dim">No frame loaded</td></tr>`
                    }
                  </tbody>
                </table>
              </div>
            </div>

            <!-- Right: Active Alarms, Trips & Trainee Actions -->
            <div class="frame-events-card">
              <h4>Active Alarms & Protective Trips</h4>
              <div class="active-status-box mono">
                <div><strong>Alarms:</strong> ${frame?.active_alarms.length ? frame.active_alarms.join(", ") : "None"}</div>
                <div><strong>Trips:</strong> ${frame?.active_trips.length ? frame.active_trips.join(", ") : "None"}</div>
              </div>

              <h4 class="mt-3">Recent Trainee Interventions</h4>
              <div class="table-container">
                <table class="data-table mono text-xs">
                  <thead>
                    <tr><th>Time</th><th>Actor</th><th>Action</th><th>Status</th></tr>
                  </thead>
                  <tbody>
                    ${
                      frame?.recent_actions?.length
                        ? frame.recent_actions
                            .map(
                              (a) => `
                          <tr>
                            <td>${a.sim_time_sec.toFixed(1)}s</td>
                            <td>${a.actor}</td>
                            <td>${a.action_type} (${a.target_entity})</td>
                            <td><span class="badge badge-sm">${a.status}</span></td>
                          </tr>
                        `
                            )
                            .join("")
                        : `<tr><td colspan="4" class="text-dim">No recent trainee actions</td></tr>`
                    }
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        </div>
      </div>
    `;
  }

  // ── Event Bindings ───────────────────────────────────────────────────────

  private bindEvents(state: TrainingState) {
    // Tab Switching
    this.querySelectorAll(".t-tab").forEach((btn) => {
      btn.addEventListener("click", () => {
        const tab = btn.getAttribute("data-tab") as TrainingStudioTab;
        if (tab) this.switchTab(tab);
      });
    });

    // Instructor: Modal open/close
    const modal = this.querySelector("#modalCreateSession") as HTMLElement;
    this.querySelector("#btnOpenCreateSession")?.addEventListener("click", () => {
      if (modal) modal.style.display = "flex";
    });
    this.querySelector("#btnCloseCreateSession")?.addEventListener("click", () => {
      if (modal) modal.style.display = "none";
    });
    this.querySelector("#btnCancelCreateSession")?.addEventListener("click", () => {
      if (modal) modal.style.display = "none";
    });

    // Instructor: Create Session Form Submit
    this.querySelector("#formCreateSession")?.addEventListener("submit", async (e) => {
      e.preventDefault();
      const name = (this.querySelector("#inpSessionName") as HTMLInputElement)?.value;
      const description = (this.querySelector("#inpSessionDesc") as HTMLTextAreaElement)?.value;
      const instructor_id = (this.querySelector("#inpInstructorId") as HTMLInputElement)?.value;
      const trainee_id = (this.querySelector("#inpTraineeId") as HTMLInputElement)?.value;
      const project_id =
        (this.querySelector("#selProjectId") as HTMLSelectElement)?.value ||
        (state.projects.length > 0 ? state.projects[0].id : "proj-vertical-slice-01");
      const scenario_id = (this.querySelector("#selScenarioId") as HTMLSelectElement)?.value;

      const req: CreateTrainingSessionRequest = {
        name,
        description,
        instructor_id,
        trainee_id,
        project_id,
        scenario_id,
      };

      await this.store.createSession(req);
      if (modal) modal.style.display = "none";
    });

    // Select Session row
    this.querySelectorAll(".btn-select-session").forEach((btn) => {
      btn.addEventListener("click", () => {
        const id = btn.getAttribute("data-id");
        if (id) this.store.selectSession(id);
      });
    });

    // Session Execution Buttons
    const session = state.activeSession;
    if (session) {
      this.querySelector("#btnLoadProject")?.addEventListener("click", () => {
        const projId =
          session.project_id || (state.projects.length > 0 ? state.projects[0].id : "proj-vertical-slice-01");
        const scnId =
          session.scenario_id || (state.scenarios.length > 0 ? state.scenarios[0].id : "scn-generic-feed-loss");
        this.store.loadSessionProject(session.id, {
          project_id: projId,
          scenario_id: scnId,
        });
      });

      this.querySelector("#btnPrepareSession")?.addEventListener("click", () => {
        this.store.prepareSession(session.id);
      });

      this.querySelector("#btnStartSession")?.addEventListener("click", () => {
        this.store.startSession(session.id);
      });

      this.querySelector("#btnPauseSession")?.addEventListener("click", () => {
        this.store.pauseSession(session.id);
      });

      this.querySelector("#btnResumeSession")?.addEventListener("click", () => {
        this.store.resumeSession(session.id);
      });

      this.querySelector("#btnResetSession")?.addEventListener("click", () => {
        this.store.resetSession(session.id);
      });

      this.querySelector("#btnCompleteSession")?.addEventListener("click", () => {
        this.store.completeSession(session.id);
      });

      this.querySelector("#btnAbortSession")?.addEventListener("click", () => {
        const reason = prompt("Enter instructor reason for aborting session:", "Terminated by instructor");
        if (reason) this.store.abortSession(session.id, reason);
      });
    }

    // Ad-Hoc Fault Injections
    this.querySelectorAll(".btn-trigger-branch").forEach((btn) => {
      btn.addEventListener("click", (e) => {
        const branchId = (e.currentTarget as HTMLElement).dataset.branch;
        if (session && branchId) {
          this.store.triggerBranch(session.id, branchId);
        }
      });
    });

    this.querySelector(".btn-inject-feed-loss")?.addEventListener("click", async () => {
      try {
        const loop = store.activeProject?.control_loops[0];
        const target = loop?.pv_tag || "LIC-902.PV";
        await this.apiClient.injectFault({
          id: `FLT-FEED-${Date.now().toString().slice(-4)}`,
          target,
          fault_type: {
            category: "sensor_bias",
            offset: -50.0,
          },
          duration_sec: 60.0,
        });
        alert(`Injected Supply Disturbance on ${target}.`);
      } catch (err: any) {
        alert(`Fault injection failed: ${err.message || err}`);
      }
    });

    this.querySelector(".btn-inject-cooling-loss")?.addEventListener("click", async () => {
      try {
        const target = store.activeProject?.control_loops[1]?.pv_tag || "TIC-101.PV";
        await this.apiClient.injectFault({
          id: `FLT-COOL-${Date.now().toString().slice(-4)}`,
          target,
          fault_type: {
            category: "sensor_bias",
            offset: 25.0,
          },
          duration_sec: 60.0,
        });
        alert(`Injected Cooling Loss disturbance on ${target}.`);
      } catch (err: any) {
        alert(`Fault injection failed: ${err.message || err}`);
      }
    });

    // Scenario Editor Actions
    this.querySelector("#btnNewScenario")?.addEventListener("click", () => {
      this.store.setEditingScenario({
        id: `scn-${Date.now().toString().slice(-6)}`,
        name: "New Operator Training Scenario",
        description: "Authoring drill in NebuDCS Training Studio",
        author: "Chief Instructor",
        version: "1.0.0",
        duration_limit_sec: 300,
        random_seed: 42,
        criteria_json: "[]",
        initial_conditions_json: "{}",
        timeline_json: "[]",
        objectives_json: "[]",
      });
    });

    this.querySelectorAll(".btn-edit-scenario").forEach((btn) => {
      btn.addEventListener("click", () => {
        const id = btn.getAttribute("data-id");
        if (id) this.store.loadScenarioForEdit(id);
      });
    });

    this.querySelectorAll(".btn-dup-scenario").forEach((btn) => {
      btn.addEventListener("click", async () => {
        const id = btn.getAttribute("data-id");
        if (id) {
          const newId = prompt("Enter new scenario ID:", `${id}-copy`);
          if (newId) {
            await this.apiClient.duplicateScenario(id, newId);
            await this.store.loadScenarios();
          }
        }
      });
    });

    this.querySelectorAll(".btn-del-scenario").forEach((btn) => {
      btn.addEventListener("click", async () => {
        const id = btn.getAttribute("data-id");
        if (id && confirm(`Delete scenario '${id}'?`)) {
          await this.store.deleteScenario(id);
        }
      });
    });

    this.querySelector("#btnSaveScenario")?.addEventListener("click", async () => {
      const id = (this.querySelector("#inpScnId") as HTMLInputElement)?.value?.trim();
      const name = (this.querySelector("#inpScnName") as HTMLInputElement)?.value?.trim();
      const description = (this.querySelector("#inpScnDesc") as HTMLTextAreaElement)?.value?.trim() || "";
      const author = (this.querySelector("#inpScnAuthor") as HTMLInputElement)?.value?.trim() || "Instructor";
      const version = (this.querySelector("#inpScnVersion") as HTMLInputElement)?.value?.trim() || "1.0.0";
      const duration_limit_sec = parseFloat((this.querySelector("#inpScnDuration") as HTMLInputElement)?.value || "300");
      const criteria_json = (this.querySelector("#inpScnCriteria") as HTMLTextAreaElement)?.value?.trim() || "[]";
      const initial_conditions_json = (this.querySelector("#inpScnInit") as HTMLTextAreaElement)?.value?.trim() || "{}";
      const timeline_json = (this.querySelector("#inpScnTimeline") as HTMLTextAreaElement)?.value?.trim() || "[]";
      const objectives_json = (this.querySelector("#inpScnObjectives") as HTMLTextAreaElement)?.value?.trim() || "[]";

      if (!id || !name) {
        alert("Scenario ID and Name are required");
        return;
      }

      const dto: ScenarioAuthoringDto = {
        id,
        name,
        description,
        author,
        version,
        duration_limit_sec,
        random_seed: state.editingScenario?.random_seed ?? 42,
        criteria_json,
        initial_conditions_json,
        timeline_json,
        objectives_json,
      };

      try {
        await this.store.saveScenario(dto);
        const box = this.querySelector("#validationResultBox");
        if (box) {
          box.innerHTML = `
            <div class="alert alert-success mono text-xs mb-2">
              💾 Scenario '<strong>${id}</strong>' saved successfully to catalog.
            </div>
          `;
        }
      } catch (err: any) {
        const box = this.querySelector("#validationResultBox");
        if (box) {
          box.innerHTML = `
            <div class="alert alert-danger mono text-xs mb-2">
              ❌ Failed to save scenario: ${err.message || err}
            </div>
          `;
        }
      }
    });

    this.querySelector("#btnValidateScenario")?.addEventListener("click", async () => {
      if (state.editingScenario) {
        const res = await this.apiClient.validateScenario(state.editingScenario.id);
        const box = this.querySelector("#validationResultBox");
        if (box) {
          box.innerHTML = `
            <div class="alert ${res.valid ? "alert-success" : "alert-danger"} mono text-xs mb-2">
              <strong>${res.valid ? "VALIDATION PASSED" : "VALIDATION FAILED"}</strong>
              ${res.errors.map((e) => `<div>❌ ${e}</div>`).join("")}
              ${res.warnings.map((w) => `<div>⚠️ ${w}</div>`).join("")}
            </div>
          `;
        }
      }
    });

    // Trainee Action Dispatches
    if (session) {
      const showTraineeFeedback = (msg: string, isError: boolean = false) => {
        const banner = this.querySelector("#traineeFeedbackBanner") as HTMLElement;
        if (banner) {
          banner.style.display = "block";
          banner.className = `mono text-xs mt-1 mb-1 alert ${isError ? "alert-danger" : "alert-success"}`;
          banner.textContent = msg;
          setTimeout(() => {
            if (banner.textContent === msg) banner.style.display = "none";
          }, 6000);
        }
      };

      const getTarget = () => {
        const sel = this.querySelector("#selTraineeTarget") as HTMLSelectElement;
        if (sel && sel.value) return sel.value;
        const inp = this.querySelector("#inpTraineeTarget") as HTMLInputElement;
        if (inp && inp.value) return inp.value;
        return store.activeProject?.control_loops[0]?.name || "LIC-101";
      };

      this.querySelector("#btnTraineeForbiddenFault")?.addEventListener("click", async () => {
        try {
          const res = await this.apiClient.recordTraineeAction(session.id, {
            session_id: session.id,
            actor: session.trainee_id,
            target_entity: "SYS-CORE",
            action_type: "INJECT_FAULT",
          });
          const statusObj = res.status as any;
          if (statusObj && statusObj.Rejected) {
            showTraineeFeedback(`🛡️ Action Rejected by Server RBAC: ${statusObj.Rejected.reason}`, true);
          } else {
            showTraineeFeedback(`Action status: ${JSON.stringify(res.status)}`, true);
          }
        } catch (err: any) {
          showTraineeFeedback(`🛡️ Request rejected by server security: ${err.message || err}`, true);
        }
      });

      this.querySelector("#btnTraineeSetSp")?.addEventListener("click", async () => {
        const sp = parseFloat((this.querySelector("#inpTraineeSp") as HTMLInputElement)?.value || "50.0");
        const target = getTarget();
        try {
          await this.apiClient.recordTraineeAction(session.id, {
            session_id: session.id,
            actor: session.trainee_id,
            target_entity: target,
            action_type: "SET_SP",
            requested_value: sp,
          });
          showTraineeFeedback(`✓ SP adjusted to ${sp.toFixed(1)} for ${target}`);
        } catch (err: any) {
          showTraineeFeedback(`Failed to set SP: ${err.message || err}`, true);
        }
      });

      this.querySelector("#btnModeMan")?.addEventListener("click", async () => {
        const target = getTarget();
        try {
          await this.apiClient.recordTraineeAction(session.id, {
            session_id: session.id,
            actor: session.trainee_id,
            target_entity: target,
            action_type: "SET_MODE",
            command_payload: { mode: "MAN" },
          });
          showTraineeFeedback(`✓ Mode switched to MANUAL for ${target}`);
        } catch (err: any) {
          showTraineeFeedback(`Failed to set mode: ${err.message || err}`, true);
        }
      });

      this.querySelector("#btnModeAuto")?.addEventListener("click", async () => {
        const target = getTarget();
        try {
          await this.apiClient.recordTraineeAction(session.id, {
            session_id: session.id,
            actor: session.trainee_id,
            target_entity: target,
            action_type: "SET_MODE",
            command_payload: { mode: "AUTO" },
          });
          showTraineeFeedback(`✓ Mode switched to AUTO for ${target}`);
        } catch (err: any) {
          showTraineeFeedback(`Failed to set mode: ${err.message || err}`, true);
        }
      });

      this.querySelector("#btnModeCas")?.addEventListener("click", async () => {
        const target = getTarget();
        try {
          await this.apiClient.recordTraineeAction(session.id, {
            session_id: session.id,
            actor: session.trainee_id,
            target_entity: target,
            action_type: "SET_MODE",
            command_payload: { mode: "CAS" },
          });
          showTraineeFeedback(`✓ Mode switched to CASCADE for ${target}`);
        } catch (err: any) {
          showTraineeFeedback(`Failed to set mode: ${err.message || err}`, true);
        }
      });

      this.querySelector("#btnTraineeSetOut")?.addEventListener("click", async () => {
        const out = parseFloat((this.querySelector("#inpTraineeOut") as HTMLInputElement)?.value || "0.0");
        const target = getTarget();
        try {
          await this.apiClient.recordTraineeAction(session.id, {
            session_id: session.id,
            actor: session.trainee_id,
            target_entity: target,
            action_type: "SET_OUTPUT",
            requested_value: out,
          });
          showTraineeFeedback(`✓ Output adjusted to ${out.toFixed(1)}% for ${target}`);
        } catch (err: any) {
          showTraineeFeedback(`Failed to set output: ${err.message || err}`, true);
        }
      });

      this.querySelector("#btnTraineeAckAlarm")?.addEventListener("click", async () => {
        const target = getTarget();
        try {
          await this.apiClient.recordTraineeAction(session.id, {
            session_id: session.id,
            actor: session.trainee_id,
            target_entity: target,
            action_type: "ACK_ALARM",
          });
          showTraineeFeedback(`✓ Alarm acknowledged by trainee`);
        } catch (err: any) {
          showTraineeFeedback(`Failed to ack alarm: ${err.message || err}`, true);
        }
      });

      this.querySelector("#btnTraineeResetTrip")?.addEventListener("click", async () => {
        const target = getTarget();
        try {
          await this.apiClient.recordTraineeAction(session.id, {
            session_id: session.id,
            actor: session.trainee_id,
            target_entity: target,
            action_type: "RESET_TRIP",
          });
          showTraineeFeedback(`✓ Safety trip reset commanded by trainee`);
        } catch (err: any) {
          showTraineeFeedback(`Failed to reset trip: ${err.message || err}`, true);
        }
      });
    }

    // Replay Step Back (-1s)
    this.querySelector("#btnReplayStepBack")?.addEventListener("click", () => {
      if (state.replaySummary) {
        const newTime = Math.max(state.replaySummary.start_sim_time_sec, state.replaySimTime - 1.0);
        this.store.seekReplay(state.replaySummary.session_id, newTime);
      }
    });

    // Replay Step Forward (+1s)
    this.querySelector("#btnReplayStepFwd")?.addEventListener("click", () => {
      if (state.replaySummary) {
        const newTime = Math.min(state.replaySummary.end_sim_time_sec, state.replaySimTime + 1.0);
        this.store.seekReplay(state.replaySummary.session_id, newTime);
      }
    });

    // Replay Play / Pause
    this.querySelector("#btnReplayPlayPause")?.addEventListener("click", () => {
      if (!state.replaySummary) return;

      if (this.replayPlaying) {
        if (this.replayTimer) {
          clearInterval(this.replayTimer);
          this.replayTimer = null;
        }
        this.replayPlaying = false;
        const btn = this.querySelector("#btnReplayPlayPause");
        if (btn) btn.textContent = "▶ Play";
      } else {
        this.replayPlaying = true;
        const btn = this.querySelector("#btnReplayPlayPause");
        if (btn) btn.textContent = "⏸ Pause";

        const sessId = state.replaySummary.session_id;
        const endTime = state.replaySummary.end_sim_time_sec;

        this.replayTimer = window.setInterval(() => {
          const current = this.store.getState().replaySimTime;
          if (current >= endTime) {
            if (this.replayTimer) clearInterval(this.replayTimer);
            this.replayTimer = null;
            this.replayPlaying = false;
            const b = this.querySelector("#btnReplayPlayPause");
            if (b) b.textContent = "▶ Play";
            return;
          }
          const nextTime = Math.min(endTime, current + 1.0);
          this.store.seekReplay(sessId, nextTime);
        }, 1000);
      }
    });

    // Replay Timeline Scrubber
    const scrubber = this.querySelector("#replayScrubber") as HTMLInputElement;
    if (scrubber && state.replaySummary) {
      scrubber.addEventListener("input", (e) => {
        const simTime = parseFloat((e.target as HTMLInputElement).value);
        this.store.seekReplay(state.replaySummary!.session_id, simTime);
      });
    }

    this.querySelector("#btnLoadReplayFromEval")?.addEventListener("click", (e) => {
      const sessId = (e.currentTarget as HTMLElement)?.getAttribute("data-session");
      if (sessId) {
        this.store.loadReplaySummary(sessId);
      }
    });
  }
}

customElements.define("nebu-training-studio", NebuTrainingStudio);
