use anyhow::{anyhow, Result};
use chrono::Utc;
use kraftverk_core::candidate::Candidate;
use kraftverk_optimizer::{list_profiles, recommend_profile, KraftProfile};
use kraftverk_system::ApplyGuard;
use uuid::Uuid;

use crate::engine::open_session;
use crate::output::{print_json, println_human, OutputOpts};

pub fn list(out: &OutputOpts) -> Result<()> {
    let profiles = list_profiles();
    if out.json {
        print_json(&serde_json::json!({"ok": true, "profiles": profiles}));
    } else {
        for p in profiles {
            let mark = if p.available { "ready" } else { "n/a" };
            println_human(
                out,
                format!(
                    "  [{mark}] {:<14} mode={} goal={} — {}",
                    p.id,
                    p.mode.as_str(),
                    p.goal.as_str(),
                    p.notes
                ),
            );
        }
    }
    Ok(())
}

pub fn recommend(out: &OutputOpts, goal: &str) -> Result<()> {
    let g = kraftverk_core::OptimizeGoal::parse(goal)
        .ok_or_else(|| anyhow!("unknown goal '{goal}'"))?;
    let p = recommend_profile(g);
    if out.json {
        print_json(&serde_json::json!({"ok": true, "profile": p}));
    } else {
        println_human(out, format!("Recommended: {} ({})", p.id, p.name));
        println_human(
            out,
            format!("Mode={} goal={}", p.mode.as_str(), p.goal.as_str()),
        );
        println_human(out, &p.notes);
    }
    Ok(())
}

pub fn export(out: &OutputOpts, path: &str) -> Result<()> {
    let session = open_session()?;
    let raw = session
        .store
        .active_config()?
        .unwrap_or_else(|| serde_json::to_string(&Candidate::identity()).unwrap());
    let candidate: Candidate = serde_json::from_str(&raw)?;
    let profile = KraftProfile {
        format: "kraftverk.profile".into(),
        version: 1,
        id: Uuid::new_v4().to_string(),
        name: "exported-active".into(),
        goal: kraftverk_core::OptimizeGoal::Balanced,
        mode: kraftverk_core::OptimizeMode::Safe,
        candidate,
        machine_fingerprint: Some(session.report_fingerprint.clone()),
        created_at: Utc::now().to_rfc3339(),
        notes: "Exported from active configuration".into(),
    };
    profile.validate().map_err(|e| anyhow!(e))?;
    std::fs::write(path, serde_json::to_string_pretty(&profile)?)?;
    println_human(out, format!("Exported profile to {path}"));
    if out.json {
        print_json(&serde_json::json!({"ok": true, "path": path, "profile": profile}));
    }
    Ok(())
}

pub fn inspect(out: &OutputOpts, path: &str) -> Result<()> {
    let raw = std::fs::read_to_string(path)?;
    let profile: KraftProfile = serde_json::from_str(&raw)?;
    profile.validate().map_err(|e| anyhow!(e))?;
    if out.json {
        print_json(&serde_json::json!({"ok": true, "profile": profile}));
    } else {
        println_human(out, format!("{} — {}", profile.id, profile.name));
        println_human(
            out,
            format!(
                "goal={} mode={}",
                profile.goal.as_str(),
                profile.mode.as_str()
            ),
        );
        println_human(
            out,
            format!("candidate: {}", profile.candidate.summary_line()),
        );
        println_human(out, &profile.notes);
    }
    Ok(())
}

pub fn validate(out: &OutputOpts, path: &str) -> Result<()> {
    let raw = std::fs::read_to_string(path)?;
    let profile: KraftProfile = serde_json::from_str(&raw)?;
    match profile.validate() {
        Ok(()) => {
            if out.json {
                print_json(&serde_json::json!({"ok": true, "valid": true}));
            } else {
                println_human(out, "Profile valid.");
            }
            Ok(())
        }
        Err(e) => Err(anyhow!("invalid profile: {e}")),
    }
}

pub fn apply(out: &OutputOpts, path: &str) -> Result<()> {
    let raw = std::fs::read_to_string(path)?;
    let profile: KraftProfile = serde_json::from_str(&raw)?;
    profile.validate().map_err(|e| anyhow!(e))?;
    let mut session = open_session()?;
    if profile.candidate.is_identity() {
        println_human(out, "Profile is identity; nothing to apply.");
        return Ok(());
    }
    let guard = ApplyGuard::apply(
        &mut session.platform,
        profile.candidate.clone(),
        &mut session.journal,
        &format!("profile:{}", profile.id),
    )?;
    guard.commit()?;
    session
        .store
        .set_active_config(&serde_json::to_string(&profile.candidate)?)?;
    println_human(
        out,
        format!(
            "Applied profile {} ({})",
            profile.name,
            profile.candidate.summary_line()
        ),
    );
    if out.json {
        print_json(&serde_json::json!({"ok": true, "applied": profile.id}));
    }
    Ok(())
}
