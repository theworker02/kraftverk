use anyhow::Result;
use kraftverk_agent::{agent_connected, trust_boundary_summary};
use kraftverk_core::BASELINE_INDEX;
use kraftverk_system::Platform;

use crate::engine::open_session;
use crate::output::{print_json, println_human, OutputOpts};

pub fn run(out: &OutputOpts) -> Result<()> {
    let session = open_session()?;
    let baseline = session.store.latest_baseline(&session.report_fingerprint)?;
    let accepted = session.store.latest_accepted(&session.report_fingerprint)?;
    let active = session.store.active_config()?;

    if out.json {
        print_json(&serde_json::json!({
            "ok": true,
            "fingerprint": session.report_fingerprint,
            "os": session.os_info,
            "platform": session.platform.name(),
            "baseline": baseline.as_ref().map(|b| serde_json::json!({
                "id": b.id.to_string(),
                "kraft_index": b.kraft_index.as_ref().map(|k| k.score).unwrap_or(BASELINE_INDEX),
                "created_at": b.created_at.to_rfc3339(),
            })),
            "latest_accepted": accepted.as_ref().map(|a| serde_json::json!({
                "id": a.id.to_string(),
                "candidate": a.candidate,
                "score": a.index_summary.as_ref().map(|s| s.mean),
                "reason": a.decision_reason,
            })),
            "active_candidate": active.as_ref().and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()),
            "agent_connected": agent_connected(),
            "trust_boundary": trust_boundary_summary(),
        }));
        return Ok(());
    }

    println_human(out, "Kraftverk status");
    println_human(out, format!("Fingerprint: {}", session.report_fingerprint));
    println_human(out, format!("OS: {}", session.os_info));
    println_human(out, format!("Platform: {}", session.platform.name()));
    match baseline {
        Some(b) => println_human(
            out,
            format!(
                "Baseline: {}  Kraft Index {:.1}  ({})",
                b.id,
                b.kraft_index
                    .as_ref()
                    .map(|k| k.score)
                    .unwrap_or(BASELINE_INDEX),
                b.created_at.to_rfc3339()
            ),
        ),
        None => println_human(out, "Baseline: none (run kraftverk baseline)"),
    }
    match accepted {
        Some(a) => {
            println_human(
                out,
                format!(
                    "Latest accepted: {} â€” {}",
                    a.id,
                    a.candidate.summary_line()
                ),
            );
            println_human(out, format!("  reason: {}", a.decision_reason));
        }
        None => println_human(out, "Latest accepted: none"),
    }
    match active {
        Some(s) => println_human(out, format!("Active config: {s}")),
        None => println_human(out, "Active config: identity / defaults"),
    }
    println_human(
        out,
        format!(
            "Privileged agent: {}",
            if agent_connected() {
                "connected"
            } else {
                "not connected (M1 in-process safe opts)"
            }
        ),
    );
    println_human(out, trust_boundary_summary());
    Ok(())
}
