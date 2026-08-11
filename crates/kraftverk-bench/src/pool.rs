//! Helpers to run parallel work on a configured local pool.

use kraftverk_core::error::{Error, Result};
use rayon::ThreadPoolBuilder;

use crate::workload_cfg::WorkloadConfig;

pub fn pool(cfg: &WorkloadConfig) -> Result<rayon::ThreadPool> {
    ThreadPoolBuilder::new()
        .num_threads(cfg.rayon_threads.max(1))
        .build()
        .map_err(|e| Error::Benchmark(format!("rayon pool: {e}")))
}
