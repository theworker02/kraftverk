use anyhow::Result;
use kraftverk_agent::agent_connected;
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
    let agent_ok = agent_connected();

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
            match snap.temp_c {
                Some(t) => format!("available ({t:.1}°C)"),
                None => "unavailable (not fabricated)".into(),
            },
            true,
        ),
        (
            "power_sensor",
            match snap.power_w {
                Some(p) => format!("available ({p:.1} W)"),
                None => "unavailable (not fabricated)".into(),
            },
            true,
        ),
        (
            "privileged_agent",
            if agent_ok {
                "ok — authenticated IPC reachable".into()
            } else {
                "FAIL — not running (kraftverk agent serve)".into()
            },
            agent_ok,
        ),
    ];

    let _ = checks.iter().all(|(_, _, pass)| *pass);

    if out.json {
        print_json(&serde_json::json!({
            "ok": true,
            "safe_mode_recommended": safe,
            "agent_connected": agent_ok,
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
        if !agent_ok {
            println_human(
                out,
                "Privileged agent optional for Safe mode; required for elevated power schemes.",
            );
        }
    }
    Ok(())
}
