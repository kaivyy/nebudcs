import { describe, it, expect, beforeEach, beforeAll, vi } from 'vitest';
import { store } from '../src/state/store';
import { NebuScenarioPanel } from '../src/components/nebu-scenario-panel';
import { ScenarioStatusDto } from '../src/protocol/types';
import { ApiClient } from '../src/api/client';

describe('Scenario Subsystem Web Verification', () => {
  beforeAll(() => {
    if (!window.customElements.get('nebu-scenario-panel')) {
      window.customElements.define('nebu-scenario-panel', NebuScenarioPanel);
    }
  });

  beforeEach(() => {
    document.body.innerHTML = '';
    store.scenarioStatus = null;
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ status: 'ok' }),
    });
  });

  it('updates store scenarioStatus and notifies subscribers', async () => {
    const listener = vi.fn();
    const unsub = store.subscribe(listener);

    const mockStatus: ScenarioStatusDto = {
      state: 'RUNNING',
      scenario_id: 'SCN-STARTUP-01',
      scenario_name: 'Cold Unit Startup',
      elapsed_sec: 42.5,
      duration_limit_sec: 300.0,
      active_faults: [
        {
          id: 'FLT-01',
          target: 'PI-101',
          category: 'sensor_bias',
          elapsed_sec: 10.0,
          duration_sec: 60.0,
        },
      ],
      active_disturbances_count: 0,
      events_total: 4,
      events_fired: 2,
      score: 50.0,
      mandatory_completed: false,
    };

    store.setScenarioStatus(mockStatus);
    await Promise.resolve();

    expect(store.scenarioStatus).toEqual(mockStatus);
    expect(listener).toHaveBeenCalled();

    unsub();
  });

  it('handles scenario_update websocket message correctly in store', async () => {
    const mockStatus: ScenarioStatusDto = {
      state: 'PAUSED',
      scenario_id: 'SCN-TEST',
      scenario_name: 'Test Scenario',
      elapsed_sec: 15.0,
      duration_limit_sec: 100.0,
      active_faults: [],
      active_disturbances_count: 0,
      events_total: 2,
      events_fired: 1,
      score: 0.0,
      mandatory_completed: false,
    };

    store.handleServerMessage({
      type: 'scenario_update',
      payload: mockStatus,
    });
    await Promise.resolve();

    expect(store.scenarioStatus?.state).toBe('PAUSED');
    expect(store.scenarioStatus?.scenario_id).toBe('SCN-TEST');
  });

  it('renders nebu-scenario-panel with proper badge, buttons, and active faults', async () => {
    const panel = document.createElement('nebu-scenario-panel') as NebuScenarioPanel;
    document.body.appendChild(panel);

    // Initial state check
    const statusBadge = panel.querySelector('#scnStateBadge');
    expect(statusBadge).not.toBeNull();
    expect(statusBadge?.textContent).toBe('IDLE');

    // Update store with active faults
    store.setScenarioStatus({
      state: 'RUNNING',
      scenario_id: 'SCN-TEST',
      scenario_name: 'Live Fault Injection Test',
      elapsed_sec: 5.0,
      duration_limit_sec: 120.0,
      active_faults: [
        {
          id: 'FLT-TEST-01',
          target: 'TI-101',
          category: 'sensor_frozen',
          elapsed_sec: 2.0,
          duration_sec: 30.0,
        },
      ],
      active_disturbances_count: 0,
      events_total: 1,
      events_fired: 0,
      score: 0,
      mandatory_completed: false,
    });

    await Promise.resolve();

    expect(statusBadge?.textContent).toBe('RUNNING');

    // Check that active faults table contains row
    const faultsTable = panel.querySelector('#faultsTbody');
    expect(faultsTable).not.toBeNull();
    expect(faultsTable?.innerHTML).toContain('FLT-TEST-01');
    expect(faultsTable?.innerHTML).toContain('sensor_frozen');
    expect(faultsTable?.innerHTML).toContain('TI-101');
  });

  it('invokes client API endpoints for scenario actions', async () => {
    const client = new ApiClient({ baseUrl: 'http://localhost:8080' });

    await client.startScenario();
    expect(globalThis.fetch).toHaveBeenCalledWith(
      'http://localhost:8080/api/v1/scenario/start',
      expect.objectContaining({ method: 'POST' })
    );

    await client.pauseScenario();
    expect(globalThis.fetch).toHaveBeenCalledWith(
      'http://localhost:8080/api/v1/scenario/pause',
      expect.objectContaining({ method: 'POST' })
    );

    await client.injectFault({
      id: 'FLT-AD-HOC',
      target: 'PI-101',
      fault_type: { category: 'sensor_bias', offset: 10.0 },
      duration_sec: 30,
    });
    expect(globalThis.fetch).toHaveBeenCalledWith(
      'http://localhost:8080/api/v1/scenario/fault/inject',
      expect.objectContaining({ method: 'POST' })
    );

    await client.clearFault('FLT-AD-HOC');
    expect(globalThis.fetch).toHaveBeenCalledWith(
      'http://localhost:8080/api/v1/scenario/fault/clear',
      expect.objectContaining({ method: 'POST' })
    );
  });
});
