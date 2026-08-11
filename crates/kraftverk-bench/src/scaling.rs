//! CPU scaling characterization across thread counts.

use std::time::Instant;

use kraftverk_core::measurement::{BenchmarkId, Measurement, MetricDirection};
use rayon::prelude::*;
use sha2::{Digest, Sha256};

use crate::workload_cfg::WorkloadConfig;

pub fn run_all(cfg: &WorkloadConfig) -> Vec<Measurement> {
    vec![cpu_scaling(cfg)]
}

fn cpu_scaling(cfg: &WorkloadConfig) -> Measurement {
    let logical = kraftverk_system::NativePlatform::worker_threads().max(1);
    let points: Vec<usize> = unique_points(logical);
    let mut results = Vec::new();

    for &threads in &points {
        let elapsed = timed_parallel(threads, cfg.seed, 8_000);
        let thruput = 8_000.0 / elapsed.max(1e-9);
        results.push((threads, thruput));
    }

    let base = results.first().map(|(_, t)| *t).unwrap_or(1.0).max(1e-9);
    let best = results.iter().map(|(_, t)| *t).fold(0.0_f64, f64::max);
    let efficiency = if let Some((t, thr)) = results.iter().max_by(|a, b| a.0.cmp(&b.0)) {
        let ideal = base * (*t as f64);
        thr / ideal.max(1e-9)
    } else {
        0.0
    };

    let score = best * efficiency.max(0.0);

    Measurement {
        id: BenchmarkId::new("cpu.scaling"),
        category: "cpu".into(),
        score: Measurement::oriented_score(score, MetricDirection::HigherIsBetter),
        raw_value: efficiency,
        unit: "scaled_ops".into(),
        direction: MetricDirection::HigherIsBetter,
        checksum: Some(
            results
                .iter()
                .map(|(t, thr)| format!("{t}:{thr:.1}"))
                .collect::<Vec<_>>()
                .join(","),
        ),
        notes: vec![format!("efficiency={efficiency:.4}")],
    }
}

fn unique_points(logical: usize) -> Vec<usize> {
    let mut v = vec![1usize];
    let half = (logical / 2).max(1);
    if half != 1 {
        v.push(half);
    }
    if logical != half && logical != 1 {
        v.push(logical);
    }
    v.sort_unstable();
    v.dedup();
    v
}

fn timed_parallel(threads: usize, seed: u64, n: usize) -> f64 {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads.max(1))
        .build();
    let start = Instant::now();
    let run = || {
        (0..n).into_par_iter().for_each(|i| {
            let mut hasher = Sha256::new();
            hasher.update(seed.to_le_bytes());
            hasher.update((i as u64).to_le_bytes());
            std::hint::black_box(hasher.finalize());
        });
    };
    match pool {
        Ok(p) => p.install(run),
        Err(_) => run(),
    }
    start.elapsed().as_secs_f64()
}
