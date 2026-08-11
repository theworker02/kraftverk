//! kraftverk-agent — elevated local IPC server.

use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    println!(
        "kraftverk-agent {} — local IPC only (no network bind)",
        env!("CARGO_PKG_VERSION")
    );
    println!("Hardware policy: {}", kraftverk_agent::policy_id());
    println!("Endpoint: {}", kraftverk_agent::default_endpoint());
    println!("If elevation is required for power schemes, start this process elevated.");
    println!("CLI remains unprivileged: kraftverk doctor / kraftverk agent status");

    if let Err(e) = kraftverk_agent::run_agent_server() {
        eprintln!("agent error: {e}");
        std::process::exit(1);
    }
}
