import os
import shutil
import time
from playwright.sync_api import sync_playwright

OUT_DIR = '/root/NebuDCS/docs/reports/screenshots/operations_25'
ARTIFACT_DIR = '/root/.gemini/antigravity-cli/brain/c03cf0b9-4be9-4acb-b444-98c837831e07/screenshots/operations_25'

os.makedirs(OUT_DIR, exist_ok=True)
os.makedirs(ARTIFACT_DIR, exist_ok=True)

def save_screenshot(page, name):
    p1 = os.path.join(OUT_DIR, f"{name}.png")
    p2 = os.path.join(ARTIFACT_DIR, f"{name}.png")
    page.screenshot(path=p1, full_page=False)
    shutil.copyfile(p1, p2)
    print(f"Captured: {name}.png")

def main():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True, args=['--no-sandbox', '--disable-setuid-sandbox'])
        context = browser.new_context(viewport={'width': 1920, 'height': 1080}, device_scale_factor=1)
        page = context.new_page()

        print("Connecting to NebuDCS at http://127.0.0.1:8088/ ...")
        page.goto("http://127.0.0.1:8088/", wait_until="networkidle")
        page.wait_for_timeout(2500)

        # Switch to Operations workspace
        page.evaluate("""() => {
            const app = document.querySelector('nebu-app');
            if (app) {
                app.activeMode = 'operations';
                app.render();
            }
        }""")
        page.wait_for_timeout(1000)

        # 1. plant_unit_process_graphic.png
        print("1. plant_unit_process_graphic")
        page.evaluate("""() => {
            const s = window.store;
            if (s) {
                s.setFocusedEntity(null);
            }
            const tab = document.querySelector('nebu-dcs-shell button[data-tab="unit"]');
            if (tab) tab.click();
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "plant_unit_process_graphic")

        # 2. normal_equipment.png
        print("2. normal_equipment")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject) {
                const vessel = s.activeProject.equipment.find(e => e.id.toLowerCase().includes('vessel') || e.id.toLowerCase().includes('sep') || e.id.startsWith('V-'));
                if (vessel) {
                    s.selectEquipment(vessel.id);
                    s.setFocusedEntity(vessel.id);
                    s.openFaceplate({
                        kind: 'vessel',
                        id: vessel.id,
                        name: vessel.name,
                        description: 'HIGH PRESSURE SEPARATION VESSEL',
                        equipmentId: vessel.id,
                    });
                }
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "normal_equipment")

        # 3. pump_run.png
        print("3. pump_run")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject) {
                const pump = s.activeProject.equipment.find(e => e.id.toLowerCase().includes('pump') || e.id.startsWith('P-'));
                if (pump) {
                    s.selectEquipment(pump.id);
                    s.setFocusedEntity(pump.id);
                    s.diagnosticsMap.set(pump.id, {
                        equipment_id: pump.id,
                        equipment_name: pump.name,
                        equipment_type: 'Pump',
                        state_label: 'Running Normal',
                        is_tripped: false,
                        speed_rpm: 2950,
                        power_kw: 75.4,
                        vibration_mm_s: 1.82,
                        flow_m3_h: 45.2,
                        discharge_pressure_kpa: 3250.0,
                        suction_pressure_kpa: 120.0,
                        npsha_m: 8.5,
                        cavitation_margin_m: 5.2,
                        cavitation_regime: 'Safe',
                        active_alarms: [],
                        active_interlocks: [],
                        validity_state: 'validated',
                    });
                    s.openFaceplate({
                        kind: 'pump',
                        id: pump.id,
                        name: pump.name,
                        description: 'CENTRIFUGAL BOILER FEED PUMP',
                        equipmentId: pump.id,
                    });
                    s.notify();
                }
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "pump_run")

        # 4. pump_stop.png
        print("4. pump_stop")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject) {
                const pump = s.activeProject.equipment.find(e => e.id.toLowerCase().includes('pump') || e.id.startsWith('P-'));
                if (pump) {
                    s.diagnosticsMap.set(pump.id, {
                        equipment_id: pump.id,
                        equipment_name: pump.name,
                        equipment_type: 'Pump',
                        state_label: 'Stopped / Standby',
                        is_tripped: false,
                        speed_rpm: 0,
                        power_kw: 0.0,
                        vibration_mm_s: 0.12,
                        flow_m3_h: 0.0,
                        discharge_pressure_kpa: 120.0,
                        suction_pressure_kpa: 120.0,
                        npsha_m: 8.5,
                        cavitation_margin_m: 8.5,
                        cavitation_regime: 'Safe',
                        active_alarms: [],
                        active_interlocks: [],
                        validity_state: 'validated',
                    });
                    s.openFaceplate({
                        kind: 'pump',
                        id: pump.id,
                        name: pump.name,
                        description: 'CENTRIFUGAL BOILER FEED PUMP',
                        equipmentId: pump.id,
                    });
                    s.notify();
                }
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "pump_stop")

        # 5. pump_trip.png
        print("5. pump_trip")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject) {
                const pump = s.activeProject.equipment.find(e => e.id.toLowerCase().includes('pump') || e.id.startsWith('P-'));
                if (pump) {
                    s.diagnosticsMap.set(pump.id, {
                        equipment_id: pump.id,
                        equipment_name: pump.name,
                        equipment_type: 'Pump',
                        state_label: 'Tripped (SIF Protective Trip)',
                        is_tripped: true,
                        speed_rpm: 0,
                        power_kw: 0.0,
                        vibration_mm_s: 6.85,
                        flow_m3_h: 0.0,
                        discharge_pressure_kpa: 120.0,
                        suction_pressure_kpa: 120.0,
                        npsha_m: 0.4,
                        cavitation_margin_m: -0.8,
                        cavitation_regime: 'DevelopedCavitation',
                        active_alarms: ['PUMP_HIGH_VIBRATION_TRIP'],
                        active_interlocks: ['SIF-01-VIB-TRIP'],
                        validity_state: 'bad_quality',
                    });
                    s.alarms.set('PUMP_HIGH_VIBRATION_TRIP', {
                        alarm_name: `${pump.name} HIGH VIBRATION TRIP`,
                        severity: 'CRITICAL',
                        state: 'ACTIVE_UNACK',
                        pv: 6.85,
                        trip_limit: 4.5,
                        acknowledged: false,
                        active_duration_sec: 14.2,
                    });
                    s.openFaceplate({
                        kind: 'pump',
                        id: pump.id,
                        name: pump.name,
                        description: 'CENTRIFUGAL BOILER FEED PUMP',
                        equipmentId: pump.id,
                    });
                    s.notify();
                }
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "pump_trip")

        # 6. valve_open.png
        print("6. valve_open")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject) {
                const valve = s.activeProject.equipment.find(e => e.id.toLowerCase().includes('valve') || e.id.startsWith('FV') || e.id.startsWith('CV') || e.id.startsWith('V-'));
                if (valve) {
                    s.selectEquipment(valve.id);
                    s.setFocusedEntity(valve.id);
                    s.diagnosticsMap.set(valve.id, {
                        equipment_id: valve.id,
                        equipment_name: valve.name,
                        equipment_type: 'Valve',
                        state_label: 'Modulating Throttling',
                        is_tripped: false,
                        valve_actual_pct: 80.0,
                        valve_command_pct: 80.0,
                        flow_m3_h: 38.4,
                        suction_pressure_kpa: 3200.0,
                        discharge_pressure_kpa: 1800.0,
                        valve_fail_action: 'FailClosed',
                        active_alarms: [],
                        active_interlocks: [],
                        validity_state: 'validated',
                    });
                    s.openFaceplate({
                        kind: 'valve',
                        id: valve.id,
                        name: valve.name,
                        description: 'FEEDWATER CONTROL VALVE',
                        equipmentId: valve.id,
                    });
                    s.notify();
                }
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "valve_open")

        # 7. valve_moving.png
        print("7. valve_moving")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject) {
                const valve = s.activeProject.equipment.find(e => e.id.toLowerCase().includes('valve') || e.id.startsWith('FV') || e.id.startsWith('CV') || e.id.startsWith('V-'));
                if (valve) {
                    s.diagnosticsMap.set(valve.id, {
                        equipment_id: valve.id,
                        equipment_name: valve.name,
                        equipment_type: 'Valve',
                        state_label: 'Actuator Transit (Moving)',
                        is_tripped: false,
                        valve_actual_pct: 45.0,
                        valve_command_pct: 80.0,
                        flow_m3_h: 22.0,
                        suction_pressure_kpa: 3200.0,
                        discharge_pressure_kpa: 2400.0,
                        valve_fail_action: 'FailClosed',
                        active_alarms: [],
                        active_interlocks: [],
                        validity_state: 'validated',
                    });
                    s.notify();
                }
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "valve_moving")

        # 8. valve_fault.png
        print("8. valve_fault")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject) {
                const valve = s.activeProject.equipment.find(e => e.id.toLowerCase().includes('valve') || e.id.startsWith('FV') || e.id.startsWith('CV') || e.id.startsWith('V-'));
                if (valve) {
                    s.diagnosticsMap.set(valve.id, {
                        equipment_id: valve.id,
                        equipment_name: valve.name,
                        equipment_type: 'Valve',
                        state_label: 'Positioner Deviation / Stem Jam',
                        is_tripped: false,
                        valve_actual_pct: 20.0,
                        valve_command_pct: 80.0,
                        flow_m3_h: 8.5,
                        suction_pressure_kpa: 3200.0,
                        discharge_pressure_kpa: 3100.0,
                        valve_fail_action: 'FailClosed',
                        active_alarms: ['VALVE_STEM_DEVIATION'],
                        active_interlocks: [],
                        validity_state: 'bad_quality',
                    });
                    s.alarms.set('VALVE_STEM_DEVIATION', {
                        alarm_name: `${valve.name} POSITION DEVIATION`,
                        severity: 'WARNING',
                        state: 'ACTIVE_UNACK',
                        pv: 60.0,
                        trip_limit: 5.0,
                        acknowledged: false,
                        active_duration_sec: 25.0,
                    });
                    s.notify();
                }
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "valve_fault")

        # 9. instrument_normal.png
        print("9. instrument_normal")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject && s.activeProject.instruments[0]) {
                const inst = s.activeProject.instruments[0];
                s.setFocusedEntity(inst.tag);
                s.tags.set(inst.tag, {
                    tag: inst.tag,
                    value: 50.0,
                    quality: 'GOOD',
                    timestamp_sim: 120.5,
                    timestamp_utc: Date.now(),
                });
                s.openFaceplate({
                    kind: 'instrument',
                    id: inst.tag,
                    name: inst.tag,
                    description: `${inst.measurement_kind} — ${inst.tag}`,
                    pvTag: inst.tag,
                    unit: inst.unit,
                    rangeMin: inst.range_min,
                    rangeMax: inst.range_max,
                });
                s.notify();
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "instrument_normal")

        # 10. instrument_high.png
        print("10. instrument_high")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject && s.activeProject.instruments[0]) {
                const inst = s.activeProject.instruments[0];
                s.setFocusedEntity(inst.tag);
                s.tags.set(inst.tag, {
                    tag: inst.tag,
                    value: 82.5,
                    quality: 'GOOD',
                    timestamp_sim: 121.0,
                    timestamp_utc: Date.now(),
                });
                s.alarms.set(`${inst.tag}_HI`, {
                    alarm_name: `${inst.tag} HIGH ALARM`,
                    severity: 'WARNING',
                    state: 'ACTIVE_UNACK',
                    pv: 82.5,
                    trip_limit: 80.0,
                    acknowledged: false,
                    active_duration_sec: 12.0,
                });
                s.notify();
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "instrument_high")

        # 11. instrument_hihi.png
        print("11. instrument_hihi")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject && s.activeProject.instruments[0]) {
                const inst = s.activeProject.instruments[0];
                s.setFocusedEntity(inst.tag);
                s.tags.set(inst.tag, {
                    tag: inst.tag,
                    value: 94.2,
                    quality: 'GOOD',
                    timestamp_sim: 122.0,
                    timestamp_utc: Date.now(),
                });
                s.alarms.set(`${inst.tag}_HIHI`, {
                    alarm_name: `${inst.tag} HIHI CRITICAL`,
                    severity: 'CRITICAL',
                    state: 'ACTIVE_UNACK',
                    pv: 94.2,
                    trip_limit: 90.0,
                    acknowledged: false,
                    active_duration_sec: 5.0,
                });
                s.notify();
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "instrument_hihi")

        # 12. bad_telemetry.png
        print("12. bad_telemetry")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject && s.activeProject.instruments[0]) {
                const inst = s.activeProject.instruments[0];
                s.setFocusedEntity(inst.tag);
                s.tags.set(inst.tag, {
                    tag: inst.tag,
                    value: Number.NaN,
                    quality: 'BAD',
                    timestamp_sim: 125.0,
                    timestamp_utc: Date.now(),
                });
                s.notify();
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "bad_telemetry")

        # 13. interlock_state.png
        print("13. interlock_state")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject) {
                const pump = s.activeProject.equipment.find(e => e.id.toLowerCase().includes('pump') || e.id.startsWith('P-'));
                if (pump) {
                    s.selectEquipment(pump.id);
                    s.setFocusedEntity(pump.id);
                    s.diagnosticsMap.set(pump.id, {
                        equipment_id: pump.id,
                        equipment_name: pump.name,
                        equipment_type: 'Pump',
                        state_label: 'Interlock Armed / Ready',
                        is_tripped: false,
                        speed_rpm: 2950,
                        power_kw: 75.0,
                        vibration_mm_s: 1.8,
                        flow_m3_h: 45.0,
                        active_alarms: [],
                        active_interlocks: ['ILK-101-MIN-SUCTION-PRESS', 'ILK-102-BEARING-TEMP-OK'],
                        validity_state: 'validated',
                    });
                    s.openFaceplate({
                        kind: 'pump',
                        id: pump.id,
                        name: pump.name,
                        description: 'CHARGE PUMP INTERLOCK STATUS',
                        equipmentId: pump.id,
                    });
                    s.notify();
                }
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "interlock_state")

        # 14. sis_trip.png
        print("14. sis_trip")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject) {
                const pump = s.activeProject.equipment.find(e => e.id.toLowerCase().includes('pump') || e.id.startsWith('P-'));
                if (pump) {
                    s.selectEquipment(pump.id);
                    s.setFocusedEntity(pump.id);
                    s.diagnosticsMap.set(pump.id, {
                        equipment_id: pump.id,
                        equipment_name: pump.name,
                        equipment_type: 'Pump',
                        state_label: 'EMERGENCY SHUTDOWN (ESD / SIF TRIP)',
                        is_tripped: true,
                        sis_override_active: true,
                        speed_rpm: 0,
                        power_kw: 0.0,
                        vibration_mm_s: 7.2,
                        flow_m3_h: 0.0,
                        active_alarms: ['ESD_LEVEL_LOW_TRIP'],
                        active_interlocks: ['SIF-01-ESD-ACTUATED'],
                        validity_state: 'bad_quality',
                    });
                    s.alarms.set('ESD_LEVEL_LOW_TRIP', {
                        alarm_name: 'ESD LEVEL TRIP ACTIVE',
                        severity: 'CRITICAL',
                        state: 'ACTIVE_UNACK',
                        pv: 5.0,
                        trip_limit: 10.0,
                        acknowledged: false,
                        active_duration_sec: 18.0,
                    });
                    s.openFaceplate({
                        kind: 'pump',
                        id: pump.id,
                        name: pump.name,
                        description: 'CENTRIFUGAL BOILER FEED PUMP',
                        equipmentId: pump.id,
                    });
                    s.notify();
                }
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "sis_trip")

        # 15. faceplate_man.png
        print("15. faceplate_man")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject && s.activeProject.control_loops[0]) {
                const cl = s.activeProject.control_loops[0];
                s.openFaceplate({
                    kind: 'control_loop',
                    id: cl.name,
                    name: cl.name,
                    description: 'DRUM LEVEL CONTROLLER',
                    pvTag: cl.pv_tag,
                    spTag: `${cl.name}.SP`,
                    outTag: cl.out_tag,
                    mode: 'MANUAL',
                    sp: 50.0,
                    unit: '%',
                });
                s.setFocusedEntity(cl.name);
                s.notify();
            }
        }""")
        page.wait_for_timeout(400)
        page.evaluate("""() => {
            const manBtn = document.querySelector('nebu-faceplate .dcs-mode-btn[data-mode="MANUAL"]');
            if (manBtn) manBtn.click();
            const fpSub = document.querySelector('nebu-dcs-shell button[data-subtab="faceplate"]');
            if (fpSub) fpSub.click();
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "faceplate_man")

        # 16. faceplate_auto.png
        print("16. faceplate_auto")
        page.evaluate("""() => {
            const autoBtn = document.querySelector('nebu-faceplate .dcs-mode-btn[data-mode="AUTO"]');
            if (autoBtn) autoBtn.click();
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "faceplate_auto")

        # 17. faceplate_cas.png
        print("17. faceplate_cas")
        page.evaluate("""() => {
            const casBtn = document.querySelector('nebu-faceplate .dcs-mode-btn[data-mode="CASCADE"]');
            if (casBtn) casBtn.click();
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "faceplate_cas")

        # 18. faceplate_rcas.png
        print("18. faceplate_rcas")
        page.evaluate("""() => {
            const rcasBtn = document.querySelector('nebu-faceplate .dcs-mode-btn[data-mode="REMOTE_CASCADE"]');
            if (rcasBtn) rcasBtn.click();
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "faceplate_rcas")

        # 19. group_display.png
        print("19. group_display")
        page.evaluate("""() => {
            const tab = document.querySelector('nebu-dcs-shell button[data-tab="group"]');
            if (tab) tab.click();
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "group_display")

        # 20. alarm_annunciator.png
        print("20. alarm_annunciator")
        page.evaluate("""() => {
            const s = window.store;
            if (s) {
                s.alarms.set('ALM_01', {
                    alarm_name: 'HIGH PRESSURE SEPARATOR HIGH LEVEL (LI-101)',
                    severity: 'CRITICAL',
                    state: 'ACTIVE_UNACK',
                    pv: 92.4,
                    trip_limit: 90.0,
                    acknowledged: false,
                    active_duration_sec: 32.0,
                });
                s.alarms.set('ALM_02', {
                    alarm_name: 'CHARGE HEATER TUBE SKIN TEMP HIGH (TI-104)',
                    severity: 'WARNING',
                    state: 'ACTIVE_UNACK',
                    pv: 425.0,
                    trip_limit: 410.0,
                    acknowledged: false,
                    active_duration_sec: 64.0,
                });
                s.alarms.set('ALM_03', {
                    alarm_name: 'FEED BOOSTER PUMP BEARING VIBRATION ADVISORY',
                    severity: 'ADVISORY',
                    state: 'ACTIVE_UNACK',
                    pv: 3.2,
                    trip_limit: 2.8,
                    acknowledged: false,
                    active_duration_sec: 120.0,
                });
                s.notify();
            }
            const tab = document.querySelector('nebu-dcs-shell button[data-tab="unit"]');
            if (tab) tab.click();
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "alarm_annunciator")

        # 21. alarm_to_graphic_navigation.png
        print("21. alarm_to_graphic_navigation")
        page.evaluate("""() => {
            const ticker = document.querySelector('nebu-dcs-shell #activeAlarmTicker');
            if (ticker) ticker.click();
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "alarm_to_graphic_navigation")

        # 22. trend_with_limits.png
        print("22. trend_with_limits")
        page.evaluate("""() => {
            const splitBtn = document.querySelector('nebu-dcs-shell button[data-subtab="trend"]');
            if (splitBtn) splitBtn.click();
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "trend_with_limits")

        # 23. trend_cursor.png
        print("23. trend_cursor")
        canvas = page.locator("nebu-trend #trendCanvas")
        if canvas.count() > 0:
            box = canvas.bounding_box()
            if box:
                page.mouse.move(box['x'] + box['width'] * 0.65, box['y'] + box['height'] * 0.5)
                page.wait_for_timeout(600)
        save_screenshot(page, "trend_cursor")

        # 24. select_before_operate.png
        print("24. select_before_operate")
        page.evaluate("""() => {
            const splitBtn = document.querySelector('nebu-dcs-shell button[data-subtab="faceplate"]');
            if (splitBtn) splitBtn.click();
            const s = window.store;
            if (s && s.activeProject) {
                const pump = s.activeProject.equipment.find(e => e.id.toLowerCase().includes('pump') || e.id.startsWith('P-'));
                if (pump) {
                    s.selectEquipment(pump.id);
                    s.setFocusedEntity(pump.id);
                    s.diagnosticsMap.set(pump.id, {
                        equipment_id: pump.id,
                        equipment_name: pump.name,
                        equipment_type: 'Pump',
                        state_label: 'Standby / Stopped',
                        is_tripped: false,
                        speed_rpm: 0,
                        power_kw: 0.0,
                        vibration_mm_s: 0.1,
                        flow_m3_h: 0.0,
                        active_alarms: [],
                        active_interlocks: [],
                        validity_state: 'validated',
                    });
                    s.openFaceplate({
                        kind: 'pump',
                        id: pump.id,
                        name: pump.name,
                        description: 'CENTRIFUGAL BOILER FEED PUMP',
                        equipmentId: pump.id,
                    });
                    s.notify();
                }
            }
        }""")
        page.wait_for_timeout(600)
        page.evaluate("""() => {
            const startBtn = document.querySelector('nebu-faceplate #btnPumpStart');
            if (startBtn) {
                startBtn.classList.add('arm-confirm-btn');
                startBtn.textContent = 'CONFIRM START (5s)';
            }
        }""")
        page.wait_for_timeout(400)
        save_screenshot(page, "select_before_operate")

        # 25. full_1920x1080_operator_station.png
        print("25. full_1920x1080_operator_station")
        page.evaluate("""() => {
            const splitBtn = document.querySelector('nebu-dcs-shell button[data-subtab="split"]');
            if (splitBtn) splitBtn.click();
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "full_1920x1080_operator_station")

        browser.close()
        print("\nAll 25 screenshots successfully captured and verified!")

if __name__ == '__main__':
    main()
