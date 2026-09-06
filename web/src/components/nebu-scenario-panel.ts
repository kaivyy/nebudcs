import { store, Store } from "../state/store";
import { ApiClient } from "../api/client";

export class NebuScenarioPanel extends HTMLElement {
  private unsubscribe?: () => void;
  public apiClient?: ApiClient;

  private stateBadge!: HTMLElement;
  private scenarioNameElem!: HTMLElement;
  private elapsedElem!: HTMLElement;
  private scoreElem!: HTMLElement;
  private faultsTableBody!: HTMLElement;

  connectedCallback() {
    this.innerHTML = `
      <div class="scenario-panel-card card">
        <div class="panel-header">
          <div class="title-group">
            <span class="panel-title">SCENARIO & FAULT INJECTION ENGINE</span>
            <span class="badge badge-info state-badge" id="scnStateBadge">IDLE</span>
          </div>
          <div class="scenario-meta">
            <span id="scnName" class="text-dim">No Scenario Loaded</span>
            <span class="separator">|</span>
            <span>Elapsed: <strong id="scnElapsed">0.0s</strong></span>
            <span class="separator">|</span>
            <span>Score: <strong id="scnScore">100.0</strong></span>
          </div>
        </div>

        <div class="scenario-actions">
          <button id="loadDemoScnBtn" class="btn btn-sm btn-secondary">Load Demo</button>
          <button id="startScnBtn" class="btn btn-sm btn-primary">Start</button>
          <button id="pauseScnBtn" class="btn btn-sm btn-secondary">Pause</button>
          <button id="resumeScnBtn" class="btn btn-sm btn-secondary">Resume</button>
          <button id="abortScnBtn" class="btn btn-sm btn-danger">Abort</button>
          <button id="resetScnBtn" class="btn btn-sm btn-secondary">Reset</button>
        </div>

        <div class="fault-injection-section">
          <div class="section-title">QUICK FAULT INJECTION</div>
          <div class="fault-buttons">
            <button id="fltBiasBtn" class="btn btn-sm btn-outline-warning" title="Add +15 bar offset to PI-101">
              Sensor Bias (+15 bar)
            </button>
            <button id="fltStuckBtn" class="btn btn-sm btn-outline-warning" title="Freeze valve FV-101 at 25%">
              Actuator Stuck (25%)
            </button>
            <button id="fltLossBtn" class="btn btn-sm btn-outline-danger" title="Force TagQuality::Bad on FT-101.FLOW">
              Signal Loss (FT-101)
            </button>
            <button id="fltTripBtn" class="btn btn-sm btn-outline-danger" title="Trigger mechanical overspeed trip on K-101">
              Trip Compressor (K-101)
            </button>
          </div>
        </div>

        <div class="active-faults-wrapper">
          <div class="section-title">ACTIVE FAULTS (<span id="activeFaultsCount">0</span>)</div>
          <table class="faults-table table table-sm">
            <thead>
              <tr>
                <th>FAULT ID</th>
                <th>TARGET</th>
                <th>CATEGORY</th>
                <th>ELAPSED</th>
                <th>ACTION</th>
              </tr>
            </thead>
            <tbody id="faultsTbody">
              <tr class="empty-row"><td colspan="5" class="text-dim">No active faults</td></tr>
            </tbody>
          </table>
        </div>
      </div>
    `;

    this.stateBadge = this.querySelector("#scnStateBadge")!;
    this.scenarioNameElem = this.querySelector("#scnName")!;
    this.elapsedElem = this.querySelector("#scnElapsed")!;
    this.scoreElem = this.querySelector("#scnScore")!;
    this.faultsTableBody = this.querySelector("#faultsTbody")!;

    this.setupEventListeners();

    this.unsubscribe = store.subscribe((s) => this.render(s));
  }

  disconnectedCallback() {
    this.unsubscribe?.();
  }

  private setupEventListeners() {
    this.querySelector("#loadDemoScnBtn")?.addEventListener("click", async () => {
      if (!this.apiClient) return;
      const demoScenario = {
        id: "SCN-DEMO-01",
        name: "Feed Pressure Disturbance & Recovery",
        description: "Step disturbance in feed pressure with required operator response",
        author: "NebuDCS Instructor",
        schema_version: "1.0.0",
        duration_limit_sec: 180.0,
        random_seed: 42,
        initial_conditions: {
          tags: [{ tag: "PI-101", value: 32.5 }],
          controllers: [{ loop_name: "LIC-101.PV", mode: "AUTO", sp: 50.0 }],
          equipment: []
        },
        timeline: [
          {
            id: "EVT-DIST-01",
            priority: 1,
            trigger: { type: "time", elapsed_sec: 5.0 },
            action: {
              type: "apply_disturbance",
              id: "DST-P1",
              target_tag: "PI-101",
              disturbance_type: { waveform: "step", value: 10.0, relative: true },
              duration_sec: 30.0
            }
          }
        ],
        objectives: [
          {
            id: "OBJ-RANGE",
            description: "Maintain level between 40% and 60%",
            criteria: {
              type: "tag_within_range",
              tag: "LIC-101.PV",
              min_val: 40.0,
              max_val: 60.0,
              hold_duration_sec: 10.0
            },
            weight: 100,
            mandatory: true
          }
        ]
      };
      try {
        await this.apiClient.loadScenario(demoScenario);
        const status = await this.apiClient.getScenarioStatus();
        store.setScenarioStatus(status);
      } catch (e) {
        console.error("Failed to load scenario", e);
      }
    });

    this.querySelector("#startScnBtn")?.addEventListener("click", async () => {
      try {
        await this.apiClient?.startScenario();
        const status = await this.apiClient?.getScenarioStatus();
        if (status) store.setScenarioStatus(status);
      } catch (e) {
        console.error("Failed to start scenario", e);
      }
    });

    this.querySelector("#pauseScnBtn")?.addEventListener("click", async () => {
      try {
        await this.apiClient?.pauseScenario();
        const status = await this.apiClient?.getScenarioStatus();
        if (status) store.setScenarioStatus(status);
      } catch (e) {
        console.error("Failed to pause scenario", e);
      }
    });

    this.querySelector("#resumeScnBtn")?.addEventListener("click", async () => {
      try {
        await this.apiClient?.resumeScenario();
        const status = await this.apiClient?.getScenarioStatus();
        if (status) store.setScenarioStatus(status);
      } catch (e) {
        console.error("Failed to resume scenario", e);
      }
    });

    this.querySelector("#abortScnBtn")?.addEventListener("click", async () => {
      try {
        await this.apiClient?.abortScenario("Manual abort");
        const status = await this.apiClient?.getScenarioStatus();
        if (status) store.setScenarioStatus(status);
      } catch (e) {
        console.error("Failed to abort scenario", e);
      }
    });

    this.querySelector("#resetScnBtn")?.addEventListener("click", async () => {
      try {
        await this.apiClient?.resetScenario();
        const status = await this.apiClient?.getScenarioStatus();
        if (status) store.setScenarioStatus(status);
      } catch (e) {
        console.error("Failed to reset scenario", e);
      }
    });

    // Fault injection quick buttons
    this.querySelector("#fltBiasBtn")?.addEventListener("click", async () => {
      try {
        await this.apiClient?.injectFault({
          id: `FLT-BIAS-${Date.now().toString().slice(-4)}`,
          target: "PI-101",
          fault_type: { category: "sensor_bias", offset: 15.0 },
          duration_sec: 60.0
        });
        const status = await this.apiClient?.getScenarioStatus();
        if (status) store.setScenarioStatus(status);
      } catch (e) {
        console.error("Failed to inject bias fault", e);
      }
    });

    this.querySelector("#fltStuckBtn")?.addEventListener("click", async () => {
      try {
        await this.apiClient?.injectFault({
          id: `FLT-STUCK-${Date.now().toString().slice(-4)}`,
          target: "FV-101.POS",
          fault_type: { category: "actuator_stuck", position: 25.0 },
          duration_sec: 60.0
        });
        const status = await this.apiClient?.getScenarioStatus();
        if (status) store.setScenarioStatus(status);
      } catch (e) {
        console.error("Failed to inject stuck valve fault", e);
      }
    });

    this.querySelector("#fltLossBtn")?.addEventListener("click", async () => {
      try {
        await this.apiClient?.injectFault({
          id: `FLT-LOSS-${Date.now().toString().slice(-4)}`,
          target: "FT-101.FLOW",
          fault_type: { category: "sensor_loss_of_signal" },
          duration_sec: 45.0
        });
        const status = await this.apiClient?.getScenarioStatus();
        if (status) store.setScenarioStatus(status);
      } catch (e) {
        console.error("Failed to inject signal loss fault", e);
      }
    });

    this.querySelector("#fltTripBtn")?.addEventListener("click", async () => {
      try {
        await this.apiClient?.injectFault({
          id: `FLT-TRIP-${Date.now().toString().slice(-4)}`,
          target: "K-101.STATUS",
          fault_type: { category: "equipment_mechanical_trip" },
          duration_sec: null
        });
        const status = await this.apiClient?.getScenarioStatus();
        if (status) store.setScenarioStatus(status);
      } catch (e) {
        console.error("Failed to inject trip fault", e);
      }
    });
  }

  private render(s: Store) {
    const scn = s.scenarioStatus;
    if (!scn) return;

    this.stateBadge.textContent = scn.state;
    this.stateBadge.className = `badge state-badge badge-${this.badgeColorForState(scn.state)}`;
    this.scenarioNameElem.textContent = scn.scenario_name || (scn.scenario_id ? `ID: ${scn.scenario_id}` : "No Scenario Loaded");
    this.elapsedElem.textContent = `${(scn.elapsed_sec ?? 0).toFixed(1)}s`;
    this.scoreElem.textContent = `${(scn.score ?? 0).toFixed(1)}`;

    const countElem = this.querySelector("#activeFaultsCount");
    if (countElem) {
      countElem.textContent = `${scn.active_faults.length}`;
    }

    if (scn.active_faults.length === 0) {
      this.faultsTableBody.innerHTML = `
        <tr class="empty-row"><td colspan="5" class="text-dim">No active faults</td></tr>
      `;
    } else {
      this.faultsTableBody.innerHTML = scn.active_faults
        .map(
          (f) => `
          <tr data-fault-id="${f.id}">
            <td><strong>${f.id}</strong></td>
            <td><code>${f.target}</code></td>
            <td><span class="badge badge-warning">${f.category}</span></td>
            <td>${f.elapsed_sec.toFixed(1)}s ${f.duration_sec ? `/ ${f.duration_sec.toFixed(0)}s` : ""}</td>
            <td>
              <button class="btn btn-xs btn-outline-danger clear-fault-btn" data-id="${f.id}">
                CLEAR
              </button>
            </td>
          </tr>
        `
        )
        .join("");

      this.faultsTableBody.querySelectorAll(".clear-fault-btn").forEach((btn) => {
        btn.addEventListener("click", async (e) => {
          const faultId = (e.currentTarget as HTMLElement).getAttribute("data-id");
          if (faultId && this.apiClient) {
            try {
              await this.apiClient.clearFault(faultId);
              const status = await this.apiClient.getScenarioStatus();
              store.setScenarioStatus(status);
            } catch (err) {
              console.error("Failed to clear fault", err);
            }
          }
        });
      });
    }
  }

  private badgeColorForState(state: string): string {
    switch (state.toUpperCase()) {
      case "RUNNING":
        return "success";
      case "ARMED":
      case "LOADED":
        return "info";
      case "PAUSED":
        return "warning";
      case "COMPLETED":
        return "primary";
      case "ABORTED":
        return "danger";
      default:
        return "secondary";
    }
  }
}

customElements.define("nebu-scenario-panel", NebuScenarioPanel);
