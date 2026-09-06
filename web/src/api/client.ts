/**
 * NebuDCS REST API Client.
 */

import {
  AlarmSnapshotDto,
  PidModeDto,
  ScenarioStatusDto,
  ServerHealthDto,
  SimControlAction,
  StateSnapshotDto,
  TagValueDto,
  TagValueSampleDto,
} from "../protocol/types";

export interface ApiClientConfig {
  baseUrl?: string;
  authToken?: string;
}

export class ApiClient {
  private baseUrl: string;
  private authToken?: string;

  constructor(config: ApiClientConfig = {}) {
    this.baseUrl = config.baseUrl || "";
    this.authToken = config.authToken;
  }

  setAuthToken(token?: string) {
    this.authToken = token;
  }

  private async request<T>(
    endpoint: string,
    method: "GET" | "POST" = "GET",
    body?: unknown
  ): Promise<T> {
    const headers: Record<string, string> = {
      Accept: "application/json",
    };

    if (this.authToken) {
      headers["Authorization"] = `Bearer ${this.authToken}`;
    }

    if (body !== undefined) {
      headers["Content-Type"] = "application/json";
    }

    const res = await fetch(`${this.baseUrl}${endpoint}`, {
      method,
      headers,
      body: body !== undefined ? JSON.stringify(body) : undefined,
    });

    if (!res.ok) {
      let errMessage = `HTTP ${res.status} ${res.statusText}`;
      try {
        const errJson = await res.json();
        if (errJson.error) {
          errMessage = errJson.error;
        } else if (errJson.message) {
          errMessage = errJson.message;
        }
      } catch {
        // Fallback to text status
      }
      throw new Error(errMessage);
    }

    return res.json() as Promise<T>;
  }

  async getHealth(): Promise<ServerHealthDto> {
    return this.request<ServerHealthDto>("/api/v1/health");
  }

  async getSnapshot(): Promise<StateSnapshotDto> {
    return this.request<StateSnapshotDto>("/api/v1/sim/snapshot");
  }

  async controlSim(cmd: SimControlAction): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/sim/control", "POST", cmd);
  }

  async listTags(): Promise<{ count: number; tags: TagValueSampleDto[] }> {
    return this.request<{ count: number; tags: TagValueSampleDto[] }>("/api/v1/tags");
  }

  async getTag(tagId: string): Promise<TagValueSampleDto> {
    return this.request<TagValueSampleDto>(`/api/v1/tags/${encodeURIComponent(tagId)}`);
  }

  async writeTag(tag: string, value: TagValueDto): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/tags/write", "POST", {
      tag,
      value,
    });
  }

  async listAlarms(): Promise<{ count: number; alarms: AlarmSnapshotDto[] }> {
    return this.request<{ count: number; alarms: AlarmSnapshotDto[] }>("/api/v1/alarms");
  }

  async acknowledgeAlarm(alarmName: string): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/alarms/acknowledge", "POST", {
      alarm_name: alarmName,
    });
  }

  async shelveAlarm(
    alarmName: string,
    durationSeconds: number
  ): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/alarms/shelve", "POST", {
      alarm_name: alarmName,
      duration_seconds: durationSeconds,
    });
  }

  async resetProtection(blockName: string): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/protection/reset", "POST", {
      block_name: blockName,
    });
  }

  async setPidSetpoint(loopName: string, sp: number): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/control/pid/setpoint", "POST", {
      loop_name: loopName,
      sp,
    });
  }

  async setPidMode(loopName: string, mode: PidModeDto): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/control/pid/mode", "POST", {
      loop_name: loopName,
      mode,
    });
  }

  async setManualOutput(
    loopName: string,
    output: number
  ): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/control/pid/output", "POST", {
      loop_name: loopName,
      output,
    });
  }

  async getScenarioStatus(): Promise<ScenarioStatusDto> {
    return this.request<ScenarioStatusDto>("/api/v1/scenario/status");
  }

  async loadScenario(scenario: unknown): Promise<{ status: string; scenario_id?: string }> {
    return this.request<{ status: string; scenario_id?: string }>("/api/v1/scenario/load", "POST", scenario);
  }

  async startScenario(): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/scenario/start", "POST");
  }

  async pauseScenario(): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/scenario/pause", "POST");
  }

  async resumeScenario(): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/scenario/resume", "POST");
  }

  async abortScenario(reason = ""): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/scenario/abort", "POST", { reason });
  }

  async resetScenario(): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/scenario/reset", "POST");
  }

  async injectFault(fault: unknown): Promise<{ status: string; fault_id?: string }> {
    return this.request<{ status: string; fault_id?: string }>("/api/v1/scenario/fault/inject", "POST", fault);
  }

  async clearFault(faultId: string): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/scenario/fault/clear", "POST", { fault_id: faultId });
  }
}
