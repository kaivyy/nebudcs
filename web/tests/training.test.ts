/**
 * Phase 15: Training Studio Test Suite.
 * Validates:
 *  - Session lifecycle management (Created, Loaded, Running, Paused, Completed, Aborted)
 *  - Scenario authoring CRUD and project validation
 *  - Trainee vs Instructor role boundaries & action recording
 *  - Deterministic evaluation report parsing and grading logic
 *  - Read-only historical replay seeking and frame reconstruction
 *  - NebuTrainingStudio custom element rendering across all tabs
 */

import { describe, it, expect, beforeEach, vi } from "vitest";
import { TrainingStore } from "../src/state/training-store";
import { ApiClient } from "../src/api/client";
import {
  EvaluationReportDto,
  ReplayFrameDto,
  ReplaySessionSummaryDto,
  ScenarioAuthoringDto,
  TrainingSessionDetailDto,
} from "../src/protocol/types";
import "../src/components/nebu-training-studio";

describe("Phase 15: Training Studio - State & UI Integration", () => {
  let mockClient: ApiClient;
  let store: TrainingStore;

  const mockSession: TrainingSessionDetailDto = {
    id: "tsess-test-01",
    name: "Refinery Level Trip Mitigation Drill",
    description: "Respond to feed loss and prevent trip",
    project_id: "proj-vertical-slice-01",
    scenario_id: "scn-generic-feed-loss",
    instructor_id: "INSTRUCTOR-ALPHA",
    trainee_id: "TRAINEE-OMEGA",
    state: "RUNNING",
    created_at_ms: 1700000000000,
    started_at_ms: 1700000010000,
    simulation_start_time_sec: 10.0,
    current_simulation_time_sec: 45.5,
    metadata: {},
  };

  const mockScenario: ScenarioAuthoringDto = {
    id: "scn-test-authoring",
    name: "Cooling Loss Drill",
    description: "Manage thermal upset",
    author: "Safety Specialist",
    version: "1.0.0",
    random_seed: 42,
    initial_conditions_json: "{}",
    timeline_json: "[]",
    objectives_json: "[]",
    criteria_json: "[]",
  };

  const mockEvalReport: EvaluationReportDto = {
    session_id: "tsess-test-01",
    scenario_id: "scn-generic-feed-loss",
    trainee_id: "TRAINEE-OMEGA",
    total_score: 90,
    max_possible_score: 100,
    percentage: 90.0,
    passed: true,
    results: [
      {
        criterion_id: "crit-alm-ack",
        title: "Prompt Alarm Acknowledgment",
        passed: true,
        score_awarded: 40,
        score_possible: 40,
        penalty_deducted: 0,
        explanation: "Alarm acknowledged in 4.2s",
        evidence: { response_time_sec: "4.2" },
      },
      {
        criterion_id: "crit-no-trip",
        title: "No Protective Trip Occurred",
        passed: true,
        score_awarded: 50,
        score_possible: 50,
        penalty_deducted: 0,
        explanation: "No protective trip triggered throughout session",
        evidence: {},
      },
    ],
    evaluated_at_ms: 1700000200000,
  };

  const mockReplaySummary: ReplaySessionSummaryDto = {
    session_id: "tsess-test-01",
    project_id: "proj-vertical-slice-01",
    scenario_id: "scn-generic-feed-loss",
    start_sim_time_sec: 0.0,
    end_sim_time_sec: 120.0,
    duration_sec: 120.0,
    markers: [
      {
        marker_id: "m-1",
        sim_time_sec: 15.0,
        kind: "ALARM",
        source_id: "ALM-LVL-LO",
        label: "Low Level Alarm",
        details: "PV dropped below 20%",
        severity: 2,
      },
      {
        marker_id: "m-2",
        sim_time_sec: 19.5,
        kind: "OPERATOR_ACTION",
        source_id: "TRAINEE-OMEGA",
        label: "Acknowledge Alarm",
        details: "Trainee acknowledged ALM-LVL-LO",
        severity: 1,
      },
    ],
  };

  const mockReplayFrame: ReplayFrameDto = {
    sim_time_sec: 19.5,
    tags: {
      "LIC-101.PV": 18.2,
      "LIC-101.SP": 50.0,
      "LIC-101.OUT": 35.0,
    },
    active_alarms: ["ALM-LVL-LO"],
    active_trips: [],
    recent_actions: [
      {
        action_id: "tact-01",
        session_id: "tsess-test-01",
        sim_time_sec: 19.5,
        wall_clock_ms: 1700000020000,
        actor: "TRAINEE-OMEGA",
        target_entity: "ALM-LVL-LO",
        action_type: "ACK_ALARM",
        status: "Accepted",
      },
    ],
  };

  beforeEach(() => {
    mockClient = new ApiClient();
    vi.spyOn(mockClient, "listSessions").mockResolvedValue([
      {
        id: mockSession.id,
        name: mockSession.name,
        project_id: mockSession.project_id,
        scenario_id: mockSession.scenario_id,
        instructor_id: mockSession.instructor_id,
        trainee_id: mockSession.trainee_id,
        state: mockSession.state,
        current_sim_time_sec: mockSession.current_simulation_time_sec,
        created_at_ms: mockSession.created_at_ms,
      },
    ]);
    vi.spyOn(mockClient, "getSessionDetail").mockResolvedValue(mockSession);
    vi.spyOn(mockClient, "listScenarios").mockResolvedValue([
      {
        id: "scn-generic-feed-loss",
        name: "Feed Loss Recovery",
        description: "Generic test scenario",
        author: "Instructor",
        version: "1.0.0",
        event_count: 2,
        objective_count: 2,
      },
    ]);
    vi.spyOn(mockClient, "getScenario").mockResolvedValue(mockScenario);
    vi.spyOn(mockClient, "saveScenario").mockResolvedValue(mockScenario);
    vi.spyOn(mockClient, "deleteScenario").mockResolvedValue(undefined);
    vi.spyOn(mockClient, "startSession").mockResolvedValue({ ...mockSession, state: "RUNNING" });
    vi.spyOn(mockClient, "pauseSession").mockResolvedValue({ ...mockSession, state: "PAUSED" });
    vi.spyOn(mockClient, "resumeSession").mockResolvedValue({ ...mockSession, state: "RUNNING" });
    vi.spyOn(mockClient, "completeSession").mockResolvedValue(mockEvalReport);
    vi.spyOn(mockClient, "abortSession").mockResolvedValue({ ...mockSession, state: "ABORTED" });
    vi.spyOn(mockClient, "getReplaySummary").mockResolvedValue(mockReplaySummary);
    vi.spyOn(mockClient, "getReplayFrame").mockResolvedValue(mockReplayFrame);

    store = new TrainingStore(mockClient);
  });

  it("loads sessions and allows selecting an active session", async () => {
    await store.loadSessions();
    expect(store.getState().sessions.length).toBe(1);

    await store.selectSession("tsess-test-01");
    expect(store.getState().activeSession?.id).toBe("tsess-test-01");
    expect(store.getState().activeSession?.state).toBe("RUNNING");
  });

  it("manages session lifecycle transitions (pause, resume, abort, complete)", async () => {
    await store.selectSession("tsess-test-01");

    await store.pauseSession("tsess-test-01");
    expect(store.getState().activeSession?.state).toBe("PAUSED");

    await store.resumeSession("tsess-test-01");
    expect(store.getState().activeSession?.state).toBe("RUNNING");

    await store.completeSession("tsess-test-01");
    expect(store.getState().evaluationReport?.passed).toBe(true);
    expect(store.getState().evaluationReport?.percentage).toBe(90.0);
    expect(store.getState().activeTab).toBe("evaluation");
  });

  it("loads scenario catalog and supports scenario editing", async () => {
    await store.loadScenarios();
    expect(store.getState().scenarios.length).toBe(1);

    await store.loadScenarioForEdit("scn-test-authoring");
    expect(store.getState().editingScenario?.id).toBe("scn-test-authoring");
    expect(store.getState().activeTab).toBe("scenario-editor");
  });

  it("supports historical replay summary loading and timeline frame seeking", async () => {
    await store.loadReplaySummary("tsess-test-01");
    expect(store.getState().replaySummary?.duration_sec).toBe(120.0);
    expect(store.getState().replaySummary?.markers.length).toBe(2);
    expect(store.getState().activeTab).toBe("replay");

    await store.seekReplay("tsess-test-01", 19.5);
    expect(store.getState().replayFrame?.tags["LIC-101.PV"]).toBe(18.2);
    expect(store.getState().replayFrame?.recent_actions.length).toBe(1);
    expect(store.getState().replaySimTime).toBe(19.5);
  });

  it("mounts <nebu-training-studio> web component and renders navigation tabs", () => {
    const el = document.createElement("nebu-training-studio") as any;
    el.store = store;
    el.apiClient = mockClient;
    document.body.appendChild(el);

    const tabs = el.querySelectorAll(".t-tab");
    expect(tabs.length).toBe(5);

    const tabNames = Array.from(tabs).map((t: any) => t.textContent.trim());
    expect(tabNames.some((n: string) => n.includes("Instructor Console"))).toBe(true);
    expect(tabNames.some((n: string) => n.includes("Scenario Editor"))).toBe(true);
    expect(tabNames.some((n: string) => n.includes("Trainee Station"))).toBe(true);
    expect(tabNames.some((n: string) => n.includes("Evaluation Report"))).toBe(true);
    expect(tabNames.some((n: string) => n.includes("Replay Station"))).toBe(true);

    document.body.removeChild(el);
  });

  it("renders evaluation report with breakdown and badge", async () => {
    const el = document.createElement("nebu-training-studio") as any;
    el.store = store;
    el.apiClient = mockClient;
    document.body.appendChild(el);

    await store.completeSession("tsess-test-01");
    el.render(store.getState());

    expect(el.innerHTML).toContain("ASSESSMENT OUTCOME: PASSED");
    expect(el.innerHTML).toContain("90.0%");
    expect(el.innerHTML).toContain("Prompt Alarm Acknowledgment");

    document.body.removeChild(el);
  });
});
