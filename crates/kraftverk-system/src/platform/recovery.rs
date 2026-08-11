//! Crash-recovery journal for in-flight parameter changes.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use kraftverk_core::candidate::{Candidate, ParamChange};
use kraftverk_core::error::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryRecord {
    pub experiment_id: String,
    pub candidate: Candidate,
    pub applied: Vec<ParamChange>,
    pub started_at: DateTime<Utc>,
    pub status: String,
    pub last_error: Option<String>,
}

#[derive(Debug)]
pub struct RecoveryJournal {
    path: PathBuf,
    record: Option<RecoveryRecord>,
}

impl RecoveryJournal {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let record = if path.exists() {
            let data = fs::read_to_string(&path)?;
            if data.trim().is_empty() {
                None
            } else {
                Some(serde_json::from_str(&data)?)
            }
        } else {
            None
        };
        Ok(Self { path, record })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn interrupted(&self) -> Option<&RecoveryRecord> {
        self.record
            .as_ref()
            .filter(|r| matches!(r.status.as_str(), "running" | "applied" | "failed"))
    }

    pub fn begin(&mut self, experiment_id: &str, candidate: &Candidate) -> Result<()> {
        self.record = Some(RecoveryRecord {
            experiment_id: experiment_id.to_string(),
            candidate: candidate.clone(),
            applied: vec![],
            started_at: Utc::now(),
            status: "running".into(),
            last_error: None,
        });
        self.flush()
    }

    pub fn record_applied(&mut self, change: &ParamChange) -> Result<()> {
        if let Some(r) = self.record.as_mut() {
            r.applied.push(change.clone());
            r.status = "applied".into();
        }
        self.flush()
    }

    pub fn complete(&mut self) -> Result<()> {
        if let Some(r) = self.record.as_mut() {
            r.status = "complete".into();
        }
        self.flush()?;
        self.clear()
    }

    pub fn rolled_back(&mut self) -> Result<()> {
        if let Some(r) = self.record.as_mut() {
            r.status = "rolled_back".into();
        }
        self.flush()?;
        self.clear()
    }

    pub fn fail(&mut self, err: &str) -> Result<()> {
        if let Some(r) = self.record.as_mut() {
            r.status = "failed".into();
            r.last_error = Some(err.to_string());
        }
        self.flush()
    }

    pub fn clear(&mut self) -> Result<()> {
        self.record = None;
        if self.path.exists() {
            fs::remove_file(&self.path)?;
        }
        Ok(())
    }

    fn flush(&self) -> Result<()> {
        if let Some(r) = &self.record {
            let data = serde_json::to_string_pretty(r)?;
            fs::write(&self.path, data)?;
        }
        Ok(())
    }

    /// Restore an interrupted journal using the provided platform.
    pub fn recover_with<P: crate::tuner::Platform>(
        &mut self,
        platform: &mut P,
    ) -> Result<Option<String>> {
        let Some(record) = self.interrupted().cloned() else {
            return Ok(None);
        };
        for change in record.applied.iter().rev() {
            platform.rollback_change(change).map_err(|e| {
                Error::RollbackRequired(format!(
                    "failed to recover experiment {}: {e}",
                    record.experiment_id
                ))
            })?;
        }
        // Also try rolling back any declared changes not in applied (best effort).
        for change in record.candidate.changes.iter().rev() {
            if !record.applied.iter().any(|a| a.key == change.key) {
                let _ = platform.rollback_change(change);
            }
        }
        let id = record.experiment_id.clone();
        self.rolled_back()?;
        Ok(Some(id))
    }
}
