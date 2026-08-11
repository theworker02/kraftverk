//! Local axum UI (default CI-friendly mode).

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Json};
use axum::routing::get;
use axum::Router;
use kraftverk_system::{evaluate_eligibility, HardwareEligibility, HARDWARE_POLICY};
use serde::Deserialize;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use crate::api;

#[derive(Clone)]
struct AppState {
    version: String,
    eligibility: HardwareEligibility,
}

#[tokio::main]
pub async fn run() {
    let eligibility = evaluate_eligibility();
    let ui_dir = ui_dir();
    let state = Arc::new(AppState {
        version: api::VERSION.into(),
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
    println!("Kraftverk Desktop {} — web UI mode", api::VERSION);
    println!("Hardware policy: {HARDWARE_POLICY}");
    println!("Eligibility: {}", eligibility.summary());
    println!("UI: {url}");
    println!("Same local DB as CLI. No fabricated metrics.");
    println!("Tip: build with --features tauri-app for the native shell.");
    let _ = open::that(&url);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind desktop port");
    axum::serve(listener, app).await.expect("serve desktop");
}

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
    Json(api::eligibility_json(&state.eligibility))
}

async fn api_overview(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let v = api::overview_json(&state.version);
    if v.get("blocked") == Some(&serde_json::Value::Bool(true)) {
        (StatusCode::FORBIDDEN, Json(v)).into_response()
    } else {
        Json(v).into_response()
    }
}

async fn api_status() -> impl IntoResponse {
    Json(api::status_json())
}

async fn api_telemetry() -> impl IntoResponse {
    Json(api::telemetry_json())
}

#[derive(Deserialize)]
struct HistQuery {
    limit: Option<usize>,
}

async fn api_history(Query(q): Query<HistQuery>) -> impl IntoResponse {
    let v = api::history_json(q.limit.unwrap_or(20));
    if v.get("blocked") == Some(&serde_json::Value::Bool(true)) {
        (StatusCode::FORBIDDEN, Json(v)).into_response()
    } else {
        Json(v).into_response()
    }
}

async fn api_benchmark() -> impl IntoResponse {
    let v = api::benchmark_json();
    if v.get("blocked") == Some(&serde_json::Value::Bool(true)) {
        (StatusCode::FORBIDDEN, Json(v)).into_response()
    } else {
        Json(v).into_response()
    }
}
