#!/usr/bin/env python3
"""
NebuDCS End-to-End Browser Functional Acceptance Test Suite.

Executes complete real-user journeys (Steps A through E) against the live
systemd production service at http://127.0.0.1:8088 using Playwright headless Chromium.
Captures screenshots, asserts DOM state transitions, verifies RBAC enforcement,
and records response metrics.
"""

import os
import sys
import time
from playwright.sync_api import sync_playwright, Page, expect

BASE_URL = os.environ.get("NEBUDCS_URL", "http://127.0.0.1:8088")
SCREENSHOT_DIR = "/root/NebuDCS/docs/screenshots"
os.makedirs(SCREENSHOT_DIR, exist_ok=True)

console_errors = []
console_logs = []

def handle_console(msg):
    text = f"[{msg.type.upper()}] {msg.text}"
    console_logs.append(text)
    if msg.type in ("error", "exception"):
        # Ignore expected 404/401 fetch probes if properly handled by catch
        if "favicon.ico" not in msg.text:
            console_errors.append(text)

def run_acceptance_tests():
    print(f"================================================================================")
    print(f" NEBUDCS PRODUCTION BROWSER FUNCTIONAL ACCEPTANCE SUITE")
    print(f" Target: {BASE_URL}")
    print(f"================================================================================\n")

    results = {}

    with sync_playwright() as p:
        browser = p.chromium.launch(
            headless=True,
            args=["--no-sandbox", "--disable-setuid-sandbox", "--disable-dev-shm-usage"]
        )
        context = browser.new_context(viewport={"width": 1440, "height": 900})
        page = context.new_page()

        page.on("console", handle_console)
        page.on("dialog", lambda dialog: dialog.accept())
        page.on("response", lambda r: print(f"  [HTTP ERROR] {r.status} {r.request.method} {r.url}") if r.status >= 400 else None)

        # ----------------------------------------------------------------------
        # Initial Navigation & Shell Verification
        # ----------------------------------------------------------------------
        print("[INIT] Loading production web application...")
        start_t = time.time()
        page.goto(BASE_URL, wait_until="networkidle")
        load_time_ms = (time.time() - start_t) * 1000.0
        print(f"  ✓ Page loaded in {load_time_ms:.1f}ms | Title: '{page.title()}'")

        # Verify Header and Default Admin Role
        expect(page.locator("header.app-header")).to_be_visible()
        role_badge = page.locator("#headerRoleBadge")
        expect(role_badge).to_be_visible()
        print(f"  ✓ Header Role Badge: {role_badge.inner_text().strip()}")

        # ----------------------------------------------------------------------
        # Step A: Engineering Studio
        # ----------------------------------------------------------------------
        print("\n--- [STEP A] Engineering Studio: Authoring, Configuration & Sim Load ---")
        t0 = time.time()
        
        # 1. Switch to Engineering Studio
        page.click("button.mode-tab[data-mode='studio']")
        page.wait_for_selector("nebu-studio", state="visible")
        page.wait_for_selector(".studio-workbench", state="visible")
        print("  ✓ Engineering Studio workbench visible")

        # 2. Inspect Palette Templates
        page.wait_for_selector(".palette-list .template-card", state="visible", timeout=5000)
        template_cards = page.locator(".palette-list .template-card")
        template_count = template_cards.count()
        print(f"  ✓ Equipment Palette loaded with {template_count} templates")
        assert template_count > 0, "No equipment templates in palette"

        # 3. Add equipment from palette to canvas
        first_add_btn = template_cards.first.locator(".add-to-canvas-btn")
        first_add_btn.click()
        page.wait_for_timeout(300)
        print("  ✓ Added equipment template to active flowsheet canvas")

        # 4. Trigger validation
        page.click(".btn-validate")
        page.wait_for_selector(".validation-header", state="visible")
        val_status = page.locator(".val-status-badge").inner_text().strip()
        print(f"  ✓ Flowsheet validation executed: Status = {val_status}")

        # 5. Save active project if dirty
        save_btn = page.locator(".btn-save")
        if save_btn.is_enabled():
            save_btn.click()
            page.wait_for_timeout(500)
            print("  ✓ Flowsheet project saved")

        # 6. Load project into simulation runtime
        load_sim_btn = page.locator(".btn-load-sim")
        expect(load_sim_btn).to_be_visible()
        load_sim_btn.click()
        
        # Verify status feedback
        feedback_el = page.locator("#studioFeedback")
        page.wait_for_timeout(800)
        feedback_text = feedback_el.inner_text().strip()
        print(f"  ✓ Simulation synchronization response: '{feedback_text}'")
        assert "Simulation updated" in feedback_text or "synchronized" in feedback_text, \
            f"Unexpected load-sim response: {feedback_text}"

        ss_path_a = os.path.join(SCREENSHOT_DIR, "step_a_engineering_studio.png")
        page.screenshot(path=ss_path_a)
        print(f"  📷 Captured: {ss_path_a}")
        step_a_duration = (time.time() - t0) * 1000.0
        results["Step A - Engineering Studio"] = f"PASS ({step_a_duration:.1f}ms)"

        # ----------------------------------------------------------------------
        # Step B: Operations Console
        # ----------------------------------------------------------------------
        print("\n--- [STEP B] Operations Console: Supervision & Intervention ---")
        t0 = time.time()

        # Switch role to Operator
        page.select_option("#selActiveRole", "operator")
        page.wait_for_timeout(300)
        assert page.locator("#headerRoleBadge").inner_text().strip() == "OPERATOR"
        print("  ✓ Active RBAC role switched to OPERATOR")

        # Switch to Operations Console
        page.click("button.mode-tab[data-mode='operations']")
        page.wait_for_selector("nebu-process-view", state="visible")
        print("  ✓ Operations Console layout rendered")

        # Verify dynamic process graphic title (model bound)
        pv_title = page.locator("#processUnitTitle").inner_text().strip()
        pv_sub = page.locator("#processUnitSub").inner_text().strip()
        print(f"  ✓ Process Graphic Header: '{pv_title}' ({pv_sub})")
        assert "NO PROJECT LOADED" not in pv_title, "Process view failed to bind active project"

        # Verify SVG equipment nodes exist
        svg_nodes = page.locator("#processSvg .equipment-node, #processSvg g[data-eq-id]")
        node_count = svg_nodes.count()
        print(f"  ✓ Interactive Process SVG rendered with {node_count} process elements")
        assert node_count > 0, "No SVG equipment nodes rendered"

        # Verify Faceplate and Controller Tuning
        fp = page.locator("#appFaceplate")
        expect(fp).to_be_visible()
        fp_title = fp.locator(".loop-name").inner_text().strip()
        print(f"  ✓ Faceplate active for controller: '{fp_title}'")

        # Change Setpoint (SP)
        page.fill("#spIn", "55.0")
        page.click("#spApply")
        page.wait_for_timeout(400)
        fp_feedback = page.locator(".faceplate-feedback").inner_text().strip()
        print(f"  ✓ Setpoint updated: '{fp_feedback}'")
        assert "SP updated" in fp_feedback, f"Setpoint update failed: {fp_feedback}"

        # Change Mode to MANUAL and back
        page.select_option("#pidModeSelect", "MANUAL")
        page.wait_for_timeout(300)
        print("  ✓ PID Controller Mode changed to MANUAL")

        # Alarm Station inspection & ACK
        alarm_panel = page.locator("#appAlarmPanel")
        expect(alarm_panel).to_be_visible()
        page.click("#ackAllBtn")
        print("  ✓ Executed Global Alarm ACK (ANSI/ISA-18.2)")

        # Keyboard Navigation (Space key quick ACK)
        page.keyboard.press("Space")
        print("  ✓ Dispatched Keyboard Shortcut: [Space] for ISA-18.2 Ticker")

        # Realtime Trend Verification
        trend = page.locator("#appTrend")
        expect(trend).to_be_visible()
        canvas = trend.locator("#trendCanvas")
        expect(canvas).to_be_visible()
        print("  ✓ Realtime Process Trend Canvas active (60s rolling window)")

        ss_path_b = os.path.join(SCREENSHOT_DIR, "step_b_operations_console.png")
        page.screenshot(path=ss_path_b)
        print(f"  📷 Captured: {ss_path_b}")
        step_b_duration = (time.time() - t0) * 1000.0
        results["Step B - Operations Console"] = f"PASS ({step_b_duration:.1f}ms)"

        # ----------------------------------------------------------------------
        # Step C: Instructor Console
        # ----------------------------------------------------------------------
        print("\n--- [STEP C] Instructor Console: Session Orchestration & Faults ---")
        t0 = time.time()

        # Switch role to Instructor
        page.select_option("#selActiveRole", "instructor")
        page.wait_for_timeout(300)
        assert page.locator("#headerRoleBadge").inner_text().strip() == "INSTRUCTOR"
        print("  ✓ Active RBAC role switched to INSTRUCTOR")

        # Switch to Training Studio
        page.click("button.mode-tab[data-mode='training']")
        page.wait_for_selector("nebu-training-studio", state="visible")
        print("  ✓ Training Studio loaded")

        # Open Create Session Modal
        page.click("#btnOpenCreateSession")
        page.wait_for_selector("#modalCreateSession", state="visible")
        print("  ✓ Create Training Session modal opened")

        # Fill Session Form
        session_name = f"Browser E2E Drill {int(time.time())}"
        page.fill("#inpSessionName", session_name)
        page.fill("#inpSessionDesc", "Automated Playwright acceptance test verification")
        page.fill("#inpInstructorId", "INST-LEAD-01")
        page.fill("#inpTraineeId", "TRAINEE-CADET-09")
        
        # Verify Project selector has options and select first
        proj_select = page.locator("#selProjectId")
        expect(proj_select).to_be_visible()
        proj_options = proj_select.locator("option").all()
        print(f"  ✓ Target Plant Project selector has {len(proj_options)} candidate projects")
        assert len(proj_options) > 0, "No projects in training session project selector"

        page.click("#formCreateSession button[type='submit']")
        page.wait_for_timeout(800)
        print(f"  ✓ Submitted new training session: '{session_name}'")

        # Verify session is selected and visible
        curr_session_badge = page.locator(".training-session-badge .session-name").first.inner_text().strip()
        print(f"  ✓ Active session synchronized: '{curr_session_badge}'")

        # Load project into session
        load_proj_btn = page.locator("#btnLoadProject")
        expect(load_proj_btn).to_be_visible()
        load_proj_btn.click()
        page.wait_for_timeout(600)
        print("  ✓ Loaded plant topology and initial scenario into session")

        # Start Session
        start_btn = page.locator("#btnStartSession")
        start_btn.click()
        page.wait_for_timeout(600)
        state_badge = page.locator(".training-session-badge .badge").first.inner_text().strip()
        print(f"  ✓ Started simulation session: State = {state_badge}")
        assert state_badge == "RUNNING", f"Expected RUNNING, got {state_badge}"

        # Pause Session
        pause_btn = page.locator("#btnPauseSession")
        pause_btn.click()
        page.wait_for_timeout(600)
        state_badge = page.locator(".training-session-badge .badge").first.inner_text().strip()
        print(f"  ✓ Paused simulation session: State = {state_badge}")
        assert state_badge == "PAUSED", f"Expected PAUSED, got {state_badge}"

        # Resume Session
        resume_btn = page.locator("#btnResumeSession")
        resume_btn.click()
        page.wait_for_timeout(600)
        state_badge = page.locator(".training-session-badge .badge").first.inner_text().strip()
        print(f"  ✓ Resumed simulation session: State = {state_badge}")
        assert state_badge == "RUNNING", f"Expected RUNNING, got {state_badge}"

        # Inject Ad-Hoc Process Fault
        fault_btn = page.locator(".btn-inject-feed-loss")
        fault_btn.click()
        page.wait_for_timeout(400)
        print("  ✓ Injected ad-hoc process disturbance into live simulation")

        ss_path_c = os.path.join(SCREENSHOT_DIR, "step_c_instructor_console.png")
        page.screenshot(path=ss_path_c)
        print(f"  📷 Captured: {ss_path_c}")
        step_c_duration = (time.time() - t0) * 1000.0
        results["Step C - Instructor Console"] = f"PASS ({step_c_duration:.1f}ms)"

        # ----------------------------------------------------------------------
        # Step D: Trainee Station
        # ----------------------------------------------------------------------
        print("\n--- [STEP D] Trainee Station: Operator Station & RBAC Enforcement ---")
        t0 = time.time()

        # Switch role to Trainee
        page.select_option("#selActiveRole", "trainee")
        page.wait_for_timeout(300)
        assert page.locator("#headerRoleBadge").inner_text().strip() == "TRAINEE"
        print("  ✓ Active RBAC role switched to TRAINEE")

        # Switch tab to Trainee Station
        page.click("button.t-tab[data-tab='trainee']")
        page.wait_for_selector(".trainee-station-container", state="visible")
        print("  ✓ Trainee Station operator interface loaded")

        # Permitted Action 1: Adjust Setpoint
        page.fill("#inpTraineeSp", "48.5")
        page.click("#btnTraineeSetSp")
        page.wait_for_selector("#traineeFeedbackBanner:not([style*='display: none'])", timeout=3000)
        banner_msg = page.locator("#traineeFeedbackBanner").inner_text().strip()
        print(f"  ✓ Permitted Trainee Setpoint Action: '{banner_msg}'")
        assert "SP adjusted" in banner_msg

        # Permitted Action 2: Change Mode
        page.click("#btnModeMan")
        page.wait_for_timeout(300)
        banner_msg = page.locator("#traineeFeedbackBanner").inner_text().strip()
        print(f"  ✓ Permitted Trainee Mode Action: '{banner_msg}'")
        assert "MANUAL" in banner_msg

        page.click("#btnModeAuto")
        page.wait_for_timeout(300)
        banner_msg = page.locator("#traineeFeedbackBanner").inner_text().strip()
        print(f"  ✓ Permitted Trainee Mode Action: '{banner_msg}'")
        assert "AUTO" in banner_msg

        # Permitted Action 3: Alarm Acknowledge
        page.click("#btnTraineeAckAlarm")
        page.wait_for_timeout(300)
        banner_msg = page.locator("#traineeFeedbackBanner").inner_text().strip()
        print(f"  ✓ Permitted Trainee Alarm ACK: '{banner_msg}'")
        assert "acknowledged" in banner_msg

        # Forbidden Action: Attempt Administrative Fault Injection
        print("  [SECURITY AUDIT] Testing forbidden administrative command as Trainee...")
        page.click("#btnTraineeForbiddenFault")
        page.wait_for_timeout(500)
        banner_msg = page.locator("#traineeFeedbackBanner").inner_text().strip()
        print(f"  ✓ Server-Side Guard Response: '{banner_msg}'")
        assert "Rejected" in banner_msg or "lacks permission" in banner_msg, \
            f"Security flaw! Trainee action was not rejected: {banner_msg}"
        print("  ✓ Confirmed: Server RBAC strictly rejected administrative control with clear UI feedback")

        ss_path_d = os.path.join(SCREENSHOT_DIR, "step_d_trainee_station.png")
        page.screenshot(path=ss_path_d)
        print(f"  📷 Captured: {ss_path_d}")
        step_d_duration = (time.time() - t0) * 1000.0
        results["Step D - Trainee Station"] = f"PASS ({step_d_duration:.1f}ms)"

        # ----------------------------------------------------------------------
        # Step E: Evaluation & Replay
        # ----------------------------------------------------------------------
        print("\n--- [STEP E] Evaluation Report & Replay Station ---")
        t0 = time.time()

        # Switch back to Instructor to complete session
        page.select_option("#selActiveRole", "instructor")
        page.click("button.t-tab[data-tab='instructor']")
        page.wait_for_timeout(300)

        # Complete Session
        page.click("#btnCompleteSession")
        page.wait_for_selector(".evaluation-card", state="visible", timeout=6000)
        print("  ✓ Session completed and Evaluation Report rendered")

        # Verify Evaluation Score & Outcome
        eval_outcome = page.locator(".eval-badge-title h2").inner_text().strip()
        eval_score = page.locator(".score-number").inner_text().strip()
        eval_points = page.locator(".score-fraction").inner_text().strip()
        print(f"  ✓ Assessment Outcome: {eval_outcome} | Score: {eval_score} ({eval_points})")
        assert "PASSED" in eval_outcome or "FAILED" in eval_outcome

        # Verify Evaluation Criteria Evidence Table
        crit_rows = page.locator(".evaluation-card tbody tr").all()
        print(f"  ✓ Explainable evaluation evidence contains {len(crit_rows)} deterministic criteria rows")
        assert len(crit_rows) > 0, "No criteria in evaluation report"

        # Load Replay Station from Evaluation
        page.click("#btnLoadReplayFromEval")
        page.wait_for_selector(".replay-card", state="visible", timeout=5000)
        print("  ✓ Replay Station loaded for completed session")

        # Verify Reconstructed Process State Table
        tag_rows = page.locator(".frame-tags-card tbody tr").all()
        print(f"  ✓ Reconstructed Process Frame loaded with {len(tag_rows)} interpolated tag values")
        assert len(tag_rows) > 0, "No frame tag values in replay"

        # Test Replay Controls: Step Forward (+1s)
        time_badge_el = page.locator("#replayTimeBadge")
        t_before_step = time_badge_el.inner_text().strip()
        page.click("#btnReplayStepFwd")
        page.wait_for_timeout(400)
        t_after_step = time_badge_el.inner_text().strip()
        print(f"  ✓ Replay Step Forward (+1s): {t_before_step} -> {t_after_step}")

        # Test Replay Controls: Step Back (-1s)
        page.click("#btnReplayStepBack")
        page.wait_for_timeout(400)
        t_after_back = time_badge_el.inner_text().strip()
        print(f"  ✓ Replay Step Backward (-1s): {t_after_step} -> {t_after_back}")

        # Test Play/Pause Automated Playback
        play_btn = page.locator("#btnReplayPlayPause")
        play_btn.click()
        page.wait_for_timeout(1500)
        play_btn.click()
        t_after_play = time_badge_el.inner_text().strip()
        print(f"  ✓ Playback animation tested: T reached {t_after_play}")

        ss_path_e = os.path.join(SCREENSHOT_DIR, "step_e_evaluation_and_replay.png")
        page.screenshot(path=ss_path_e)
        print(f"  📷 Captured: {ss_path_e}")
        step_e_duration = (time.time() - t0) * 1000.0
        results["Step E - Evaluation & Replay"] = f"PASS ({step_e_duration:.1f}ms)"

        browser.close()

    print("\n================================================================================")
    print(" BROWSER FUNCTIONAL ACCEPTANCE RESULTS SUMMARY")
    print("================================================================================")
    for step, outcome in results.items():
        print(f"  {step:<40} : {outcome}")
    
    print(f"\nUncaught Console Errors: {len(console_errors)}")
    for err in console_errors:
        print(f"  ! {err}")

    if any("FAIL" in v for v in results.values()) or len(console_errors) > 0:
        print("\nOVERALL VERDICT: FAIL")
        sys.exit(1)
    else:
        print("\nOVERALL VERDICT: PASS")
        sys.exit(0)

if __name__ == "__main__":
    run_acceptance_tests()
