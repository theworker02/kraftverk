//! Kraftverk Desktop — Tauri shell and/or local web UI over the same DB as the CLI.

mod api;

#[cfg(feature = "web-server")]
mod web;

#[cfg(feature = "tauri-app")]
mod tauri_app;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("warn")
        .with_target(false)
        .init();

    #[cfg(all(feature = "tauri-app", feature = "web-server"))]
    {
        if std::env::args().any(|a| a == "--web") {
            web::run();
            return;
        }
        tauri_app::run();
        return;
    }

    #[cfg(all(feature = "tauri-app", not(feature = "web-server")))]
    {
        tauri_app::run();
    }

    #[cfg(all(feature = "web-server", not(feature = "tauri-app")))]
    {
        web::run();
    }

    #[cfg(not(any(feature = "web-server", feature = "tauri-app")))]
    {
        eprintln!("kraftverk-desktop: enable feature web-server and/or tauri-app");
        std::process::exit(2);
    }
}
