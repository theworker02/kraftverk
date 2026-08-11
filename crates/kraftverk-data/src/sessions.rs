//! Session persistence for resumable optimize.

use kraftverk_core::error::{Error, Result};
use kraftverk_core::session::{OptimizeSession, SessionId, SessionStatus};
use rusqlite::{params, OptionalExtension};

use crate::db::ExperimentStore;

impl ExperimentStore {
    pub fn upsert_session(&self, session: &OptimizeSession) -> Result<()> {
        self.conn
            .execute(
                r#"
                INSERT INTO sessions (
                    id, status, goal, config_json, constraints_json, checkpoint_json,
                    failure_streak, notes, created_at, updated_at
                ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)
                ON CONFLICT(id) DO UPDATE SET
                    status=excluded.status,
                    goal=excluded.goal,
                    config_json=excluded.config_json,
                    constraints_json=excluded.constraints_json,
                    checkpoint_json=excluded.checkpoint_json,
                    failure_streak=excluded.failure_streak,
                    notes=excluded.notes,
                    updated_at=excluded.updated_at
                "#,
                params![
                    session.id.to_string(),
                    session.status.as_str(),
                    session.goal.as_str(),
                    serde_json::to_string(&session.config)?,
                    serde_json::to_string(&session.constraints)?,
                    session
                        .checkpoint
                        .as_ref()
                        .map(serde_json::to_string)
                        .transpose()?,
                    session.failure_streak as i64,
                    session.notes,
                    session.created_at.to_rfc3339(),
                    session.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|e| Error::Storage(format!("upsert session: {e}")))?;
        Ok(())
    }

    pub fn get_session(&self, id: &str) -> Result<Option<OptimizeSession>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, status, goal, config_json, constraints_json, checkpoint_json, failure_streak, notes, created_at, updated_at FROM sessions WHERE id = ?1 OR id LIKE ?2",
            )
            .map_err(|e| Error::Storage(e.to_string()))?;
        let like = format!("{id}%");
        let row = stmt
            .query_row(params![id, like], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                ))
            })
            .optional()
            .map_err(|e| Error::Storage(e.to_string()))?;

        match row {
            Some((
                sid,
                status,
                goal,
                config_json,
                constraints_json,
                checkpoint_json,
                failure_streak,
                notes,
                created_at,
                updated_at,
            )) => {
                let status = match status.as_str() {
                    "running" => SessionStatus::Running,
                    "paused" => SessionStatus::Paused,
                    "completed" => SessionStatus::Completed,
                    "failed" => SessionStatus::Failed,
                    "crashed" => SessionStatus::Crashed,
                    _ => SessionStatus::Failed,
                };
                let goal = kraftverk_core::OptimizeGoal::parse(&goal)
                    .unwrap_or(kraftverk_core::OptimizeGoal::Balanced);
                Ok(Some(OptimizeSession {
                    id: SessionId::parse(&sid).map_err(|e| Error::Storage(e.to_string()))?,
                    status,
                    goal,
                    config: serde_json::from_str(&config_json)?,
                    constraints: serde_json::from_str(&constraints_json)?,
                    checkpoint: checkpoint_json
                        .as_ref()
                        .map(|s| serde_json::from_str(s))
                        .transpose()?,
                    failure_streak: failure_streak as u32,
                    notes,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                        .map(|d| d.with_timezone(&chrono::Utc))
                        .map_err(|e| Error::Storage(e.to_string()))?,
                    updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
                        .map(|d| d.with_timezone(&chrono::Utc))
                        .map_err(|e| Error::Storage(e.to_string()))?,
                }))
            }
            None => Ok(None),
        }
    }

    pub fn list_sessions(&self, limit: usize) -> Result<Vec<OptimizeSession>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM sessions ORDER BY updated_at DESC LIMIT ?1")
            .map_err(|e| Error::Storage(e.to_string()))?;
        let ids: Vec<String> = stmt
            .query_map(params![limit as i64], |row| row.get(0))
            .map_err(|e| Error::Storage(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();
        let mut out = Vec::new();
        for id in ids {
            if let Some(s) = self.get_session(&id)? {
                out.push(s);
            }
        }
        Ok(out)
    }

    pub fn failure_streak(&self) -> Result<u32> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM meta WHERE key = 'failure_streak'")
            .map_err(|e| Error::Storage(e.to_string()))?;
        let v = stmt
            .query_row([], |row| row.get::<_, String>(0))
            .optional()
            .map_err(|e| Error::Storage(e.to_string()))?;
        Ok(v.and_then(|s| s.parse().ok()).unwrap_or(0))
    }

    pub fn set_failure_streak(&self, n: u32) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO meta(key, value) VALUES('failure_streak', ?1)
                 ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                params![n.to_string()],
            )
            .map_err(|e| Error::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn safe_mode_recommended(&self) -> Result<bool> {
        Ok(self.failure_streak()? >= 3)
    }
}
