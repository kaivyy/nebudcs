#!/usr/bin/env python3
"""
Verification of AUD-03 (Clock Keep-Alive Heartbeat) & AUD-04 (Hot Deploy).
"""
import time
from playwright.sync_api import sync_playwright

def main():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page(viewport={"width": 1920, "height": 1080})
        page.goto("http://127.0.0.1:8088")
        time.sleep(2)

        # Check initial status bar
        status_bar = page.query_selector("#globalStatusBar")
        assert status_bar is not None, "Global status bar not found"
        clock_text_initial = page.inner_text("#sbSimClock")
        print(f"Initial Clock Text: {clock_text_initial}")

        # If not running, click play
        if "RUNNING" not in clock_text_initial:
            btn_play = page.query_selector(".btn-play")
            assert btn_play is not None, "Play button not found"
            btn_play.click()
            time.sleep(2)
            clock_text_running = page.inner_text("#sbSimClock")
            print(f"Running Clock Text: {clock_text_running}")
            assert "RUNNING" in clock_text_running, f"Expected RUNNING in clock, got {clock_text_running}"
        else:
            print("Simulation is actively running @ 50 Hz.")

        # Verify time advancement on controls clock
        t0_str = page.inner_text(".time-val")
        print(f"Simulation Time at t0: {t0_str}")

        time.sleep(2.5)
        t1_str = page.inner_text(".time-val")
        print(f"Simulation Time at t1: {t1_str}")
        assert float(t1_str) > float(t0_str), f"Simulation clock did not advance: {t0_str} -> {t1_str}"
        print("  [PASS] AUD-03 Remediated: Simulation clock continuously advances during steady state!")

        # Navigate to Engineering Workspace
        page.click('.mode-tab[data-mode="engineering"]')
        time.sleep(1)

        # Verify Deploy button
        btn_deploy = page.query_selector("#btnHotDeploy")
        assert btn_deploy is not None, "Hot Deploy button not found"
        print(f"Deploy Button Text: {btn_deploy.inner_text()}")
        btn_deploy.click()
        time.sleep(1)

        # Verify status bar shows SYNCHRONIZED
        deploy_status = page.inner_text("#sbDeployStatus")
        print(f"Deploy Status Bar Text: {deploy_status}")
        assert "SYNCHRONIZED" in deploy_status, f"Expected SYNCHRONIZED, got {deploy_status}"
        print("  [PASS] AUD-04 Remediated: Hot Deploy synchronizes project state with runtime!")

        # Navigate back to Operations
        page.click('.mode-tab[data-mode="operations"]')
        time.sleep(1)
        page.screenshot(path="/root/NebuDCS/docs/screenshots/stage_14_20_steady_state_clock_running.png")
        page.screenshot(path="/root/.gemini/antigravity-cli/brain/c03cf0b9-4be9-4acb-b444-98c837831e07/screenshots/stage_14_20_steady_state_clock_running.png")
        print("  [PASS] Screenshot captured: stage_14_20_steady_state_clock_running.png")

        browser.close()
        print("\nALL VERIFICATIONS PASSED SUCCESSFULLY!")

if __name__ == "__main__":
    main()
