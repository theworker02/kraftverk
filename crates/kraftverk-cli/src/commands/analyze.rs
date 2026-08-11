use anyhow::Result;

use crate::engine::{load_experiment, open_session};
use crate::output::{print_json, println_human, OutputOpts};

/// Analyze an experiment or recent history for evidence summaries.
pub fn run(out: &OutputOpts, target: &str) -> Result<()> {
    let session = open_session()?;
    if target == "recent" || target.is_empty() {
        let hist = session
            .store
            .history(Some(&session.report_fingerprint), 10)?;
        if out.json {
            print_json(&serde_json::json!({"ok": true, "experiments": hist}));
        } else {
            println_human(out, "Recent experiments:");
            for e in hist {
                let score = e
                    .index_summary
                    .as_ref()
                    .map(|s| format!("{:.1}", s.mean))
                    .unwrap_or_else(|| "-".into());
                println_human(
                    out,
                    format!(
                        "  {}  {:?}  {}  score={}  {}",
                        &e.id.to_string()[..8],
                        e.kind,
                        e.stability.as_str(),
                        score,
                        e.decision_reason
                    ),
                );
            }
        }
        return Ok(());
    }

    let exp = load_experiment(&session.store, target)?;
    let snap_notes: Vec<_> = exp
        .telemetry
        .iter()
        .filter_map(|t| t.get("notes"))
        .collect();
    if out.json {
        print_json(&serde_json::json!({
            "ok": true,
            "experiment": exp,
            "analysis": {
                "sample_count": exp.index_samples.len(),
                "decision": exp.decision,
                "stability": exp.stability.as_str(),
                "comparison": exp.comparison,
                "telemetry_notes": snap_notes,
            }
        }));
    } else {
        println_human(out, format!("Analyze {}", exp.id));
        println_human(
            out,
            format!("Kind: {:?}  Decision: {:?}", exp.kind, exp.decision),
        );
        println_human(out, format!("Reason: {}", exp.decision_reason));
        println_human(out, format!("Stability: {}", exp.stability.as_str()));
        if let Some(s) = &exp.index_summary {
            println_human(
                out,
                format!("Index: mean={:.1} cov={:.3} n={}", s.mean, s.cov, s.n),
            );
        }
        if let Some(c) = &exp.comparison {
            println_human(
                out,
                format!(
                    "Comparison: {} ({:+.2}%)",
                    c.class.as_str(),
                    c.relative_change * 100.0
                ),
            );
        }
        println_human(out, format!("Candidate: {}", exp.candidate.summary_line()));
    }
    Ok(())
}
