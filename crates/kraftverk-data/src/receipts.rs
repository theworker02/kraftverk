//! Evidence receipts for accepted optimizations (hash-bound, no fabricated claims).

use std::path::{Path, PathBuf};

use kraftverk_core::Experiment;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::paths::default_data_dir;
use kraftverk_core::error::{Error, Result};

/// Portable evidence receipt written when a change is accepted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceReceipt {
    pub format: String,
    pub version: u32,
    pub receipt_id: String,
    pub experiment_id: String,
    pub decision: String,
    pub stability: String,
    pub kraft_index_mean: Option<f64>,
    pub kraft_index_cov: Option<f64>,
    pub relative_change: Option<f64>,
    pub comparison_class: Option<String>,
    pub candidate_summary: String,
    pub candidate: kraftverk_core::Candidate,
    pub machine_fingerprint: String,
    pub kraftverk_version: String,
    pub created_at: String,
    pub sample_count: usize,
    pub decision_reason: String,
    /// SHA-256 of canonical evidence payload (hex).
    pub evidence_hash: String,
    pub notes: Vec<String>,
}

impl EvidenceReceipt {
    pub fn from_experiment(exp: &Experiment) -> Self {
        let kraft_index_mean = exp.index_summary.as_ref().map(|s| s.mean);
        let kraft_index_cov = exp.index_summary.as_ref().map(|s| s.cov);
        let relative_change = exp.comparison.as_ref().map(|c| c.relative_change);
        let comparison_class = exp.comparison_class.map(|c| c.as_str().to_string());
        let created_at = chrono::Utc::now().to_rfc3339();

        let mut receipt = Self {
            format: "kraftverk.receipt".into(),
            version: 1,
            receipt_id: uuid::Uuid::new_v4().to_string(),
            experiment_id: exp.id.to_string(),
            decision: format!("{:?}", exp.decision),
            stability: exp.stability.as_str().into(),
            kraft_index_mean,
            kraft_index_cov,
            relative_change,
            comparison_class,
            candidate_summary: exp.candidate.summary_line(),
            candidate: exp.candidate.clone(),
            machine_fingerprint: exp.machine_fingerprint.clone(),
            kraftverk_version: exp.kraftverk_version.clone(),
            created_at: created_at.clone(),
            sample_count: exp.index_samples.len(),
            decision_reason: exp.decision_reason.clone(),
            evidence_hash: String::new(),
            notes: vec![
                "Receipt binds to measured experiment evidence.".into(),
                "Hash excludes receipt_id so re-exports of the same evidence collide.".into(),
            ],
        };
        receipt.evidence_hash = receipt.compute_hash();
        receipt
    }

    fn canonical_payload(&self) -> serde_json::Value {
        serde_json::json!({
            "format": self.format,
            "version": self.version,
            "experiment_id": self.experiment_id,
            "decision": self.decision,
            "stability": self.stability,
            "kraft_index_mean": self.kraft_index_mean,
            "kraft_index_cov": self.kraft_index_cov,
            "relative_change": self.relative_change,
            "comparison_class": self.comparison_class,
            "candidate": self.candidate,
            "machine_fingerprint": self.machine_fingerprint,
            "kraftverk_version": self.kraftverk_version,
            "sample_count": self.sample_count,
            "decision_reason": self.decision_reason,
        })
    }

    pub fn compute_hash(&self) -> String {
        let bytes = serde_json::to_vec(&self.canonical_payload()).unwrap_or_default();
        let digest = Sha256::digest(&bytes);
        hex::encode(digest)
    }

    pub fn verify(&self) -> bool {
        self.evidence_hash == self.compute_hash()
            && self.format == "kraftverk.receipt"
            && self.version == 1
    }

    pub fn validate(&self) -> std::result::Result<(), String> {
        if self.format != "kraftverk.receipt" {
            return Err(format!("unknown format '{}'", self.format));
        }
        if self.version != 1 {
            return Err(format!("unsupported receipt version {}", self.version));
        }
        if !self.verify() {
            return Err("evidence hash mismatch".into());
        }
        Ok(())
    }
}

pub fn receipts_dir() -> Result<PathBuf> {
    let p = default_data_dir()?.join("receipts");
    std::fs::create_dir_all(&p).map_err(|e| Error::Storage(e.to_string()))?;
    Ok(p)
}

pub fn write_receipt(exp: &Experiment, path: Option<&Path>) -> Result<(PathBuf, EvidenceReceipt)> {
    let receipt = EvidenceReceipt::from_experiment(exp);
    let out = match path {
        Some(p) => p.to_path_buf(),
        None => receipts_dir()?.join(format!("{}.kraft-receipt.json", receipt.experiment_id)),
    };
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::Storage(e.to_string()))?;
    }
    let body = serde_json::to_string_pretty(&receipt).map_err(|e| Error::Storage(e.to_string()))?;
    std::fs::write(&out, body).map_err(|e| Error::Storage(e.to_string()))?;
    Ok((out, receipt))
}

pub fn load_receipt(path: &Path) -> Result<EvidenceReceipt> {
    let raw = std::fs::read_to_string(path).map_err(|e| Error::Storage(e.to_string()))?;
    let receipt: EvidenceReceipt =
        serde_json::from_str(&raw).map_err(|e| Error::Storage(e.to_string()))?;
    receipt
        .validate()
        .map_err(|e| Error::Storage(format!("invalid receipt: {e}")))?;
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kraftverk_core::statistics::StatsConfig;

    #[test]
    fn receipt_hash_stable_and_verifies() {
        let mut exp = Experiment::new_baseline("fp", "0.2.0", "test");
        exp.index_samples = vec![10000.0, 10010.0, 9995.0];
        exp.index_summary =
            kraftverk_core::summarize(&exp.index_samples, &StatsConfig::default()).ok();
        exp.decision = kraftverk_core::Decision::Accept;
        exp.decision_reason = "improved".into();

        let r1 = EvidenceReceipt::from_experiment(&exp);
        assert!(r1.verify());
        let h1 = r1.evidence_hash.clone();
        let r2 = EvidenceReceipt::from_experiment(&exp);
        assert_eq!(h1, r2.evidence_hash);
    }
}
