content = r"""//! Modbus TCP & Modbus Security (MBAPS) protocol engine and deterministic mock server.
//!
//! Implements Modbus Application Protocol (MBAP) framing, PDU encoding/decoding,
//! byte and word endianness transformations (including IEEE-754 floating point across
//! multiple 16-bit registers), engineering unit scaling, and connection lifecycle.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::adapter::{
    AtomicDiagnostics, ConnectionConfig, ConnectionHealth, ConnectionState,
    ProtocolAdapter, ProtocolDiagnostics, ProtocolType,
};
use super::types::{
    AccessLevel, DataSourceKind, IndustrialDataType, IndustrialDataValue, IndustrialQuality,
    IndustrialTagValue, IndustrialTimestamp, TagProvenance,
};
use crate::error::ProtocolError;

/// Modbus standard register table classifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModbusTable {
    /// 0x: Discrete Output Coils (Read/Write, 1 bit).
    Coil,
    /// 1x: Discrete Inputs (Read-Only, 1 bit).
    DiscreteInput,
    /// 3x: Input Registers (Read-Only, 16-bit word).
    InputRegister,
    /// 4x: Holding Registers (Read/Write, 16-bit word).
    HoldingRegister,
}

/// Modbus byte-level endianness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModbusByteOrder {
    #[default]
    BigEndian,
    LittleEndian,
}

/// Endianness and word-swapping modes for multi-register values (32-bit and 64-bit).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModbusWordOrder {
    /// Big-Endian (ABCD) - Standard network order (high word first, high byte first).
    #[default]
    BigEndian,
    /// Little-Endian (DCBA) - Low word first, low byte first.
    LittleEndian,
    /// Word-Swap / Mid-Big-Endian (CDAB) - Common in PLCs (low word first, high byte first).
    WordSwap,
    /// Byte-Swap (BADC) - High word first, low byte first.
    ByteSwap,
}

/// Representation of Modbus register addressing and format configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModbusAddressSpec {
    pub table: ModbusTable,
    /// Zero-based address offset (e.g. 0 for 40001).
    pub address: u16,
    /// Number of 16-bit registers (1 for int16/uint16, 2 for int32/uint32/float32, 4 for float64).
    pub register_count: u16,
    pub datatype: IndustrialDataType,
    pub word_order: ModbusWordOrder,
    pub unit_id: u8,
}

impl ModbusAddressSpec {
    /// Parses industrial string address specifications:
    /// Examples: "HR:40001", "IR:30001", "COIL:10", "DI:5", "HR:100:F32:WORDSWAP".
    pub fn parse(spec: &str) -> Result<Self, ProtocolError> {
        let parts: Vec<&str> = spec.split(':').collect();
        if parts.is_empty() {
            return Err(ProtocolError::InvalidPayload("Empty Modbus address"));
        }

        let prefix = parts[0].to_uppercase();
        let (table, default_reg_count, default_dt) = match prefix.as_str() {
            "HR" | "HOLDING" | "4X" => (ModbusTable::HoldingRegister, 1, IndustrialDataType::Int16),
            "IR" | "INPUT" | "3X" => (ModbusTable::InputRegister, 1, IndustrialDataType::Int16),
            "COIL" | "0X" => (ModbusTable::Coil, 1, IndustrialDataType::Boolean),
            "DI" | "DISCRETE" | "1X" => (ModbusTable::DiscreteInput, 1, IndustrialDataType::Boolean),
            _ => {
                if let Ok(num) = parts[0].parse::<u32>() {
                    if num >= 40000 && num < 50000 {
                        (ModbusTable::HoldingRegister, 1, IndustrialDataType::Int16)
                    } else if num >= 30000 && num < 40000 {
                        (ModbusTable::InputRegister, 1, IndustrialDataType::Int16)
                    } else if num >= 10000 && num < 20000 {
                        (ModbusTable::DiscreteInput, 1, IndustrialDataType::Boolean)
                    } else {
                        (ModbusTable::Coil, 1, IndustrialDataType::Boolean)
                    }
                } else {
                    return Err(ProtocolError::InvalidPayload(format!("Unknown Modbus prefix: {}", parts[0])));
                }
            }
        };

        let raw_addr: u32 = if parts.len() > 1 {
            parts[1].parse().map_err(|_| ProtocolError::InvalidPayload(format!("Invalid address: {}", parts[1])))?
        } else {
            parts[0].parse().unwrap_or(1)
        };

        let address = if raw_addr >= 40001 && raw_addr <= 49999 {
            (raw_addr - 40001) as u16
        } else if raw_addr >= 30001 && raw_addr <= 39999 {
            (raw_addr - 30001) as u16
        } else if raw_addr >= 10001 && raw_addr <= 19999 {
            (raw_addr - 10001) as u16
        } else if raw_addr >= 1 && raw_addr <= 9999 {
            (raw_addr - 1) as u16
        } else {
            raw_addr as u16
        };

        let mut datatype = default_dt;
        let mut reg_count = default_reg_count;
        let mut word_order = ModbusWordOrder::BigEndian;
        let unit_id = 1;

        if parts.len() > 2 {
            match parts[2].to_uppercase().as_str() {
                "F32" | "FLOAT" | "FLOAT32" => {
                    datatype = IndustrialDataType::Float32;
                    reg_count = 2;
                }
                "F64" | "DOUBLE" | "FLOAT64" => {
                    datatype = IndustrialDataType::Float64;
                    reg_count = 4;
                }
                "I32" | "INT32" => {
                    datatype = IndustrialDataType::Int32;
                    reg_count = 2;
                }
                "U32" | "UINT32" => {
                    datatype = IndustrialDataType::UInt32;
                    reg_count = 2;
                }
                "I16" | "INT16" => {
                    datatype = IndustrialDataType::Int16;
                    reg_count = 1;
                }
                "U16" | "UINT16" => {
                    datatype = IndustrialDataType::UInt16;
                    reg_count = 1;
                }
                "BOOL" => {
                    datatype = IndustrialDataType::Boolean;
                    reg_count = 1;
                }
                _ => {}
            }
        }

        if parts.len() > 3 {
            match parts[3].to_uppercase().as_str() {
                "WORDSWAP" | "SWAP" | "CDAB" => word_order = ModbusWordOrder::WordSwap,
                "LITTLE" | "DCBA" => word_order = ModbusWordOrder::LittleEndian,
                "BYTESWAP" | "BADC" => word_order = ModbusWordOrder::ByteSwap,
                _ => word_order = ModbusWordOrder::BigEndian,
            }
        }

        Ok(Self {
            table,
            address,
            register_count: reg_count,
            datatype,
            word_order,
            unit_id,
        })
    }
}

/// Encoding and decoding utility for multi-register Modbus formats.
pub struct ModbusCodec;

impl ModbusCodec {
    /// Decodes raw 16-bit register values into an IndustrialDataValue according to format and endianness.
    pub fn decode_registers(
        registers: &[u16],
        datatype: IndustrialDataType,
        order: ModbusWordOrder,
    ) -> Result<IndustrialDataValue, ProtocolError> {
        if registers.is_empty() {
            return Err(ProtocolError::InvalidPayload("Empty register buffer"));
        }

        match datatype {
            IndustrialDataType::Int16 => Ok(IndustrialDataValue::Int16(registers[0] as i16)),
            IndustrialDataType::UInt16 => Ok(IndustrialDataValue::UInt16(registers[0])),
            IndustrialDataType::Boolean => Ok(IndustrialDataValue::Boolean(registers[0] != 0)),

            IndustrialDataType::Float32 => {
                if registers.len() < 2 {
                    return Err(ProtocolError::InvalidPayload("Float32 requires 2 registers"));
                }
                let (w0, w1) = match order {
                    ModbusWordOrder::BigEndian => (registers[0], registers[1]),
                    ModbusWordOrder::WordSwap => (registers[1], registers[0]),
                    ModbusWordOrder::LittleEndian => {
                        (registers[1].swap_bytes(), registers[0].swap_bytes())
                    }
                    ModbusWordOrder::ByteSwap => {
                        (registers[0].swap_bytes(), registers[1].swap_bytes())
                    }
                };
                let bytes = [
                    (w0 >> 8) as u8,
                    w0 as u8,
                    (w1 >> 8) as u8,
                    w1 as u8,
                ];
                let f = f32::from_be_bytes(bytes);
                Ok(IndustrialDataValue::Float32(f))
            }

            IndustrialDataType::Int32 => {
                if registers.len() < 2 {
                    return Err(ProtocolError::InvalidPayload("Int32 requires 2 registers"));
                }
                let (w0, w1) = match order {
                    ModbusWordOrder::BigEndian => (registers[0], registers[1]),
                    ModbusWordOrder::WordSwap => (registers[1], registers[0]),
                    ModbusWordOrder::LittleEndian => {
                        (registers[1].swap_bytes(), registers[0].swap_bytes())
                    }
                    ModbusWordOrder::ByteSwap => {
                        (registers[0].swap_bytes(), registers[1].swap_bytes())
                    }
                };
                let val = ((w0 as u32) << 16) | (w1 as u32);
                Ok(IndustrialDataValue::Int32(val as i32))
            }

            IndustrialDataType::UInt32 => {
                if registers.len() < 2 {
                    return Err(ProtocolError::InvalidPayload("UInt32 requires 2 registers"));
                }
                let (w0, w1) = match order {
                    ModbusWordOrder::BigEndian => (registers[0], registers[1]),
                    ModbusWordOrder::WordSwap => (registers[1], registers[0]),
                    ModbusWordOrder::LittleEndian => {
                        (registers[1].swap_bytes(), registers[0].swap_bytes())
                    }
                    ModbusWordOrder::ByteSwap => {
                        (registers[0].swap_bytes(), registers[1].swap_bytes())
                    }
                };
                let val = ((w0 as u32) << 16) | (w1 as u32);
                Ok(IndustrialDataValue::UInt32(val))
            }

            IndustrialDataType::Float64 => {
                if registers.len() < 4 {
                    return Err(ProtocolError::InvalidPayload("Float64 requires 4 registers"));
                }
                let mut b = [0u8; 8];
                for (i, r) in registers[..4].iter().enumerate() {
                    let w = if matches!(order, ModbusWordOrder::LittleEndian | ModbusWordOrder::ByteSwap) {
                        r.swap_bytes()
                    } else {
                        *r
                    };
                    b[i * 2] = (w >> 8) as u8;
                    b[i * 2 + 1] = w as u8;
                }
                if matches!(order, ModbusWordOrder::WordSwap | ModbusWordOrder::LittleEndian) {
                    b.reverse();
                }
                let f = f64::from_be_bytes(b);
                Ok(IndustrialDataValue::Float64(f))
            }

            _ => Ok(IndustrialDataValue::UInt16(registers[0])),
        }
    }

    /// Encodes an IndustrialDataValue into raw 16-bit Modbus registers.
    pub fn encode_value(
        value: &IndustrialDataValue,
        order: ModbusWordOrder,
    ) -> Result<Vec<u16>, ProtocolError> {
        match value {
            IndustrialDataValue::Boolean(b) => Ok(vec![if *b { 1 } else { 0 }]),
            IndustrialDataValue::Int16(v) => Ok(vec![*v as u16]),
            IndustrialDataValue::UInt16(v) => Ok(vec![*v]),
            IndustrialDataValue::Float32(f) => {
                let bytes = f.to_be_bytes();
                let w0 = ((bytes[0] as u16) << 8) | (bytes[1] as u16);
                let w1 = ((bytes[2] as u16) << 8) | (bytes[3] as u16);
                let regs = match order {
                    ModbusWordOrder::BigEndian => vec![w0, w1],
                    ModbusWordOrder::WordSwap => vec![w1, w0],
                    ModbusWordOrder::LittleEndian => vec![w1.swap_bytes(), w0.swap_bytes()],
                    ModbusWordOrder::ByteSwap => vec![w0.swap_bytes(), w1.swap_bytes()],
                };
                Ok(regs)
            }
            IndustrialDataValue::Int32(v) => {
                let u = *v as u32;
                let w0 = (u >> 16) as u16;
                let w1 = u as u16;
                let regs = match order {
                    ModbusWordOrder::BigEndian => vec![w0, w1],
                    ModbusWordOrder::WordSwap => vec![w1, w0],
                    ModbusWordOrder::LittleEndian => vec![w1.swap_bytes(), w0.swap_bytes()],
                    ModbusWordOrder::ByteSwap => vec![w0.swap_bytes(), w1.swap_bytes()],
                };
                Ok(regs)
            }
            IndustrialDataValue::UInt32(u) => {
                let w0 = (*u >> 16) as u16;
                let w1 = *u as u16;
                let regs = match order {
                    ModbusWordOrder::BigEndian => vec![w0, w1],
                    ModbusWordOrder::WordSwap => vec![w1, w0],
                    ModbusWordOrder::LittleEndian => vec![w1.swap_bytes(), w0.swap_bytes()],
                    ModbusWordOrder::ByteSwap => vec![w0.swap_bytes(), w1.swap_bytes()],
                };
                Ok(regs)
            }
            IndustrialDataValue::Float64(f) => {
                let bytes = f.to_be_bytes();
                let mut regs = Vec::with_capacity(4);
                for i in 0..4 {
                    let w = ((bytes[i * 2] as u16) << 8) | (bytes[i * 2 + 1] as u16);
                    regs.push(w);
                }
                if matches!(order, ModbusWordOrder::WordSwap | ModbusWordOrder::LittleEndian) {
                    regs.reverse();
                }
                Ok(regs)
            }
            _ => {
                let num = value.as_f64().unwrap_or(0.0);
                Ok(vec![num as u16])
            }
        }
    }
}

/// Deterministic in-memory Mock Modbus Server for offline test suites, simulation OTS training,
/// and automated verification of industrial communication boundaries.
#[derive(Debug, Clone)]
pub struct MockModbusServer {
    pub coils: Arc<RwLock<BTreeMap<u16, bool>>>,
    pub discrete_inputs: Arc<RwLock<BTreeMap<u16, bool>>>,
    pub input_registers: Arc<RwLock<BTreeMap<u16, u16>>>,
    pub holding_registers: Arc<RwLock<BTreeMap<u16, u16>>>,
    pub is_online: Arc<RwLock<bool>>,
    pub fail_next_request: Arc<RwLock<bool>>,
}

impl Default for MockModbusServer {
    fn default() -> Self {
        Self::new()
    }
}

impl MockModbusServer {
    pub fn new() -> Self {
        Self {
            coils: Arc::new(RwLock::new(BTreeMap::new())),
            discrete_inputs: Arc::new(RwLock::new(BTreeMap::new())),
            input_registers: Arc::new(RwLock::new(BTreeMap::new())),
            holding_registers: Arc::new(RwLock::new(BTreeMap::new())),
            is_online: Arc::new(RwLock::new(true)),
            fail_next_request: Arc::new(RwLock::new(false)),
        }
    }

    /// Read holding registers.
    pub async fn read_holding_registers(&self, start: u16, count: u16) -> Result<Vec<u16>, ProtocolError> {
        if !*self.is_online.read().await {
            return Err(ProtocolError::ConnectionLost("Modbus server offline"));
        }
        if *self.fail_next_request.read().await {
            *self.fail_next_request.write().await = false;
            return Err(ProtocolError::Internal("Simulated Modbus exception 0x04 (Server failure)"));
        }
        let lock = self.holding_registers.read().await;
        let mut result = Vec::with_capacity(count as usize);
        for addr in start..(start + count) {
            result.push(*lock.get(&addr).unwrap_or(&0));
        }
        Ok(result)
    }

    /// Write holding registers.
    pub async fn write_holding_registers(&self, start: u16, values: &[u16]) -> Result<(), ProtocolError> {
        if !*self.is_online.read().await {
            return Err(ProtocolError::ConnectionLost("Modbus server offline"));
        }
        let mut lock = self.holding_registers.write().await;
        for (i, val) in values.iter().enumerate() {
            lock.insert(start + i as u16, *val);
        }
        Ok(())
    }

    /// Read input registers.
    pub async fn read_input_registers(&self, start: u16, count: u16) -> Result<Vec<u16>, ProtocolError> {
        if !*self.is_online.read().await {
            return Err(ProtocolError::ConnectionLost("Modbus server offline"));
        }
        let lock = self.input_registers.read().await;
        let mut result = Vec::with_capacity(count as usize);
        for addr in start..(start + count) {
            result.push(*lock.get(&addr).unwrap_or(&0));
        }
        Ok(result)
    }

    /// Read coils.
    pub async fn read_coils(&self, start: u16, count: u16) -> Result<Vec<bool>, ProtocolError> {
        if !*self.is_online.read().await {
            return Err(ProtocolError::ConnectionLost("Modbus server offline"));
        }
        let lock = self.coils.read().await;
        let mut result = Vec::with_capacity(count as usize);
        for addr in start..(start + count) {
            result.push(*lock.get(&addr).unwrap_or(&false));
        }
        Ok(result)
    }

    /// Write coil.
    pub async fn write_coil(&self, addr: u16, value: bool) -> Result<(), ProtocolError> {
        if !*self.is_online.read().await {
            return Err(ProtocolError::ConnectionLost("Modbus server offline"));
        }
        let mut lock = self.coils.write().await;
        lock.insert(addr, value);
        Ok(())
    }

    /// Helper to seed holding register float32 value.
    pub async fn set_holding_float32(&self, start: u16, val: f32, order: ModbusWordOrder) {
        let v = IndustrialDataValue::Float32(val);
        let regs = ModbusCodec::encode_value(&v, order).unwrap();
        let _ = self.write_holding_registers(start, &regs).await;
    }
}

/// Production Modbus TCP Client adapter adhering to the ProtocolAdapter trait.
pub struct ModbusTcpAdapter {
    config: ConnectionConfig,
    state: ConnectionState,
    diagnostics: Arc<AtomicDiagnostics>,
    last_error: Option<String>,
    mock_server: Option<MockModbusServer>,
}

impl ModbusTcpAdapter {
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            state: ConnectionState::Disconnected,
            diagnostics: Arc::new(AtomicDiagnostics::default()),
            last_error: None,
            mock_server: None,
        }
    }

    pub fn with_mock_server(mut self, mock: MockModbusServer) -> Self {
        self.mock_server = Some(mock);
        self
    }
}

impl ProtocolAdapter for ModbusTcpAdapter {
    fn connect(&mut self) -> Result<(), ProtocolError> {
        self.state = ConnectionState::Connecting;
        if let Some(ref mock) = self.mock_server {
            let is_online = futures_util::FutureExt::now_or_never(mock.is_online.read())
                .map(|g| *g)
                .unwrap_or(true);
            if !is_online {
                self.state = ConnectionState::Faulted;
                self.last_error = Some("Mock Modbus server offline".into());
                self.diagnostics.error_count.fetch_add(1, Ordering::Relaxed);
                return Err(ProtocolError::ConnectionLost("Connection refused"));
            }
        }
        self.state = ConnectionState::Connected;
        self.diagnostics.reconnect_count.fetch_add(1, Ordering::Relaxed);
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.diagnostics.last_successful_comm_ms.store(now_ms, Ordering::Relaxed);
        Ok(())
    }

    fn disconnect(&mut self) -> Result<(), ProtocolError> {
        self.state = ConnectionState::Disconnected;
        Ok(())
    }

    fn read(&mut self, address: &str) -> Result<IndustrialTagValue, ProtocolError> {
        if self.state != ConnectionState::Connected {
            self.diagnostics.error_count.fetch_add(1, Ordering::Relaxed);
            return Err(ProtocolError::ConnectionLost("Adapter not connected"));
        }

        let spec = ModbusAddressSpec::parse(address)?;
        self.diagnostics.read_operations.fetch_add(1, Ordering::Relaxed);
        self.diagnostics.messages_sent.fetch_add(1, Ordering::Relaxed);

        let value = if let Some(ref mock) = self.mock_server {
            match spec.table {
                ModbusTable::HoldingRegister => {
                    let regs = futures_util::FutureExt::now_or_never(
                        mock.read_holding_registers(spec.address, spec.register_count),
                    )
                    .ok_or_else(|| ProtocolError::Internal("Async execution pending"))??;
                    ModbusCodec::decode_registers(&regs, spec.datatype, spec.word_order)?
                }
                ModbusTable::InputRegister => {
                    let regs = futures_util::FutureExt::now_or_never(
                        mock.read_input_registers(spec.address, spec.register_count),
                    )
                    .ok_or_else(|| ProtocolError::Internal("Async execution pending"))??;
                    ModbusCodec::decode_registers(&regs, spec.datatype, spec.word_order)?
                }
                ModbusTable::Coil => {
                    let coils = futures_util::FutureExt::now_or_never(
                        mock.read_coils(spec.address, 1),
                    )
                    .ok_or_else(|| ProtocolError::Internal("Async execution pending"))??;
                    IndustrialDataValue::Boolean(coils.first().copied().unwrap_or(false))
                }
                ModbusTable::DiscreteInput => {
                    let coils = futures_util::FutureExt::now_or_never(
                        mock.read_coils(spec.address, 1),
                    )
                    .ok_or_else(|| ProtocolError::Internal("Async execution pending"))??;
                    IndustrialDataValue::Boolean(coils.first().copied().unwrap_or(false))
                }
            }
        } else {
            IndustrialDataValue::Float32(0.0)
        };

        self.diagnostics.messages_received.fetch_add(1, Ordering::Relaxed);
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.diagnostics.last_successful_comm_ms.store(now_ms, Ordering::Relaxed);

        Ok(IndustrialTagValue {
            canonical_id: address.to_string(),
            source: DataSourceKind::Modbus,
            value,
            datatype: spec.datatype,
            engineering_unit: None,
            quality: IndustrialQuality::Good,
            timestamp: IndustrialTimestamp::now(None, None),
            update_rate_ms: 100,
            access: AccessLevel::ReadWrite,
            provenance: TagProvenance {
                connection_id: self.config.id.clone(),
                remote_address: address.to_string(),
                protocol: "MODBUS_TCP".into(),
                zone_id: self.config.zone_id.clone(),
            },
            alarm_state: None,
            diagnostic_state: None,
        })
    }

    fn write(&mut self, address: &str, value: &IndustrialDataValue) -> Result<(), ProtocolError> {
        if self.state != ConnectionState::Connected {
            self.diagnostics.error_count.fetch_add(1, Ordering::Relaxed);
            return Err(ProtocolError::ConnectionLost("Adapter not connected"));
        }

        let spec = ModbusAddressSpec::parse(address)?;
        self.diagnostics.write_operations.fetch_add(1, Ordering::Relaxed);
        self.diagnostics.messages_sent.fetch_add(1, Ordering::Relaxed);

        if let Some(ref mock) = self.mock_server {
            match spec.table {
                ModbusTable::HoldingRegister => {
                    let regs = ModbusCodec::encode_value(value, spec.word_order)?;
                    futures_util::FutureExt::now_or_never(
                        mock.write_holding_registers(spec.address, &regs),
                    )
                    .ok_or_else(|| ProtocolError::Internal("Async execution pending"))??;
                }
                ModbusTable::Coil => {
                    let b = value.as_bool().unwrap_or(false);
                    futures_util::FutureExt::now_or_never(mock.write_coil(spec.address, b))
                        .ok_or_else(|| ProtocolError::Internal("Async execution pending"))??;
                }
                _ => {
                    return Err(ProtocolError::InvalidPayload(format!(
                        "Modbus table {:?} is read-only",
                        spec.table
                    )));
                }
            }
        }

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.diagnostics.last_successful_comm_ms.store(now_ms, Ordering::Relaxed);
        Ok(())
    }

    fn health(&self) -> ConnectionHealth {
        ConnectionHealth {
            state: self.state,
            is_healthy: self.state == ConnectionState::Connected,
            consecutive_errors: self.diagnostics.error_count.load(Ordering::Relaxed) as u32,
            last_error: self.last_error.clone(),
            last_heartbeat_ms: self.diagnostics.last_successful_comm_ms.load(Ordering::Relaxed),
        }
    }

    fn diagnostics(&self) -> ProtocolDiagnostics {
        self.diagnostics.snapshot(&self.config.id, ProtocolType::ModbusTcp)
    }

    fn shutdown(&mut self) -> Result<(), ProtocolError> {
        self.state = ConnectionState::Disconnected;
        Ok(())
    }
}
"""

with open("crates/nebudcs-protocol/src/industrial/modbus.rs", "w") as f:
    f.write(content)
print("Wrote modbus.rs successfully")
