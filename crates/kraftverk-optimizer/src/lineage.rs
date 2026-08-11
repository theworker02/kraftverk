//! Experiment lineage helpers.

use kraftverk_core::Experiment;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageNode {
    pub id: String,
    pub kind: String,
    pub decision: String,
    pub score: Option<f64>,
    pub parent_id: Option<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageTree {
    pub root_id: String,
    pub nodes: Vec<LineageNode>,
}

pub fn build_lineage(experiments: &[Experiment], focus_id: &str) -> LineageTree {
    let focus = experiments
        .iter()
        .find(|e| e.id.to_string() == focus_id || e.id.to_string().starts_with(focus_id));

    let root_id = focus
        .and_then(|e| e.parent_id.map(|p| p.to_string()))
        .or_else(|| focus.map(|e| e.id.to_string()))
        .unwrap_or_else(|| focus_id.to_string());

    // Collect ancestor chain + siblings sharing parent.
    let mut ids = std::collections::BTreeSet::new();
    if let Some(f) = focus {
        ids.insert(f.id.to_string());
        let mut cur = f.parent_id;
        while let Some(pid) = cur {
            ids.insert(pid.to_string());
            cur = experiments
                .iter()
                .find(|e| e.id == pid)
                .and_then(|e| e.parent_id);
        }
        if let Some(parent) = f.parent_id {
            for e in experiments {
                if e.parent_id == Some(parent) {
                    ids.insert(e.id.to_string());
                }
            }
        }
    }

    let nodes: Vec<_> = experiments
        .iter()
        .filter(|e| ids.contains(&e.id.to_string()) || e.id.to_string().starts_with(focus_id))
        .map(|e| LineageNode {
            id: e.id.to_string(),
            kind: format!("{:?}", e.kind).to_ascii_lowercase(),
            decision: format!("{:?}", e.decision).to_ascii_lowercase(),
            score: e.index_summary.as_ref().map(|s| s.mean),
            parent_id: e.parent_id.map(|p| p.to_string()),
            summary: e.candidate.summary_line(),
        })
        .collect();

    LineageTree { root_id, nodes }
}
