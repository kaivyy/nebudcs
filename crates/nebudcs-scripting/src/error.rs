//! Error types for the NebuDCS custom equipment and scripting subsystem.

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ScriptingError {
    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Curve error: {0}")]
    Curve(String),

    #[error("Script compilation error: {0}")]
    Compilation(String),

    #[error("Script execution error: {0}")]
    Execution(String),

    #[error("Instruction quota exceeded: {0}")]
    InstructionQuotaExceeded(String),

    #[error("Script timeout: {0}")]
    Timeout(String),

    #[error("Memory or recursion limit exceeded: {0}")]
    ResourceLimit(String),

    #[error("Equipment definition not found: {0}")]
    NotFound(String),

    #[error("Equipment instance error: {0}")]
    Instance(String),

    #[error("Security violation: {0}")]
    Security(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Template error: {0}")]
    Template(String),

    #[error("Package error: {0}")]
    Package(String),

    #[error("Dependency resolution failed: {0}")]
    Dependency(String),

    #[error("Versioning error: {0}")]
    Version(String),

    #[error("Import/Export error: {0}")]
    ImportExport(String),
}

pub type ScriptingResult<T> = Result<T, ScriptingError>;
