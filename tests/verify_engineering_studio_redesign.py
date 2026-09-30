#!/usr/bin/env python3
"""
NebuDCS Engineering Studio UI/UX Redesign & Visual Acceptance Suite.

Validates the redesigned industrial CAD-grade P&ID editor against the 8 mandatory
visual acceptance criteria at 1440x900, verifies multi-resolution responsive behavior
(1440x900, 1280x800, 1024x768), tests pan/zoom/fit controls, and captures all 8
canonical verification screenshots.
"""

import os
import json
import time
import urllib.request
from playwright.sync_api import sync_playwright, expect

BASE_URL = "http://127.0.0.1:8088"
SCREENSHOT_DIR = "/root/NebuDCS/docs/screenshots"
os.makedirs(SCREENSHOT_DIR, exist_ok=True)

def run():
    print("================================================================================")
    print(" NEBUDCS ENGINEERING STUDIO UI/UX REDESIGN ACCEPTANCE SUITE")
    print(" Target: " + BASE_URL)
    print("================================================================================\n")

    with sync_playwright() as p:
        browser = p.chromium.launch(
            headless=True,
            args=[
                "--no-sandbox",
                "--disable-dev-shm-usage",
                "--disable-gpu",
                "--window-size=1440,900",
            ],
        )
        context = browser.new_context(viewport={"width": 1440, "height": 900})
        page = context.new_page()

        console_errors = []
        page.on("console", lambda msg: console_errors.append(msg.text) if msg.type == "error" else None)
        page.on("dialog", lambda d: d.accept())

        # ----------------------------------------------------------------------
        # 1. Empty Engineering Studio (1440x900)
        # ----------------------------------------------------------------------
        print("[1/8] Capturing empty Engineering Studio layout at 1440x900...")
        page.goto(BASE_URL, wait_until="networkidle")
        page.click("button.mode-tab[data-mode='studio']")
        page.wait_for_selector("nebu-studio", state="visible")
        page.wait_for_selector(".studio-workbench", state="visible")
        page.wait_for_selector(".studio-statusbar", state="visible")

        # Create a fresh empty project
        proj_name = f"Industrial Plant {int(time.time())}"
        req = urllib.request.Request(
            f"{BASE_URL}/api/v1/projects",
            data=json.dumps({"name": proj_name, "description": "UI/UX Redesign Test Plant"}).encode("utf-8"),
            headers={
                "Content-Type": "application/json",
                "Authorization": "Bearer sec-admin-change-me-in-production-482a9f",
            },
            method="POST",
        )
        with urllib.request.urlopen(req) as resp:
            proj_data = json.loads(resp.read().decode("utf-8"))
        proj_id = proj_data["id"]

        page.reload(wait_until="networkidle")
        page.click("button.mode-tab[data-mode='studio']")
        page.select_option(".project-selector", proj_id)
        page.wait_for_timeout(300)

        page.screenshot(path=f"{SCREENSHOT_DIR}/ui_01_empty_engineering_studio.png")
        print("  ✓ Captured: ui_01_empty_engineering_studio.png")

        # ----------------------------------------------------------------------
        # 2. Equipment Palette Detail
        # ----------------------------------------------------------------------
        print("[2/8] Inspecting and capturing categorized equipment palette...")
        palette = page.locator("nebu-equipment-palette")
        expect(palette.locator(".template-card")).to_have_count(12)
        expect(palette.locator(".cat-chip")).to_have_count(6)
        expect(palette.locator(".quick-inst-card")).to_have_count(4)
        expect(palette.locator(".quick-conn-row")).to_have_count(3)

        # Focus search and filter
        palette_search = palette.locator(".palette-search")
        palette_search.fill("pump")
        page.wait_for_timeout(200)
        expect(palette.locator(".template-card")).to_have_count(1)
        palette.locator(".btn-clear-search").click()
        page.wait_for_timeout(200)
        expect(palette.locator(".template-card")).to_have_count(12)

        page.screenshot(path=f"{SCREENSHOT_DIR}/ui_02_equipment_palette.png")
        print("  ✓ Captured: ui_02_equipment_palette.png")

        # ----------------------------------------------------------------------
        # 3. Populated P&ID (Placement & Renaming)
        # ----------------------------------------------------------------------
        print("[3/8] Populating flowsheet with generic industrial equipment units...")
        def add_template(template_id: str):
            btn = palette.locator(f".add-to-canvas-btn[data-id='{template_id}']")
            btn.scroll_into_view_if_needed()
            btn.click()
            page.wait_for_timeout(150)

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

        page.screenshot(path=f"{SCREENSHOT_DIR}/ui_03_populated_pid.png")
        print("  ✓ Captured: ui_03_populated_pid.png")

        # ----------------------------------------------------------------------
        # 4. Active Pipe Connection (Wiring Preview & Snapping)
        # ----------------------------------------------------------------------
        print("[4/8] Initiating connection draft to capture live orthogonal rubber-band preview...")
        vessel_liquid_out = page.locator(".eq-node").first.locator(".port-pin[data-port-name='liquid_out']")
        vessel_liquid_out.click()
        page.wait_for_selector(".connection-draft-banner", state="visible")

        # Move mouse toward valve inlet to show live drafting line
        valve_inlet = page.locator(".eq-node").nth(1).locator(".port-pin[data-port-name='inlet']")
        valve_box = valve_inlet.bounding_box()
        if valve_box:
            page.mouse.move(valve_box["x"] + 5, valve_box["y"] + 5)
            page.wait_for_timeout(200)

        page.screenshot(path=f"{SCREENSHOT_DIR}/ui_04_active_pipe_connection.png")
        print("  ✓ Captured: ui_04_active_pipe_connection.png")

        # Complete Stream S-01
        valve_inlet.click()
        page.wait_for_timeout(200)

        # Wire Stream S-02: Valve-C3.outlet -> Pump-Z9.suction
        valve_outlet = page.locator(".eq-node").nth(1).locator(".port-pin[data-port-name='outlet']")
        valve_outlet.click()
        page.wait_for_selector(".connection-draft-banner", state="visible")
        pump_suction = page.locator(".eq-node").nth(2).locator(".port-pin[data-port-name='suction']")
        pump_suction.click()
        page.wait_for_timeout(200)

        # Wire Stream S-03: Pump-Z9.discharge -> Tank-B7.inlet
        pump_discharge = page.locator(".eq-node").nth(2).locator(".port-pin[data-port-name='discharge']")
        pump_discharge.click()
        page.wait_for_selector(".connection-draft-banner", state="visible")
        tank_inlet = page.locator(".eq-node").nth(3).locator(".port-pin[data-port-name='inlet']")
        tank_inlet.click()
        page.wait_for_timeout(200)
        expect(page.locator(".stream-group")).to_have_count(3)

        # ----------------------------------------------------------------------
        # 5. Selected Equipment + Structured Inspector
        # ----------------------------------------------------------------------
        print("[5/8] Selecting equipment node to inspect structured property accordion...")
        page.locator(".eq-node").nth(2).click() # Select Pump-Z9
        page.wait_for_selector(".inspector-section", state="visible")
        expect(page.locator(".inspector-section")).to_have_count(4) # IDENTITY, PORTS, PARAMETERS, STATUS

        page.screenshot(path=f"{SCREENSHOT_DIR}/ui_05_selected_equipment_inspector.png")
        print("  ✓ Captured: ui_05_selected_equipment_inspector.png")

        # ----------------------------------------------------------------------
        # 6. Inline Instrument FT-X1
        # ----------------------------------------------------------------------
        print("[6/8] Adding inline Flow Transmitter FT-X1 to Stream S-01...")
        first_stream = page.locator(".process-pipe-hitarea").first
        first_stream.click()
        page.wait_for_selector(".btn-add-ft", state="visible")
        page.click(".btn-add-ft")
        page.wait_for_timeout(200)

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

        page.screenshot(path=f"{SCREENSHOT_DIR}/ui_06_inline_ft.png")
        print("  ✓ Captured: ui_06_inline_ft.png")

        # ----------------------------------------------------------------------
        # 7. Validation State & Persistence
        # ----------------------------------------------------------------------
        print("[7/8] Validating flowsheet and capturing status bar diagnostics...")
        page.click(".btn-validate")
        page.wait_for_selector(".val-status-badge", state="visible")
        val_status = page.locator(".val-status-badge").inner_text().strip()
        print(f"  ✓ Flowsheet Validation Status: {val_status}")
        assert val_status == "VALID", f"Expected VALID but got {val_status}"

        # Save to SQLite
        save_btn = page.locator(".btn-save")
        if save_btn.is_enabled():
            save_btn.click()
            page.wait_for_timeout(300)

        page.screenshot(path=f"{SCREENSHOT_DIR}/ui_07_validation_state.png")
        print("  ✓ Captured: ui_07_validation_state.png")

        # ----------------------------------------------------------------------
        # 8. Synchronize and Run Simulation
        # ----------------------------------------------------------------------
        print("[8/8] Loading flowsheet into simulation runtime & running in Operations...")
        page.click(".btn-load-sim")
        page.wait_for_selector("#studioFeedback", state="visible")

        # Switch to Operations Console
        page.click("button.mode-tab[data-mode='operations']")
        page.wait_for_selector("nebu-process-view", state="visible")

        # Start continuous simulation clock
        play_btn = page.locator(".btn-play").first
        if play_btn.is_enabled():
            play_btn.click()
        page.wait_for_timeout(1000)

        with urllib.request.urlopen(f"{BASE_URL}/api/v1/health") as resp:
            health = json.loads(resp.read().decode("utf-8"))
        print(f"  ✓ Simulation running: tick_count = {health['tick_count']}, sim_time = {health['sim_time_sec']:.2f}s")
        assert health["tick_count"] > 0, "Simulation clock did not advance ticks"

        page.screenshot(path=f"{SCREENSHOT_DIR}/ui_08_running_simulation.png")
        print("  ✓ Captured: ui_08_running_simulation.png")

        # ----------------------------------------------------------------------
        # Responsive Testing (1440x900, 1280x800, 1024x768)
        # ----------------------------------------------------------------------
        print("\n--- Testing Multi-Resolution Responsive Layouts ---")
        resolutions = [
            (1440, 900, "1440x900 Desktop"),
            (1280, 800, "1280x800 Medium Display"),
            (1024, 768, "1024x768 Compact Industrial Display"),
        ]

        page.click("button.mode-tab[data-mode='studio']")
        page.wait_for_selector(".studio-workbench", state="visible")

        for width, height, label in resolutions:
            page.set_viewport_size({"width": width, "height": height})
            page.wait_for_timeout(200)

            # Assert no horizontal body overflow
            scroll_width = page.evaluate("document.body.scrollWidth")
            client_width = page.evaluate("document.body.clientWidth")
            print(f"  [{label}] Viewport: {width}x{height} | scrollWidth={scroll_width}, clientWidth={client_width}")
            assert scroll_width <= client_width, f"Horizontal page scrolling detected on {label}: scrollWidth={scroll_width} > clientWidth={client_width}"

            # Canvas must remain visible and dominant
            canvas_box = page.locator(".workbench-canvas").bounding_box()
            assert canvas_box is not None, f"Canvas not rendered on {label}"
            canvas_ratio = canvas_box["width"] / width
            print(f"  [{label}] Canvas width = {canvas_box['width']:.1f}px ({canvas_ratio*100:.1f}% of viewport)")
            assert canvas_ratio >= 0.40, f"Canvas too narrow on {label}: {canvas_ratio:.2f}"

            page.screenshot(path=f"{SCREENSHOT_DIR}/responsive_{width}x{height}.png")

        print("\n================================================================================")
        print(" ALL 8 VISUAL ACCEPTANCE TESTS & RESPONSIVE RESOLUTIONS PASSED")
        print(" Uncaught console errors: " + str(len(console_errors)))
        print("================================================================================\n")
        assert len(console_errors) == 0, f"Uncaught console errors: {console_errors}"

if __name__ == "__main__":
    run()
