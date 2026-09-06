/**
 * NebuDCS Protocol Types & Data Transfer Objects.
 * Authoritative mirror of `crates/nebudcs-protocol`.
 */

export const PROTOCOL_VERSION = "1.0.0";

export type TagQualityDto = "GOOD" | "UNCERTAIN" | "BAD" | "NOT_CONNECTED";

export type TagValueDto = number | boolean;

export interface TagValueSampleDto {
  tag: string;
  value: TagValueDto;
  quality: TagQualityDto;
  unit?: string;
}

export type AlarmStateDto =
  | "NORMAL"
  | "ACTIVE_UNACK"
  | "ACTIVE_ACK"
  | "CLEARED"
  | "SHELVED";

export type AlarmSeverityDto =
  | "DIAGNOSTIC"
  | "ADVISORY"
  | "WARNING"
  | "CRITICAL";

export interface AlarmSnapshotDto {
  alarm_name: string;
  state: AlarmStateDto;
  severity: AlarmSeverityDto;
  pv: number;
  trip_limit: number;
}

export interface StateSnapshotDto {
  snapshot_id: string;
  sim_time_sec: number;
  tick_count: number;
  tags: TagValueSampleDto[];
  active_alarms: AlarmSnapshotDto[];
}

export interface DeltaUpdateDto {
  seq: number;
  sim_time_sec: number;
  tick_count: number;
  changed_tags: TagValueSampleDto[];
}

export interface ServerHealthDto {
  status: string;
  simulation_running: boolean;
  sim_time_sec: number;
  tick_count: number;
  speed_multiplier: number;
  connected_clients: number;
  protocol_version: string;
  historian_healthy: boolean;
}

export interface HistoricalEventDto {
  event_id: string;
  event_type: string;
  sim_time_sec: number;
  tick_count: number;
  source_id: string;
  description: string;
  payload: Record<string, unknown>;
  real_timestamp_ms: number;
}

export interface HistoricalSampleDto {
  point_id: number;
  sim_time_sec: number;
  tick_count: number;
  value: number;
  quality: TagQualityDto;
  real_timestamp_ms: number;
}

export type PidModeDto = "MANUAL" | "AUTO" | "CASCADE" | "REMOTE_CASCADE";

export type SimControlAction =
  | { action: "play" }
  | { action: "pause" }
  | { action: "step" }
  | { action: "set_speed_multiplier"; multiplier: number }
  | { action: "reset" };

export type RuntimeCommand =
  | ({ type: "sim_control" } & SimControlAction)
  | { type: "write_tag"; tag: string; value: TagValueDto }
  | { type: "set_setpoint"; loop_name: string; sp: number }
  | { type: "set_mode"; loop_name: string; mode: PidModeDto }
  | { type: "set_manual_output"; loop_name: string; output: number }
  | { type: "acknowledge_alarm"; alarm_name: string }
  | { type: "shelve_alarm"; alarm_name: string; duration_seconds: number }
  | { type: "reset_protection"; block_name: string };

export interface ClientCapabilities {
  supports_binary: boolean;
  max_batch_size: number;
}

export type ClientMessage =
  | {
      type: "handshake";
      protocol_version: string;
      client_id: string;
      capabilities: ClientCapabilities;
    }
  | {
      type: "subscribe";
      tags: string[];
      all_tags: boolean;
      include_alarms: boolean;
    }
  | {
      type: "unsubscribe";
      tags: string[];
    }
  | {
      type: "request_snapshot";
    }
  | {
      type: "command";
      command_id: string;
      command: RuntimeCommand;
    }
  | {
      type: "ping";
      nonce: number;
    };

export type ServerMessage =
  | {
      type: "handshake_ack";
      payload: {
        protocol_version: string;
        server_version: string;
        session_id: string;
        model_version: string;
      };
    }
  | {
      type: "snapshot";
      payload: StateSnapshotDto;
    }
  | {
      type: "delta";
      payload: DeltaUpdateDto;
    }
  | {
      type: "alarm_update";
      payload: AlarmSnapshotDto[];
    }
  | {
      type: "event";
      payload: HistoricalEventDto;
    }
  | {
      type: "command_ack";
      payload: {
        command_id: string;
        success: boolean;
        message?: string;
        error_code?: string;
      };
    }
  | {
      type: "pong";
      payload: {
        nonce: number;
      };
    }
  | {
      type: "error";
      payload: {
        code: string;
        message: string;
        details?: string;
        correlation_id?: string;
      };
    }
  | {
      type: "scenario_update";
      payload: ScenarioStatusDto;
    };

export interface ActiveFaultDto {
  id: string;
  target: string;
  category: string;
  elapsed_sec: number;
  duration_sec?: number;
}

export interface ScenarioStatusDto {
  scenario_id?: string;
  scenario_name?: string;
  state: string;
  elapsed_sec: number;
  duration_limit_sec?: number;
  active_faults: ActiveFaultDto[];
  active_disturbances_count: number;
  events_total: number;
  events_fired: number;
  score: number;
  mandatory_completed: boolean;
}

export function isCompatibleProtocol(version: string): boolean {
  const parts = version.split(".");
  if (parts.length !== 3) return false;
  return parts[0] === "1" && parts[1] === "0";
}

