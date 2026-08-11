use anyhow::Result;
use kraftverk_agent::{agent_connected, trust_boundary_summary};
use kraftverk_optimizer::list_search_plugins;
use kraftverk_system::{detect_platform, Platform};

use crate::engine::VERSION;
use crate::output::{print_json, println_human, OutputOpts};

pub fn run(out: &OutputOpts) -> Result<()> {
    let platform = detect_platform()?;
    let caps = platform.capabilities();
    let topo = platform.topology()?;
    let plugins = list_search_plugins();

    if out.json {
        print_json(&serde_json::json!({
            "ok": true,
            "version": VERSION,
            "agent_connected": agent_connected(),
            "trust_boundary": trust_boundary_summary(),
            "capabilities": caps,
            "topology": topo,
            "search_plugins": plugins,
        }));
    } else {
        println_human(out, format!("Kraftverk {VERSION}"));
        println_human(out, format!("Agent connected: {}", agent_connected()));
        println_human(out, trust_boundary_summary());
        println_human(out, format!("Logical CPUs: {}", topo.cpu.logical_cpus));
        println_human(out, "Capabilities:");
        for c in &caps.features {
            println_human(
                out,
                format!("  {} => {} — {}", c.id, c.support.as_str(), c.notes),
            );
        }
        println_human(out, "Search plugins:");
        for p in &plugins {
            let mark = if p.available { "ready" } else { "n/a" };
            println_human(out, format!("  [{mark}] {} — {}", p.id, p.notes));
        }
    }
    Ok(())
}
