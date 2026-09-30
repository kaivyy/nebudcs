//! Industrial Protocol & Communication Architecture (Stage 7).
//!
//! Provides vendor-neutral industrial communication models, protocol adapters
//! (Modbus TCP, OPC UA, MQTT, Sparkplug B), timestamp/quality semantics,
//! gateway mapping engine, store-and-forward buffers, and ISA/IEC 62443 cybersecurity models.

pub mod adapter;
pub mod mapping;
pub mod modbus;
pub mod mqtt;
pub mod opcua;
pub mod security;
pub mod sparkplug;
pub mod store_forward;
pub mod types;

pub use adapter::{
    AtomicDiagnostics, ConnectionConfig, ConnectionHealth, ConnectionSecurityConfig,
    ConnectionState, ProtocolAdapter, ProtocolDiagnostics, ProtocolType,
};
pub use mapping::{MappingDirection, TagMapping, TagMappingEngine};
pub use modbus::{
    MockModbusServer, ModbusAddressSpec, ModbusByteOrder, ModbusCodec, ModbusTable,
    ModbusTcpAdapter, ModbusWordOrder,
};
pub use mqtt::{MockMqttBroker, MqttAdapter, MqttMessage, MqttQoS};
pub use opcua::{
    MockOpcUaServer, OpcUaAdapter, OpcUaDataValue, OpcUaMonitoredItem, OpcUaNodeId, OpcUaStatusCode,
};
pub use security::{IndustrialConduit, IndustrialZoneDefinition, Isa95Level, SsrfGuard};
pub use sparkplug::{
    MockMqttBroker as SparkplugBroker, SparkplugAdapter, SparkplugEdgeNode, SparkplugMessageType,
    SparkplugMetric, SparkplugPayload, SparkplugTopic,
};
pub use store_forward::{OverflowPolicy, StoreAndForwardQueue};
pub use types::{
    AccessLevel, DataSourceKind, IndustrialDataType, IndustrialDataValue, IndustrialQuality,
    IndustrialTagValue, IndustrialTimestamp, TagProvenance,
};
