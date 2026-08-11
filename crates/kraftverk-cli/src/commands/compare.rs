use anyhow::Result;
use kraftverk_core::statistics::{compare_samples, StatsConfig};

use crate::engine::{load_experiment, open_session};
use crate::output::{print_json, println_human, OutputOpts};

pub fn run(out: &OutputOpts, a: &str, b: &str) -> Result<()> {
    let session = open_session()?;
    let ea = load_experiment(&session.store, a)?;
    let eb = load_experiment(&session.store, b)?;

    let cmp = if !ea.index_samples.is_empty() && !eb.index_samples.is_empty() {
        Some(compare_samples(
            &ea.index_samples,
            &eb.index_samples,
            &StatsConfig::default(),
        )?)
    } else {
        None
    };

    if out.json {
        print_json(&serde_json::json!({
            "ok": true,
            "a": { "id": ea.id.to_string(), "score_mean": ea.index_summary.as_ref().map(|s| s.mean), "candidate": ea.candidate },
            "b": { "id": eb.id.to_string(), "score_mean": eb.index_summary.as_ref().map(|s| s.mean), "candidate": eb.candidate },
            "comparison": cmp,
        }));
        return Ok(());
    }

    println_human(out, format!("Compare {} vs {}", ea.id, eb.id));
    println_human(
        out,
        format!(
            "A: {} — {}",
            ea.candidate.summary_line(),
            ea.decision_reason
        ),
    );
    println_human(
        out,
        format!(
            "B: {} — {}",
            eb.candidate.summary_line(),
            eb.decision_reason
        ),
    );
    if let Some(c) = cmp {
        println_human(out, format!("B relative to A: {}", c.explanation));
    } else {
        println_human(out, "Insufficient index samples to compare statistically.");
    }
    Ok(())
}
