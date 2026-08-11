//! Suite runner with warm-up and repeated samples.

use kraftverk_core::config::RunConfig;
use kraftverk_core::error::{Error, Result};
use kraftverk_core::kraft_index::{KraftIndex, KraftIndexWeights};
use kraftverk_core::measurement::MeasurementSet;
use kraftverk_core::statistics::{summarize, SampleSummary, StatsConfig};
use tracing::info;

use crate::compile;
use crate::cpu;
use crate::gpu;
use crate::memory;
use crate::realtime;
use crate::responsiveness;
use crate::scaling;
use crate::storage;
use crate::sustained;
use crate::system;
use crate::workload_cfg::WorkloadConfig;

pub trait BenchProgress {
    fn on_event(&mut self, message: &str);
}

pub struct NullProgress;
impl BenchProgress for NullProgress {
    fn on_event(&mut self, _message: &str) {}
}

#[derive(Debug, Clone)]
pub struct SuiteResult {
    pub samples: Vec<MeasurementSet>,
    pub index_samples_raw: Vec<f64>,
    pub baseline_raw: Option<f64>,
    pub index_samples_normalized: Vec<f64>,
    pub index_summary: Option<SampleSummary>,
    pub final_index: Option<KraftIndex>,
}

/// Run the full suite once (one sample).
pub fn run_benchmark_suite(cfg: &WorkloadConfig) -> Result<MeasurementSet> {
    let mut set = MeasurementSet::default();
    set.meta
        .insert("worker_threads".into(), cfg.worker_threads.to_string());
    set.meta
        .insert("rayon_threads".into(), cfg.rayon_threads.to_string());
    set.meta.insert("seed".into(), cfg.seed.to_string());

    for m in cpu::run_all(cfg) {
        set.push(m);
    }
    for m in memory::run_all(cfg) {
        set.push(m);
    }
    if cfg.include_storage {
        match storage::run_all(cfg) {
            Ok(ms) => {
                for m in ms {
                    set.push(m);
                }
            }
            Err(e) => {
                set.meta.insert("storage_error".into(), e.to_string());
            }
        }
    }
    if cfg.include_system {
        for m in system::run_all(cfg) {
            set.push(m);
        }
    }
    for m in realtime::run_all(cfg) {
        set.push(m);
    }
    if cfg.include_compile {
        for m in compile::run_all(cfg) {
            set.push(m);
        }
    }
    if cfg.include_responsiveness {
        for m in responsiveness::run_all(cfg) {
            set.push(m);
        }
    }
    if cfg.include_scaling {
        for m in scaling::run_all(cfg) {
            set.push(m);
        }
    }
    for m in sustained::run_all(cfg) {
        set.push(m);
    }

    // GPU: real AMD Vulkan backend when available — never invent scores.
    let gpu_result = gpu::run_all(cfg.seed);
    match &gpu_result.status {
        gpu::GpuBackendStatus::Available { device, api } => {
            set.meta
                .insert("gpu".into(), format!("available device={device} api={api}"));
            for m in gpu_result.measurements {
                set.push(m);
            }
        }
        gpu::GpuBackendStatus::Unsupported { reason } => {
            set.meta
                .insert("gpu".into(), format!("unsupported ({reason})"));
        }
    }

    if set.measurements.is_empty() {
        return Err(Error::Benchmark("no measurements produced".into()));
    }
    Ok(set)
}

/// Warm-up + repeated samples; optionally normalize to a baseline raw composite.
pub fn run_suite_samples(
    run: &RunConfig,
    workload: &WorkloadConfig,
    weights: &KraftIndexWeights,
    baseline_raw: Option<f64>,
    progress: &mut dyn BenchProgress,
    score_multiplier: f64,
) -> Result<SuiteResult> {
    for i in 0..run.warmup {
        progress.on_event(&format!("warm-up {}/{}", i + 1, run.warmup));
        let _ = run_benchmark_suite(workload)?;
    }

    let mut samples = Vec::with_capacity(run.samples);
    let mut index_samples_raw = Vec::with_capacity(run.samples);

    for i in 0..run.samples {
        progress.on_event(&format!("sample {}/{}", i + 1, run.samples));
        let mut set = run_benchmark_suite(workload)?;
        if (score_multiplier - 1.0).abs() > f64::EPSILON {
            for m in &mut set.measurements {
                m.score *= score_multiplier;
            }
        }
        // Prefer caller weights, but enable GPU weight automatically when real
        // AMD Vulkan measurements exist (never invent scores — only reweight).
        let sample_weights = {
            let auto = KraftIndexWeights::for_measurements(&set);
            if auto.gpu > 0.0 && weights.gpu == 0.0 {
                auto
            } else {
                weights.clone()
            }
        };
        let idx = KraftIndex::from_measurements(&set, &sample_weights)?;
        info!(sample = i + 1, raw = idx.raw_composite, "suite sample");
        index_samples_raw.push(idx.raw_composite);
        samples.push(set);
    }

    let index_samples_normalized: Vec<f64> = match baseline_raw {
        Some(b) => index_samples_raw
            .iter()
            .map(|r| (r / b) * kraftverk_core::BASELINE_INDEX)
            .collect(),
        None => index_samples_raw.clone(),
    };

    let stats_cfg = StatsConfig::default();
    let index_summary = summarize(&index_samples_normalized, &stats_cfg).ok();

    let final_index = if let Some(last) = samples.last() {
        let sample_weights = {
            let auto = KraftIndexWeights::for_measurements(last);
            if auto.gpu > 0.0 && weights.gpu == 0.0 {
                auto
            } else {
                weights.clone()
            }
        };
        let raw = KraftIndex::from_measurements(last, &sample_weights)?;
        if let Some(b) = baseline_raw {
            Some(raw.normalize_to_baseline(b)?)
        } else if index_summary.is_some() {
            let mean_raw = index_samples_raw.iter().sum::<f64>() / index_samples_raw.len() as f64;
            Some(raw.normalize_to_baseline(mean_raw)?)
        } else {
            Some(raw)
        }
    } else {
        None
    };

    Ok(SuiteResult {
        samples,
        index_samples_raw,
        baseline_raw,
        index_samples_normalized,
        index_summary,
        final_index,
    })
}
