//! Inspect-only hardware inventory (no full eligibility gate).

use anyhow::Result;
use kraftverk_system::{
    detect_architecture, detect_cpu_vendor, detect_gpu_devices, evaluate_eligibility,
    HARDWARE_POLICY,
};

use crate::engine::VERSION;
use crate::output::{print_json, println_human, OutputOpts};

pub fn run(out: &OutputOpts) -> Result<()> {
    let (cpu_vendor, cpu_raw) = detect_cpu_vendor();
    let arch = detect_architecture();
    let gpus = detect_gpu_devices();
    let eligibility = evaluate_eligibility();

    if out.json {
        print_json(&serde_json::json!({
            "ok": true,
            "inspect_only": true,
            "version": VERSION,
            "policy": HARDWARE_POLICY,
            "architecture": arch.as_str(),
            "cpu": {
                "vendor": cpu_vendor.as_str(),
                "vendor_raw": cpu_raw,
            },
            "gpus": gpus.iter().map(|g| serde_json::json!({
                "vendor": g.vendor.as_str(),
                "pci_vendor_id": g.pci_vendor_id,
                "name": g.name,
                "bus_id": g.bus_id,
            })).collect::<Vec<_>>(),
            "eligibility": {
                "supported": eligibility.supported,
                "compatibility": eligibility.compatibility.as_str(),
                "summary": eligibility.summary(),
            },
            "disclaimer": "Independent product; not affiliated with AMD.",
        }));
        return Ok(());
    }

    println_human(out, format!("Kraftverk hardware ({VERSION})"));
    println_human(out, format!("Policy: {HARDWARE_POLICY}"));
    println_human(out, format!("Architecture: {}", arch.as_str()));
    println_human(
        out,
        format!("CPU vendor: {} ({cpu_raw})", cpu_vendor.as_str()),
    );
    if gpus.is_empty() {
        println_human(out, "GPUs: none (PCI display class)");
    } else {
        for g in &gpus {
            let pci = g
                .pci_vendor_id
                .map(|id| format!("0x{id:04X}"))
                .unwrap_or_else(|| "n/a".into());
            println_human(
                out,
                format!(
                    "GPU: {} | vendor={} pci={} bus={}",
                    g.name,
                    g.vendor.as_str(),
                    pci,
                    g.bus_id
                ),
            );
        }
    }
    println_human(out, format!("Eligibility: {}", eligibility.summary()));
    Ok(())
}
