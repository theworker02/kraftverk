//! Runtime and optimization configuration.

use serde::{Deserialize, Serialize};

use crate::constraints::OptimizeConstraints;
use crate::goals::OptimizeGoal;

/// How aggressively Kraftverk may experiment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OptimizeMode {
    /// Only reversible, low-risk, process-scoped or workload-scoped parameters.
    #[default]
    Safe,
    /// Broader safe search space; still reversible.
    Balanced,
    /// Widest reversible search; still no irreversible firmware changes.
    Aggressive,
}

impl OptimizeMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Balanced => "balanced",
            Self::Aggressive => "aggressive",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "safe" => Some(Self::Safe),
            "balanced" => Some(Self::Balanced),
            "aggressive" => Some(Self::Aggressive),
            _ => None,
        }
    }

    /// Modes available without privileged agent.
    pub fn supported_without_agent(self) -> bool {
        matches!(self, Self::Safe | Self::Balanced | Self::Aggressive)
    }

    #[deprecated(note = "use supported_without_agent")]
    pub fn supported_in_m1(self) -> bool {
        self.supported_without_agent()
    }
}

/// Controls a single benchmark / baseline run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfig {
    /// Warm-up iterations discarded before sampling.
    pub warmup: usize,
    /// Measured samples after warm-up.
    pub samples: usize,
    /// Random seed for deterministic benches that need one.
    pub seed: u64,
    /// Max wall-clock seconds for a full suite (soft budget).
    pub time_budget_secs: u64,
    /// Include storage benches (writes only under Kraftverk temp dirs).
    pub include_storage: bool,
    /// Include system/scheduler proxies.
    pub include_system: bool,
    /// Include compile-throughput characterization (v2).
    pub include_compile: bool,
    /// Include responsiveness index (v2).
    pub include_responsiveness: bool,
    /// Include CPU scaling characterization (v2).
    pub include_scaling: bool,
    /// Sustained run duration in seconds (0 = off).
    pub sustained_secs: u64,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            warmup: 2,
            samples: 5,
            seed: 42,
            time_budget_secs: 300,
            include_storage: true,
            include_system: true,
            include_compile: false,
            include_responsiveness: false,
            include_scaling: false,
            sustained_secs: 0,
        }
    }
}

/// Controls an optimization loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizeConfig {
    pub mode: OptimizeMode,
    pub goal: OptimizeGoal,
    pub seed: u64,
    /// Max candidate experiments (excluding baseline).
    pub max_experiments: usize,
    /// Wall-clock budget for the whole optimize session.
    pub time_budget_secs: u64,
    /// Stop after this many non-improving experiments.
    pub plateau_limit: usize,
    /// Samples per candidate during search.
    pub search_samples: usize,
    /// Extra validation samples for a provisional winner.
    pub validation_samples: usize,
    /// Max coefficient of variation (stddev/mean) allowed for PASS.
    pub max_cov: f64,
    pub constraints: OptimizeConstraints,
    pub run: RunConfig,
}

impl Default for OptimizeConfig {
    fn default() -> Self {
        Self {
            mode: OptimizeMode::Safe,
            goal: OptimizeGoal::Balanced,
            seed: 42,
            max_experiments: 12,
            time_budget_secs: 600,
            plateau_limit: 6,
            search_samples: 3,
            validation_samples: 5,
            max_cov: 0.15,
            constraints: OptimizeConstraints::default(),
            run: RunConfig::default(),
        }
    }
}
