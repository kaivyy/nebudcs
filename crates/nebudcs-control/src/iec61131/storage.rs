//! IEC 61131-3 Variable Scoping, Declarations, and Memory Storage.
//!
//! Provides structured variable management across controller, program,
//! function block, and local execution boundaries.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::error::ControlError;
use crate::iec61131::types::{IecType, IecValue};

/// Scoping classification of an IEC variable declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VarScope {
    /// Local program/routine variable (`VAR ... END_VAR`).
    Var,
    /// Formal input parameter (`VAR_INPUT ... END_VAR`).
    VarInput,
    /// Formal output parameter (`VAR_OUTPUT ... END_VAR`).
    VarOutput,
    /// In-out pass-by-reference parameter (`VAR_IN_OUT ... END_VAR`).
    VarInOut,
    /// Global controller-scoped variable (`VAR_GLOBAL ... END_VAR`).
    VarGlobal,
    /// Temporary stack variable (`VAR_TEMP ... END_VAR`).
    VarTemp,
    /// Read-only constant value (`VAR_CONSTANT ... END_VAR`).
    VarConstant,
}

/// Variable declaration metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariableDeclaration {
    /// Unique variable identifier within its scope.
    pub name: String,
    /// Declared data type.
    pub data_type: IecType,
    /// Initial value assigned on startup or reset.
    pub initial_value: IecValue,
    /// Scope classification.
    pub scope: VarScope,
    /// Optional physical I/O address binding (e.g. "%IX0.0", "%QW100").
    pub address_binding: Option<String>,
    /// Optional engineering unit or descriptive documentation.
    pub description: Option<String>,
}

impl VariableDeclaration {
    /// Creates a new local variable declaration.
    pub fn new(name: impl Into<String>, data_type: IecType) -> Self {
        let initial_value = data_type.default_value();
        Self {
            name: name.into(),
            data_type,
            initial_value,
            scope: VarScope::Var,
            address_binding: None,
            description: None,
        }
    }

    /// Sets the scope classification.
    pub fn with_scope(mut self, scope: VarScope) -> Self {
        self.scope = scope;
        self
    }

    /// Sets a custom initial value.
    pub fn with_initial_value(mut self, val: IecValue) -> Self {
        self.initial_value = val;
        self
    }

    /// Sets an I/O memory address binding (e.g. "%IW0", "%QW0").
    pub fn with_address(mut self, addr: impl Into<String>) -> Self {
        self.address_binding = Some(addr.into());
        self
    }
}

/// Symbol table tracking all declared variables and function block instances.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SymbolTable {
    symbols: BTreeMap<String, VariableDeclaration>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Declares a variable in the symbol table.
    pub fn declare(&mut self, decl: VariableDeclaration) -> Result<(), ControlError> {
        let name = decl.name.to_ascii_uppercase();
        if self.symbols.contains_key(&name) {
            return Err(ControlError::InvalidParameter(format!(
                "Duplicate variable declaration '{}'",
                decl.name
            )));
        }
        self.symbols.insert(name, decl);
        Ok(())
    }

    /// Looks up a variable declaration by name (case-insensitive per IEC standard).
    pub fn lookup(&self, name: &str) -> Option<&VariableDeclaration> {
        self.symbols.get(&name.to_ascii_uppercase())
    }

    /// Access all declarations immutably.
    pub fn declarations(&self) -> &BTreeMap<String, VariableDeclaration> {
        &self.symbols
    }
}

/// Live runtime variable memory store for a program or function block instance.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MemoryStore {
    values: BTreeMap<String, IecValue>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Initializes storage from a symbol table's initial values.
    pub fn from_symbol_table(symbols: &SymbolTable) -> Self {
        let mut store = Self::new();
        for (name, decl) in symbols.declarations() {
            store
                .values
                .insert(name.clone(), decl.initial_value.clone());
        }
        store
    }

    /// Gets a variable value by name (case-insensitive).
    pub fn get(&self, name: &str) -> Option<&IecValue> {
        self.values.get(&name.to_ascii_uppercase())
    }

    /// Sets a variable value by name (case-insensitive).
    pub fn set(&mut self, name: impl Into<String>, value: IecValue) {
        self.values.insert(name.into().to_ascii_uppercase(), value);
    }

    /// Clears and resets all variables.
    pub fn clear(&mut self) {
        self.values.clear();
    }

    /// Returns a reference to all stored variables.
    pub fn all_values(&self) -> &BTreeMap<String, IecValue> {
        &self.values
    }
}
