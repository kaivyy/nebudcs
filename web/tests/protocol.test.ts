import { describe, it, expect } from 'vitest';
import {
  ClientMessage,
  ServerMessage,
  PROTOCOL_VERSION,
  isCompatibleProtocol,
  StateSnapshotDto,
  DeltaUpdateDto,
} from '../src/protocol/types';

describe('Protocol Types & Serialization', () => {
  it('validates protocol version matching', () => {
    expect(isCompatibleProtocol('1.0.0')).toBe(true);
    expect(isCompatibleProtocol('1.0.1')).toBe(true);
    expect(isCompatibleProtocol('1.1.0')).toBe(false);
    expect(isCompatibleProtocol('2.0.0')).toBe(false);
    expect(isCompatibleProtocol('invalid')).toBe(false);
  });

  it('correctly structures handshake message', () => {
    const msg: ClientMessage = {
      type: 'handshake',
      protocol_version: PROTOCOL_VERSION,
      client_id: 'test-client-1',
      capabilities: {
        supports_binary: false,
        max_batch_size: 1000,
      },
    };
    const json = JSON.stringify(msg);
    const parsed = JSON.parse(json);

    expect(parsed.type).toBe('handshake');
    expect(parsed.protocol_version).toBe('1.0.0');
    expect(parsed.client_id).toBe('test-client-1');
    expect(parsed.capabilities.supports_binary).toBe(false);
  });

  it('correctly structures subscription message', () => {
    const msg: ClientMessage = {
      type: 'subscribe',
      tags: ['LIC-101.PV', 'LIC-101.OUT'],
      all_tags: false,
      include_alarms: true,
    };
    const parsed = JSON.parse(JSON.stringify(msg));
    expect(parsed.type).toBe('subscribe');
    expect(parsed.tags).toEqual(['LIC-101.PV', 'LIC-101.OUT']);
    expect(parsed.all_tags).toBe(false);
    expect(parsed.include_alarms).toBe(true);
  });

  it('correctly structures simulation control commands', () => {
    const playMsg: ClientMessage = {
      type: 'command',
      command_id: 'cmd-001',
      command: {
        type: 'sim_control',
        action: 'play',
      },
    };
    expect(JSON.parse(JSON.stringify(playMsg))).toEqual({
      type: 'command',
      command_id: 'cmd-001',
      command: { type: 'sim_control', action: 'play' },
    });

    const speedMsg: ClientMessage = {
      type: 'command',
      command_id: 'cmd-002',
      command: {
        type: 'sim_control',
        action: 'set_speed_multiplier',
        multiplier: 2.5,
      },
    };
    const parsedSpeed = JSON.parse(JSON.stringify(speedMsg));
    expect(parsedSpeed.command.action).toBe('set_speed_multiplier');
    expect(parsedSpeed.command.multiplier).toBe(2.5);
  });

  it('correctly structures tag write commands', () => {
    const writeFloat: ClientMessage = {
      type: 'command',
      command_id: 'cmd-003',
      command: {
        type: 'write_tag',
        tag: 'LIC-101.SP',
        value: 52.5,
      },
    };
    const parsedFloat = JSON.parse(JSON.stringify(writeFloat));
    expect(parsedFloat.command.tag).toBe('LIC-101.SP');
    expect(parsedFloat.command.value).toBe(52.5);

    const writeBool: ClientMessage = {
      type: 'command',
      command_id: 'cmd-004',
      command: {
        type: 'write_tag',
        tag: 'K-101.RUN_CMD',
        value: true,
      },
    };
    const parsedBool = JSON.parse(JSON.stringify(writeBool));
    expect(parsedBool.command.value).toBe(true);
  });

  it('correctly structures PID mode and setpoint commands', () => {
    const modeCmd: ClientMessage = {
      type: 'command',
      command_id: 'cmd-005',
      command: {
        type: 'set_mode',
        loop_name: 'LIC-101.OUT',
        mode: 'AUTO',
      },
    };
    const parsedMode = JSON.parse(JSON.stringify(modeCmd));
    expect(parsedMode.command.mode).toBe('AUTO');
    expect(parsedMode.command.loop_name).toBe('LIC-101.OUT');

    const spCmd: ClientMessage = {
      type: 'command',
      command_id: 'cmd-006',
      command: {
        type: 'set_setpoint',
        loop_name: 'LIC-101.OUT',
        sp: 62.0,
      },
    };
    const parsedSp = JSON.parse(JSON.stringify(spCmd));
    expect(parsedSp.command.sp).toBe(62.0);
  });

  it('correctly structures alarm acknowledge and shelve commands', () => {
    const ackCmd: ClientMessage = {
      type: 'command',
      command_id: 'cmd-007',
      command: {
        type: 'acknowledge_alarm',
        alarm_name: 'ALM-LIC-101-HIHI',
      },
    };
    const parsedAck = JSON.parse(JSON.stringify(ackCmd));
    expect(parsedAck.command.type).toBe('acknowledge_alarm');
    expect(parsedAck.command.alarm_name).toBe('ALM-LIC-101-HIHI');

    const shelveCmd: ClientMessage = {
      type: 'command',
      command_id: 'cmd-008',
      command: {
        type: 'shelve_alarm',
        alarm_name: 'ALM-LIC-101-HIHI',
        duration_seconds: 300,
      },
    };
    const parsedShelve = JSON.parse(JSON.stringify(shelveCmd));
    expect(parsedShelve.command.type).toBe('shelve_alarm');
    expect(parsedShelve.command.duration_seconds).toBe(300);
  });

  it('validates server message deserialization', () => {
    const snapshotPayload: StateSnapshotDto = {
      snapshot_id: 'snap-1',
      sim_time_sec: 15.0,
      tick_count: 750,
      tags: [
        { tag: 'LIC-101.PV', value: 50.5, quality: 'GOOD', unit: '%' },
        { tag: 'LIC-101.OUT', value: 45.2, quality: 'GOOD', unit: '%' },
      ],
      active_alarms: [
        {
          alarm_name: 'ALM-LIC-101-HI',
          state: 'ACTIVE_UNACK',
          severity: 'WARNING',
          pv: 50.5,
          trip_limit: 80.0,
        },
      ],
    };

    const serverMsg: ServerMessage = {
      type: 'snapshot',
      payload: snapshotPayload,
    };

    expect(serverMsg.type).toBe('snapshot');
    expect(serverMsg.payload.tags.length).toBe(2);
    expect(serverMsg.payload.active_alarms[0].severity).toBe('WARNING');

    const deltaPayload: DeltaUpdateDto = {
      seq: 751,
      sim_time_sec: 15.02,
      tick_count: 751,
      changed_tags: [
        { tag: 'LIC-101.PV', value: 50.6, quality: 'GOOD', unit: '%' },
      ],
    };

    const deltaMsg: ServerMessage = {
      type: 'delta',
      payload: deltaPayload,
    };
    expect(deltaMsg.payload.seq).toBe(751);
  });
});
