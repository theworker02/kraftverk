//! Shared desktop API payloads (axum routes + Tauri commands).

use std::path::PathBuf;

use kraftverk_bench::{run_suite_samples, WorkloadConfig};
use kraftverk_core::{KraftIndexWeights, RunConfig};
use kraftverk_data::{default_db_path, ExperimentStore};
use kraftverk_system::{
    capture_snapshot, evaluate_eligibility, exit_code_for, inspect_machine, HardwareEligibility,
    HARDWARE_POLICY,
};
use serde_json::{json, Value};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn eligibility_json(startup: &HardwareEligibility) -> Value {
    let el = evaluate_eligibility();
    json!({
        "ok": true,
        "hardware_policy": HARDWARE_POLICY,
        "supported": el.supported,
        "eligibility": el,
        "startup_eligibility": startup,
        "exit_code": el.primary_rejection().map(|r| exit_code_for(r).as_i32()),
    })
}

pub fn blocked_json(el: &HardwareEligibility) -> Value {
    json!({
        "ok": false,
        "blocked": true,
        "hardware_policy": HARDWARE_POLICY,
        "error": el.summary(),
        "eligibility": el,
        "exit_code": el.primary_rejection().map(|r| exit_code_for(r).as_i32()),
    })
}

pub fn overview_json(version: &str) -> Value {
    let el = evaluate_eligibility();
    if !el.supported {
        return blocked_json(&el);
    }
    let report = inspect_machine(version);
    let store =
        ExperimentStore::open(default_db_path().unwrap_or_else(|_| PathBuf::from("kraftverk.db")));
    let (baseline, accepted, hist_n) = match store {
        Ok(s) => {
            let b = s.latest_baseline(&report.fingerprint).ok().flatten();
            let a = s.latest_accepted(&report.fingerprint).ok().flatten();
            let n = s
                .history(Some(&report.fingerprint), 50)
                .map(|h| h.len())
                .unwrap_or(0);
            (b, a, n)
        }
        Err(_) => (None, None, 0),
    };
    json!({
        "ok": true,
        "version": version,
        "hardware_policy": HARDWARE_POLICY,
        "eligibility": el,
        "fingerprint": report.fingerprint,
        "os": format!("{} {} ({})", report.os_family, report.os_version, report.arch),
        "baseline_id": baseline.as_ref().map(|e| e.id.to_string()),
        "baseline_score": baseline.as_ref().and_then(|e| e.kraft_index.as_ref().map(|k| k.score)),
        "accepted_id": accepted.as_ref().map(|e| e.id.to_string()),
        "history_count": hist_n,
        "philosophy": "Measure → Experiment → Benchmark → Validate → Compare → Keep or Revert → Learn → Repeat",
    })
}

pub fn status_json() -> Value {
    let el = evaluate_eligibility();
    let report = inspect_machine(VERSION);
    let path = default_db_path().ok();
    let active = path
        .as_ref()
        .and_then(|p| ExperimentStore::open(p).ok())
        .and_then(|s| s.active_config().ok().flatten());
    json!({
        "ok": true,
        "db": path.map(|p| p.display().to_string()),
        "fingerprint": report.fingerprint,
        "active_candidate": active,
        "hardware_policy": HARDWARE_POLICY,
        "eligibility": el,
        "agent": kraftverk_agent::trust_boundary_summary(),
        "agent_connected": kraftverk_agent::agent_connected(),
    })
}

pub fn telemetry_json() -> Value {
    json!({
        "ok": true,
        "snapshot": capture_snapshot(),
    })
}

pub fn history_json(limit: usize) -> Value {
    let el = evaluate_eligibility();
    if !el.supported {
        return blocked_json(&el);
    }
    let report = inspect_machine(VERSION);
    match ExperimentStore::open(default_db_path().unwrap_or_else(|_| PathBuf::from("kraftverk.db")))
    {
        Ok(s) => {
            let hist = s
                .history(Some(&report.fingerprint), limit)
                .unwrap_or_default();
            json!({"ok": true, "experiments": hist})
        }
        Err(e) => json!({"ok": false, "error": e.to_string()}),
    }
}

pub fn benchmark_json() -> Value {
    let el = evaluate_eligibility();
    if !el.supported {
        return blocked_json(&el);
    }
    let run = RunConfig {
        warmup: 0,
        samples: 1,
        seed: 42,
        include_compile: false,
        include_scaling: false,
        sustained_secs: 0,
        ..RunConfig::default()
    };
    let mut workload = WorkloadConfig::from_run_config(&run);
    workload.include_storage = false;
    let weights = KraftIndexWeights::default();
    struct NullProgress;
    impl kraftverk_bench::BenchProgress for NullProgress {
        fn on_event(&mut self, _message: &str) {}
    }
    match run_suite_samples(&run, &workload, &weights, None, &mut NullProgress, 1.0) {
        Ok(suite) => json!({
            "ok": true,
            "hardware_policy": HARDWARE_POLICY,
            "raw_mean": suite.index_samples_raw.first(),
            "measurements": suite.samples.last().map(|s| &s.measurements),
            "note": "Single-sample desktop probe — use CLI baseline/optimize for decisions."
        }),
        Err(e) => json!({"ok": false, "error": e.to_string()}),
    }
}
