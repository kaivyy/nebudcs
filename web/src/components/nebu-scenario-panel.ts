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
          <div class="fault-form mono" id="faultForm">
            <div class="fault-field">
              <label>TARGET:</label>
              <select id="faultTargetSelect">
                <option value="">-- Load project --</option>
              </select>
            </div>
            <div class="fault-field">
              <label>FAULT TYPE:</label>
              <select id="faultTypeSelect">
                <option value="sensor_bias">Sensor Bias</option>
                <option value="sensor_loss_of_signal">Signal Loss</option>
                <option value="actuator_stuck">Actuator Stuck</option>
                <option value="equipment_mechanical_trip">Mechanical Trip</option>
              </select>
            </div>
            <div class="fault-field" id="biasValueField">
              <label>BIAS:</label>
              <input type="number" id="biasValueInput" value="10" style="width:55px;"/>
            </div>
            <div class="fault-field" id="stuckPosField" style="display:none">
              <label>POS (%):</label>
              <input type="number" id="stuckPosInput" value="25" min="0" max="100" style="width:55px;"/>
            </div>
            <div class="fault-field">
              <label>DUR (s):</label>
              <input type="number" id="faultDurationInput" value="60" min="1" style="width:55px;"/>
            </div>
            <button id="injectFaultBtn" class="btn btn-sm btn-outline-warning">INJECT</button>
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

      // Build a generic demo scenario from the active project model
      const proj = store.activeProject;
      const firstLoop = proj?.control_loops[0];
      const firstInstrument = proj?.instruments[0];

      // Initial conditions: set all control loops to AUTO at their configured SP
      const controllers = proj
        ? proj.control_loops.map((cl) => ({ loop_name: cl.pv_tag, mode: "AUTO", sp: cl.sp }))
        : [];

      // Disturbance target: first instrument (if any), else first loop PV
      const disturbanceTag = firstInstrument?.tag ?? firstLoop?.pv_tag ?? "SIM.TIME";
      // Objective: keep first loop PV near its SP
      const objectiveTag = firstLoop?.pv_tag ?? disturbanceTag;
      const objectiveSp = firstLoop?.sp ?? 50;

      const demoScenario = {
        id: `SCN-DEMO-${Date.now().toString().slice(-4)}`,
        name: `Disturbance & Recovery — ${proj?.name ?? "Demo Plant"}`,
        description: "Step disturbance with required operator response. Targets derived from active project.",
        author: "NebuDCS Instructor",
        schema_version: "1.0.0",
        duration_limit_sec: 180.0,
        random_seed: 42,
        initial_conditions: {
          tags: [],
          controllers,
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
              target_tag: disturbanceTag,
              disturbance_type: { waveform: "step", value: 10.0, relative: true },
              duration_sec: 30.0
            }
          }
        ],
        objectives: firstLoop ? [
          {
            id: "OBJ-RANGE",
            description: `Maintain ${objectiveTag} within ±10% of SP`,
            criteria: {
              type: "tag_within_range",
              tag: objectiveTag,
              min_val: objectiveSp * 0.9,
              max_val: objectiveSp * 1.1,
              hold_duration_sec: 10.0
            },
            weight: 100,
            mandatory: true
          }
        ] : []
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

    // Generic fault injection — targets derived from active project model
    this.querySelector("#faultTypeSelect")?.addEventListener("change", () => {
      const ft = (this.querySelector("#faultTypeSelect") as HTMLSelectElement).value;
      const biasField = this.querySelector("#biasValueField") as HTMLElement | null;
      const stuckField = this.querySelector("#stuckPosField") as HTMLElement | null;
      if (biasField) biasField.style.display = ft === "sensor_bias" ? "" : "none";
      if (stuckField) stuckField.style.display = ft === "actuator_stuck" ? "" : "none";
    });

    this.querySelector("#injectFaultBtn")?.addEventListener("click", async () => {
      const target = (this.querySelector("#faultTargetSelect") as HTMLSelectElement)?.value;
      const faultType = (this.querySelector("#faultTypeSelect") as HTMLSelectElement)?.value;
      const duration = parseFloat((this.querySelector("#faultDurationInput") as HTMLInputElement)?.value) || 60;
      const bias = parseFloat((this.querySelector("#biasValueInput") as HTMLInputElement)?.value) || 10;
      const stuckPos = parseFloat((this.querySelector("#stuckPosInput") as HTMLInputElement)?.value) || 25;
      if (!target || !this.apiClient) return;

      let fault_type: Record<string, unknown>;
      switch (faultType) {
        case "sensor_bias":
          fault_type = { category: "sensor_bias", offset: bias };
          break;
        case "actuator_stuck":
          fault_type = { category: "actuator_stuck", position: stuckPos };
          break;
        case "sensor_loss_of_signal":
          fault_type = { category: "sensor_loss_of_signal" };
          break;
        default:
          fault_type = { category: "equipment_mechanical_trip" };
          break;
      }

      try {
        await this.apiClient.injectFault({
          id: `FLT-${Date.now().toString().slice(-6)}`,
          target,
          fault_type,
          duration_sec: faultType === "equipment_mechanical_trip" ? null : duration
        });
        const status = await this.apiClient.getScenarioStatus();
        if (status) store.setScenarioStatus(status);
      } catch (e) {
        console.error("Failed to inject fault", e);
      }
    });
  }

  private _lastProject: unknown = null;

  private render(s: Store) {
    // Rebuild fault targets when project changes
    if (s.activeProject !== this._lastProject) {
      this._lastProject = s.activeProject;
      this.rebuildFaultTargets();
    }

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

  /**
   * Builds a list of fault injection targets from the active project model.
   * Instruments → sensor faults; control loop outputs → actuator faults;
   * equipment → mechanical trip faults.
   */
  private buildFaultTargets(): Array<{ value: string; label: string }> {
    const proj = store.activeProject;
    if (!proj) return [];

    const targets: Array<{ value: string; label: string }> = [];

    // Instruments → sensor bias / signal loss
    for (const inst of proj.instruments) {
      targets.push({
        value: inst.tag,
        label: `${inst.tag} (${inst.measurement_kind})`,
      });
    }

    // Control loop output tags → actuator stuck
    for (const cl of proj.control_loops) {
      targets.push({
        value: cl.out_tag,
        label: `${cl.out_tag} (actuator)`,
      });
    }

    // Equipment → mechanical trip
    for (const eq of proj.equipment) {
      targets.push({
        value: `${eq.name}.STATUS`,
        label: `${eq.name} (equipment trip)`,
      });
    }

    return targets;
  }

  private rebuildFaultTargets() {
    const sel = this.querySelector("#faultTargetSelect") as HTMLSelectElement | null;
    if (!sel) return;
    const targets = this.buildFaultTargets();
    if (targets.length === 0) {
      sel.innerHTML = '<option value="">-- Load project to enable fault injection --</option>';
    } else {
      sel.innerHTML = targets
        .map((t) => `<option value="${t.value}">${t.label}</option>`)
        .join("");
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
