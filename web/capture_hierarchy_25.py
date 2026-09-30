import os
import shutil
import time
from playwright.sync_api import sync_playwright

OUT_DIR = '/root/NebuDCS/docs/reports/screenshots/hierarchy_25'
OUT_DIR_LEGACY = '/root/NebuDCS/docs/screenshots/hierarchy_25'
ARTIFACT_DIR = '/root/.gemini/antigravity-cli/brain/c03cf0b9-4be9-4acb-b444-98c837831e07/screenshots/hierarchy_25'

os.makedirs(OUT_DIR, exist_ok=True)
os.makedirs(OUT_DIR_LEGACY, exist_ok=True)
os.makedirs(ARTIFACT_DIR, exist_ok=True)

def save_screenshot(page, name):
    p1 = os.path.join(OUT_DIR, f"{name}.png")
    p2 = os.path.join(OUT_DIR_LEGACY, f"{name}.png")
    p3 = os.path.join(ARTIFACT_DIR, f"{name}.png")
    page.screenshot(path=p1, full_page=False)
    shutil.copyfile(p1, p2)
    shutil.copyfile(p1, p3)
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

        # 01. Plant Overview (Situational awareness grid)
        print("01. Plant Overview")
        page.evaluate("""() => {
            const s = window.store;
            if (s) {
                s.navigateDisplay('disp-plant-overview');
                s.notify();
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "01_plant_overview")

        # 02. Area Overview (Area 01 system cards)
        print("02. Area Overview")
        page.evaluate("""() => {
            const s = window.store;
            if (s) {
                s.navigateDisplay('disp-area-01');
                s.notify();
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "02_area_overview")

        # 03. HP System (High pressure system scoped flowsheet)
        print("03. HP System")
        page.evaluate("""() => {
            const s = window.store;
            if (s) {
                s.navigateDisplay('disp-hp-system');
                s.notify();
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "03_hp_system")

        # 04. LP System (Low pressure recovery flowsheet)
        print("04. LP System")
        page.evaluate("""() => {
            const s = window.store;
            if (s) {
                s.navigateDisplay('disp-lp-system');
                s.notify();
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "04_lp_system")

        # 05. Unit Process Graphic (72/28 zero-scroll flowsheet)
        print("05. Unit Process Graphic")
        page.evaluate("""() => {
            const s = window.store;
            if (s) {
                s.navigateDisplay('disp-unit-01');
                s.setFocusedEntity(null);
                s.notify();
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "05_unit_process_graphic")

        # 06. Large Process Graphic (Wide flowsheet with minimap)
        print("06. Large Process Graphic")
        page.evaluate("""() => {
            const s = window.store;
            if (s) {
                s.navigateDisplay('disp-large-process');
                s.notify();
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "06_large_process_graphic")

        # 07. Selected Equipment (Equipment highlighted with cyan focus ring)
        print("07. Selected Equipment")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject) {
                s.navigateDisplay('disp-unit-01');
                const vessel = s.activeProject.equipment.find(e => e.id.toLowerCase().includes('vessel') || e.id.startsWith('V-')) || s.activeProject.equipment[0];
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
                    s.notify();
                }
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "07_selected_equipment")

        # 08. Selected Process Line (Selected stream line with cyan glow and docked stream card)
        print("08. Selected Process Line")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject && s.activeProject.streams.length > 0) {
                const stream = s.activeProject.streams[0];
                s.selectStream(stream.name);
                s.notify();
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "08_selected_process_line")

        # 09. TO Equipment navigation (Clicking TO continuation badge)
        print("09. TO Equipment navigation")
        page.evaluate("""() => {
            const btnTo = document.querySelector('.stream-detail-card .btn-jump-to') || document.querySelector('.stream-connector-badge');
            if (btnTo) {
                btnTo.click();
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "09_to_equipment_navigation")

        # 10. FROM Equipment navigation (Clicking FROM continuation badge)
        print("10. FROM Equipment navigation")
        page.evaluate("""() => {
            const btnFrom = document.querySelector('.stream-detail-card .btn-jump-from');
            if (btnFrom) {
                btnFrom.click();
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "10_from_equipment_navigation")

        # 11. Cross-display transition (Display transition with target focused)
        print("11. Cross-display transition")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject) {
                s.navigateDisplay('disp-hp-system');
                const p = s.activeProject.equipment.find(e => e.id.startsWith('P-') || e.id.toLowerCase().includes('pump')) || s.activeProject.equipment[0];
                s.setFocusedEntity(p.id);
                s.selectEquipment(p.id);
                s.notify();
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "11_cross_display_transition")

        # 12. Pump faceplate (Pump faceplate with Select-Before-Operate)
        print("12. Pump faceplate")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject) {
                s.navigateDisplay('disp-unit-01');
                const pump = s.activeProject.equipment.find(e => e.id.toLowerCase().includes('pump') || e.id.startsWith('P-')) || s.activeProject.equipment[0];
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
        save_screenshot(page, "12_pump_faceplate")

        # 13. Valve faceplate (Valve faceplate with CMD/ACT stroke nudges)
        print("13. Valve faceplate")
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
        save_screenshot(page, "13_valve_faceplate")

        # 14. Controller faceplate (PID faceplate with MAN/AUTO/CAS/RCAS)
        print("14. Controller faceplate")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject) {
                const loop = s.activeProject.control_loops ? s.activeProject.control_loops[0] : null;
                if (loop) {
                    s.openFaceplate({
                        kind: 'controller',
                        id: loop.name,
                        name: loop.name,
                        description: 'PRESSURE REGULATORY LOOP',
                        pv: 54.2,
                        sp: 50.0,
                        op: 48.0,
                        mode: 'AUTO',
                        pvTag: loop.pv_tag,
                        spTag: loop.sp_tag,
                        opTag: loop.op_tag,
                    });
                    s.notify();
                }
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "14_controller_faceplate")

        # 15. Point Detail (Point detail view)
        print("15. Point Detail")
        page.evaluate("""() => {
            const s = window.store;
            if (s) {
                s.navigateDisplay('disp-point-detail');
                s.notify();
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "15_point_detail")

        # 16. Group Display (8-loop regulatory matrix)
        print("16. Group Display")
        page.evaluate("""() => {
            const s = window.store;
            if (s) {
                s.navigateDisplay('disp-group-01');
                s.notify();
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "16_group_display")

        # 17. Trend (Realtime trend with dashed limits)
        print("17. Trend")
        page.evaluate("""() => {
            const s = window.store;
            if (s) {
                s.navigateDisplay('disp-trend-01');
                s.notify();
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "17_trend")

        # 18. Alarm annunciator (Annunciator ribbon with active alarms)
        print("18. Alarm annunciator")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject) {
                s.navigateDisplay('disp-alarms');
                s.alarms.set('ALM-CRIT-01', {
                    alarm_name: 'REACTION FURNACE HIGH HIGH PRESSURE TRIP',
                    severity: 'CRITICAL',
                    state: 'ACTIVE_UNACK',
                    pv: 98.4,
                    trip_limit: 85.0,
                    acknowledged: false,
                    active_duration_sec: 25.1,
                });
                s.alarms.set('ALM-WARN-02', {
                    alarm_name: 'SEPARATOR DRUM LEVEL HIGH WARNING',
                    severity: 'WARNING',
                    state: 'ACTIVE_UNACK',
                    pv: 82.5,
                    trip_limit: 75.0,
                    acknowledged: false,
                    active_duration_sec: 14.8,
                });
                s.alarms.set('ALM-ADV-03', {
                    alarm_name: 'RECYCLE COMPRESSOR SUCTION FLOW ADVISORY',
                    severity: 'ADVISORY',
                    state: 'ACTIVE_UNACK',
                    pv: 110.2,
                    trip_limit: 120.0,
                    acknowledged: false,
                    active_duration_sec: 8.2,
                });
                s.notify();
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "18_alarm_annunciator")

        # 19. Alarm -> source display (Clicking alarm in ticker jumps to source display)
        print("19. Alarm -> source display")
        page.evaluate("""() => {
            const ticker = document.querySelector('#activeAlarmTicker');
            if (ticker) {
                ticker.click();
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "19_alarm_to_source_display")

        # 20. Alarm -> equipment (Source equipment highlighted with pulsing alarm halo)
        print("20. Alarm -> equipment")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject) {
                s.navigateDisplay('disp-unit-01');
                const vessel = s.activeProject.equipment[0];
                s.selectEquipment(vessel.id);
                s.setFocusedEntity(vessel.id);
                s.alarms.set('V-101-CRIT', {
                    alarm_name: `${vessel.name} CRITICAL OVERPRESSURE`,
                    severity: 'CRITICAL',
                    state: 'ACTIVE_UNACK',
                    pv: 98.0,
                    trip_limit: 85.0,
                    acknowledged: false,
                    active_duration_sec: 32.0,
                });
                s.notify();
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "20_alarm_to_equipment")

        # 21. Interlock (Docked interlock cause-and-effect panel)
        print("21. Interlock")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject) {
                const pump = s.activeProject.equipment.find(e => e.id.toLowerCase().includes('pump') || e.id.startsWith('P-')) || s.activeProject.equipment[0];
                s.selectEquipment(pump.id);
                s.setFocusedEntity(pump.id);
                s.diagnosticsMap.set(pump.id, {
                    equipment_id: pump.id,
                    equipment_name: pump.name,
                    equipment_type: 'Pump',
                    state_label: 'Interlocked Trip Armed',
                    is_tripped: false,
                    speed_rpm: 2950,
                    power_kw: 62.0,
                    vibration_mm_s: 2.1,
                    flow_m3_h: 42.0,
                    discharge_pressure_kpa: 3100.0,
                    suction_pressure_kpa: 150.0,
                    npsha_m: 7.2,
                    cavitation_margin_m: 4.5,
                    cavitation_regime: 'Safe',
                    active_alarms: [],
                    active_interlocks: ['ILK-01-SUCT-MIN', 'SIF-02-OVERPRESSURE'],
                    validity_state: 'validated',
                });
                s.openFaceplate({
                    kind: 'pump',
                    id: pump.id,
                    name: pump.name,
                    description: 'CENTRIFUGAL FEED PUMP WITH ACTIVE INTERLOCK',
                    equipmentId: pump.id,
                });
                s.notify();
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "21_interlock")

        # 22. SIS trip (ESD trip overlay banner)
        print("22. SIS trip")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject) {
                const pump = s.activeProject.equipment.find(e => e.id.toLowerCase().includes('pump') || e.id.startsWith('P-')) || s.activeProject.equipment[0];
                s.diagnosticsMap.set(pump.id, {
                    equipment_id: pump.id,
                    equipment_name: pump.name,
                    equipment_type: 'Pump',
                    state_label: 'Tripped (SIF-101 ESD ACTIVE)',
                    is_tripped: true,
                    speed_rpm: 0,
                    power_kw: 0.0,
                    vibration_mm_s: 7.4,
                    flow_m3_h: 0.0,
                    discharge_pressure_kpa: 120.0,
                    suction_pressure_kpa: 120.0,
                    npsha_m: 0.2,
                    cavitation_margin_m: -1.2,
                    cavitation_regime: 'DevelopedCavitation',
                    active_alarms: ['PUMP_HIGH_VIBRATION_TRIP', 'ESD_SYSTEM_TRIP'],
                    active_interlocks: ['SIF-01-VIB-TRIP', 'ESD-LOOP-99'],
                    validity_state: 'bad_quality',
                });
                s.alarms.set('ESD_SYSTEM_TRIP', {
                    alarm_name: 'EMERGENCY SHUTDOWN ESD-01 INITIATED',
                    severity: 'CRITICAL',
                    state: 'ACTIVE_UNACK',
                    pv: 1.0,
                    trip_limit: 0.0,
                    acknowledged: false,
                    active_duration_sec: 45.2,
                });
                s.openFaceplate({
                    kind: 'pump',
                    id: pump.id,
                    name: pump.name,
                    description: 'TRIPPED BOILER FEED PUMP',
                    equipmentId: pump.id,
                });
                s.notify();
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "22_sis_trip")

        # 23. Bad telemetry (ISA-101 [!] BAD quality indicator)
        print("23. Bad telemetry")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject && s.activeProject.instruments.length > 0) {
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
        save_screenshot(page, "23_bad_telemetry")

        # 24. Display navigation (Display navigation strip and toolbar)
        print("24. Display navigation")
        page.evaluate("""() => {
            const s = window.store;
            if (s) {
                s.navigateDisplay('disp-plant-overview');
                s.notify();
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "24_display_navigation")

        # 25. Full DCS operator station (Complete 1920×1080 station view)
        print("25. Full DCS operator station")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject) {
                s.navigateDisplay('disp-unit-01');
                const pump = s.activeProject.equipment.find(e => e.id.toLowerCase().includes('pump') || e.id.startsWith('P-')) || s.activeProject.equipment[0];
                s.selectEquipment(pump.id);
                s.setFocusedEntity(pump.id);
                s.openFaceplate({
                    kind: 'pump',
                    id: pump.id,
                    name: pump.name,
                    description: 'CENTRIFUGAL MAIN PROCESS FEED PUMP',
                    equipmentId: pump.id,
                });
                s.notify();
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "25_full_dcs_operator_station")

        # 26. Responsive 2560x1440
        print("26. Responsive 2560x1440")
        page.set_viewport_size({'width': 2560, 'height': 1440})
        page.wait_for_timeout(600)
        save_screenshot(page, "26_responsive_2560x1440")

        # 27. Responsive 3840x2160
        print("27. Responsive 3840x2160")
        page.set_viewport_size({'width': 3840, 'height': 2160})
        page.wait_for_timeout(600)
        save_screenshot(page, "27_responsive_3840x2160")

        browser.close()
        print("All 25 + 2 responsive screenshots captured successfully!")

if __name__ == '__main__':
    main()
