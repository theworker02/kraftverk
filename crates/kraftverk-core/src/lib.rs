//! Kraftverk core: domain models, statistics, and Kraft Index.
//!
//! This crate contains no platform-specific code and no I/O beyond pure computation.

pub mod candidate;
pub mod classification;
pub mod config;
pub mod constraints;
pub mod error;
pub mod experiment;
pub mod goals;
pub mod kraft_index;
pub mod measurement;
pub mod session;
pub mod statistics;

pub use candidate::{Candidate, ParamChange, ParamValue};
pub use classification::ComparisonClass;
pub use config::{OptimizeConfig, OptimizeMode, RunConfig};
pub use constraints::{ConstraintCheck, OptimizeConstraints};
pub use error::{Error, Result};
pub use experiment::{
    Decision, Experiment, ExperimentId, ExperimentKind, ExperimentStatus, StabilityVerdict,
};
pub use goals::OptimizeGoal;
pub use kraft_index::{KraftIndex, KraftIndexWeights, BASELINE_INDEX};
pub use measurement::{BenchmarkId, Measurement, MeasurementSet, MetricDirection};
pub use session::{OptimizeCheckpoint, OptimizeSession, SessionId, SessionStatus};
pub use statistics::{compare_samples, summarize, ComparisonResult, SampleSummary, StatsConfig};
