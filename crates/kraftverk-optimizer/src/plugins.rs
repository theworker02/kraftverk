//! Named search strategy plugins (in-process registry; no dynamic loading).

use kraftverk_core::error::{Error, Result};

use crate::hill_climb::HillClimbStrategy;
use crate::search::{BayesianStrategy, EpsilonGreedyStrategy};
use crate::strategy::SearchStrategy;

/// Built-in plugin identifiers.
pub const PLUGIN_HILL_CLIMB: &str = "hill-climb";
pub const PLUGIN_SAFE_HILL_CLIMB: &str = "safe-hill-climb";
pub const PLUGIN_EPSILON_GREEDY: &str = "epsilon-greedy";
pub const PLUGIN_BAYESIAN: &str = "bayesian";

#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchPluginInfo {
    pub id: String,
    pub name: String,
    pub available: bool,
    pub notes: String,
}

/// List registered search plugins.
pub fn list_search_plugins() -> Vec<SearchPluginInfo> {
    vec![
        SearchPluginInfo {
            id: PLUGIN_HILL_CLIMB.into(),
            name: "Deterministic hill-climb".into(),
            available: true,
            notes: "Discrete neighborhood search over reversible knobs; seedable.".into(),
        },
        SearchPluginInfo {
            id: PLUGIN_SAFE_HILL_CLIMB.into(),
            name: "Safe hill-climb (alias)".into(),
            available: true,
            notes: "Same as hill-climb; default for Safe mode.".into(),
        },
        SearchPluginInfo {
            id: PLUGIN_EPSILON_GREEDY.into(),
            name: "ε-greedy".into(),
            available: true,
            notes: "Multi-armed bandit over discrete arms; configurable decaying ε.".into(),
        },
        SearchPluginInfo {
            id: PLUGIN_BAYESIAN.into(),
            name: "Bayesian optimization".into(),
            available: true,
            notes: "GP surrogate + Expected Improvement over numeric thread params.".into(),
        },
    ]
}

/// Construct a strategy plugin by id.
pub fn create_search_plugin(id: &str, seed: u64) -> Result<Box<dyn SearchStrategy>> {
    match id {
        PLUGIN_HILL_CLIMB | PLUGIN_SAFE_HILL_CLIMB | "default" => {
            Ok(Box::new(HillClimbStrategy::new(seed)))
        }
        PLUGIN_EPSILON_GREEDY | "epsilon_greedy" | "egreedy" => {
            Ok(Box::new(EpsilonGreedyStrategy::new(seed)))
        }
        PLUGIN_BAYESIAN | "bo" => Ok(Box::new(BayesianStrategy::new(seed))),
        other => Err(Error::InvalidConfig(format!(
            "search plugin '{other}' is unavailable; try: {}",
            list_search_plugins()
                .into_iter()
                .filter(|p| p.available)
                .map(|p| p.id)
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

/// Default plugin for an optimize mode string.
pub fn default_plugin_for_mode(mode: &str) -> &'static str {
    match mode {
        "safe" | "balanced" | "aggressive" => PLUGIN_SAFE_HILL_CLIMB,
        _ => PLUGIN_HILL_CLIMB,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_hill_climb() {
        let p = create_search_plugin(PLUGIN_HILL_CLIMB, 1).unwrap();
        assert_eq!(p.name(), "hill_climb");
    }

    #[test]
    fn creates_epsilon_and_bayesian() {
        let e = create_search_plugin(PLUGIN_EPSILON_GREEDY, 1).unwrap();
        assert_eq!(e.name(), "epsilon_greedy");
        let b = create_search_plugin(PLUGIN_BAYESIAN, 1).unwrap();
        assert_eq!(b.name(), "bayesian");
    }
}
