//! Platform trait and apply-guard for reversible changes.

use kraftverk_core::candidate::{Candidate, ParamChange, ParamValue};
use kraftverk_core::error::Result;
use kraftverk_core::OptimizeMode;

use crate::capabilities::Capabilities;
use crate::recovery::RecoveryJournal;
use crate::topology::Topology;

/// Effect of a tuner parameter on measured performance (for MockPlatform).
#[derive(Debug, Clone)]
pub struct TunerEffect {
    pub key: String,
    /// Multiplicative factor applied to Kraft Index raw composite when this value is active.
    pub score_multiplier: f64,
}

/// OS / environment boundary for discovery and reversible tuning.
pub trait Platform: Send {
    fn name(&self) -> &str;
    fn capabilities(&self) -> Capabilities;
    fn topology(&self) -> Result<Topology>;

    /// Read the current value of a tunable parameter.
    fn read_param(&self, key: &str) -> Result<ParamValue>;

    /// Apply a single change. Must be reversible via [`rollback_change`].
    fn apply_change(&mut self, change: &ParamChange) -> Result<()>;

    /// Verify the parameter currently equals `change.next`.
    fn verify_change(&self, change: &ParamChange) -> Result<bool>;

    /// Restore `change.previous`.
    fn rollback_change(&mut self, change: &ParamChange) -> Result<()>;

    /// Optional simulated score multiplier (MockPlatform); native returns 1.0.
    fn score_multiplier(&self) -> f64 {
        1.0
    }

    /// Whether this mode is allowed on this platform right now.
    fn mode_allowed(&self, mode: OptimizeMode) -> Result<()> {
        if !mode.supported_without_agent() {
            return Err(kraftverk_core::Error::unsupported(format!(
                "optimize mode '{}' is not available in Milestone 1 (use --mode safe)",
                mode.as_str()
            )));
        }
        Ok(())
    }

    /// Keys that may be searched in safe mode.
    fn safe_param_keys(&self) -> Vec<String> {
        self.capabilities()
            .features
            .iter()
            .filter(|f| {
                f.support.is_usable()
                    && matches!(
                        f.id.as_str(),
                        "bench.worker_threads"
                            | "bench.rayon_threads"
                            | "process.priority"
                            | "process.affinity"
                    )
            })
            .map(|f| f.id.clone())
            .collect()
    }
}

/// RAII guard: applies a candidate, writes recovery journal, rolls back on drop unless committed.
pub struct ApplyGuard<'a, P: Platform> {
    platform: &'a mut P,
    candidate: Candidate,
    journal: &'a mut RecoveryJournal,
    committed: bool,
    rolled_back: bool,
}

impl<'a, P: Platform> ApplyGuard<'a, P> {
    pub fn apply(
        platform: &'a mut P,
        candidate: Candidate,
        journal: &'a mut RecoveryJournal,
        experiment_id: &str,
    ) -> Result<Self> {
        journal.begin(experiment_id, &candidate)?;
        for change in &candidate.changes {
            platform.apply_change(change).map_err(|e| {
                // Best-effort rollback of already applied prefix.
                let _ = rollback_prefix(platform, &candidate, change);
                let _ = journal.fail(&e.to_string());
                e
            })?;
            if !platform.verify_change(change)? {
                let _ = rollback_prefix(platform, &candidate, change);
                let _ = platform.rollback_change(change);
                let err =
                    kraftverk_core::Error::Platform(format!("verify failed for {}", change.key));
                let _ = journal.fail(&err.to_string());
                return Err(err);
            }
            journal.record_applied(change)?;
        }
        Ok(Self {
            platform,
            candidate,
            journal,
            committed: false,
            rolled_back: false,
        })
    }

    pub fn commit(mut self) -> Result<()> {
        self.journal.complete()?;
        self.committed = true;
        Ok(())
    }

    pub fn rollback(mut self) -> Result<()> {
        self.rollback_inner()?;
        Ok(())
    }

    fn rollback_inner(&mut self) -> Result<()> {
        if self.rolled_back || self.committed {
            return Ok(());
        }
        for change in self.candidate.changes.iter().rev() {
            self.platform.rollback_change(change)?;
        }
        self.journal.rolled_back()?;
        self.rolled_back = true;
        Ok(())
    }

    pub fn candidate(&self) -> &Candidate {
        &self.candidate
    }
}

impl<'a, P: Platform> Drop for ApplyGuard<'a, P> {
    fn drop(&mut self) {
        if !self.committed && !self.rolled_back {
            let _ = self.rollback_inner();
        }
    }
}

fn rollback_prefix<P: Platform>(
    platform: &mut P,
    candidate: &Candidate,
    failed_at: &ParamChange,
) -> Result<()> {
    for change in candidate.changes.iter() {
        if change.key == failed_at.key {
            break;
        }
        platform.rollback_change(change)?;
    }
    Ok(())
}
