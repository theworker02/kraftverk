//! Responsiveness index — latency-oriented microbench suite.

use std::time::Instant;

use kraftverk_core::measurement::{BenchmarkId, Measurement, MetricDirection};
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::workload_cfg::WorkloadConfig;

pub fn run_all(cfg: &WorkloadConfig) -> Vec<Measurement> {
    vec![responsiveness_index(cfg)]
}

fn responsiveness_index(cfg: &WorkloadConfig) -> Measurement {
    let mut rng = ChaCha8Rng::seed_from_u64(cfg.seed ^ 0x5245_5350);
    let iters = 2_000usize;
    let mut latencies_ns = Vec::with_capacity(iters);

    for _ in 0..iters {
        let work = rng.gen_range(50..200usize);
        let start = Instant::now();
        let mut x = cfg.seed;
        for i in 0..work {
            x = x.wrapping_mul(1664525).wrapping_add(1013904223) ^ (i as u64);
        }
        std::hint::black_box(x);
        latencies_ns.push(start.elapsed().as_nanos() as f64);
    }

    latencies_ns.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p50 = percentile(&latencies_ns, 0.50);
    let p95 = percentile(&latencies_ns, 0.95);
    let p99 = percentile(&latencies_ns, 0.99);
    let score = 1_000_000.0 / p95.max(1.0);

    Measurement {
        id: BenchmarkId::new("responsiveness.index"),
        category: "realtime".into(),
        score: Measurement::oriented_score(score, MetricDirection::HigherIsBetter),
        raw_value: p95,
        unit: "index".into(),
        direction: MetricDirection::HigherIsBetter,
        checksum: Some(format!("p50={p50:.0};p95={p95:.0};p99={p99:.0}")),
        notes: vec![format!("iters={iters}")],
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}
