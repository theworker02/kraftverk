//! Memory bandwidth / access-pattern proxies.

use std::time::Instant;

use kraftverk_core::measurement::{BenchmarkId, Measurement, MetricDirection};
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::workload_cfg::WorkloadConfig;

pub fn run_all(cfg: &WorkloadConfig) -> Vec<Measurement> {
    vec![mem_sequential(cfg), mem_random(cfg), mem_allocation(cfg)]
}

fn mem_sequential(cfg: &WorkloadConfig) -> Measurement {
    let size = 8 << 20; // 8 MiB
    let mut buf = vec![0u64; size / 8];
    let mut state = cfg.seed;
    for v in buf.iter_mut() {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        *v = state;
    }
    let start = Instant::now();
    let mut acc = 0u64;
    for _ in 0..4 {
        for v in &buf {
            acc = acc.wrapping_add(*v);
        }
        for v in buf.iter_mut() {
            *v = v.wrapping_mul(3).wrapping_add(acc);
        }
    }
    let elapsed = start.elapsed().as_secs_f64().max(1e-12);
    let bytes = (size * 4 * 2) as f64; // read+write passes
    let bps = bytes / elapsed;
    Measurement {
        id: BenchmarkId::new("memory.sequential"),
        category: "memory".into(),
        score: bps,
        raw_value: bps,
        unit: "B/s".into(),
        direction: MetricDirection::HigherIsBetter,
        checksum: Some(format!("{acc:x}")),
        notes: vec!["proxy bandwidth via sequential R/W".into()],
    }
}

fn mem_random(cfg: &WorkloadConfig) -> Measurement {
    let n = 1 << 20; // 1M entries
    let mut buf = vec![0u64; n];
    let mut rng = ChaCha8Rng::seed_from_u64(cfg.seed);
    for v in buf.iter_mut() {
        *v = rng.gen();
    }
    // Build pointer-chasing permutation
    let mut idx: Vec<usize> = (0..n).collect();
    for i in (1..n).rev() {
        let j = rng.gen_range(0..=i);
        idx.swap(i, j);
    }
    let start = Instant::now();
    let mut i = 0usize;
    let mut acc = 0u64;
    for _ in 0..(n * 2) {
        i = idx[i];
        acc = acc.wrapping_add(buf[i]);
    }
    let elapsed = start.elapsed().as_secs_f64().max(1e-12);
    let ops = (n * 2) as f64 / elapsed;
    Measurement {
        id: BenchmarkId::new("memory.random"),
        category: "memory".into(),
        score: ops,
        raw_value: ops,
        unit: "lookups/s".into(),
        direction: MetricDirection::HigherIsBetter,
        checksum: Some(format!("{acc:x}")),
        notes: vec!["random pointer-chase latency proxy".into()],
    }
}

fn mem_allocation(cfg: &WorkloadConfig) -> Measurement {
    let count = 20_000usize;
    let start = Instant::now();
    let mut held = Vec::with_capacity(count);
    let mut checksum = cfg.seed;
    for i in 0..count {
        let sz = 64 + (i % 256);
        let mut v = vec![(i as u8).wrapping_add(cfg.seed as u8); sz];
        checksum = checksum.wrapping_add(v.iter().map(|&b| b as u64).sum::<u64>());
        // Touch and keep briefly
        v[0] ^= 1;
        held.push(v);
        if held.len() > 1000 {
            held.drain(0..500);
        }
    }
    drop(held);
    let elapsed = start.elapsed().as_secs_f64().max(1e-12);
    let rate = count as f64 / elapsed;
    Measurement {
        id: BenchmarkId::new("memory.allocation"),
        category: "memory".into(),
        score: rate,
        raw_value: rate,
        unit: "allocs/s".into(),
        direction: MetricDirection::HigherIsBetter,
        checksum: Some(format!("{checksum:x}")),
        notes: vec![],
    }
}
