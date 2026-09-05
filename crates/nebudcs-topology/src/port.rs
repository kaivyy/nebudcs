//! Port compatibility validation rules.
//!
//! Evaluates whether two equipment ports can form a valid process, utility,
//! or signal connection according to their domain, direction, phase compatibility,
//! and cardinality rules.

use nebudcs_model::equipment::{
    Cardinality, Phase, PortDefinition, PortDirection, PortDomain, UtilityKind,
};
use nebudcs_model::stream::StreamType;
use nebudcs_types::{EquipmentId, PortId};

use crate::error::TopologyError;

/// Validates whether a connection can be established from `source_port` on `source_eq`
/// to `sink_port` on `sink_eq`.
pub fn validate_port_compatibility(
    source_eq: EquipmentId,
    source_port: &PortDefinition,
    source_curr_conns: usize,
    sink_eq: EquipmentId,
    sink_port: &PortDefinition,
    sink_curr_conns: usize,
    stream_type: Option<&StreamType>,
) -> Result<(), TopologyError> {
    // 1. Cannot connect an identical port on the same equipment to itself
    if source_eq == sink_eq && source_port.id == sink_port.id {
        return Err(TopologyError::SelfPortConnection {
            equipment_id: source_eq,
            port_id: source_port.id,
        });
    }

    // 2. Domain compatibility check
    validate_domains(&source_port.domain, &sink_port.domain)?;

    // 3. StreamType vs Domain consistency if stream_type is provided
    if let Some(st) = stream_type {
        validate_stream_type_domain(st, &source_port.domain)?;
        validate_stream_type_domain(st, &sink_port.domain)?;
    }

    // 4. Direction compatibility check
    validate_directions(source_port.direction, sink_port.direction)?;

    // 5. Phase compatibility check (only applies if both specify phases)
    validate_phases(
        &source_port.phase_compatibility,
        &sink_port.phase_compatibility,
    )?;

    // 6. Cardinality check for source port (+1 connection)
    validate_cardinality(
        source_eq,
        source_port.id,
        source_port.cardinality,
        source_curr_conns,
    )?;

    // 7. Cardinality check for sink port (+1 connection)
    validate_cardinality(
        sink_eq,
        sink_port.id,
        sink_port.cardinality,
        sink_curr_conns,
    )?;

    Ok(())
}

/// Validates that two port domains can interconnect.
pub fn validate_domains(source: &PortDomain, sink: &PortDomain) -> Result<(), TopologyError> {
    match (source, sink) {
        (PortDomain::Material, PortDomain::Material) => Ok(()),
        (PortDomain::Utility(u1), PortDomain::Utility(u2)) if u1 == u2 => Ok(()),
        (PortDomain::Electrical, PortDomain::Electrical) => Ok(()),
        (PortDomain::Mechanical, PortDomain::Mechanical) => Ok(()),
        (PortDomain::Signal, PortDomain::Signal) => Ok(()),
        (PortDomain::Catalyst, PortDomain::Catalyst) => Ok(()),
        (PortDomain::Energy, PortDomain::Energy) => Ok(()),
        _ => Err(TopologyError::DomainMismatch {
            source_domain: source.clone(),
            sink_domain: sink.clone(),
        }),
    }
}

/// Validates that a StreamType matches the port's domain.
pub fn validate_stream_type_domain(
    stream_type: &StreamType,
    port_domain: &PortDomain,
) -> Result<(), TopologyError> {
    let matches = matches!(
        (stream_type, port_domain),
        (StreamType::Material, PortDomain::Material)
            | (
                StreamType::UtilityHydrogen,
                PortDomain::Utility(UtilityKind::Hydrogen)
            )
            | (
                StreamType::UtilitySteamHp,
                PortDomain::Utility(UtilityKind::SteamHp)
            )
            | (
                StreamType::UtilitySteamMp,
                PortDomain::Utility(UtilityKind::SteamMp)
            )
            | (
                StreamType::UtilitySteamLp,
                PortDomain::Utility(UtilityKind::SteamLp)
            )
            | (
                StreamType::UtilityCoolingWater,
                PortDomain::Utility(UtilityKind::CoolingWater)
            )
            | (
                StreamType::UtilityInstrumentAir,
                PortDomain::Utility(UtilityKind::InstrumentAir)
            )
            | (
                StreamType::UtilityPlantAir,
                PortDomain::Utility(UtilityKind::PlantAir)
            )
            | (
                StreamType::UtilityNitrogen,
                PortDomain::Utility(UtilityKind::Nitrogen)
            )
            | (
                StreamType::UtilityFuelGas,
                PortDomain::Utility(UtilityKind::FuelGas)
            )
            | (
                StreamType::UtilityElectrical,
                PortDomain::Utility(UtilityKind::Electricity)
            )
            | (
                StreamType::UtilityFlare,
                PortDomain::Utility(UtilityKind::Flare)
            )
            | (StreamType::CatalystTransport, PortDomain::Catalyst)
    );

    if matches {
        Ok(())
    } else {
        Err(TopologyError::StreamTypeDomainMismatch {
            stream_type: format!("{stream_type:?}"),
            port_domain: port_domain.clone(),
        })
    }
}

/// Validates that source and sink directions are compatible.
///
/// In connection semantics:
/// - Outlet can connect to Inlet or Bidirectional
/// - Bidirectional can connect to Inlet, Outlet, or Bidirectional
/// - Inlet CANNOT connect to Inlet (two inputs cannot feed each other)
/// - Outlet CANNOT connect to Outlet (two outputs cannot push against each other without merge element)
pub fn validate_directions(
    source_dir: PortDirection,
    sink_dir: PortDirection,
) -> Result<(), TopologyError> {
    match (source_dir, sink_dir) {
        (PortDirection::Outlet, PortDirection::Inlet) => Ok(()),
        (PortDirection::Outlet, PortDirection::Bidirectional) => Ok(()),
        (PortDirection::Bidirectional, PortDirection::Inlet) => Ok(()),
        (PortDirection::Bidirectional, PortDirection::Outlet) => Ok(()),
        (PortDirection::Bidirectional, PortDirection::Bidirectional) => Ok(()),
        _ => Err(TopologyError::DirectionMismatch {
            source_dir,
            sink_dir,
        }),
    }
}

/// Validates that the phases supported by two ports have non-empty intersection,
/// unless one or both declare no phase constraints (empty vector = universal).
pub fn validate_phases(
    source_phases: &[Phase],
    sink_phases: &[Phase],
) -> Result<(), TopologyError> {
    if source_phases.is_empty() || sink_phases.is_empty() {
        return Ok(());
    }

    let has_overlap = source_phases.iter().any(|p1| {
        sink_phases.iter().any(|p2| {
            matches!(
                (p1, p2),
                (Phase::Liquid, Phase::Liquid)
                    | (Phase::Gas, Phase::Gas)
                    | (Phase::TwoPhase, _)
                    | (_, Phase::TwoPhase)
                    | (Phase::Solid, Phase::Solid)
            )
        })
    });

    if has_overlap {
        Ok(())
    } else {
        Err(TopologyError::PhaseMismatch {
            source_phases: source_phases.to_vec(),
            sink_phases: sink_phases.to_vec(),
        })
    }
}

/// Validates cardinality limit when adding one new connection.
pub fn validate_cardinality(
    equipment_id: EquipmentId,
    port_id: PortId,
    cardinality: Cardinality,
    current_count: usize,
) -> Result<(), TopologyError> {
    let allowed = match cardinality {
        Cardinality::ExactlyOne => current_count < 1,
        Cardinality::ZeroOrOne => current_count < 1,
        Cardinality::OneOrMore | Cardinality::ZeroOrMore => true,
    };

    if allowed {
        Ok(())
    } else {
        Err(TopologyError::CardinalityExceeded {
            equipment_id,
            port_id,
            cardinality,
            current_count,
        })
    }
}
