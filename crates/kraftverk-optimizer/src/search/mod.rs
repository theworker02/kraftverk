//! Search strategy plugins (ε-greedy, Bayesian, …).

pub mod bayesian;
pub mod epsilon_greedy;

pub use bayesian::BayesianStrategy;
pub use epsilon_greedy::EpsilonGreedyStrategy;
