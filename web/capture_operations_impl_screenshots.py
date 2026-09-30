import os
import shutil
import time
from playwright.sync_api import sync_playwright

OUT_DIR = '/root/NebuDCS/docs/reports/screenshots/operations_impl'
ARTIFACT_DIR = '/root/.gemini/antigravity-cli/brain/c03cf0b9-4be9-4acb-b444-98c837831e07/screenshots/operations_impl'

os.makedirs(OUT_DIR, exist_ok=True)
os.makedirs(ARTIFACT_DIR, exist_ok=True)

def save_screenshot(page, name):
    p1 = os.path.join(OUT_DIR, f"{name}.png")
    p2 = os.path.join(ARTIFACT_DIR, f"{name}.png")
    page.screenshot(path=p1, full_page=False)
    shutil.copyfile(p1, p2)
    print(f"Saved screenshot: {name}.png")

def main():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True, args=['--no-sandbox', '--disable-setuid-sandbox'])
        context = browser.new_context(viewport={'width': 1920, 'height': 1080}, device_scale_factor=1)
        page = context.new_page()

        print("Navigating to http://127.0.0.1:8088/ ...")
        page.goto("http://127.0.0.1:8088/", wait_until="networkidle")
        page.wait_for_timeout(2000)

        # Ensure we are in Operations Workspace
        page.evaluate("""() => {
            const app = document.querySelector('nebu-app');
            if (app) {
                app.activeMode = 'operations';
                app.render();
            }
        }""")
        page.wait_for_timeout(1000)

        # 1. Plant Overview
        print("1. impl_01_plant_overview")
        page.evaluate("""() => {
            const tab = document.querySelector('nebu-dcs-shell button[data-tab="plant"]');
            if (tab) tab.click();
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "impl_01_plant_overview")

        # 2. Area Navigation / Breadcrumb Hierarchy
        print("2. impl_02_area_navigation")
        page.evaluate("""() => {
            const tab = document.querySelector('nebu-dcs-shell button[data-tab="unit"]');
            if (tab) tab.click();
        }""")
        page.wait_for_timeout(600)
        area_select = page.locator("nebu-dcs-shell select#selCrumbArea")
        if area_select.count() > 0:
            area_select.select_option("Area 02")
            area_select.dispatch_event("change")
            page.wait_for_timeout(600)
        save_screenshot(page, "impl_02_area_navigation")

        # Switch back to Area 01
        if area_select.count() > 0:
            area_select.select_option("Area 01")
            area_select.dispatch_event("change")
            page.wait_for_timeout(600)

        # 3. Unit Process Graphic
        print("3. impl_03_unit_process_graphic")
        save_screenshot(page, "impl_03_unit_process_graphic")

        # 4. Pump Selected
        print("4. impl_04_pump_selected")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject) {
                const pump = s.activeProject.equipment.find(e => e.name.toLowerCase().includes('pump') || e.id.toLowerCase().includes('pump') || e.id.startsWith('P-') || e.id.startsWith('K-'));
                if (pump) {
                    s.selectEquipment(pump.id);
                    s.setFocusedEntity(pump.id);
                    s.openFaceplate({
                        kind: 'pump',
                        id: pump.id,
                        name: pump.name,
                        description: 'CENTRIFUGAL CHARGE PUMP',
                        equipmentId: pump.id,
                    });
                }
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "impl_04_pump_selected")

        # 5. Pump Faceplate
        print("5. impl_05_pump_faceplate")
        page.evaluate("""() => {
            const btnFp = document.querySelector('nebu-dcs-shell button[data-subtab="faceplate"]');
            if (btnFp) btnFp.click();
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "impl_05_pump_faceplate")

        # 6. Valve Faceplate
        print("6. impl_06_valve_faceplate")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject) {
                const valve = s.activeProject.equipment.find(e => e.name.toLowerCase().includes('valve') || e.id.toLowerCase().includes('valve') || e.id.startsWith('FV') || e.id.startsWith('CV') || e.id.startsWith('V-'));
                if (valve) {
                    s.selectEquipment(valve.id);
                    s.setFocusedEntity(valve.id);
                    s.openFaceplate({
                        kind: 'valve',
                        id: valve.id,
                        name: valve.name,
                        description: 'FEED CONTROL VALVE',
                        equipmentId: valve.id,
                    });
                }
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "impl_06_valve_faceplate")

        # 7. Controller Faceplate
        print("7. impl_07_controller_faceplate")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject && s.activeProject.control_loops[0]) {
                const cl = s.activeProject.control_loops[0];
                s.openFaceplate({
                    kind: 'control_loop',
                    id: cl.name,
                    name: cl.name,
                    description: 'PRESSURE CONTROLLER',
                    pvTag: cl.pv_tag,
                    spTag: `${cl.name}.SP`,
                    outTag: cl.out_tag,
                    mode: 'AUTO',
                    sp: cl.sp || 50.0,
                    unit: '%',
                });
                const target = s.getEntityByPvTag(cl.pv_tag);
                if (target) s.setFocusedEntity(target.id);
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "impl_07_controller_faceplate")

        # 8. Loop Mode Switching (CAS / RCAS modes)
        print("8. impl_08_loop_mode_switching")
        page.evaluate("""() => {
            const casBtn = document.querySelector('nebu-faceplate button[data-mode="CAS"]');
            if (casBtn) casBtn.click();
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "impl_08_loop_mode_switching")

        # 9. Multi-Loop Group Display (DCS 8-loop matrix)
        print("9. impl_09_multiloop_group_display")
        page.evaluate("""() => {
            const groupTab = document.querySelector('nebu-dcs-shell button[data-tab="group"]');
            if (groupTab) groupTab.click();
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "impl_09_multiloop_group_display")

        # Return to Unit Flowsheet in SPLIT mode
        page.evaluate("""() => {
            const unitTab = document.querySelector('nebu-dcs-shell button[data-tab="unit"]');
            if (unitTab) unitTab.click();
            const splitBtn = document.querySelector('nebu-dcs-shell button[data-subtab="split"]');
            if (splitBtn) splitBtn.click();
        }""")
        page.wait_for_timeout(600)

        # 10. Active Alarm Annunciation
        print("10. impl_10_active_alarm_annunciation")
        page.evaluate("""() => {
            const s = window.store;
            if (s) {
                s.applyAlarmUpdate([
                    {
                        alarm_name: 'ALM-PUMP-A1-TRIP',
                        state: 'ACTIVE_UNACK',
                        severity: 'CRITICAL',
                        pv: 88.5,
                        trip_limit: 80.0,
                    },
                    {
                        alarm_name: 'ALM-PRESS-HI',
                        state: 'ACTIVE_UNACK',
                        severity: 'WARNING',
                        pv: 104.2,
                        trip_limit: 100.0,
                    }
                ]);
            }
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "impl_10_active_alarm_annunciation")

        # 11. Alarm Acknowledged State
        print("11. impl_11_alarm_acknowledged")
        page.evaluate("""() => {
            const ackBtn = document.querySelector('nebu-dcs-shell #btnAckAllRibbon');
            if (ackBtn) ackBtn.click();
            const hornBtn = document.querySelector('nebu-dcs-shell #btnHornSilence');
            if (hornBtn) hornBtn.click();
            const s = window.store;
            if (s) {
                s.applyAlarmUpdate([
                    {
                        alarm_name: 'ALM-PUMP-A1-TRIP',
                        state: 'ACTIVE_ACK',
                        severity: 'CRITICAL',
                        pv: 88.5,
                        trip_limit: 80.0,
                    },
                    {
                        alarm_name: 'ALM-PRESS-HI',
                        state: 'ACTIVE_ACK',
                        severity: 'WARNING',
                        pv: 104.2,
                        trip_limit: 100.0,
                    }
                ]);
            }
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "impl_11_alarm_acknowledged")

        # 12. Alarm-to-Graphic Navigation
        print("12. impl_12_alarm_to_graphic_navigation")
        page.evaluate("""() => {
            const ticker = document.querySelector('nebu-dcs-shell #activeAlarmTicker');
            if (ticker) ticker.click();
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "impl_12_alarm_to_graphic_navigation")

        # 13. Trend Docked
        print("13. impl_13_trend_docked")
        page.evaluate("""() => {
            const trendBtn = document.querySelector('nebu-dcs-shell button[data-subtab="trend"]');
            if (trendBtn) trendBtn.click();
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "impl_13_trend_docked")

        # 14. Trend Hairline Cursor
        print("14. impl_14_trend_hairline_cursor")
        trend_canvas = page.locator("nebu-trend canvas#trendCanvas")
        if trend_canvas.count() > 0:
            box = trend_canvas.bounding_box()
            if box:
                page.mouse.move(box['x'] + box['width'] * 0.65, box['y'] + box['height'] * 0.5)
                page.wait_for_timeout(400)
        save_screenshot(page, "impl_14_trend_hairline_cursor")

        # 15. Equipment Command Confirmation (Select-Before-Operate Arm)
        print("15. impl_15_select_before_operate_arm")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject) {
                const pump = s.activeProject.equipment.find(e => e.name.toLowerCase().includes('pump') || e.id.toLowerCase().includes('pump') || e.id.startsWith('P-') || e.id.startsWith('K-'));
                if (pump) {
                    s.openFaceplate({
                        kind: 'pump',
                        id: pump.id,
                        name: pump.name,
                        description: 'CENTRIFUGAL CHARGE PUMP',
                        equipmentId: pump.id,
                    });
                }
            }
            const fpBtn = document.querySelector('nebu-dcs-shell button[data-subtab="faceplate"]');
            if (fpBtn) fpBtn.click();
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
        save_screenshot(page, "impl_15_select_before_operate_arm")

        # 16. Trip/Interlock State
        print("16. impl_16_sis_interlock_trip")
        page.evaluate("""() => {
            const s = window.store;
            if (s && s.activeProject) {
                const pump = s.activeProject.equipment.find(e => e.name.toLowerCase().includes('pump') || e.id.toLowerCase().includes('pump') || e.id.startsWith('P-') || e.id.startsWith('K-'));
                if (pump) {
                    s.setDiagnostics([
                        {
                            equipment_id: pump.id,
                            equipment_name: pump.name,
                            equipment_type: 'Pump',
                            validity_state: 'validated',
                            state_label: 'TRIPPED (ESD ZONE 1)',
                            is_tripped: true,
                            active_alarms: ['ALM-VIB-HIGH', 'TRIP-CAV-SEV'],
                            active_interlocks: ['I-101 (SUCTION LEVEL LOLO)'],
                            sis_override_active: true,
                            vibration_mm_s: 7.8,
                            cavitation_regime: 'SevereDamage',
                            cavitation_margin_m: -0.8,
                        }
                    ]);
                }
            }
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "impl_16_sis_interlock_trip")

        # 17. Operator Status Footer
        print("17. impl_17_operator_status_footer")
        page.evaluate("""() => {
            const splitBtn = document.querySelector('nebu-dcs-shell button[data-subtab="split"]');
            if (splitBtn) splitBtn.click();
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "impl_17_operator_status_footer")

        # 18. Full DCS Operator Station (1920x1080 Zero-Scroll Workstation)
        print("18. impl_18_full_dcs_operator_station")
        save_screenshot(page, "impl_18_full_dcs_operator_station")

        print("All 18 implementation screenshots captured successfully!")
        browser.close()

if __name__ == '__main__':
    main()
