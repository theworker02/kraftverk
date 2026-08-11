//! Shared AMD-only hardware gate for CLI command dispatch.

use kraftverk_system::{evaluate_eligibility, exit_code_for, HardwareEligibility};

use crate::output::{println_human, OutputOpts};

/// Commands that may run without full eligibility (inspect/explain only).
pub fn command_bypasses_gate(name: &str) -> bool {
    matches!(
        name,
        "compatibility" | "hardware" | "amd" | "inspect" | "doctor" | "help"
    )
}

/// Enforce hardware eligibility. Returns eligibility on success; exits process on failure.
pub fn enforce_or_exit(out: &OutputOpts) -> HardwareEligibility {
    let el = evaluate_eligibility();
    if el.supported {
        if out.verbose && !out.quiet && !out.json {
            println_human(out, format!("hardware policy {}: {}", el.policy, el.summary()));
        }
        return el;
    }
    let code = el
        .primary_rejection()
        .map(exit_code_for)
        .unwrap_or(kraftverk_system::ExitCode::UnsupportedCombination)
        .as_i32();
    if out.json {
        eprintln!(
            "{}",
            serde_json::json!({
                "ok": false,
                "error": el.summary(),
                "eligibility": el,
                "exit_code": code,
                "hardware_policy": el.policy,
            })
        );
    } else if !out.quiet {
        eprintln!("Kraftverk hardware gate ({}): BLOCKED", el.policy);
        eprintln!("{}", el.summary());
        for r in &el.rejection_reasons {
            eprintln!("  - {}", r.message());
        }
        eprintln!(
            "Inspect-only commands still work: kraftverk compatibility | kraftverk hardware | kraftverk amd"
        );
    }
    std::process::exit(code);
}
