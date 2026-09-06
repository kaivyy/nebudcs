import { describe, it, expect, beforeEach, beforeAll, vi } from 'vitest';
import { store } from '../src/state/store';
import { NebuConnectionStatus } from '../src/components/nebu-connection-status';
import { NebuSimControls } from '../src/components/nebu-sim-controls';
import { NebuProcessView } from '../src/components/nebu-process-view';
import { NebuFaceplate } from '../src/components/nebu-faceplate';
import { NebuAlarmPanel } from '../src/components/nebu-alarm-panel';
import { NebuTrend } from '../src/components/nebu-trend';
import { NebuTagTable } from '../src/components/nebu-tag-table';
import { NebuApp } from '../src/components/nebu-app';

describe('Native Web Components Suite', () => {
  beforeAll(() => {
    // Ensure all custom elements are defined on the active test window
    const elements: Array<[string, CustomElementConstructor]> = [
      ['nebu-connection-status', NebuConnectionStatus],
      ['nebu-sim-controls', NebuSimControls],
      ['nebu-process-view', NebuProcessView],
      ['nebu-faceplate', NebuFaceplate],
      ['nebu-alarm-panel', NebuAlarmPanel],
      ['nebu-trend', NebuTrend],
      ['nebu-tag-table', NebuTagTable],
      ['nebu-app', NebuApp],
    ];
    for (const [tag, cls] of elements) {
      if (!window.customElements.get(tag)) {
        window.customElements.define(tag, cls);
      }
    }

    // Mock fetch for health checks
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        status: 'healthy',
        simulation_running: false,
        sim_time_sec: 0,
        tick_count: 0,
        speed_multiplier: 1.0,
        connected_clients: 1,
        protocol_version: '1.0.0',
        historian_healthy: true,
      }),
    });
  });

  beforeEach(() => {
    document.body.innerHTML = '';
  });

  it('renders nebu-connection-status and updates text on state change', async () => {
    const el = document.createElement('nebu-connection-status') as NebuConnectionStatus;
    document.body.appendChild(el);

    const badge = el.querySelector('.status-text');
    expect(badge).not.toBeNull();
    expect(badge?.textContent).toBe('DISCONNECTED');

    store.setConnectionState('synchronized');
    await Promise.resolve();
    expect(badge?.textContent).toBe('SYNCHRONIZED');

    el.remove();
  });

  it('renders nebu-sim-controls with play/pause/step buttons and speed selector', () => {
    const el = document.createElement('nebu-sim-controls') as NebuSimControls;
    document.body.appendChild(el);

    expect(el.querySelector('.btn-play')).not.toBeNull();
    expect(el.querySelector('.btn-pause')).not.toBeNull();
    expect(el.querySelector('.btn-step')).not.toBeNull();
    expect(el.querySelector('.btn-reset')).not.toBeNull();
    expect(el.querySelector('#speed-select')).not.toBeNull();
    expect(el.querySelector('.telemetry-clock')).not.toBeNull();

    el.remove();
  });

  it('renders nebu-process-view SVG and reflects dynamic process values', async () => {
    const el = document.createElement('nebu-process-view') as NebuProcessView;
    document.body.appendChild(el);

    const svg = el.querySelector('svg.process-svg');
    expect(svg).not.toBeNull();
    expect(el.querySelector('#vesselFluid')).not.toBeNull();
    expect(el.querySelector('#valvePlug')).not.toBeNull();

    // Push values into store
    store.applySnapshot({
      snapshot_id: 'snap-1',
      sim_time_sec: 1.0,
      tick_count: 50,
      tags: [
        { tag: 'TI-101', value: 195.5, quality: 'GOOD' },
        { tag: 'PI-101', value: 34.2, quality: 'GOOD' },
        { tag: 'LIC-101.PV', value: 75.0, quality: 'GOOD' },
        { tag: 'LIC-101.OUT', value: 80.0, quality: 'GOOD' },
      ],
      active_alarms: [],
    });
    await Promise.resolve();

    const tiEl = el.querySelector('#ti101Val');
    expect(tiEl?.textContent).toBe('195.5 °C');

    const vesselEl = el.querySelector('#vesselLevelVal');
    expect(vesselEl?.textContent).toBe('75.0 %');

    el.remove();
  });

  it('renders nebu-faceplate and binds mode and setpoints', async () => {
    const el = document.createElement('nebu-faceplate') as NebuFaceplate;
    document.body.appendChild(el);

    expect(el.querySelector('.faceplate-card')).not.toBeNull();
    expect(el.querySelector('.pv-val')).not.toBeNull();
    expect(el.querySelector('#spIn')).not.toBeNull();
    expect(el.querySelector('#outIn')).not.toBeNull();

    // Set loop tags
    store.applySnapshot({
      snapshot_id: 'snap-2',
      sim_time_sec: 2.0,
      tick_count: 100,
      tags: [
        { tag: 'LIC-101.PV', value: 50.0, quality: 'GOOD' },
        { tag: 'LIC-101.SP', value: 50.0, quality: 'GOOD' },
        { tag: 'LIC-101.OUT', value: 40.0, quality: 'GOOD' },
      ],
      active_alarms: [],
    });
    await Promise.resolve();

    const pvEl = el.querySelector('.pv-val');
    expect(pvEl?.textContent).toContain('50.0');

    el.remove();
  });

  it('renders nebu-alarm-panel and reflects alarm entries', async () => {
    const el = document.createElement('nebu-alarm-panel') as NebuAlarmPanel;
    document.body.appendChild(el);

    store.applyAlarmUpdate([
      {
        alarm_name: 'ALM-LIC-101-HI',
        state: 'ACTIVE_UNACK',
        severity: 'WARNING',
        pv: 82.0,
        trip_limit: 80.0,
      },
    ]);
    await Promise.resolve();

    const rows = el.querySelectorAll('table.alarm-table tbody tr');
    expect(rows.length).toBe(1);
    expect(rows[0].textContent).toContain('ALM-LIC-101-HI');

    el.remove();
  });

  it('renders nebu-trend and nebu-tag-table components', () => {
    const trendEl = document.createElement('nebu-trend') as NebuTrend;
    document.body.appendChild(trendEl);
    expect(trendEl.querySelector('canvas#trendCanvas')).not.toBeNull();
    trendEl.remove();

    const tableEl = document.createElement('nebu-tag-table') as NebuTagTable;
    document.body.appendChild(tableEl);
    expect(tableEl.querySelector('table.tag-table')).not.toBeNull();
    tableEl.remove();
  });

  it('renders root nebu-app shell containing all vertical slice sub-components', () => {
    const appEl = document.createElement('nebu-app') as NebuApp;
    document.body.appendChild(appEl);

    expect(appEl.querySelector('nebu-connection-status')).not.toBeNull();
    expect(appEl.querySelector('nebu-sim-controls')).not.toBeNull();
    expect(appEl.querySelector('nebu-process-view')).not.toBeNull();
    expect(appEl.querySelector('nebu-faceplate')).not.toBeNull();
    expect(appEl.querySelector('nebu-trend')).not.toBeNull();
    expect(appEl.querySelector('nebu-alarm-panel')).not.toBeNull();
    expect(appEl.querySelector('nebu-tag-table')).not.toBeNull();

    appEl.remove();
  });
});
