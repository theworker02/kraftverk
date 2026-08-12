//! Shared engine: recovery, store open, suite wiring.

use anyhow::{anyhow, Context, Result};
use kraftverk_agent::AgentBackedPlatform;
use kraftverk_bench::{run_suite_samples, BenchProgress, WorkloadConfig};
use kraftverk_core::config::RunConfig;
use kraftverk_core::experiment::{Decision, Experiment, ExperimentStatus, StabilityVerdict};
use kraftverk_core::kraft_index::KraftIndexWeights;
use kraftverk_core::statistics::{compare_samples, summarize, StatsConfig};
use kraftverk_data::{bench_scratch_dir, default_db_path, recovery_journal_path, ExperimentStore};
use kraftverk_system::inspect_machine;
use kraftverk_system::RecoveryJournal;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct Session {
    pub store: ExperimentStore,
    pub platform: AgentBackedPlatform,
    pub journal: RecoveryJournal,
    pub report_fingerprint: String,
    pub os_info: String,
    pub eligibility_policy: String,
    pub session_guard: kraftverk_system::SessionGuard,
}

pub fn open_session() -> Result<Session> {
    // Revalidate hardware identity before privileged/optimize sessions.
    let guard = kraftverk_system::SessionGuard::start().map_err(|e| anyhow!(e.to_string()))?;
    let report = inspect_machine(VERSION);
    let store = ExperimentStore::open(default_db_path()?)?;
    let mut platform = AgentBackedPlatform::detect().map_err(|e| anyhow!(e.to_string()))?;
    let mut journal = RecoveryJournal::open(recovery_journal_path()?)?;

    if let Some(id) = journal.recover_with(&mut platform)? {
        tracing::warn!(experiment = %id, "recovered interrupted experiment; changes rolled back");
    }

    // Hot-plug recheck after recovery restore.
    let _ = guard
        .assert_still_supported()
        .map_err(|e| anyhow!(e.to_string()))?;

    Ok(Session {
        store,
        platform,
        journal,
        report_fingerprint: report.fingerprint.clone(),
        os_info: format!(
            "{} {} ({})",
            report.os_family, report.os_version, report.arch
        ),
        eligibility_policy: kraftverk_system::HARDWARE_POLICY.to_string(),
        session_guard: guard,
    })
}

pub struct CliProgress<'a> {
    pub quiet: bool,
    pub json: bool,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> CliProgress<'a> {
    pub fn new(quiet: bool, json: bool) -> Self {
        Self {
            quiet,
            json,
            _marker: std::marker::PhantomData,
        }
    }
}

impl BenchProgress for CliProgress<'_> {
    fn on_event(&mut self, message: &str) {
        if !self.quiet && !self.json {
            eprintln!("… {message}");
        }
    }
}

pub fn make_workload(run: &RunConfig) -> Result<WorkloadConfig> {
    let mut cfg = WorkloadConfig::from_run_config(run);
    cfg.storage_dir = Some(bench_scratch_dir()?);
    Ok(cfg)
}

pub fn run_measured(
    run: &RunConfig,
    baseline_raw: Option<f64>,
    progress: &mut dyn BenchProgress,
    score_multiplier: f64,
) -> Result<kraftverk_bench::SuiteResult> {
    let workload = make_workload(run)?;
    // Suite runner upgrades to with_gpu() when real GPU measurements exist.
    let weights = KraftIndexWeights::default();
    Ok(run_suite_samples(
        run,
        &workload,
        &weights,
        baseline_raw,
        progress,
        score_multiplier,
    )?)
}

pub fn fill_experiment_from_suite(
    exp: &mut Experiment,
    suite: kraftverk_bench::SuiteResult,
    baseline_scores: Option<&[f64]>,
    max_cov: f64,
) -> Result<()> {
    exp.samples = suite.samples;
    exp.index_samples = suite.index_samples_normalized.clone();
    exp.index_summary = suite.index_summary.clone();
    exp.kraft_index = suite.final_index.clone();
    exp.telemetry
        .push(kraftverk_system::telemetry::snapshot_json());

    if let Some(base) = baseline_scores {
        let cmp = compare_samples(
            base,
            &suite.index_samples_normalized,
            &StatsConfig::default(),
        )?;
        exp.comparison = Some(cmp.clone());
        exp.comparison_class = Some(cmp.class);
    }

    // Stability: CoV and finite scores.
    let cov = exp
        .index_summary
        .as_ref()
        .map(|s| s.cov)
        .unwrap_or(f64::INFINITY);
    let checksum_ok = exp
        .samples
        .iter()
        .flat_map(|s| s.measurements.iter())
        .filter_map(|m| m.checksum.as_ref())
        .count()
        > 0;

    exp.stability = if cov <= max_cov && checksum_ok && exp.index_samples.len() >= 2 {
        StabilityVerdict::Pass
    } else if cov > max_cov {
        StabilityVerdict::Fail
    } else {
        StabilityVerdict::Unknown
    };

    exp.status = ExperimentStatus::Completed;
    exp.completed_at = Some(chrono::Utc::now());
    exp.touch();
    Ok(())
}

pub fn require_baseline(session: &Session) -> Result<Experiment> {
    session
        .store
        .latest_baseline(&session.report_fingerprint)?
        .ok_or_else(|| anyhow!("no baseline found; run `kraftverk baseline` first"))
}

pub fn mean_raw(samples: &[f64]) -> Result<f64> {
    if samples.is_empty() {
        return Err(anyhow!("empty samples"));
    }
    Ok(samples.iter().sum::<f64>() / samples.len() as f64)
}

pub fn decide_candidate(exp: &mut Experiment) {
    let Some(cmp) = exp.comparison.as_ref() else {
        exp.decision = Decision::Reject;
        exp.decision_reason = "missing comparison".into();
        return;
    };
    if exp.stability != StabilityVerdict::Pass {
        exp.decision = Decision::Reject;
        exp.decision_reason = format!("stability {}", exp.stability.as_str());
        return;
    }
    if cmp.class.is_improvement() {
        exp.decision = Decision::Accept;
        exp.decision_reason = format!(
            "{} ({:+.2}%)",
            cmp.class.as_str(),
            cmp.relative_change * 100.0
        );
    } else {
        exp.decision = Decision::Reject;
        exp.decision_reason = format!(
            "{} ({:+.2}%)",
            cmp.class.as_str(),
            cmp.relative_change * 100.0
        );
    }
}

#[allow(dead_code)]
pub fn summarize_or_err(samples: &[f64]) -> Result<kraftverk_core::SampleSummary> {
    Ok(summarize(samples, &StatsConfig::default())?)
}

pub fn load_experiment(store: &ExperimentStore, id: &str) -> Result<Experiment> {
    store
        .get_str(id)?
        .with_context(|| format!("experiment '{id}' not found"))
}
