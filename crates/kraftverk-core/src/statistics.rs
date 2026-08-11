//! Statistical summaries and A/B comparison.

use serde::{Deserialize, Serialize};

use crate::classification::ComparisonClass;
use crate::error::{Error, Result};

/// Configuration for statistical analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsConfig {
    /// Confidence level for mean CI (e.g. 0.95).
    pub confidence: f64,
    /// Absolute relative change below which we call it noise (when CI overlaps).
    pub indifference_threshold: f64,
    /// IQR multiplier for outlier flagging.
    pub outlier_iqr_k: f64,
}

impl Default for StatsConfig {
    fn default() -> Self {
        Self {
            confidence: 0.95,
            indifference_threshold: 0.005,
            outlier_iqr_k: 1.5,
        }
    }
}

/// Summary statistics for a sample vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleSummary {
    pub n: usize,
    pub mean: f64,
    pub median: f64,
    pub min: f64,
    pub max: f64,
    pub stddev: f64,
    pub variance: f64,
    pub cov: f64,
    pub p10: f64,
    pub p25: f64,
    pub p75: f64,
    pub p90: f64,
    pub p95: f64,
    pub ci_low: f64,
    pub ci_high: f64,
    pub outlier_count: usize,
}

/// Result of comparing candidate samples against baseline samples.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResult {
    pub class: ComparisonClass,
    pub relative_change: f64,
    pub absolute_change: f64,
    pub baseline: SampleSummary,
    pub candidate: SampleSummary,
    pub explanation: String,
}

/// Compute summary statistics for `samples` (must be non-empty).
pub fn summarize(samples: &[f64], cfg: &StatsConfig) -> Result<SampleSummary> {
    if samples.is_empty() {
        return Err(Error::Statistics("empty sample set".into()));
    }
    let n = samples.len();
    let mut sorted = samples.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let mean = samples.iter().sum::<f64>() / n as f64;
    let variance = if n > 1 {
        samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1) as f64
    } else {
        0.0
    };
    let stddev = variance.sqrt();
    let cov = if mean.abs() > f64::EPSILON {
        stddev / mean.abs()
    } else {
        0.0
    };

    let median = percentile_sorted(&sorted, 50.0);
    let p10 = percentile_sorted(&sorted, 10.0);
    let p25 = percentile_sorted(&sorted, 25.0);
    let p75 = percentile_sorted(&sorted, 75.0);
    let p90 = percentile_sorted(&sorted, 90.0);
    let p95 = percentile_sorted(&sorted, 95.0);

    let iqr = p75 - p25;
    let lo = p25 - cfg.outlier_iqr_k * iqr;
    let hi = p75 + cfg.outlier_iqr_k * iqr;
    let outlier_count = samples.iter().filter(|&&x| x < lo || x > hi).count();

    // Approximate normal CI using t-ish z for large n; for small n use 1.96 as pragmatic M1 bound.
    let z = z_for_confidence(cfg.confidence);
    let se = if n > 0 {
        stddev / (n as f64).sqrt()
    } else {
        0.0
    };
    let ci_low = mean - z * se;
    let ci_high = mean + z * se;

    Ok(SampleSummary {
        n,
        mean,
        median,
        min: *sorted.first().unwrap(),
        max: *sorted.last().unwrap(),
        stddev,
        variance,
        cov,
        p10,
        p25,
        p75,
        p90,
        p95,
        ci_low,
        ci_high,
        outlier_count,
    })
}

/// Compare candidate to baseline. Both vectors are higher-is-better scores.
pub fn compare_samples(
    baseline: &[f64],
    candidate: &[f64],
    cfg: &StatsConfig,
) -> Result<ComparisonResult> {
    let b = summarize(baseline, cfg)?;
    let c = summarize(candidate, cfg)?;

    let absolute_change = c.mean - b.mean;
    let relative_change = if b.mean.abs() > f64::EPSILON {
        absolute_change / b.mean
    } else {
        0.0
    };

    let cis_overlap = c.ci_low <= b.ci_high && b.ci_low <= c.ci_high;
    let unstable = b.cov > 0.25 || c.cov > 0.25 || b.n < 2 || c.n < 2;

    let class = if unstable {
        ComparisonClass::UnstableResult
    } else if !cis_overlap && relative_change > cfg.indifference_threshold {
        if relative_change >= 0.03 && c.ci_low > b.ci_high {
            ComparisonClass::ConfirmedImprovement
        } else {
            ComparisonClass::LikelyImprovement
        }
    } else if !cis_overlap && relative_change < -cfg.indifference_threshold {
        if relative_change <= -0.03 && c.ci_high < b.ci_low {
            ComparisonClass::ConfirmedRegression
        } else {
            ComparisonClass::LikelyRegression
        }
    } else if relative_change.abs() < cfg.indifference_threshold || cis_overlap {
        ComparisonClass::NoSignificantChange
    } else if relative_change > 0.0 {
        ComparisonClass::LikelyImprovement
    } else {
        ComparisonClass::LikelyRegression
    };

    let explanation = format!(
        "baseline mean={:.4} CI[{:.4},{:.4}] cov={:.3}; candidate mean={:.4} CI[{:.4},{:.4}] cov={:.3}; Δ={:+.2}% → {}",
        b.mean,
        b.ci_low,
        b.ci_high,
        b.cov,
        c.mean,
        c.ci_low,
        c.ci_high,
        c.cov,
        relative_change * 100.0,
        class.as_str()
    );

    Ok(ComparisonResult {
        class,
        relative_change,
        absolute_change,
        baseline: b,
        candidate: c,
        explanation,
    })
}

fn percentile_sorted(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let rank = (p / 100.0) * (sorted.len() - 1) as f64;
    let lo = rank.floor() as usize;
    let hi = rank.ceil() as usize;
    if lo == hi {
        sorted[lo]
    } else {
        let w = rank - lo as f64;
        sorted[lo] * (1.0 - w) + sorted[hi] * w
    }
}

fn z_for_confidence(c: f64) -> f64 {
    // Common levels; default 1.96 for ~95%.
    if (c - 0.99).abs() < 0.001 {
        2.576
    } else if (c - 0.90).abs() < 0.001 {
        1.645
    } else {
        1.96
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_basic() {
        let s = summarize(&[1.0, 2.0, 3.0, 4.0, 5.0], &StatsConfig::default()).unwrap();
        assert_eq!(s.n, 5);
        assert!((s.mean - 3.0).abs() < 1e-9);
        assert!((s.median - 3.0).abs() < 1e-9);
        assert!((s.min - 1.0).abs() < 1e-9);
        assert!((s.max - 5.0).abs() < 1e-9);
    }

    #[test]
    fn confirmed_improvement() {
        let baseline = vec![100.0, 101.0, 99.0, 100.5, 100.2];
        let candidate = vec![110.0, 111.0, 109.5, 110.2, 110.8];
        let r = compare_samples(&baseline, &candidate, &StatsConfig::default()).unwrap();
        assert!(matches!(
            r.class,
            ComparisonClass::ConfirmedImprovement | ComparisonClass::LikelyImprovement
        ));
        assert!(r.relative_change > 0.05);
    }

    #[test]
    fn no_change_on_noise() {
        let baseline = vec![100.0, 100.1, 99.9, 100.05, 99.95];
        let candidate = vec![100.02, 99.98, 100.01, 100.0, 99.99];
        let r = compare_samples(&baseline, &candidate, &StatsConfig::default()).unwrap();
        assert_eq!(r.class, ComparisonClass::NoSignificantChange);
    }

    #[test]
    fn unstable_high_cov() {
        let baseline = vec![10.0, 100.0, 50.0];
        let candidate = vec![12.0, 90.0, 40.0];
        let r = compare_samples(&baseline, &candidate, &StatsConfig::default()).unwrap();
        assert_eq!(r.class, ComparisonClass::UnstableResult);
    }
}
