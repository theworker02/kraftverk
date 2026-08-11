//! Real-world-ish workloads: compression, hashing pipelines, parsing, parallel tasks.

use std::time::Instant;

use kraftverk_core::measurement::{BenchmarkId, Measurement, MetricDirection};
use rayon::prelude::*;
use sha2::{Digest, Sha256};

use crate::pool;
use crate::workload_cfg::WorkloadConfig;

pub fn run_all(cfg: &WorkloadConfig) -> Vec<Measurement> {
    vec![file_processing(cfg), parallel_reduce(cfg), parse_lines(cfg)]
}

fn file_processing(cfg: &WorkloadConfig) -> Measurement {
    let files = 256usize;
    let size = 8192usize;
    let payloads: Vec<Vec<u8>> = (0..files)
        .map(|i| {
            let mut v = vec![0u8; size];
            let mut s = cfg.seed.wrapping_add(i as u64);
            for b in v.iter_mut() {
                s = s.wrapping_mul(1664525).wrapping_add(1013904223);
                *b = (s >> 24) as u8;
            }
            v
        })
        .collect();

    let pool = pool::pool(cfg).ok();
    let start = Instant::now();
    let digests: Vec<[u8; 32]> = if let Some(pool) = pool.as_ref() {
        pool.install(|| {
            payloads
                .par_iter()
                .map(|p| {
                    let compressed = cheap_compress(p);
                    let mut h = Sha256::new();
                    h.update(&compressed);
                    let out = h.finalize();
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&out);
                    arr
                })
                .collect()
        })
    } else {
        payloads
            .iter()
            .map(|p| {
                let compressed = cheap_compress(p);
                let mut h = Sha256::new();
                h.update(&compressed);
                let out = h.finalize();
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&out);
                arr
            })
            .collect()
    };
    let elapsed = start.elapsed().as_secs_f64().max(1e-12);
    let bytes = (files * size) as f64;
    let bps = bytes / elapsed;
    let mut fold = Sha256::new();
    for d in &digests {
        fold.update(d);
    }
    Measurement {
        id: BenchmarkId::new("realtime.file_processing"),
        category: "realtime".into(),
        score: bps,
        raw_value: bps,
        unit: "B/s".into(),
        direction: MetricDirection::HigherIsBetter,
        checksum: Some(hex::encode(fold.finalize())[..16].to_string()),
        notes: vec!["in-memory compress+hash pipeline".into()],
    }
}

fn parallel_reduce(cfg: &WorkloadConfig) -> Measurement {
    let n = 5_000_000usize;
    let pool = pool::pool(cfg).ok();
    let start = Instant::now();
    let sum: u64 = if let Some(pool) = pool.as_ref() {
        pool.install(|| {
            (0..n)
                .into_par_iter()
                .map(|i| {
                    let mut x = (i as u64).wrapping_add(cfg.seed);
                    x = x.wrapping_mul(0x9E3779B97F4A7C15);
                    x ^ (x >> 16)
                })
                .reduce(|| 0u64, |a, b| a.wrapping_add(b))
        })
    } else {
        (0..n)
            .map(|i| {
                let mut x = (i as u64).wrapping_add(cfg.seed);
                x = x.wrapping_mul(0x9E3779B97F4A7C15);
                x ^ (x >> 16)
            })
            .fold(0u64, |a, b| a.wrapping_add(b))
    };
    let elapsed = start.elapsed().as_secs_f64().max(1e-12);
    let rate = n as f64 / elapsed;
    Measurement {
        id: BenchmarkId::new("realtime.parallel_reduce"),
        category: "realtime".into(),
        score: rate,
        raw_value: rate,
        unit: "items/s".into(),
        direction: MetricDirection::HigherIsBetter,
        checksum: Some(format!("{sum:x}")),
        notes: vec![],
    }
}

fn parse_lines(cfg: &WorkloadConfig) -> Measurement {
    let lines = 50_000usize;
    let mut text = String::with_capacity(lines * 40);
    for i in 0..lines {
        text.push_str(&format!(
            "ts={} level=INFO msg=event_{} value={}\n",
            cfg.seed + i as u64,
            i,
            (i as u64).wrapping_mul(17)
        ));
    }
    let start = Instant::now();
    let mut count = 0u64;
    let mut sum = 0u64;
    for line in text.lines() {
        if let Some(v) = line.split("value=").nth(1) {
            if let Ok(n) = v.trim().parse::<u64>() {
                sum = sum.wrapping_add(n);
                count += 1;
            }
        }
    }
    let elapsed = start.elapsed().as_secs_f64().max(1e-12);
    let rate = count as f64 / elapsed;
    Measurement {
        id: BenchmarkId::new("realtime.parse_lines"),
        category: "realtime".into(),
        score: rate,
        raw_value: rate,
        unit: "lines/s".into(),
        direction: MetricDirection::HigherIsBetter,
        checksum: Some(format!("{sum:x}")),
        notes: vec![],
    }
}

fn cheap_compress(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        let b = data[i];
        let mut run = 1usize;
        while i + run < data.len() && data[i + run] == b && run < 255 {
            run += 1;
        }
        out.push(run as u8);
        out.push(b);
        i += run;
    }
    out
}
