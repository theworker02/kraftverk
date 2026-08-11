use anyhow::{anyhow, Result};
use std::process::Command;
use std::time::Instant;

use crate::engine::open_session;
use crate::output::{print_json, println_human, OutputOpts};

/// Measure an external command's wall time (chase a workload).
pub fn run(out: &OutputOpts, argv: &[String], samples: usize) -> Result<()> {
    if argv.is_empty() {
        return Err(anyhow!("usage: kraftverk chase -- <command> [args...]"));
    }
    let _session = open_session()?;
    let mut times = Vec::new();
    for i in 0..samples.max(1) {
        println_human(
            out,
            format!("chase sample {}/{}: {}", i + 1, samples, argv.join(" ")),
        );
        let start = Instant::now();
        let status = Command::new(&argv[0])
            .args(&argv[1..])
            .status()
            .map_err(|e| anyhow!("failed to spawn '{}': {e}", argv[0]))?;
        let elapsed = start.elapsed().as_secs_f64();
        times.push(elapsed);
        if !status.success() {
            return Err(anyhow!(
                "chased command exited with {status} after {elapsed:.3}s"
            ));
        }
    }
    let mean = times.iter().sum::<f64>() / times.len() as f64;
    if out.json {
        print_json(&serde_json::json!({
            "ok": true,
            "command": argv,
            "samples_s": times,
            "mean_s": mean,
            "note": "Wall-clock only; Kraftverk does not invent internal counters for foreign processes."
        }));
    } else {
        println_human(
            out,
            format!(
                "Chase mean wall time: {mean:.3}s over {} sample(s)",
                times.len()
            ),
        );
        for (i, t) in times.iter().enumerate() {
            println_human(out, format!("  sample {}: {t:.3}s", i + 1));
        }
    }
    Ok(())
}
