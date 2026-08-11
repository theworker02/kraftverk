use anyhow::Result;
use kraftverk_core::candidate::Candidate;
use kraftverk_system::{ApplyGuard, Platform};

use crate::engine::open_session;
use crate::output::{print_json, println_human, OutputOpts};

pub fn run(out: &OutputOpts, baseline: bool) -> Result<()> {
    let mut session = open_session()?;

    if let Some(raw) = session.store.active_config()? {
        let candidate: Candidate = serde_json::from_str(&raw)?;
        if !candidate.is_identity() {
            println_human(
                out,
                format!(
                    "Restoring from active candidate: {}",
                    candidate.summary_line()
                ),
            );
            let mut inverted = candidate.clone();
            for ch in &mut inverted.changes {
                std::mem::swap(&mut ch.previous, &mut ch.next);
            }
            for ch in &mut inverted.changes {
                if let Ok(cur) = session.platform.read_param(&ch.key) {
                    ch.previous = cur;
                }
            }
            let guard = ApplyGuard::apply(
                &mut session.platform,
                inverted,
                &mut session.journal,
                "restore",
            )?;
            guard.commit()?;
        }
        session.store.clear_active_config()?;
    } else {
        println_human(
            out,
            "No active accepted config; ensuring identity parameters.",
        );
    }

    if session.journal.interrupted().is_some() {
        let _ = session.journal.recover_with(&mut session.platform)?;
    }

    if baseline {
        println_human(
            out,
            "Baseline measurement retained. Configuration restored toward pre-accept identity.",
        );
    }

    if out.json {
        print_json(&serde_json::json!({
            "ok": true,
            "restored": true,
            "baseline_flag": baseline,
            "active_candidate": serde_json::Value::Null,
        }));
    } else {
        println_human(out, "Restored. Active configuration cleared.");
        println_human(
            out,
            "Note: baseline measurement itself is unchanged; re-run optimize to search again.",
        );
    }
    Ok(())
}
