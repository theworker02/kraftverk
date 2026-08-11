use anyhow::Result;
use kraftverk_system::inspect_machine;
use kraftverk_system::{detect_platform, Platform, HARDWARE_POLICY};

use crate::engine::VERSION;
use crate::output::{print_json, println_human, OutputOpts};

pub fn run(out: &OutputOpts) -> Result<()> {
    let report = inspect_machine(VERSION);
    let platform = detect_platform()?;
    let caps = platform.capabilities();
    let eligibility = report.eligibility.clone();

    if out.json {
        print_json(&serde_json::json!({
            "ok": true,
            "machine": report,
            "platform": platform.name(),
            "capabilities": caps,
            "hardware_policy": HARDWARE_POLICY,
            "eligibility": eligibility,
        }));
        return Ok(());
    }

    println_human(out, format!("Kraftverk inspect ({VERSION})"));
    println_human(out, format!("Fingerprint: {}", report.fingerprint));
    println_human(
        out,
        format!(
            "OS: {} {} ({})",
            report.os_family, report.os_version, report.arch
        ),
    );
    println_human(
        out,
        format!(
            "CPU: {} | {} phys / {} logical | {} MHz | vendor={}",
            report.cpu.brand,
            report.cpu.physical_cores,
            report.cpu.logical_cpus,
            report.cpu.frequency_mhz,
            report.cpu.vendor_id
        ),
    );
    println_human(
        out,
        format!(
            "Memory: {:.1} GiB total, {:.1} GiB available",
            report.memory.total_bytes as f64 / (1 << 30) as f64,
            report.memory.available_bytes as f64 / (1 << 30) as f64
        ),
    );
    println_human(out, format!("Storage volumes: {}", report.storage.len()));
    for g in &report.gpus {
        println_human(
            out,
            format!("GPU: {} ({:?}) — {}", g.name, g.status, g.notes),
        );
    }
    if let Some(el) = &eligibility {
        println_human(
            out,
            format!("Eligibility ({HARDWARE_POLICY}): {}", el.summary()),
        );
    }
    println_human(out, format!("Temperature: {:?}", report.temperature));
    if !report.unsupported.is_empty() {
        println_human(out, "Unsupported / not collected:");
        for u in &report.unsupported {
            println_human(out, format!("  - {u}"));
        }
    }
    println_human(out, "Platform capabilities:");
    for c in &caps.features {
        println_human(
            out,
            format!("  [{}] {} — {}", c.support.as_str(), c.id, c.notes),
        );
    }
    Ok(())
}
