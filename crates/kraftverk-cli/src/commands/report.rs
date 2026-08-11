use anyhow::{anyhow, Result};
use kraftverk_data::{report_html, report_json};

use crate::engine::{load_experiment, open_session};
use crate::output::{print_json, println_human, OutputOpts};

pub fn run(
    out: &OutputOpts,
    experiment: Option<&str>,
    format: &str,
    output: Option<&str>,
) -> Result<()> {
    let session = open_session()?;
    let exps = if let Some(id) = experiment {
        vec![load_experiment(&session.store, id)?]
    } else {
        session
            .store
            .history(Some(&session.report_fingerprint), 25)?
    };
    let title = "Kraftverk Evidence Report";
    match format {
        "json" => {
            let doc = report_json(&exps, title);
            if let Some(path) = output {
                std::fs::write(path, serde_json::to_string_pretty(&doc)?)?;
                println_human(out, format!("Wrote {path}"));
            } else if out.json {
                print_json(&doc);
            } else {
                println!("{}", serde_json::to_string_pretty(&doc)?);
            }
        }
        "html" => {
            let html = report_html(&exps, title);
            let path = output.unwrap_or("kraftverk-report.html");
            std::fs::write(path, html)?;
            println_human(out, format!("Wrote HTML report: {path}"));
            if out.json {
                print_json(
                    &serde_json::json!({"ok": true, "path": path, "experiments": exps.len()}),
                );
            }
        }
        other => return Err(anyhow!("unsupported format '{other}' (use html|json)")),
    }
    Ok(())
}
