use anyhow::Result;
use kraftverk_optimizer::list_profiles;

use crate::output::{print_json, println_human, OutputOpts};

pub fn run(out: &OutputOpts) -> Result<()> {
    let profiles = list_profiles();
    if out.json {
        print_json(&serde_json::json!({ "ok": true, "profiles": profiles }));
        return Ok(());
    }
    println_human(out, "Optimization profiles:");
    for p in profiles {
        let flag = if p.available {
            "available"
        } else {
            "UNSUPPORTED"
        };
        println_human(
            out,
            format!(
                "  [{flag}] {} ({}) â€” mode={} â€” {}",
                p.id,
                p.name,
                p.mode.as_str(),
                p.notes
            ),
        );
    }
    Ok(())
}
