//! Optimization candidates: reversible parameter change sets.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A typed parameter value that can be applied and rolled back.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ParamValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
}

impl ParamValue {
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Int(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_usize(&self) -> Option<usize> {
        self.as_i64().and_then(|v| usize::try_from(v).ok())
    }

    pub fn display(&self) -> String {
        match self {
            Self::Int(v) => v.to_string(),
            Self::Float(v) => format!("{v}"),
            Self::Bool(v) => v.to_string(),
            Self::String(v) => v.clone(),
        }
    }
}

/// One reversible change relative to a known previous value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamChange {
    /// Stable parameter id, e.g. `bench.worker_threads`.
    pub key: String,
    pub previous: ParamValue,
    pub next: ParamValue,
    /// Human-readable rationale for why this might help.
    pub rationale: String,
}

/// A named set of parameter changes forming one experiment candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Candidate {
    pub id: String,
    pub label: String,
    pub changes: Vec<ParamChange>,
    /// Extra metadata (strategy generation, etc.).
    #[serde(default)]
    pub meta: IndexMap<String, String>,
}

impl Candidate {
    pub fn identity() -> Self {
        Self {
            id: "identity".into(),
            label: "no changes".into(),
            changes: vec![],
            meta: IndexMap::new(),
        }
    }

    pub fn is_identity(&self) -> bool {
        self.changes.is_empty()
    }

    pub fn content_hash(&self) -> String {
        let mut hasher = Sha256::new();
        let mut keys: Vec<_> = self.changes.iter().map(|c| &c.key).collect();
        keys.sort();
        for key in keys {
            if let Some(c) = self.changes.iter().find(|c| &c.key == key) {
                hasher.update(key.as_bytes());
                hasher.update(b"=");
                hasher.update(c.next.display().as_bytes());
                hasher.update(b";");
            }
        }
        hex::encode(hasher.finalize())[..16].to_string()
    }

    pub fn summary_line(&self) -> String {
        if self.changes.is_empty() {
            return "no changes".into();
        }
        self.changes
            .iter()
            .map(|c| format!("{}={}→{}", c.key, c.previous.display(), c.next.display()))
            .collect::<Vec<_>>()
            .join(", ")
    }
}
