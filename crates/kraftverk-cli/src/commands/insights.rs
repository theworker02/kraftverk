use anyhow::Result;
use kraftverk_optimizer::ParamImportance;

use crate::engine::open_session;
use crate::output::{print_json, println_human, OutputOpts};

pub fn run(out: &OutputOpts) -> Result<()> {
    let session = open_session()?;
    let history = session
        .store
        .history(Some(&session.report_fingerprint), 100)?;
    let mut importance = ParamImportance::default();
    let mut accepts = 0usize;
    let mut rejects = 0usize;

    for e in &history {
        let accepted = matches!(e.decision, kraftverk_core::Decision::Accept);
        if accepted {
            accepts += 1;
        } else if matches!(e.decision, kraftverk_core::Decision::Reject) {
            rejects += 1;
        }
        for ch in &e.candidate.changes {
            importance.record(&ch.key, accepted);
        }
    }

    let ranked = importance.rank();
    if out.json {
        print_json(&serde_json::json!({
            "ok": true,
            "experiments": history.len(),
            "accepts": accepts,
            "rejects": rejects,
            "param_importance": ranked,
            "notes": [
                "Importance is derived from measured accept/reject outcomes.",
                "No placebo insights are invented."
            ],
        }));
    } else {
        println_human(
            out,
            format!(
                "Insights from {} experiments (accepts={accepts}, rejects={rejects})",
                history.len()
            ),
        );
        if ranked.is_empty() {
            println_human(out, "No parameter trials yet. Run kraftverk optimize.");
        } else {
            println_human(out, "Parameter importance (accept rate):");
            for (k, rate) in ranked.iter().take(10) {
                println_human(out, format!("  {k}: {:.0}%", rate * 100.0));
            }
        }
    }
    Ok(())
}
