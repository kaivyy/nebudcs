//! IEC 61131-3 Program Execution Unit.
//!
//! Implements `ControllerProgram`, enabling multi-routine, multi-language execution
//! within Stage 1 Controller Tasks, Stage 3 I/O HAL integration, and shared memory.

use std::collections::BTreeMap;
use std::fmt::Debug;

use crate::error::ControlError;
use crate::iec61131::fb::FunctionBlockInstance;
use crate::iec61131::routine::IecRoutine;
use crate::iec61131::storage::{MemoryStore, SymbolTable, VariableDeclaration};
use crate::iec61131::types::IecValue;
use crate::iec61131::vm::IecVm;
use crate::runtime::io_image::IoImageTable;
use crate::runtime::program::ControllerProgram;

/// An IEC 61131-3 program running within a controller task.
#[derive(Debug)]
pub struct IecProgram {
    name: String,
    routines: Vec<IecRoutine>,
    symbols: SymbolTable,
    memory: MemoryStore,
    fb_instances: BTreeMap<String, Box<dyn FunctionBlockInstance>>,
}

impl Clone for IecProgram {
    fn clone(&self) -> Self {
        let mut fb_clone = BTreeMap::new();
        for (k, v) in &self.fb_instances {
            fb_clone.insert(k.clone(), v.clone_box());
        }

        Self {
            name: self.name.clone(),
            routines: self.routines.clone(),
            symbols: self.symbols.clone(),
            memory: self.memory.clone(),
            fb_instances: fb_clone,
        }
    }
}

impl IecProgram {
    /// Creates a new empty IEC 61131-3 program.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            routines: Vec::new(),
            symbols: SymbolTable::new(),
            memory: MemoryStore::new(),
            fb_instances: BTreeMap::new(),
        }
    }

    /// Declares a program-scoped variable.
    pub fn declare_variable(&mut self, decl: VariableDeclaration) -> Result<(), ControlError> {
        let initial_val = decl.initial_value.clone();
        let name = decl.name.clone();
        self.symbols.declare(decl)?;
        self.memory.set(name, initial_val);
        Ok(())
    }

    /// Adds a compiled routine to the program.
    pub fn add_routine(&mut self, routine: IecRoutine) -> Result<(), ControlError> {
        // Register routine symbols into program memory
        for (name, decl) in routine.symbols.declarations() {
            if self.symbols.lookup(name).is_none() {
                let _ = self.symbols.declare(decl.clone());
                self.memory.set(name.clone(), decl.initial_value.clone());
            }
        }
        self.routines.push(routine);
        Ok(())
    }

    /// Returns a reference to all routines in this program.
    pub fn routines(&self) -> &[IecRoutine] {
        &self.routines
    }

    /// Gets a variable value from the program's live memory store.
    pub fn get_variable(&self, name: &str) -> Option<&IecValue> {
        self.memory.get(name)
    }

    /// Sets a variable value in the program's live memory store.
    pub fn set_variable(&mut self, name: impl Into<String>, val: IecValue) {
        self.memory.set(name, val);
    }

    /// Registers or replaces a function block instance.
    pub fn register_fb(&mut self, fb: Box<dyn FunctionBlockInstance>) {
        self.fb_instances.insert(fb.name().to_string(), fb);
    }

    /// Returns the live memory store.
    pub fn memory(&self) -> &MemoryStore {
        &self.memory
    }
}

impl ControllerProgram for IecProgram {
    fn name(&self) -> &str {
        &self.name
    }

    fn execute(
        &mut self,
        dt: f64,
        sim_time: f64,
        io_image: &mut IoImageTable,
    ) -> Result<(), ControlError> {
        for routine in &self.routines {
            if !routine.enabled {
                continue;
            }
            let mut vm = IecVm::new(
                dt,
                sim_time,
                &mut self.memory,
                io_image,
                &mut self.fb_instances,
            );
            vm.execute_block(&routine.statements)?;
        }
        Ok(())
    }

    fn reset(&mut self) {
        self.memory = MemoryStore::from_symbol_table(&self.symbols);
        for routine in &self.routines {
            for (name, decl) in routine.symbols.declarations() {
                self.memory.set(name.clone(), decl.initial_value.clone());
            }
        }
        for fb in self.fb_instances.values_mut() {
            fb.reset();
        }
    }

    fn clone_box(&self) -> Box<dyn ControllerProgram> {
        Box::new(self.clone())
    }
}
