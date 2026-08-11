use anyhow::{anyhow, Result};
use kraftverk_bench::parse_duration_to_secs;
use kraftverk_core::config::RunConfig;
use kraftverk_core::BASELINE_INDEX;

use crate::engine::{mean_raw, open_session, require_baseline, run_measured, CliProgress};
use crate::output::{print_json, println_human, OutputOpts};

pub fn run(
    out: &OutputOpts,
    warmup: usize,
    samples: usize,
    seed: u64,
    sustained: Option<&str>,
) -> Result<()> {
    let session = open_session()?;
    let mut progress = CliProgress::new(out.quiet, out.json);
    let sustained_secs = match sustained {
        Some(s) => parse_duration_to_secs(s)
            .ok_or_else(|| anyhow!("invalid --sustained duration '{s}' (try 10m, 30s, 1h)"))?,
        None => 0,
    };
    let run_cfg = RunConfig {
        warmup,
        samples,
        seed,
        sustained_secs,
        ..RunConfig::default()
    };

    let baseline_raw = session
        .store
        .latest_baseline(&session.report_fingerprint)?
        .and_then(|b| b.kraft_index.map(|k| k.raw_composite).filter(|r| *r > 0.0));

    if sustained_secs > 0 {
        println_human(
            out,
            format!("Running KraftBench with sustained window {sustained_secs}s…"),
        );
    } else {
        println_human(out, "Running KraftBench…");
    }
    let suite = run_measured(&run_cfg, baseline_raw, &mut progress, 1.0)?;
    let raw = mean_raw(&suite.index_samples_raw)?;

    let score = if let Some(b) = baseline_raw {
        (raw / b) * BASELINE_INDEX
    } else {
        raw
    };

    if out.json {
        print_json(&serde_json::json!({
            "ok": true,
            "kraft_index": score,
            "raw_composite_mean": raw,
            "baseline_normalized": baseline_raw.is_some(),
            "sustained_secs": sustained_secs,
            "index_samples": suite.index_samples_normalized,
            "measurements_last": suite.samples.last().map(|s| &s.measurements),
        }));
    } else {
        if baseline_raw.is_some() {
            println_human(out, format!("Kraft Index: {score:.1} (vs baseline 10000)"));
        } else {
            println_human(
                out,
                format!("Raw composite: {score:.4} (no baseline yet — run kraftverk baseline)"),
            );
        }
        if let Some(s) = &suite.index_summary {
            println_human(
                out,
                format!(
                    "n={} mean={:.2} stddev={:.2} cov={:.3}",
                    s.n, s.mean, s.stddev, s.cov
                ),
            );
        }
        if let Some(last) = suite.samples.last() {
            println_human(out, "Measurements:");
            for m in &last.measurements {
                println_human(
                    out,
                    format!(
                        "  {} [{}] {:.3} {} (score {:.3})",
                        m.id, m.category, m.raw_value, m.unit, m.score
                    ),
                );
            }
        }
        let _ = require_baseline;
    }
    Ok(())
}
