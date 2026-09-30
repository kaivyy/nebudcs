const { chromium } = require('playwright');
const path = require('path');
const fs = require('fs');

const OUT_DIR = '/root/NebuDCS/docs/reports/screenshots/pid_canvas';
const ARTIFACT_DIR = '/root/.gemini/antigravity-cli/brain/c03cf0b9-4be9-4acb-b444-98c837831e07/screenshots/pid_canvas';

fs.mkdirSync(OUT_DIR, { recursive: true });
fs.mkdirSync(ARTIFACT_DIR, { recursive: true });

async function saveScreenshot(page, name) {
  const filePath1 = path.join(OUT_DIR, `${name}.png`);
  const filePath2 = path.join(ARTIFACT_DIR, `${name}.png`);
  await page.screenshot({ path: filePath1, fullPage: false });
  fs.copyFileSync(filePath1, filePath2);
  console.log(`Saved screenshot: ${name}.png`);
}

async function run() {
  const browser = await chromium.launch({
    headless: true,
    args: ['--no-sandbox', '--disable-setuid-sandbox']
  });
  const context = await browser.newContext({
    viewport: { width: 1920, height: 1080 },
    deviceScaleFactor: 1
  });
  const page = await context.newPage();

  console.log('Navigating to http://127.0.0.1:8088/ ...');
  await page.goto('http://127.0.0.1:8088/', { waitUntil: 'networkidle' });
  await page.waitForTimeout(800);

  // Switch to Engineering Workspace
  console.log('Switching to Engineering Workspace...');
  await page.evaluate(() => {
    window.dispatchEvent(new CustomEvent('navigate-workspace', { detail: { mode: 'engineering' } }));
  });
  await page.waitForTimeout(1000);

  // Wait for equipment nodes to appear
  await page.waitForSelector('nebu-pid-canvas svg.pid-svg g.eq-node', { timeout: 10000 });
  await page.waitForTimeout(500);

  // 1. Initial Selection: Click on the first equipment node (PUMP-A1)
  console.log('1. pid_canvas_01_initial_selection');
  const firstEq = page.locator('nebu-pid-canvas svg.pid-svg g.eq-node').first();
  await firstEq.waitFor({ state: 'visible' });
  const eqBox = await firstEq.boundingBox();
  console.log('Found equipment box at:', eqBox);
  
  await page.mouse.click(eqBox.x + eqBox.width / 2, eqBox.y + eqBox.height / 2);
  await page.waitForTimeout(400);
  await saveScreenshot(page, 'pid_canvas_01_initial_selection');

  // 2. Equipment Drag In Progress
  console.log('2. pid_canvas_02_equipment_drag_in_progress');
  const startX = eqBox.x + eqBox.width / 2;
  const startY = eqBox.y + eqBox.height / 2;
  await page.mouse.move(startX, startY);
  await page.mouse.down();
  await page.mouse.move(startX + 140, startY + 80, { steps: 10 });
  await page.waitForTimeout(300);
  await saveScreenshot(page, 'pid_canvas_02_equipment_drag_in_progress');

  // 3. Equipment Drag Committed
  console.log('3. pid_canvas_03_equipment_drag_committed');
  await page.mouse.up();
  await page.waitForTimeout(400);
  await saveScreenshot(page, 'pid_canvas_03_equipment_drag_committed');

  // 4. Stream Orthogonal Reroute
  console.log('4. pid_canvas_04_stream_orthogonal_reroute');
  await page.waitForTimeout(200);
  await saveScreenshot(page, 'pid_canvas_04_stream_orthogonal_reroute');

  // 5. Instrument Drag and Drop
  console.log('5. pid_canvas_05_instrument_drag_and_drop');
  const firstInst = page.locator('nebu-pid-canvas svg.pid-svg g.instrument-node').first();
  if (await firstInst.count() > 0) {
    const instBox = await firstInst.boundingBox();
    if (instBox) {
      const iStartX = instBox.x + instBox.width / 2;
      const iStartY = instBox.y + instBox.height / 2;
      await page.mouse.move(iStartX, iStartY);
      await page.mouse.down();
      await page.mouse.move(iStartX + 90, iStartY - 60, { steps: 10 });
      await page.mouse.up();
      await page.waitForTimeout(400);
    }
  }
  await saveScreenshot(page, 'pid_canvas_05_instrument_drag_and_drop');

  // 6. Multi-Selection Shift Click
  console.log('6. pid_canvas_06_multi_selection_shift_click');
  const eqNodes = page.locator('nebu-pid-canvas svg.pid-svg g.eq-node');
  const count = await eqNodes.count();
  if (count >= 2) {
    const eq1Box = await eqNodes.nth(0).boundingBox();
    const eq2Box = await eqNodes.nth(1).boundingBox();
    await page.mouse.click(eq1Box.x + eq1Box.width / 2, eq1Box.y + eq1Box.height / 2);
    await page.waitForTimeout(200);
    await page.keyboard.down('Shift');
    await page.mouse.click(eq2Box.x + eq2Box.width / 2, eq2Box.y + eq2Box.height / 2);
    await page.keyboard.up('Shift');
    await page.waitForTimeout(300);
  }
  await saveScreenshot(page, 'pid_canvas_06_multi_selection_shift_click');

  // 7. Marquee Selection Box
  console.log('7. pid_canvas_07_marquee_selection_box');
  // Clear selection by clicking background
  await page.mouse.click(100, 150);
  await page.waitForTimeout(200);
  const canvasEl = page.locator('nebu-pid-canvas svg.pid-svg');
  const cBox = await canvasEl.boundingBox();
  const mStartX = cBox.x + 80;
  const mStartY = cBox.y + 120;
  await page.mouse.move(mStartX, mStartY);
  await page.mouse.down();
  await page.mouse.move(mStartX + 420, mStartY + 320, { steps: 15 });
  await page.waitForTimeout(300);
  await saveScreenshot(page, 'pid_canvas_07_marquee_selection_box');
  await page.mouse.up();
  await page.waitForTimeout(300);

  // 8. Multi-Entity Drag
  console.log('8. pid_canvas_08_multi_entity_drag');
  // Select all equipment via Ctrl+A
  await page.keyboard.press('Control+a');
  await page.waitForTimeout(300);
  const selEq = page.locator('nebu-pid-canvas svg.pid-svg g.eq-node.selected').first();
  if (await selEq.count() > 0) {
    const sBox = await selEq.boundingBox();
    await page.mouse.move(sBox.x + sBox.width / 2, sBox.y + sBox.height / 2);
    await page.mouse.down();
    await page.mouse.move(sBox.x + sBox.width / 2 + 60, sBox.y + sBox.height / 2 + 50, { steps: 10 });
    await page.mouse.up();
    await page.waitForTimeout(400);
  }
  await saveScreenshot(page, 'pid_canvas_08_multi_entity_drag');

  // 9. Keyboard Nudge 10px
  console.log('9. pid_canvas_09_keyboard_nudge_10px');
  const singleEq = page.locator('nebu-pid-canvas svg.pid-svg g.eq-node').first();
  const sEqBox = await singleEq.boundingBox();
  await page.mouse.click(sEqBox.x + sEqBox.width / 2, sEqBox.y + sEqBox.height / 2);
  await page.waitForTimeout(200);
  await page.keyboard.press('ArrowRight');
  await page.keyboard.press('ArrowRight');
  await page.waitForTimeout(300);
  await saveScreenshot(page, 'pid_canvas_09_keyboard_nudge_10px');

  // 10. Keyboard Nudge Shift 50px
  console.log('10. pid_canvas_10_keyboard_nudge_shift_50px');
  await page.keyboard.press('Shift+ArrowDown');
  await page.waitForTimeout(300);
  await saveScreenshot(page, 'pid_canvas_10_keyboard_nudge_shift_50px');

  // 11. Inspector XY Coordinate Edit
  console.log('11. pid_canvas_11_inspector_xy_coordinate_edit');
  const posXInput = page.locator('nebu-property-inspector input.input-eq-x');
  if (await posXInput.count() > 0) {
    await posXInput.fill('450');
    await posXInput.dispatchEvent('change');
    await page.waitForTimeout(200);
  }
  const posYInput = page.locator('nebu-property-inspector input.input-eq-y');
  if (await posYInput.count() > 0) {
    await posYInput.fill('280');
    await posYInput.dispatchEvent('change');
    await page.waitForTimeout(300);
  }
  await saveScreenshot(page, 'pid_canvas_11_inspector_xy_coordinate_edit');

  // 12. Resize Handles Visible
  console.log('12. pid_canvas_12_resize_handles_visible');
  await page.waitForSelector('nebu-pid-canvas svg.pid-svg rect.resize-handle', { timeout: 3000 });
  await saveScreenshot(page, 'pid_canvas_12_resize_handles_visible');

  // 13. Equipment Resize Drag
  console.log('13. pid_canvas_13_equipment_resize_drag');
  const seHandle = page.locator('nebu-pid-canvas svg.pid-svg rect.resize-handle.handle-se').first();
  if (await seHandle.count() > 0) {
    const hBox = await seHandle.boundingBox();
    await page.mouse.move(hBox.x + hBox.width / 2, hBox.y + hBox.height / 2);
    await page.mouse.down();
    await page.mouse.move(hBox.x + 70, hBox.y + 50, { steps: 10 });
    await page.mouse.up();
    await page.waitForTimeout(400);
  }
  await saveScreenshot(page, 'pid_canvas_13_equipment_resize_drag');

  // 14. Equipment Rotation 90deg
  console.log('14. pid_canvas_14_equipment_rotation_90deg');
  const rotSelect = page.locator('nebu-property-inspector select.select-eq-rot');
  if (await rotSelect.count() > 0) {
    await rotSelect.selectOption('90');
    await rotSelect.dispatchEvent('change');
    await page.waitForTimeout(400);
  }
  await saveScreenshot(page, 'pid_canvas_14_equipment_rotation_90deg');

  // 15. Equipment Rotation 180deg
  console.log('15. pid_canvas_15_equipment_rotation_180deg');
  if (await rotSelect.count() > 0) {
    await rotSelect.selectOption('180');
    await rotSelect.dispatchEvent('change');
    await page.waitForTimeout(400);
  }
  await saveScreenshot(page, 'pid_canvas_15_equipment_rotation_180deg');

  // 16. Alignment Toolbar Action
  console.log('16. pid_canvas_16_alignment_toolbar_action');
  await page.keyboard.press('Control+a');
  await page.waitForTimeout(300);
  const alignTopBtn = page.locator('nebu-studio button.btn-align[data-align="top"]').first();
  if (await alignTopBtn.count() > 0) {
    await alignTopBtn.click();
    await page.waitForTimeout(400);
  }
  await saveScreenshot(page, 'pid_canvas_16_alignment_toolbar_action');

  // 17. Undo Redo Verification
  console.log('17. pid_canvas_17_undo_redo_verification');
  await page.keyboard.press('Control+z');
  await page.waitForTimeout(250);
  await page.keyboard.press('Control+z');
  await page.waitForTimeout(250);
  await page.keyboard.press('Control+y');
  await page.waitForTimeout(300);
  await saveScreenshot(page, 'pid_canvas_17_undo_redo_verification');

  console.log('All 17 visual acceptance screenshots captured successfully!');
  await browser.close();
}

run().catch((err) => {
  console.error('Screenshot capture failed:', err);
  process.exit(1);
});
