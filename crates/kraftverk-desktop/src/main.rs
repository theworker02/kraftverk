//! Kraftverk Desktop — engineering instrument UI over the same local DB as the CLI.
//!
//! Serves a local UI and JSON API. Opens the default browser. Not a gamer overlay.
//! AMD-only hardware eligibility is enforced before dashboard APIs that optimize/benchmark.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Json};
use axum::routing::get;
use axum::Router;
use kraftverk_bench::{run_suite_samples, WorkloadConfig};
use kraftverk_core::{KraftIndexWeights, RunConfig};
use kraftverk_data::{default_db_path, ExperimentStore};
use kraftverk_system::{
    capture_snapshot, evaluate_eligibility, exit_code_for, inspect_machine, HardwareEligibility,
    HARDWARE_POLICY,
};
use serde::Deserialize;
use serde_json::json;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

#[derive(Clone)]
struct AppState {
    version: String,
    eligibility: HardwareEligibility,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("warn")
        .with_target(false)
        .init();

    let eligibility = evaluate_eligibility();
    let ui_dir = ui_dir();
    let state = Arc::new(AppState {
        version: env!("CARGO_PKG_VERSION").into(),
        eligibility: eligibility.clone(),
    });

    let app = Router::new()
        .route("/", get(index))
        .route("/api/eligibility", get(api_eligibility))
        .route("/api/overview", get(api_overview))
        .route("/api/history", get(api_history))
        .route("/api/telemetry", get(api_telemetry))
        .route("/api/status", get(api_status))
        .route("/api/benchmark", get(api_benchmark))
        .nest_service("/static", ServeDir::new(&ui_dir))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 47821));
    let url = format!("http://{addr}/");
    println!("Kraftverk Desktop {VERSION} — engineering instrument");
    println!("Hardware policy: {HARDWARE_POLICY}");
    println!("Eligibility: {}", eligibility.summary());
    println!("UI: {url}");
    println!("Same local DB as CLI. No fabricated metrics.");
    let _ = open::that(&url);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn ui_dir() -> PathBuf {
    let mut candidates = vec![
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ui"),
        PathBuf::from("crates/kraftverk-desktop/ui"),
        PathBuf::from("ui"),
    ];
    for c in candidates.drain(..) {
        if c.join("app.js").exists() {
            return c;
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ui")
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../ui/index.html"))
}

async fn api_eligibility(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Live re-check (hot-plug aware).
    let el = evaluate_eligibility();
    Json(json!({
        "ok": true,
        "hardware_policy": HARDWARE_POLICY,
        "supported": el.supported,
        "eligibility": el,
        "startup_eligibility": state.eligibility,
        "exit_code": el.primary_rejection().map(|r| exit_code_for(r).as_i32()),
    }))
}

fn blocked_json(el: &HardwareEligibility) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::FORBIDDEN,
        Json(json!({
            "ok": false,
            "blocked": true,
            "hardware_policy": HARDWARE_POLICY,
            "error": el.summary(),
            "eligibility": el,
            "exit_code": el.primary_rejection().map(|r| exit_code_for(r).as_i32()),
        })),
    )
}

async fn api_overview(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let el = evaluate_eligibility();
    if !el.supported {
        return blocked_json(&el).into_response();
    }
    let report = inspect_machine(&state.version);
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
    Json(json!({
        "ok": true,
        "version": state.version,
        "hardware_policy": HARDWARE_POLICY,
        "eligibility": el,
        "fingerprint": report.fingerprint,
        "os": format!("{} {} ({})", report.os_family, report.os_version, report.arch),
        "baseline_id": baseline.as_ref().map(|e| e.id.to_string()),
        "baseline_score": baseline.as_ref().and_then(|e| e.kraft_index.as_ref().map(|k| k.score)),
        "accepted_id": accepted.as_ref().map(|e| e.id.to_string()),
        "history_count": hist_n,
        "philosophy": "Measure → Experiment → Benchmark → Validate → Compare → Keep or Revert → Learn → Repeat",
    }))
    .into_response()
}

async fn api_status() -> impl IntoResponse {
    let el = evaluate_eligibility();
    let report = inspect_machine(VERSION);
    let path = default_db_path().ok();
    let active = path
        .as_ref()
        .and_then(|p| ExperimentStore::open(p).ok())
        .and_then(|s| s.active_config().ok().flatten());
    Json(json!({
        "ok": true,
        "db": path.map(|p| p.display().to_string()),
        "fingerprint": report.fingerprint,
        "active_candidate": active,
        "hardware_policy": HARDWARE_POLICY,
        "eligibility": el,
        "agent": kraftverk_agent::trust_boundary_summary(),
    }))
}

async fn api_telemetry() -> impl IntoResponse {
    Json(json!({
        "ok": true,
        "snapshot": capture_snapshot(),
    }))
}

#[derive(Deserialize)]
struct HistQuery {
    limit: Option<usize>,
}

async fn api_history(Query(q): Query<HistQuery>) -> impl IntoResponse {
    let el = evaluate_eligibility();
    if !el.supported {
        return blocked_json(&el).into_response();
    }
    let report = inspect_machine(VERSION);
    let limit = q.limit.unwrap_or(20);
    match ExperimentStore::open(default_db_path().unwrap_or_else(|_| PathBuf::from("kraftverk.db")))
    {
        Ok(s) => {
            let hist = s
                .history(Some(&report.fingerprint), limit)
                .unwrap_or_default();
            Json(json!({"ok": true, "experiments": hist})).into_response()
        }
        Err(e) => Json(json!({"ok": false, "error": e.to_string()})).into_response(),
    }
}

async fn api_benchmark() -> impl IntoResponse {
    let el = evaluate_eligibility();
    if !el.supported {
        return blocked_json(&el).into_response();
    }
    // Short real bench — never invent scores.
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
    match run_suite_samples(&run, &workload, &weights, None, &mut NullProgress, 1.0) {
        Ok(suite) => Json(json!({
            "ok": true,
            "hardware_policy": HARDWARE_POLICY,
            "raw_mean": suite.index_samples_raw.first(),
            "measurements": suite.samples.last().map(|s| &s.measurements),
            "note": "Single-sample desktop probe — use CLI baseline/optimize for decisions."
        }))
        .into_response(),
        Err(e) => Json(json!({"ok": false, "error": e.to_string()})).into_response(),
    }
}

struct NullProgress;
impl kraftverk_bench::BenchProgress for NullProgress {
    fn on_event(&mut self, _message: &str) {}
}
