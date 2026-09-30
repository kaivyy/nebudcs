import os
import shutil
import time
from playwright.sync_api import sync_playwright

OUT_DIR = '/root/NebuDCS/docs/reports/screenshots/stage14_plant_recon'
ARTIFACT_DIR = '/root/.gemini/antigravity-cli/brain/c03cf0b9-4be9-4acb-b444-98c837831e07/screenshots/stage14_recon'

for d in [OUT_DIR, ARTIFACT_DIR]:
    os.makedirs(d, exist_ok=True)

def save_screenshot(page, name):
    p1 = os.path.join(OUT_DIR, f"{name}.png")
    p2 = os.path.join(ARTIFACT_DIR, f"{name}.png")
    page.screenshot(path=p1, full_page=False)
    shutil.copyfile(p1, p2)
    print(f"Captured: {name}.png")

def main():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True, args=['--no-sandbox', '--disable-setuid-sandbox'])
        context = browser.new_context(viewport={'width': 1920, 'height': 1080}, device_scale_factor=1)
        page = context.new_page()

        print("Navigating to http://127.0.0.1:8088/ ...")
        page.goto("http://127.0.0.1:8088/", wait_until="networkidle")
        page.wait_for_timeout(2500)

        # Inject Acceptance Scenario Project (TK-101 -> P-101 -> C-101 -> E-101 -> V-101 -> C-201)
        print("Injecting Section 20 Acceptance Plant Model...")
        page.evaluate("""() => {
            const project = {
                id: 'proj-acceptance-full-plant',
                name: 'Integrated Olefins & Aromatics Plant',
                schema_version: '0.1.0',
                description: 'Stage 14 Full Plant Reconstruction Acceptance Fixture',
                author: 'Principal Systems Architect',
                plant_structure: {
                    plant_name: 'Alpha Petrochemical Complex',
                    areas: [{
                        id: 'area-01',
                        name: 'Process Area 01',
                        systems: [
                            {
                                id: 'sys-lp',
                                name: 'Low Pressure Feed & Transfer System',
                                area_id: 'area-01',
                                default_display_id: 'disp-lp-overview',
                                subsystems: [{
                                    id: 'subsys-lp-feed',
                                    name: 'Feed Section',
                                    system_id: 'sys-lp',
                                    units: [{
                                        id: 'unit-lp-feed',
                                        name: 'Charge Unit',
                                        subsystem_id: 'subsys-lp-feed',
                                        system_id: 'sys-lp',
                                        area_id: 'area-01',
                                        equipment_ids: ['eq-tk101', 'eq-p101']
                                    }]
                                }]
                            },
                            {
                                id: 'sys-hp',
                                name: 'High Pressure Reaction & Compression System',
                                area_id: 'area-01',
                                default_display_id: 'disp-hp-overview',
                                subsystems: [{
                                    id: 'subsys-hp-react',
                                    name: 'Reactor Loop Section',
                                    system_id: 'sys-hp',
                                    units: [{
                                        id: 'unit-hp-compress',
                                        name: 'Recycle Compression Unit',
                                        subsystem_id: 'subsys-hp-react',
                                        system_id: 'sys-hp',
                                        area_id: 'area-01',
                                        equipment_ids: ['eq-c101', 'eq-e101']
                                    }]
                                }]
                            },
                            {
                                id: 'sys-sep',
                                name: 'Product Separation & Purification System',
                                area_id: 'area-01',
                                default_display_id: 'disp-sep-overview',
                                subsystems: [{
                                    id: 'subsys-sep-flash',
                                    name: 'Flash & Boost Section',
                                    system_id: 'sys-sep',
                                    units: [{
                                        id: 'unit-sep-flash',
                                        name: 'Flash Separation Unit',
                                        subsystem_id: 'subsys-sep-flash',
                                        system_id: 'sys-sep',
                                        area_id: 'area-01',
                                        equipment_ids: ['eq-v101', 'eq-c201']
                                    }]
                                }]
                            }
                        ]
                    }],
                    displays: []
                },
                displays: [
                    {
                        id: 'disp-plant-overview',
                        name: 'Plant Overview',
                        display_type: 'plant_overview',
                        equipment_ids: ['eq-tk101', 'eq-p101', 'eq-c101', 'eq-e101', 'eq-v101', 'eq-c201'],
                        stream_ids: ['str-01', 'str-02', 'str-03', 'str-04', 'str-05'],
                        is_default: true,
                        order: 1,
                        description: 'Global complex overview flowsheet',
                        placements: []
                    },
                    {
                        id: 'disp-lp-overview',
                        name: 'LP Feed System Overview',
                        display_type: 'system_overview',
                        system_id: 'sys-lp',
                        equipment_ids: ['eq-tk101', 'eq-p101', 'eq-c101'],
                        stream_ids: ['str-01', 'str-02'],
                        is_default: false,
                        order: 2,
                        description: 'Low-pressure feed section overview',
                        placements: []
                    },
                    {
                        id: 'disp-hp-overview',
                        name: 'HP Reaction System Overview',
                        display_type: 'system_overview',
                        system_id: 'sys-hp',
                        equipment_ids: ['eq-p101', 'eq-c101', 'eq-e101', 'eq-v101'],
                        stream_ids: ['str-02', 'str-03', 'str-04'],
                        is_default: false,
                        order: 3,
                        description: 'High-pressure reactor and compressor section',
                        placements: []
                    },
                    {
                        id: 'disp-sep-overview',
                        name: 'Separation System Overview',
                        display_type: 'system_overview',
                        system_id: 'sys-sep',
                        equipment_ids: ['eq-e101', 'eq-v101', 'eq-c201'],
                        stream_ids: ['str-04', 'str-05'],
                        is_default: false,
                        order: 4,
                        description: 'Separation and flash compression overview',
                        placements: []
                    },
                    {
                        id: 'disp-comp-overview',
                        name: 'Compression System Overview',
                        display_type: 'system_overview',
                        equipment_ids: ['eq-c101', 'eq-c201'],
                        stream_ids: [],
                        is_default: false,
                        order: 5,
                        description: 'Multi-machine rotating equipment summary',
                        placements: []
                    },
                    {
                        id: 'disp-c101-detail',
                        name: 'C-101 Recycle Compressor Detail',
                        display_type: 'unit_process_graphic',
                        system_id: 'sys-hp',
                        unit_id: 'unit-hp-compress',
                        equipment_ids: ['eq-c101'],
                        stream_ids: ['str-02', 'str-03'],
                        is_default: false,
                        order: 6,
                        description: 'C-101 detailed faceplate graphic and operating envelope',
                        placements: [
                            { equipment_id: 'eq-c101', x: 260.0, y: 280.0, width: 140.0, height: 110.0, rotation: 0 }
                        ]
                    },
                    {
                        id: 'disp-v101-detail',
                        name: 'V-101 Separator Detail',
                        display_type: 'unit_process_graphic',
                        system_id: 'sys-sep',
                        unit_id: 'unit-sep-flash',
                        equipment_ids: ['eq-v101'],
                        stream_ids: ['str-04', 'str-05'],
                        is_default: false,
                        order: 7,
                        description: 'High pressure separator vessel level and pressure detail',
                        placements: []
                    }
                ],
                navigation_links: [
                    {
                        id: 'nav-lp-to-hp',
                        source_display_id: 'disp-lp-overview',
                        source_element_id: 'eq-p101',
                        target_display_id: 'disp-hp-overview',
                        target_element_id: 'eq-c101',
                        label: 'To HP Reaction Loop (C-101)',
                        behavior: 'focus_and_center',
                        description: 'Boundary link following stream STR-02'
                    },
                    {
                        id: 'nav-hp-to-c101-detail',
                        source_display_id: 'disp-hp-overview',
                        source_element_id: 'eq-c101',
                        target_display_id: 'disp-c101-detail',
                        target_element_id: 'eq-c101',
                        label: 'Open C-101 Detail Faceplate',
                        behavior: 'focus_and_center',
                        description: 'Drill-down to equipment detail'
                    },
                    {
                        id: 'nav-hp-to-sep',
                        source_display_id: 'disp-hp-overview',
                        source_element_id: 'eq-e101',
                        target_display_id: 'disp-sep-overview',
                        target_element_id: 'eq-v101',
                        label: 'To Separation Overview (V-101)',
                        behavior: 'focus_and_center',
                        description: 'Boundary link following reactor effluent STR-04'
                    },
                    {
                        id: 'nav-comp-to-c101',
                        source_display_id: 'disp-comp-overview',
                        source_element_id: 'eq-c101',
                        target_display_id: 'disp-c101-detail',
                        target_element_id: 'eq-c101',
                        label: 'C-101 Machine Diagnostics',
                        behavior: 'focus_and_center',
                        description: 'Rotating equipment cross link'
                    }
                ],
                equipment: [
                    {
                        id: 'eq-tk101',
                        tag: 'TK-101',
                        name: 'Crude Charge Tank',
                        definition_id: 'eq-def-vessel-vertical',
                        position: { x: 100, y: 300 }
                    },
                    {
                        id: 'eq-p101',
                        tag: 'P-101',
                        name: 'Feed Booster Pump',
                        definition_id: 'eq-def-pump-centrifugal',
                        position: { x: 300, y: 300 }
                    },
                    {
                        id: 'eq-c101',
                        tag: 'C-101',
                        name: 'Recycle Gas Compressor',
                        definition_id: 'eq-def-compressor-centrifugal',
                        position: { x: 550, y: 300 }
                    },
                    {
                        id: 'eq-e101',
                        tag: 'E-101',
                        name: 'Reactor Effluent Exchanger',
                        definition_id: 'eq-def-heat-exchanger',
                        position: { x: 800, y: 300 }
                    },
                    {
                        id: 'eq-v101',
                        tag: 'V-101',
                        name: 'HP Flash Separator',
                        definition_id: 'eq-def-vessel-vertical',
                        position: { x: 1050, y: 300 }
                    },
                    {
                        id: 'eq-c201',
                        tag: 'C-201',
                        name: 'Offgas Booster Compressor',
                        definition_id: 'eq-def-compressor-centrifugal',
                        position: { x: 1300, y: 300 }
                    }
                ],
                streams: [
                    {
                        id: 'str-01',
                        name: 'Tank Effluent',
                        stream_type: 'material',
                        source_equipment_id: 'eq-tk101',
                        source_port_id: 'liquid_out',
                        sink_equipment_id: 'eq-p101',
                        sink_port_id: 'suction'
                    },
                    {
                        id: 'str-02',
                        name: 'LP to HP Feed Transfer',
                        stream_type: 'material',
                        source_equipment_id: 'eq-p101',
                        source_port_id: 'discharge',
                        sink_equipment_id: 'eq-c101',
                        sink_port_id: 'suction'
                    },
                    {
                        id: 'str-03',
                        name: 'Compressor Discharge',
                        stream_type: 'material',
                        source_equipment_id: 'eq-c101',
                        source_port_id: 'discharge',
                        sink_equipment_id: 'eq-e101',
                        sink_port_id: 'shell_in'
                    },
                    {
                        id: 'str-04',
                        name: 'Reactor Effluent Stream',
                        stream_type: 'material',
                        source_equipment_id: 'eq-e101',
                        source_port_id: 'shell_out',
                        sink_equipment_id: 'eq-v101',
                        sink_port_id: 'inlet'
                    },
                    {
                        id: 'str-05',
                        name: 'Overhead Gas Stream',
                        stream_type: 'material',
                        source_equipment_id: 'eq-v101',
                        source_port_id: 'vapor_out',
                        sink_equipment_id: 'eq-c201',
                        sink_port_id: 'suction'
                    }
                ]
            };

            // Inject into both studioStore and window.store
            if (window.studioStore) {
                window.studioStore.loadProject(project);
            }
            if (window.store) {
                window.store.project = project;
                window.store.displays = project.displays;
                window.store.activeDisplay = project.displays[0];
                window.store.notify();
            }
        }""")
        page.wait_for_timeout(1000)

        # ---------------------------------------------------------------------
        # 1. Full Plant Explorer
        # ---------------------------------------------------------------------
        print("1. audit_01_full_plant_explorer")
        page.evaluate("""() => {
            const app = document.querySelector('nebu-app');
            if (app) { app.activeMode = 'engineering'; app.render(); }
            const studio = document.querySelector('nebu-studio');
            if (studio) {
                studio.sidebarMode = 'explorer';
                studio.activeView = 'plant_structure';
                studio.render();
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "audit_01_full_plant_explorer")

        # ---------------------------------------------------------------------
        # 2. Physical topology
        # ---------------------------------------------------------------------
        print("2. audit_02_physical_topology")
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
        save_screenshot(page, "audit_02_physical_topology")

        # ---------------------------------------------------------------------
        # 3. System mapping
        # ---------------------------------------------------------------------
        print("3. audit_03_system_mapping")
        page.evaluate("""() => {
            const studio = document.querySelector('nebu-studio');
            if (studio) {
                studio.activeView = 'plant_structure';
                studio.render();
            }
            const editor = document.querySelector('nebu-plant-structure-editor');
            if (editor) {
                const sysBtn = editor.querySelector('[data-select-system]');
                if (sysBtn) sysBtn.click();
            }
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "audit_03_system_mapping")

        # ---------------------------------------------------------------------
        # 4. Unit mapping
        # ---------------------------------------------------------------------
        print("4. audit_04_unit_mapping")
        page.evaluate("""() => {
            const editor = document.querySelector('nebu-plant-structure-editor');
            if (editor) {
                const unitBtn = editor.querySelector('[data-select-unit]');
                if (unitBtn) unitBtn.click();
            }
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "audit_04_unit_mapping")

        # ---------------------------------------------------------------------
        # 5. Display mapping
        # ---------------------------------------------------------------------
        print("5. audit_05_display_mapping")
        page.evaluate("""() => {
            const studio = document.querySelector('nebu-studio');
            if (studio) {
                studio.sidebarMode = 'displays';
                studio.render();
            }
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "audit_05_display_mapping")

        # ---------------------------------------------------------------------
        # 6. C-101 referenced on multiple displays
        # ---------------------------------------------------------------------
        print("6. audit_06_c101_multi_display")
        page.evaluate("""() => {
            const studio = document.querySelector('nebu-studio');
            if (studio) {
                studio.activeView = 'flowsheet';
                studio.render();
            }
            if (window.studioStore) {
                window.studioStore.selectEntity('eq-c101');
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "audit_06_c101_multi_display")

        # ---------------------------------------------------------------------
        # 7. LP/HP System boundary
        # ---------------------------------------------------------------------
        print("7. audit_07_lphp_system_boundary")
        page.evaluate("""() => {
            if (window.studioStore) {
                window.studioStore.setActiveDisplay('disp-lp-overview');
                window.studioStore.selectEntity('eq-p101');
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "audit_07_lphp_system_boundary")

        # ---------------------------------------------------------------------
        # 8. Cross-display stream continuation suggestion
        # ---------------------------------------------------------------------
        print("8. audit_08_cross_display_stream_continuation")
        page.evaluate("""() => {
            const inspector = document.querySelector('nebu-property-inspector');
            if (inspector) inspector.render();
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "audit_08_cross_display_stream_continuation")

        # ---------------------------------------------------------------------
        # 9. Navigation editor
        # ---------------------------------------------------------------------
        print("9. audit_09_navigation_editor")
        page.evaluate("""() => {
            const studio = document.querySelector('nebu-studio');
            if (studio) {
                studio.sidebarMode = 'navigation';
                studio.render();
            }
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "audit_09_navigation_editor")

        # ---------------------------------------------------------------------
        # 10. Navigation target selection
        # ---------------------------------------------------------------------
        print("10. audit_10_navigation_target_selection")
        page.evaluate("""() => {
            const navEditor = document.querySelector('nebu-navigation-editor');
            if (navEditor) {
                const addBtn = navEditor.querySelector('.btn-add-nav');
                if (addBtn) addBtn.click();
            }
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "audit_10_navigation_target_selection")

        # ---------------------------------------------------------------------
        # 11. Validation errors
        # ---------------------------------------------------------------------
        print("11. audit_11_validation_errors")
        page.evaluate("""() => {
            const studio = document.querySelector('nebu-studio');
            if (studio) {
                studio.sidebarMode = 'validation';
                studio.render();
            }
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "audit_11_validation_errors")

        # ---------------------------------------------------------------------
        # 12. Valid deployment
        # ---------------------------------------------------------------------
        print("12. audit_12_valid_deployment")
        page.evaluate("""() => {
            const studio = document.querySelector('nebu-studio');
            if (studio) {
                studio.sidebarMode = 'deploy';
                studio.render();
            }
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "audit_12_valid_deployment")

        # ---------------------------------------------------------------------
        # 13. LP Overview (Operations)
        # ---------------------------------------------------------------------
        print("13. audit_13_dcs_lp_overview")
        page.evaluate("""() => {
            const app = document.querySelector('nebu-app');
            if (app) { app.activeMode = 'operations'; app.render(); }
            if (window.store) {
                window.store.navigateDisplay('disp-lp-overview');
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "audit_13_dcs_lp_overview")

        # ---------------------------------------------------------------------
        # 14. HP Overview (Operations)
        # ---------------------------------------------------------------------
        print("14. audit_14_dcs_hp_overview")
        page.evaluate("""() => {
            if (window.store) {
                window.store.navigateDisplay('disp-hp-overview');
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "audit_14_dcs_hp_overview")

        # ---------------------------------------------------------------------
        # 15. Compression Overview (Operations)
        # ---------------------------------------------------------------------
        print("15. audit_15_dcs_compression_overview")
        page.evaluate("""() => {
            if (window.store) {
                window.store.navigateDisplay('disp-comp-overview');
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "audit_15_dcs_compression_overview")

        # ---------------------------------------------------------------------
        # 16. C-101 Detail (Operations)
        # ---------------------------------------------------------------------
        print("16. audit_16_dcs_c101_detail")
        page.evaluate("""() => {
            if (window.store) {
                window.store.navigateDisplay('disp-c101-detail');
                window.store.selectEquipment('eq-c101');
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "audit_16_dcs_c101_detail")

        # ---------------------------------------------------------------------
        # 17. Navigation from LP -> HP
        # ---------------------------------------------------------------------
        print("17. audit_17_dcs_navigation_lp_to_hp")
        page.evaluate("""() => {
            if (window.store) {
                window.store.navigateDisplay('disp-lp-overview');
            }
        }""")
        page.wait_for_timeout(800)
        page.evaluate("""() => {
            // Trigger navigation link
            if (window.store) {
                window.store.executeNavigationLink('nav-lp-to-hp');
            }
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "audit_17_dcs_navigation_lp_to_hp")

        # ---------------------------------------------------------------------
        # 18. Destination equipment focus
        # ---------------------------------------------------------------------
        print("18. audit_18_dcs_destination_focus")
        page.evaluate("""() => {
            if (window.store) {
                window.store.focusedEquipmentId = 'eq-c101';
                window.store.notify();
            }
            const canvas = document.querySelector('nebu-process-view');
            if (canvas && canvas.centerOnEntity) canvas.centerOnEntity('eq-c101');
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "audit_18_dcs_destination_focus")

        # ---------------------------------------------------------------------
        # 19. Large P&ID
        # ---------------------------------------------------------------------
        print("19. audit_19_large_pid")
        page.evaluate("""() => {
            const app = document.querySelector('nebu-app');
            if (app) { app.activeMode = 'engineering'; app.render(); }
            const studio = document.querySelector('nebu-studio');
            if (studio) {
                studio.activeView = 'flowsheet';
                studio.render();
            }
            if (window.studioStore) {
                window.studioStore.setActiveDisplay('disp-plant-overview');
            }
            const canvas = document.querySelector('nebu-pid-canvas');
            if (canvas && canvas.fitToView) canvas.fitToView();
        }""")
        page.wait_for_timeout(1000)
        save_screenshot(page, "audit_19_large_pid")

        # ---------------------------------------------------------------------
        # 20. Custom non-refinery plant
        # ---------------------------------------------------------------------
        print("20. audit_20_custom_non_refinery_plant")
        page.evaluate("""async () => {
            // Switch to Chemical continuous exothermic reaction plant
            const studio = document.querySelector('nebu-studio');
            if (studio && studio['loadProject']) {
                await studio['loadProject']('proj-domain-chem-reaction');
                studio.sidebarMode = 'explorer';
                studio.activeView = 'plant_structure';
                studio.render();
            }
        }""")
        page.wait_for_timeout(1500)
        save_screenshot(page, "audit_20_custom_non_refinery_plant")

        browser.close()
        print("All 20 visual acceptance screenshots successfully captured!")

if __name__ == '__main__':
    main()
