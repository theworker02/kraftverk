use anyhow::Result;

use crate::engine::open_session;
use crate::output::{print_json, println_human, OutputOpts};

pub fn run(out: &OutputOpts, limit: usize) -> Result<()> {
    let session = open_session()?;
    let sessions = session.store.list_sessions(limit)?;
    if out.json {
        print_json(&serde_json::json!({"ok": true, "sessions": sessions}));
    } else if sessions.is_empty() {
        println_human(out, "No optimize sessions stored yet.");
    } else {
        println_human(out, "Optimize sessions:");
        for s in sessions {
            println_human(
                out,
                format!(
                    "  {}  {}  goal={}  failures={}  updated={}",
                    &s.id.to_string()[..8],
                    s.status.as_str(),
                    s.goal.as_str(),
                    s.failure_streak,
                    s.updated_at.to_rfc3339()
                ),
            );
        }
    }
    Ok(())
}
