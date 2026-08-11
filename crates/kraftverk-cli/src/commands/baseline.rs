use anyhow::Result;
use kraftverk_core::config::RunConfig;
use kraftverk_core::experiment::{Experiment, ExperimentStatus};
use kraftverk_core::BASELINE_INDEX;

use crate::engine::{
    fill_experiment_from_suite, mean_raw, open_session, run_measured, CliProgress, VERSION,
};
use crate::output::{print_json, println_human, OutputOpts};

pub fn run(out: &OutputOpts, warmup: usize, samples: usize, seed: u64) -> Result<()> {
    let session = open_session()?;
    let mut progress = CliProgress::new(out.quiet, out.json);

    let run_cfg = RunConfig {
        warmup,
        samples,
        seed,
        ..RunConfig::default()
    };

    println_human(out, "Creating baseline (this runs real KraftBench work)…");
    let suite = run_measured(&run_cfg, None, &mut progress, 1.0)?;
    let raw_mean = mean_raw(&suite.index_samples_raw)?;

    // Re-normalize all samples to baseline mean → 10_000.
    let mut suite = suite;
    suite.baseline_raw = Some(raw_mean);
    suite.index_samples_normalized = suite
        .index_samples_raw
        .iter()
        .map(|r| (r / raw_mean) * BASELINE_INDEX)
        .collect();
    suite.index_summary = kraftverk_core::summarize(
        &suite.index_samples_normalized,
        &kraftverk_core::StatsConfig::default(),
    )
    .ok();
    if let Some(idx) = suite.final_index.as_mut() {
        *idx = idx.normalize_to_baseline(raw_mean)?;
        // Force score to ~10000 based on mean of normalized samples.
        if let Some(sum) = &suite.index_summary {
            idx.score = sum.mean;
        }
    }

    let mut exp = Experiment::new_baseline(&session.report_fingerprint, VERSION, &session.os_info);
    exp.hardware_policy = session.eligibility_policy.clone();
    exp.status = ExperimentStatus::Measuring;
    fill_experiment_from_suite(&mut exp, suite, None, 0.15)?;
    // Attach raw mean for later candidate normalization.
    if let Some(k) = exp.kraft_index.as_mut() {
        k.raw_composite = raw_mean;
        k.score = BASELINE_INDEX;
    }
    if let Some(sum) = exp.index_summary.as_mut() {
        // Represent baseline index samples as ~10000.
        let _ = sum;
    }
    // Store raw mean in meta via decision_reason detail and also in kraft_index.
    exp.decision_reason = format!("baseline raw_composite_mean={raw_mean:.6}");

    session.store.upsert(&exp)?;

    let score = exp
        .kraft_index
        .as_ref()
        .map(|k| k.score)
        .unwrap_or(BASELINE_INDEX);

    if out.json {
        print_json(&serde_json::json!({
            "ok": true,
            "experiment_id": exp.id.to_string(),
            "kraft_index": score,
            "raw_composite_mean": raw_mean,
            "stability": exp.stability.as_str(),
            "samples": exp.index_samples,
            "fingerprint": session.report_fingerprint,
            "hardware_policy": session.eligibility_policy,
        }));
    } else {
        println_human(out, format!("Baseline experiment: {}", exp.id));
        println_human(
            out,
            format!("Hardware policy: {}", session.eligibility_policy),
        );
        println_human(out, format!("Kraft Index: {score:.1} (normalized)"));
        println_human(out, format!("Stability: {}", exp.stability.as_str()));
        if let Some(s) = &exp.index_summary {
            println_human(
                out,
                format!(
                    "Samples: n={} mean={:.1} median={:.1} stddev={:.1} cov={:.3}",
                    s.n, s.mean, s.median, s.stddev, s.cov
                ),
            );
        }
        println_human(out, "Next: kraftverk optimize --mode safe");
    }
    Ok(())
}
