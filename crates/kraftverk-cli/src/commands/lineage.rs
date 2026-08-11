use anyhow::Result;
use kraftverk_optimizer::build_lineage;

use crate::engine::open_session;
use crate::output::{print_json, println_human, OutputOpts};

pub fn run(out: &OutputOpts, id: &str) -> Result<()> {
    let session = open_session()?;
    let history = session
        .store
        .history(Some(&session.report_fingerprint), 200)?;
    let tree = build_lineage(&history, id);
    if out.json {
        print_json(&serde_json::json!({"ok": true, "lineage": tree}));
    } else {
        println_human(out, format!("Lineage around {id} (root {})", tree.root_id));
        for n in &tree.nodes {
            let score = n
                .score
                .map(|s| format!("{s:.1}"))
                .unwrap_or_else(|| "-".into());
            println_human(
                out,
                format!(
                    "  {} [{}] decision={} score={} parent={} | {}",
                    &n.id[..8.min(n.id.len())],
                    n.kind,
                    n.decision,
                    score,
                    n.parent_id.as_deref().unwrap_or("-"),
                    n.summary
                ),
            );
        }
        if tree.nodes.is_empty() {
            println_human(out, "No matching experiments.");
        }
    }
    Ok(())
}
