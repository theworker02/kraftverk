//! Workload knobs read from the platform (worker/rayon counts).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadConfig {
    pub worker_threads: usize,
    pub rayon_threads: usize,
    pub seed: u64,
    /// Directory for storage benches (must be Kraftverk-owned temp).
    pub storage_dir: Option<std::path::PathBuf>,
    pub include_storage: bool,
    pub include_system: bool,
    pub include_compile: bool,
    pub include_responsiveness: bool,
    pub include_scaling: bool,
    /// Sustained characterization duration (seconds); 0 disables.
    pub sustained_secs: u64,
}

impl Default for WorkloadConfig {
    fn default() -> Self {
        let n = kraftverk_system::NativePlatform::worker_threads();
        Self {
            worker_threads: n,
            rayon_threads: kraftverk_system::NativePlatform::rayon_threads(),
            seed: 42,
            storage_dir: None,
            include_storage: true,
            include_system: true,
            include_compile: true,
            include_responsiveness: true,
            include_scaling: true,
            sustained_secs: 0,
        }
    }
}

impl WorkloadConfig {
    pub fn from_platform_params(seed: u64) -> Self {
        Self {
            worker_threads: kraftverk_system::NativePlatform::worker_threads(),
            rayon_threads: kraftverk_system::NativePlatform::rayon_threads(),
            seed,
            storage_dir: None,
            include_storage: true,
            include_system: true,
            include_compile: true,
            include_responsiveness: true,
            include_scaling: true,
            sustained_secs: 0,
        }
    }

    pub fn from_run_config(run: &kraftverk_core::RunConfig) -> Self {
        let mut w = Self::from_platform_params(run.seed);
        w.include_storage = run.include_storage;
        w.include_system = run.include_system;
        w.include_compile = run.include_compile;
        w.include_responsiveness = run.include_responsiveness;
        w.include_scaling = run.include_scaling;
        w.sustained_secs = run.sustained_secs;
        w
    }
}
