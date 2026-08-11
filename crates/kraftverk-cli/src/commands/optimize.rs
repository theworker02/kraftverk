use std::time::Instant;

use anyhow::{anyhow, Result};
use kraftverk_core::candidate::Candidate;
use kraftverk_core::config::{OptimizeConfig, OptimizeMode, RunConfig};
use kraftverk_core::constraints::OptimizeConstraints;
use kraftverk_core::experiment::{Decision, Experiment, ExperimentStatus, StabilityVerdict};
use kraftverk_core::session::{OptimizeCheckpoint, OptimizeSession, SessionStatus};
use kraftverk_core::{OptimizeGoal, BASELINE_INDEX};
use kraftverk_data::write_receipt;
use kraftverk_optimizer::{
    create_search_plugin, default_plugin_for_mode, score_objective, SearchContext, SearchDecision,
};
use kraftverk_system::telemetry::capture_snapshot;
use kraftverk_system::{ApplyGuard, Platform};

use crate::engine::{
    decide_candidate, fill_experiment_from_suite, mean_raw, open_session, require_baseline,
    run_measured, CliProgress,
};
use crate::output::{eprintln_human, print_json, println_human, OutputOpts};

#[allow(clippy::too_many_arguments)]
pub fn run(
    out: &OutputOpts,
    mode: OptimizeMode,
    goal_s: &str,
    seed: u64,
    max_experiments: usize,
    time_budget_secs: u64,
    max_temp: Option<f64>,
    max_power: Option<f64>,
    max_workers: Option<usize>,
    resume: Option<&str>,
    strategy_id: Option<&str>,
) -> Result<()> {
    if !mode.supported_without_agent() {
        return Err(anyhow!(
            "optimize mode '{}' is not available without the privileged agent",
            mode.as_str()
        ));
    }

    let goal = OptimizeGoal::parse(goal_s).ok_or_else(|| anyhow!("unknown goal '{goal_s}'"))?;

    let mut session = open_session()?;
    if session.store.safe_mode_recommended()? && !matches!(mode, OptimizeMode::Safe) {
        eprintln_human(
            out,
            "Warning: repeated failures detected — prefer --mode safe (doctor).",
        );
    }
    session.platform.mode_allowed(mode)?;
    let baseline = require_baseline(&session)?;
    let baseline_raw = baseline
        .kraft_index
        .as_ref()
        .map(|k| k.raw_composite)
        .filter(|r| *r > 0.0)
        .ok_or_else(|| anyhow!("baseline missing raw_composite; re-run kraftverk baseline"))?;
    let baseline_scores = baseline.index_samples.clone();

    let constraints = OptimizeConstraints {
        max_temp_c: max_temp,
        max_power_w: max_power,
        max_workers,
        max_rayon: max_workers,
        max_cov: None,
        min_improvement: None,
    };

    let mut cfg = OptimizeConfig {
        mode,
        goal,
        seed,
        max_experiments,
        time_budget_secs,
        constraints: constraints.clone(),
        ..OptimizeConfig::default()
    };
    cfg.run.seed = seed;

    let mut opt_session = if let Some(id) = resume {
        session
            .store
            .get_session(id)?
            .ok_or_else(|| anyhow!("session '{id}' not found"))?
    } else {
        OptimizeSession::new(goal, cfg.clone(), constraints.clone())
    };
    if let Some(cp) = &opt_session.checkpoint {
        cfg = opt_session.config.clone();
        println_human(
            out,
            format!(
                "Resuming session {} from generation {}",
                opt_session.id, cp.generation
            ),
        );
    }

    let plugin_id = strategy_id.unwrap_or_else(|| default_plugin_for_mode(mode.as_str()));
    let mut strategy = create_search_plugin(plugin_id, cfg.seed)?;
    println_human(
        out,
        format!("Search plugin: {} ({plugin_id})", strategy.name()),
    );
    let mut best_candidate = opt_session
        .checkpoint
        .as_ref()
        .map(|c| c.best_candidate.clone())
        .unwrap_or_else(Candidate::identity);
    let mut best_score = opt_session
        .checkpoint
        .as_ref()
        .map(|c| c.best_score)
        .unwrap_or(BASELINE_INDEX);
    let mut best_exp_id = opt_session
        .checkpoint
        .as_ref()
        .and_then(|c| c.best_experiment_id.clone())
        .unwrap_or_else(|| baseline.id.to_string());
    let mut plateau = opt_session
        .checkpoint
        .as_ref()
        .map(|c| c.plateau)
        .unwrap_or(0);
    let mut generation = opt_session
        .checkpoint
        .as_ref()
        .map(|c| c.generation)
        .unwrap_or(0);
    let mut experiments_done = opt_session
        .checkpoint
        .as_ref()
        .map(|c| c.experiments_done)
        .unwrap_or(0);
    let mut last_class = None;
    let started = Instant::now();
    let mut accepted: Option<Experiment> = None;

    println_human(
        out,
        format!(
            "Optimize mode={} goal={} seed={} max_experiments={}",
            mode.as_str(),
            goal.as_str(),
            cfg.seed,
            cfg.max_experiments
        ),
    );

    opt_session.status = SessionStatus::Running;
    session.store.upsert_session(&opt_session)?;

    loop {
        let env = capture_snapshot();
        let check = cfg.constraints.check_environment(env.temp_c, env.power_w);
        if !check.ok {
            eprintln_human(
                out,
                format!("Constraint violation: {}", check.violations.join("; ")),
            );
            opt_session.failure_streak += 1;
            session
                .store
                .set_failure_streak(opt_session.failure_streak)?;
            break;
        }

        let ctx = SearchContext {
            generation,
            experiments_done,
            plateau_count: plateau,
            best_score,
            last_class,
            elapsed_secs: started.elapsed().as_secs(),
            max_experiments: cfg.max_experiments,
            time_budget_secs: cfg.time_budget_secs,
            plateau_limit: cfg.plateau_limit,
        };

        // Hot-plug: if forbidden hardware appears mid-session → stop, restore, block.
        if let Err(e) = session.session_guard.assert_still_supported() {
            eprintln_human(out, format!("Hardware gate abort: {e}"));
            let _ = session.journal.recover_with(&mut session.platform);
            let _ = session.store.clear_active_config();
            return Err(anyhow!(e.to_string()));
        }

        let decision = strategy.next_candidate(&session.platform, &ctx, &best_candidate)?;
        let candidate = match decision {
            SearchDecision::Stop { reason } => {
                println_human(out, format!("Search stopped: {reason}"));
                break;
            }
            SearchDecision::Try(c) => c,
        };

        // Soft worker constraints
        let mut skip = false;
        for ch in &candidate.changes {
            if ch.key == "bench.worker_threads" {
                if let Some(n) = ch.next.as_usize() {
                    if !cfg.constraints.allows_workers(n) {
                        println_human(
                            out,
                            format!("Skipping candidate: workers {n} exceed constraint"),
                        );
                        skip = true;
                    }
                }
            }
        }
        if skip {
            continue;
        }

        generation += 1;
        experiments_done += 1;
        println_human(
            out,
            format!(
                "Experiment {experiments_done}/{}: {}",
                cfg.max_experiments,
                candidate.summary_line()
            ),
        );

        let mut exp = Experiment::new_candidate(&baseline, candidate.clone());
        exp.status = ExperimentStatus::Running;
        session.store.upsert(&exp)?;

        let journal_path = session.journal.path().display().to_string();
        let measure_result = {
            let guard = ApplyGuard::apply(
                &mut session.platform,
                candidate.clone(),
                &mut session.journal,
                &exp.id.to_string(),
            )?;
            exp.status = ExperimentStatus::Measuring;
            exp.recovery_token = Some(journal_path);
            session.store.upsert(&exp)?;

            let mut progress = CliProgress::new(out.quiet, out.json);
            let run_cfg = RunConfig {
                warmup: cfg.run.warmup.min(1),
                samples: cfg.search_samples,
                seed: cfg.run.seed,
                ..cfg.run.clone()
            };
            let suite = run_measured(&run_cfg, Some(baseline_raw), &mut progress, 1.0);
            guard.rollback()?;
            suite
        };

        let suite = match measure_result {
            Ok(s) => s,
            Err(e) => {
                opt_session.failure_streak += 1;
                session
                    .store
                    .set_failure_streak(opt_session.failure_streak)?;
                exp.status = ExperimentStatus::Failed;
                exp.decision_reason = e.to_string();
                session.store.upsert(&exp)?;
                eprintln_human(out, format!("Experiment failed: {e}"));
                continue;
            }
        };

        fill_experiment_from_suite(&mut exp, suite, Some(&baseline_scores), cfg.max_cov)?;
        decide_candidate(&mut exp);
        let score = exp.index_summary.as_ref().map(|s| s.mean).unwrap_or(0.0);
        last_class = exp.comparison_class;

        let improved = exp.decision == Decision::Accept && score > best_score;
        if improved {
            best_score = score;
            best_candidate = candidate.clone();
            best_exp_id = exp.id.to_string();
            plateau = 0;
            println_human(
                out,
                format!("  -> provisional best {score:.1} ({})", exp.decision_reason),
            );
        } else {
            plateau += 1;
            println_human(
                out,
                format!(
                    "  -> {} (score {score:.1}, plateau {plateau})",
                    exp.decision_reason
                ),
            );
            if exp.decision == Decision::Accept {
                exp.decision = Decision::Reject;
                exp.decision_reason.push_str(" (not best so far; deferred)");
            }
        }
        exp.recovery_token = None;
        session.store.upsert(&exp)?;

        opt_session.checkpoint = Some(OptimizeCheckpoint {
            generation,
            experiments_done,
            plateau,
            best_score,
            best_candidate: best_candidate.clone(),
            best_experiment_id: Some(best_exp_id.clone()),
            baseline_id: baseline.id.to_string(),
            last_experiment_id: Some(exp.id.to_string()),
        });
        opt_session.touch();
        session.store.upsert_session(&opt_session)?;
    }

    if !best_candidate.is_identity() {
        println_human(
            out,
            format!(
                "Validating best candidate with {} samples: {}",
                cfg.validation_samples,
                best_candidate.summary_line()
            ),
        );
        let mut exp = Experiment::new_candidate(&baseline, best_candidate.clone());
        exp.kind = kraftverk_core::experiment::ExperimentKind::Validation;
        exp.status = ExperimentStatus::Running;

        let guard = ApplyGuard::apply(
            &mut session.platform,
            best_candidate.clone(),
            &mut session.journal,
            &exp.id.to_string(),
        )?;

        let mut progress = CliProgress::new(out.quiet, out.json);
        let run_cfg = RunConfig {
            warmup: cfg.run.warmup,
            samples: cfg.validation_samples,
            seed: cfg.run.seed ^ 0x56414C31,
            ..cfg.run.clone()
        };
        let suite = run_measured(&run_cfg, Some(baseline_raw), &mut progress, 1.0)?;
        fill_experiment_from_suite(&mut exp, suite, Some(&baseline_scores), cfg.max_cov)?;
        decide_candidate(&mut exp);

        if exp.decision == Decision::Accept && exp.stability == StabilityVerdict::Pass {
            guard.commit()?;
            session
                .store
                .set_active_config(&serde_json::to_string(&best_candidate)?)?;
            println_human(
                out,
                format!(
                    "ACCEPTED {} — Kraft Index {:.1} ({})",
                    exp.id,
                    exp.index_summary.as_ref().map(|s| s.mean).unwrap_or(0.0),
                    exp.decision_reason
                ),
            );
            accepted = Some(exp.clone());
            session.store.set_failure_streak(0)?;
            match write_receipt(&exp, None) {
                Ok((path, receipt)) => {
                    println_human(
                        out,
                        format!(
                            "Evidence receipt: {} (hash {})",
                            path.display(),
                            &receipt.evidence_hash[..12.min(receipt.evidence_hash.len())]
                        ),
                    );
                }
                Err(e) => eprintln_human(out, format!("Warning: could not write receipt: {e}")),
            }
        } else {
            guard.rollback()?;
            exp.decision = Decision::Reject;
            if exp.decision_reason.is_empty() {
                exp.decision_reason = "failed validation".into();
            }
            eprintln_human(
                out,
                format!(
                    "Validation rejected: {} / stability {}",
                    exp.decision_reason,
                    exp.stability.as_str()
                ),
            );
            opt_session.failure_streak += 1;
            session
                .store
                .set_failure_streak(opt_session.failure_streak)?;
        }
        session.store.upsert(&exp)?;
        best_exp_id = exp.id.to_string();
        best_score = exp
            .index_summary
            .as_ref()
            .map(|s| s.mean)
            .unwrap_or(best_score);
    } else {
        println_human(
            out,
            "No improving candidate found; baseline configuration retained.",
        );
    }

    let improvement = (best_score / BASELINE_INDEX - 1.0) * 100.0;
    let workers = best_candidate
        .changes
        .iter()
        .find(|c| c.key == "bench.worker_threads")
        .and_then(|c| c.next.as_usize());
    let objective = score_objective(goal, best_score, workers, None);

    opt_session.status = SessionStatus::Completed;
    opt_session.touch();
    session.store.upsert_session(&opt_session)?;

    if out.json {
        print_json(&serde_json::json!({
            "ok": true,
            "mode": mode.as_str(),
            "goal": goal.as_str(),
            "session_id": opt_session.id.to_string(),
            "baseline_id": baseline.id.to_string(),
            "best_experiment_id": best_exp_id,
            "best_score": best_score,
            "improvement_pct": improvement,
            "accepted": accepted.as_ref().map(|e| e.id.to_string()),
            "candidate": best_candidate,
            "experiments_done": experiments_done,
            "stability": accepted.as_ref().map(|e| e.stability.as_str()),
            "objective": objective,
            "constraint_unchecked": constraints.check_environment(None, None).unchecked,
        }));
    } else {
        println_human(out, "--- Optimize summary ---");
        println_human(out, format!("Session: {}", opt_session.id));
        println_human(out, format!("Baseline Kraft Index: {BASELINE_INDEX:.0}"));
        println_human(
            out,
            format!("Best measured: {best_score:.1} ({improvement:+.2}%)"),
        );
        println_human(out, format!("Best experiment: {best_exp_id}"));
        println_human(out, format!("Changes: {}", best_candidate.summary_line()));
        if let Some(eff) = objective.efficiency {
            println_human(out, format!("Efficiency proxy (score/worker): {eff:.1}"));
        }
        for n in &objective.notes {
            println_human(out, format!("Note: {n}"));
        }
        if let Some(a) = accepted {
            println_human(
                out,
                format!("Status: ACCEPTED (stability {})", a.stability.as_str()),
            );
        } else {
            println_human(
                out,
                "Status: no accepted change (all reversible trials rolled back)",
            );
        }
    }

    let _ = mean_raw;
    Ok(())
}
