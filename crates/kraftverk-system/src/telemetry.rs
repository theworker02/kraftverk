//! Expanded telemetry: environment awareness and measurement noise model.
//!
//! Telemetry informs stability/safety decisions. It is never used to invent
//! benchmark scores.

use serde::{Deserialize, Serialize};
use sysinfo::System;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySnapshot {
    pub timestamp_rfc3339: String,
    pub cpu_usage_pct: f32,
    pub mem_used_bytes: u64,
    pub mem_total_bytes: u64,
    pub load_hint: String,
    /// CPU package temperature when a portable source exists; else None.
    pub temp_c: Option<f64>,
    /// Estimated package power when available; else None.
    pub power_w: Option<f64>,
    /// Process count hint for environmental noise.
    pub process_count: usize,
    pub noise: NoiseEstimate,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseEstimate {
    /// Qualitative label derived from load + contention proxies.
    pub level: String,
    /// 0.0 = quiet lab, 1.0 = highly contended (heuristic, not a score).
    pub score: f64,
    pub reasons: Vec<String>,
}

pub fn capture_snapshot() -> TelemetrySnapshot {
    let mut sys = System::new_all();
    sys.refresh_cpu_usage();
    sys.refresh_memory();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    std::thread::sleep(std::time::Duration::from_millis(50));
    sys.refresh_cpu_usage();

    let cpu = sys.global_cpu_usage();
    let process_count = sys.processes().len();
    let mem_ratio = if sys.total_memory() > 0 {
        sys.used_memory() as f64 / sys.total_memory() as f64
    } else {
        0.0
    };

    let mut reasons = Vec::new();
    let mut noise_score: f64 = 0.0;
    if cpu > 85.0 {
        noise_score += 0.35;
        reasons.push("high CPU utilization".into());
    } else if cpu > 50.0 {
        noise_score += 0.15;
        reasons.push("moderate CPU utilization".into());
    }
    if mem_ratio > 0.9 {
        noise_score += 0.25;
        reasons.push("memory pressure".into());
    } else if mem_ratio > 0.75 {
        noise_score += 0.1;
        reasons.push("elevated memory use".into());
    }
    if process_count > 400 {
        noise_score += 0.2;
        reasons.push(format!("high process count ({process_count})"));
    } else if process_count > 250 {
        noise_score += 0.1;
        reasons.push(format!("elevated process count ({process_count})"));
    }
    noise_score = noise_score.clamp(0.0, 1.0);
    let level = if noise_score >= 0.6 {
        "high"
    } else if noise_score >= 0.3 {
        "moderate"
    } else {
        "low"
    };

    let mut notes =
        vec!["Snapshot is informational; never used to fabricate benchmark scores.".into()];

    // Portable thermal/power: unavailable without vendor backends.
    let temp_c = None;
    let power_w = None;
    notes.push(
        "Temperature/power sensors: unsupported portably (no vendor backend); constraints remain unchecked."
            .into(),
    );

    TelemetrySnapshot {
        timestamp_rfc3339: chrono::Utc::now().to_rfc3339(),
        cpu_usage_pct: cpu,
        mem_used_bytes: sys.used_memory(),
        mem_total_bytes: sys.total_memory(),
        load_hint: if cpu > 90.0 {
            "high".into()
        } else if cpu > 50.0 {
            "moderate".into()
        } else {
            "low".into()
        },
        temp_c,
        power_w,
        process_count,
        noise: NoiseEstimate {
            level: level.into(),
            score: noise_score,
            reasons,
        },
        notes,
    }
}

pub fn snapshot_json() -> serde_json::Value {
    serde_json::to_value(capture_snapshot()).unwrap_or(serde_json::json!({}))
}

/// Whether the environment is quiet enough for sensitive comparisons.
pub fn environment_suitable_for_bench(snap: &TelemetrySnapshot) -> bool {
    snap.noise.score < 0.7
}
