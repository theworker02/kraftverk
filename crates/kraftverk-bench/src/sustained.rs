//! Sustained performance characterization over a wall-clock window.

use std::time::{Duration, Instant};

use kraftverk_core::measurement::{BenchmarkId, Measurement, MetricDirection};
use sha2::{Digest, Sha256};

use crate::workload_cfg::WorkloadConfig;

pub fn run_all(cfg: &WorkloadConfig) -> Vec<Measurement> {
    if cfg.sustained_secs == 0 {
        return Vec::new();
    }
    vec![sustained_window(cfg)]
}

fn sustained_window(cfg: &WorkloadConfig) -> Measurement {
    let budget = Duration::from_secs(cfg.sustained_secs.max(1));
    let slice = Duration::from_millis(500);
    let start = Instant::now();
    let mut slice_scores = Vec::new();
    let mut iter = 0u64;

    while start.elapsed() < budget {
        let slice_start = Instant::now();
        let mut ops = 0u64;
        while slice_start.elapsed() < slice && start.elapsed() < budget {
            let mut hasher = Sha256::new();
            hasher.update(cfg.seed.to_le_bytes());
            hasher.update(iter.to_le_bytes());
            hasher.update(ops.to_le_bytes());
            std::hint::black_box(hasher.finalize());
            ops += 1;
            iter += 1;
        }
        let elapsed = slice_start.elapsed().as_secs_f64().max(1e-9);
        slice_scores.push(ops as f64 / elapsed);
    }

    let mean = if slice_scores.is_empty() {
        0.0
    } else {
        slice_scores.iter().sum::<f64>() / slice_scores.len() as f64
    };
    let first = slice_scores.first().copied().unwrap_or(mean).max(1e-9);
    let last = slice_scores.last().copied().unwrap_or(mean);
    let retention = last / first;
    let score = mean * retention.max(0.0);

    Measurement {
        id: BenchmarkId::new("sustained.window"),
        category: "cpu".into(),
        score: Measurement::oriented_score(score, MetricDirection::HigherIsBetter),
        raw_value: mean,
        unit: "ops/s".into(),
        direction: MetricDirection::HigherIsBetter,
        checksum: Some(format!(
            "slices={};retention={retention:.4}",
            slice_scores.len()
        )),
        notes: vec![
            format!("duration_secs={}", cfg.sustained_secs),
            "Real sustained load; thermal throttling reduces retention if present.".into(),
        ],
    }
}

/// Parse CLI duration like `10m`, `30s`, `1h` into seconds.
pub fn parse_duration_to_secs(s: &str) -> Option<u64> {
    let s = s.trim().to_ascii_lowercase();
    if s.is_empty() {
        return None;
    }
    if let Ok(v) = s.parse::<u64>() {
        return Some(v);
    }
    let (num, unit) = s.split_at(s.len().saturating_sub(1));
    let n: u64 = num.parse().ok()?;
    match unit {
        "s" => Some(n),
        "m" => Some(n.saturating_mul(60)),
        "h" => Some(n.saturating_mul(3600)),
        _ => None,
    }
}
