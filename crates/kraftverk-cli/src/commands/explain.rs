use anyhow::Result;

use crate::engine::{load_experiment, open_session};
use crate::output::{print_json, println_human, OutputOpts};

pub fn run(out: &OutputOpts, id: &str) -> Result<()> {
    let session = open_session()?;
    let exp = load_experiment(&session.store, id)?;

    if out.json {
        print_json(&serde_json::json!({ "ok": true, "experiment": exp }));
        return Ok(());
    }

    println_human(out, format!("Experiment {}", exp.id));
    println_human(out, format!("Kind: {:?}", exp.kind));
    println_human(out, format!("Status: {:?}", exp.status));
    println_human(
        out,
        format!("Decision: {:?} — {}", exp.decision, exp.decision_reason),
    );
    println_human(out, format!("Stability: {}", exp.stability.as_str()));
    if let Some(c) = exp.comparison_class {
        println_human(out, format!("Class: {}", c.as_str()));
    }
    println_human(out, format!("Candidate: {}", exp.candidate.summary_line()));
    for ch in &exp.candidate.changes {
        println_human(
            out,
            format!(
                "  {} : {} → {} ({})",
                ch.key,
                ch.previous.display(),
                ch.next.display(),
                ch.rationale
            ),
        );
    }
    if let Some(s) = &exp.index_summary {
        println_human(
            out,
            format!(
                "Index samples: n={} mean={:.2} median={:.2} min={:.2} max={:.2} stddev={:.2} cov={:.3}",
                s.n, s.mean, s.median, s.min, s.max, s.stddev, s.cov
            ),
        );
        println_human(
            out,
            format!(
                "CI: [{:.2}, {:.2}]  outliers={}",
                s.ci_low, s.ci_high, s.outlier_count
            ),
        );
    }
    if let Some(cmp) = &exp.comparison {
        println_human(out, format!("Comparison: {}", cmp.explanation));
    }
    if let Some(k) = &exp.kraft_index {
        println_human(out, format!("Kraft Index score: {:.2}", k.score));
        println_human(out, "Category scores:");
        for (cat, v) in &k.category_scores {
            println_human(out, format!("  {cat}: {v:.4}"));
        }
    }
    println_human(out, format!("Raw sample indices: {:?}", exp.index_samples));
    Ok(())
}
