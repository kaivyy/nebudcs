import { BASELINE_DEMO_PROJECT } from './fixtures/baseline';
import { describe, it, expect, beforeEach, beforeAll, vi } from 'vitest';
import { store } from '../src/state/store';
import { NebuProcessView } from '../src/components/nebu-process-view';
import { NebuFaceplate } from '../src/components/nebu-faceplate';
import { NebuAlarmPanel } from '../src/components/nebu-alarm-panel';
import { NebuTrend } from '../src/components/nebu-trend';
import { NebuApp } from '../src/components/nebu-app';

describe('Phase 14 Operator / DCS Station Suite', () => {
  beforeAll(() => {
    const elements: Array<[string, CustomElementConstructor]> = [
      ['nebu-process-view', NebuProcessView],
      ['nebu-faceplate', NebuFaceplate],
      ['nebu-alarm-panel', NebuAlarmPanel],
      ['nebu-trend', NebuTrend],
      ['nebu-app', NebuApp],
    ];
    for (const [tag, cls] of elements) {
      if (!window.customElements.get(tag)) {
        window.customElements.define(tag, cls);
      }
    }

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        status: 'healthy',
        simulation_running: true,
        sim_time_sec: 10.0,
        tick_count: 500,
        speed_multiplier: 1.0,
        connected_clients: 1,
        protocol_version: '1.0.0',
        historian_healthy: true,
      }),
    });
  });

  beforeEach(() => {
    document.body.innerHTML = '';
    store.tags.clear();
    store.alarms.clear();
    store.tagHistory.clear();
    store.setFocusedEntity(null);
    store.setAlarmSeverityFilter('ALL');
    store.setAlarmStateFilter('ALL');
    store.setActiveProject(BASELINE_DEMO_PROJECT);
  });

  describe('Multi-Profile Faceplate Ergonomics', () => {
    it('renders control loop profile by default with PID mode & setpoint controls', async () => {
      const el = document.createElement('nebu-faceplate') as NebuFaceplate;
      document.body.appendChild(el);

      expect(el.querySelector('.faceplate-card')).not.toBeNull();
      expect(el.querySelector('.fp-profile-loop')).not.toBeNull();
      expect(el.querySelector('.pv-val')?.textContent).toBe('50.0');

      const modeSelect = el.querySelector('#pidModeSelect') as HTMLSelectElement;
      expect(modeSelect).not.toBeNull();

      // Test SP nudge
      const nudgePlus5 = el.querySelector('button[data-delta="5"]') as HTMLButtonElement;
      expect(nudgePlus5).not.toBeNull();
      nudgePlus5.click();

      const spInput = el.querySelector('#spIn') as HTMLInputElement;
      expect(spInput.value).toBe('55.0');

      el.remove();
    });

    it('switches to pump profile and dispatches equipment control commands', async () => {
      const el = document.createElement('nebu-faceplate') as NebuFaceplate;
      const mockApi = {
        controlEquipment: vi.fn().mockResolvedValue({ status: 'ok' }),
      } as any;
      el.apiClient = mockApi;
      document.body.appendChild(el);

      store.openFaceplate({
        kind: 'pump',
        id: 'K-101',
        name: 'PUMP K-101',
        description: 'CENTRIFUGAL CHARGE PUMP',
        equipmentId: 'K-101',
      });
      await Promise.resolve();

      expect(el.querySelector('.fp-profile-equipment')?.getAttribute('style')).not.toContain('display: none');
      expect(el.textContent).toContain('PUMP K-101');
      expect(el.textContent).toContain('DISCHARGE P');

      const startBtn = el.querySelector('#btnPumpStart') as HTMLButtonElement;
      expect(startBtn).not.toBeNull();
      startBtn.click();
      await Promise.resolve();

      expect(mockApi.controlEquipment).toHaveBeenCalledWith(
        expect.objectContaining({
          instance_id: 'K-101',
          action: 'start',
        })
      );

      // Emergency trip
      const tripBtn = el.querySelector('#btnPumpTrip') as HTMLButtonElement;
      expect(tripBtn).not.toBeNull();
      tripBtn.click();
      await Promise.resolve();

      expect(mockApi.controlEquipment).toHaveBeenCalledWith(
        expect.objectContaining({
          instance_id: 'K-101',
          action: 'stop',
        })
      );

      el.remove();
    });

    it('renders valve profile with stroke presets and direct modulation', async () => {
      const el = document.createElement('nebu-faceplate') as NebuFaceplate;
      const mockApi = {
        setManualOutput: vi.fn().mockResolvedValue({ status: 'ok' }),
      } as any;
      el.apiClient = mockApi;
      document.body.appendChild(el);

      store.openFaceplate({
        kind: 'valve',
        id: 'FV-101',
        name: 'FV-101',
        description: 'FEED CONTROL VALVE',
        equipmentId: 'FV-101',
      });
      await Promise.resolve();

      expect(el.textContent).toContain('VALVE MODULATING');
      expect(el.textContent).toContain('ACTUAL POSITION');

      const preset50 = el.querySelector('.preset-btn[data-val="50"]') as HTMLButtonElement;
      expect(preset50).not.toBeNull();
      preset50.click();
      await Promise.resolve();

      expect(mockApi.setManualOutput).toHaveBeenCalledWith(expect.any(String), 50);

      el.remove();
    });

    it('renders compressor, vessel, and instrument profiles', async () => {
      const el = document.createElement('nebu-faceplate') as NebuFaceplate;
      document.body.appendChild(el);

      // Compressor
      store.openFaceplate({
        kind: 'compressor',
        id: 'C-101',
        name: 'COMPRESSOR C-101',
        equipmentId: 'C-101',
      });
      await Promise.resolve();
      expect(el.textContent).toContain('SURGE MARGIN');
      expect(el.querySelector('#btnCompTrip')).not.toBeNull();

      // Vessel
      store.openFaceplate({
        kind: 'vessel',
        id: 'V-101',
        name: 'VESSEL V-101',
        equipmentId: 'V-101',
      });
      await Promise.resolve();
      expect(el.textContent).toContain('LIQUID LEVEL');
      expect(el.textContent).toContain('PSV STATUS');

      // Instrument
      store.openFaceplate({
        kind: 'instrument',
        id: 'TI-101',
        name: 'TI-101',
        pvTag: 'TI-101',
        unit: '°C',
        rangeMin: 0,
        rangeMax: 300,
      });
      await Promise.resolve();
      expect(el.textContent).toContain('TRANSMITTER SIGNAL VALID');
      expect(el.querySelector('.large-pv')).not.toBeNull();

      // Reset button returns to control loop
      const resetBtn = el.querySelector('#btnFpClose') as HTMLButtonElement;
      expect(resetBtn).not.toBeNull();
      resetBtn.click();
      await Promise.resolve();

      expect(store.selectedFaceplate?.kind).toBe('control_loop');
      expect(store.focusedEntityId).toBeNull();

      el.remove();
    });
  });

  describe('Dynamic Process View & Topology Interaction', () => {
    it('clicking SVG equipment opens corresponding faceplates and sets focus', async () => {
      const el = document.createElement('nebu-process-view') as NebuProcessView;
      document.body.appendChild(el);

      // Click Pump
      const pump = el.querySelector('#pumpGroup') as SVGGElement;
      expect(pump).not.toBeNull();
      pump.dispatchEvent(new MouseEvent('click', { bubbles: true }));
      await Promise.resolve();

      expect(store.selectedFaceplate?.kind).toBe('pump');
      expect(store.selectedFaceplate?.id).toBe('K-101');
      expect(store.focusedEntityId).toBe('K-101');

      // Click Vessel
      const vessel = el.querySelector('#vesselGroup') as SVGGElement;
      expect(vessel).not.toBeNull();
      vessel.dispatchEvent(new MouseEvent('click', { bubbles: true }));
      await Promise.resolve();

      expect(store.selectedFaceplate?.kind).toBe('vessel');
      expect(store.selectedFaceplate?.id).toBe('V-101');
      expect(store.focusedEntityId).toBe('V-101');

      // Click Valve
      const valve = el.querySelector('#valveGroup') as SVGGElement;
      expect(valve).not.toBeNull();
      valve.dispatchEvent(new MouseEvent('click', { bubbles: true }));
      await Promise.resolve();

      expect(store.selectedFaceplate?.kind).toBe('valve');
      expect(store.selectedFaceplate?.id).toBe('FV-101');
      expect(store.focusedEntityId).toBe('FV-101');

      el.remove();
    });

    it('highlights equipment node in abnormal alarm state', async () => {
      const el = document.createElement('nebu-process-view') as NebuProcessView;
      document.body.appendChild(el);

      // Trigger alarm on pump K-101
      store.applyAlarmUpdate([
        {
          alarm_name: 'ALM-K-101-TRIP',
          state: 'ACTIVE_UNACK',
          severity: 'CRITICAL',
          pv: 0,
          trip_limit: 1,
        },
      ]);
      await Promise.resolve();

      const pump = el.querySelector('#pumpGroup') as SVGGElement;
      expect(pump.classList.contains('alarm-pulsing')).toBe(true);

      const ring = pump.querySelector('.focus-ring') as SVGElement;
      expect(ring.getAttribute('stroke')).toBe('#ef4444');

      el.remove();
    });
  });

  describe('Realtime Trend Viewer', () => {
    it('updates time window when operator changes selection', async () => {
      const el = document.createElement('nebu-trend') as NebuTrend;
      document.body.appendChild(el);

      const winSelect = el.querySelector('#trendWindowSelect') as HTMLSelectElement;
      expect(winSelect).not.toBeNull();

      winSelect.value = '300';
      winSelect.dispatchEvent(new Event('change'));

      expect(store.trendTimeWindowSec).toBe(300);
      expect(el.querySelector('#trendWindowSub')?.textContent).toContain('300s ROLLING WINDOW');

      el.remove();
    });

    it('switches pen groups across process variables', async () => {
      const el = document.createElement('nebu-trend') as NebuTrend;
      document.body.appendChild(el);

      const tagSelect = el.querySelector('#trendTagSelect') as HTMLSelectElement;
      expect(tagSelect).not.toBeNull();

      tagSelect.value = 'FIC-101';
      tagSelect.dispatchEvent(new Event('change'));

      tagSelect.value = 'PI-101';
      tagSelect.dispatchEvent(new Event('change'));

      el.remove();
    });
  });

  describe('Alarm & Protection Station', () => {
    it('filters alarms by severity and state', async () => {
      const el = document.createElement('nebu-alarm-panel') as NebuAlarmPanel;
      document.body.appendChild(el);

      store.applyAlarmUpdate([
        {
          alarm_name: 'ALM-K-101-TRIP',
          state: 'ACTIVE_UNACK',
          severity: 'CRITICAL',
          pv: 0,
          trip_limit: 1,
        },
        {
          alarm_name: 'ALM-LIC-101-HI',
          state: 'ACTIVE_ACK',
          severity: 'WARNING',
          pv: 82,
          trip_limit: 80,
        },
      ]);
      await Promise.resolve();

      let rows = el.querySelectorAll('table.alarm-table tbody tr');
      expect(rows.length).toBe(2);

      // Filter to CRITICAL only
      const sevSelect = el.querySelector('#alarmSevFilter') as HTMLSelectElement;
      sevSelect.value = 'CRITICAL';
      sevSelect.dispatchEvent(new Event('change'));
      await Promise.resolve();

      rows = el.querySelectorAll('table.alarm-table tbody tr');
      expect(rows.length).toBe(1);
      expect(rows[0].textContent).toContain('ALM-K-101-TRIP');

      // Filter to UNACK state only
      sevSelect.value = 'ALL';
      sevSelect.dispatchEvent(new Event('change'));
      const stateSelect = el.querySelector('#alarmStateFilter') as HTMLSelectElement;
      stateSelect.value = 'UNACK';
      stateSelect.dispatchEvent(new Event('change'));
      await Promise.resolve();

      rows = el.querySelectorAll('table.alarm-table tbody tr');
      expect(rows.length).toBe(1);
      expect(rows[0].textContent).toContain('ALM-K-101-TRIP');

      el.remove();
    });

    it('navigates to equipment from alarm row via GOTO button', async () => {
      const el = document.createElement('nebu-alarm-panel') as NebuAlarmPanel;
      document.body.appendChild(el);

      store.applyAlarmUpdate([
        {
          alarm_name: 'ALM-K-101-TRIP',
          state: 'ACTIVE_UNACK',
          severity: 'CRITICAL',
          pv: 0,
          trip_limit: 1,
        },
      ]);
      await Promise.resolve();

      const gotoBtn = el.querySelector('.goto-btn') as HTMLButtonElement;
      expect(gotoBtn).not.toBeNull();
      gotoBtn.click();
      await Promise.resolve();

      expect(store.focusedEntityId).toBe('K-101');
      expect(store.selectedFaceplate?.kind).toBe('pump');
      expect(store.selectedFaceplate?.id).toBe('K-101');

      el.remove();
    });

    it('provides protection reset button', async () => {
      const el = document.createElement('nebu-alarm-panel') as NebuAlarmPanel;
      const mockApi = {
        resetProtection: vi.fn().mockResolvedValue({ status: 'ok' }),
      } as any;
      el.apiClient = mockApi;
      document.body.appendChild(el);

      const resetBtn = el.querySelector('#resetTripsBtn') as HTMLButtonElement;
      expect(resetBtn).not.toBeNull();
      resetBtn.click();
      await Promise.resolve();

      expect(mockApi.resetProtection).toHaveBeenCalled();

      el.remove();
    });
  });

  describe('DCS Operator Console Ergonomics & Keyboard Navigation', () => {
    it('displays top priority alarm ticker and acknowledges via Space key', async () => {
      const appEl = document.createElement('nebu-app') as NebuApp;
      const mockApi = {
        getHealth: vi.fn().mockResolvedValue({
          status: 'healthy',
          simulation_running: true,
          speed_multiplier: 1.0,
        }),
        acknowledgeAlarm: vi.fn().mockResolvedValue({ status: 'ok' }),
      } as any;
      appEl.apiClient = mockApi;
      document.body.appendChild(appEl);

      store.applyAlarmUpdate([
        {
          alarm_name: 'ALM-LIC-101-HI',
          state: 'ACTIVE_UNACK',
          severity: 'CRITICAL',
          pv: 85,
          trip_limit: 80,
        },
      ]);
      await Promise.resolve();

      const ticker = appEl.querySelector('#alarmTickerBanner') as HTMLElement;
      expect(ticker.style.display).not.toBe('none');
      expect(ticker.textContent).toContain('ALM-LIC-101-HI');

      // Dispatch Space keydown
      window.dispatchEvent(new KeyboardEvent('keydown', { code: 'Space' }));
      await Promise.resolve();

      expect(mockApi.acknowledgeAlarm).toHaveBeenCalledWith('ALM-LIC-101-HI');

      appEl.remove();
    });
  });
});
