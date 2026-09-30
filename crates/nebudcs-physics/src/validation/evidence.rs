//! Scientific evidence governance, validation classification, and provenance models.
//!
//! Enforces:
//! - Complete traceability for reference datasets (no unprovenanced benchmarks)
//! - Explicit validation state machine (A model must never appear "validated" merely because tests pass)
//! - Provenance for tolerance thresholds (SourceDefined, EngineeringDefined, Numerical, ProjectDefined)
//! - Machine-readable citation and reproducibility metadata

use serde::{Deserialize, Serialize};

/// Classification of scientific evidence backing a benchmark or validation case.
///
/// Synthetic tests MUST NEVER be represented as experimental validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvidenceClassification {
    /// Direct experimental laboratory or plant measurements published in peer-reviewed literature.
    PeerReviewedExperimental,
    /// Established benchmark datasets widely accepted across scientific or industrial literature.
    PublishedBenchmark,
    /// Publicly accessible, curated scientific data repository (e.g. NIST Chemistry WebBook).
    PublicDataset,
    /// Standard engineering reference handbooks (e.g. Perry's, GPSA Engineering Data Book).
    EngineeringReference,
    /// Official engineering standards body specification (e.g. API 610/617, IAPWS-IF97, TEMA).
    StandardReference,
    /// Certified equipment manufacturer performance curves and test records.
    ManufacturerData,
    /// Exact closed-form mathematical or analytical solution to physical balance equations.
    AnalyticalReference,
    /// Internal first-principles thermodynamic and conservation consistency check.
    EngineeringConsistency,
    /// Software integration or regression test with synthetic inputs.
    SyntheticTest,
}

impl std::fmt::Display for EvidenceClassification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PeerReviewedExperimental => write!(f, "PEER_REVIEWED_EXPERIMENTAL"),
            Self::PublishedBenchmark => write!(f, "PUBLISHED_BENCHMARK"),
            Self::PublicDataset => write!(f, "PUBLIC_DATASET"),
            Self::EngineeringReference => write!(f, "ENGINEERING_REFERENCE"),
            Self::StandardReference => write!(f, "STANDARD_REFERENCE"),
            Self::ManufacturerData => write!(f, "MANUFACTURER_DATA"),
            Self::AnalyticalReference => write!(f, "ANALYTICAL_REFERENCE"),
            Self::EngineeringConsistency => write!(f, "ENGINEERING_CONSISTENCY"),
            Self::SyntheticTest => write!(f, "SYNTHETIC_TEST"),
        }
    }
}

/// Explicit scientific validation lifecycle state.
///
/// A model must never appear "validated" merely because unit tests pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidationStatus {
    /// Model has not yet undergone comparison against reference data.
    NotValidated,
    /// First-principles conservation and qualitative trends verified; empirical reference pending.
    EngineeringConsistent,
    /// Quantitatively compared against published benchmark data within specified error margins.
    Benchmarked,
    /// Fully verified against peer-reviewed experimental or standard reference within validity range.
    Validated,
    /// Validated within a restricted sub-domain or with documented engineering limitations.
    ValidatedWithLimitations,
    /// Model operating conditions lie outside calibrated reference envelope (extrapolated).
    Extrapolated,
    /// Model fails acceptance tolerance or violates physical conservation laws.
    Invalid,
    /// No defensible public or literature reference dataset exists; marked honestly.
    InsufficientData,
}

impl std::fmt::Display for ValidationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotValidated => write!(f, "NOT_VALIDATED"),
            Self::EngineeringConsistent => write!(f, "ENGINEERING_CONSISTENCY_VERIFIED"),
            Self::Benchmarked => write!(f, "BENCHMARKED"),
            Self::Validated => write!(f, "INTERNALLY_VALIDATED"),
            Self::ValidatedWithLimitations => write!(f, "VALIDATED_WITH_LIMITATIONS"),
            Self::Extrapolated => write!(f, "EXTRAPOLATED"),
            Self::Invalid => write!(f, "INVALID"),
            Self::InsufficientData => write!(f, "INSUFFICIENT_DATA"),
        }
    }
}

/// Provenance and origin of an acceptance tolerance threshold.
///
/// PROJECT_DEFINED thresholds must not be presented as scientific truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ToleranceThresholdProvenance {
    /// Tolerance stated directly by the original publication author or experimental study.
    SourceDefined,
    /// Standard industry engineering margin (e.g. API 610 head tolerance, TEMA fouling margin).
    EngineeringDefined,
    /// Numerical convergence limit determined by machine precision or ODE/DAE solver tolerance.
    Numerical,
    /// Project-specific engineering tolerance. Must not be presented as scientific ground truth.
    ProjectDefined,
}

/// Full bibliographic citation and provenance record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScientificCitation {
    pub title: String,
    pub authors_or_organization: String,
    pub publication_year: u32,
    pub doi_or_identifier: Option<String>,
    pub standard_id: Option<String>,
    pub url: Option<String>,
}

impl ScientificCitation {
    pub fn new(
        title: impl Into<String>,
        authors: impl Into<String>,
        year: u32,
        doi: Option<&str>,
        standard: Option<&str>,
    ) -> Self {
        Self {
            title: title.into(),
            authors_or_organization: authors.into(),
            publication_year: year,
            doi_or_identifier: doi.map(|s| s.to_string()),
            standard_id: standard.map(|s| s.to_string()),
            url: doi.map(|d| format!("https://doi.org/{d}")),
        }
    }
}

/// Reproducibility metadata for a validation run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReproducibilityMetadata {
    pub software_version: String,
    pub model_version: String,
    pub solver_tolerance: f64,
    pub random_seed: Option<u64>,
    pub platform_arch: String,
    pub execution_timestamp: String,
    pub deterministic_replay_id: String,
}

impl Default for ReproducibilityMetadata {
    fn default() -> Self {
        Self {
            software_version: "0.1.0".to_string(),
            model_version: "1.0.0".to_string(),
            solver_tolerance: 1.0e-6,
            random_seed: Some(42),
            platform_arch: std::env::consts::ARCH.to_string(),
            execution_timestamp: "2026-09-08T00:00:00Z".to_string(),
            deterministic_replay_id: "rep-det-stg13-verified".to_string(),
        }
    }
}
