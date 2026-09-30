//! Reference dataset schemas and comparison representations.
//!
//! Supports:
//! - Scalar reference values with published uncertainty
//! - Tabular (X, Y) curves (e.g. pump head vs flow, compressor maps)
//! - Spatial profiles across axial coordinates $z$
//! - Multi-component phase-equilibrium (VLE) tie-line points
//! - Time-series responses

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Type of reference dataset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "dataset_type", content = "data")]
pub enum ReferenceDataset {
    /// Single scalar measurement point.
    Scalar(ScalarReference),
    /// Discrete characteristic curve (X, Y) pairs.
    Curve(CurveReference),
    /// Axial or spatial profile along 1D dimension.
    SpatialProfile(SpatialProfileReference),
    /// Phase-equilibrium vapor-liquid equilibrium (VLE) tie-line.
    Vle(VleReference),
    /// Dynamic time-series trajectory.
    TimeSeries(TimeSeriesReference),
}

/// Single scalar property benchmark point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScalarReference {
    pub property_name: String,
    pub unit: String,
    pub value: f64,
    pub published_uncertainty: Option<f64>,
    pub significant_figures: Option<u8>,
    pub source_conditions: String,
}

/// 2D Curve dataset (e.g. Pump H-Q curve, Compressor map).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurveReference {
    pub x_name: String,
    pub x_unit: String,
    pub y_name: String,
    pub y_unit: String,
    pub points: Vec<(f64, f64)>,
    pub is_interpolated: bool,
}

/// 1D spatial profile (e.g. PFR reactor concentration profile).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpatialProfileReference {
    pub coordinate_name: String,
    pub coordinate_unit: String,
    pub value_name: String,
    pub value_unit: String,
    pub z_coordinates: Vec<f64>,
    pub values: Vec<f64>,
}

/// Vapor-Liquid Equilibrium reference tie-line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VleReference {
    pub temperature_k: f64,
    pub pressure_pa: f64,
    pub liquid_mole_fractions: IndexMap<String, f64>,
    pub vapor_mole_fractions: IndexMap<String, f64>,
    pub k_values: IndexMap<String, f64>,
}

/// Time-series trajectory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeSeriesReference {
    pub time_unit: String,
    pub value_unit: String,
    pub time_steps: Vec<f64>,
    pub values: Vec<f64>,
    pub sampling_interval_s: f64,
}
