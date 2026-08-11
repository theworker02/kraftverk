//! Compile-throughput characterization (synthetic, deterministic).

use std::time::Instant;

use kraftverk_core::measurement::{BenchmarkId, Measurement, MetricDirection};
use rayon::prelude::*;
use sha2::{Digest, Sha256};

use crate::workload_cfg::WorkloadConfig;

pub fn run_all(cfg: &WorkloadConfig) -> Vec<Measurement> {
    vec![compile_throughput(cfg)]
}

fn compile_throughput(cfg: &WorkloadConfig) -> Measurement {
    let n = 48_000usize;
    let threads = cfg.rayon_threads.max(1);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .ok();

    let start = Instant::now();
    let digest = if let Some(pool) = pool.as_ref() {
        pool.install(|| work(n, cfg.seed))
    } else {
        work(n, cfg.seed)
    };
    let elapsed = start.elapsed().as_secs_f64().max(1e-9);
    let ops = n as f64 / elapsed;

    Measurement {
        id: BenchmarkId::new("compile.throughput_proxy"),
        category: "cpu".into(),
        score: Measurement::oriented_score(ops, MetricDirection::HigherIsBetter),
        raw_value: ops,
        unit: "ops/s".into(),
        direction: MetricDirection::HigherIsBetter,
        checksum: Some(hex::encode(&digest[..8])),
        notes: vec![
            format!("threads={threads}"),
            "Synthetic compile proxy — not wall-clock of rustc/MSVC.".into(),
        ],
    }
}

fn work(n: usize, seed: u64) -> [u8; 32] {
    let partials: Vec<[u8; 32]> = (0..n)
        .into_par_iter()
        .map(|i| {
            let mut hasher = Sha256::new();
            hasher.update(seed.to_le_bytes());
            hasher.update((i as u64).to_le_bytes());
            let mut buf = [0u8; 64];
            for (j, b) in buf.iter_mut().enumerate() {
                *b = ((i.wrapping_mul(131) + j.wrapping_mul(17) + seed as usize) % 97) as u8;
            }
            let digits = buf.iter().filter(|&&b| b < 10).count();
            hasher.update(digits.to_le_bytes());
            hasher.update(buf);
            let out = hasher.finalize();
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&out);
            arr
        })
        .collect();

    let mut hasher = Sha256::new();
    for p in partials {
        hasher.update(p);
    }
    let out = hasher.finalize();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&out);
    arr
}
