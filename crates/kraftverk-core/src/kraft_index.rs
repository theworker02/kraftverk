//! Kraft Index: weighted composite score normalized so baseline = 10_000.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::measurement::MeasurementSet;

/// Canonical baseline Kraft Index value.
pub const BASELINE_INDEX: f64 = 10_000.0;

/// Category weights. Must sum to 1.0.
///
/// These weight *capability domains*, not raw microbenchmark averages.
/// GPU weight is 0 when no GPU measurements are present; use
/// [`KraftIndexWeights::with_gpu`] / [`for_measurements`] when real AMD GPU
/// benches produced scores.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KraftIndexWeights {
    pub cpu: f64,
    pub memory: f64,
    pub storage: f64,
    pub system: f64,
    pub realtime: f64,
    /// Non-zero only when real GPU measurements exist.
    pub gpu: f64,
}

impl Default for KraftIndexWeights {
    fn default() -> Self {
        Self {
            cpu: 0.40,
            memory: 0.20,
            storage: 0.15,
            system: 0.10,
            realtime: 0.15,
            gpu: 0.0,
        }
    }
}

impl KraftIndexWeights {
    /// Weights when real GPU measurements are present.
    pub fn with_gpu() -> Self {
        Self {
            cpu: 0.34,
            memory: 0.17,
            storage: 0.13,
            system: 0.09,
            realtime: 0.12,
            gpu: 0.15,
        }
    }

    /// Pick default or with_gpu based on whether any gpu-category score exists.
    pub fn for_measurements(set: &MeasurementSet) -> Self {
        if set.measurements.iter().any(|m| m.category == "gpu") {
            Self::with_gpu()
        } else {
            Self::default()
        }
    }

    pub fn validate(&self) -> Result<()> {
        let sum = self.cpu + self.memory + self.storage + self.system + self.realtime + self.gpu;
        if (sum - 1.0).abs() > 1e-6 {
            return Err(Error::InvalidConfig(format!(
                "Kraft Index weights must sum to 1.0, got {sum}"
            )));
        }
        Ok(())
    }

    pub fn weight_for_category(&self, category: &str) -> f64 {
        match category {
            "cpu" => self.cpu,
            "memory" => self.memory,
            "storage" => self.storage,
            "system" => self.system,
            "realtime" | "real_world" => self.realtime,
            "gpu" => self.gpu,
            _ => 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KraftIndex {
    /// Composite score (baseline-normalized when baseline provided).
    pub score: f64,
    /// Unnormalized geometric-ish category blend before baseline scaling.
    pub raw_composite: f64,
    pub category_scores: IndexMap<String, f64>,
    pub weights: KraftIndexWeights,
    /// Per-benchmark contribution details.
    pub components: IndexMap<String, f64>,
}

impl KraftIndex {
    /// Build an absolute (unnormalized) index from one measurement set.
    pub fn from_measurements(set: &MeasurementSet, weights: &KraftIndexWeights) -> Result<Self> {
        weights.validate()?;

        let mut category_acc: IndexMap<String, Vec<f64>> = IndexMap::new();
        let mut components = IndexMap::new();

        for m in &set.measurements {
            if m.score.is_finite() && m.score > 0.0 {
                category_acc
                    .entry(m.category.clone())
                    .or_default()
                    .push(m.score);
                components.insert(m.id.0.clone(), m.score);
            }
        }

        let mut category_scores = IndexMap::new();
        let mut weighted = 0.0;
        let mut used_weight = 0.0;

        for (cat, scores) in &category_acc {
            let w = weights.weight_for_category(cat);
            if w <= 0.0 || scores.is_empty() {
                continue;
            }
            // Geometric mean within category to avoid one microbench dominating.
            let geo = geometric_mean(scores);
            category_scores.insert(cat.clone(), geo);
            weighted += geo * w;
            used_weight += w;
        }

        if used_weight <= 0.0 {
            return Err(Error::Statistics(
                "no weighted categories available for Kraft Index".into(),
            ));
        }

        // Renormalize over categories that actually produced scores.
        let raw_composite = weighted / used_weight;

        Ok(Self {
            score: raw_composite,
            raw_composite,
            category_scores,
            weights: weights.clone(),
            components,
        })
    }

    /// Normalize so that `baseline_raw` maps to BASELINE_INDEX.
    pub fn normalize_to_baseline(&self, baseline_raw: f64) -> Result<Self> {
        if baseline_raw <= 0.0 || !baseline_raw.is_finite() {
            return Err(Error::Statistics(
                "baseline raw composite must be positive".into(),
            ));
        }
        let mut out = self.clone();
        out.score = (self.raw_composite / baseline_raw) * BASELINE_INDEX;
        Ok(out)
    }
}

fn geometric_mean(xs: &[f64]) -> f64 {
    let pos: Vec<f64> = xs
        .iter()
        .copied()
        .filter(|x| *x > 0.0 && x.is_finite())
        .collect();
    if pos.is_empty() {
        return 0.0;
    }
    let log_sum: f64 = pos.iter().map(|x| x.ln()).sum();
    (log_sum / pos.len() as f64).exp()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::measurement::{BenchmarkId, Measurement, MetricDirection};

    fn m(id: &str, cat: &str, score: f64) -> Measurement {
        Measurement {
            id: BenchmarkId::new(id),
            category: cat.into(),
            score,
            raw_value: score,
            unit: "ops".into(),
            direction: MetricDirection::HigherIsBetter,
            checksum: None,
            notes: vec![],
        }
    }

    #[test]
    fn baseline_normalizes_to_10000() {
        let mut set = MeasurementSet::default();
        set.push(m("cpu.int", "cpu", 100.0));
        set.push(m("mem.bw", "memory", 200.0));
        set.push(m("stor.seq", "storage", 50.0));
        set.push(m("sys.wake", "system", 80.0));
        set.push(m("rw.hash", "realtime", 120.0));
        let w = KraftIndexWeights::default();
        let idx = KraftIndex::from_measurements(&set, &w).unwrap();
        let norm = idx.normalize_to_baseline(idx.raw_composite).unwrap();
        assert!((norm.score - BASELINE_INDEX).abs() < 1e-6);
    }
}
