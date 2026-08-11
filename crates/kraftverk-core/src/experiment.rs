//! Experiment records.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::candidate::Candidate;
use crate::classification::ComparisonClass;
use crate::kraft_index::KraftIndex;
use crate::measurement::MeasurementSet;
use crate::statistics::{ComparisonResult, SampleSummary};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExperimentId(pub Uuid);

impl ExperimentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn parse(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl Default for ExperimentId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ExperimentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentKind {
    Baseline,
    Candidate,
    Validation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentStatus {
    Pending,
    Running,
    Applied,
    Measuring,
    Completed,
    RolledBack,
    Failed,
    Interrupted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Accept,
    Reject,
    Baseline,
    Pending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StabilityVerdict {
    Pass,
    Fail,
    Unknown,
}

impl StabilityVerdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Fail => "FAIL",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experiment {
    pub id: ExperimentId,
    pub kind: ExperimentKind,
    pub status: ExperimentStatus,
    pub parent_id: Option<ExperimentId>,
    pub candidate: Candidate,
    pub machine_fingerprint: String,
    pub kraftverk_version: String,
    pub os_info: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    /// Raw suite samples (each sample is a full MeasurementSet).
    pub samples: Vec<MeasurementSet>,
    /// Per-sample Kraft Index scores (normalized to baseline when available).
    pub index_samples: Vec<f64>,
    pub index_summary: Option<SampleSummary>,
    pub kraft_index: Option<KraftIndex>,
    pub comparison: Option<ComparisonResult>,
    pub decision: Decision,
    pub decision_reason: String,
    pub stability: StabilityVerdict,
    pub comparison_class: Option<ComparisonClass>,
    /// Serialized telemetry snapshots (JSON strings) per sample.
    pub telemetry: Vec<serde_json::Value>,
    /// Recovery journal path / token if an apply is in progress.
    pub recovery_token: Option<String>,
    /// Hardware eligibility policy active when the experiment was recorded.
    /// Current production value: `amd-only-v1`.
    #[serde(default = "default_hardware_policy")]
    pub hardware_policy: String,
}

fn default_hardware_policy() -> String {
    "amd-only-v1".into()
}

impl Experiment {
    pub fn new_baseline(
        fingerprint: impl Into<String>,
        version: impl Into<String>,
        os_info: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: ExperimentId::new(),
            kind: ExperimentKind::Baseline,
            status: ExperimentStatus::Pending,
            parent_id: None,
            candidate: Candidate::identity(),
            machine_fingerprint: fingerprint.into(),
            kraftverk_version: version.into(),
            os_info: os_info.into(),
            created_at: now,
            updated_at: now,
            completed_at: None,
            samples: vec![],
            index_samples: vec![],
            index_summary: None,
            kraft_index: None,
            comparison: None,
            decision: Decision::Baseline,
            decision_reason: "baseline reference".into(),
            stability: StabilityVerdict::Unknown,
            comparison_class: None,
            telemetry: vec![],
            recovery_token: None,
            hardware_policy: default_hardware_policy(),
        }
    }

    pub fn new_candidate(parent: &Experiment, candidate: Candidate) -> Self {
        let now = Utc::now();
        Self {
            id: ExperimentId::new(),
            kind: ExperimentKind::Candidate,
            status: ExperimentStatus::Pending,
            parent_id: Some(parent.id),
            candidate,
            machine_fingerprint: parent.machine_fingerprint.clone(),
            kraftverk_version: parent.kraftverk_version.clone(),
            os_info: parent.os_info.clone(),
            created_at: now,
            updated_at: now,
            completed_at: None,
            samples: vec![],
            index_samples: vec![],
            index_summary: None,
            kraft_index: None,
            comparison: None,
            decision: Decision::Pending,
            decision_reason: String::new(),
            stability: StabilityVerdict::Unknown,
            comparison_class: None,
            telemetry: vec![],
            recovery_token: None,
            hardware_policy: parent.hardware_policy.clone(),
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}
