//! Benchmark measurement types.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Whether higher or lower values are better for a metric.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricDirection {
    HigherIsBetter,
    LowerIsBetter,
}

/// Stable benchmark identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BenchmarkId(pub String);

impl BenchmarkId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for BenchmarkId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// One timed / scored measurement from a single sample.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Measurement {
    pub id: BenchmarkId,
    pub category: String,
    /// Primary score used for indexing (already oriented: higher is better).
    pub score: f64,
    /// Raw observed value (e.g. seconds, bytes/s) before orientation.
    pub raw_value: f64,
    pub unit: String,
    pub direction: MetricDirection,
    /// Optional correctness checksum / token proving the work was real.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    #[serde(default)]
    pub notes: Vec<String>,
}

impl Measurement {
    /// Convert raw value into a higher-is-better score.
    pub fn oriented_score(raw: f64, direction: MetricDirection) -> f64 {
        match direction {
            MetricDirection::HigherIsBetter => raw,
            MetricDirection::LowerIsBetter => {
                if raw <= 0.0 {
                    0.0
                } else {
                    1.0 / raw
                }
            }
        }
    }
}

/// Collection of measurements from one suite run (one sample of the suite).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MeasurementSet {
    pub measurements: Vec<Measurement>,
    #[serde(default)]
    pub meta: IndexMap<String, String>,
}

impl MeasurementSet {
    pub fn push(&mut self, m: Measurement) {
        self.measurements.push(m);
    }

    pub fn get(&self, id: &str) -> Option<&Measurement> {
        self.measurements.iter().find(|m| m.id.as_str() == id)
    }

    pub fn scores_by_id(&self) -> IndexMap<String, f64> {
        self.measurements
            .iter()
            .map(|m| (m.id.0.clone(), m.score))
            .collect()
    }
}
