use anyhow::Result;

use crate::engine::open_session;
use crate::output::{print_json, println_human, OutputOpts};

pub fn run(out: &OutputOpts, limit: usize) -> Result<()> {
    let session = open_session()?;
    let rows = session
        .store
        .history(Some(&session.report_fingerprint), limit)?;

    if out.json {
        let items: Vec<_> = rows
            .iter()
            .map(|e| {
                serde_json::json!({
                    "id": e.id.to_string(),
                    "kind": e.kind,
                    "decision": e.decision,
                    "stability": e.stability.as_str(),
                    "score": e.index_summary.as_ref().map(|s| s.mean),
                    "class": e.comparison_class.map(|c| c.as_str()),
                    "candidate": e.candidate.summary_line(),
                    "created_at": e.created_at.to_rfc3339(),
                    "reason": e.decision_reason,
                })
            })
            .collect();
        print_json(&serde_json::json!({ "ok": true, "experiments": items }));
        return Ok(());
    }

    if rows.is_empty() {
        println_human(out, "No experiments yet.");
        return Ok(());
    }
    println_human(
        out,
        format!(
            "{:<8} {:<12} {:>8} {:<22} {}",
            "kind", "decision", "score", "when", "summary"
        ),
    );
    for e in rows {
        let score = e
            .index_summary
            .as_ref()
            .map(|s| format!("{:.1}", s.mean))
            .unwrap_or_else(|| "-".into());
        println_human(
            out,
            format!(
                "{:<8} {:<12} {:>8} {:<22} {} ({})",
                format!("{:?}", e.kind).to_ascii_lowercase(),
                format!("{:?}", e.decision).to_ascii_lowercase(),
                score,
                e.created_at.format("%Y-%m-%d %H:%M:%S"),
                e.candidate.summary_line(),
                &e.id.to_string()[..8]
            ),
        );
    }
    Ok(())
}
