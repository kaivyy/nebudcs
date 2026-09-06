/**
 * NebuDCS Frontend State Store.
 * Central single-source-of-truth for client telemetry, state buffering, and reactivity.
 */

import {
  AlarmSnapshotDto,
  DeltaUpdateDto,
  ScenarioStatusDto,
  ServerHealthDto,
  ServerMessage,
  StateSnapshotDto,
  TagValueSampleDto,
} from "../protocol/types";
import { ConnectionState } from "../websocket/client";

export type StoreListener = (store: Store) => void;

export class Store {
  public connectionState: ConnectionState = "disconnected";
  public simTime: number = 0;
  public tickCount: number = 0;
  public isSimRunning: boolean = false;
  public speedMultiplier: number = 1.0;
  public serverHealth: ServerHealthDto | null = null;
  public sessionId: string | null = null;
  public scenarioStatus: ScenarioStatusDto | null = null;

  // Authoritative simulation tags
  public tags: Map<string, TagValueSampleDto> = new Map();

  // Authoritative active alarms
  public alarms: Map<string, AlarmSnapshotDto> = new Map();

  // Live timeseries ring buffers for trend charts (max 300 points per tag)
  public tagHistory: Map<string, Array<{ time: number; value: number }>> = new Map();
  private maxHistoryLength: number = 300;

  // Selected PID controller for faceplate
  public selectedLoop: string = "LIC-101.OUT";

  // Selected tags for live trend view
  public selectedTrendTags: string[] = ["LIC-101.PV", "LIC-101.SP", "LIC-101.OUT"];

  private listeners: Set<StoreListener> = new Set();
  private notifyScheduled: boolean = false;

  public subscribe(listener: StoreListener): () => void {
    this.listeners.add(listener);
    listener(this);
    return () => this.listeners.delete(listener);
  }

  public notify() {
    if (!this.notifyScheduled) {
      this.notifyScheduled = true;
      queueMicrotask(() => {
        this.notifyScheduled = false;
        this.listeners.forEach((l) => l(this));
      });
    }
  }

  public setConnectionState(state: ConnectionState) {
    if (this.connectionState !== state) {
      this.connectionState = state;
      this.notify();
    }
  }

  public handleServerMessage(msg: ServerMessage) {
    switch (msg.type) {
      case "handshake_ack":
        this.sessionId = msg.payload.session_id;
        break;

      case "snapshot":
        this.applySnapshot(msg.payload);
        break;

      case "delta":
        this.applyDelta(msg.payload);
        break;

      case "alarm_update":
        this.applyAlarmUpdate(msg.payload);
        break;

      case "scenario_update":
        this.scenarioStatus = msg.payload;
        this.notify();
        break;

      case "command_ack":
        break;

      default:
        break;
    }
  }

  public setScenarioStatus(status: ScenarioStatusDto) {
    this.scenarioStatus = status;
    this.notify();
  }

  public applySnapshot(snapshot: StateSnapshotDto) {
    this.simTime = snapshot.sim_time_sec;
    this.tickCount = snapshot.tick_count;

    this.tags.clear();
    for (const sample of snapshot.tags) {
      this.tags.set(sample.tag, sample);
      this.recordHistoryPoint(sample.tag, snapshot.sim_time_sec, sample.value);
    }

    this.alarms.clear();
    for (const alm of snapshot.active_alarms) {
      this.alarms.set(alm.alarm_name, alm);
    }

    this.notify();
  }

  public applyDelta(delta: DeltaUpdateDto) {
    this.simTime = delta.sim_time_sec;
    this.tickCount = delta.tick_count;

    for (const sample of delta.changed_tags) {
      this.tags.set(sample.tag, sample);
      this.recordHistoryPoint(sample.tag, delta.sim_time_sec, sample.value);
    }

    this.notify();
  }

  public applyAlarmUpdate(alarmList: AlarmSnapshotDto[]) {
    this.alarms.clear();
    for (const alm of alarmList) {
      this.alarms.set(alm.alarm_name, alm);
    }
    this.notify();
  }

  private recordHistoryPoint(tag: string, time: number, value: unknown) {
    if (typeof value !== "number") return;
    let history = this.tagHistory.get(tag);
    if (!history) {
      history = [];
      this.tagHistory.set(tag, history);
    }
    history.push({ time, value });
    if (history.length > this.maxHistoryLength) {
      history.shift();
    }
  }

  public getTag(tag: string): TagValueSampleDto | undefined {
    return this.tags.get(tag);
  }

  public getTagFloat(tag: string, fallback: number = 0.0): number {
    const s = this.tags.get(tag);
    if (s && typeof s.value === "number") {
      return s.value;
    }
    return fallback;
  }

  public getAlarms(): AlarmSnapshotDto[] {
    return Array.from(this.alarms.values());
  }

  public getTrendData(tag: string): Array<{ time: number; value: number }> {
    return this.tagHistory.get(tag) || [];
  }

  public setServerHealth(health: ServerHealthDto) {
    this.serverHealth = health;
    this.isSimRunning = health.simulation_running;
    this.speedMultiplier = health.speed_multiplier;
    this.notify();
  }

  public setSelectedLoop(loop: string) {
    this.selectedLoop = loop;
    this.notify();
  }
}

// Global default singleton instance
export const store = new Store();
