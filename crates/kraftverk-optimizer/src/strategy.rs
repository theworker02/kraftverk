//! Search strategy trait.

use kraftverk_core::candidate::Candidate;
use kraftverk_core::classification::ComparisonClass;
use kraftverk_core::error::Result;
use kraftverk_system::Platform;

#[derive(Debug, Clone)]
pub struct SearchContext {
    pub generation: usize,
    pub experiments_done: usize,
    pub plateau_count: usize,
    pub best_score: f64,
    pub last_class: Option<ComparisonClass>,
    pub elapsed_secs: u64,
    pub max_experiments: usize,
    pub time_budget_secs: u64,
    pub plateau_limit: usize,
}

#[derive(Debug, Clone)]
pub enum SearchDecision {
    Try(Candidate),
    Stop { reason: String },
}

pub trait SearchStrategy: Send {
    fn name(&self) -> &str;
    fn seed(&self) -> u64;
    fn next_candidate(
        &mut self,
        platform: &dyn Platform,
        ctx: &SearchContext,
        current_best: &Candidate,
    ) -> Result<SearchDecision>;
}
