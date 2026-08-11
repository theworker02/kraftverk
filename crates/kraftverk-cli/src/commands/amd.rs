//! AMD capability / topology inspect commands.

use anyhow::Result;
use kraftverk_system::probe_amd_capabilities;

use crate::output::{print_json, println_human, OutputOpts};

#[derive(Debug, Clone, Copy)]
pub enum AmdTarget {
    Cpu,
    Gpu,
}

pub fn run(out: &OutputOpts, target: AmdTarget) -> Result<()> {
    let caps = probe_amd_capabilities();

    match target {
        AmdTarget::Cpu => {
            if out.json {
                print_json(&serde_json::json!({
                    "ok": true,
                    "cpu_is_amd": caps.cpu_is_amd,
                    "cpu_brand": caps.cpu_brand,
                    "topology": caps.topology,
                    "cache_aware_search_hooks": caps.cache_aware_search_hooks,
                    "unsupported_surfaces": caps.unsupported_surfaces,
                    "disclaimer": "Independent product; not affiliated with AMD.",
                }));
                return Ok(());
            }
            println_human(out, "Kraftverk amd cpu");
            println_human(
                out,
                format!("AMD CPU: {} | brand: {}", caps.cpu_is_amd, caps.cpu_brand),
            );
            println_human(
                out,
                format!(
                    "Topology: phys={:?} logical={:?} smt={:?} packages={:?} ccd={:?} ccx/ccd={:?}",
                    caps.topology.physical_cores,
                    caps.topology.logical_cpus,
                    caps.topology.smt_likely,
                    caps.topology.packages,
                    caps.topology.ccd_count,
                    caps.topology.ccx_per_ccd
                ),
            );
            println_human(out, &caps.topology.preferred_cores_note);
            for n in &caps.topology.detection_notes {
                println_human(out, format!("  note: {n}"));
            }
        }
        AmdTarget::Gpu => {
            if out.json {
                print_json(&serde_json::json!({
                    "ok": true,
                    "amd_gpus": caps.amd_gpus,
                    "platform_profile_note": caps.platform_profile_note,
                    "unsupported_surfaces": caps.unsupported_surfaces,
                    "disclaimer": "Independent product; not affiliated with AMD.",
                }));
                return Ok(());
            }
            println_human(out, "Kraftverk amd gpu");
            if caps.amd_gpus.is_empty() {
                println_human(out, "No AMD GPUs detected (PCI 0x1002 display class).");
            } else {
                for g in &caps.amd_gpus {
                    println_human(out, format!("  {g}"));
                }
            }
            println_human(out, &caps.platform_profile_note);
        }
    }
    Ok(())
}
