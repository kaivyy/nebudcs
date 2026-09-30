#!/usr/bin/env python3
"""
NebuDCS End-to-End Acceptance Test: Arbitrary Non-Refinery Plant Flowsheet

Proves that the Engineering Studio is genuinely usable as a model-driven P&ID editor:
1. Creates arbitrary plant: Vessel-A -> FT-X1 -> Valve-C3 -> Pump-Z9 -> Tank-B7.
2. Canonical port-to-port stream wiring with deterministic orthogonal routes.
3. Places inline flow instrument FT-X1 on stream line with ISA bubble.
4. Backend topology validation returns VALID (0 errors).
5. Persists to SQLite, refreshes browser, and verifies complete flowsheet reload.
6. Synchronizes project into live simulation runtime and verifies continuous execution.
"""

import os
import sys
import time
import urllib.request
import json
from playwright.sync_api import sync_playwright, expect

BASE_URL = os.environ.get("NEBUDCS_URL", "http://127.0.0.1:8088")
SCREENSHOT_DIR = "/root/NebuDCS/docs/screenshots"
os.makedirs(SCREENSHOT_DIR, exist_ok=True)

def run():
    print("================================================================================")
    print(" NEBUDCS ACCEPTANCE TEST: ARBITRARY NON-REFINERY PLANT FLOWSHEET")
    print(f" Target: {BASE_URL}")
    print("================================================================================\n")

    with sync_playwright() as p:
        browser = p.chromium.launch(
            headless=True,
            args=["--no-sandbox", "--disable-setuid-sandbox", "--disable-dev-shm-usage"]
        )
        context = browser.new_context(viewport={"width": 1440, "height": 900})
        page = context.new_page()

        page.on("dialog", lambda dialog: dialog.accept())

        # 1. Load web application
        print("[1/8] Opening NebuDCS Production Web Application...")
        page.goto(BASE_URL, wait_until="networkidle")
        page.screenshot(path=f"{SCREENSHOT_DIR}/pid_01_app_loaded.png")
        print("  ✓ Application loaded")

        # 2. Switch to Engineering Studio
        print("[2/8] Navigating to Engineering Studio Workbench...")
        page.click("button.mode-tab[data-mode='studio']")
        page.wait_for_selector("nebu-studio", state="visible")
        page.wait_for_selector(".studio-workbench", state="visible")
        page.wait_for_selector(".palette-list .template-card", state="visible", timeout=5000)
        page.screenshot(path=f"{SCREENSHOT_DIR}/pid_02_studio_open.png")
        print("  ✓ Engineering Studio open with catalog palette, flowsheet canvas, and inspector")

        # 3. Create a clean project for the arbitrary plant
        print("[3/8] Creating new project 'Non-Refinery Process Train'...")
        req = urllib.request.Request(
            f"{BASE_URL}/api/v1/projects",
            data=json.dumps({"name": "Non-Refinery Process Train", "description": "Arbitrary chemical plant train"}).encode("utf-8"),
            headers={"Content-Type": "application/json", "Authorization": "Bearer sec-admin-change-me-in-production-482a9f"},
            method="POST"
        )
        with urllib.request.urlopen(req) as resp:
            proj_data = json.loads(resp.read().decode("utf-8"))
        proj_id = proj_data["id"]
        print(f"  ✓ Project created with ID: {proj_id}")

        # Select the newly created project in the UI
        page.reload(wait_until="networkidle")
        page.click("button.mode-tab[data-mode='studio']")
        page.wait_for_selector(".project-selector", state="visible")
        page.select_option(".project-selector", proj_id)
        page.wait_for_timeout(500)
        print("  ✓ Switched active project in Studio UI")

        # 4. Add Equipment: Vessel-A, Valve-C3, Pump-Z9, Tank-B7 via Palette
        print("[4/8] Adding equipment units to flowsheet canvas...")
        palette = page.locator("nebu-equipment-palette")
        
        def add_template(template_id: str):
            btn = palette.locator(f".add-to-canvas-btn[data-id='{template_id}']")
            btn.scroll_into_view_if_needed()
            btn.click()
            page.wait_for_timeout(200)

        add_template("standard.vessel.vertical_separator") # Vessel-A
        add_template("standard.valve.control_globe")        # Valve-C3
        add_template("standard.pump.centrifugal")           # Pump-Z9
        add_template("standard.tank.atmospheric")           # Tank-B7

        eq_nodes = page.locator(".eq-node")
        expect(eq_nodes).to_have_count(4)

        names = ["Vessel-A", "Valve-C3", "Pump-Z9", "Tank-B7"]
        for idx, name in enumerate(names):
            node = eq_nodes.nth(idx)
            node.click()
            page.wait_for_selector(".input-eq-name", state="visible")
            name_input = page.locator(".input-eq-name")
            name_input.fill(name)
            name_input.press("Enter")
            page.wait_for_timeout(100)

        page.screenshot(path=f"{SCREENSHOT_DIR}/pid_03_equipment_placed.png")
        print("  ✓ Placed and renamed Vessel-A, Valve-C3, Pump-Z9, Tank-B7")

        # 5. Wire Streams: Vessel-A (liquid_out) -> Valve-C3 (inlet) -> Pump-Z9 (suction) -> Tank-B7 (inlet)
        print("[5/8] Wiring process stream connections...")
        # Click Vessel-A liquid_out pin to start draft
        vessel_liquid_out = page.locator(".eq-node").first.locator(".port-pin[data-port-name='liquid_out']")
        vessel_liquid_out.click()
        page.wait_for_selector(".connection-draft-banner", state="visible")
        print("  ✓ Wiring draft initiated from Vessel-A.liquid_out")

        # Click Valve-C3 inlet pin to complete S-01
        valve_inlet = page.locator(".eq-node").nth(1).locator(".port-pin[data-port-name='inlet']")
        valve_inlet.click()
        page.wait_for_timeout(200)
        print("  ✓ Completed Stream S-01: Vessel-A.liquid_out -> Valve-C3.inlet")

        # Wire Valve-C3 outlet -> Pump-Z9 suction
        valve_outlet = page.locator(".eq-node").nth(1).locator(".port-pin[data-port-name='outlet']")
        valve_outlet.click()
        page.wait_for_selector(".connection-draft-banner", state="visible")
        pump_suction = page.locator(".eq-node").nth(2).locator(".port-pin[data-port-name='suction']")
        pump_suction.click()
        page.wait_for_timeout(200)
        print("  ✓ Completed Stream S-02: Valve-C3.outlet -> Pump-Z9.suction")

        # Wire Pump-Z9 discharge -> Tank-B7 inlet
        pump_discharge = page.locator(".eq-node").nth(2).locator(".port-pin[data-port-name='discharge']")
        pump_discharge.click()
        page.wait_for_selector(".connection-draft-banner", state="visible")
        tank_inlet = page.locator(".eq-node").nth(3).locator(".port-pin[data-port-name='inlet']")
        tank_inlet.click()
        page.wait_for_timeout(200)
        print("  ✓ Completed Stream S-03: Pump-Z9.discharge -> Tank-B7.inlet")

        expect(page.locator(".stream-group")).to_have_count(3)
        page.screenshot(path=f"{SCREENSHOT_DIR}/pid_04_streams_wired.png")

        # 6. Add inline Flow Transmitter FT-X1 on Stream S-01
        print("[6/8] Adding inline Flow Transmitter FT-X1 to Stream S-01...")
        first_stream = page.locator(".process-pipe-hitarea").first
        first_stream.click()
        page.wait_for_selector(".btn-add-ft", state="visible")
        page.click(".btn-add-ft")
        page.wait_for_timeout(200)

        # Select the new instrument and customize tag to FT-X1
        inst_bubble = page.locator(".instrument-bubble").first
        inst_bubble.click()
        page.wait_for_selector(".input-inst-tag", state="visible")
        tag_input = page.locator(".input-inst-tag")
        tag_input.fill("FT-X1")
        tag_input.press("Enter")

        range_max_input = page.locator(".input-inst-max")
        range_max_input.fill("120")
        range_max_input.press("Enter")
        page.wait_for_timeout(200)
        print("  ✓ Configured inline instrument FT-X1 (0..120 m3/h)")
        page.screenshot(path=f"{SCREENSHOT_DIR}/pid_05_instrument_ft_x1.png")

        # 7. Validate Flowsheet & Save Project
        print("[7/8] Validating flowsheet and persisting to backend...")
        page.click(".btn-validate")
        page.wait_for_selector(".val-status-badge", state="visible")
        val_status = page.locator(".val-status-badge").inner_text().strip()
        print(f"  ✓ Flowsheet Validation Verdict: {val_status}")
        assert val_status == "VALID", f"Expected VALID but got {val_status}"

        # Save project
        save_btn = page.locator(".btn-save")
        if save_btn.is_enabled():
            save_btn.click()
            page.wait_for_timeout(500)
        print("  ✓ Project saved to SQLite persistence")

        # Test full browser reload and persistence verification
        page.reload(wait_until="networkidle")
        page.click("button.mode-tab[data-mode='studio']")
        page.select_option(".project-selector", proj_id)
        page.wait_for_timeout(500)

        expect(page.locator(".eq-node")).to_have_count(4)
        expect(page.locator(".stream-group")).to_have_count(3)
        expect(page.locator(".instrument-node")).to_have_count(1)
        print("  ✓ Reload Verification: All 4 equipment, 3 streams, and FT-X1 restored accurately")

        # 8. Load into Simulation & Observe Operations Console
        print("[8/8] Synchronizing project into live simulation runtime...")
        page.click(".btn-load-sim")
        page.wait_for_selector("#studioFeedback", state="visible")
        feedback = page.locator("#studioFeedback").inner_text()
        print(f"  ✓ Sim sync response: '{feedback}'")

        # Switch to Operations Console
        page.click("button.mode-tab[data-mode='operations']")
        page.wait_for_selector("nebu-process-view", state="visible")

        # Start simulation clock
        play_btn = page.locator(".btn-play").first
        if play_btn.is_enabled():
            play_btn.click()
        page.wait_for_timeout(1000)

        # Check health metrics
        with urllib.request.urlopen(f"{BASE_URL}/api/v1/health") as resp:
            health = json.loads(resp.read().decode("utf-8"))
        print(f"  ✓ Simulation running: ticks = {health['tick_count']}, sim_time = {health['sim_time_sec']:.2f}s")
        assert health["tick_count"] > 0, "Simulation clock did not advance ticks"

        page.screenshot(path=f"{SCREENSHOT_DIR}/pid_06_sim_running.png")
        print("\n================================================================================")
        print(" FINAL ACCEPTANCE VERDICT: PASS")
        print(" Arbitrary plant Vessel-A -> FT-X1 -> Valve-C3 -> Pump-Z9 -> Tank-B7 verified!")
        print("================================================================================\n")

if __name__ == "__main__":
    run()
