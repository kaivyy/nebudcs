//! Gateway mapping engine, deadband filtering, engineering scaling, and stale detection.
//!
//! Enforces unidirectional or bidirectional bindings between external protocol points
//! (Modbus registers, OPC UA NodeIds, MQTT/Sparkplug metrics) and canonical NebuDCS tags.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::types::{
    AccessLevel, DataSourceKind, IndustrialDataType, IndustrialDataValue, IndustrialQuality,
    IndustrialTagValue, IndustrialTimestamp, TagProvenance,
};
use crate::error::ProtocolError;

/// Direction of signal data flow through the gateway.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MappingDirection {
    /// External System -> Protocol Adapter -> Gateway -> NebuDCS Tag
    Inbound,
    /// NebuDCS Process Tag -> Gateway -> Protocol Adapter -> External System
    Outbound,
    /// Synchronized bidirectional mirror
    Bidirectional,
}

/// Binding configuration connecting an external protocol address to a canonical NebuDCS tag.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TagMapping {
    pub id: String,
    pub connection_id: String,
    pub external_address: String,
    pub canonical_tag: String,
    pub direction: MappingDirection,
    pub datatype: IndustrialDataType,
    pub scale: f64,
    pub offset: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engineering_unit: Option<String>,
    pub deadband: f64,
    pub update_rate_ms: u32,
    pub stale_timeout_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_value: Option<IndustrialDataValue>,
    pub fallback_quality: IndustrialQuality,
    pub access: AccessLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_inbound_value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_outbound_value: Option<f64>,
    pub last_update_timestamp_ms: u64,
}

impl TagMapping {
    pub fn new(
        id: impl Into<String>,
        connection_id: impl Into<String>,
        external_address: impl Into<String>,
        canonical_tag: impl Into<String>,
        direction: MappingDirection,
    ) -> Self {
        Self {
            id: id.into(),
            connection_id: connection_id.into(),
            external_address: external_address.into(),
            canonical_tag: canonical_tag.into(),
            direction,
            datatype: IndustrialDataType::Float64,
            scale: 1.0,
            offset: 0.0,
            engineering_unit: None,
            deadband: 0.0,
            update_rate_ms: 200,
            stale_timeout_ms: 5000,
            fallback_value: None,
            fallback_quality: IndustrialQuality::Stale,
            access: match direction {
                MappingDirection::Inbound => AccessLevel::ReadOnly,
                MappingDirection::Outbound => AccessLevel::WriteOnly,
                MappingDirection::Bidirectional => AccessLevel::ReadWrite,
            },
            last_inbound_value: None,
            last_outbound_value: None,
            last_update_timestamp_ms: 0,
        }
    }

    /// Validates mapping parameters. Rejects scale == 0 and empty addresses.
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.external_address.trim().is_empty() {
            return Err(ProtocolError::InvalidPayload(
                "External address cannot be empty",
            ));
        }
        if self.canonical_tag.trim().is_empty() {
            return Err(ProtocolError::InvalidPayload(
                "Canonical tag cannot be empty",
            ));
        }
        if self.scale.abs() < 1e-12 {
            return Err(ProtocolError::InvalidPayload(
                "Scaling multiplier cannot be zero (singularity in inverse conversion)",
            ));
        }
        Ok(())
    }
}

/// Gateway Mapping Engine evaluating data transformations, deadbands, and stale fallbacks.
#[derive(Debug, Clone, Default)]
pub struct TagMappingEngine {
    mappings: BTreeMap<String, TagMapping>,
}

impl TagMappingEngine {
    pub fn new() -> Self {
        Self {
            mappings: BTreeMap::new(),
        }
    }

    /// Adds and validates a new mapping, checking for output ownership conflicts.
    pub fn add_mapping(&mut self, mapping: TagMapping) -> Result<(), ProtocolError> {
        mapping.validate()?;

        // Ownership Conflict Detection: If this mapping writes into a canonical tag (Inbound),
        // verify no other existing Inbound mapping targets the same canonical tag.
        if matches!(
            mapping.direction,
            MappingDirection::Inbound | MappingDirection::Bidirectional
        ) {
            for existing in self.mappings.values() {
                if existing.id != mapping.id
                    && existing.canonical_tag == mapping.canonical_tag
                    && matches!(
                        existing.direction,
                        MappingDirection::Inbound | MappingDirection::Bidirectional
                    )
                {
                    return Err(ProtocolError::InvalidPayload(format!(
                        "Tag ownership conflict: canonical tag '{}' already has an inbound mapping from '{}'",
                        mapping.canonical_tag, existing.external_address
                    )));
                }
            }
        }

        self.mappings.insert(mapping.id.clone(), mapping);
        Ok(())
    }

    /// Removes a mapping by ID.
    pub fn remove_mapping(&mut self, id: &str) -> Option<TagMapping> {
        self.mappings.remove(id)
    }

    /// Returns a list of all active mappings.
    pub fn list_mappings(&self) -> Vec<&TagMapping> {
        self.mappings.values().collect()
    }

    /// Transforms an inbound external sample into a canonical NebuDCS tag update.
    ///
    /// Applies engineering unit scaling: $EU = Raw \times Scale + Offset$.
    /// Enforces deadband suppression: if $|EU - EU_{prev}| \le Deadband$, update is ignored.
    pub fn process_inbound(
        &mut self,
        connection_id: &str,
        external_address: &str,
        raw_val: &IndustrialDataValue,
        quality: IndustrialQuality,
        timestamp: IndustrialTimestamp,
    ) -> Option<IndustrialTagValue> {
        // Find matching mapping
        let mapping = self.mappings.values_mut().find(|m| {
            m.connection_id == connection_id
                && m.external_address == external_address
                && matches!(
                    m.direction,
                    MappingDirection::Inbound | MappingDirection::Bidirectional
                )
        })?;

        let now_ms = timestamp
            .source_timestamp_ms
            .unwrap_or(timestamp.ingestion_timestamp_ms);
        mapping.last_update_timestamp_ms = now_ms;

        // Apply scaling if numeric
        let (scaled_value, _numeric_val) = match raw_val.as_f64() {
            Some(raw_num) => {
                let eu_num = raw_num * mapping.scale + mapping.offset;
                // Deadband check
                if let Some(prev) = mapping.last_inbound_value {
                    if (eu_num - prev).abs() <= mapping.deadband
                        && quality == IndustrialQuality::Good
                    {
                        return None; // Deadband suppressed
                    }
                }
                mapping.last_inbound_value = Some(eu_num);
                (IndustrialDataValue::Float64(eu_num), Some(eu_num))
            }
            None => {
                // Non-numeric value (boolean, string)
                (raw_val.clone(), None)
            }
        };

        Some(IndustrialTagValue {
            canonical_id: mapping.canonical_tag.clone(),
            source: DataSourceKind::Internal,
            value: scaled_value,
            datatype: mapping.datatype,
            engineering_unit: mapping.engineering_unit.clone(),
            quality,
            timestamp,
            update_rate_ms: mapping.update_rate_ms,
            access: mapping.access,
            provenance: TagProvenance {
                connection_id: connection_id.to_string(),
                remote_address: external_address.to_string(),
                protocol: "GATEWAY".into(),
                zone_id: "ControlZone".into(),
            },
            alarm_state: None,
            diagnostic_state: None,
        })
    }

    /// Evaluates stale timeouts across all active inbound mappings.
    /// Emits fallback value and `IndustrialQuality::Stale` if elapsed time > stale_timeout_ms.
    pub fn check_stale_mappings(&mut self, current_wall_clock_ms: u64) -> Vec<IndustrialTagValue> {
        let mut stale_updates = Vec::new();

        for mapping in self.mappings.values_mut() {
            if !matches!(
                mapping.direction,
                MappingDirection::Inbound | MappingDirection::Bidirectional
            ) {
                continue;
            }
            if mapping.stale_timeout_ms == 0 || mapping.last_update_timestamp_ms == 0 {
                continue;
            }

            let elapsed = current_wall_clock_ms.saturating_sub(mapping.last_update_timestamp_ms);
            if elapsed > mapping.stale_timeout_ms {
                // Check if already marked stale
                let fallback = mapping
                    .fallback_value
                    .clone()
                    .unwrap_or(IndustrialDataValue::Float64(0.0));
                stale_updates.push(IndustrialTagValue {
                    canonical_id: mapping.canonical_tag.clone(),
                    source: DataSourceKind::Internal,
                    value: fallback,
                    datatype: mapping.datatype,
                    engineering_unit: mapping.engineering_unit.clone(),
                    quality: mapping.fallback_quality,
                    timestamp: IndustrialTimestamp::now(None, None),
                    update_rate_ms: mapping.update_rate_ms,
                    access: mapping.access,
                    provenance: TagProvenance {
                        connection_id: mapping.connection_id.clone(),
                        remote_address: mapping.external_address.clone(),
                        protocol: "GATEWAY_STALE".into(),
                        zone_id: "ControlZone".into(),
                    },
                    alarm_state: Some("COMM_STALE".into()),
                    diagnostic_state: Some(format!("Stale timeout: {} ms elapsed", elapsed)),
                });
            }
        }

        stale_updates
    }

    /// Transforms an internal NebuDCS tag update to external representation for dispatch.
    ///
    /// Applies inverse engineering unit scaling: $Raw = \frac{ProcessVal - Offset}{Scale}$.
    pub fn process_outbound(
        &mut self,
        canonical_tag: &str,
        process_val: &IndustrialDataValue,
    ) -> Vec<(String, String, IndustrialDataValue)> {
        let mut outbound = Vec::new();

        for mapping in self.mappings.values_mut() {
            if mapping.canonical_tag != canonical_tag {
                continue;
            }
            if !matches!(
                mapping.direction,
                MappingDirection::Outbound | MappingDirection::Bidirectional
            ) {
                continue;
            }

            let raw_value = match process_val.as_f64() {
                Some(eu_num) => {
                    let raw_num = (eu_num - mapping.offset) / mapping.scale;
                    if let Some(prev) = mapping.last_outbound_value {
                        if (raw_num - prev).abs() <= mapping.deadband {
                            continue; // Suppressed by deadband
                        }
                    }
                    mapping.last_outbound_value = Some(raw_num);
                    IndustrialDataValue::Float64(raw_num)
                }
                None => process_val.clone(),
            };

            outbound.push((
                mapping.connection_id.clone(),
                mapping.external_address.clone(),
                raw_value,
            ));
        }

        outbound
    }
}
