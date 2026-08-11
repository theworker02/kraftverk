//! Optimization profiles and .kraft profile packages.

use kraftverk_core::{OptimizeGoal, OptimizeMode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileInfo {
    pub id: String,
    pub name: String,
    pub mode: OptimizeMode,
    pub goal: OptimizeGoal,
    pub available: bool,
    pub notes: String,
}

pub fn list_profiles() -> Vec<ProfileInfo> {
    vec![
        ProfileInfo {
            id: "safe-default".into(),
            name: "Safe default".into(),
            mode: OptimizeMode::Safe,
            goal: OptimizeGoal::Balanced,
            available: true,
            notes: "Process-scoped worker/rayon counts, optional priority & affinity.".into(),
        },
        ProfileInfo {
            id: "balanced".into(),
            name: "Balanced".into(),
            mode: OptimizeMode::Balanced,
            goal: OptimizeGoal::Balanced,
            available: true,
            notes: "Broader reversible search within safe knobs.".into(),
        },
        ProfileInfo {
            id: "gaming".into(),
            name: "Gaming".into(),
            mode: OptimizeMode::Aggressive,
            goal: OptimizeGoal::Gaming,
            available: true,
            notes: "Biases responsiveness; GPU backends remain unsupported.".into(),
        },
        ProfileInfo {
            id: "compile".into(),
            name: "Compile".into(),
            mode: OptimizeMode::Balanced,
            goal: OptimizeGoal::Compile,
            available: true,
            notes: "Biases multi-core / compile-proxy benches.".into(),
        },
        ProfileInfo {
            id: "workstation".into(),
            name: "Workstation".into(),
            mode: OptimizeMode::Balanced,
            goal: OptimizeGoal::Workstation,
            available: true,
            notes: "Mixed productivity bias.".into(),
        },
        ProfileInfo {
            id: "throughput".into(),
            name: "Throughput".into(),
            mode: OptimizeMode::Aggressive,
            goal: OptimizeGoal::Throughput,
            available: true,
            notes: "Maximize aggregate work rate within reversible knobs.".into(),
        },
        ProfileInfo {
            id: "latency".into(),
            name: "Latency".into(),
            mode: OptimizeMode::Safe,
            goal: OptimizeGoal::Latency,
            available: true,
            notes: "Favor responsiveness index.".into(),
        },
        ProfileInfo {
            id: "efficiency".into(),
            name: "Efficiency".into(),
            mode: OptimizeMode::Safe,
            goal: OptimizeGoal::Efficiency,
            available: true,
            notes: "Prefer fewer workers when score-per-thread is better.".into(),
        },
        ProfileInfo {
            id: "sustained".into(),
            name: "Sustained".into(),
            mode: OptimizeMode::Balanced,
            goal: OptimizeGoal::Sustained,
            available: true,
            notes: "Longer validation; optional sustained benches.".into(),
        },
        ProfileInfo {
            id: "quiet".into(),
            name: "Quiet".into(),
            mode: OptimizeMode::Safe,
            goal: OptimizeGoal::Quiet,
            available: true,
            notes: "Prefer lower intensity; thermal limits when sensors exist.".into(),
        },
        ProfileInfo {
            id: "aggressive".into(),
            name: "Aggressive".into(),
            mode: OptimizeMode::Aggressive,
            goal: OptimizeGoal::Throughput,
            available: true,
            notes: "Widest reversible search; still no firmware/power-plan changes without agent."
                .into(),
        },
    ]
}

/// Portable profile document (`.kraft` JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KraftProfile {
    pub format: String,
    pub version: u32,
    pub id: String,
    pub name: String,
    pub goal: OptimizeGoal,
    pub mode: OptimizeMode,
    pub candidate: kraftverk_core::Candidate,
    pub machine_fingerprint: Option<String>,
    pub created_at: String,
    pub notes: String,
}

impl KraftProfile {
    pub fn validate(&self) -> Result<(), String> {
        if self.format != "kraftverk.profile" {
            return Err(format!("unknown format '{}'", self.format));
        }
        if self.version == 0 || self.version > 1 {
            return Err(format!("unsupported profile version {}", self.version));
        }
        if self.id.is_empty() || self.name.is_empty() {
            return Err("id and name are required".into());
        }
        Ok(())
    }
}

pub fn recommend_profile(goal: OptimizeGoal) -> ProfileInfo {
    list_profiles()
        .into_iter()
        .find(|p| p.goal == goal && p.available)
        .unwrap_or_else(|| list_profiles()[0].clone())
}
