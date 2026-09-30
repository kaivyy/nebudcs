//! Standard Equipment Library Package Catalog.
//!
//! Provides authoritative, pre-verified standard industrial equipment templates
//! and composite machinery packages covering rotating machinery, heat transfer,
//! separation vessels, tanks, piping/control elements, and instrument sensors.

use indexmap::IndexMap;
use std::collections::BTreeMap;

use nebudcs_model::equipment::{
    Cardinality, EnvelopeDimension, EquipmentCategory, OperatingEnvelope, ParameterDefinition,
    Phase, PortDefinition, PortDirection, PortDomain, UtilityKind,
};
use nebudcs_types::{PortId, SchemaVersion, TagId};
use nebudcs_units::display::QuantityKind;

use crate::curve::{Curve1D, ExtrapolationMode, InterpolationMethod};
use crate::error::ScriptingResult;
use crate::library::package::{
    EquipmentPackage, PackageDependency, PackageInternalStream, PackageParameterMapping,
    PackagePortMapping, PackageSubComponent,
};
use crate::library::template::{EquipmentTemplate, TemplateMetadata, TemplateSource};
use crate::model::{
    CustomEquipmentDefinition, CustomizationTier, TripConditionDefinition, TripOperator,
};

fn std_version() -> SchemaVersion {
    SchemaVersion::new(1, 0, 0)
}

fn std_meta(desc: &str, category: EquipmentCategory, tags: Vec<&str>) -> TemplateMetadata {
    TemplateMetadata {
        author: "NebuDCS Standard Library".to_string(),
        description: desc.to_string(),
        tags: tags.into_iter().map(String::from).collect(),
        icon_hint: Some(format!("{:?}", category).to_lowercase()),
        documentation: None,
        created_at: "2026-09-06T00:00:00Z".to_string(),
        schema_version: std_version(),
        custom_attributes: IndexMap::new(),
    }
}

fn make_port(
    name: &str,
    direction: PortDirection,
    domain: PortDomain,
    phases: Vec<Phase>,
) -> PortDefinition {
    PortDefinition {
        id: name.parse::<PortId>().unwrap_or_else(|_| PortId::new()),
        name: name.to_string(),
        domain,
        direction,
        phase_compatibility: phases,
        required: true,
        cardinality: Cardinality::ZeroOrMore,
        description: format!("Port {name}"),
    }
}

fn make_param(
    name: &str,
    quantity_kind: QuantityKind,
    default_value: f64,
    min_value: Option<f64>,
    max_value: Option<f64>,
    desc: &str,
) -> ParameterDefinition {
    ParameterDefinition {
        name: name.to_string(),
        description: desc.to_string(),
        quantity_kind,
        default_value,
        min_value,
        max_value,
        required: false,
    }
}

/// 1. Centrifugal Pump (`standard.pump.centrifugal`)
pub fn make_standard_centrifugal_pump() -> ScriptingResult<EquipmentTemplate> {
    let mut params = IndexMap::new();
    params.insert(
        "rated_flow".to_string(),
        make_param(
            "rated_flow",
            QuantityKind::VolumetricFlow,
            120.0,
            Some(0.0),
            Some(500.0),
            "Rated flow (m3/h)",
        ),
    );
    params.insert(
        "rated_head".to_string(),
        make_param(
            "rated_head",
            QuantityKind::Pressure,
            850000.0,
            Some(0.0),
            Some(3000000.0),
            "Rated differential pressure head (Pa)",
        ),
    );
    params.insert(
        "design_speed".to_string(),
        make_param(
            "design_speed",
            QuantityKind::Speed,
            2950.0,
            Some(500.0),
            Some(3600.0),
            "Design rotor speed (rpm)",
        ),
    );

    let ports = vec![
        make_port(
            "suction",
            PortDirection::Inlet,
            PortDomain::Material,
            vec![Phase::Liquid],
        ),
        make_port(
            "discharge",
            PortDirection::Outlet,
            PortDomain::Material,
            vec![Phase::Liquid],
        ),
        make_port(
            "motor_feeder",
            PortDirection::Inlet,
            PortDomain::Electrical,
            Vec::new(),
        ),
    ];

    let mut curves = IndexMap::new();
    let mut curve = Curve1D::new(
        "head_flow",
        "flow",
        "delta_p",
        vec![
            (0.0, 1050000.0),
            (30.0, 1020000.0),
            (60.0, 970000.0),
            (90.0, 890000.0),
            (120.0, 780000.0),
            (150.0, 620000.0),
        ],
    );
    curve.interpolation = InterpolationMethod::MonotonicCubic;
    curve.extrapolation = ExtrapolationMode::Clamp;
    curves.insert("head_flow".to_string(), curve);

    let trips = vec![TripConditionDefinition {
        id: "trip_high_vibration".to_string(),
        name: "Vibration Trip".to_string(),
        condition_var: "vibration".to_string(),
        operator: TripOperator::GreaterThan,
        threshold: 7.1,
        fail_safe_state: "tripped".to_string(),
        latched: true,
    }];

    let envelope = OperatingEnvelope {
        dimensions: vec![EnvelopeDimension {
            tag_id: TagId::new(),
            description: "Operating flow range".to_string(),
            normal_min: 20.0,
            normal_max: 140.0,
            warning_min: Some(15.0),
            warning_max: Some(150.0),
            critical_min: Some(10.0),
            critical_max: Some(160.0),
            trip_min: None,
            trip_max: None,
        }],
    };

    let def = CustomEquipmentDefinition {
        id: "standard.pump.centrifugal".to_string(),
        name: "Standard Centrifugal Pump".to_string(),
        version: std_version(),
        category: EquipmentCategory::Rotating,
        tier: CustomizationTier::Level1Declarative,
        description: "Generic API 610 single-stage centrifugal process pump".to_string(),
        parameters: params,
        ports,
        curves,
        curves_2d: IndexMap::new(),
        operating_envelope: envelope,
        alarm_definitions: Vec::new(),
        trips,
        interlocks: Vec::new(),
        script: None,
        composition: None,
        metadata: IndexMap::new(),
    };

    EquipmentTemplate::new(
        "standard.pump.centrifugal",
        "Centrifugal Pump",
        std_version(),
        EquipmentCategory::Rotating,
        TemplateSource::Standard,
        std_meta(
            "Generic API 610 single-stage centrifugal pump",
            EquipmentCategory::Rotating,
            vec!["pump", "rotating"],
        ),
        def,
    )
}

/// 2. Centrifugal Compressor (`standard.compressor.centrifugal`)
pub fn make_standard_centrifugal_compressor() -> ScriptingResult<EquipmentTemplate> {
    let mut params = IndexMap::new();
    params.insert(
        "design_speed".to_string(),
        make_param(
            "design_speed",
            QuantityKind::Speed,
            10500.0,
            Some(3000.0),
            Some(15000.0),
            "Design speed (rpm)",
        ),
    );
    params.insert(
        "rated_mass_flow".to_string(),
        make_param(
            "rated_mass_flow",
            QuantityKind::MassFlow,
            25.0,
            Some(1.0),
            Some(150.0),
            "Rated gas flow (kg/s)",
        ),
    );

    let ports = vec![
        make_port(
            "suction",
            PortDirection::Inlet,
            PortDomain::Material,
            vec![Phase::Gas],
        ),
        make_port(
            "discharge",
            PortDirection::Outlet,
            PortDomain::Material,
            vec![Phase::Gas],
        ),
        make_port(
            "recycle",
            PortDirection::Inlet,
            PortDomain::Material,
            vec![Phase::Gas],
        ),
        make_port(
            "shaft_drive",
            PortDirection::Inlet,
            PortDomain::Mechanical,
            Vec::new(),
        ),
    ];

    let mut curves = IndexMap::new();
    let mut curve = Curve1D::new(
        "head_flow",
        "flow",
        "delta_p",
        vec![
            (10.0, 1250000.0),
            (20.0, 1180000.0),
            (30.0, 1080000.0),
            (40.0, 920000.0),
            (50.0, 700000.0),
        ],
    );
    curve.interpolation = InterpolationMethod::MonotonicCubic;
    curve.extrapolation = ExtrapolationMode::Clamp;
    curves.insert("head_flow".to_string(), curve);

    let trips = vec![TripConditionDefinition {
        id: "trip_overspeed".to_string(),
        name: "Rotor Overspeed Trip".to_string(),
        condition_var: "speed".to_string(),
        operator: TripOperator::GreaterThan,
        threshold: 12100.0,
        fail_safe_state: "tripped".to_string(),
        latched: true,
    }];

    let def = CustomEquipmentDefinition {
        id: "standard.compressor.centrifugal".to_string(),
        name: "Standard Centrifugal Compressor".to_string(),
        version: std_version(),
        category: EquipmentCategory::Rotating,
        tier: CustomizationTier::Level1Declarative,
        description: "High-speed barrel-type centrifugal gas compressor".to_string(),
        parameters: params,
        ports,
        curves,
        curves_2d: IndexMap::new(),
        operating_envelope: Default::default(),
        alarm_definitions: Vec::new(),
        trips,
        interlocks: Vec::new(),
        script: None,
        composition: None,
        metadata: IndexMap::new(),
    };

    EquipmentTemplate::new(
        "standard.compressor.centrifugal",
        "Centrifugal Compressor",
        std_version(),
        EquipmentCategory::Rotating,
        TemplateSource::Standard,
        std_meta(
            "Generic API 617 centrifugal gas compressor",
            EquipmentCategory::Rotating,
            vec!["compressor", "gas"],
        ),
        def,
    )
}

/// 3. Centrifugal Fan (`standard.fan.centrifugal`)
pub fn make_standard_fan() -> ScriptingResult<EquipmentTemplate> {
    let mut params = IndexMap::new();
    params.insert(
        "rated_flow".to_string(),
        make_param(
            "rated_flow",
            QuantityKind::VolumetricFlow,
            25000.0,
            Some(100.0),
            Some(100000.0),
            "Rated air flow capacity (m3/h)",
        ),
    );
    params.insert(
        "static_head_pa".to_string(),
        make_param(
            "static_head_pa",
            QuantityKind::Pressure,
            2500.0,
            Some(0.0),
            Some(10000.0),
            "Static head (Pa)",
        ),
    );

    let ports = vec![
        make_port(
            "air_inlet",
            PortDirection::Inlet,
            PortDomain::Material,
            vec![Phase::Gas],
        ),
        make_port(
            "air_outlet",
            PortDirection::Outlet,
            PortDomain::Material,
            vec![Phase::Gas],
        ),
        make_port(
            "power",
            PortDirection::Inlet,
            PortDomain::Electrical,
            Vec::new(),
        ),
    ];

    let def = CustomEquipmentDefinition {
        id: "standard.fan.centrifugal".to_string(),
        name: "Standard Centrifugal Fan".to_string(),
        version: std_version(),
        category: EquipmentCategory::Rotating,
        tier: CustomizationTier::Level1Declarative,
        description: "Industrial forced draft / induced draft centrifugal fan".to_string(),
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
    };

    EquipmentTemplate::new(
        "standard.fan.centrifugal",
        "Centrifugal Fan",
        std_version(),
        EquipmentCategory::Rotating,
        TemplateSource::Standard,
        std_meta(
            "Industrial draft fan for air movement",
            EquipmentCategory::Rotating,
            vec!["fan", "air"],
        ),
        def,
    )
}

/// 4. Globe Control Valve (`standard.valve.control_globe`)
pub fn make_standard_control_valve() -> ScriptingResult<EquipmentTemplate> {
    let mut params = IndexMap::new();
    params.insert(
        "rated_cv".to_string(),
        make_param(
            "rated_cv",
            QuantityKind::VolumetricFlow,
            65.0,
            Some(0.1),
            Some(1000.0),
            "Flow coefficient Cv at 100% stroke",
        ),
    );

    let ports = vec![
        make_port(
            "inlet",
            PortDirection::Inlet,
            PortDomain::Material,
            vec![Phase::Liquid, Phase::Gas, Phase::TwoPhase],
        ),
        make_port(
            "outlet",
            PortDirection::Outlet,
            PortDomain::Material,
            vec![Phase::Liquid, Phase::Gas, Phase::TwoPhase],
        ),
        make_port(
            "demand_signal",
            PortDirection::Inlet,
            PortDomain::Signal,
            Vec::new(),
        ),
    ];

    let mut curves = IndexMap::new();
    let mut curve = Curve1D::new(
        "cv_travel",
        "travel",
        "cv_fraction",
        vec![
            (0.0, 0.02),
            (0.2, 0.05),
            (0.4, 0.12),
            (0.6, 0.27),
            (0.8, 0.55),
            (1.0, 1.0),
        ],
    );
    curve.interpolation = InterpolationMethod::MonotonicCubic;
    curve.extrapolation = ExtrapolationMode::Clamp;
    curves.insert("cv_travel".to_string(), curve);

    let def = CustomEquipmentDefinition {
        id: "standard.valve.control_globe".to_string(),
        name: "Globe Control Valve".to_string(),
        version: std_version(),
        category: EquipmentCategory::Valve,
        tier: CustomizationTier::Level1Declarative,
        description: "Pneumatically actuated globe control valve with equal-percentage trim"
            .to_string(),
        parameters: params,
        ports,
        curves,
        curves_2d: IndexMap::new(),
        operating_envelope: Default::default(),
        alarm_definitions: Vec::new(),
        trips: Vec::new(),
        interlocks: Vec::new(),
        script: None,
        composition: None,
        metadata: IndexMap::new(),
    };

    EquipmentTemplate::new(
        "standard.valve.control_globe",
        "Globe Control Valve",
        std_version(),
        EquipmentCategory::Valve,
        TemplateSource::Standard,
        std_meta(
            "Pneumatic globe control valve with equal-percentage trim",
            EquipmentCategory::Valve,
            vec!["valve", "control"],
        ),
        def,
    )
}

/// 5. Vertical Knockout Drum / Separator (`standard.vessel.vertical_separator`)
pub fn make_standard_separator() -> ScriptingResult<EquipmentTemplate> {
    let mut params = IndexMap::new();
    params.insert(
        "volume_m3".to_string(),
        make_param(
            "volume_m3",
            QuantityKind::Volume,
            8.5,
            Some(0.5),
            Some(200.0),
            "Vessel internal volume (m3)",
        ),
    );
    params.insert(
        "design_pressure_bar".to_string(),
        make_param(
            "design_pressure_bar",
            QuantityKind::Pressure,
            2500000.0,
            Some(100000.0),
            Some(15000000.0),
            "MAWP (Pa)",
        ),
    );

    let ports = vec![
        make_port(
            "feed",
            PortDirection::Inlet,
            PortDomain::Material,
            vec![Phase::TwoPhase, Phase::Liquid, Phase::Gas],
        ),
        make_port(
            "gas_out",
            PortDirection::Outlet,
            PortDomain::Material,
            vec![Phase::Gas],
        ),
        make_port(
            "liquid_out",
            PortDirection::Outlet,
            PortDomain::Material,
            vec![Phase::Liquid],
        ),
        make_port(
            "psv_relief",
            PortDirection::Outlet,
            PortDomain::Utility(UtilityKind::Flare),
            Vec::new(),
        ),
    ];

    let trips = vec![TripConditionDefinition {
        id: "trip_high_level".to_string(),
        name: "High High Level Trip".to_string(),
        condition_var: "level".to_string(),
        operator: TripOperator::GreaterThan,
        threshold: 0.92,
        fail_safe_state: "tripped".to_string(),
        latched: true,
    }];

    let def = CustomEquipmentDefinition {
        id: "standard.vessel.vertical_separator".to_string(),
        name: "Vertical Knockout Drum".to_string(),
        version: std_version(),
        category: EquipmentCategory::Process,
        tier: CustomizationTier::Level1Declarative,
        description: "Vertical two-phase gas/liquid separation vessel with demister pad"
            .to_string(),
        parameters: params,
        ports,
        curves: IndexMap::new(),
        curves_2d: IndexMap::new(),
        operating_envelope: Default::default(),
        alarm_definitions: Vec::new(),
        trips,
        interlocks: Vec::new(),
        script: None,
        composition: None,
        metadata: IndexMap::new(),
    };

    EquipmentTemplate::new(
        "standard.vessel.vertical_separator",
        "Vertical Separator",
        std_version(),
        EquipmentCategory::Process,
        TemplateSource::Standard,
        std_meta(
            "Vertical 2-phase gas-liquid separator",
            EquipmentCategory::Process,
            vec!["vessel", "separator"],
        ),
        def,
    )
}

/// 6. Atmospheric Storage Tank (`standard.tank.atmospheric`)
pub fn make_standard_storage_tank() -> ScriptingResult<EquipmentTemplate> {
    let mut params = IndexMap::new();
    params.insert(
        "capacity_m3".to_string(),
        make_param(
            "capacity_m3",
            QuantityKind::Volume,
            500.0,
            Some(10.0),
            Some(50000.0),
            "Storage capacity (m3)",
        ),
    );

    let ports = vec![
        make_port(
            "inlet",
            PortDirection::Inlet,
            PortDomain::Material,
            vec![Phase::Liquid],
        ),
        make_port(
            "outlet",
            PortDirection::Outlet,
            PortDomain::Material,
            vec![Phase::Liquid],
        ),
        make_port(
            "drain_overflow",
            PortDirection::Outlet,
            PortDomain::Utility(UtilityKind::PlantAir),
            Vec::new(),
        ),
    ];

    let def = CustomEquipmentDefinition {
        id: "standard.tank.atmospheric".to_string(),
        name: "Atmospheric Storage Tank".to_string(),
        version: std_version(),
        category: EquipmentCategory::Process,
        tier: CustomizationTier::Level1Declarative,
        description: "Cylindrical vertical storage tank operating at atmospheric pressure"
            .to_string(),
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
    };

    EquipmentTemplate::new(
        "standard.tank.atmospheric",
        "Atmospheric Storage Tank",
        std_version(),
        EquipmentCategory::Process,
        TemplateSource::Standard,
        std_meta(
            "Atmospheric liquid storage tank",
            EquipmentCategory::Process,
            vec!["tank", "storage"],
        ),
        def,
    )
}

/// 7. Shell and Tube Heat Exchanger (`standard.heat_exchanger.shell_and_tube`)
pub fn make_standard_heat_exchanger() -> ScriptingResult<EquipmentTemplate> {
    let mut params = IndexMap::new();
    params.insert(
        "area_m2".to_string(),
        make_param(
            "area_m2",
            QuantityKind::Volume,
            150.0,
            Some(1.0),
            Some(3000.0),
            "Heat transfer surface area (m2)",
        ),
    );

    let ports = vec![
        make_port(
            "tube_in",
            PortDirection::Inlet,
            PortDomain::Material,
            vec![Phase::Liquid, Phase::Gas],
        ),
        make_port(
            "tube_out",
            PortDirection::Outlet,
            PortDomain::Material,
            vec![Phase::Liquid, Phase::Gas],
        ),
        make_port(
            "shell_in",
            PortDirection::Inlet,
            PortDomain::Material,
            vec![Phase::Liquid, Phase::Gas],
        ),
        make_port(
            "shell_out",
            PortDirection::Outlet,
            PortDomain::Material,
            vec![Phase::Liquid, Phase::Gas],
        ),
    ];

    let def = CustomEquipmentDefinition {
        id: "standard.heat_exchanger.shell_and_tube".to_string(),
        name: "Shell and Tube Exchanger".to_string(),
        version: std_version(),
        category: EquipmentCategory::HeatTransfer,
        tier: CustomizationTier::Level1Declarative,
        description: "TEMA-type shell and tube process heat exchanger".to_string(),
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
    };

    EquipmentTemplate::new(
        "standard.heat_exchanger.shell_and_tube",
        "Shell and Tube Exchanger",
        std_version(),
        EquipmentCategory::HeatTransfer,
        TemplateSource::Standard,
        std_meta(
            "Shell and tube thermal exchange unit",
            EquipmentCategory::HeatTransfer,
            vec!["exchanger", "heat"],
        ),
        def,
    )
}

/// 8. Fired Process Heater (`standard.heater.fired_process`)
pub fn make_standard_fired_heater() -> ScriptingResult<EquipmentTemplate> {
    let mut params = IndexMap::new();
    params.insert(
        "rated_duty_mw".to_string(),
        make_param(
            "rated_duty_mw",
            QuantityKind::Power,
            15000000.0,
            Some(500000.0),
            Some(150000000.0),
            "Furnace duty (W)",
        ),
    );

    let ports = vec![
        make_port(
            "process_in",
            PortDirection::Inlet,
            PortDomain::Material,
            vec![Phase::Liquid, Phase::TwoPhase],
        ),
        make_port(
            "process_out",
            PortDirection::Outlet,
            PortDomain::Material,
            vec![Phase::Liquid, Phase::TwoPhase],
        ),
        make_port(
            "fuel_gas",
            PortDirection::Inlet,
            PortDomain::Utility(UtilityKind::FuelGas),
            vec![Phase::Gas],
        ),
        make_port(
            "flue_gas_stack",
            PortDirection::Outlet,
            PortDomain::Utility(UtilityKind::FuelGas),
            vec![Phase::Gas],
        ),
    ];

    let trips = vec![TripConditionDefinition {
        id: "trip_high_skin_temp".to_string(),
        name: "High Skin Temp Trip".to_string(),
        condition_var: "skin_temp".to_string(),
        operator: TripOperator::GreaterThan,
        threshold: 893.15, // Kelvin (620 C)
        fail_safe_state: "tripped".to_string(),
        latched: true,
    }];

    let def = CustomEquipmentDefinition {
        id: "standard.heater.fired_process".to_string(),
        name: "Fired Process Heater".to_string(),
        version: std_version(),
        category: EquipmentCategory::HeatTransfer,
        tier: CustomizationTier::Level1Declarative,
        description: "Gas-fired direct process furnace with radiant and convection sections"
            .to_string(),
        parameters: params,
        ports,
        curves: IndexMap::new(),
        curves_2d: IndexMap::new(),
        operating_envelope: Default::default(),
        alarm_definitions: Vec::new(),
        trips,
        interlocks: Vec::new(),
        script: None,
        composition: None,
        metadata: IndexMap::new(),
    };

    EquipmentTemplate::new(
        "standard.heater.fired_process",
        "Fired Process Heater",
        std_version(),
        EquipmentCategory::HeatTransfer,
        TemplateSource::Standard,
        std_meta(
            "Gas-fired process heater / furnace",
            EquipmentCategory::HeatTransfer,
            vec!["heater", "furnace"],
        ),
        def,
    )
}

/// 9. Continuous Stirred Tank Reactor (`standard.reactor.cstr`)
pub fn make_standard_cstr_reactor() -> ScriptingResult<EquipmentTemplate> {
    let mut params = IndexMap::new();
    params.insert(
        "reactor_volume_m3".to_string(),
        make_param(
            "reactor_volume_m3",
            QuantityKind::Volume,
            12.0,
            Some(0.5),
            Some(100.0),
            "Reactor volume (m3)",
        ),
    );

    let ports = vec![
        make_port(
            "feed",
            PortDirection::Inlet,
            PortDomain::Material,
            vec![Phase::Liquid],
        ),
        make_port(
            "effluent",
            PortDirection::Outlet,
            PortDomain::Material,
            vec![Phase::Liquid],
        ),
        make_port(
            "coolant_in",
            PortDirection::Inlet,
            PortDomain::Utility(UtilityKind::CoolingWater),
            vec![Phase::Liquid],
        ),
        make_port(
            "coolant_out",
            PortDirection::Outlet,
            PortDomain::Utility(UtilityKind::CoolingWater),
            vec![Phase::Liquid],
        ),
    ];

    let trips = vec![TripConditionDefinition {
        id: "trip_runaway_temp".to_string(),
        name: "Thermal Runaway Trip".to_string(),
        condition_var: "temp".to_string(),
        operator: TripOperator::GreaterThan,
        threshold: 438.15,
        fail_safe_state: "tripped".to_string(),
        latched: true,
    }];

    let def = CustomEquipmentDefinition {
        id: "standard.reactor.cstr".to_string(),
        name: "Continuous Stirred Reactor".to_string(),
        version: std_version(),
        category: EquipmentCategory::Process,
        tier: CustomizationTier::Level1Declarative,
        description: "Jacketed continuous stirred tank reactor with exothermic heat kinetics"
            .to_string(),
        parameters: params,
        ports,
        curves: IndexMap::new(),
        curves_2d: IndexMap::new(),
        operating_envelope: Default::default(),
        alarm_definitions: Vec::new(),
        trips,
        interlocks: Vec::new(),
        script: None,
        composition: None,
        metadata: IndexMap::new(),
    };

    EquipmentTemplate::new(
        "standard.reactor.cstr",
        "CSTR Reactor",
        std_version(),
        EquipmentCategory::Process,
        TemplateSource::Standard,
        std_meta(
            "Continuous stirred chemical reaction vessel",
            EquipmentCategory::Process,
            vec!["reactor", "cstr"],
        ),
        def,
    )
}

/// 10. Static In-Line Mixer (`standard.mixer.static_inline`)
pub fn make_standard_mixer() -> ScriptingResult<EquipmentTemplate> {
    let mut params = IndexMap::new();
    params.insert(
        "pressure_loss_factor".to_string(),
        make_param(
            "pressure_loss_factor",
            QuantityKind::Pressure,
            1450.0,
            Some(10.0),
            Some(100000.0),
            "Pressure loss factor (Pa)",
        ),
    );

    let ports = vec![
        make_port(
            "primary_in",
            PortDirection::Inlet,
            PortDomain::Material,
            vec![Phase::Liquid, Phase::Gas],
        ),
        make_port(
            "secondary_in",
            PortDirection::Inlet,
            PortDomain::Material,
            vec![Phase::Liquid, Phase::Gas],
        ),
        make_port(
            "mixture_out",
            PortDirection::Outlet,
            PortDomain::Material,
            vec![Phase::Liquid, Phase::Gas],
        ),
    ];

    let def = CustomEquipmentDefinition {
        id: "standard.mixer.static_inline".to_string(),
        name: "Static In-Line Mixer".to_string(),
        version: std_version(),
        category: EquipmentCategory::Process,
        tier: CustomizationTier::Level1Declarative,
        description: "Motionless in-line turbulent static mixer".to_string(),
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
    };

    EquipmentTemplate::new(
        "standard.mixer.static_inline",
        "Static Mixer",
        std_version(),
        EquipmentCategory::Process,
        TemplateSource::Standard,
        std_meta(
            "Motionless pipeline mixing element",
            EquipmentCategory::Process,
            vec!["mixer", "blending"],
        ),
        def,
    )
}

/// 11. Flow Splitter (`standard.splitter.flow_splitter`)
pub fn make_standard_splitter() -> ScriptingResult<EquipmentTemplate> {
    let mut params = IndexMap::new();
    params.insert(
        "max_flow".to_string(),
        make_param(
            "max_flow",
            QuantityKind::MassFlow,
            100.0,
            Some(0.0),
            Some(1000.0),
            "Max inlet flow (kg/s)",
        ),
    );

    let ports = vec![
        make_port(
            "feed_in",
            PortDirection::Inlet,
            PortDomain::Material,
            vec![Phase::Liquid, Phase::Gas],
        ),
        make_port(
            "branch_a",
            PortDirection::Outlet,
            PortDomain::Material,
            vec![Phase::Liquid, Phase::Gas],
        ),
        make_port(
            "branch_b",
            PortDirection::Outlet,
            PortDomain::Material,
            vec![Phase::Liquid, Phase::Gas],
        ),
    ];

    let def = CustomEquipmentDefinition {
        id: "standard.splitter.flow_splitter".to_string(),
        name: "Flow Splitter".to_string(),
        version: std_version(),
        category: EquipmentCategory::Process,
        tier: CustomizationTier::Level1Declarative,
        description: "Controllable dual-branch manifold flow splitter".to_string(),
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
    };

    EquipmentTemplate::new(
        "standard.splitter.flow_splitter",
        "Flow Splitter",
        std_version(),
        EquipmentCategory::Process,
        TemplateSource::Standard,
        std_meta(
            "Dual-branch stream dividing junction",
            EquipmentCategory::Process,
            vec!["splitter", "flow"],
        ),
        def,
    )
}

/// 12. Pressure / Flow Sensor Transmitter (`standard.sensor.transmitter`)
pub fn make_standard_sensor() -> ScriptingResult<EquipmentTemplate> {
    let mut params = IndexMap::new();
    params.insert(
        "range_max".to_string(),
        make_param(
            "range_max",
            QuantityKind::Pressure,
            3000000.0,
            Some(10000.0),
            Some(50000000.0),
            "Upper measurement range limit (Pa)",
        ),
    );

    let ports = vec![
        make_port(
            "process_tap",
            PortDirection::Inlet,
            PortDomain::Material,
            vec![Phase::Liquid, Phase::Gas],
        ),
        make_port(
            "signal_out",
            PortDirection::Outlet,
            PortDomain::Signal,
            Vec::new(),
        ),
    ];

    let def = CustomEquipmentDefinition {
        id: "standard.sensor.transmitter".to_string(),
        name: "Process Transmitter".to_string(),
        version: std_version(),
        category: EquipmentCategory::Instrumentation,
        tier: CustomizationTier::Level1Declarative,
        description: "Smart 4-20mA HART industrial process transmitter".to_string(),
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
    };

    EquipmentTemplate::new(
        "standard.sensor.transmitter",
        "Process Transmitter",
        std_version(),
        EquipmentCategory::Instrumentation,
        TemplateSource::Standard,
        std_meta(
            "Industrial process variable transmitter",
            EquipmentCategory::Instrumentation,
            vec!["sensor", "signal"],
        ),
        def,
    )
}

/// 13. Composite Package: Pumping Skid with Bypass (`standard.package.pump_bypass_skid`)
pub fn make_standard_pump_bypass_package() -> EquipmentPackage {
    let mut params = IndexMap::new();
    params.insert(
        "skid_flow".to_string(),
        make_param(
            "skid_flow",
            QuantityKind::VolumetricFlow,
            120.0,
            Some(10.0),
            Some(400.0),
            "Rated skid delivery flow (m3/h)",
        ),
    );

    let sub_components = vec![
        PackageSubComponent {
            id: "main_pump".to_string(),
            template_id: "standard.pump.centrifugal".to_string(),
            template_version: Some(std_version()),
            parameter_overrides: BTreeMap::new(),
        },
        PackageSubComponent {
            id: "bypass_valve".to_string(),
            template_id: "standard.valve.control_globe".to_string(),
            template_version: Some(std_version()),
            parameter_overrides: BTreeMap::new(),
        },
    ];

    let internal_streams = vec![PackageInternalStream {
        id: "pump_to_bypass".to_string(),
        from_component: "main_pump".to_string(),
        from_port: "discharge".to_string(),
        to_component: "bypass_valve".to_string(),
        to_port: "inlet".to_string(),
    }];

    let external_ports = vec![
        PackagePortMapping {
            external_port_name: "skid_suction".to_string(),
            direction: PortDirection::Inlet,
            domain: PortDomain::Material,
            internal_component: "main_pump".to_string(),
            internal_port: "suction".to_string(),
        },
        PackagePortMapping {
            external_port_name: "skid_discharge".to_string(),
            direction: PortDirection::Outlet,
            domain: PortDomain::Material,
            internal_component: "main_pump".to_string(),
            internal_port: "discharge".to_string(),
        },
        PackagePortMapping {
            external_port_name: "bypass_spillback".to_string(),
            direction: PortDirection::Outlet,
            domain: PortDomain::Material,
            internal_component: "bypass_valve".to_string(),
            internal_port: "outlet".to_string(),
        },
        PackagePortMapping {
            external_port_name: "bypass_cmd".to_string(),
            direction: PortDirection::Inlet,
            domain: PortDomain::Signal,
            internal_component: "bypass_valve".to_string(),
            internal_port: "demand_signal".to_string(),
        },
    ];

    let parameter_mappings = vec![PackageParameterMapping {
        package_param: "skid_flow".to_string(),
        target_component: "main_pump".to_string(),
        target_param: "rated_flow".to_string(),
    }];

    EquipmentPackage {
        id: "standard.package.pump_bypass_skid".to_string(),
        name: "Pumping Skid with Minimum Flow Bypass".to_string(),
        version: std_version(),
        description:
            "Packaged centrifugal pumping station with automated spillback recirculation valve"
                .to_string(),
        author: "NebuDCS Standard Library".to_string(),
        dependencies: vec![
            PackageDependency {
                id: "standard.pump.centrifugal".to_string(),
                version_req: "^1.0.0".to_string(),
            },
            PackageDependency {
                id: "standard.valve.control_globe".to_string(),
                version_req: "^1.0.0".to_string(),
            },
        ],
        sub_components,
        internal_streams,
        external_ports,
        parameters: params,
        parameter_mappings,
        metadata: IndexMap::new(),
    }
}

/// 14. Composite Package: Two-Stage Compressor Skid (`standard.package.two_stage_compressor_skid`)
pub fn make_standard_two_stage_compressor_package() -> EquipmentPackage {
    let mut params = IndexMap::new();
    params.insert(
        "skid_speed".to_string(),
        make_param(
            "skid_speed",
            QuantityKind::Speed,
            10500.0,
            Some(4000.0),
            Some(15000.0),
            "Common driver shaft speed (rpm)",
        ),
    );

    let sub_components = vec![
        PackageSubComponent {
            id: "stage1".to_string(),
            template_id: "standard.compressor.centrifugal".to_string(),
            template_version: Some(std_version()),
            parameter_overrides: BTreeMap::new(),
        },
        PackageSubComponent {
            id: "intercooler".to_string(),
            template_id: "standard.heat_exchanger.shell_and_tube".to_string(),
            template_version: Some(std_version()),
            parameter_overrides: BTreeMap::new(),
        },
        PackageSubComponent {
            id: "stage2".to_string(),
            template_id: "standard.compressor.centrifugal".to_string(),
            template_version: Some(std_version()),
            parameter_overrides: BTreeMap::new(),
        },
        PackageSubComponent {
            id: "anti_surge_valve".to_string(),
            template_id: "standard.valve.control_globe".to_string(),
            template_version: Some(std_version()),
            parameter_overrides: BTreeMap::new(),
        },
    ];

    let internal_streams = vec![
        PackageInternalStream {
            id: "stage1_to_intercooler".to_string(),
            from_component: "stage1".to_string(),
            from_port: "discharge".to_string(),
            to_component: "intercooler".to_string(),
            to_port: "tube_in".to_string(),
        },
        PackageInternalStream {
            id: "intercooler_to_stage2".to_string(),
            from_component: "intercooler".to_string(),
            from_port: "tube_out".to_string(),
            to_component: "stage2".to_string(),
            to_port: "suction".to_string(),
        },
        PackageInternalStream {
            id: "stage2_to_anti_surge".to_string(),
            from_component: "stage2".to_string(),
            from_port: "discharge".to_string(),
            to_component: "anti_surge_valve".to_string(),
            to_port: "inlet".to_string(),
        },
    ];

    let external_ports = vec![
        PackagePortMapping {
            external_port_name: "package_suction".to_string(),
            direction: PortDirection::Inlet,
            domain: PortDomain::Material,
            internal_component: "stage1".to_string(),
            internal_port: "suction".to_string(),
        },
        PackagePortMapping {
            external_port_name: "package_discharge".to_string(),
            direction: PortDirection::Outlet,
            domain: PortDomain::Material,
            internal_component: "stage2".to_string(),
            internal_port: "discharge".to_string(),
        },
        PackagePortMapping {
            external_port_name: "cooling_water_in".to_string(),
            direction: PortDirection::Inlet,
            domain: PortDomain::Material,
            internal_component: "intercooler".to_string(),
            internal_port: "shell_in".to_string(),
        },
        PackagePortMapping {
            external_port_name: "cooling_water_out".to_string(),
            direction: PortDirection::Outlet,
            domain: PortDomain::Material,
            internal_component: "intercooler".to_string(),
            internal_port: "shell_out".to_string(),
        },
    ];

    let parameter_mappings = vec![PackageParameterMapping {
        package_param: "skid_speed".to_string(),
        target_component: "stage1".to_string(),
        target_param: "design_speed".to_string(),
    }];

    EquipmentPackage {
        id: "standard.package.two_stage_compressor_skid".to_string(),
        name: "Two-Stage Compression Skid".to_string(),
        version: std_version(),
        description:
            "Two-stage centrifugal gas compressor package with intercooler and anti-surge loop"
                .to_string(),
        author: "NebuDCS Standard Library".to_string(),
        dependencies: vec![
            PackageDependency {
                id: "standard.compressor.centrifugal".to_string(),
                version_req: "^1.0.0".to_string(),
            },
            PackageDependency {
                id: "standard.heat_exchanger.shell_and_tube".to_string(),
                version_req: "^1.0.0".to_string(),
            },
            PackageDependency {
                id: "standard.valve.control_globe".to_string(),
                version_req: "^1.0.0".to_string(),
            },
        ],
        sub_components,
        internal_streams,
        external_ports,
        parameters: params,
        parameter_mappings,
        metadata: IndexMap::new(),
    }
}

/// Generates the complete standard library of templates and packages.
pub fn load_all_standard_templates() -> ScriptingResult<Vec<EquipmentTemplate>> {
    Ok(vec![
        make_standard_centrifugal_pump()?,
        make_standard_centrifugal_compressor()?,
        make_standard_fan()?,
        make_standard_control_valve()?,
        make_standard_separator()?,
        make_standard_storage_tank()?,
        make_standard_heat_exchanger()?,
        make_standard_fired_heater()?,
        make_standard_cstr_reactor()?,
        make_standard_mixer()?,
        make_standard_splitter()?,
        make_standard_sensor()?,
    ])
}

/// Generates all standard library composite packages.
pub fn load_all_standard_packages() -> Vec<EquipmentPackage> {
    vec![
        make_standard_pump_bypass_package(),
        make_standard_two_stage_compressor_package(),
    ]
}
