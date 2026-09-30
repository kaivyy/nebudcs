//! IEC 61131-3 Elementary and Composite Data Types.
//!
//! Provides strict industrial type safety, deterministic conversions,
//! typed literals, bounds checking, and persistence.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

use crate::error::ControlError;

/// Elementary and composite data type classifications per IEC 61131-3 3rd Ed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IecType {
    // Boolean
    Bool,

    // Bit Strings
    Byte,
    Word,
    DWord,
    LWord,

    // Signed Integers
    Sint, // 8-bit
    Int,  // 16-bit
    Dint, // 32-bit
    Lint, // 64-bit

    // Unsigned Integers
    Usint, // 8-bit
    Uint,  // 16-bit
    Udint, // 32-bit
    Ulint, // 64-bit

    // Floating Point
    Real,  // 32-bit IEEE 754
    LReal, // 64-bit IEEE 754

    // Time & Duration
    Time, // Microsecond-precision duration (seconds as f64)
    Date, // Seconds since Unix epoch (date component)
    Tod,  // Time Of Day (seconds since midnight)
    Dt,   // Date and Time (seconds since Unix epoch)

    // Character Strings
    String,

    // Composite Types
    Array {
        element_type: Box<IecType>,
        lower_bound: i32,
        upper_bound: i32,
    },
    Struct {
        type_name: String,
        fields: Vec<(String, IecType)>,
    },
    Enum {
        type_name: String,
        variants: Vec<String>,
    },
}

impl IecType {
    /// Returns the canonical IEC keyword name (e.g. "BOOL", "REAL", "DINT").
    pub fn keyword(&self) -> String {
        match self {
            Self::Bool => "BOOL".to_string(),
            Self::Byte => "BYTE".to_string(),
            Self::Word => "WORD".to_string(),
            Self::DWord => "DWORD".to_string(),
            Self::LWord => "LWORD".to_string(),
            Self::Sint => "SINT".to_string(),
            Self::Int => "INT".to_string(),
            Self::Dint => "DINT".to_string(),
            Self::Lint => "LINT".to_string(),
            Self::Usint => "USINT".to_string(),
            Self::Uint => "UINT".to_string(),
            Self::Udint => "UDINT".to_string(),
            Self::Ulint => "ULINT".to_string(),
            Self::Real => "REAL".to_string(),
            Self::LReal => "LREAL".to_string(),
            Self::Time => "TIME".to_string(),
            Self::Date => "DATE".to_string(),
            Self::Tod => "TOD".to_string(),
            Self::Dt => "DT".to_string(),
            Self::String => "STRING".to_string(),
            Self::Array {
                element_type,
                lower_bound,
                upper_bound,
            } => format!(
                "ARRAY [{}..{}] OF {}",
                lower_bound,
                upper_bound,
                element_type.keyword()
            ),
            Self::Struct { type_name, .. } => type_name.clone(),
            Self::Enum { type_name, .. } => type_name.clone(),
        }
    }

    /// Whether this type is a numeric integer or real.
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            Self::Sint
                | Self::Int
                | Self::Dint
                | Self::Lint
                | Self::Usint
                | Self::Uint
                | Self::Udint
                | Self::Ulint
                | Self::Real
                | Self::LReal
                | Self::Byte
                | Self::Word
                | Self::DWord
                | Self::LWord
        )
    }

    /// Default zero/empty initialized value for this type.
    pub fn default_value(&self) -> IecValue {
        match self {
            Self::Bool => IecValue::Bool(false),
            Self::Byte => IecValue::Byte(0),
            Self::Word => IecValue::Word(0),
            Self::DWord => IecValue::DWord(0),
            Self::LWord => IecValue::LWord(0),
            Self::Sint => IecValue::Sint(0),
            Self::Int => IecValue::Int(0),
            Self::Dint => IecValue::Dint(0),
            Self::Lint => IecValue::Lint(0),
            Self::Usint => IecValue::Usint(0),
            Self::Uint => IecValue::Uint(0),
            Self::Udint => IecValue::Udint(0),
            Self::Ulint => IecValue::Ulint(0),
            Self::Real => IecValue::Real(0.0),
            Self::LReal => IecValue::LReal(0.0),
            Self::Time => IecValue::Time(0.0),
            Self::Date => IecValue::Date(0),
            Self::Tod => IecValue::Tod(0.0),
            Self::Dt => IecValue::Dt(0),
            Self::String => IecValue::String(String::new()),
            Self::Array {
                element_type,
                lower_bound,
                upper_bound,
            } => {
                let count = (upper_bound - lower_bound + 1).max(0) as usize;
                let default_elem = element_type.default_value();
                IecValue::Array(vec![default_elem; count])
            }
            Self::Struct { fields, .. } => {
                let mut map = BTreeMap::new();
                for (name, ty) in fields {
                    map.insert(name.clone(), ty.default_value());
                }
                IecValue::Struct(map)
            }
            Self::Enum {
                type_name,
                variants,
            } => {
                let first = variants.first().cloned().unwrap_or_default();
                IecValue::Enum {
                    type_name: type_name.clone(),
                    variant: first,
                    value: 0,
                }
            }
        }
    }
}

impl fmt::Display for IecType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.keyword())
    }
}

/// Strongly typed runtime value for IEC 61131-3 logic evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum IecValue {
    Bool(bool),
    Byte(u8),
    Word(u16),
    DWord(u32),
    LWord(u64),
    Sint(i8),
    Int(i16),
    Dint(i32),
    Lint(i64),
    Usint(u8),
    Uint(u16),
    Udint(u32),
    Ulint(u64),
    Real(f32),
    LReal(f64),
    Time(f64), // Seconds
    Date(i64), // Seconds since epoch
    Tod(f64),  // Seconds from midnight
    Dt(i64),   // Seconds since epoch
    String(String),
    Array(Vec<IecValue>),
    Struct(BTreeMap<String, IecValue>),
    Enum {
        type_name: String,
        variant: String,
        value: i32,
    },
}

impl IecValue {
    /// Returns the concrete `IecType` for this runtime value.
    pub fn get_type(&self) -> IecType {
        match self {
            Self::Bool(_) => IecType::Bool,
            Self::Byte(_) => IecType::Byte,
            Self::Word(_) => IecType::Word,
            Self::DWord(_) => IecType::DWord,
            Self::LWord(_) => IecType::LWord,
            Self::Sint(_) => IecType::Sint,
            Self::Int(_) => IecType::Int,
            Self::Dint(_) => IecType::Dint,
            Self::Lint(_) => IecType::Lint,
            Self::Usint(_) => IecType::Usint,
            Self::Uint(_) => IecType::Uint,
            Self::Udint(_) => IecType::Udint,
            Self::Ulint(_) => IecType::Ulint,
            Self::Real(_) => IecType::Real,
            Self::LReal(_) => IecType::LReal,
            Self::Time(_) => IecType::Time,
            Self::Date(_) => IecType::Date,
            Self::Tod(_) => IecType::Tod,
            Self::Dt(_) => IecType::Dt,
            Self::String(_) => IecType::String,
            Self::Array(elems) => {
                let elem_type = elems
                    .first()
                    .map(|e| Box::new(e.get_type()))
                    .unwrap_or_else(|| Box::new(IecType::Dint));
                IecType::Array {
                    element_type: elem_type,
                    lower_bound: 0,
                    upper_bound: elems.len().saturating_sub(1) as i32,
                }
            }
            Self::Struct(fields) => IecType::Struct {
                type_name: "ANONYMOUS_STRUCT".to_string(),
                fields: fields
                    .iter()
                    .map(|(k, v)| (k.clone(), v.get_type()))
                    .collect(),
            },
            Self::Enum { type_name, .. } => IecType::Enum {
                type_name: type_name.clone(),
                variants: Vec::new(),
            },
        }
    }

    /// Converts to floating-point `f64` representation.
    pub fn as_f64(&self) -> Result<f64, ControlError> {
        match self {
            Self::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
            Self::Byte(v) => Ok(*v as f64),
            Self::Word(v) => Ok(*v as f64),
            Self::DWord(v) => Ok(*v as f64),
            Self::LWord(v) => Ok(*v as f64),
            Self::Sint(v) => Ok(*v as f64),
            Self::Int(v) => Ok(*v as f64),
            Self::Dint(v) => Ok(*v as f64),
            Self::Lint(v) => Ok(*v as f64),
            Self::Usint(v) => Ok(*v as f64),
            Self::Uint(v) => Ok(*v as f64),
            Self::Udint(v) => Ok(*v as f64),
            Self::Ulint(v) => Ok(*v as f64),
            Self::Real(v) => Ok(*v as f64),
            Self::LReal(v) => Ok(*v),
            Self::Time(v) => Ok(*v),
            Self::Date(v) => Ok(*v as f64),
            Self::Tod(v) => Ok(*v),
            Self::Dt(v) => Ok(*v as f64),
            Self::Enum { value, .. } => Ok(*value as f64),
            other => Err(ControlError::InvalidParameter(format!(
                "Cannot convert {:?} to f64",
                other
            ))),
        }
    }

    /// Converts to boolean `bool` representation.
    pub fn as_bool(&self) -> Result<bool, ControlError> {
        match self {
            Self::Bool(b) => Ok(*b),
            Self::Byte(v) => Ok(*v != 0),
            Self::Word(v) => Ok(*v != 0),
            Self::DWord(v) => Ok(*v != 0),
            Self::LWord(v) => Ok(*v != 0),
            Self::Sint(v) => Ok(*v != 0),
            Self::Int(v) => Ok(*v != 0),
            Self::Dint(v) => Ok(*v != 0),
            Self::Lint(v) => Ok(*v != 0),
            Self::Usint(v) => Ok(*v != 0),
            Self::Uint(v) => Ok(*v != 0),
            Self::Udint(v) => Ok(*v != 0),
            Self::Ulint(v) => Ok(*v != 0),
            Self::Real(v) => Ok(*v != 0.0),
            Self::LReal(v) => Ok(*v != 0.0),
            other => Err(ControlError::InvalidParameter(format!(
                "Cannot convert {:?} to bool",
                other
            ))),
        }
    }

    /// Converts to 64-bit integer `i64` representation.
    pub fn as_i64(&self) -> Result<i64, ControlError> {
        match self {
            Self::Bool(b) => Ok(if *b { 1 } else { 0 }),
            Self::Byte(v) => Ok(*v as i64),
            Self::Word(v) => Ok(*v as i64),
            Self::DWord(v) => Ok(*v as i64),
            Self::LWord(v) => Ok(*v as i64),
            Self::Sint(v) => Ok(*v as i64),
            Self::Int(v) => Ok(*v as i64),
            Self::Dint(v) => Ok(*v as i64),
            Self::Lint(v) => Ok(*v),
            Self::Usint(v) => Ok(*v as i64),
            Self::Uint(v) => Ok(*v as i64),
            Self::Udint(v) => Ok(*v as i64),
            Self::Ulint(v) => Ok(*v as i64),
            Self::Real(v) => Ok(*v as i64),
            Self::LReal(v) => Ok(*v as i64),
            Self::Date(v) => Ok(*v),
            Self::Dt(v) => Ok(*v),
            Self::Enum { value, .. } => Ok(*value as i64),
            other => Err(ControlError::InvalidParameter(format!(
                "Cannot convert {:?} to i64",
                other
            ))),
        }
    }

    /// Casts or converts this value to a target `IecType`.
    pub fn cast_to(&self, target: &IecType) -> Result<IecValue, ControlError> {
        match target {
            IecType::Bool => Ok(IecValue::Bool(self.as_bool()?)),
            IecType::Byte => Ok(IecValue::Byte(self.as_i64()? as u8)),
            IecType::Word => Ok(IecValue::Word(self.as_i64()? as u16)),
            IecType::DWord => Ok(IecValue::DWord(self.as_i64()? as u32)),
            IecType::LWord => Ok(IecValue::LWord(self.as_i64()? as u64)),
            IecType::Sint => Ok(IecValue::Sint(self.as_i64()? as i8)),
            IecType::Int => Ok(IecValue::Int(self.as_i64()? as i16)),
            IecType::Dint => Ok(IecValue::Dint(self.as_i64()? as i32)),
            IecType::Lint => Ok(IecValue::Lint(self.as_i64()?)),
            IecType::Usint => Ok(IecValue::Usint(self.as_i64()? as u8)),
            IecType::Uint => Ok(IecValue::Uint(self.as_i64()? as u16)),
            IecType::Udint => Ok(IecValue::Udint(self.as_i64()? as u32)),
            IecType::Ulint => Ok(IecValue::Ulint(self.as_i64()? as u64)),
            IecType::Real => Ok(IecValue::Real(self.as_f64()? as f32)),
            IecType::LReal => Ok(IecValue::LReal(self.as_f64()?)),
            IecType::Time => Ok(IecValue::Time(self.as_f64()?)),
            IecType::Date => Ok(IecValue::Date(self.as_i64()?)),
            IecType::Tod => Ok(IecValue::Tod(self.as_f64()?)),
            IecType::Dt => Ok(IecValue::Dt(self.as_i64()?)),
            IecType::String => Ok(IecValue::String(self.to_string())),
            _ => {
                if self.get_type() == *target {
                    Ok(self.clone())
                } else {
                    Err(ControlError::InvalidParameter(format!(
                        "Unsupported cast from {:?} to {:?}",
                        self.get_type(),
                        target
                    )))
                }
            }
        }
    }

    /// Evaluates binary arithmetic operators (+, -, *, /, MOD, **).
    pub fn binary_op(&self, op: &str, rhs: &IecValue) -> Result<IecValue, ControlError> {
        // Real / LReal takes precedence in float arithmetic
        let is_float = matches!(
            (self, rhs),
            (IecValue::Real(_), _)
                | (_, IecValue::Real(_))
                | (IecValue::LReal(_), _)
                | (_, IecValue::LReal(_))
                | (IecValue::Time(_), _)
                | (_, IecValue::Time(_))
        );

        if is_float {
            let a = self.as_f64()?;
            let b = rhs.as_f64()?;
            let res = match op {
                "+" => a + b,
                "-" => a - b,
                "*" => a * b,
                "/" => {
                    if b.abs() < 1e-15 {
                        return Err(ControlError::InvalidParameter(
                            "Division by zero in IEC expression".to_string(),
                        ));
                    }
                    a / b
                }
                "MOD" => a % b,
                "**" => a.powf(b),
                _ => {
                    return Err(ControlError::InvalidParameter(format!(
                        "Unknown arithmetic operator '{op}'"
                    )))
                }
            };
            if matches!(self, IecValue::Real(_)) && matches!(rhs, IecValue::Real(_)) {
                Ok(IecValue::Real(res as f32))
            } else if matches!(self, IecValue::Time(_)) {
                Ok(IecValue::Time(res))
            } else {
                Ok(IecValue::LReal(res))
            }
        } else {
            let a = self.as_i64()?;
            let b = rhs.as_i64()?;
            let res = match op {
                "+" => a.wrapping_add(b),
                "-" => a.wrapping_sub(b),
                "*" => a.wrapping_mul(b),
                "/" => {
                    if b == 0 {
                        return Err(ControlError::InvalidParameter(
                            "Integer division by zero in IEC expression".to_string(),
                        ));
                    }
                    a.wrapping_div(b)
                }
                "MOD" => {
                    if b == 0 {
                        return Err(ControlError::InvalidParameter(
                            "Modulo by zero in IEC expression".to_string(),
                        ));
                    }
                    a % b
                }
                "**" => {
                    if b < 0 {
                        0
                    } else {
                        a.wrapping_pow(b as u32)
                    }
                }
                _ => {
                    return Err(ControlError::InvalidParameter(format!(
                        "Unknown arithmetic operator '{op}'"
                    )))
                }
            };
            // Preserve integer subtype if homogeneous
            match (self, rhs) {
                (IecValue::Dint(_), IecValue::Dint(_)) => Ok(IecValue::Dint(res as i32)),
                (IecValue::Int(_), IecValue::Int(_)) => Ok(IecValue::Int(res as i16)),
                (IecValue::Sint(_), IecValue::Sint(_)) => Ok(IecValue::Sint(res as i8)),
                (IecValue::Lint(_), _) | (_, IecValue::Lint(_)) => Ok(IecValue::Lint(res)),
                _ => Ok(IecValue::Dint(res as i32)),
            }
        }
    }

    /// Evaluates binary relational operators (=, <>, <, <=, >, >=).
    pub fn compare_op(&self, op: &str, rhs: &IecValue) -> Result<bool, ControlError> {
        let is_float = matches!(
            (self, rhs),
            (IecValue::Real(_), _)
                | (_, IecValue::Real(_))
                | (IecValue::LReal(_), _)
                | (_, IecValue::LReal(_))
                | (IecValue::Time(_), _)
                | (_, IecValue::Time(_))
        );

        if is_float {
            let a = self.as_f64()?;
            let b = rhs.as_f64()?;
            Ok(match op {
                "=" | "==" => (a - b).abs() < 1e-9,
                "<>" | "!=" => (a - b).abs() >= 1e-9,
                "<" => a < b,
                "<=" => a <= b,
                ">" => a > b,
                ">=" => a >= b,
                _ => {
                    return Err(ControlError::InvalidParameter(format!(
                        "Unknown relational operator '{op}'"
                    )))
                }
            })
        } else if let (IecValue::Bool(a), IecValue::Bool(b)) = (self, rhs) {
            Ok(match op {
                "=" | "==" => a == b,
                "<>" | "!=" => a != b,
                _ => {
                    return Err(ControlError::InvalidParameter(format!(
                        "Invalid boolean relational operator '{op}'"
                    )))
                }
            })
        } else if let (IecValue::String(a), IecValue::String(b)) = (self, rhs) {
            Ok(match op {
                "=" | "==" => a == b,
                "<>" | "!=" => a != b,
                "<" => a < b,
                "<=" => a <= b,
                ">" => a > b,
                ">=" => a >= b,
                _ => {
                    return Err(ControlError::InvalidParameter(format!(
                        "Invalid string comparison operator '{op}'"
                    )))
                }
            })
        } else {
            let a = self.as_i64()?;
            let b = rhs.as_i64()?;
            Ok(match op {
                "=" | "==" => a == b,
                "<>" | "!=" => a != b,
                "<" => a < b,
                "<=" => a <= b,
                ">" => a > b,
                ">=" => a >= b,
                _ => {
                    return Err(ControlError::InvalidParameter(format!(
                        "Unknown relational operator '{op}'"
                    )))
                }
            })
        }
    }

    /// Evaluates binary boolean and bitwise operators (AND, OR, XOR).
    pub fn boolean_op(&self, op: &str, rhs: &IecValue) -> Result<IecValue, ControlError> {
        match (self, rhs) {
            (IecValue::Bool(a), IecValue::Bool(b)) => match op {
                "AND" | "&" => Ok(IecValue::Bool(*a && *b)),
                "OR" => Ok(IecValue::Bool(*a || *b)),
                "XOR" => Ok(IecValue::Bool(*a ^ *b)),
                _ => Err(ControlError::InvalidParameter(format!(
                    "Unknown boolean operator '{op}'"
                ))),
            },
            // Bitwise operations on Byte, Word, DWord
            (IecValue::DWord(a), IecValue::DWord(b)) => match op {
                "AND" | "&" => Ok(IecValue::DWord(a & b)),
                "OR" => Ok(IecValue::DWord(a | b)),
                "XOR" => Ok(IecValue::DWord(a ^ b)),
                _ => Err(ControlError::InvalidParameter(format!(
                    "Unknown bitwise operator '{op}'"
                ))),
            },
            (IecValue::Word(a), IecValue::Word(b)) => match op {
                "AND" | "&" => Ok(IecValue::Word(a & b)),
                "OR" => Ok(IecValue::Word(a | b)),
                "XOR" => Ok(IecValue::Word(a ^ b)),
                _ => Err(ControlError::InvalidParameter(format!(
                    "Unknown bitwise operator '{op}'"
                ))),
            },
            (IecValue::Byte(a), IecValue::Byte(b)) => match op {
                "AND" | "&" => Ok(IecValue::Byte(a & b)),
                "OR" => Ok(IecValue::Byte(a | b)),
                "XOR" => Ok(IecValue::Byte(a ^ b)),
                _ => Err(ControlError::InvalidParameter(format!(
                    "Unknown bitwise operator '{op}'"
                ))),
            },
            // Fallback boolean coercion
            (a, b) => {
                let abool = a.as_bool()?;
                let bbool = b.as_bool()?;
                match op {
                    "AND" | "&" => Ok(IecValue::Bool(abool && bbool)),
                    "OR" => Ok(IecValue::Bool(abool || bbool)),
                    "XOR" => Ok(IecValue::Bool(abool ^ bbool)),
                    _ => Err(ControlError::InvalidParameter(format!(
                        "Unknown boolean operator '{op}'"
                    ))),
                }
            }
        }
    }

    /// Evaluates unary operators (NOT, -).
    pub fn unary_op(&self, op: &str) -> Result<IecValue, ControlError> {
        match op {
            "NOT" => Ok(IecValue::Bool(!self.as_bool()?)),
            "-" => match self {
                Self::Real(v) => Ok(Self::Real(-v)),
                Self::LReal(v) => Ok(Self::LReal(-v)),
                Self::Time(v) => Ok(Self::Time(-v)),
                Self::Sint(v) => Ok(Self::Sint(-v)),
                Self::Int(v) => Ok(Self::Int(-v)),
                Self::Dint(v) => Ok(Self::Dint(-v)),
                Self::Lint(v) => Ok(Self::Lint(-v)),
                other => Ok(Self::LReal(-other.as_f64()?)),
            },
            "+" => Ok(self.clone()),
            _ => Err(ControlError::InvalidParameter(format!(
                "Unknown unary operator '{op}'"
            ))),
        }
    }
}

impl fmt::Display for IecValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool(b) => write!(f, "{}", if *b { "TRUE" } else { "FALSE" }),
            Self::Byte(v) => write!(f, "16#{:02X}", v),
            Self::Word(v) => write!(f, "16#{:04X}", v),
            Self::DWord(v) => write!(f, "16#{:08X}", v),
            Self::LWord(v) => write!(f, "16#{:016X}", v),
            Self::Sint(v) => write!(f, "{}", v),
            Self::Int(v) => write!(f, "{}", v),
            Self::Dint(v) => write!(f, "{}", v),
            Self::Lint(v) => write!(f, "{}", v),
            Self::Usint(v) => write!(f, "{}", v),
            Self::Uint(v) => write!(f, "{}", v),
            Self::Udint(v) => write!(f, "{}", v),
            Self::Ulint(v) => write!(f, "{}", v),
            Self::Real(v) => write!(f, "{:.4}", v),
            Self::LReal(v) => write!(f, "{:.6}", v),
            Self::Time(v) => {
                if *v < 1.0 {
                    write!(f, "T#{}ms", (v * 1000.0).round() as i64)
                } else {
                    write!(f, "T#{:.3}s", v)
                }
            }
            Self::Date(v) => write!(f, "D#{}", v),
            Self::Tod(v) => write!(f, "TOD#{:.3}s", v),
            Self::Dt(v) => write!(f, "DT#{}", v),
            Self::String(s) => write!(f, "'{}'", s),
            Self::Array(elems) => {
                write!(f, "[")?;
                for (i, el) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", el)?;
                }
                write!(f, "]")
            }
            Self::Struct(fields) => {
                write!(f, "{{")?;
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            Self::Enum { variant, .. } => write!(f, "{}", variant),
        }
    }
}
