import { describe, it, expect, beforeEach, beforeAll } from 'vitest';
import { studioStore } from '../src/state/studio-store';
import { NebuAdvancedControl } from '../src/components/nebu-advanced-control';
import { ProjectDetailDto } from '../src/protocol/types';

describe('NebuAdvancedControl Web Component', () => {
  beforeAll(() => {
    if (!window.customElements.get('nebu-advanced-control')) {
      window.customElements.define('nebu-advanced-control', NebuAdvancedControl);
    }
  });

  beforeEach(() => {
    document.body.innerHTML = '';
  });

  it('renders placeholder when no project is loaded', () => {
    (studioStore as any).project = null;
    const el = document.createElement('nebu-advanced-control') as NebuAdvancedControl;
    document.body.appendChild(el);

    expect(el.innerHTML).toContain('No project loaded');
  });

  it('renders strategies table with badges when project is loaded', () => {
    const mockProject: ProjectDetailDto = {
      id: 'proj-apc-test',
      name: 'APC Test Plant',
      schema_version: '1.0.0',
      description: 'Test project for Stage 5 APC',
      author: 'NebuDCS',
      created: new Date().toISOString(),
      modified: new Date().toISOString(),
      equipment: [],
      streams: [],
      instruments: [],
      control_loops: [],
      alarms: [],
      interlocks: [],
      advanced_control: [
        {
          id: 'strat-cas-1',
          name: 'Column Level-Flow Cascade',
          strategy_type: 'Cascade',
          description: 'LIC-101 to FIC-101 master-slave',
          enabled: true,
          parameters: {
            primary_pv: 'LIC_101_PV',
            secondary_pv: 'FIC_101_PV',
            target_actuator: 'FV-101',
          },
        },
        {
          id: 'strat-surge-1',
          name: 'Main Compressor Anti-Surge',
          strategy_type: 'AntiSurge',
          description: 'SLL envelope protection',
          enabled: true,
          parameters: {
            flow_tag: 'FT_601_FLOW',
            p_suction_tag: 'PT_601_SUC',
            p_discharge_tag: 'PT_601_DIS',
            target_actuator: 'FV-601-RECYCLE',
          },
        },
      ],
    };

    (studioStore as any).project = mockProject;
    const el = document.createElement('nebu-advanced-control') as NebuAdvancedControl;
    document.body.appendChild(el);

    expect(el.innerHTML).toContain('Column Level-Flow Cascade');
    expect(el.innerHTML).toContain('Main Compressor Anti-Surge');
    expect(el.innerHTML).toContain('Cascade');
    expect(el.innerHTML).toContain('AntiSurge');
    expect(el.innerHTML).toContain('2 Strategies');
  });

  it('supports filtering by strategy type', () => {
    const mockProject: ProjectDetailDto = {
      id: 'proj-filter-test',
      name: 'Filter Test',
      schema_version: '1.0.0',
      description: '',
      author: 'NebuDCS',
      created: '',
      modified: '',
      equipment: [],
      streams: [],
      instruments: [],
      control_loops: [],
      alarms: [],
      interlocks: [],
      advanced_control: [
        {
          id: 'strat-ratio-1',
          name: 'Fuel-Air Ratio',
          strategy_type: 'Ratio',
          description: '',
          enabled: true,
          parameters: { wild_pv: 'AIR_FLOW', controlled_pv: 'GAS_FLOW' },
        },
        {
          id: 'strat-ff-1',
          name: 'Feedforward Disturbance Rejection',
          strategy_type: 'FeedForward',
          description: '',
          enabled: true,
          parameters: { pv_tag: 'TEMP', disturbance_tag: 'FEED_FLOW' },
        },
      ],
    };

    (studioStore as any).project = mockProject;
    const el = document.createElement('nebu-advanced-control') as NebuAdvancedControl;
    document.body.appendChild(el);

    expect(el.innerHTML).toContain('Fuel-Air Ratio');
    expect(el.innerHTML).toContain('Feedforward Disturbance Rejection');

    // Simulate selecting Ratio filter
    const select = el.querySelector('.filter-type-select') as HTMLSelectElement;
    select.value = 'RATIO';
    select.dispatchEvent(new Event('change'));

    expect(el.innerHTML).toContain('Fuel-Air Ratio');
    expect(el.innerHTML).not.toContain('Feedforward Disturbance Rejection');
  });

  it('toggles strategy enabled state', () => {
    const mockProject: ProjectDetailDto = {
      id: 'proj-toggle-test',
      name: 'Toggle Test',
      schema_version: '1.0.0',
      description: '',
      author: 'NebuDCS',
      created: '',
      modified: '',
      equipment: [],
      streams: [],
      instruments: [],
      control_loops: [],
      alarms: [],
      interlocks: [],
      advanced_control: [
        {
          id: 'strat-toggle-1',
          name: 'Toggleable Strategy',
          strategy_type: 'Override',
          description: '',
          enabled: true,
          parameters: {},
        },
      ],
    };

    (studioStore as any).project = mockProject;
    const el = document.createElement('nebu-advanced-control') as NebuAdvancedControl;
    document.body.appendChild(el);

    const toggleBtn = el.querySelector('.btn-toggle-strat') as HTMLButtonElement;
    expect(toggleBtn).not.toBeNull();
    expect(toggleBtn.textContent?.trim()).toBe('Disable');

    toggleBtn.click();
    expect(studioStore.getProject()!.advanced_control![0].enabled).toBe(false);
  });
});
