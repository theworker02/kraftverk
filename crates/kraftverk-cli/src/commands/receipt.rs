use anyhow::{anyhow, Result};
use kraftverk_data::{load_receipt, write_receipt};
use std::path::PathBuf;

use crate::engine::{load_experiment, open_session};
use crate::output::{print_json, println_human, OutputOpts};

/// Export or verify an evidence receipt for an experiment.
pub fn run(
    out: &OutputOpts,
    experiment: &str,
    output: Option<&str>,
    verify: Option<&str>,
) -> Result<()> {
    if let Some(path) = verify {
        let receipt = load_receipt(PathBuf::from(path).as_path())?;
        if out.json {
            print_json(&serde_json::json!({
                "ok": true,
                "valid": true,
                "receipt": receipt,
            }));
        } else {
            println_human(out, format!("Receipt valid: {}", receipt.receipt_id));
            println_human(
                out,
                format!(
                    "Experiment {} · hash {}",
                    receipt.experiment_id, receipt.evidence_hash
                ),
            );
        }
        return Ok(());
    }

    if experiment.is_empty() {
        return Err(anyhow!("experiment id required"));
    }

    let session = open_session()?;
    let exp = load_experiment(&session.store, experiment)?;
    let path = output.map(PathBuf::from);
    let (written, receipt) = write_receipt(&exp, path.as_deref())?;
    if out.json {
        print_json(&serde_json::json!({
            "ok": true,
            "path": written,
            "receipt": receipt,
        }));
    } else {
        println_human(out, format!("Wrote receipt {}", written.display()));
        println_human(
            out,
            format!(
                "Evidence hash {} ({})",
                receipt.evidence_hash, receipt.candidate_summary
            ),
        );
    }
    Ok(())
}
