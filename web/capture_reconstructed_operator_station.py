import os
import shutil
import time
from playwright.sync_api import sync_playwright

OUT_DIR = '/root/NebuDCS/docs/reports/screenshots/operator_reconstruction'
ARTIFACT_DIR = '/root/.gemini/antigravity-cli/brain/c03cf0b9-4be9-4acb-b444-98c837831e07/screenshots/operator_reconstruction'

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

        # Switch to Operations Workspace
        page.evaluate("""() => {
            const app = document.querySelector('nebu-app');
            if (app) {
                app.activeMode = 'operations';
                app.render();
            }
        }""")
        page.wait_for_timeout(1000)

        # 1. CDU-100 Unit Overview
        print("1. recon_01_cdu100_unit_overview")
        page.evaluate("""() => {
            const btn = document.querySelector('button[data-disp-id="disp-unit-01"]');
            if (btn) btn.click();
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "recon_01_cdu100_unit_overview")

        # 2. Feed & Pre-Heat Train
        print("2. recon_02_feed_preheat_train")
        page.evaluate("""() => {
            const btn = document.querySelector('button[data-disp-id="disp-feed-preheat"]');
            if (btn) btn.click();
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "recon_02_feed_preheat_train")

        # 3. Furnace H-101 Fired Process Heater
        print("3. recon_03_furnace_h101")
        page.evaluate("""() => {
            const btn = document.querySelector('button[data-disp-id="disp-furnace"]');
            if (btn) btn.click();
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "recon_03_furnace_h101")

        # 4. Column T-101 Atmospheric Distillation
        print("4. recon_04_column_t101")
        page.evaluate("""() => {
            const btn = document.querySelector('button[data-disp-id="disp-column"]');
            if (btn) btn.click();
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "recon_04_column_t101")

        # 5. Compression C-301 Train & Anti-Surge
        print("5. recon_05_compression_c301")
        page.evaluate("""() => {
            const btn = document.querySelector('button[data-disp-id="disp-compression"]');
            if (btn) btn.click();
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "recon_05_compression_c301")

        # 6. Separation V-103 Three-Phase Separator
        print("6. recon_06_separation_v103")
        page.evaluate("""() => {
            const btn = document.querySelector('button[data-disp-id="disp-separation"]');
            if (btn) btn.click();
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "recon_06_separation_v103")

        # 7. Utilities Network Headers
        print("7. recon_07_utilities_network")
        page.evaluate("""() => {
            const btn = document.querySelector('button[data-disp-id="disp-utilities"]');
            if (btn) btn.click();
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "recon_07_utilities_network")

        # 8. Multi-Loop Group Display (8-pack controller matrix)
        print("8. recon_08_multiloop_group")
        page.evaluate("""() => {
            const btn = document.querySelector('button[data-disp-id="disp-group-01"]');
            if (btn) btn.click();
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "recon_08_multiloop_group")

        # 9. Point Detail Specification
        print("9. recon_09_point_detail")
        page.evaluate("""() => {
            const btn = document.querySelector('button[data-disp-id="disp-point-detail"]');
            if (btn) btn.click();
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "recon_09_point_detail")

        # 10. Pump Equipment Faceplate Inspection on Unit Flowsheet
        print("10. recon_10_equipment_faceplate_inspect")
        page.evaluate("""() => {
            const btn = document.querySelector('button[data-disp-id="disp-unit-01"]');
            if (btn) btn.click();
        }""")
        page.wait_for_timeout(600)
        # Click pump
        pump = page.locator("#pumpGroup")
        if pump.count() > 0:
            pump.first.click()
            page.wait_for_timeout(600)
        save_screenshot(page, "recon_10_equipment_faceplate_inspect")

        # 11. Alarm Active Flowsheet Highlighting
        print("11. recon_11_alarm_active_highlighting")
        page.evaluate("""() => {
            const store = window.__nebu_store || document.querySelector('nebu-dcs-shell')?.store;
            // Inject an alarm to verify visual severity halo
            window.dispatchEvent(new CustomEvent('dcs-test-alarm', { detail: { name: 'ALM-K-101-TRIP' } }));
        }""")
        page.wait_for_timeout(600)
        save_screenshot(page, "recon_11_alarm_active_highlighting")

        # 12. Plant Overview Grid
        print("12. recon_12_plant_overview_grid")
        page.evaluate("""() => {
            const tab = document.querySelector('nebu-dcs-shell button[data-tab="plant"]');
            if (tab) tab.click();
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "recon_12_plant_overview_grid")

        browser.close()
        print("All visual verification screenshots captured successfully!")

if __name__ == "__main__":
    main()
