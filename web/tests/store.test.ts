import { describe, it, expect, beforeEach, vi } from 'vitest';
import { Store } from '../src/state/store';
import { StateSnapshotDto, DeltaUpdateDto, AlarmSnapshotDto } from '../src/protocol/types';

describe('Store State Management', () => {
  let store: Store;

  beforeEach(() => {
    store = new Store();
  });

  it('initializes with default values', () => {
    expect(store.connectionState).toBe('disconnected');
    expect(store.simTime).toBe(0);
    expect(store.tickCount).toBe(0);
    expect(store.isSimRunning).toBe(false);
    expect(store.speedMultiplier).toBe(1.0);
    expect(store.tags.size).toBe(0);
    expect(store.alarms.size).toBe(0);
    expect(store.selectedLoop).toBe('');
  });

  it('notifies subscribers upon connection state changes', async () => {
    const listener = vi.fn();
    const unsub = store.subscribe(listener);

    // First call happens immediately on subscription
    expect(listener).toHaveBeenCalledTimes(1);

    store.setConnectionState('connected');
    // Microtask batching
    await Promise.resolve();
    expect(listener).toHaveBeenCalledTimes(2);
    expect(store.connectionState).toBe('connected');

    unsub();
    store.setConnectionState('synchronized');
    await Promise.resolve();
    expect(listener).toHaveBeenCalledTimes(2); // no further calls after unsub
  });

  it('correctly applies a full state snapshot', async () => {
    const listener = vi.fn();
    store.subscribe(listener);

    const snapshot: StateSnapshotDto = {
      snapshot_id: 'snap-42',
      sim_time_sec: 10.5,
      tick_count: 525,
      tags: [
        { tag: 'LIC-101.PV', value: 48.7, quality: 'GOOD' },
        { tag: 'LIC-101.SP', value: 50.0, quality: 'GOOD' },
        { tag: 'LIC-101.OUT', value: 45.0, quality: 'GOOD' },
      ],
      active_alarms: [
        {
          alarm_name: 'ALM-LIC-101-HI',
          state: 'ACTIVE_UNACK',
          severity: 'WARNING',
          pv: 48.7,
          trip_limit: 80.0,
        },
      ],
    };

    store.applySnapshot(snapshot);
    await Promise.resolve();

    expect(store.simTime).toBe(10.5);
    expect(store.tickCount).toBe(525);
    expect(store.tags.size).toBe(3);
    expect(store.getTagFloat('LIC-101.PV')).toBe(48.7);
    expect(store.getTagFloat('LIC-101.SP')).toBe(50.0);
    expect(store.getTagFloat('NON_EXISTENT', 99.9)).toBe(99.9);
    expect(store.alarms.size).toBe(1);
    expect(store.getAlarms()[0].alarm_name).toBe('ALM-LIC-101-HI');

    // Trend history populated
    const trendPv = store.getTrendData('LIC-101.PV');
    expect(trendPv.length).toBe(1);
    expect(trendPv[0]).toEqual({ time: 10.5, value: 48.7 });
  });

  it('correctly applies incremental delta updates and caps history ring buffer', async () => {
    const snapshot: StateSnapshotDto = {
      snapshot_id: 'snap-1',
      sim_time_sec: 1.0,
      tick_count: 50,
      tags: [{ tag: 'LIC-101.PV', value: 50.0, quality: 'GOOD' }],
      active_alarms: [],
    };
    store.applySnapshot(snapshot);

    // Apply 350 deltas to test ring buffer cap at 300
    for (let i = 1; i <= 350; i++) {
      const delta: DeltaUpdateDto = {
        seq: 50 + i,
        sim_time_sec: 1.0 + i * 0.02,
        tick_count: 50 + i,
        changed_tags: [
          { tag: 'LIC-101.PV', value: 50.0 + i * 0.1, quality: 'GOOD' },
        ],
      };
      store.applyDelta(delta);
    }

    expect(store.simTime).toBeCloseTo(1.0 + 350 * 0.02);
    expect(store.tickCount).toBe(400);
    expect(store.getTagFloat('LIC-101.PV')).toBeCloseTo(50.0 + 35.0);

    const history = store.getTrendData('LIC-101.PV');
    expect(history.length).toBe(300); // capped at maxHistoryLength
    // First element should have been shifted, newest at end
    expect(history[history.length - 1].time).toBeCloseTo(1.0 + 350 * 0.02);
  });

  it('updates alarm collection on applyAlarmUpdate', async () => {
    const alarms: AlarmSnapshotDto[] = [
      {
        alarm_name: 'ALM-K-101-VIB',
        state: 'ACTIVE_ACK',
        severity: 'CRITICAL',
        pv: 9.8,
        trip_limit: 8.5,
      },
    ];

    store.applyAlarmUpdate(alarms);
    await Promise.resolve();

    expect(store.alarms.size).toBe(1);
    expect(store.getAlarms()[0].severity).toBe('CRITICAL');
  });

  it('updates loop selection and server health', async () => {
    store.setSelectedLoop('FIC-101.OUT');
    expect(store.selectedLoop).toBe('FIC-101.OUT');

    store.setServerHealth({
      status: 'healthy',
      simulation_running: true,
      sim_time_sec: 120.0,
      tick_count: 6000,
      speed_multiplier: 2.0,
      connected_clients: 3,
      protocol_version: '1.0.0',
      historian_healthy: true,
    });

    expect(store.isSimRunning).toBe(true);
    expect(store.speedMultiplier).toBe(2.0);
    expect(store.serverHealth?.connected_clients).toBe(3);
  });
});
