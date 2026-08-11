use anyhow::Result;
use kraftverk_system::telemetry::{capture_snapshot, environment_suitable_for_bench};

use crate::engine::{open_session, VERSION};
use crate::output::{print_json, println_human, OutputOpts};

pub fn run(out: &OutputOpts) -> Result<()> {
    let session = open_session()?;
    let snap = capture_snapshot();
    let quiet = environment_suitable_for_bench(&snap);
    let streak = session.store.failure_streak()?;
    let safe = session.store.safe_mode_recommended()?;
    let baseline = session.store.latest_baseline(&session.report_fingerprint)?;
    let journal_interrupted = session.journal.interrupted().is_some();

    let checks = vec![
        ("version", VERSION.to_string(), true),
        ("database", session.store.path().display().to_string(), true),
        (
            "baseline",
            if baseline.is_some() {
                "present".into()
            } else {
                "missing — run kraftverk baseline".into()
            },
            baseline.is_some(),
        ),
        (
            "environment_noise",
            format!("{} ({:.2})", snap.noise.level, snap.noise.score),
            quiet,
        ),
        ("failure_streak", streak.to_string(), !safe),
        (
            "recovery_journal",
            if journal_interrupted {
                "interrupted — will auto-recover".into()
            } else {
                "clean".into()
            },
            !journal_interrupted,
        ),
        (
            "temp_sensor",
            if snap.temp_c.is_some() {
                "available".into()
            } else {
                "unavailable (not fabricated)".into()
            },
            true,
        ),
        (
            "power_sensor",
            if snap.power_w.is_some() {
                "available".into()
            } else {
                "unavailable (not fabricated)".into()
            },
            true,
        ),
    ];

    // Doctor is informational: missing sensors do not fail the command.
    let _informational = checks.iter().all(|(_, _, pass)| *pass);

    if out.json {
        print_json(&serde_json::json!({
            "ok": true,
            "safe_mode_recommended": safe,
            "checks": checks.iter().map(|(n,v,p)| serde_json::json!({"name": n, "value": v, "pass": p})).collect::<Vec<_>>(),
            "telemetry": snap,
        }));
    } else {
        println_human(out, "Kraftverk doctor");
        for (name, value, pass) in &checks {
            let mark = if *pass { "ok" } else { "!!" };
            println_human(out, format!("  [{mark}] {name}: {value}"));
        }
        if safe {
            println_human(
                out,
                "Recommendation: start with --mode safe after repeated failures.",
            );
        }
    }
    Ok(())
}
