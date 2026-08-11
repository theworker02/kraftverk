//! Optimization sessions and checkpoints for resumable runs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::candidate::Candidate;
use crate::config::OptimizeConfig;
use crate::constraints::OptimizeConstraints;
use crate::goals::OptimizeGoal;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub Uuid);

impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn parse(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Running,
    Paused,
    Completed,
    Failed,
    Crashed,
}

impl SessionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Crashed => "crashed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizeCheckpoint {
    pub generation: usize,
    pub experiments_done: usize,
    pub plateau: usize,
    pub best_score: f64,
    pub best_candidate: Candidate,
    pub best_experiment_id: Option<String>,
    pub baseline_id: String,
    pub last_experiment_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizeSession {
    pub id: SessionId,
    pub status: SessionStatus,
    pub goal: OptimizeGoal,
    pub config: OptimizeConfig,
    pub constraints: OptimizeConstraints,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub checkpoint: Option<OptimizeCheckpoint>,
    pub failure_streak: u32,
    pub notes: String,
}

impl OptimizeSession {
    pub fn new(
        goal: OptimizeGoal,
        config: OptimizeConfig,
        constraints: OptimizeConstraints,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: SessionId::new(),
            status: SessionStatus::Running,
            goal,
            config,
            constraints,
            created_at: now,
            updated_at: now,
            checkpoint: None,
            failure_streak: 0,
            notes: String::new(),
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}
