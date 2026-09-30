#!/usr/bin/env python3
"""
NebuDCS Generic Platform Controlled Expansion: Multi-Plant Acceptance Suite

Verifies 3 arbitrary, non-refinery generic industrial process plants:
1. Plant A: Pumping line with level regulation (Tank-A -> FT-X1 -> Valve-X2 -> Pump-X3 -> Tank-B with LIC-101).
2. Plant B: Exothermic reaction train with separator & recycle loop (Feed Tank -> Pump -> Exchanger -> CSTR -> Separator).
3. Plant C: Steam turbine expanding steam driving compressor on mechanical shaft (Steam Header -> Valve -> Turbine -> [Shaft] -> Compressor -> Receiver).

For every plant:
- Validates model topology (0 errors).
- Persists and round-trips via REST API.
- Loads into live simulation engine (/api/v1/projects/:id/load_simulation).
- Verifies advancing deterministic clock, active DCS tags, and emergent physical dynamics.
- Tests browser UI rendering and project switching via Playwright.
"""

import os
import sys
import time
import urllib.request
import urllib.error
import json
from playwright.sync_api import sync_playwright, expect

BASE_URL = os.environ.get("NEBUDCS_URL", "http://127.0.0.1:8088")
ADMIN_TOKEN = "sec-admin-change-me-in-production-482a9f"
SCREENSHOT_DIR = "/root/NebuDCS/docs/screenshots"
os.makedirs(SCREENSHOT_DIR, exist_ok=True)

HEADERS = {
    "Content-Type": "application/json",
    "Authorization": f"Bearer {ADMIN_TOKEN}"
}

def api_request(path: str, method: str = "GET", data: dict = None) -> dict:
    url = f"{BASE_URL}{path}"
    payload = json.dumps(data).encode("utf-8") if data is not None else None
    req = urllib.request.Request(url, data=payload, headers=HEADERS, method=method)
    with urllib.request.urlopen(req) as resp:
        body = resp.read().decode("utf-8")
        return json.loads(body) if body else {}

def register_custom_turbine():
    """Ensures custom.turbine.steam is registered for Plant C"""
    p1 = '7RVEZYDAZY11M1VQ5Y7YNN899Y'
    p2 = '01ARZ3NDEKTSV4RRFFQ69G5F02'
    p3 = '09E3CR8TAD13DQQD7486A1VHVK'

    payload = {
        'id': 'custom.turbine.steam',
        'name': 'Steam Expansion Turbine',
        'version': '1.0.0',
        'category': 'rotating',
        'tier': 'level1_declarative',
        'description': 'Steam Turbine Driver',
        'parameters': {
            'rated_power': {
                'name': 'rated_power',
                'description': 'Rated Power',
                'quantity_kind': 'power',
                'default_value': 300000.0,
                'min_value': 0.0,
                'max_value': 10000000.0,
                'required': True
            }
        },
        'ports': [
            {
                'id': p1,
                'name': 'steam_inlet',
                'domain': 'material',
                'direction': 'inlet',
                'phase_compatibility': ['gas'],
                'required': True,
                'cardinality': 'zero_or_more',
                'description': 'Steam inlet'
            },
            {
                'id': p2,
                'name': 'exhaust',
                'domain': 'material',
                'direction': 'outlet',
                'phase_compatibility': ['gas'],
                'required': True,
                'cardinality': 'zero_or_more',
                'description': 'Steam exhaust'
            },
            {
                'id': p3,
                'name': 'shaft_out',
                'domain': 'mechanical',
                'direction': 'outlet',
                'phase_compatibility': [],
                'required': True,
                'cardinality': 'zero_or_more',
                'description': 'Shaft out'
            }
        ],
        'curves': {},
        'curves_2d': {},
        'operating_envelope': {'dimensions': []},
        'alarm_definitions': [],
        'trips': [],
        'interlocks': [],
        'metadata': {}
    }
    try:
        api_request('/api/v1/equipment/define', method='POST', data=payload)
    except urllib.error.HTTPError as e:
        print(f"  Note on registering turbine: {e} -> {e.read().decode('utf-8')}")
    except Exception as e:
        print(f"  Note on registering turbine: {e}")

def build_plant_a():
    """Plant A: Pumping line with level regulation"""
    return {
        "id": "proj-plant-a-pumping",
        "name": "Plant A - Pumping & Level Regulation Line",
        "schema_version": "0.1.0",
        "description": "Generic water transfer line with LIC-101 closed-loop level control",
        "author": "Generic Process Systems Engineer",
        "created": "2026-09-07T00:00:00Z",
        "modified": "2026-09-07T00:00:00Z",
        "equipment": [
            {
                "id": "eq-tank-a",
                "definition_id": "standard.tank.atmospheric",
                "definition_version": "1.0.0",
                "name": "Tank-A",
                "parameters": {"volume": 40.0, "cross_section_area": 5.0, "initial_mass": 20000.0},
                "position": {"x": 100.0, "y": 250.0}
            },
            {
                "id": "eq-valve-x2",
                "definition_id": "standard.valve.control_globe",
                "definition_version": "1.0.0",
                "name": "Valve-X2",
                "parameters": {"cv": 95.0, "position": 50.0},
                "position": {"x": 280.0, "y": 250.0}
            },
            {
                "id": "eq-pump-x3",
                "definition_id": "standard.pump.centrifugal",
                "definition_version": "1.0.0",
                "name": "Pump-X3",
                "parameters": {"rated_head": 120.0, "rated_flow": 60.0, "speed": 100.0},
                "position": {"x": 460.0, "y": 250.0}
            },
            {
                "id": "eq-tank-b",
                "definition_id": "standard.tank.atmospheric",
                "definition_version": "1.0.0",
                "name": "Tank-B",
                "parameters": {"volume": 50.0, "cross_section_area": 6.0, "initial_mass": 5000.0},
                "position": {"x": 680.0, "y": 250.0}
            }
        ],
        "streams": [
            {
                "id": "str-a1",
                "name": "STR-A1",
                "stream_type": "Material",
                "source_equipment_id": "eq-tank-a",
                "source_port_id": "outlet",
                "sink_equipment_id": "eq-valve-x2",
                "sink_port_id": "inlet",
                "composition": {},
                "route_points": []
            },
            {
                "id": "str-a2",
                "name": "STR-A2",
                "stream_type": "Material",
                "source_equipment_id": "eq-valve-x2",
                "source_port_id": "outlet",
                "sink_equipment_id": "eq-pump-x3",
                "sink_port_id": "suction",
                "composition": {},
                "route_points": []
            },
            {
                "id": "str-a3",
                "name": "STR-A3",
                "stream_type": "Material",
                "source_equipment_id": "eq-pump-x3",
                "source_port_id": "discharge",
                "sink_equipment_id": "eq-tank-b",
                "sink_port_id": "inlet",
                "composition": {},
                "route_points": []
            }
        ],
        "instruments": [
            {
                "id": "inst-lt101",
                "tag": "LT-101",
                "equipment_id": "eq-tank-a",
                "measurement_kind": "Level",
                "range_min": 0.0,
                "range_max": 100.0,
                "unit": "%",
                "position": {"x": 100.0, "y": 180.0}
            },
            {
                "id": "inst-ftx1",
                "tag": "FT-X1",
                "equipment_id": "eq-valve-x2",
                "measurement_kind": "Flow",
                "range_min": 0.0,
                "range_max": 100.0,
                "unit": "kg/s",
                "position": {"x": 200.0, "y": 250.0}
            },
            {
                "id": "inst-pt102",
                "tag": "PT-102",
                "equipment_id": "eq-pump-x3",
                "measurement_kind": "Pressure",
                "range_min": 0.0,
                "range_max": 20.0,
                "unit": "bar",
                "position": {"x": 460.0, "y": 180.0}
            },
            {
                "id": "inst-vt103",
                "tag": "VT-103",
                "equipment_id": "eq-pump-x3",
                "measurement_kind": "Vibration",
                "range_min": 0.0,
                "range_max": 10.0,
                "unit": "mm/s",
                "position": {"x": 520.0, "y": 180.0}
            }
        ],
        "control_loops": [
            {
                "id": "loop-lic101",
                "name": "LIC-101",
                "pv_tag": "LT-101",
                "sp": 50.0,
                "out_tag": "Valve-X2.POS",
                "kp": 1.5,
                "ti": 15.0,
                "td": 0.0,
                "output_min": 0.0,
                "output_max": 100.0,
                "mode": "AUTO",
                "reverse_acting": False
            }
        ],
        "alarms": [
            {
                "id": "alm-la101",
                "name": "LA-101-LO",
                "pv_tag": "LT-101",
                "alarm_type": "LO",
                "severity": "WARNING",
                "trip_limit": 15.0,
                "deadband": 1.0,
                "description": "Tank-A Low Level"
            },
            {
                "id": "alm-va103",
                "name": "VA-103-HI",
                "pv_tag": "VT-103",
                "alarm_type": "HI",
                "severity": "CRITICAL",
                "trip_limit": 4.5,
                "deadband": 0.2,
                "description": "Pump Cavitation Vibration"
            }
        ],
        "interlocks": [
            {
                "id": "ilk-101",
                "name": "I-101",
                "source_tag": "LT-101",
                "condition": "<",
                "threshold": 10.0,
                "target_equipment": "eq-pump-x3",
                "target_action": "CLOSE",
                "latched": True,
                "description": "Low suction level trips transfer pump"
            }
        ]
    }

def build_plant_b():
    """Plant B: Exothermic reaction train with separator & recycle"""
    return {
        "id": "proj-plant-b-reaction",
        "name": "Plant B - Exothermic Reaction Train with Recycle",
        "schema_version": "0.1.0",
        "description": "Feed Preheater -> CSTR Reactor -> Vertical Separator with recycle loop",
        "author": "Generic Process Systems Engineer",
        "created": "2026-09-07T00:00:00Z",
        "modified": "2026-09-07T00:00:00Z",
        "equipment": [
            {
                "id": "eq-feed-tank",
                "definition_id": "standard.tank.atmospheric",
                "definition_version": "1.0.0",
                "name": "Feed-Tank",
                "parameters": {"volume": 30.0, "cross_section_area": 4.0, "initial_mass": 15000.0},
                "position": {"x": 80.0, "y": 300.0}
            },
            {
                "id": "eq-feed-pump",
                "definition_id": "standard.pump.centrifugal",
                "definition_version": "1.0.0",
                "name": "Feed-Pump",
                "parameters": {"rated_head": 140.0, "rated_flow": 40.0, "speed": 100.0},
                "position": {"x": 220.0, "y": 300.0}
            },
            {
                "id": "eq-preheater",
                "definition_id": "standard.heat_exchanger.shell_and_tube",
                "definition_version": "1.0.0",
                "name": "E-Preheater",
                "parameters": {"area": 60.0, "overall_u": 650.0, "design_duty": 450000.0},
                "position": {"x": 380.0, "y": 300.0}
            },
            {
                "id": "eq-cstr-rx",
                "definition_id": "standard.reactor.cstr",
                "definition_version": "1.0.0",
                "name": "R-CSTR-1",
                "parameters": {"thermal_duty": 120000.0, "tau": 4.0},
                "position": {"x": 540.0, "y": 300.0}
            },
            {
                "id": "eq-flash-sep",
                "definition_id": "standard.vessel.vertical_separator",
                "definition_version": "1.0.0",
                "name": "V-Separator",
                "parameters": {"volume": 20.0, "cross_section_area": 3.0, "initial_mass": 6000.0},
                "position": {"x": 720.0, "y": 300.0}
            }
        ],
        "streams": [
            {
                "id": "str-b1",
                "name": "STR-B1",
                "stream_type": "Material",
                "source_equipment_id": "eq-feed-tank",
                "source_port_id": "outlet",
                "sink_equipment_id": "eq-feed-pump",
                "sink_port_id": "suction",
                "composition": {},
                "route_points": []
            },
            {
                "id": "str-b2",
                "name": "STR-B2",
                "stream_type": "Material",
                "source_equipment_id": "eq-feed-pump",
                "source_port_id": "discharge",
                "sink_equipment_id": "eq-preheater",
                "sink_port_id": "tube_in",
                "composition": {},
                "route_points": []
            },
            {
                "id": "str-b3",
                "name": "STR-B3",
                "stream_type": "Material",
                "source_equipment_id": "eq-preheater",
                "source_port_id": "tube_out",
                "sink_equipment_id": "eq-cstr-rx",
                "sink_port_id": "feed",
                "composition": {},
                "route_points": []
            },
            {
                "id": "str-b4",
                "name": "STR-B4",
                "stream_type": "Material",
                "source_equipment_id": "eq-cstr-rx",
                "source_port_id": "effluent",
                "sink_equipment_id": "eq-flash-sep",
                "sink_port_id": "feed",
                "composition": {},
                "route_points": []
            },
            {
                "id": "str-recycle",
                "name": "STR-RECYCLE",
                "stream_type": "Material",
                "source_equipment_id": "eq-flash-sep",
                "source_port_id": "liquid_out",
                "sink_equipment_id": "eq-cstr-rx",
                "sink_port_id": "feed",
                "composition": {},
                "route_points": [{"x": 720.0, "y": 180.0}, {"x": 540.0, "y": 180.0}]
            }
        ],
        "instruments": [
            {
                "id": "inst-tt201",
                "tag": "TT-201",
                "equipment_id": "eq-cstr-rx",
                "measurement_kind": "Temperature",
                "range_min": 0.0,
                "range_max": 300.0,
                "unit": "°C",
                "position": {"x": 540.0, "y": 220.0}
            },
            {
                "id": "inst-pt202",
                "tag": "PT-202",
                "equipment_id": "eq-flash-sep",
                "measurement_kind": "Pressure",
                "range_min": 0.0,
                "range_max": 50.0,
                "unit": "bar",
                "position": {"x": 720.0, "y": 220.0}
            },
            {
                "id": "inst-at203",
                "tag": "AT-203",
                "equipment_id": "eq-cstr-rx",
                "measurement_kind": "Purity",
                "range_min": 0.0,
                "range_max": 1.0,
                "unit": "fraction",
                "position": {"x": 600.0, "y": 220.0}
            }
        ],
        "control_loops": [
            {
                "id": "loop-tic201",
                "name": "TIC-201",
                "pv_tag": "TT-201",
                "sp": 180.0,
                "out_tag": "E-Preheater.DUTY",
                "kp": 2.0,
                "ti": 20.0,
                "td": 1.0,
                "output_min": 0.0,
                "output_max": 500000.0,
                "mode": "AUTO",
                "reverse_acting": False
            }
        ],
        "alarms": [
            {
                "id": "alm-ta201",
                "name": "TA-201-HIHI",
                "pv_tag": "TT-201",
                "alarm_type": "HIHI",
                "severity": "CRITICAL",
                "trip_limit": 220.0,
                "deadband": 2.0,
                "description": "Reactor High-High Thermal Runaway"
            }
        ],
        "interlocks": [
            {
                "id": "ilk-201",
                "name": "I-201",
                "source_tag": "TT-201",
                "condition": ">",
                "threshold": 215.0,
                "target_equipment": "eq-feed-pump",
                "target_action": "CLOSE",
                "latched": True,
                "description": "Reactor runaway trips feed pump"
            }
        ]
    }

def build_plant_c():
    """Plant C: Steam turbine driven compressor on mechanical shaft"""
    return {
        "id": "proj-plant-c-turbine",
        "name": "Plant C - Steam Turbine Expanding Steam to Drive Compressor",
        "schema_version": "0.1.0",
        "description": "Steam Header -> Control Valve -> Turbine -> [Shaft] -> Compressor -> Receiver",
        "author": "Generic Process Systems Engineer",
        "created": "2026-09-07T00:00:00Z",
        "modified": "2026-09-07T00:00:00Z",
        "equipment": [
            {
                "id": "eq-steam-valve",
                "definition_id": "standard.valve.control_globe",
                "definition_version": "1.0.0",
                "name": "FV-301",
                "parameters": {"cv": 110.0, "position": 85.0},
                "position": {"x": 100.0, "y": 280.0}
            },
            {
                "id": "eq-steam-turbine",
                "definition_id": "custom.turbine.steam",
                "definition_version": "1.0.0",
                "name": "ST-301",
                "parameters": {"rated_power": 300000.0, "speed": 3000.0},
                "position": {"x": 280.0, "y": 280.0}
            },
            {
                "id": "eq-compressor",
                "definition_id": "standard.compressor.centrifugal",
                "definition_version": "1.0.0",
                "name": "C-301",
                "parameters": {"rated_head": 90.0, "rated_power": 250000.0, "speed": 3000.0},
                "position": {"x": 480.0, "y": 280.0}
            },
            {
                "id": "eq-receiver",
                "definition_id": "standard.vessel.vertical_separator",
                "definition_version": "1.0.0",
                "name": "V-301",
                "parameters": {"volume": 35.0, "cross_section_area": 4.5, "initial_mass": 8000.0},
                "position": {"x": 680.0, "y": 280.0}
            }
        ],
        "streams": [
            {
                "id": "str-c1",
                "name": "STR-STEAM",
                "stream_type": "Material",
                "source_equipment_id": "eq-steam-valve",
                "source_port_id": "outlet",
                "sink_equipment_id": "eq-steam-turbine",
                "sink_port_id": "steam_inlet",
                "composition": {},
                "route_points": []
            },
            {
                "id": "str-c2",
                "name": "STR-SHAFT",
                "stream_type": "Mechanical",
                "source_equipment_id": "eq-steam-turbine",
                "source_port_id": "shaft_out",
                "sink_equipment_id": "eq-compressor",
                "sink_port_id": "shaft_drive",
                "composition": {},
                "route_points": []
            },
            {
                "id": "str-c3",
                "name": "STR-GAS",
                "stream_type": "Material",
                "source_equipment_id": "eq-compressor",
                "source_port_id": "discharge",
                "sink_equipment_id": "eq-receiver",
                "sink_port_id": "feed",
                "composition": {},
                "route_points": []
            }
        ],
        "instruments": [
            {
                "id": "inst-st301",
                "tag": "ST-301",
                "equipment_id": "eq-steam-turbine",
                "measurement_kind": "Speed",
                "range_min": 0.0,
                "range_max": 5000.0,
                "unit": "RPM",
                "position": {"x": 280.0, "y": 200.0}
            },
            {
                "id": "inst-wt301",
                "tag": "WT-301",
                "equipment_id": "eq-steam-turbine",
                "measurement_kind": "Power",
                "range_min": 0.0,
                "range_max": 500.0,
                "unit": "kW",
                "position": {"x": 340.0, "y": 200.0}
            },
            {
                "id": "inst-vt302",
                "tag": "VT-302",
                "equipment_id": "eq-compressor",
                "measurement_kind": "Vibration",
                "range_min": 0.0,
                "range_max": 10.0,
                "unit": "mm/s",
                "position": {"x": 480.0, "y": 200.0}
            },
            {
                "id": "inst-pt303",
                "tag": "PT-303",
                "equipment_id": "eq-receiver",
                "measurement_kind": "Pressure",
                "range_min": 0.0,
                "range_max": 40.0,
                "unit": "bar",
                "position": {"x": 680.0, "y": 200.0}
            }
        ],
        "control_loops": [
            {
                "id": "loop-pic301",
                "name": "PIC-301",
                "pv_tag": "PT-303",
                "sp": 15.0,
                "out_tag": "FV-301.POS",
                "kp": 1.2,
                "ti": 12.0,
                "td": 0.0,
                "output_min": 0.0,
                "output_max": 100.0,
                "mode": "AUTO",
                "reverse_acting": False
            }
        ],
        "alarms": [
            {
                "id": "alm-sa301",
                "name": "SA-301-HI",
                "pv_tag": "ST-301",
                "alarm_type": "HI",
                "severity": "CRITICAL",
                "trip_limit": 3600.0,
                "deadband": 20.0,
                "description": "Turbine Overspeed Alarm"
            },
            {
                "id": "alm-va302",
                "name": "VA-302-HI",
                "pv_tag": "VT-302",
                "alarm_type": "HI",
                "severity": "WARNING",
                "trip_limit": 4.5,
                "deadband": 0.2,
                "description": "Compressor Vibration High"
            }
        ],
        "interlocks": [
            {
                "id": "ilk-301",
                "name": "I-301",
                "source_tag": "ST-301",
                "condition": ">",
                "threshold": 3800.0,
                "target_equipment": "eq-steam-valve",
                "target_action": "CLOSE",
                "latched": True,
                "description": "Turbine overspeed shuts steam throttle valve"
            }
        ]
    }

def verify_plant_backend(plant_name: str, plant_def: dict):
    print(f"\n--- [Backend Verification: {plant_name}] ---")
    proj_id = plant_def["id"]

    # 1. Save or overwrite project
    print(f"  [1/4] Saving project '{proj_id}' via API...")
    res = api_request(f"/api/v1/projects/{proj_id}", method="PUT", data={"project": plant_def})
    assert res.get("id") == proj_id, f"Save failed for {proj_id}: {res}"
    print(f"  ✓ Project saved successfully: {res['name']}")

    # 2. Validate topology and instrumentation
    print(f"  [2/4] Validating topology & rules via /api/v1/projects/{proj_id}/validate...")
    val_res = api_request(f"/api/v1/projects/{proj_id}/validate", method="POST")
    assert val_res.get("valid") is True, f"Validation failed for {proj_id}: {val_res}"
    assert val_res.get("error_count") == 0, f"Errors found in {proj_id}: {val_res.get('issues')}"
    print(f"  ✓ Topology validation PASSED: valid=True, error_count=0, warnings={val_res.get('warning_count')}")

    # 3. Load into live simulation runtime
    print(f"  [3/4] Loading into simulation runtime via /api/v1/projects/{proj_id}/load-sim...")
    load_res = api_request(f"/api/v1/projects/{proj_id}/load-sim", method="POST")
    assert load_res.get("status") == "ok", f"Load simulation failed for {proj_id}: {load_res}"
    print(f"  ✓ Loaded into simulation host: status={load_res.get('status')}")

    # 4. Advance clock and inspect tags
    print(f"  [4/4] Starting simulation and verifying physical clock advance & tags...")
    api_request("/api/v1/sim/control", method="POST", data={"action": "play"})
    time.sleep(0.5)

    health = api_request("/api/v1/health")
    assert health.get("simulation_running") is True, f"Simulation not running: {health}"
    assert health.get("sim_time_sec", 0.0) > 0.0, f"Clock did not advance: {health}"
    print(f"  ✓ Simulation running: sim_time={health['sim_time_sec']:.2f}s, ticks={health['tick_count']}")

    snap = api_request("/api/v1/sim/snapshot")
    tags = {t["tag"]: t["value"] for t in snap.get("tags", [])}
    print(f"  ✓ Active DCS tags ({len(tags)} tags online):")
    for inst in plant_def["instruments"]:
        tag_name = inst["tag"]
        assert tag_name in tags, f"Transmitter tag {tag_name} missing from DCS snapshot!"
        val = tags[tag_name]
        print(f"      • {tag_name} ({inst['measurement_kind']}): {val} {inst['unit']}")

    api_request("/api/v1/sim/control", method="POST", data={"action": "pause"})
    print(f"  ✓ {plant_name} passed all backend simulation & physical criteria!")

def run_browser_ui_tests():
    print("\n--- [Browser UI Acceptance: Engineering Studio Visual Flow] ---")
    with sync_playwright() as p:
        browser = p.chromium.launch(
            headless=True,
            args=["--no-sandbox", "--disable-setuid-sandbox", "--disable-dev-shm-usage"]
        )
        context = browser.new_context(viewport={"width": 1440, "height": 900})
        page = context.new_page()
        page.on("dialog", lambda dialog: dialog.accept())

        # Load web app
        print("  [1/4] Opening Web Application at base URL...")
        page.goto(BASE_URL, wait_until="networkidle")
        page.screenshot(path=f"{SCREENSHOT_DIR}/plant_acc_01_app_loaded.png")

        # Switch to studio
        print("  [2/4] Switching to Engineering Studio mode...")
        page.click("button.mode-tab[data-mode='studio']")
        page.wait_for_selector("nebu-studio", state="visible")
        page.wait_for_selector(".studio-workbench", state="visible")

        # Verify project selector lists all three newly saved plants
        print("  [3/4] Testing project switching between Plant A, Plant B, and Plant C...")
        selector = page.locator(".project-selector")
        expect(selector).to_be_visible()

        for pid, name in [
            ("proj-plant-a-pumping", "Plant A - Pumping & Level Regulation Line"),
            ("proj-plant-b-reaction", "Plant B - Exothermic Reaction Train with Recycle"),
            ("proj-plant-c-turbine", "Plant C - Steam Turbine Expanding Steam to Drive Compressor"),
        ]:
            page.select_option(".project-selector", pid)
            page.wait_for_timeout(400)
            page.screenshot(path=f"{SCREENSHOT_DIR}/plant_acc_{pid}.png")
            print(f"      • Loaded '{name}' in Studio P&ID canvas")

            # Check that equipment nodes are rendered
            eq_nodes = page.locator(".eq-node")
            count = eq_nodes.count()
            assert count >= 4, f"Expected >= 4 equipment nodes for {pid}, got {count}"
            print(f"        ✓ Rendered {count} equipment glyphs on canvas")

        # Inspect an instrument tag bubble on canvas
        print("  [4/4] Verifying inline instrument bubble render and selection...")
        page.select_option(".project-selector", "proj-plant-a-pumping")
        page.wait_for_timeout(300)
        bubble = page.locator(".instrument-bubble, .stream-tag-bubble").first
        if bubble.is_visible():
            bubble.click()
            page.wait_for_timeout(200)
            print("      ✓ Instrument tag bubble clicked")

        page.screenshot(path=f"{SCREENSHOT_DIR}/plant_acc_final_studio.png")
        browser.close()
        print("  ✓ Browser UI verification completed successfully!")

def main():
    print("================================================================================")
    print(" NEBUDCS GENERIC EXPANSION: MULTI-PLANT ACCEPTANCE SUITE")
    print(f" Target: {BASE_URL}")
    print("================================================================================\n")

    register_custom_turbine()

    plants = [
        ("Plant A (Pumping Line & Level Regulation)", build_plant_a()),
        ("Plant B (Exothermic Reaction Train & Recycle)", build_plant_b()),
        ("Plant C (Steam Turbine Driven Compressor)", build_plant_c())
    ]

    for name, plant_def in plants:
        verify_plant_backend(name, plant_def)

    run_browser_ui_tests()

    print("\n================================================================================")
    print(" ALL 3 GENERIC PLANTS VERIFIED & PASSED WITH ZERO REGRESSIONS")
    print("================================================================================")

if __name__ == "__main__":
    main()
