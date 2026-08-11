//! Named search strategy plugins (in-process registry; no dynamic loading).

use kraftverk_core::error::{Error, Result};

use crate::hill_climb::HillClimbStrategy;
use crate::strategy::SearchStrategy;

/// Built-in plugin identifiers.
pub const PLUGIN_HILL_CLIMB: &str = "hill-climb";
pub const PLUGIN_SAFE_HILL_CLIMB: &str = "safe-hill-climb";

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
            id: "epsilon-greedy".into(),
            name: "ε-greedy".into(),
            available: false,
            notes: "Planned Milestone 2 — not registered in 0.2.".into(),
        },
        SearchPluginInfo {
            id: "bayesian".into(),
            name: "Bayesian optimization".into(),
            available: false,
            notes: "Planned Milestone 2 — not registered in 0.2.".into(),
        },
    ]
}

/// Construct a strategy plugin by id.
pub fn create_search_plugin(id: &str, seed: u64) -> Result<Box<dyn SearchStrategy>> {
    match id {
        PLUGIN_HILL_CLIMB | PLUGIN_SAFE_HILL_CLIMB | "default" => {
            Ok(Box::new(HillClimbStrategy::new(seed)))
        }
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
    fn rejects_unavailable() {
        assert!(create_search_plugin("bayesian", 1).is_err());
    }
}
