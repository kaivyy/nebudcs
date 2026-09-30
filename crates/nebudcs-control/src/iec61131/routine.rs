//! IEC 61131-3 Routine Abstraction.
//!
//! Encapsulates a single logic routine compiled from ST, LD, FBD, or SFC.

use serde::{Deserialize, Serialize};

use crate::error::ControlError;
use crate::iec61131::fbd::{compile_fbd, FbdGraph};
use crate::iec61131::ir::IecStatement;
use crate::iec61131::ladder::{compile_ladder, LdDiagram};
use crate::iec61131::sfc::{compile_sfc, SfcChart};
use crate::iec61131::st::compile_st;
use crate::iec61131::storage::SymbolTable;

/// Language of an IEC 61131-3 routine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IecLanguage {
    /// Structured Text (textual high-level logic)
    St,
    /// Ladder Diagram (graphical relay ladder logic)
    Ld,
    /// Function Block Diagram (graphical signal-flow network)
    Fbd,
    /// Sequential Function Chart (graphical sequence state machine)
    Sfc,
}

/// A compiled IEC 61131-3 routine executable within an `IecProgram`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IecRoutine {
    pub name: String,
    pub language: IecLanguage,
    pub source_text: String,
    pub symbols: SymbolTable,
    pub statements: Vec<IecStatement>,
    pub enabled: bool,
}

impl IecRoutine {
    /// Compiles an ST routine from textual source code.
    pub fn from_st(
        name: impl Into<String>,
        source: impl Into<String>,
    ) -> Result<Self, ControlError> {
        let name = name.into();
        let source_text = source.into();
        let (symbols, statements) = compile_st(&source_text)?;
        Ok(Self {
            name,
            language: IecLanguage::St,
            source_text,
            symbols,
            statements,
            enabled: true,
        })
    }

    /// Compiles an LD routine from a Ladder Diagram model.
    pub fn from_ld(name: impl Into<String>, diagram: &LdDiagram) -> Result<Self, ControlError> {
        let name = name.into();
        let source_text = serde_json::to_string_pretty(diagram).map_err(|e| {
            ControlError::InvalidParameter(format!("Failed to serialize LD diagram: {e}"))
        })?;
        let (symbols, statements) = compile_ladder(diagram)?;
        Ok(Self {
            name,
            language: IecLanguage::Ld,
            source_text,
            symbols,
            statements,
            enabled: true,
        })
    }

    /// Compiles an FBD routine from a Function Block Diagram graph.
    pub fn from_fbd(name: impl Into<String>, graph: &FbdGraph) -> Result<Self, ControlError> {
        let name = name.into();
        let source_text = serde_json::to_string_pretty(graph).map_err(|e| {
            ControlError::InvalidParameter(format!("Failed to serialize FBD graph: {e}"))
        })?;
        let (symbols, statements) = compile_fbd(graph)?;
        Ok(Self {
            name,
            language: IecLanguage::Fbd,
            source_text,
            symbols,
            statements,
            enabled: true,
        })
    }

    /// Compiles an SFC routine from a Sequential Function Chart.
    pub fn from_sfc(name: impl Into<String>, chart: &SfcChart) -> Result<Self, ControlError> {
        let name = name.into();
        let source_text = serde_json::to_string_pretty(chart).map_err(|e| {
            ControlError::InvalidParameter(format!("Failed to serialize SFC chart: {e}"))
        })?;
        let (symbols, statements) = compile_sfc(chart)?;
        Ok(Self {
            name,
            language: IecLanguage::Sfc,
            source_text,
            symbols,
            statements,
            enabled: true,
        })
    }
}
