//! Industrial Domain Packages for NebuDCS.
//!
//! Provides reusable, domain-scoped industrial configurations across:
//! - Refinery Domain (Crude distillation, preheat trains, fractionation, reboilers, reflux)
//! - Power / Steam Domain (Boilers, steam expansion turbines, mechanical shafts, generators/compressors)
//! - Gas Processing Domain (Separation, multi-stage compression, intercoolers, anti-surge recycle)
//! - Chemical Processing Domain (Exothermic CSTR reaction trains, preheaters, jacket cooling, recycle)
//!
//! Each domain package is a first-class citizen defining thermodynamic criteria,
//! required utilities, nominal operating envelopes, control strategies, and scenario presets
//! without duplicating simulation code or creating hardcoded monolithic simulators.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use nebudcs_types::SchemaVersion;

/// The primary industrial sector / domain classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndustrialDomainKind {
    /// Petroleum refining, atmospheric fractionation, and hydrocarbon handling.
    Refinery,
    /// Thermal power generation, steam expansion, and mechanical drive machinery.
    PowerSteam,
    /// Natural gas compression, conditioning, separation, and anti-surge protection.
    GasProcessing,
    /// Chemical synthesis, continuous stirred-tank reactors, and exothermic kinetics.
    ChemicalProcessing,
    /// Petrochemical steam cracking, olefin separation, and polymer precursor synthesis.
    Petrochemical,
}

impl std::fmt::Display for IndustrialDomainKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Refinery => write!(f, "Refinery"),
            Self::PowerSteam => write!(f, "Power & Steam"),
            Self::GasProcessing => write!(f, "Gas Processing"),
            Self::ChemicalProcessing => write!(f, "Chemical Processing"),
            Self::Petrochemical => write!(f, "Petrochemical"),
        }
    }
}

/// An industrial operating range specification for key process variables.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatingRangeDefinition {
    pub quantity: String,
    pub min_value: f64,
    pub max_value: f64,
    pub unit: String,
    pub normal_operating_value: f64,
}

impl OperatingRangeDefinition {
    pub fn new(
        quantity: impl Into<String>,
        min_value: f64,
        max_value: f64,
        unit: impl Into<String>,
        normal: f64,
    ) -> Self {
        Self {
            quantity: quantity.into(),
            min_value,
            max_value,
            unit: unit.into(),
            normal_operating_value: normal,
        }
    }
}

/// A comprehensive, reusable industrial domain package.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainPackage {
    pub id: String,
    pub name: String,
    pub version: SchemaVersion,
    pub domain_kind: IndustrialDomainKind,
    pub description: String,
    pub author: String,
    /// Minimum thermodynamic capabilities (e.g. "Peng-Robinson", "Ideal Gas", "Steam Tables / IAPWS").
    pub thermodynamic_requirements: Vec<String>,
    /// Required utility headers (e.g. "STEAM_PRESSURE", "COOLING_WATER", "FUEL_GAS", "INSTRUMENT_AIR").
    pub utility_requirements: Vec<String>,
    /// Default fluid or gas component fractions.
    pub composition_defaults: BTreeMap<String, f64>,
    /// Safe and nominal operating envelopes for key domain variables.
    pub operating_ranges: BTreeMap<String, OperatingRangeDefinition>,
    /// Standard regulatory and advanced control strategies.
    pub control_strategies: Vec<String>,
    /// Pre-configured training & abnormal operating condition scenario identifiers.
    pub scenario_presets: Vec<String>,
    /// Equipment template identifiers relevant to this domain.
    pub equipment_templates: Vec<String>,
    /// Demonstration plant project identifiers.
    pub plant_template_ids: Vec<String>,
    /// Engineering validation heuristics.
    pub validation_rules: Vec<String>,
    /// Authoritative scientific literature and recognized engineering standards citations.
    #[serde(default)]
    pub scientific_references: Vec<String>,
    /// Engineering model classifications (Standards-based, Theoretical, Literature-based, etc.).
    #[serde(default)]
    pub model_classifications: Vec<String>,
}

/// Generates the standard Refinery Domain Package.
pub fn make_refinery_cdu_domain_package() -> DomainPackage {
    let mut comp = BTreeMap::new();
    comp.insert("Naphtha".to_string(), 0.20);
    comp.insert("Kerosene".to_string(), 0.25);
    comp.insert("Diesel".to_string(), 0.30);
    comp.insert("Atmospheric_Residue".to_string(), 0.25);

    let mut ranges = BTreeMap::new();
    ranges.insert(
        "column_top_temp".to_string(),
        OperatingRangeDefinition::new("Temperature", 60.0, 130.0, "°C", 85.0),
    );
    ranges.insert(
        "column_bottom_temp".to_string(),
        OperatingRangeDefinition::new("Temperature", 280.0, 385.0, "°C", 345.0),
    );
    ranges.insert(
        "furnace_coil_outlet_temp".to_string(),
        OperatingRangeDefinition::new("Temperature", 300.0, 385.0, "°C", 360.0),
    );
    ranges.insert(
        "reflux_ratio".to_string(),
        OperatingRangeDefinition::new("Ratio", 1.0, 6.0, "fraction", 2.5),
    );
    ranges.insert(
        "column_overhead_pressure".to_string(),
        OperatingRangeDefinition::new("Pressure", 1.0, 3.5, "bar", 1.5),
    );

    DomainPackage {
        id: "domain.refinery.cdu".to_string(),
        name: "Refinery Atmospheric Crude Distillation Package".to_string(),
        version: SchemaVersion::new(1, 0, 0),
        domain_kind: IndustrialDomainKind::Refinery,
        description: "Standard petroleum refinery crude distillation unit (CDU) flowsheet featuring crude preheat train, fired process heater, 30-tray fractional distillation column, overhead condenser, reflux accumulator, and bottom reboiler circulation.".to_string(),
        author: "NebuDCS Standard Library".to_string(),
        thermodynamic_requirements: vec![
            "Hydrocarbon Characterization (TBP/ASTM D86)".to_string(),
            "Vapor-Liquid Equilibrium (Peng-Robinson / Chao-Seader)".to_string(),
            "LMTD / NTU Heat Transfer".to_string(),
        ],
        utility_requirements: vec![
            "STEAM_PRESSURE".to_string(),
            "COOLING_WATER".to_string(),
            "FUEL_GAS".to_string(),
            "INSTRUMENT_AIR".to_string(),
        ],
        composition_defaults: comp,
        operating_ranges: ranges,
        control_strategies: vec![
            "PIC-101 Column Head Pressure Control via Overhead Vapor/Condenser".to_string(),
            "TIC-101 Column Top Temperature Control via Reflux Flow".to_string(),
            "LIC-101 Column Bottoms Sump Level Control via Residue Draw".to_string(),
            "TIC-102 Furnace Coil Outlet Temperature Control via Fuel Gas Throttle".to_string(),
            "LIC-102 Reflux Drum Level Control via Overhead Distillate Take-off".to_string(),
        ],
        scenario_presets: vec![
            "scn-refinery-feed-loss".to_string(),
            "scn-refinery-furnace-flameout".to_string(),
            "scn-refinery-reflux-pump-trip".to_string(),
            "scn-refinery-column-overpressure".to_string(),
        ],
        equipment_templates: vec![
            "standard.tank.atmospheric".to_string(),
            "standard.pump.centrifugal".to_string(),
            "standard.valve.control_globe".to_string(),
            "standard.heat_exchanger.shell_and_tube".to_string(),
            "standard.heater.fired_process".to_string(),
            "standard.vessel.vertical_separator".to_string(),
        ],
        plant_template_ids: vec!["proj-domain-refinery-cdu".to_string()],
        validation_rules: vec![
            "VLD-REF01: Distillation column requires connected overhead condenser and reflux return".to_string(),
            "VLD-REF02: Fired heater fuel gas supply must be interlocked with process coil minimum flow".to_string(),
            "VLD-REF03: Atmospheric crude charge pump must maintain positive NPSH margin > 1.5m".to_string(),
        ],
        scientific_references: vec![
            "Gary, J.H. & Handwerk, G.E., Petroleum Refining: Technology and Economics, 4th Ed., Marcel Dekker".to_string(),
            "Watkins, R.N., Petroleum Refinery Distillation, 2nd Ed., Gulf Publishing".to_string(),
            "API Standard 560: Fired Heaters for General Refinery Service".to_string(),
        ],
        model_classifications: vec![
            "IndustryCommonEngineeringModel (McCabe-Thiele/Fenske-Underwood multicomponent fractionation)".to_string(),
            "TheoreticalModel (Rigorous mass, species, and enthalpy conservation)".to_string(),
            "StandardsBased (API 560 thermal efficiency and coil flux limits)".to_string(),
        ],
    }
}

/// Generates the standard Power & Steam Domain Package.
pub fn make_power_steam_domain_package() -> DomainPackage {
    let mut comp = BTreeMap::new();
    comp.insert("Steam_H2O".to_string(), 1.0);

    let mut ranges = BTreeMap::new();
    ranges.insert(
        "turbine_shaft_speed".to_string(),
        OperatingRangeDefinition::new("Speed", 0.0, 4200.0, "RPM", 3600.0),
    );
    ranges.insert(
        "steam_header_pressure".to_string(),
        OperatingRangeDefinition::new("Pressure", 20.0, 65.0, "bar", 42.0),
    );
    ranges.insert(
        "turbine_shaft_power".to_string(),
        OperatingRangeDefinition::new("Power", 0.0, 600.0, "kW", 480.0),
    );
    ranges.insert(
        "compressor_discharge_pressure".to_string(),
        OperatingRangeDefinition::new("Pressure", 1.0, 35.0, "bar", 18.5),
    );

    DomainPackage {
        id: "domain.power.steam_turbine".to_string(),
        name: "Thermal Power & Steam Mechanical Drive Package".to_string(),
        version: SchemaVersion::new(1, 0, 0),
        domain_kind: IndustrialDomainKind::PowerSteam,
        description: "Industrial steam power generation and mechanical shaft drive package with HP steam boiler, electro-hydraulic throttle valve, multi-stage expansion turbine, mechanical shaft torque transmission, driven centrifugal compressor, and surface condenser.".to_string(),
        author: "NebuDCS Standard Library".to_string(),
        thermodynamic_requirements: vec![
            "Steam Enthalpy-Entropy State Functions (IAPWS-IF97)".to_string(),
            "Isentropic Turbine Expansion Efficiency".to_string(),
            "Rankine Cycle Thermodynamics".to_string(),
        ],
        utility_requirements: vec![
            "STEAM_PRESSURE".to_string(),
            "COOLING_WATER".to_string(),
            "INSTRUMENT_AIR".to_string(),
            "ELECTRICITY".to_string(),
        ],
        composition_defaults: comp,
        operating_ranges: ranges,
        control_strategies: vec![
            "SIC-201 Electronic Governor Speed Regulation via Throttle Valve".to_string(),
            "PIC-201 Downstream Compressor Discharge Pressure Regulation".to_string(),
            "Emergency Overspeed Quick-Trip Latch (> 3800 RPM shutoff)".to_string(),
            "PIC-202 Surface Condenser Vacuum Pressure Control".to_string(),
        ],
        scenario_presets: vec![
            "scn-power-steam-loss".to_string(),
            "scn-power-turbine-overspeed".to_string(),
            "scn-power-compressor-surge".to_string(),
            "scn-power-condenser-vacuum-loss".to_string(),
        ],
        equipment_templates: vec![
            "standard.valve.control_globe".to_string(),
            "standard.compressor.centrifugal".to_string(),
            "standard.heat_exchanger.shell_and_tube".to_string(),
            "standard.vessel.vertical_separator".to_string(),
        ],
        plant_template_ids: vec!["proj-domain-power-turbine".to_string()],
        validation_rules: vec![
            "VLD-PWR01: Steam turbine shaft_out must connect to rotating load with PortDomain::Mechanical".to_string(),
            "VLD-PWR02: Turbine throttle valve must fail closed (FailClosed) upon trip or loss of signal".to_string(),
        ],
        scientific_references: vec![
            "IAPWS-IF97: International Association for the Properties of Water and Steam Formulation 1997".to_string(),
            "Kehlhofer, R. et al., Combined-Cycle Gas & Steam Turbine Power Plants, 3rd Ed., PennWell".to_string(),
            "ASME PTC 6: Steam Turbines Performance Test Codes".to_string(),
        ],
        model_classifications: vec![
            "StandardsBased (IAPWS-IF97 steam property formulation)".to_string(),
            "TheoreticalModel (Isentropic expansion enthalpy drops and torque balance)".to_string(),
            "IndustryCommonEngineeringModel (Stodola ellipse flow-pressure relationship)".to_string(),
        ],
    }
}

/// Generates the standard Gas Processing Domain Package.
pub fn make_gas_processing_domain_package() -> DomainPackage {
    let mut comp = BTreeMap::new();
    comp.insert("Methane_CH4".to_string(), 0.88);
    comp.insert("Ethane_C2H6".to_string(), 0.07);
    comp.insert("Propane_C3H8".to_string(), 0.03);
    comp.insert("Nitrogen_N2".to_string(), 0.02);

    let mut ranges = BTreeMap::new();
    ranges.insert(
        "suction_pressure".to_string(),
        OperatingRangeDefinition::new("Pressure", 1.0, 10.0, "bar", 4.2),
    );
    ranges.insert(
        "discharge_pressure".to_string(),
        OperatingRangeDefinition::new("Pressure", 10.0, 50.0, "bar", 26.5),
    );
    ranges.insert(
        "discharge_temperature".to_string(),
        OperatingRangeDefinition::new("Temperature", 30.0, 160.0, "°C", 115.0),
    );
    ranges.insert(
        "compressor_vibration".to_string(),
        OperatingRangeDefinition::new("Vibration", 0.0, 12.0, "mm/s", 1.2),
    );

    DomainPackage {
        id: "domain.gas.compression_train".to_string(),
        name: "Gas Processing & Multi-Stage Compression Package".to_string(),
        version: SchemaVersion::new(1, 0, 0),
        domain_kind: IndustrialDomainKind::GasProcessing,
        description: "Natural gas compression and conditioning train featuring suction knockout drum, high-ratio centrifugal gas compressor, shell-and-tube aftercooler, high-pressure separator, and automated anti-surge recycle loop.".to_string(),
        author: "NebuDCS Standard Library".to_string(),
        thermodynamic_requirements: vec![
            "Real Gas Compressibility (Z-factor, Peng-Robinson)".to_string(),
            "Polytropic Head and Compression Work Curves".to_string(),
            "Two-Phase Liquid Knockout Separation".to_string(),
        ],
        utility_requirements: vec![
            "COOLING_WATER".to_string(),
            "INSTRUMENT_AIR".to_string(),
            "FUEL_GAS".to_string(),
        ],
        composition_defaults: comp,
        operating_ranges: ranges,
        control_strategies: vec![
            "PIC-301 Gas Header Discharge Pressure Control".to_string(),
            "FIC-301 Minimum-Flow Anti-Surge Recirculation Loop".to_string(),
            "TIC-301 Gas Aftercooler Outlet Temperature Control via Cooling Water".to_string(),
            "LIC-301 Suction Knockout Drum Condensate Drain Level Control".to_string(),
        ],
        scenario_presets: vec![
            "scn-gas-loss-of-cooling".to_string(),
            "scn-gas-antisurge-valve-failure".to_string(),
            "scn-gas-high-vibration-trip".to_string(),
            "scn-gas-suction-blockage".to_string(),
        ],
        equipment_templates: vec![
            "standard.vessel.vertical_separator".to_string(),
            "standard.compressor.centrifugal".to_string(),
            "standard.heat_exchanger.shell_and_tube".to_string(),
            "standard.valve.control_globe".to_string(),
        ],
        plant_template_ids: vec!["proj-domain-gas-compression".to_string()],
        validation_rules: vec![
            "VLD-GAS01: Gas compressor must have automated anti-surge recycle or bypass stream".to_string(),
            "VLD-GAS02: Gas compressor suction must be preceded by liquid knockout protection".to_string(),
        ],
        scientific_references: vec![
            "GPSA Engineering Data Book, 14th Ed., Gas Processors Suppliers Association, Sec 13 & 20".to_string(),
            "API Standard 617: Axial and Centrifugal Compressors for Petroleum, Chemical, and Gas Industry Services".to_string(),
            "Campbell, J.M., Gas Conditioning and Processing, Vol. 2, Campbell Petroleum Series".to_string(),
        ],
        model_classifications: vec![
            "StandardsBased (API 617 compressor operating envelope, surge margin, and choke criteria)".to_string(),
            "TheoreticalModel (Peng-Robinson cubic equation of state real gas departure functions)".to_string(),
            "IndustryCommonEngineeringModel (GPSA polytropic head and efficiency curves)".to_string(),
        ],
    }
}

/// Generates the standard Chemical Processing Domain Package.
pub fn make_chemical_processing_domain_package() -> DomainPackage {
    let mut comp = BTreeMap::new();
    comp.insert("Reactant_A".to_string(), 0.85);
    comp.insert("Solvent".to_string(), 0.15);

    let mut ranges = BTreeMap::new();
    ranges.insert(
        "reactor_core_temp".to_string(),
        OperatingRangeDefinition::new("Temperature", 80.0, 220.0, "°C", 165.0),
    );
    ranges.insert(
        "reactor_pressure".to_string(),
        OperatingRangeDefinition::new("Pressure", 1.0, 30.0, "bar", 12.0),
    );
    ranges.insert(
        "reactant_conversion".to_string(),
        OperatingRangeDefinition::new("Conversion", 0.0, 1.0, "fraction", 0.82),
    );
    ranges.insert(
        "reactor_liquid_level".to_string(),
        OperatingRangeDefinition::new("Level", 0.2, 0.95, "fraction", 0.65),
    );

    DomainPackage {
        id: "domain.chemical.exothermic_reaction".to_string(),
        name: "Chemical Process & Exothermic CSTR Reaction Package".to_string(),
        version: SchemaVersion::new(1, 0, 0),
        domain_kind: IndustrialDomainKind::ChemicalProcessing,
        description: "Continuous chemical synthesis plant with raw material charge pump, reactant feed preheater, jacket-cooled CSTR with Arrhenius exothermic kinetics, volatile fractionator/degasser separator, and unreacted feed recycle loop.".to_string(),
        author: "NebuDCS Standard Library".to_string(),
        thermodynamic_requirements: vec![
            "Arrhenius Reaction Kinetics (k = A * exp(-Ea / RT))".to_string(),
            "Exothermic Heat of Reaction Release Coupling".to_string(),
            "Flash Vapor-Liquid Equilibrium Phase Separation".to_string(),
        ],
        utility_requirements: vec![
            "COOLING_WATER".to_string(),
            "STEAM_PRESSURE".to_string(),
            "INSTRUMENT_AIR".to_string(),
            "NITROGEN".to_string(),
        ],
        composition_defaults: comp,
        operating_ranges: ranges,
        control_strategies: vec![
            "TIC-401 Exothermic Reactor Temperature Control via Cooling Water Jacket Valve".to_string(),
            "LIC-401 Reactor Level Regulation via Bottoms Effluent Valve".to_string(),
            "FIC-401 Fresh Reactant Feed Pacing Control".to_string(),
            "High-Temperature Runaway Emergency Interlock (Trip Feed, Flood Coolant)".to_string(),
        ],
        scenario_presets: vec![
            "scn-chem-cooling-jacket-failure".to_string(),
            "scn-chem-thermal-runaway".to_string(),
            "scn-chem-recycle-pump-trip".to_string(),
            "scn-chem-feed-composition-disturbance".to_string(),
        ],
        equipment_templates: vec![
            "standard.tank.atmospheric".to_string(),
            "standard.pump.centrifugal".to_string(),
            "standard.heat_exchanger.shell_and_tube".to_string(),
            "standard.reactor.cstr".to_string(),
            "standard.vessel.vertical_separator".to_string(),
            "standard.valve.control_globe".to_string(),
        ],
        plant_template_ids: vec!["proj-domain-chem-reaction".to_string()],
        validation_rules: vec![
            "VLD-CHM01: Exothermic reactor must have active heat removal mechanism (jacket or internal coils)".to_string(),
            "VLD-CHM02: Reactor recycle stream must connect to feed mixer or pump suction prior to preheater".to_string(),
        ],
        scientific_references: vec![
            "Fogler, H.S., Elements of Chemical Reaction Engineering, 4th Ed., Prentice Hall".to_string(),
            "Levenspiel, O., Chemical Reaction Engineering, 3rd Ed., John Wiley & Sons".to_string(),
            "Smith, J.M., Chemical Engineering Kinetics, 3rd Ed., McGraw-Hill".to_string(),
        ],
        model_classifications: vec![
            "TheoreticalModel (Arrhenius reaction kinetics, multi-species mass conservation, and thermal energy balances)".to_string(),
            "IndustryCommonEngineeringModel (Semenov Frank-Kamenetskii thermal runaway criteria)".to_string(),
            "TheoreticalModel (Rigorous CSTR and PFR differential formulations)".to_string(),
        ],
    }
}

/// Generates the standard Refinery Vacuum Distillation Unit (VDU) Package.
pub fn make_refinery_vdu_domain_package() -> DomainPackage {
    let mut comp = BTreeMap::new();
    comp.insert("Light_VGO".to_string(), 0.20);
    comp.insert("Heavy_VGO".to_string(), 0.45);
    comp.insert("Vacuum_Residue".to_string(), 0.35);

    let mut ranges = BTreeMap::new();
    ranges.insert(
        "column_flash_zone_temp".to_string(),
        OperatingRangeDefinition::new("Temperature", 380.0, 425.0, "°C", 405.0),
    );
    ranges.insert(
        "column_overhead_pressure".to_string(),
        OperatingRangeDefinition::new("Pressure", 10.0, 100.0, "mbar", 40.0),
    );
    ranges.insert(
        "ejector_motive_steam_pressure".to_string(),
        OperatingRangeDefinition::new("Pressure", 6.0, 18.0, "bar", 10.5),
    );
    ranges.insert(
        "lvgo_draw_temp".to_string(),
        OperatingRangeDefinition::new("Temperature", 180.0, 260.0, "°C", 220.0),
    );

    DomainPackage {
        id: "domain.refinery.vdu".to_string(),
        name: "Refinery Vacuum Distillation Unit (VDU) Package".to_string(),
        version: SchemaVersion::new(1, 0, 0),
        domain_kind: IndustrialDomainKind::Refinery,
        description: "Deep vacuum petroleum fractionation column (10-80 mbar) processing atmospheric residue feed into Light Vacuum Gas Oil (LVGO), Heavy Vacuum Gas Oil (HVGO), and Vacuum Pitch Residue with ejector vacuum condensing train.".to_string(),
        author: "NebuDCS Standard Library".to_string(),
        thermodynamic_requirements: vec![
            "Deep Vacuum Hydrocarbon Vapor-Liquid Equilibrium (Chao-Seader / Peng-Robinson)".to_string(),
            "Steam Surface Condenser & Ejector Gas Hydraulics".to_string(),
        ],
        utility_requirements: vec![
            "STEAM_PRESSURE".to_string(),
            "COOLING_WATER".to_string(),
            "INSTRUMENT_AIR".to_string(),
        ],
        composition_defaults: comp,
        operating_ranges: ranges,
        control_strategies: vec![
            "PIC-151 Column Overhead Deep Vacuum Pressure Control via Steam Ejector Suction".to_string(),
            "TIC-151 Vacuum Furnace Coil Outlet Temperature Control".to_string(),
            "LIC-151 Vacuum Column Sump Level Control via Pitch Residue Bottoms Pump".to_string(),
        ],
        scenario_presets: vec![
            "scn-vdu-vacuum-loss".to_string(),
            "scn-vdu-furnace-tube-coking".to_string(),
            "scn-vdu-bottoms-pump-cavitation".to_string(),
        ],
        equipment_templates: vec![
            "standard.tank.atmospheric".to_string(),
            "standard.pump.centrifugal".to_string(),
            "standard.heater.fired_process".to_string(),
            "standard.heat_exchanger.shell_and_tube".to_string(),
            "standard.valve.control_globe".to_string(),
        ],
        plant_template_ids: vec!["proj-domain-refinery-vdu".to_string()],
        validation_rules: vec![
            "VLD-VDU01: Vacuum column top pressure must remain strictly below 150 mbar during operation".to_string(),
            "VLD-VDU02: Bottoms residue pump suction requires minimum 2.5m static head to prevent cavitation under vacuum".to_string(),
        ],
        scientific_references: vec![
            "Gary, J.H. & Handwerk, G.E., Petroleum Refining: Technology and Economics, 4th Ed.".to_string(),
            "Nelson, W.L., Petroleum Refinery Engineering, 4th Ed., McGraw-Hill".to_string(),
            "API Technical Data Book - Petroleum Refining, Chapter 8".to_string(),
        ],
        model_classifications: vec![
            "TheoreticalModel (Deep vacuum multicomponent phase equilibrium)".to_string(),
            "IndustryCommonEngineeringModel (Ejector entrainment ratio and condenser duty)".to_string(),
        ],
    }
}

/// Generates the standard Refinery Hydroprocessing & Desulfurization (HDS/HDN) Package.
pub fn make_refinery_hydroprocessing_domain_package() -> DomainPackage {
    let mut comp = BTreeMap::new();
    comp.insert("Diesel_Hydrocarbons".to_string(), 0.82);
    comp.insert("Dissolved_H2".to_string(), 0.12);
    comp.insert("Hydrogen_Sulfide".to_string(), 0.06);

    let mut ranges = BTreeMap::new();
    ranges.insert(
        "reactor_inlet_pressure".to_string(),
        OperatingRangeDefinition::new("Pressure", 40.0, 120.0, "bar", 75.0),
    );
    ranges.insert(
        "catalytic_bed_temp".to_string(),
        OperatingRangeDefinition::new("Temperature", 320.0, 415.0, "°C", 365.0),
    );
    ranges.insert(
        "h2_treat_gas_ratio".to_string(),
        OperatingRangeDefinition::new("Ratio", 150.0, 600.0, "Nm3/m3", 350.0),
    );
    ranges.insert(
        "sulfur_conversion".to_string(),
        OperatingRangeDefinition::new("Conversion", 0.90, 0.999, "fraction", 0.985),
    );

    DomainPackage {
        id: "domain.refinery.hydroprocessing".to_string(),
        name: "Refinery Catalytic Hydrotreating & Desulfurization (HDS) Package".to_string(),
        version: SchemaVersion::new(1, 0, 0),
        domain_kind: IndustrialDomainKind::Refinery,
        description: "High-pressure catalytic trickle-bed hydrodesulfurization system converting organosulfur and organonitrogen molecules in middle distillates with recycled hydrogen, interstage quench cooling, high-pressure separator (HPS), and low-pressure separator (LPS).".to_string(),
        author: "NebuDCS Standard Library".to_string(),
        thermodynamic_requirements: vec![
            "High-Pressure Hydrocarbon/H2 Phase Equilibrium (Peng-Robinson Cubic EOS)".to_string(),
            "Exothermic Hydrodesulfurization Reaction Kinetics".to_string(),
        ],
        utility_requirements: vec![
            "HIGH_PRESSURE_HYDROGEN".to_string(),
            "COOLING_WATER".to_string(),
            "FUEL_GAS".to_string(),
            "INSTRUMENT_AIR".to_string(),
        ],
        composition_defaults: comp,
        operating_ranges: ranges,
        control_strategies: vec![
            "TIC-181 Catalytic Bed Peak Temperature Control via Interstage H2 Quench Valves".to_string(),
            "PIC-181 High-Pressure Separator (HPS) Loop Pressure Regulation".to_string(),
            "FIC-181 Fresh Hydrogen Makeup Pacing Controller".to_string(),
        ],
        scenario_presets: vec![
            "scn-hds-h2-recycle-loss".to_string(),
            "scn-hds-thermal-runaway-quench-trip".to_string(),
            "scn-hds-hps-level-blowby".to_string(),
        ],
        equipment_templates: vec![
            "standard.reactor.cstr".to_string(),
            "standard.compressor.centrifugal".to_string(),
            "standard.heat_exchanger.shell_and_tube".to_string(),
            "standard.vessel.vertical_separator".to_string(),
            "standard.valve.control_globe".to_string(),
        ],
        plant_template_ids: vec!["proj-domain-refinery-hydroprocessing".to_string()],
        validation_rules: vec![
            "VLD-HDS01: Hydroprocessing reactor must have automated hydrogen quench interlocks".to_string(),
            "VLD-HDS02: High-pressure separator gas outlet must route to amine scrubber or recycle compressor".to_string(),
        ],
        scientific_references: vec![
            "Meyers, R.A., Handbook of Petroleum Refining Processes, 3rd Ed., McGraw-Hill".to_string(),
            "Speight, J.G., The Chemistry and Technology of Petroleum, 5th Ed., CRC Press".to_string(),
            "Ancheyta, J. & Speight, J.G., Hydroprocessing of Heavy Oils and Residua, CRC Press".to_string(),
        ],
        model_classifications: vec![
            "TheoreticalModel (Langmuir-Hinshelwood catalytic desulfurization kinetics)".to_string(),
            "TheoreticalModel (High-pressure cubic EOS hydrogen solubility in hydrocarbons)".to_string(),
        ],
    }
}

/// Generates the standard Petrochemical Olefin Separation & Pyrolysis Train Package.
pub fn make_petrochemical_olefin_domain_package() -> DomainPackage {
    let mut comp = BTreeMap::new();
    comp.insert("Ethylene".to_string(), 0.48);
    comp.insert("Ethane".to_string(), 0.32);
    comp.insert("Methane_Hydrogen".to_string(), 0.15);
    comp.insert("Propylene_C3".to_string(), 0.05);

    let mut ranges = BTreeMap::new();
    ranges.insert(
        "furnace_cot".to_string(),
        OperatingRangeDefinition::new("Temperature", 780.0, 875.0, "°C", 840.0),
    );
    ranges.insert(
        "demethanizer_overhead_temp".to_string(),
        OperatingRangeDefinition::new("Temperature", -120.0, -70.0, "°C", -98.0),
    );
    ranges.insert(
        "c2_splitter_purity".to_string(),
        OperatingRangeDefinition::new("Purity", 0.985, 0.9998, "fraction", 0.9995),
    );
    ranges.insert(
        "cracked_gas_compressor_pr".to_string(),
        OperatingRangeDefinition::new("PressureRatio", 15.0, 38.0, "ratio", 32.0),
    );

    DomainPackage {
        id: "domain.petrochemical.olefin".to_string(),
        name: "Petrochemical Steam Cracking & Olefin Fractionation Package".to_string(),
        version: SchemaVersion::new(1, 0, 0),
        domain_kind: IndustrialDomainKind::Petrochemical,
        description: "Petrochemical pyrolysis furnace and cryogenic olefin separation train: thermal cracking of hydrocarbon feed at 840°C, multi-stage cracked gas compression, cryogenic chilling train, demethanizer (-100°C), deethanizer, and C2 splitter with unreacted ethane recycle loop.".to_string(),
        author: "NebuDCS Standard Library".to_string(),
        thermodynamic_requirements: vec![
            "Cryogenic Hydrocarbon Vapor-Liquid Equilibrium (Peng-Robinson EOS down to -120°C)".to_string(),
            "Thermal Cracking Pyrolysis Free-Radical Kinetics".to_string(),
            "Superfractionation Column Dynamics (High Reflux R > 3.5)".to_string(),
        ],
        utility_requirements: vec![
            "HIGH_PRESSURE_STEAM".to_string(),
            "PROPYLENE_REFRIGERANT".to_string(),
            "ETHYLENE_REFRIGERANT".to_string(),
            "COOLING_WATER".to_string(),
            "INSTRUMENT_AIR".to_string(),
        ],
        composition_defaults: comp,
        operating_ranges: ranges,
        control_strategies: vec![
            "TIC-501 Pyrolysis Furnace Coil Outlet Temperature Control (COT)".to_string(),
            "PIC-501 Demethanizer Cryogenic Overhead Pressure Control via Chilling Condenser".to_string(),
            "AIC-501 C2 Splitter Overhead Polymer-Grade Ethylene Purity Control".to_string(),
            "FIC-501 Unreacted Ethane Recycle Flow Pacing to Furnace".to_string(),
        ],
        scenario_presets: vec![
            "scn-olefin-cracking-furnace-flameout".to_string(),
            "scn-olefin-demethanizer-cryo-upset".to_string(),
            "scn-olefin-c2-splitter-reflux-pump-trip".to_string(),
            "scn-olefin-cracked-gas-compressor-surge".to_string(),
        ],
        equipment_templates: vec![
            "standard.heater.fired_process".to_string(),
            "standard.compressor.centrifugal".to_string(),
            "standard.heat_exchanger.shell_and_tube".to_string(),
            "standard.vessel.vertical_separator".to_string(),
            "standard.valve.control_globe".to_string(),
            "standard.pump.centrifugal".to_string(),
        ],
        plant_template_ids: vec!["proj-domain-petrochem-olefin".to_string()],
        validation_rules: vec![
            "VLD-PET01: Demethanizer overhead requires cryogenic refrigerant utility supply".to_string(),
            "VLD-PET02: Ethane recycle stream must close the loop back to cracking furnace charge mixer".to_string(),
        ],
        scientific_references: vec![
            "Chauvel, A. & Lefebvre, G., Petrochemical Processes: Technical and Economic Characteristics, Editions Technip".to_string(),
            "Zimmermann, H. & Walzl, R., Ethylene in Ullmann's Encyclopedia of Industrial Chemistry, Wiley-VCH".to_string(),
            "King, C.J., Separation Processes, 2nd Ed., McGraw-Hill".to_string(),
        ],
        model_classifications: vec![
            "TheoreticalModel (Cryogenic low-temperature cubic EOS phase equilibrium)".to_string(),
            "IndustryCommonEngineeringModel (Radical cracking conversion and product yields)".to_string(),
            "TheoreticalModel (Wegstein recycle convergence closure)".to_string(),
        ],
    }
}

/// Generates the standard LNG Liquefaction & Mixed Refrigerant Package.
pub fn make_lng_refrigeration_domain_package() -> DomainPackage {
    let mut comp = BTreeMap::new();
    comp.insert("Methane".to_string(), 0.94);
    comp.insert("Ethane".to_string(), 0.04);
    comp.insert("Propane".to_string(), 0.015);
    comp.insert("Nitrogen".to_string(), 0.005);

    let mut ranges = BTreeMap::new();
    ranges.insert(
        "lng_rundown_temp".to_string(),
        OperatingRangeDefinition::new("Temperature", -165.0, -155.0, "°C", -161.5),
    );
    ranges.insert(
        "mr_discharge_pressure".to_string(),
        OperatingRangeDefinition::new("Pressure", 35.0, 70.0, "bar", 52.0),
    );
    ranges.insert(
        "mr_suction_pressure".to_string(),
        OperatingRangeDefinition::new("Pressure", 2.0, 6.0, "bar", 3.8),
    );
    ranges.insert(
        "subcooling_delta_t".to_string(),
        OperatingRangeDefinition::new("Temperature", 2.0, 10.0, "K", 4.5),
    );

    DomainPackage {
        id: "domain.gas.lng_liquefaction".to_string(),
        name: "Natural Gas Liquefaction & Mixed Refrigerant (MR) Package".to_string(),
        version: SchemaVersion::new(1, 0, 0),
        domain_kind: IndustrialDomainKind::GasProcessing,
        description: "Cryogenic natural gas liquefaction process: sweet gas deep dehydration, propane pre-cooling, mixed refrigerant (MR) compression cycle, J-T valve expansion, and main cryogenic heat exchanger (MCHE) subcooling natural gas to -161°C atmospheric liquid.".to_string(),
        author: "NebuDCS Standard Library".to_string(),
        thermodynamic_requirements: vec![
            "Cryogenic Multicomponent Phase Equilibrium (Peng-Robinson EOS down to -165°C)".to_string(),
            "Joule-Thomson Isenthalpic Expansion and Refrigerant Thermodynamics".to_string(),
            "Multi-Stream Cryogenic Heat Exchanger Temperature Difference Tracking".to_string(),
        ],
        utility_requirements: vec![
            "COOLING_WATER".to_string(),
            "INSTRUMENT_AIR".to_string(),
            "FUEL_GAS".to_string(),
            "NITROGEN".to_string(),
        ],
        composition_defaults: comp,
        operating_ranges: ranges,
        control_strategies: vec![
            "TIC-601 LNG Rundown Temperature Control via Heavy/Light MR Expansion Ratio".to_string(),
            "PIC-601 MR Compressor Suction Pressure Control via Cold Box Recirculation".to_string(),
            "LIC-601 LNG Storage Tank Level and Boil-off Gas (BOG) Recompression".to_string(),
        ],
        scenario_presets: vec![
            "scn-lng-mr-compressor-trip".to_string(),
            "scn-lng-cold-box-temperature-crossover".to_string(),
            "scn-lng-jt-valve-hydrate-blockage".to_string(),
        ],
        equipment_templates: vec![
            "standard.compressor.centrifugal".to_string(),
            "standard.heat_exchanger.shell_and_tube".to_string(),
            "standard.valve.control_globe".to_string(),
            "standard.vessel.vertical_separator".to_string(),
            "standard.pump.centrifugal".to_string(),
        ],
        plant_template_ids: vec!["proj-domain-gas-lng".to_string()],
        validation_rules: vec![
            "VLD-LNG01: Cryogenic cold box must maintain positive temperature difference (no temperature crossover)".to_string(),
            "VLD-LNG02: LNG rundown temperature must reach -160°C or below prior to atmospheric storage".to_string(),
        ],
        scientific_references: vec![
            "Mokhatab, S., Mak, J.Y., Valappil, J.V., Wood, D.A., Handbook of Liquefied Natural Gas, Elsevier, 2014".to_string(),
            "Barron, R.F., Cryogenic Systems, 2nd Ed., Oxford University Press".to_string(),
            "NFPA 59A: Standard for the Production, Storage, and Handling of Liquefied Natural Gas".to_string(),
        ],
        model_classifications: vec![
            "TheoreticalModel (Cryogenic multicomponent VLE and isenthalpic flash expansion)".to_string(),
            "StandardsBased (NFPA 59A operating temperature and containment guidelines)".to_string(),
        ],
    }
}

/// Generates all eight authoritative standard industrial domain packages.
pub fn load_all_standard_domain_packages() -> Vec<DomainPackage> {
    vec![
        make_refinery_cdu_domain_package(),
        make_refinery_vdu_domain_package(),
        make_refinery_hydroprocessing_domain_package(),
        make_power_steam_domain_package(),
        make_gas_processing_domain_package(),
        make_lng_refrigeration_domain_package(),
        make_chemical_processing_domain_package(),
        make_petrochemical_olefin_domain_package(),
    ]
}

/// Generates standard equipment definitions associated with domain packages (e.g., steam turbine).
pub fn standard_domain_equipment_definitions() -> Vec<nebudcs_model::equipment::EquipmentDefinition>
{
    use crate::model::{CustomEquipmentDefinition, CustomizationTier};
    use indexmap::IndexMap;
    use nebudcs_model::equipment::{
        Cardinality, EquipmentCategory, ParameterDefinition, Phase, PortDefinition, PortDirection,
        PortDomain,
    };
    use nebudcs_types::{PortId, SchemaVersion};
    use nebudcs_units::display::QuantityKind;

    let make_turbine = |id: &str| {
        let mut params = IndexMap::new();
        params.insert(
            "rated_power".to_string(),
            ParameterDefinition {
                name: "rated_power".to_string(),
                description: "Rated shaft power (W)".to_string(),
                quantity_kind: QuantityKind::Power,
                default_value: 300_000.0,
                min_value: Some(0.0),
                max_value: Some(10_000_000.0),
                required: false,
            },
        );
        params.insert(
            "speed".to_string(),
            ParameterDefinition {
                name: "speed".to_string(),
                description: "Operating shaft speed (rpm)".to_string(),
                quantity_kind: QuantityKind::Speed,
                default_value: 3000.0,
                min_value: Some(0.0),
                max_value: Some(6000.0),
                required: false,
            },
        );
        params.insert(
            "rated_head".to_string(),
            ParameterDefinition {
                name: "rated_head".to_string(),
                description: "Rated expansion head".to_string(),
                quantity_kind: QuantityKind::Pressure,
                default_value: 120.0,
                min_value: Some(0.0),
                max_value: Some(1000.0),
                required: false,
            },
        );

        let ports = vec![
            PortDefinition {
                id: "steam_inlet".parse::<PortId>().unwrap(),
                name: "steam_inlet".to_string(),
                domain: PortDomain::Material,
                direction: PortDirection::Inlet,
                phase_compatibility: vec![Phase::Gas],
                required: true,
                cardinality: Cardinality::ZeroOrMore,
                description: "Steam inlet".to_string(),
            },
            PortDefinition {
                id: "exhaust".parse::<PortId>().unwrap(),
                name: "exhaust".to_string(),
                domain: PortDomain::Material,
                direction: PortDirection::Outlet,
                phase_compatibility: vec![Phase::Gas],
                required: true,
                cardinality: Cardinality::ZeroOrMore,
                description: "Exhaust outlet".to_string(),
            },
            PortDefinition {
                id: "shaft_out".parse::<PortId>().unwrap(),
                name: "shaft_out".to_string(),
                domain: PortDomain::Mechanical,
                direction: PortDirection::Outlet,
                phase_compatibility: Vec::new(),
                required: true,
                cardinality: Cardinality::ZeroOrMore,
                description: "Mechanical shaft output".to_string(),
            },
        ];

        CustomEquipmentDefinition {
            id: id.to_string(),
            name: "Steam Expansion Turbine".to_string(),
            version: SchemaVersion::new(1, 0, 0),
            category: EquipmentCategory::Rotating,
            tier: CustomizationTier::Level1Declarative,
            description: "Industrial steam expansion turbine driver".to_string(),
            parameters: params,
            ports,
            curves: IndexMap::new(),
            curves_2d: IndexMap::new(),
            operating_envelope: Default::default(),
            alarm_definitions: Vec::new(),
            trips: Vec::new(),
            interlocks: Vec::new(),
            script: None,
            composition: None,
            metadata: IndexMap::new(),
        }
        .to_domain_definition()
    };

    let make_fixed_bed_reactor = |id: &str| {
        let mut params = IndexMap::new();
        params.insert(
            "catalyst_volume_m3".to_string(),
            ParameterDefinition {
                name: "catalyst_volume_m3".to_string(),
                description: "Catalyst inventory volume (m3)".to_string(),
                quantity_kind: QuantityKind::Volume,
                default_value: 19.9,
                min_value: Some(0.1),
                max_value: Some(500.0),
                required: false,
            },
        );
        params.insert(
            "catalyst_mass_kg".to_string(),
            ParameterDefinition {
                name: "catalyst_mass_kg".to_string(),
                description: "Catalyst inventory mass (kg)".to_string(),
                quantity_kind: QuantityKind::MassFlow,
                default_value: 15503.0,
                min_value: Some(10.0),
                max_value: Some(500000.0),
                required: false,
            },
        );
        params.insert(
            "lhsv".to_string(),
            ParameterDefinition {
                name: "lhsv".to_string(),
                description: "Liquid hourly space velocity (1/h)".to_string(),
                quantity_kind: QuantityKind::VolumetricFlow,
                default_value: 10.0,
                min_value: Some(0.1),
                max_value: Some(50.0),
                required: false,
            },
        );
        params.insert(
            "operating_pressure".to_string(),
            ParameterDefinition {
                name: "operating_pressure".to_string(),
                description: "Operating pressure (Pa)".to_string(),
                quantity_kind: QuantityKind::Pressure,
                default_value: 2442000.0,
                min_value: Some(100000.0),
                max_value: Some(25000000.0),
                required: false,
            },
        );
        params.insert(
            "operating_temp".to_string(),
            ParameterDefinition {
                name: "operating_temp".to_string(),
                description: "Bed operating temperature (C)".to_string(),
                quantity_kind: QuantityKind::Temperature,
                default_value: 332.0,
                min_value: Some(0.0),
                max_value: Some(600.0),
                required: false,
            },
        );
        params.insert(
            "max_bed_delta_t".to_string(),
            ParameterDefinition {
                name: "max_bed_delta_t".to_string(),
                description: "Maximum allowable bed delta T (C)".to_string(),
                quantity_kind: QuantityKind::Temperature,
                default_value: 10.0,
                min_value: Some(0.0),
                max_value: Some(50.0),
                required: false,
            },
        );

        let ports = vec![
            PortDefinition {
                id: "feed".parse::<PortId>().unwrap(),
                name: "feed".to_string(),
                domain: PortDomain::Material,
                direction: PortDirection::Inlet,
                phase_compatibility: vec![Phase::TwoPhase, Phase::Liquid, Phase::Gas],
                required: true,
                cardinality: Cardinality::ZeroOrMore,
                description: "Feed mixture inlet".to_string(),
            },
            PortDefinition {
                id: "effluent".parse::<PortId>().unwrap(),
                name: "effluent".to_string(),
                domain: PortDomain::Material,
                direction: PortDirection::Outlet,
                phase_compatibility: vec![Phase::TwoPhase, Phase::Liquid, Phase::Gas],
                required: true,
                cardinality: Cardinality::ZeroOrMore,
                description: "Hydrotreated effluent outlet".to_string(),
            },
            PortDefinition {
                id: "quench_in".parse::<PortId>().unwrap(),
                name: "quench_in".to_string(),
                domain: PortDomain::Material,
                direction: PortDirection::Inlet,
                phase_compatibility: vec![Phase::Gas],
                required: false,
                cardinality: Cardinality::ZeroOrMore,
                description: "Quench gas inlet".to_string(),
            },
            PortDefinition {
                id: "psv_relief".parse::<PortId>().unwrap(),
                name: "psv_relief".to_string(),
                domain: PortDomain::Utility(nebudcs_model::equipment::UtilityKind::Flare),
                direction: PortDirection::Outlet,
                phase_compatibility: Vec::new(),
                required: false,
                cardinality: Cardinality::ZeroOrMore,
                description: "Reactor PSV relief".to_string(),
            },
        ];

        CustomEquipmentDefinition {
            id: id.to_string(),
            name: "Fixed-Bed Catalytic Reactor".to_string(),
            version: SchemaVersion::new(1, 0, 0),
            category: EquipmentCategory::Process,
            tier: CustomizationTier::Level1Declarative,
            description: "Fixed-bed catalytic downflow hydroprocessing reactor".to_string(),
            parameters: params,
            ports,
            curves: IndexMap::new(),
            curves_2d: IndexMap::new(),
            operating_envelope: Default::default(),
            alarm_definitions: Vec::new(),
            trips: Vec::new(),
            interlocks: Vec::new(),
            script: None,
            composition: None,
            metadata: IndexMap::new(),
        }
        .to_domain_definition()
    };

    vec![
        make_turbine("custom.turbine.steam"),
        make_turbine("standard.turbine.steam"),
        make_fixed_bed_reactor("standard.reactor.fixed_bed"),
    ]
}
