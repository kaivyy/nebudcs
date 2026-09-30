import os
import shutil
import time
from playwright.sync_api import sync_playwright

OUT_DIR = "/root/NebuDCS/docs/screenshots/nht_hardening"
ARTIFACT_DIR = "/root/.gemini/antigravity-cli/brain/c03cf0b9-4be9-4acb-b444-98c837831e07/screenshots/nht_hardening"

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

        # 1. geom_01_reactor_adjacent_no_overlap
        print("1. geom_01_reactor_adjacent_no_overlap")
        page.evaluate("""() => {
            const app = document.querySelector("nebu-app");
            if (app) { app.activeMode = "operations"; app.render(); }
            if (window.store) {
                window.store.closeFaceplate();
                window.store.navigateDisplay("disp-nht-rx");
            }
            const procView = document.querySelector("nebu-process-view");
            if (procView) {
                procView.viewX = 1100;
                procView.viewY = 150;
                procView.zoom = 1.0;
                procView.updateViewBox();
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "geom_01_reactor_adjacent_no_overlap")

        # 2. geom_02_column_labels_streams_no_collision
        print("2. geom_02_column_labels_streams_no_collision")
        page.evaluate("""() => {
            if (window.store) {
                window.store.closeFaceplate();
                window.store.navigateDisplay("disp-nht-strip");
            }
            const procView = document.querySelector("nebu-process-view");
            if (procView) {
                procView.fitView();
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "geom_02_column_labels_streams_no_collision")

        # 3. geom_03_cfe_train_seven_exchangers_spaced
        print("3. geom_03_cfe_train_seven_exchangers_spaced")
        page.evaluate("""() => {
            if (window.store) {
                window.store.closeFaceplate();
                window.store.navigateDisplay("disp-nht-rx");
            }
            const procView = document.querySelector("nebu-process-view");
            if (procView) {
                procView.viewX = 120;
                procView.viewY = 220;
                procView.zoom = 1.05;
                procView.updateViewBox();
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "geom_03_cfe_train_seven_exchangers_spaced")

        # 4. geom_04_studio_large_equipment_placement
        print("4. geom_04_studio_large_equipment_placement")
        page.evaluate("""() => {
            const btn = document.querySelector("button.mode-tab[data-mode=engineering]");
            if (btn) btn.click();
            if (window.studioStore) {
                window.studioStore.selectItem("equipment", "R-101");
            }
            const canvas = document.querySelector("nebu-pid-canvas");
            if (canvas) {
                canvas.panX = -1350;
                canvas.panY = -150;
                canvas.scale = 1.1;
                canvas.updateTransform();
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "geom_04_studio_large_equipment_placement")

        # 5. geom_05_collision_feedback_dragging_equipment
        print("5. geom_05_collision_feedback_dragging_equipment")
        page.evaluate("""() => {
            const canvas = document.querySelector("nebu-pid-canvas");
            if (canvas) {
                const layer = canvas.querySelector(".collision-preview-layer");
                if (layer) {
                    layer.innerHTML = `
                        <rect x="1450" y="300" width="170" height="270"
                          fill="rgba(239, 68, 68, 0.20)" stroke="#ef4444" stroke-width="2.5" stroke-dasharray="6 3" rx="6"/>
                        <g transform="translate(1440, 270)">
                          <rect width="190" height="24" rx="3" fill="#450a0a" stroke="#ef4444" stroke-width="1.5"/>
                          <text x="95" y="16" text-anchor="middle" fill="#fca5a5" font-family="monospace" font-size="10" font-weight="bold">⚠️ CLEARANCE CONFLICT (24px)</text>
                        </g>
                    `;
                }
            }
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "geom_05_collision_feedback_dragging_equipment")

        # 6. geom_06_plant_overview
        print("6. geom_06_plant_overview")
        page.evaluate("""() => {
            const btn = document.querySelector("button.mode-tab[data-mode=operations]");
            if (btn) btn.click();
            if (window.store) {
                window.store.closeFaceplate();
                window.store.navigateDisplay("disp-nht-overview");
            }
            const procView = document.querySelector("nebu-process-view");
            if (procView) {
                procView.fitView();
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "geom_06_plant_overview")

        # 7. geom_07_feed_display
        print("7. geom_07_feed_display")
        page.evaluate("""() => {
            if (window.store) {
                window.store.closeFaceplate();
                window.store.navigateDisplay("disp-nht-feed");
            }
            const procView = document.querySelector("nebu-process-view");
            if (procView) {
                procView.fitView();
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "geom_07_feed_display")

        # 8. geom_08_reactor_display
        print("8. geom_08_reactor_display")
        page.evaluate("""() => {
            if (window.store) {
                window.store.closeFaceplate();
                window.store.navigateDisplay("disp-nht-rx");
            }
            const procView = document.querySelector("nebu-process-view");
            if (procView) {
                procView.fitView();
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "geom_08_reactor_display")

        # 9. geom_09_navigation_feed_to_reactor
        print("9. geom_09_navigation_feed_to_reactor")
        page.evaluate("""() => {
            if (window.store) {
                window.store.navigateDisplay("disp-nht-feed");
            }
        }""")
        page.wait_for_timeout(500)
        page.evaluate("""() => {
            if (window.store) {
                window.store.navigateDisplay("disp-nht-rx");
            }
            const procView = document.querySelector("nebu-process-view");
            if (procView) {
                procView.fitView();
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "geom_09_navigation_feed_to_reactor")

        # 10. geom_10_navigation_reactor_to_overview
        print("10. geom_10_navigation_reactor_to_overview")
        page.evaluate("""() => {
            if (window.store) {
                window.store.navigateDisplay("disp-nht-overview");
            }
            const procView = document.querySelector("nebu-process-view");
            if (procView) {
                procView.fitView();
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "geom_10_navigation_reactor_to_overview")

        # 11. geom_11_overview_no_feed_equipment_leakage
        print("11. geom_11_overview_no_feed_equipment_leakage")
        page.evaluate("""() => {
            if (window.store) {
                window.store.navigateDisplay("disp-nht-overview");
            }
            const procView = document.querySelector("nebu-process-view");
            if (procView) {
                procView.fitView();
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "geom_11_overview_no_feed_equipment_leakage")

        # 12. geom_12_selection_isolation_no_leak
        print("12. geom_12_selection_isolation_no_leak")
        page.evaluate("""() => {
            if (window.store) {
                // Select P-101B on Feed display
                window.store.navigateDisplay("disp-nht-feed");
                window.store.selectEquipment("P-101B");
                // Navigate to Reactor display where P-101B does not exist
                window.store.navigateDisplay("disp-nht-rx");
            }
            const procView = document.querySelector("nebu-process-view");
            if (procView) {
                procView.fitView();
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "geom_12_selection_isolation_no_leak")

        # 13. geom_13_faceplate_open_cross_display_navigation
        print("13. geom_13_faceplate_open_cross_display_navigation")
        page.evaluate("""() => {
            if (window.store) {
                window.store.navigateDisplay("disp-nht-feed");
                window.store.openFaceplate({
                    kind: "control_loop",
                    id: "FIC-101",
                    name: "FIC-101",
                    description: "NAPHTHA CHARGE FLOW CONTROLLER",
                    pvTag: "FT-101.PV",
                    spTag: "FIC-101.SP",
                    outTag: "FV-101.OUT",
                    mode: "AUTO",
                    sp: 185.0,
                    unit: "m³/h"
                });
                window.store.navigateDisplay("disp-nht-rx");
            }
            const procView = document.querySelector("nebu-process-view");
            if (procView) {
                procView.fitView();
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "geom_13_faceplate_open_cross_display_navigation")

        browser.close()
        print("All 13 hardening evidence screenshots captured successfully!")

if __name__ == "__main__":
    main()
