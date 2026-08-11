//! AMD-focused GPU benchmarks via Vulkan (when available).
//!
//! Never invents scores. If no AMD Vulkan device / API is present, returns
//! `GpuBackendStatus::Unsupported` and the suite runner skips GPU category.

use kraftverk_core::measurement::{BenchmarkId, Measurement, MetricDirection};
use kraftverk_system::{detect_gpu_devices, GpuVendor};
use serde::{Deserialize, Serialize};

#[cfg(feature = "gpu")]
mod vulkan;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GpuBackendStatus {
    Available { device: String, api: String },
    Unsupported { reason: String },
}

#[derive(Debug, Clone)]
pub struct GpuBenchResult {
    pub status: GpuBackendStatus,
    pub measurements: Vec<Measurement>,
}

/// Probe whether an AMD GPU compute backend can run.
pub fn probe_gpu_backend() -> GpuBackendStatus {
    let amd = detect_gpu_devices()
        .into_iter()
        .filter(|g| g.vendor == GpuVendor::Amd)
        .collect::<Vec<_>>();
    if amd.is_empty() {
        return GpuBackendStatus::Unsupported {
            reason: "no AMD GPU (PCI 0x1002) detected".into(),
        };
    }
    #[cfg(feature = "gpu")]
    {
        vulkan::probe(&amd[0].name)
    }
    #[cfg(not(feature = "gpu"))]
    {
        let _ = amd;
        GpuBackendStatus::Unsupported {
            reason: "GPU feature disabled at compile time (build with --features gpu)".into(),
        }
    }
}

/// Run GPU suites when backend available; otherwise empty measurements + status.
pub fn run_all(seed: u64) -> GpuBenchResult {
    let status = probe_gpu_backend();
    match &status {
        GpuBackendStatus::Unsupported { .. } => GpuBenchResult {
            status,
            measurements: vec![],
        },
        GpuBackendStatus::Available { .. } => {
            #[cfg(feature = "gpu")]
            {
                match vulkan::run_suites(seed) {
                    Ok(ms) => GpuBenchResult {
                        status,
                        measurements: ms,
                    },
                    Err(reason) => GpuBenchResult {
                        status: GpuBackendStatus::Unsupported { reason },
                        measurements: vec![],
                    },
                }
            }
            #[cfg(not(feature = "gpu"))]
            {
                let _ = seed;
                GpuBenchResult {
                    status: GpuBackendStatus::Unsupported {
                        reason: "GPU feature disabled".into(),
                    },
                    measurements: vec![],
                }
            }
        }
    }
}

/// Helper used by unit tests / mocks without touching the GPU.
pub fn mock_measurement(id: &str, score: f64) -> Measurement {
    Measurement {
        id: BenchmarkId::new(id),
        category: "gpu".into(),
        score: Measurement::oriented_score(score, MetricDirection::HigherIsBetter),
        raw_value: score,
        unit: "ops/s".into(),
        direction: MetricDirection::HigherIsBetter,
        checksum: Some("mock".into()),
        notes: vec!["mock GPU measurement for unit tests".into()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_never_fabricates_available_without_backend() {
        let status = probe_gpu_backend();
        match status {
            GpuBackendStatus::Available { device, api } => {
                assert!(!device.is_empty());
                assert!(!api.is_empty());
            }
            GpuBackendStatus::Unsupported { reason } => {
                assert!(!reason.is_empty());
            }
        }
    }

    #[test]
    fn run_all_skips_cleanly_when_unsupported() {
        let result = run_all(42);
        if matches!(result.status, GpuBackendStatus::Unsupported { .. }) {
            assert!(result.measurements.is_empty());
        } else {
            // Real GPU path — measurements must be finite and positive.
            for m in &result.measurements {
                assert!(m.score.is_finite() && m.score > 0.0);
                assert_eq!(m.category, "gpu");
            }
        }
    }

    #[test]
    fn mock_measurement_category_gpu() {
        let m = mock_measurement("gpu.test", 123.0);
        assert_eq!(m.category, "gpu");
        assert!((m.score - 123.0).abs() < 1e-9);
    }
}
