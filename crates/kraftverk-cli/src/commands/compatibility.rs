//! Inspect-only hardware compatibility report (no full eligibility gate).

use anyhow::Result;
use kraftverk_system::{evaluate_eligibility, exit_code_for, HARDWARE_POLICY};

use crate::output::{print_json, println_human, OutputOpts};

pub fn run(out: &OutputOpts) -> Result<()> {
    let e = evaluate_eligibility();
    let exit_hint = e
        .primary_rejection()
        .map(|r| exit_code_for(r).as_i32())
        .unwrap_or(0);

    if out.json {
        print_json(&serde_json::json!({
            "ok": true,
            "inspect_only": true,
            "policy": HARDWARE_POLICY,
            "compatibility": e.compatibility.as_str(),
            "supported": e.supported,
            "architecture": e.architecture.as_str(),
            "cpu_vendor": e.cpu_vendor.as_str(),
            "cpu_vendor_raw": e.cpu_vendor_raw,
            "gpu_vendors": e.gpu_vendors.iter().map(|v| v.as_str()).collect::<Vec<_>>(),
            "gpu_details": e.gpu_details,
            "rejection_reasons": e.rejection_reasons.iter().map(|r| r.message()).collect::<Vec<_>>(),
            "summary": e.summary(),
            "exit_code_if_gated": exit_hint,
            "disclaimer": "Kraftverk is an independent product. Not affiliated with, endorsed by, or impersonating Advanced Micro Devices, Inc. (AMD).",
        }));
        return Ok(());
    }

    println_human(out, format!("Kraftverk compatibility ({HARDWARE_POLICY})"));
    println_human(out, format!("Status: {}", e.compatibility.as_str()));
    println_human(
        out,
        format!(
            "Arch: {} | CPU: {} ({})",
            e.architecture.as_str(),
            e.cpu_vendor.as_str(),
            e.cpu_vendor_raw
        ),
    );
    if e.gpu_details.is_empty() {
        println_human(out, "GPUs: none detected (allowed with AMD CPU)");
    } else {
        for d in &e.gpu_details {
            println_human(out, format!("GPU: {d}"));
        }
    }
    if !e.supported {
        println_human(out, "Reasons:");
        for r in &e.rejection_reasons {
            println_human(out, format!("  - {}", r.message()));
        }
        println_human(
            out,
            format!("Gated commands would exit with code {exit_hint}"),
        );
    }
    println_human(
        out,
        "Disclaimer: Kraftverk is independent — not an AMD product or endorsement.",
    );
    Ok(())
}
