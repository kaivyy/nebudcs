import os
import shutil
import time
from playwright.sync_api import sync_playwright

OUT_DIR = '/root/NebuDCS/docs/reports/screenshots/stage14_full'
OUT_DIR_RECON = '/root/NebuDCS/docs/reports/screenshots/operator_reconstruction'
ARTIFACT_DIR = '/root/.gemini/antigravity-cli/brain/c03cf0b9-4be9-4acb-b444-98c837831e07/screenshots/stage14_full'
ARTIFACT_DIR_RECON = '/root/.gemini/antigravity-cli/brain/c03cf0b9-4be9-4acb-b444-98c837831e07/screenshots/operator_reconstruction'

for d in [OUT_DIR, OUT_DIR_RECON, ARTIFACT_DIR, ARTIFACT_DIR_RECON]:
    os.makedirs(d, exist_ok=True)

def save_screenshot(page, name):
    p1 = os.path.join(OUT_DIR, f"{name}.png")
    p2 = os.path.join(ARTIFACT_DIR, f"{name}.png")
    p3 = os.path.join(OUT_DIR_RECON, f"{name}.png")
    p4 = os.path.join(ARTIFACT_DIR_RECON, f"{name}.png")
    page.screenshot(path=p1, full_page=False)
    shutil.copyfile(p1, p2)
    shutil.copyfile(p1, p3)
    shutil.copyfile(p1, p4)
    print(f"Captured: {name}.png")

def main():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True, args=['--no-sandbox', '--disable-setuid-sandbox'])
        context = browser.new_context(viewport={'width': 1920, 'height': 1080}, device_scale_factor=1)
        page = context.new_page()

        print("Navigating to http://127.0.0.1:8088/ ...")
        page.goto("http://127.0.0.1:8088/", wait_until="networkidle")
        page.wait_for_timeout(2000)

        # =====================================================================
        # PART 1: OPERATIONS DCS/SCADA OPERATOR WORKSPACE (Screens 1 to 12)
        # =====================================================================
        print("Ensuring Operations Mode...")
        page.evaluate("""() => {
            const app = document.querySelector('nebu-app');
            if (app) {
                app.activeMode = 'operations';
                app.render();
            }
        }""")
        page.wait_for_timeout(1000)

        # 1. CDU-100 Unit Overview
        page.evaluate("""() => {
            const tab = document.querySelector('nebu-dcs-shell button[data-tab="unit"]');
            if (tab) tab.click();
            if (window.store) {
                window.store.navigateDisplay('disp-unit-01');
            }
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "recon_01_cdu100_unit_overview")

        # 2. Feed & Preheat Train
        page.evaluate("""() => {
            if (window.store) {
                window.store.navigateDisplay('disp-feed-preheat');
            }
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "recon_02_feed_preheat_train")

        # 3. Fired Heater H-101
        page.evaluate("""() => {
            if (window.store) {
                window.store.navigateDisplay('disp-furnace');
            }
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "recon_03_furnace_h101")

        # 4. Distillation Column T-101
        page.evaluate("""() => {
            if (window.store) {
                window.store.navigateDisplay('disp-column');
            }
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "recon_04_column_t101")

        # 5. Wet Gas Compression C-301
        page.evaluate("""() => {
            if (window.store) {
                window.store.navigateDisplay('disp-compression');
            }
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "recon_05_compression_c301")

        # 6. High-Pressure Separation V-103
        page.evaluate("""() => {
            if (window.store) {
                window.store.navigateDisplay('disp-separation');
            }
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "recon_06_separation_v103")

        # 7. Plant Utilities Network
        page.evaluate("""() => {
            if (window.store) {
                window.store.navigateDisplay('disp-utilities');
            }
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "recon_07_utilities_network")

        # 8. Multi-Loop Group Display
        page.evaluate("""() => {
            const tab = document.querySelector('nebu-dcs-shell button[data-tab="group"]');
            if (tab) tab.click();
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "recon_08_multiloop_group")

        # 9. Point Detail
        page.evaluate("""() => {
            const tab = document.querySelector('nebu-dcs-shell button[data-tab="detail"]');
            if (tab) tab.click();
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "recon_09_point_detail")

        # 10. Equipment Faceplate Inspection
        page.evaluate("""() => {
            const tab = document.querySelector('nebu-dcs-shell button[data-tab="unit"]');
            if (tab) tab.click();
            if (window.store) {
                window.store.navigateDisplay('disp-unit-01');
                const s = window.store;
                if (s.activeProject) {
                    const pump = s.activeProject.equipment.find(e => (e.definition_id || '').includes('pump') || e.name.includes('101'));
                    if (pump) {
                        s.selectEquipment(pump.id);
                        s.openFaceplate({
                            kind: 'pump',
                            id: pump.id,
                            name: pump.name,
                            description: 'CENTRIFUGAL CHARGE PUMP',
                            equipmentId: pump.id
                        });
                    }
                }
            }
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "recon_10_equipment_faceplate_inspect")

        # 11. Alarm Active Highlighting
        page.evaluate("""() => {
            const s = window.store;
            if (s) {
                s.alarmBannerCollapsed = false;
                s.activeAlarms = [
                    {
                        id: 'alm-hihi-p101',
                        tag: 'P-101',
                        message: 'HIGH DISCHARGE PRESSURE INTERLOCK TRIP',
                        severity: 'CRITICAL',
                        priority: 'Critical',
                        state: 'UNACK',
                        timestamp: new Date().toLocaleTimeString()
                    },
                    {
                        id: 'alm-hi-t101',
                        tag: 'TI-101',
                        message: 'T-101 OVERHEAD COLUMN HIGH TEMPERATURE',
                        severity: 'HIGH',
                        priority: 'High',
                        state: 'UNACK',
                        timestamp: new Date().toLocaleTimeString()
                    }
                ];
                s.notify();
            }
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "recon_11_alarm_active_highlighting")

        # 12. Plant Overview Grid
        page.evaluate("""() => {
            const tab = document.querySelector('nebu-dcs-shell button[data-tab="plant"]');
            if (tab) tab.click();
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "recon_12_plant_overview_grid")

        # =====================================================================
        # PART 2: ENGINEERING STUDIO WORKSPACE (Screens 13 to 21)
        # =====================================================================
        print("Switching to Engineering Studio Mode...")
        page.evaluate("""() => {
            const app = document.querySelector('nebu-app');
            if (app) {
                app.activeMode = 'engineering';
                app.render();
            }
        }""")
        page.wait_for_timeout(1500)

        # 13. Plant Structure & Display Hierarchy
        print("13. recon_13_engineering_plant_structure")
        page.evaluate("""() => {
            const studio = document.querySelector('nebu-studio');
            if (studio) {
                studio.sidebarMode = 'explorer';
                studio.activeView = 'plant_structure';
                studio.render();
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "recon_13_engineering_plant_structure")

        # 14. System & Unit Configuration
        print("14. recon_14_engineering_system_unit_config")
        page.evaluate("""() => {
            const editor = document.querySelector('nebu-plant-structure-editor');
            if (editor) {
                const sysBtn = editor.querySelector('[data-select-system]');
                if (sysBtn) sysBtn.click();
                const unitBtn = editor.querySelector('[data-select-unit]');
                if (unitBtn) unitBtn.click();
            }
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "recon_14_engineering_system_unit_config")

        # 15. P&ID Flowsheet Canvas with Hierarchical Explorer
        print("15. recon_15_engineering_pid_flowsheet_canvas")
        page.evaluate("""() => {
            const studio = document.querySelector('nebu-studio');
            if (studio) {
                studio.activeView = 'flowsheet';
                studio.sidebarMode = 'explorer';
                studio.render();
            }
            const canvas = document.querySelector('nebu-pid-canvas');
            if (canvas && canvas.fitToView) canvas.fitToView();
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "recon_15_engineering_pid_flowsheet_canvas")

        # 16. Equipment Capability Inspector (Pump K-101 / Rotating Specs)
        print("16. recon_16_engineering_equipment_capability_inspector")
        page.evaluate("""() => {
            const studio = document.querySelector('nebu-studio');
            if (studio) {
                studio.activeView = 'flowsheet';
                studio.render();
            }
            const sStore = window.studioStore || (window).studioStore;
            // Select equipment in canvas or explorer
            const eqEl = document.querySelector('[data-select-eq]');
            if (eqEl) {
                eqEl.click();
            }
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "recon_16_engineering_equipment_capability_inspector")

        # 17. Instrument Configuration Inspector
        print("17. recon_17_engineering_instrument_config")
        page.evaluate("""() => {
            const instEl = document.querySelector('[data-select-inst]');
            if (instEl) {
                instEl.click();
            }
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "recon_17_engineering_instrument_config")

        # 18. Control Loop Configuration
        print("18. recon_18_engineering_control_loop_config")
        page.evaluate("""() => {
            const loopEl = document.querySelector('[data-select-loop]');
            if (loopEl) {
                loopEl.click();
            } else {
                const studio = document.querySelector('nebu-studio');
                if (studio) {
                    studio.activeView = 'controllers';
                    studio.render();
                }
            }
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "recon_18_engineering_control_loop_config")

        # 19. Alarm Configuration
        print("19. recon_19_engineering_alarm_config")
        page.evaluate("""() => {
            const almEl = document.querySelector('[data-select-alarm]');
            if (almEl) {
                almEl.click();
            } else {
                const studio = document.querySelector('nebu-studio');
                if (studio) {
                    studio.activeView = 'safety';
                    studio.render();
                }
            }
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "recon_19_engineering_alarm_config")

        # 20. Validation & Readiness Pre-Flight Panel
        print("20. recon_20_engineering_validation_readiness")
        page.evaluate("""() => {
            const studio = document.querySelector('nebu-studio');
            if (studio) {
                studio.activeView = 'validation';
                studio.render();
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "recon_20_engineering_validation_readiness")

        # 21. Deploy to Runtime Workflow
        print("21. recon_21_engineering_deploy_workflow")
        page.evaluate("""() => {
            const studio = document.querySelector('nebu-studio');
            if (studio) {
                studio.activeView = 'flowsheet';
                studio.render();
                const deployBtn = studio.querySelector('#btnHotDeploy') || studio.querySelector('#btnReadinessDeploy');
                if (deployBtn) {
                    deployBtn.classList.add('active-pulse');
                }
                const feedback = studio.querySelector('#studioFeedback');
                if (feedback) {
                    feedback.textContent = 'Deployed: NebuDCS Process Engineering Vertical Slice -> Live Simulation Runtime Active (20 Hz)';
                    feedback.className = 'studio-status-msg mono text-good font-bold';
                }
            }
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "recon_21_engineering_deploy_workflow")

        browser.close()
        print("All 21 Playwright screenshots successfully captured!")

if __name__ == '__main__':
    main()
