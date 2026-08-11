//! Experiment store operations.

use std::path::{Path, PathBuf};

use kraftverk_core::error::{Error, Result};
use kraftverk_core::experiment::{Decision, Experiment, ExperimentId, ExperimentKind};
use rusqlite::{params, Connection, OptionalExtension};
use tracing::info;

use crate::schema::SCHEMA;

pub struct ExperimentStore {
    path: PathBuf,
    pub(crate) conn: Connection,
}

impl ExperimentStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&path).map_err(|e| Error::Storage(format!("open db: {e}")))?;
        conn.execute_batch(SCHEMA)
            .map_err(|e| Error::Storage(format!("schema: {e}")))?;
        // Best-effort migration for stores created before hardware_policy existed.
        let _ = conn.execute(
            "ALTER TABLE experiments ADD COLUMN hardware_policy TEXT NOT NULL DEFAULT 'amd-only-v1'",
            [],
        );
        conn.execute(
            "INSERT OR IGNORE INTO meta(key, value) VALUES ('schema_version', '3')",
            [],
        )
        .map_err(|e| Error::Storage(e.to_string()))?;
        let _ = conn.execute(
            "INSERT OR REPLACE INTO meta(key, value) VALUES ('hardware_policy', 'amd-only-v1')",
            [],
        );
        info!(path = %path.display(), "opened experiment store");
        Ok(Self { path, conn })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn upsert(&self, exp: &Experiment) -> Result<()> {
        let samples_json = serde_json::to_string(&exp.samples)?;
        let index_samples_json = serde_json::to_string(&exp.index_samples)?;
        let index_summary_json = match &exp.index_summary {
            Some(s) => Some(serde_json::to_string(s)?),
            None => None,
        };
        let kraft_index_json = match &exp.kraft_index {
            Some(k) => Some(serde_json::to_string(k)?),
            None => None,
        };
        let comparison_json = match &exp.comparison {
            Some(c) => Some(serde_json::to_string(c)?),
            None => None,
        };
        let telemetry_json = serde_json::to_string(&exp.telemetry)?;
        let candidate_json = serde_json::to_string(&exp.candidate)?;
        let parent = exp.parent_id.map(|p| p.to_string());
        let class = exp.comparison_class.map(|c| c.as_str().to_string());
        let score = exp.kraft_index.as_ref().map(|k| k.score);

        self.conn
            .execute(
                r#"
                INSERT INTO experiments (
                    id, kind, status, parent_id, candidate_json, machine_fingerprint,
                    kraftverk_version, os_info, created_at, updated_at, completed_at,
                    samples_json, index_samples_json, index_summary_json, kraft_index_json,
                    comparison_json, decision, decision_reason, stability, comparison_class,
                    telemetry_json, recovery_token, kraft_index_score, hardware_policy
                ) VALUES (
                    ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24
                )
                ON CONFLICT(id) DO UPDATE SET
                    kind=excluded.kind,
                    status=excluded.status,
                    parent_id=excluded.parent_id,
                    candidate_json=excluded.candidate_json,
                    machine_fingerprint=excluded.machine_fingerprint,
                    kraftverk_version=excluded.kraftverk_version,
                    os_info=excluded.os_info,
                    created_at=excluded.created_at,
                    updated_at=excluded.updated_at,
                    completed_at=excluded.completed_at,
                    samples_json=excluded.samples_json,
                    index_samples_json=excluded.index_samples_json,
                    index_summary_json=excluded.index_summary_json,
                    kraft_index_json=excluded.kraft_index_json,
                    comparison_json=excluded.comparison_json,
                    decision=excluded.decision,
                    decision_reason=excluded.decision_reason,
                    stability=excluded.stability,
                    comparison_class=excluded.comparison_class,
                    telemetry_json=excluded.telemetry_json,
                    recovery_token=excluded.recovery_token,
                    kraft_index_score=excluded.kraft_index_score,
                    hardware_policy=excluded.hardware_policy
                "#,
                params![
                    exp.id.to_string(),
                    serde_json::to_string(&exp.kind)?.trim_matches('"').to_string(),
                    serde_json::to_string(&exp.status)?.trim_matches('"').to_string(),
                    parent,
                    candidate_json,
                    exp.machine_fingerprint,
                    exp.kraftverk_version,
                    exp.os_info,
                    exp.created_at.to_rfc3339(),
                    exp.updated_at.to_rfc3339(),
                    exp.completed_at.map(|t| t.to_rfc3339()),
                    samples_json,
                    index_samples_json,
                    index_summary_json,
                    kraft_index_json,
                    comparison_json,
                    serde_json::to_string(&exp.decision)?.trim_matches('"').to_string(),
                    exp.decision_reason,
                    exp.stability.as_str(),
                    class,
                    telemetry_json,
                    exp.recovery_token,
                    score,
                    exp.hardware_policy,
                ],
            )
            .map_err(|e| Error::Storage(format!("upsert: {e}")))?;
        Ok(())
    }

    pub fn get(&self, id: &ExperimentId) -> Result<Option<Experiment>> {
        self.get_str(&id.to_string())
    }

    pub fn get_str(&self, id: &str) -> Result<Option<Experiment>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM experiments WHERE id = ?1 OR id LIKE ?2")
            .map_err(|e| Error::Storage(e.to_string()))?;
        let like = format!("{id}%");
        let row = stmt
            .query_row(params![id, like], self::row_to_json)
            .optional()
            .map_err(|e| Error::Storage(e.to_string()))?;
        match row {
            Some(v) => Ok(Some(json_to_experiment(v)?)),
            None => Ok(None),
        }
    }

    pub fn latest_baseline(&self, fingerprint: &str) -> Result<Option<Experiment>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT * FROM experiments
                WHERE kind = 'baseline' AND machine_fingerprint = ?1 AND decision = 'baseline'
                ORDER BY created_at DESC LIMIT 1
                "#,
            )
            .map_err(|e| Error::Storage(e.to_string()))?;
        let row = stmt
            .query_row(params![fingerprint], self::row_to_json)
            .optional()
            .map_err(|e| Error::Storage(e.to_string()))?;
        match row {
            Some(v) => Ok(Some(json_to_experiment(v)?)),
            None => Ok(None),
        }
    }

    pub fn latest_accepted(&self, fingerprint: &str) -> Result<Option<Experiment>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT * FROM experiments
                WHERE machine_fingerprint = ?1 AND decision = 'accept'
                ORDER BY created_at DESC LIMIT 1
                "#,
            )
            .map_err(|e| Error::Storage(e.to_string()))?;
        let row = stmt
            .query_row(params![fingerprint], self::row_to_json)
            .optional()
            .map_err(|e| Error::Storage(e.to_string()))?;
        match row {
            Some(v) => Ok(Some(json_to_experiment(v)?)),
            None => Ok(None),
        }
    }

    pub fn history(&self, fingerprint: Option<&str>, limit: usize) -> Result<Vec<Experiment>> {
        let mut out = Vec::new();
        if let Some(fp) = fingerprint {
            let mut stmt = self
                .conn
                .prepare(
                    r#"
                    SELECT * FROM experiments
                    WHERE machine_fingerprint = ?1
                    ORDER BY created_at DESC LIMIT ?2
                    "#,
                )
                .map_err(|e| Error::Storage(e.to_string()))?;
            let rows = stmt
                .query_map(params![fp, limit as i64], self::row_to_json)
                .map_err(|e| Error::Storage(e.to_string()))?;
            for r in rows {
                let v = r.map_err(|e| Error::Storage(e.to_string()))?;
                out.push(json_to_experiment(v)?);
            }
        } else {
            let mut stmt = self
                .conn
                .prepare(
                    r#"
                    SELECT * FROM experiments
                    ORDER BY created_at DESC LIMIT ?1
                    "#,
                )
                .map_err(|e| Error::Storage(e.to_string()))?;
            let rows = stmt
                .query_map(params![limit as i64], self::row_to_json)
                .map_err(|e| Error::Storage(e.to_string()))?;
            for r in rows {
                let v = r.map_err(|e| Error::Storage(e.to_string()))?;
                out.push(json_to_experiment(v)?);
            }
        }
        Ok(out)
    }

    pub fn find_by_kind(&self, kind: ExperimentKind, limit: usize) -> Result<Vec<Experiment>> {
        let kind_s = serde_json::to_string(&kind)?.trim_matches('"').to_string();
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM experiments WHERE kind = ?1 ORDER BY created_at DESC LIMIT ?2")
            .map_err(|e| Error::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(params![kind_s, limit as i64], self::row_to_json)
            .map_err(|e| Error::Storage(e.to_string()))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(json_to_experiment(
                r.map_err(|e| Error::Storage(e.to_string()))?,
            )?);
        }
        Ok(out)
    }

    pub fn set_active_config(&self, candidate_json: &str) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO meta(key, value) VALUES('active_candidate', ?1)
                 ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                params![candidate_json],
            )
            .map_err(|e| Error::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn active_config(&self) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM meta WHERE key = 'active_candidate'")
            .map_err(|e| Error::Storage(e.to_string()))?;
        let v = stmt
            .query_row([], |row| row.get::<_, String>(0))
            .optional()
            .map_err(|e| Error::Storage(e.to_string()))?;
        Ok(v)
    }

    pub fn clear_active_config(&self) -> Result<()> {
        self.conn
            .execute("DELETE FROM meta WHERE key = 'active_candidate'", [])
            .map_err(|e| Error::Storage(e.to_string()))?;
        Ok(())
    }
}

fn row_to_json(row: &rusqlite::Row<'_>) -> rusqlite::Result<serde_json::Value> {
    let hardware_policy = row
        .get::<_, String>("hardware_policy")
        .unwrap_or_else(|_| "amd-only-v1".into());
    Ok(serde_json::json!({
        "id": row.get::<_, String>("id")?,
        "kind": row.get::<_, String>("kind")?,
        "status": row.get::<_, String>("status")?,
        "parent_id": row.get::<_, Option<String>>("parent_id")?,
        "candidate_json": row.get::<_, String>("candidate_json")?,
        "machine_fingerprint": row.get::<_, String>("machine_fingerprint")?,
        "kraftverk_version": row.get::<_, String>("kraftverk_version")?,
        "os_info": row.get::<_, String>("os_info")?,
        "created_at": row.get::<_, String>("created_at")?,
        "updated_at": row.get::<_, String>("updated_at")?,
        "completed_at": row.get::<_, Option<String>>("completed_at")?,
        "samples_json": row.get::<_, String>("samples_json")?,
        "index_samples_json": row.get::<_, String>("index_samples_json")?,
        "index_summary_json": row.get::<_, Option<String>>("index_summary_json")?,
        "kraft_index_json": row.get::<_, Option<String>>("kraft_index_json")?,
        "comparison_json": row.get::<_, Option<String>>("comparison_json")?,
        "decision": row.get::<_, String>("decision")?,
        "decision_reason": row.get::<_, String>("decision_reason")?,
        "stability": row.get::<_, String>("stability")?,
        "comparison_class": row.get::<_, Option<String>>("comparison_class")?,
        "telemetry_json": row.get::<_, String>("telemetry_json")?,
        "recovery_token": row.get::<_, Option<String>>("recovery_token")?,
        "hardware_policy": hardware_policy,
    }))
}

fn json_to_experiment(v: serde_json::Value) -> Result<Experiment> {
    use chrono::{DateTime, Utc};
    use kraftverk_core::classification::ComparisonClass;
    use kraftverk_core::experiment::{ExperimentStatus, StabilityVerdict};
    use kraftverk_core::kraft_index::KraftIndex;
    use kraftverk_core::measurement::MeasurementSet;
    use kraftverk_core::statistics::{ComparisonResult, SampleSummary};

    let id = ExperimentId::parse(v["id"].as_str().unwrap_or_default())
        .map_err(|e| Error::Storage(e.to_string()))?;
    let kind: ExperimentKind = serde_json::from_value(serde_json::Value::String(
        v["kind"].as_str().unwrap_or("candidate").into(),
    ))?;
    let status: ExperimentStatus = serde_json::from_value(serde_json::Value::String(
        v["status"].as_str().unwrap_or("completed").into(),
    ))?;
    let decision: Decision = serde_json::from_value(serde_json::Value::String(
        v["decision"].as_str().unwrap_or("pending").into(),
    ))?;
    let stability = match v["stability"].as_str().unwrap_or("UNKNOWN") {
        "PASS" => StabilityVerdict::Pass,
        "FAIL" => StabilityVerdict::Fail,
        _ => StabilityVerdict::Unknown,
    };
    let parent_id = v["parent_id"]
        .as_str()
        .map(ExperimentId::parse)
        .transpose()
        .map_err(|e| Error::Storage(e.to_string()))?;
    let comparison_class = v["comparison_class"].as_str().and_then(|s| {
        serde_json::from_value::<ComparisonClass>(serde_json::Value::String(s.into())).ok()
    });

    Ok(Experiment {
        id,
        kind,
        status,
        parent_id,
        candidate: serde_json::from_str(v["candidate_json"].as_str().unwrap_or("{}"))?,
        machine_fingerprint: v["machine_fingerprint"].as_str().unwrap_or("").to_string(),
        kraftverk_version: v["kraftverk_version"].as_str().unwrap_or("").to_string(),
        os_info: v["os_info"].as_str().unwrap_or("").to_string(),
        created_at: DateTime::parse_from_rfc3339(v["created_at"].as_str().unwrap_or(""))
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        updated_at: DateTime::parse_from_rfc3339(v["updated_at"].as_str().unwrap_or(""))
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        completed_at: v["completed_at"].as_str().and_then(|s| {
            DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|d| d.with_timezone(&Utc))
        }),
        samples: serde_json::from_str::<Vec<MeasurementSet>>(
            v["samples_json"].as_str().unwrap_or("[]"),
        )
        .unwrap_or_default(),
        index_samples: serde_json::from_str(v["index_samples_json"].as_str().unwrap_or("[]"))
            .unwrap_or_default(),
        index_summary: v["index_summary_json"]
            .as_str()
            .and_then(|s| serde_json::from_str::<SampleSummary>(s).ok()),
        kraft_index: v["kraft_index_json"]
            .as_str()
            .and_then(|s| serde_json::from_str::<KraftIndex>(s).ok()),
        comparison: v["comparison_json"]
            .as_str()
            .and_then(|s| serde_json::from_str::<ComparisonResult>(s).ok()),
        decision,
        decision_reason: v["decision_reason"].as_str().unwrap_or("").to_string(),
        stability,
        comparison_class,
        telemetry: serde_json::from_str(v["telemetry_json"].as_str().unwrap_or("[]"))
            .unwrap_or_default(),
        recovery_token: v["recovery_token"].as_str().map(|s| s.to_string()),
        hardware_policy: v["hardware_policy"]
            .as_str()
            .unwrap_or("amd-only-v1")
            .to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kraftverk_core::experiment::Experiment;
    use tempfile::tempdir;

    #[test]
    fn roundtrip_baseline() {
        let dir = tempdir().unwrap();
        let store = ExperimentStore::open(dir.path().join("t.db")).unwrap();
        let exp = Experiment::new_baseline("fp-test", "0.1.0", "test-os");
        store.upsert(&exp).unwrap();
        let got = store.get(&exp.id).unwrap().unwrap();
        assert_eq!(got.machine_fingerprint, "fp-test");
        assert!(matches!(got.kind, ExperimentKind::Baseline));
    }
}
