//! Optimization goals (v2).

use serde::{Deserialize, Serialize};

use crate::config::OptimizeMode;

/// High-level optimization goals that bias search and scoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OptimizeGoal {
    #[default]
    Balanced,
    Gaming,
    Compile,
    Workstation,
    Throughput,
    Latency,
    Efficiency,
    Sustained,
    Quiet,
}

impl OptimizeGoal {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Balanced => "balanced",
            Self::Gaming => "gaming",
            Self::Compile => "compile",
            Self::Workstation => "workstation",
            Self::Throughput => "throughput",
            Self::Latency => "latency",
            Self::Efficiency => "efficiency",
            Self::Sustained => "sustained",
            Self::Quiet => "quiet",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "balanced" => Some(Self::Balanced),
            "gaming" => Some(Self::Gaming),
            "compile" => Some(Self::Compile),
            "workstation" => Some(Self::Workstation),
            "throughput" => Some(Self::Throughput),
            "latency" => Some(Self::Latency),
            "efficiency" => Some(Self::Efficiency),
            "sustained" => Some(Self::Sustained),
            "quiet" => Some(Self::Quiet),
            _ => None,
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Balanced,
            Self::Gaming,
            Self::Compile,
            Self::Workstation,
            Self::Throughput,
            Self::Latency,
            Self::Efficiency,
            Self::Sustained,
            Self::Quiet,
        ]
    }

    /// Suggested optimize mode for this goal. Safe is always available;
    /// Balanced/Aggressive remain gated by platform capabilities.
    pub fn suggested_mode(self) -> OptimizeMode {
        match self {
            Self::Gaming | Self::Throughput => OptimizeMode::Aggressive,
            Self::Compile | Self::Workstation | Self::Sustained => OptimizeMode::Balanced,
            Self::Balanced | Self::Latency | Self::Efficiency | Self::Quiet => OptimizeMode::Safe,
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Balanced => "General-purpose mix of throughput and latency",
            Self::Gaming => "Favor responsiveness and frame-time proxies (GPU still optional)",
            Self::Compile => "Favor multi-core compile/throughput benches",
            Self::Workstation => "Mixed productivity workloads",
            Self::Throughput => "Maximize aggregate work rate",
            Self::Latency => "Minimize tail latency / responsiveness index",
            Self::Efficiency => "Performance per unit resource (workers/power proxies)",
            Self::Sustained => "Hold performance under longer sustained load",
            Self::Quiet => "Prefer lower intensity / quieter thermal proxies when available",
        }
    }
}
