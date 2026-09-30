/**
 * Phase 15: Training Studio state management.
 * Manages active training session, scenario catalog, evaluation, and replay state.
 */

import { ApiClient, sharedApiClient } from "../api/client";
import {
  CreateTrainingSessionRequest,
  EvaluationReportDto,
  LoadSessionProjectRequest,
  ProjectSummaryDto,
  ReplayFrameDto,
  ReplaySessionSummaryDto,
  ScenarioAuthoringDto,
  ScenarioBranchDto,
  ScenarioSummaryDto,
  TrainingDebriefDto,
  TrainingSessionDetailDto,
  TrainingSessionSummaryDto,
} from "../protocol/types";
import { store } from "./store";

export type TrainingStudioTab =
  | "instructor"
  | "scenario-editor"
  | "trainee"
  | "evaluation"
  | "replay";

export interface TrainingState {
  activeTab: TrainingStudioTab;
  sessions: TrainingSessionSummaryDto[];
  activeSession: TrainingSessionDetailDto | null;
  projects: ProjectSummaryDto[];
  scenarios: ScenarioSummaryDto[];
  editingScenario: ScenarioAuthoringDto | null;
  evaluationReport: EvaluationReportDto | null;
  debrief: TrainingDebriefDto | null;
  branches: ScenarioBranchDto[];
  replaySummary: ReplaySessionSummaryDto | null;
  replayFrame: ReplayFrameDto | null;
  replaySimTime: number;
  loading: boolean;
  error: string | null;
}

type TrainingListener = (state: TrainingState) => void;

export class TrainingStore {
  private state: TrainingState = {
    activeTab: "instructor",
    sessions: [],
    activeSession: null,
    projects: [],
    scenarios: [],
    editingScenario: null,
    evaluationReport: null,
    debrief: null,
    branches: [],
    replaySummary: null,
    replayFrame: null,
    replaySimTime: 0,
    loading: false,
    error: null,
  };

  private listeners: Set<TrainingListener> = new Set();

  constructor(private client: ApiClient = sharedApiClient) {}

  getState(): Readonly<TrainingState> {
    return this.state;
  }

  subscribe(listener: TrainingListener): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private notify() {
    this.listeners.forEach((l) => l(this.state));
  }

  private setLoading(loading: boolean) {
    this.state = { ...this.state, loading, error: loading ? null : this.state.error };
    this.notify();
  }

  private setError(error: string) {
    this.state = { ...this.state, loading: false, error };
    this.notify();
  }

  setActiveTab(tab: TrainingStudioTab) {
    this.state = { ...this.state, activeTab: tab };
    this.notify();
  }

  // ── Project Catalog ─────────────────────────────────────────────────────

  async loadProjects() {
    this.setLoading(true);
    try {
      const projects = await this.client.listProjects();
      this.state = { ...this.state, projects, loading: false };
      this.notify();
    } catch (e) {
      this.setError(`Failed to load projects: ${e}`);
    }
  }

  // ── Scenario Catalog ────────────────────────────────────────────────────

  async loadScenarios() {
    this.setLoading(true);
    try {
      const scenarios = await this.client.listScenarios();
      this.state = { ...this.state, scenarios, loading: false };
      this.notify();
    } catch (e) {
      this.setError(`Failed to load scenarios: ${e}`);
    }
  }

  async loadScenarioForEdit(id: string) {
    this.setLoading(true);
    try {
      const editing = await this.client.getScenario(id);
      this.state = { ...this.state, editingScenario: editing, loading: false, activeTab: "scenario-editor" };
      this.notify();
    } catch (e) {
      this.setError(`Failed to load scenario: ${e}`);
    }
  }

  async saveScenario(dto: ScenarioAuthoringDto) {
    this.setLoading(true);
    try {
      const saved = await this.client.saveScenario(dto);
      this.state = { ...this.state, editingScenario: saved, loading: false };
      await this.loadScenarios();
    } catch (e) {
      this.setError(`Failed to save scenario: ${e}`);
    }
  }

  async deleteScenario(id: string) {
    this.setLoading(true);
    try {
      await this.client.deleteScenario(id);
      await this.loadScenarios();
    } catch (e) {
      this.setError(`Failed to delete scenario: ${e}`);
    }
  }

  setEditingScenario(dto: ScenarioAuthoringDto | null) {
    this.state = { ...this.state, editingScenario: dto, activeTab: dto ? "scenario-editor" : this.state.activeTab };
    this.notify();
  }

  // ── Session Lifecycle ────────────────────────────────────────────────────

  async loadSessions() {
    this.setLoading(true);
    try {
      const sessions = await this.client.listSessions();
      let activeSession = this.state.activeSession;
      if (activeSession) {
        const found = sessions.find((s) => s.id === activeSession!.id);
        if (found) {
          activeSession = {
            ...activeSession,
            state: found.state,
            current_simulation_time_sec: found.current_sim_time_sec,
          };
        }
      } else if (sessions.length > 0) {
        const running = sessions.find((s) => s.state === "RUNNING");
        const candidate = running || sessions[0];
        try {
          activeSession = await this.client.getSessionDetail(candidate.id);
        } catch {
          // ignore error
        }
      }
      this.state = { ...this.state, sessions, activeSession, loading: false };
      this.notify();
    } catch (e) {
      this.setError(`Failed to load sessions: ${e}`);
    }
  }

  async createSession(req: CreateTrainingSessionRequest) {
    this.setLoading(true);
    try {
      const detail = await this.client.createSession(req);
      this.state = { ...this.state, activeSession: detail, loading: false };
      await this.loadSessions();
    } catch (e) {
      this.setError(`Failed to create session: ${e}`);
    }
  }

  async selectSession(id: string) {
    this.setLoading(true);
    try {
      const detail = await this.client.getSessionDetail(id);
      this.state = {
        ...this.state,
        activeSession: detail,
        evaluationReport: null,
        replaySummary: null,
        replayFrame: null,
        loading: false,
      };
      this.notify();
    } catch (e) {
      this.setError(`Failed to load session: ${e}`);
    }
  }

  async loadSessionProject(sessionId: string, req: LoadSessionProjectRequest) {
    this.setLoading(true);
    try {
      const detail = await this.client.loadSessionProject(sessionId, req);
      this.state = { ...this.state, activeSession: detail, loading: false };
      try {
        const proj = await this.client.getProject(req.project_id);
        store.setActiveProject(proj);
      } catch (err) {
        console.warn("[training-store] Could not sync project to store:", err);
      }
      this.notify();
    } catch (e) {
      this.setError(`Failed to load project into session: ${e}`);
    }
  }

  async prepareSession(sessionId: string) {
    this.setLoading(true);
    try {
      const detail = await this.client.prepareSession(sessionId);
      this.state = { ...this.state, activeSession: detail, loading: false };
      await this.loadSessions();
      await this.loadBranches(sessionId);
    } catch (e) {
      this.setError(`Failed to prepare session: ${e}`);
    }
  }

  async startSession(sessionId: string) {
    this.setLoading(true);
    try {
      const detail = await this.client.startSession(sessionId);
      this.state = { ...this.state, activeSession: detail, loading: false };
      await this.loadBranches(sessionId);
      this.notify();
    } catch (e) {
      this.setError(`Failed to start session: ${e}`);
    }
  }

  async loadBranches(sessionId: string) {
    try {
      const branches = await this.client.getBranches(sessionId);
      this.state = { ...this.state, branches };
      this.notify();
    } catch (e) {
      console.warn("[training-store] Failed to load branches:", e);
    }
  }

  async triggerBranch(sessionId: string, branchId: string) {
    this.setLoading(true);
    try {
      await this.client.triggerBranch(sessionId, branchId);
      await this.loadBranches(sessionId);
      this.setLoading(false);
    } catch (e) {
      this.setError(`Failed to trigger branch: ${e}`);
    }
  }

  async loadDebrief(sessionId: string) {
    try {
      const debrief = await this.client.getDebrief(sessionId);
      this.state = { ...this.state, debrief };
      this.notify();
    } catch (e) {
      console.warn("[training-store] Failed to load debrief:", e);
    }
  }

  async pauseSession(sessionId: string) {
    this.setLoading(true);
    try {
      const detail = await this.client.pauseSession(sessionId);
      this.state = { ...this.state, activeSession: detail, loading: false };
      this.notify();
    } catch (e) {
      this.setError(`Failed to pause session: ${e}`);
    }
  }

  async resumeSession(sessionId: string) {
    this.setLoading(true);
    try {
      const detail = await this.client.resumeSession(sessionId);
      this.state = { ...this.state, activeSession: detail, loading: false };
      this.notify();
    } catch (e) {
      this.setError(`Failed to resume session: ${e}`);
    }
  }

  async resetSession(sessionId: string) {
    this.setLoading(true);
    try {
      const detail = await this.client.resetSession(sessionId);
      this.state = { ...this.state, activeSession: detail, loading: false };
      this.notify();
    } catch (e) {
      this.setError(`Failed to reset session: ${e}`);
    }
  }

  async completeSession(sessionId: string) {
    this.setLoading(true);
    try {
      const report = await this.client.completeSession(sessionId);
      this.state = {
        ...this.state,
        evaluationReport: report,
        debrief: report.debrief || null,
        loading: false,
        activeTab: "evaluation",
      };
      await this.loadSessions();
      if (this.state.activeSession) {
        const updated = await this.client.getSessionDetail(sessionId);
        this.state = { ...this.state, activeSession: updated };
        this.notify();
      }
    } catch (e) {
      this.setError(`Failed to complete session: ${e}`);
    }
  }

  async abortSession(sessionId: string, reason: string) {
    this.setLoading(true);
    try {
      const detail = await this.client.abortSession(sessionId, { reason });
      this.state = { ...this.state, activeSession: detail, loading: false };
      await this.loadSessions();
    } catch (e) {
      this.setError(`Failed to abort session: ${e}`);
    }
  }

  // ── Replay ───────────────────────────────────────────────────────────────

  async loadReplaySummary(sessionId: string) {
    this.setLoading(true);
    try {
      const summary = await this.client.getReplaySummary(sessionId);
      this.state = {
        ...this.state,
        replaySummary: summary,
        replaySimTime: summary.start_sim_time_sec,
        loading: false,
        activeTab: "replay",
      };
      this.notify();
      // Auto-load initial frame so UI immediately shows reconstructed state
      await this.seekReplay(sessionId, summary.start_sim_time_sec);
    } catch (e) {
      this.setError(`Failed to load replay: ${e}`);
    }
  }

  async seekReplay(sessionId: string, simTime: number) {
    try {
      const frame = await this.client.getReplayFrame(sessionId, simTime);
      this.state = { ...this.state, replayFrame: frame, replaySimTime: simTime };
      this.notify();
    } catch (e) {
      this.setError(`Failed to seek replay frame: ${e}`);
    }
  }
}

export const trainingStore = new TrainingStore(sharedApiClient);

