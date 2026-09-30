//! IEC 61131-3 Control Logic Runtime for NebuDCS.
//!
//! Provides vendor-neutral compilation and execution for:
//! - Structured Text (ST)
//! - Ladder Diagram (LD)
//! - Function Block Diagram (FBD)
//! - Sequential Function Chart (SFC)
//!
//! All languages compile into a common, typed, deterministic Intermediate Representation (IR)
//! executed by a single-threaded virtual machine (`IecVm`) integrated directly with
//! the Stage 1 Controller Task Scheduler and Stage 3 I/O HAL Image Tables.

pub mod fb;
pub mod fbd;
pub mod ir;
pub mod ladder;
pub mod program;
pub mod routine;
pub mod sfc;
pub mod st;
pub mod storage;
pub mod types;
pub mod vm;

pub use fb::{
    AntiSurgeBlock, CtdBlock, CtuBlock, FTrigBlock, FilterFirstOrderBlock, FunctionBlockInstance,
    HighSelectorBlock, LeadLagBlock, LowSelectorBlock, PidIecBlock, RTrigBlock, RatioBlock,
    RsBlock, SplitRangeBlock, SrBlock, TofBlock, TonBlock, TpBlock, ValveCharBlock,
};
pub use fbd::{compile_fbd, FbdConnection, FbdGraph, FbdNode, FbdNodeKind};
pub use ir::{IecExpr, IecStatement};
pub use ladder::{
    compile_ladder, LdCoil, LdCoilKind, LdContact, LdContactKind, LdDiagram, LdElement,
    LdParallelBranch, LdRung,
};
pub use program::IecProgram;
pub use routine::{IecLanguage, IecRoutine};
pub use sfc::{compile_sfc, SfcAction, SfcActionQualifier, SfcChart, SfcStep, SfcTransition};
pub use st::compile_st;
pub use storage::{MemoryStore, SymbolTable, VarScope, VariableDeclaration};
pub use types::{IecType, IecValue};
pub use vm::{FlowControl, IecVm, MAX_LOOP_ITERATIONS};
