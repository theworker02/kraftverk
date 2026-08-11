//! CPU benchmarks: integer, float, single/multi-thread, hashing, compression.

use std::time::Instant;

use kraftverk_core::measurement::{BenchmarkId, Measurement, MetricDirection};
use rayon::prelude::*;
use sha2::{Digest, Sha256};

use crate::pool;
use crate::workload_cfg::WorkloadConfig;

pub fn run_all(cfg: &WorkloadConfig) -> Vec<Measurement> {
    vec![
        cpu_integer_single(),
        cpu_float_single(),
        cpu_integer_multi(cfg),
        cpu_hashing(cfg),
        cpu_compression(cfg),
    ]
}

fn cpu_integer_single() -> Measurement {
    let iters = 5_000_000u64;
    let start = Instant::now();
    let mut acc: u64 = 1;
    for i in 0..iters {
        acc = acc
            .wrapping_mul(6364136223846793005)
            .wrapping_add(i ^ 0x9E3779B97F4A7C15);
        acc ^= acc >> 17;
    }
    let elapsed = start.elapsed().as_secs_f64().max(1e-12);
    let ops = iters as f64 / elapsed;
    Measurement {
        id: BenchmarkId::new("cpu.integer_single"),
        category: "cpu".into(),
        score: Measurement::oriented_score(ops, MetricDirection::HigherIsBetter),
        raw_value: ops,
        unit: "ops/s".into(),
        direction: MetricDirection::HigherIsBetter,
        checksum: Some(format!("{acc:x}")),
        notes: vec![],
    }
}

fn cpu_float_single() -> Measurement {
    let iters = 2_000_000u64;
    let start = Instant::now();
    let mut acc = 1.0f64;
    for i in 0..iters {
        let x = (i as f64) * 0.000001 + acc;
        acc = (x.sin() * x.cos() + 1.0000001).sqrt();
        if !acc.is_finite() {
            acc = 1.0;
        }
    }
    let elapsed = start.elapsed().as_secs_f64().max(1e-12);
    let ops = iters as f64 / elapsed;
    Measurement {
        id: BenchmarkId::new("cpu.float_single"),
        category: "cpu".into(),
        score: ops,
        raw_value: ops,
        unit: "ops/s".into(),
        direction: MetricDirection::HigherIsBetter,
        checksum: Some(format!("{acc:.8}")),
        notes: vec![],
    }
}

fn cpu_integer_multi(cfg: &WorkloadConfig) -> Measurement {
    let workers = cfg.worker_threads.max(1);
    let per = 1_000_000u64;
    let pool = pool::pool(cfg).ok();
    let start = Instant::now();
    let results: Vec<u64> = if let Some(pool) = pool.as_ref() {
        pool.install(|| {
            (0..workers)
                .into_par_iter()
                .map(|w| {
                    let mut acc: u64 = w as u64 + 1;
                    for i in 0..per {
                        acc = acc
                            .wrapping_mul(6364136223846793005)
                            .wrapping_add(i.wrapping_mul(0x85EBCA77C2B2AE63));
                        acc ^= acc >> 13;
                    }
                    acc
                })
                .collect()
        })
    } else {
        (0..workers)
            .map(|w| {
                let mut acc: u64 = w as u64 + 1;
                for i in 0..per {
                    acc = acc
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(i.wrapping_mul(0x85EBCA77C2B2AE63));
                    acc ^= acc >> 13;
                }
                acc
            })
            .collect()
    };
    let elapsed = start.elapsed().as_secs_f64().max(1e-12);
    let total_ops = (workers as u64 * per) as f64;
    let ops = total_ops / elapsed;
    let mut hasher = Sha256::new();
    for r in &results {
        hasher.update(r.to_le_bytes());
    }
    Measurement {
        id: BenchmarkId::new("cpu.integer_multi"),
        category: "cpu".into(),
        score: ops,
        raw_value: ops,
        unit: "ops/s".into(),
        direction: MetricDirection::HigherIsBetter,
        checksum: Some(hex::encode(hasher.finalize())[..16].to_string()),
        notes: vec![format!("workers={workers}")],
    }
}

fn cpu_hashing(cfg: &WorkloadConfig) -> Measurement {
    let blocks = 2_000usize;
    let block_size = 4096usize;
    let seed = cfg.seed;
    let pool = pool::pool(cfg).ok();
    let start = Instant::now();
    let digests: Vec<[u8; 32]> = if let Some(pool) = pool.as_ref() {
        pool.install(|| hash_blocks(blocks, block_size, seed))
    } else {
        hash_blocks(blocks, block_size, seed)
    };
    let elapsed = start.elapsed().as_secs_f64().max(1e-12);
    let bytes = (blocks * block_size) as f64;
    let bps = bytes / elapsed;
    let mut fold = Sha256::new();
    for d in &digests {
        fold.update(d);
    }
    Measurement {
        id: BenchmarkId::new("cpu.hashing"),
        category: "cpu".into(),
        score: bps,
        raw_value: bps,
        unit: "B/s".into(),
        direction: MetricDirection::HigherIsBetter,
        checksum: Some(hex::encode(fold.finalize())[..16].to_string()),
        notes: vec![],
    }
}

fn hash_blocks(blocks: usize, block_size: usize, seed: u64) -> Vec<[u8; 32]> {
    (0..blocks)
        .into_par_iter()
        .map(|i| {
            let mut buf = vec![0u8; block_size];
            for (j, b) in buf.iter_mut().enumerate() {
                *b = ((seed as usize).wrapping_mul(i + 1).wrapping_add(j) % 251) as u8;
            }
            let mut hasher = Sha256::new();
            hasher.update(&buf);
            let out = hasher.finalize();
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&out);
            arr
        })
        .collect()
}

fn cpu_compression(cfg: &WorkloadConfig) -> Measurement {
    let size = 1 << 20;
    let mut data = vec![0u8; size];
    let mut state = cfg.seed;
    for b in data.iter_mut() {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        *b = ((state >> 33) as u8).wrapping_add((state as u8) & 0x0f);
    }
    let start = Instant::now();
    let compressed = rle_compress(&data);
    let elapsed = start.elapsed().as_secs_f64().max(1e-12);
    let bps = size as f64 / elapsed;
    let mut hasher = Sha256::new();
    hasher.update((compressed.len() as u64).to_le_bytes());
    hasher.update(&compressed[..compressed.len().min(64)]);
    Measurement {
        id: BenchmarkId::new("cpu.compression"),
        category: "cpu".into(),
        score: bps,
        raw_value: bps,
        unit: "B/s".into(),
        direction: MetricDirection::HigherIsBetter,
        checksum: Some(hex::encode(hasher.finalize())[..16].to_string()),
        notes: vec![format!("compressed_len={}", compressed.len())],
    }
}

fn rle_compress(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() / 2);
    let mut i = 0;
    while i < data.len() {
        let b = data[i];
        let mut run = 1u8;
        while i + (run as usize) < data.len() && data[i + (run as usize)] == b && run < 255 {
            run += 1;
        }
        out.push(run);
        out.push(b ^ 0xA5);
        i += run as usize;
    }
    out
}
