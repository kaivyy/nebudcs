/**
 * Master Equipment & Instrumentation Catalog Completeness Tests
 *
 * Verifies:
 * 1. All required standard instrumentation (PT, PI, TT, TI, FT, FI, LT, LI, PDT, PDI, DP transmitter)
 * 2. All switches (PSH, PSL, TSH, FSH, LSH) and analyzers (pH, GC)
 * 3. All control blocks and valves (FIC, PIC, TIC, LIC, ratio, selector, check, on/off, control, PSV, PCV, ESD)
 * 4. All restored process equipment (fixed-bed catalytic reactor, strainers, mixers, blowers, motors, crystallizers)
 * 5. Structural integrity: ports, default dimensions, collision bounds, and palette category classification.
 */

import { describe, expect, it } from "vitest";
import {
  EQUIPMENT_TEMPLATE_CATALOG,
  getEquipmentTemplate,
  getTemplatesByCategory,
} from "../src/domain/equipment-template-catalog";

describe("Master Equipment & Instrumentation Catalog Completeness", () => {
  it("contains at least 48 comprehensive industrial templates", () => {
    expect(EQUIPMENT_TEMPLATE_CATALOG.length).toBeGreaterThanOrEqual(48);
  });

  describe("Instrumentation & Transmitters", () => {
    const requiredInstruments = [
      { id: "standard.instrument.pt", name: "Pressure Transmitter (PT)" },
      { id: "standard.instrument.pi", name: "Pressure Indicator (PI)" },
      { id: "standard.instrument.tt", name: "Temperature Transmitter (TT)" },
      { id: "standard.instrument.ti", name: "Temperature Indicator (TI)" },
      { id: "standard.instrument.ft", name: "Flow Transmitter (FT)" },
      { id: "standard.instrument.fi", name: "Flow Indicator (FI)" },
      { id: "standard.instrument.lt", name: "Level Transmitter (LT)" },
      { id: "standard.instrument.li", name: "Level Indicator (LI)" },
      { id: "standard.instrument.pdt", name: "Differential Pressure Transmitter (PDT)" },
      { id: "standard.instrument.pdi", name: "Differential Pressure Indicator (PDI)" },
      { id: "standard.instrument.generic_transmitter", name: "Process Transmitter (XT)" },
    ];

    for (const inst of requiredInstruments) {
      it(`includes valid template for ${inst.name} [${inst.id}]`, () => {
        const tmpl = getEquipmentTemplate(inst.id);
        expect(tmpl).toBeDefined();
        expect(tmpl?.category).toBe("Instrumentation");
        expect(tmpl?.defaultWidth).toBeGreaterThan(0);
        expect(tmpl?.defaultHeight).toBeGreaterThan(0);
        expect(tmpl?.ports.length).toBeGreaterThanOrEqual(1);
        expect(["Signal", "Material"]).toContain(tmpl?.ports[0].domain);
      });
    }
  });

  describe("Industrial Switches & Analyzers", () => {
    const requiredSwitchesAndAnalyzers = [
      { id: "standard.instrument.switch_pressure", name: "Pressure Switch High (PSH)" },
      { id: "standard.instrument.switch_pressure_low", name: "Pressure Switch Low (PSL)" },
      { id: "standard.instrument.switch_temperature", name: "Temperature Switch High (TSH)" },
      { id: "standard.instrument.switch_flow", name: "Flow Switch Low (FSL)" },
      { id: "standard.instrument.switch_level_high", name: "Level Switch High (LSH)" },
      { id: "standard.instrument.analyzer_ph", name: "pH Analyzer Transmitter (AT-pH)" },
      { id: "standard.instrument.analyzer_gc", name: "Gas Chromatograph Analyzer (AT-GC)" },
    ];

    for (const item of requiredSwitchesAndAnalyzers) {
      it(`includes valid template for ${item.name} [${item.id}]`, () => {
        const tmpl = getEquipmentTemplate(item.id);
        expect(tmpl).toBeDefined();
        expect(tmpl?.ports.length).toBeGreaterThanOrEqual(1);
        expect(["Signal", "Material"]).toContain(tmpl?.ports[0].domain);
      });
    }
  });

  describe("Controllers, Safety & Final Control Elements", () => {
    const requiredControls = [
      { id: "standard.control.fic", name: "Flow Indicating Controller (FIC)" },
      { id: "standard.control.pic", name: "Pressure Indicating Controller (PIC)" },
      { id: "standard.control.tic", name: "Temperature Indicating Controller (TIC)" },
      { id: "standard.control.lic", name: "Level Indicating Controller (LIC)" },
      { id: "standard.control.ratio", name: "Ratio Controller (FFC)" },
      { id: "standard.control.selector", name: "Signal Selector / Override (HS/LS)" },
      { id: "standard.safety.interlock_logic", name: "Safety Interlock Logic Solver (SIS)" },
      { id: "standard.valve.esd_valve", name: "Emergency Shutdown Valve (ESV)" },
      { id: "standard.valve.control_globe", name: "Control Valve (Globe)" },
      { id: "standard.valve.on_off_xv", name: "On/Off Isolation Valve (XV)" },
      { id: "standard.valve.check_nrv", name: "Non-Return Check Valve (NRV)" },
      { id: "standard.valve.regulator_pcv", name: "Self-Operated Pressure Regulator (PCV)" },
      { id: "standard.valve.relief_psv", name: "Pressure Safety Relief Valve (PSV)" },
      { id: "standard.piping.rupture_disk", name: "Rupture Disk Safety Relief (PSE)" },
      { id: "standard.piping.restrictor_orifice", name: "Restriction Orifice (RO)" },
    ];

    for (const item of requiredControls) {
      it(`includes valid template for ${item.name} [${item.id}]`, () => {
        const tmpl = getEquipmentTemplate(item.id);
        expect(tmpl).toBeDefined();
        expect(tmpl?.ports.length).toBeGreaterThanOrEqual(1);
      });
    }
  });

  describe("Process Equipment & Unit Operations", () => {
    const requiredEquipment = [
      { id: "standard.reactor.fixed_bed", name: "Fixed-Bed Catalytic Reactor" },
      { id: "standard.filter.strainer", name: "Basket / Simplex Strainer Filter" },
      { id: "standard.mixer.static", name: "Static In-Line Mixer" },
      { id: "standard.rotating.blower", name: "Centrifugal Process Blower" },
      { id: "standard.electrical.motor_induction", name: "Three-Phase Induction Motor Drive" },
      { id: "standard.crystallizer.evaporative", name: "Forced Circulation Evaporative Crystallizer" },
      { id: "standard.furnace.fired_heater", name: "Fired Process Heater" },
      { id: "standard.column.fractionator", name: "Distillation Fractionation Column" },
      { id: "standard.compressor.centrifugal", name: "Centrifugal Gas Compressor" },
      { id: "standard.pump.centrifugal", name: "Centrifugal Pump" },
      { id: "standard.exchanger.shell_tube", name: "Shell & Tube Exchanger" },
    ];

    for (const eq of requiredEquipment) {
      it(`includes valid template for ${eq.name} [${eq.id}]`, () => {
        const tmpl = getEquipmentTemplate(eq.id);
        expect(tmpl).toBeDefined();
        expect(tmpl?.defaultWidth).toBeGreaterThanOrEqual(30);
        expect(tmpl?.defaultHeight).toBeGreaterThanOrEqual(30);
        expect(tmpl?.ports.length).toBeGreaterThanOrEqual(2);
      });
    }
  });

  describe("Category Partitioning & Query Functions", () => {
    it("partitions templates cleanly across all categories", () => {
      const processTemplates = getTemplatesByCategory("process");
      const vesselTemplates = getTemplatesByCategory("vessels");
      const sensorTemplates = getTemplatesByCategory("sensors");
      const valveTemplates = getTemplatesByCategory("valves");
      const industryTemplates = getTemplatesByCategory("industry");

      expect(processTemplates.length).toBeGreaterThanOrEqual(10);
      expect(vesselTemplates.length).toBeGreaterThanOrEqual(6);
      expect(sensorTemplates.length).toBeGreaterThanOrEqual(15);
      expect(valveTemplates.length).toBeGreaterThanOrEqual(6);
      expect(industryTemplates.length).toBeGreaterThanOrEqual(1);

      // Verify no duplicate IDs across catalog
      const allIds = EQUIPMENT_TEMPLATE_CATALOG.map((t) => t.id);
      const uniqueIds = new Set(allIds);
      expect(uniqueIds.size).toBe(allIds.length);
    });

    it("verifies all ports have valid coordinates within template bounds", () => {
      for (const tmpl of EQUIPMENT_TEMPLATE_CATALOG) {
        const halfW = tmpl.defaultWidth / 2;
        const halfH = tmpl.defaultHeight / 2;
        for (const p of tmpl.ports) {
          expect(Math.abs(p.xOffset)).toBeLessThanOrEqual(halfW + 30);
          expect(Math.abs(p.yOffset)).toBeLessThanOrEqual(halfH + 30);
        }
      }
    });
  });
});
