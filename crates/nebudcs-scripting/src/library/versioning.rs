//! Model Versioning and Provenance Governance.
//!
//! Enforces immutable versioning, semantic compatibility, and dependency locking
//! to guarantee that existing projects and simulations remain 100% reproducible.

use nebudcs_types::SchemaVersion;
use serde::{Deserialize, Serialize};

use crate::error::{ScriptingError, ScriptingResult};

/// Lifecycle state of a versioned template or package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum VersionLifecycle {
    /// In active development; subject to breaking changes.
    Draft,
    /// Locked, immutable, published for production modeling and training.
    #[default]
    Stable,
    /// Superseded by a newer version; operational in existing units but warned against in new units.
    Deprecated,
    /// End of support; retained only for legacy historical replay.
    Archived,
}

/// Composite key uniquely identifying a specific immutable version.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct VersionedKey {
    pub id: String,
    pub version: SchemaVersion,
}

impl VersionedKey {
    pub fn new(id: impl Into<String>, version: SchemaVersion) -> Self {
        Self {
            id: id.into(),
            version,
        }
    }
}

impl std::fmt::Display for VersionedKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.id, self.version)
    }
}

/// Provenance record tracking the locked version and checksum of an instantiated template.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelProvenance {
    pub model_id: String,
    pub version: SchemaVersion,
    pub content_hash: String,
    pub instantiated_at: String,
}

/// Calculates a deterministic content checksum string from serialized content.
pub fn compute_content_fingerprint(content: &str) -> String {
    // FNV-1a 64-bit deterministic hash formatted as hex string
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in content.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

/// Verifies immutability: once published, a version cannot be overwritten with different content.
pub fn verify_immutability(
    id: &str,
    version: &SchemaVersion,
    existing_fingerprint: &str,
    new_fingerprint: &str,
) -> ScriptingResult<()> {
    if existing_fingerprint != new_fingerprint {
        return Err(ScriptingError::Version(format!(
            "Immutable version violation for '{}@{}': definition has already been published and cannot be modified",
            id, version
        )));
    }
    Ok(())
}
