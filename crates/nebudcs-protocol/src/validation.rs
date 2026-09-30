//! Validation, Benchmark & Scientific Certification Protocol DTOs.
//!
//! Provides typed contracts for validation cases, reference datasets, benchmark runs,
//! and executive certification manifests.
//!
//! CERTIFICATION LANGUAGE POLICY:
//! Permitted terms: "INTERNALLY VALIDATED", "BENCHMARKED", "ENGINEERING CONSISTENCY VERIFIED", "VALIDATED WITH LIMITATIONS".
//! Forbidden terms: "IEC CERTIFIED", "API CERTIFIED", "ISA CERTIFIED", "SIL CERTIFIED", "INDUSTRY CERTIFIED".

use serde::{Deserialize, Serialize};

/// Evidence classification category per ASME V&V 10 / 20 and scientific practice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvidenceClassificationDto {
    PeerReviewedExperimental,
    PublishedBenchmark,
    PublicDataset,
    EngineeringReference,
    StandardReference,
    ManufacturerData,
    AnalyticalReference,
    EngineeringConsistency,
    SyntheticTest,
}

impl EvidenceClassificationDto {
    pub fn display_label(&self) -> &'static str {
        match self {
            Self::PeerReviewedExperimental => "Peer-Reviewed Experimental Measurement",
            Self::PublishedBenchmark => "Published Benchmark Dataset",
            Self::PublicDataset => "Curated Public Repository (NIST/GPA)",
            Self::EngineeringReference => "Engineering Reference Handbook (Perry/Crane)",
            Self::StandardReference => "Authoritative Standard Reference (API/ISA/IAPWS)",
            Self::ManufacturerData => "Manufacturer Performance Data",
            Self::AnalyticalReference => "Exact Analytical Closed-Form Solution",
            Self::EngineeringConsistency => "First-Principles Conservation Law Invariant",
            Self::SyntheticTest => "Synthetic Numerical Invariant Test",
        }
    }
}

/// Validation status adhering to scientific qualification standards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidationStatusDto {
    NotValidated,
    EngineeringConsistent,
    Benchmarked,
    Validated,
    ValidatedWithLimitations,
    Extrapolated,
    Invalid,
    InsufficientData,
}

impl ValidationStatusDto {
    pub fn is_passing(&self) -> bool {
        matches!(
            self,
            Self::Validated
                | Self::Benchmarked
                | Self::EngineeringConsistent
                | Self::ValidatedWithLimitations
        )
    }

    pub fn status_text(&self) -> &'static str {
        match self {
            Self::NotValidated => "NOT VALIDATED",
            Self::EngineeringConsistent => "ENGINEERING CONSISTENCY VERIFIED",
            Self::Benchmarked => "BENCHMARKED",
            Self::Validated => "INTERNALLY VALIDATED",
            Self::ValidatedWithLimitations => "VALIDATED WITH LIMITATIONS",
            Self::Extrapolated => "EXTRAPOLATED",
            Self::Invalid => "FAILED / INVALID",
            Self::InsufficientData => "INSUFFICIENT DATA",
        }
    }
}

/// Provenance of the numerical tolerance threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ToleranceThresholdProvenanceDto {
    SourceDefined,
    EngineeringDefined,
    Numerical,
    ProjectDefined,
}

impl ToleranceThresholdProvenanceDto {
    pub fn display_label(&self) -> &'static str {
        match self {
            Self::SourceDefined => "Author / Source Defined",
            Self::EngineeringDefined => "Standard Engineering Margin (API/TEMA)",
            Self::Numerical => "Solver Machine Precision / Numerical Bound",
            Self::ProjectDefined => "Project-Defined Baseline",
        }
    }
}

/// Formal academic or standard citation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScientificCitationDto {
    pub title: String,
    pub authors_or_organization: String,
    pub publication_year: u32,
    pub doi_or_identifier: Option<String>,
    pub standard_id: Option<String>,
    pub url: Option<String>,
}

/// Operating validity envelope defining model domain boundaries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicabilityEnvelopeDto {
    pub min_temperature_k: f64,
    pub max_temperature_k: f64,
    pub min_pressure_pa: f64,
    pub max_pressure_pa: f64,
    pub supported_phases: Vec<String>,
    pub supported_fluids: Vec<String>,
    pub limitation_notes: Option<String>,
}

/// Statistical error and deviation metrics for a benchmark case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorMetricsDto {
    pub mean_absolute_error: f64,
    pub root_mean_squared_error: f64,
    pub max_absolute_deviation: f64,
    pub max_relative_error_pct: f64,
    pub mean_bias_error: f64,
    pub conservation_residual: f64,
}

/// Summary representation of an individual validation case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationCaseSummaryDto {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub tested_feature: String,
    pub classification: EvidenceClassificationDto,
    pub status: ValidationStatusDto,
    pub standard_id: Option<String>,
    pub passed: bool,
    pub max_relative_error_pct: f64,
    pub relative_tolerance_pct: f64,
    pub last_executed_iso: String,
}

/// Detailed specification and benchmark results of a validation case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationCaseDetailDto {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub tested_feature: String,
    pub model_name: String,
    pub classification: EvidenceClassificationDto,
    pub status: ValidationStatusDto,
    pub tolerance_provenance: ToleranceThresholdProvenanceDto,
    pub citation: ScientificCitationDto,
    pub applicability_envelope: ApplicabilityEnvelopeDto,
    pub relative_tolerance_pct: f64,
    pub absolute_tolerance: f64,
    pub metrics: ErrorMetricsDto,
    pub passed: bool,
    pub last_executed_iso: String,
    pub execution_duration_ms: f64,
    pub notes: String,
    pub disclaimer: String,
}

/// Metadata and summary for an authoritative reference dataset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferenceDatasetDto {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub source_citation: String,
    pub standard_organization: Option<String>,
    pub point_count: usize,
    pub parameters: Vec<String>,
}

/// Validation summary for a specific physics or control model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelValidationSummaryDto {
    pub model_name: String,
    pub domain: String,
    pub status: ValidationStatusDto,
    pub cases_count: usize,
    pub cases_passed: usize,
    pub worst_case_relative_error_pct: f64,
    pub validity_summary: String,
}

/// High-level executive certification dashboard metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationExecutiveSummaryDto {
    pub total_cases: usize,
    pub passed_cases: usize,
    pub failed_cases: usize,
    pub pass_rate_pct: f64,
    pub overall_status: String,
    pub domain_counts: Vec<(String, usize, usize)>, // (domain, total, passed)
    pub classification_counts: Vec<(String, usize)>,
    pub timestamp_iso: String,
    pub engine_version: String,
    pub compliance_disclaimer: String,
}

/// Request to run validation benchmarks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationRunRequestDto {
    pub domain_filter: Option<String>,
    pub case_ids: Option<Vec<String>>,
}

/// Response returned after running validation benchmarks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationRunResponseDto {
    pub executed_cases: usize,
    pub passed_cases: usize,
    pub failed_cases: usize,
    pub total_duration_ms: f64,
    pub cases: Vec<ValidationCaseSummaryDto>,
    pub summary: ValidationExecutiveSummaryDto,
}

/// Complete exported validation manifest JSON artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationManifestDto {
    pub version: String,
    pub generated_at_iso: String,
    pub system_id: String,
    pub summary: ValidationExecutiveSummaryDto,
    pub cases: Vec<ValidationCaseDetailDto>,
    pub models: Vec<ModelValidationSummaryDto>,
    pub references: Vec<ReferenceDatasetDto>,
    pub disclaimers: Vec<String>,
}
