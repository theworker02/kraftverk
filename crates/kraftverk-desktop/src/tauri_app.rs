//! Tauri 2 native shell.

use kraftverk_system::evaluate_eligibility;
use tauri::Manager;

use crate::api;

#[tauri::command]
fn cmd_eligibility() -> serde_json::Value {
    let el = evaluate_eligibility();
    api::eligibility_json(&el)
}

#[tauri::command]
fn cmd_overview() -> serde_json::Value {
    api::overview_json(api::VERSION)
}

#[tauri::command]
fn cmd_status() -> serde_json::Value {
    api::status_json()
}

#[tauri::command]
fn cmd_telemetry() -> serde_json::Value {
    api::telemetry_json()
}

#[tauri::command]
fn cmd_history(limit: Option<usize>) -> serde_json::Value {
    api::history_json(limit.unwrap_or(20))
}

#[tauri::command]
fn cmd_benchmark() -> serde_json::Value {
    api::benchmark_json()
}

pub fn run() {
    let eligibility = evaluate_eligibility();
    println!("Kraftverk Desktop {} — Tauri shell", api::VERSION);
    println!("Eligibility: {}", eligibility.summary());

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            cmd_eligibility,
            cmd_overview,
            cmd_status,
            cmd_telemetry,
            cmd_history,
            cmd_benchmark
        ])
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_title(&format!("Kraftverk {}", api::VERSION));
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Kraftverk desktop");
}
