//! KraftBench v2 — deterministic, real workloads (no fabricated scores).

pub mod compile;
pub mod cpu;
pub mod gpu;
pub mod memory;
pub mod pool;
pub mod realtime;
pub mod responsiveness;
pub mod runner;
pub mod scaling;
pub mod storage;
pub mod sustained;
pub mod system;
pub mod workload_cfg;

pub use gpu::{probe_gpu_backend, run_all as run_gpu_all, GpuBackendStatus, GpuBenchResult};
pub use runner::{run_benchmark_suite, run_suite_samples, BenchProgress, SuiteResult};
pub use sustained::parse_duration_to_secs;
pub use workload_cfg::WorkloadConfig;
