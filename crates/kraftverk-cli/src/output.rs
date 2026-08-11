//! Shared CLI output helpers.

use serde::Serialize;

#[derive(Debug, Clone, Copy)]
pub struct OutputOpts {
    pub json: bool,
    pub quiet: bool,
    #[allow(dead_code)]
    pub verbose: bool,
}

pub fn print_json<T: Serialize>(value: &T) {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .unwrap_or_else(|e| { format!("{{\"ok\":false,\"error\":\"{e}\"}}") })
    );
}

pub fn println_human(out: &OutputOpts, msg: impl AsRef<str>) {
    if !out.json && !out.quiet {
        println!("{}", msg.as_ref());
    }
}

pub fn eprintln_human(out: &OutputOpts, msg: impl AsRef<str>) {
    if !out.json && !out.quiet {
        eprintln!("{}", msg.as_ref());
    }
}
