/**
 * IEC 61131-3 Logic Routine Editor & Online Monitor Web Component.
 *
 * Implements Stage 2 Control Engineering authoring interface:
 * - Language selection: Structured Text (ST), Ladder Diagram (LD), FBD, SFC
 * - Source editing with live syntax compilation & diagnostic reporting
 * - Symbol & Variable declaration inspection
 * - Online variable state monitoring
 * - Integration with Virtual Controllers and Task scheduling
 */

import { ApiClient, sharedApiClient } from "../api/client";
import { IecCompileResponseDto, IecVariableDto } from "../protocol/types";

const DEFAULT_ST_SAMPLE = `VAR
    PV_PRESSURE : REAL;
    SP_PRESSURE : REAL := 12.0;
    OUT_VALVE : REAL := 0.0;
    TRIP_HIGH : BOOL := FALSE;
END_VAR

// Read analog pressure input from %IW100
PV_PRESSURE := %IW100;

// High pressure relief control
IF PV_PRESSURE > SP_PRESSURE THEN
    OUT_VALVE := (PV_PRESSURE - SP_PRESSURE) * 10.0;
    IF OUT_VALVE > 100.0 THEN
        OUT_VALVE := 100.0;
    END_IF;
ELSE
    OUT_VALVE := 0.0;
END_IF;

// Safety trip evaluation
IF PV_PRESSURE >= 20.0 THEN
    TRIP_HIGH := TRUE;
ELSE
    TRIP_HIGH := FALSE;
END_IF;

// Command physical actuator via %QW100
%QW100 := OUT_VALVE;
`;

const DEFAULT_LD_SAMPLE = `{
  "rungs": [
    {
      "number": 0,
      "comment": "Motor Start/Stop Seal-in Circuit",
      "elements": [
        {
          "type": "PARALLEL",
          "paths": [
            [{ "type": "CONTACT", "kind": "NORMALLY_OPEN", "variable": "%IX0.0" }],
            [{ "type": "CONTACT", "kind": "NORMALLY_OPEN", "variable": "%QX0.0" }]
          ]
        },
        { "type": "CONTACT", "kind": "NORMALLY_CLOSED", "variable": "%IX0.1" }
      ],
      "coils": [
        { "kind": "OUTPUT", "variable": "%QX0.0" }
      ]
    }
  ],
  "variables": []
}`;

const DEFAULT_FBD_SAMPLE = `{
  "nodes": [
    { "id": "N1", "kind": { "KIND": "INPUT_VAR", "name": "%IW10" } },
    { "id": "N2", "kind": { "KIND": "CONSTANT", "value": { "REAL": 50.0 } } },
    { "id": "N3", "kind": { "KIND": "ARITHMETIC", "op": "SUB" } },
    { "id": "N4", "kind": { "KIND": "OUTPUT_VAR", "name": "%QW10" } }
  ],
  "connections": [
    { "from_node": "N1", "from_pin": "OUT", "to_node": "N3", "to_pin": "IN1" },
    { "from_node": "N2", "from_pin": "OUT", "to_node": "N3", "to_pin": "IN2" },
    { "from_node": "N3", "from_pin": "OUT", "to_node": "N4", "to_pin": "IN" }
  ],
  "variables": []
}`;

const DEFAULT_SFC_SAMPLE = `{
  "steps": [
    { "name": "STANDBY", "is_initial": true },
    { "name": "PURGE", "is_initial": false },
    { "name": "RUN", "is_initial": false }
  ],
  "transitions": [
    { "id": "T1", "from_step": "STANDBY", "to_step": "PURGE", "condition_expr": "%IX1.0", "priority": 0 },
    { "id": "T2", "from_step": "PURGE", "to_step": "RUN", "condition_expr": "TRUE", "priority": 0 }
  ],
  "actions": [
    { "step_name": "PURGE", "qualifier": "N", "target_var": "%QX1.0", "duration": null },
    { "step_name": "RUN", "qualifier": "N", "target_var": "%QX1.1", "duration": null }
  ],
  "variables": []
}`;

export class NebuIecEditor extends HTMLElement {
  public apiClient: ApiClient = sharedApiClient;
  private selectedLanguage: "ST" | "LD" | "FBD" | "SFC" = "ST";
  private sourceCode: string = DEFAULT_ST_SAMPLE;
  private isCompiling = false;
  private compileResult: IecCompileResponseDto | null = null;

  connectedCallback() {
    this.render();
  }

  private handleLanguageChange(lang: "ST" | "LD" | "FBD" | "SFC") {
    if (this.selectedLanguage === lang) return;
    this.selectedLanguage = lang;
    switch (lang) {
      case "ST":
        this.sourceCode = DEFAULT_ST_SAMPLE;
        break;
      case "LD":
        this.sourceCode = DEFAULT_LD_SAMPLE;
        break;
      case "FBD":
        this.sourceCode = DEFAULT_FBD_SAMPLE;
        break;
      case "SFC":
        this.sourceCode = DEFAULT_SFC_SAMPLE;
        break;
    }
    this.compileResult = null;
    this.render();
  }

  public async compile() {
    this.isCompiling = true;
    this.render();

    try {
      this.compileResult = await this.apiClient.compileIec({
        language: this.selectedLanguage,
        source_text: this.sourceCode,
      });
    } catch (err: any) {
      this.compileResult = {
        success: false,
        error_message: err?.message || "Failed to communicate with compiler service",
        symbol_count: 0,
        statement_count: 0,
        variables: [],
      };
    } finally {
      this.isCompiling = false;
      this.render();
    }
  }

  render() {
    const result = this.compileResult;
    const isSuccess = result?.success === true;
    const isError = result !== null && !result.success;

    this.innerHTML = `
      <div class="iec-editor-container" style="display: flex; flex-direction: column; height: 100%; background: #181b20; color: #e1e4e8; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;">
        <!-- Header Toolbar -->
        <div class="iec-toolbar" style="display: flex; align-items: center; justify-content: space-between; padding: 8px 16px; background: #20242b; border-bottom: 1px solid #2d333b;">
          <div style="display: flex; align-items: center; gap: 12px;">
            <span style="font-weight: 600; font-size: 13px; color: #58a6ff; letter-spacing: 0.5px;">IEC 61131-3 LOGIC WORKBENCH</span>
            <div class="btn-group" style="display: flex; background: #13161a; border-radius: 4px; padding: 2px; border: 1px solid #30363d;">
              <button class="lang-btn ${this.selectedLanguage === "ST" ? "active" : ""}" data-lang="ST" style="padding: 4px 10px; font-size: 12px; font-weight: 500; border: none; border-radius: 3px; cursor: pointer; background: ${this.selectedLanguage === "ST" ? "#1f6feb" : "transparent"}; color: ${this.selectedLanguage === "ST" ? "#ffffff" : "#8b949e"};">ST</button>
              <button class="lang-btn ${this.selectedLanguage === "LD" ? "active" : ""}" data-lang="LD" style="padding: 4px 10px; font-size: 12px; font-weight: 500; border: none; border-radius: 3px; cursor: pointer; background: ${this.selectedLanguage === "LD" ? "#1f6feb" : "transparent"}; color: ${this.selectedLanguage === "LD" ? "#ffffff" : "#8b949e"};">LD</button>
              <button class="lang-btn ${this.selectedLanguage === "FBD" ? "active" : ""}" data-lang="FBD" style="padding: 4px 10px; font-size: 12px; font-weight: 500; border: none; border-radius: 3px; cursor: pointer; background: ${this.selectedLanguage === "FBD" ? "#1f6feb" : "transparent"}; color: ${this.selectedLanguage === "FBD" ? "#ffffff" : "#8b949e"};">FBD</button>
              <button class="lang-btn ${this.selectedLanguage === "SFC" ? "active" : ""}" data-lang="SFC" style="padding: 4px 10px; font-size: 12px; font-weight: 500; border: none; border-radius: 3px; cursor: pointer; background: ${this.selectedLanguage === "SFC" ? "#1f6feb" : "transparent"}; color: ${this.selectedLanguage === "SFC" ? "#ffffff" : "#8b949e"};">SFC</button>
            </div>
          </div>
          <div style="display: flex; align-items: center; gap: 10px;">
            ${
              isSuccess
                ? `<span style="display: inline-flex; align-items: center; font-size: 12px; color: #3fb950; background: rgba(46,160,67,0.15); padding: 3px 8px; border-radius: 4px; border: 1px solid rgba(46,160,67,0.4);">
                     ✓ COMPILED (${result.statement_count} statements, ${result.symbol_count} symbols)
                   </span>`
                : ""
            }
            ${
              isError
                ? `<span style="display: inline-flex; align-items: center; font-size: 12px; color: #f85149; background: rgba(248,81,73,0.15); padding: 3px 8px; border-radius: 4px; border: 1px solid rgba(248,81,73,0.4);">
                     ✕ COMPILE ERROR
                   </span>`
                : ""
            }
            <button id="btn-compile" style="padding: 5px 14px; font-size: 12px; font-weight: 600; border: none; border-radius: 4px; background: #238636; color: #ffffff; cursor: pointer;" ${this.isCompiling ? "disabled" : ""}>
              ${this.isCompiling ? "Compiling..." : "▶ Compile & Validate"}
            </button>
          </div>
        </div>

        <!-- Main Body: Editor + Inspector -->
        <div style="display: flex; flex: 1; overflow: hidden;">
          <!-- Editor Area -->
          <div style="flex: 1; display: flex; flex-direction: column; border-right: 1px solid #2d333b;">
            <div style="padding: 6px 12px; background: #1c2128; font-size: 11px; color: #8b949e; border-bottom: 1px solid #2d333b; display: flex; justify-content: space-between;">
              <span>ROUTINE SOURCE (${this.selectedLanguage})</span>
              <span>1,000 Loop Iteration Safety Active</span>
            </div>
            <textarea id="iec-source-input" style="flex: 1; width: 100%; box-sizing: border-box; background: #0d1117; color: #c9d1d9; font-family: 'Fira Code', 'Cascadia Code', Consolas, monospace; font-size: 13px; line-height: 1.5; padding: 12px; border: none; outline: none; resize: none;">${this.sourceCode}</textarea>
            
            ${
              isError
                ? `<div style="padding: 10px 14px; background: #221215; border-top: 1px solid #8b1822; color: #ff7b72; font-family: monospace; font-size: 12px; max-height: 120px; overflow-y: auto;">
                     <strong>Error:</strong> ${result.error_message}
                   </div>`
                : ""
            }
          </div>

          <!-- Variable Inspector Panel -->
          <div style="width: 320px; display: flex; flex-direction: column; background: #161b22;">
            <div style="padding: 6px 12px; background: #1c2128; font-size: 11px; font-weight: 600; color: #8b949e; border-bottom: 1px solid #2d333b;">
              DECLARED SYMBOLS & MEMORY
            </div>
            <div style="flex: 1; overflow-y: auto; padding: 8px;">
              ${
                result && result.variables.length > 0
                  ? `<table style="width: 100%; border-collapse: collapse; font-size: 11px;">
                       <thead>
                         <tr style="border-bottom: 1px solid #30363d; color: #8b949e; text-align: left;">
                           <th style="padding: 4px;">Name</th>
                           <th style="padding: 4px;">Type</th>
                           <th style="padding: 4px;">Value</th>
                           <th style="padding: 4px;">Address</th>
                         </tr>
                       </thead>
                       <tbody>
                         ${result.variables
                           .map(
                             (v: IecVariableDto) => `
                           <tr style="border-bottom: 1px solid #21262d; font-family: monospace;">
                             <td style="padding: 4px; color: #58a6ff;">${v.name}</td>
                             <td style="padding: 4px; color: #f0883e;">${v.data_type}</td>
                             <td style="padding: 4px; color: #7ee787;">${v.value}</td>
                             <td style="padding: 4px; color: #a5d6ff;">${v.address_binding || "—"}</td>
                           </tr>
                         `
                           )
                           .join("")}
                       </tbody>
                     </table>`
                  : `<div style="padding: 16px; text-align: center; color: #8b949e; font-size: 12px;">
                       No variables declared. Compile a routine to inspect symbol memory.
                     </div>`
              }
            </div>
          </div>
        </div>
      </div>
    `;

    // Event listeners
    this.querySelectorAll(".lang-btn").forEach((btn) => {
      btn.addEventListener("click", (e) => {
        const target = e.currentTarget as HTMLElement;
        const lang = target.getAttribute("data-lang") as any;
        if (lang) this.handleLanguageChange(lang);
      });
    });

    const textarea = this.querySelector("#iec-source-input") as HTMLTextAreaElement | null;
    if (textarea) {
      textarea.addEventListener("input", () => {
        this.sourceCode = textarea.value;
      });
    }

    const compileBtn = this.querySelector("#btn-compile");
    if (compileBtn) {
      compileBtn.addEventListener("click", () => this.compile());
    }
  }
}

customElements.define("nebu-iec-editor", NebuIecEditor);
