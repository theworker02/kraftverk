//! Objective scoring helpers (efficiency / thermal frontiers are informational).

use kraftverk_core::OptimizeGoal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectiveScore {
    pub goal: OptimizeGoal,
    /// Primary score (usually Kraft Index mean).
    pub primary: f64,
    /// Efficiency proxy: primary / worker_threads (when known).
    pub efficiency: Option<f64>,
    /// Notes about unavailable frontiers (no invented thermal curves).
    pub notes: Vec<String>,
}

pub fn score_objective(
    goal: OptimizeGoal,
    primary: f64,
    worker_threads: Option<usize>,
    temp_c: Option<f64>,
) -> ObjectiveScore {
    let mut notes = Vec::new();
    let efficiency = worker_threads.map(|w| primary / (w.max(1) as f64));

    match goal {
        OptimizeGoal::Efficiency => {
            notes.push("Efficiency uses score-per-worker when worker count is known.".into());
        }
        OptimizeGoal::Quiet | OptimizeGoal::Sustained => {
            if temp_c.is_none() {
                notes.push(
                    "Thermal frontier unavailable without temperature sensors; not fabricated."
                        .into(),
                );
            }
        }
        OptimizeGoal::Gaming => {
            notes.push(
                "GPU contribution unsupported; objective uses CPU/realtime proxies only.".into(),
            );
        }
        _ => {}
    }

    ObjectiveScore {
        goal,
        primary,
        efficiency,
        notes,
    }
}

/// Adaptive parameter importance from experiment outcomes (simple frequency of accepts).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParamImportance {
    pub counts: std::collections::BTreeMap<String, ImportanceStat>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImportanceStat {
    pub trials: u32,
    pub accepts: u32,
}

impl ParamImportance {
    pub fn record(&mut self, key: &str, accepted: bool) {
        let e = self.counts.entry(key.to_string()).or_default();
        e.trials += 1;
        if accepted {
            e.accepts += 1;
        }
    }

    pub fn rank(&self) -> Vec<(String, f64)> {
        let mut v: Vec<_> = self
            .counts
            .iter()
            .map(|(k, s)| {
                let rate = if s.trials == 0 {
                    0.0
                } else {
                    s.accepts as f64 / s.trials as f64
                };
                (k.clone(), rate)
            })
            .collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        v
    }
}
