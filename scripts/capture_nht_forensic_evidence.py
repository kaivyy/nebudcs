import os
import shutil
import time
from playwright.sync_api import sync_playwright

OUT_DIR = "/root/NebuDCS/docs/screenshots/nht_forensic"
ARTIFACT_DIR = "/root/.gemini/antigravity-cli/brain/c03cf0b9-4be9-4acb-b444-98c837831e07/screenshots"

for d in [OUT_DIR, ARTIFACT_DIR]:
    os.makedirs(d, exist_ok=True)

def save_screenshot(page, name):
    p1 = os.path.join(OUT_DIR, f"{name}.png")
    p2 = os.path.join(ARTIFACT_DIR, f"{name}.png")
    page.screenshot(path=p1, full_page=False)
    shutil.copyfile(p1, p2)
    print(f"Captured: {name}.png -> {p1}")

def main():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True, args=["--no-sandbox", "--disable-setuid-sandbox"])
        context = browser.new_context(viewport={"width": 1920, "height": 1080}, device_scale_factor=1)
        page = context.new_page()

        print("Navigating to http://127.0.0.1:8088/ ...")
        page.goto("http://127.0.0.1:8088/", wait_until="networkidle")
        page.wait_for_timeout(2000)

        # 0. Authenticate & load NHT Reference Plant
        print("Loading NHT Reference Plant...")
        page.evaluate("""async () => {
            const token = "sec-admin-change-me-in-production-482a9f";
            localStorage.setItem("nebudcs_auth_token", token);
            
            try {
                const res = await fetch("/api/v1/projects/proj-domain-nht-reference", {
                    headers: { "Authorization": "Bearer " + token }
                });
                if (res.ok) {
                    const proj = await res.json();
                    if (window.store) {
                        window.store.setActiveProject(proj);
                    }
                    if (window.studioStore) {
                        window.studioStore.setProject(proj, true);
                    }
                }
            } catch (e) {
                console.error("Failed to fetch NHT project", e);
            }
        }""")
        page.wait_for_timeout(1500)

        # 1. audit_01_nht_overview
        print("1. audit_01_nht_overview")
        page.evaluate("""() => {
            const app = document.querySelector("nebu-app");
            if (app) { app.activeMode = "operations"; app.render(); }
            if (window.store) {
                window.store.navigateDisplay("disp-nht-overview");
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "audit_01_nht_overview")

        # 2. audit_02_nht_feed_surge
        print("2. audit_02_nht_feed_surge")
        page.evaluate("""() => {
            if (window.store) {
                window.store.navigateDisplay("disp-nht-feed");
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "audit_02_nht_feed_surge")

        # 3. audit_03_nht_combined_feed_exchanger
        print("3. audit_03_nht_combined_feed_exchanger")
        page.evaluate("""() => {
            if (window.store) {
                window.store.navigateDisplay("disp-nht-rx");
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "audit_03_nht_combined_feed_exchanger")

        # 4. audit_04_nht_furnace_reactor
        print("4. audit_04_nht_furnace_reactor")
        page.evaluate("""() => {
            if (window.store) {
                window.store.navigateDisplay("disp-nht-rx-detail");
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "audit_04_nht_furnace_reactor")

        # 5. audit_05_nht_hp_separator
        print("5. audit_05_nht_hp_separator")
        page.evaluate("""() => {
            if (window.store) {
                window.store.navigateDisplay("disp-nht-sep");
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "audit_05_nht_hp_separator")

        # 6. audit_06_nht_recycle_compressor
        print("6. audit_06_nht_recycle_compressor")
        page.evaluate("""() => {
            if (window.store) {
                window.store.navigateDisplay("disp-nht-h2-recycle");
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "audit_06_nht_recycle_compressor")

        # 7. audit_07_nht_compressor_detail
        print("7. audit_07_nht_compressor_detail")
        page.evaluate("""() => {
            if (window.store) {
                window.store.navigateDisplay("disp-nht-comp-detail");
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "audit_07_nht_compressor_detail")

        # 8. audit_08_nht_stripper
        print("8. audit_08_nht_stripper")
        page.evaluate("""() => {
            if (window.store) {
                window.store.navigateDisplay("disp-nht-strip");
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "audit_08_nht_stripper")

        # 9. audit_09_nht_splitter
        print("9. audit_09_nht_splitter")
        page.evaluate("""() => {
            if (window.store) {
                window.store.navigateDisplay("disp-nht-split");
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "audit_09_nht_splitter")

        # 10. audit_10_studio_palette_instrumentation
        print("10. audit_10_studio_palette_instrumentation")
        page.evaluate("""() => {
            const app = document.querySelector("nebu-app");
            if (app) { app.activeMode = "engineering"; app.render(); }
            const studio = document.querySelector("nebu-studio");
            if (studio) {
                studio.sidebarMode = "palette";
                studio.render();
            }
            const palette = document.querySelector("nebu-equipment-palette");
            if (palette) {
                palette.selectedCategory = "sensors";
                palette.searchQuery = "";
                palette.render();
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "audit_10_studio_palette_instrumentation")

        # 11. audit_11_studio_palette_controls
        print("11. audit_11_studio_palette_controls")
        page.evaluate("""() => {
            const palette = document.querySelector("nebu-equipment-palette");
            if (palette) {
                palette.selectedCategory = "valves";
                palette.searchQuery = "";
                palette.render();
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "audit_11_studio_palette_controls")

        # 12. audit_12_studio_canvas_placement
        print("12. audit_12_studio_canvas_placement")
        page.evaluate("""() => {
            const studio = document.querySelector("nebu-studio");
            if (studio) {
                studio.sidebarMode = "none";
                studio.render();
            }
            const canvas = document.querySelector("nebu-pid-canvas");
            if (canvas) {
                canvas.render();
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "audit_12_studio_canvas_placement")

        # 13. audit_13_operations_faceplate_fic101
        print("13. audit_13_operations_faceplate_fic101")
        page.evaluate("""() => {
            const app = document.querySelector("nebu-app");
            if (app) { app.activeMode = "operations"; app.render(); }
            if (window.store) {
                window.store.navigateDisplay("disp-nht-feed");
                window.store.openFaceplate({
                    kind: "control_loop",
                    id: "loop-fic-101",
                    name: "FIC-101",
                    description: "Naphtha Charge Feed Flow Controller",
                    pvTag: "FT-101",
                    spTag: "FIC-101.SP",
                    outTag: "FV-101.OPENING",
                    unit: "m3/h",
                    rangeMin: 0,
                    rangeMax: 250,
                    mode: "AUTO",
                    setpoint: 199.0
                });
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "audit_13_operations_faceplate_fic101")

        # 14. audit_14_operations_cross_display_navigation
        print("14. audit_14_operations_cross_display_navigation")
        page.evaluate("""() => {
            if (window.store) {
                window.store.closeFaceplate();
                window.store.navigateDisplay("disp-nht-feed");
            }
        }""")
        page.wait_for_timeout(500)
        # Click the off-page continuation badge
        page.evaluate("""() => {
            const badge = document.querySelector(".off-page-badge");
            if (badge) {
                badge.dispatchEvent(new MouseEvent("click", { bubbles: true }));
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "audit_14_operations_cross_display_navigation")

        # 15. audit_15_operations_alarm_summary
        print("15. audit_15_operations_alarm_summary")
        page.evaluate("""() => {
            if (window.store) {
                window.store.activeDisplayTab = "alarms";
                window.store.alarms.set("alm-tah-101", {
                    alarm_name: "TAH-101",
                    tag_id: "TT-102",
                    state: "ACTIVE_ACK",
                    severity: "WARNING",
                    pv: 341.2,
                    trip_limit: 340.0,
                    deadband: 1.0,
                    timestamp: new Date().toISOString()
                });
                window.store.alarms.set("alm-tahh-101", {
                    alarm_name: "TAHH-101",
                    tag_id: "TT-102",
                    state: "ACTIVE_UNACK",
                    severity: "CRITICAL",
                    pv: 344.1,
                    trip_limit: 343.3,
                    deadband: 0.5,
                    timestamp: new Date().toISOString()
                });
                window.store.alarms.set("alm-pahh-101", {
                    alarm_name: "PAHH-101",
                    tag_id: "PT-102",
                    state: "ACTIVE_UNACK",
                    severity: "CRITICAL",
                    pv: 23.4,
                    trip_limit: 23.0,
                    deadband: 0.2,
                    timestamp: new Date().toISOString()
                });
                window.store.alarms.set("alm-fal-103", {
                    alarm_name: "FAL-103",
                    tag_id: "FT-103",
                    state: "ACTIVE_ACK",
                    severity: "WARNING",
                    pv: 3.8,
                    trip_limit: 4.5,
                    deadband: 0.2,
                    timestamp: new Date().toISOString()
                });
                window.store.notify();
            }
            const shell = document.querySelector("nebu-dcs-shell");
            if (shell) shell.render();
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "audit_15_operations_alarm_summary")

        # 16. audit_16_operations_emergency_trip
        print("16. audit_16_operations_emergency_trip")
        page.evaluate("""() => {
            if (window.store) {
                window.store.activeDisplayTab = "graphic";
                window.store.navigateDisplay("disp-nht-rx");
                window.store.diagnosticsMap.set("F-101", {
                    is_tripped: true,
                    trip_reason: "ESD-R101-TEMP Reactor Runaway Protection",
                    sis_override_active: true
                });
                window.store.notify();
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "audit_16_operations_emergency_trip")

        print("Successfully captured all 16 forensic evidence screenshots!")
        browser.close()

if __name__ == "__main__":
    main()
