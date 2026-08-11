//! Optimization: search strategies, profiles, objectives, and lineage.
//!
//! Consolidated from kraftverk-search and kraftverk-profiles.

pub mod hill_climb;
pub mod lineage;
pub mod objectives;
pub mod plugins;
pub mod profiles;
pub mod strategy;

pub use hill_climb::HillClimbStrategy;
pub use lineage::{build_lineage, LineageNode, LineageTree};
pub use objectives::{score_objective, ObjectiveScore, ParamImportance};
pub use plugins::{
    create_search_plugin, default_plugin_for_mode, list_search_plugins, SearchPluginInfo,
    PLUGIN_HILL_CLIMB, PLUGIN_SAFE_HILL_CLIMB,
};
pub use profiles::{list_profiles, recommend_profile, KraftProfile, ProfileInfo};
pub use strategy::{SearchContext, SearchDecision, SearchStrategy};
