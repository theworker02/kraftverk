//! Stable facade re-exports for embedding Kraftverk.
//!
//! API stability policy: see `docs/api-stability.md`.
//! Prefer this crate over reaching into internal modules when integrating.

use kraftverk_core::error::{Error, Result};

pub use kraftverk_agent as agent;
pub use kraftverk_bench as bench;
pub use kraftverk_core as core;
pub use kraftverk_data as data;
pub use kraftverk_optimizer as optimizer;
pub use kraftverk_system as system;

pub use kraftverk_agent::{agent_connected, trust_boundary_summary, AgentRequest, AgentResponse};

pub use kraftverk_core::{
    Candidate, ComparisonClass, Experiment, ExperimentId, KraftIndex, OptimizeConfig,
    OptimizeConstraints, OptimizeGoal, OptimizeMode, OptimizeSession, RunConfig, BASELINE_INDEX,
};

pub use kraftverk_bench::{run_benchmark_suite, run_suite_samples, WorkloadConfig};
pub use kraftverk_data::{
    default_db_path, load_receipt, report_html, report_json, write_receipt, EvidenceReceipt,
    ExperimentStore,
};
pub use kraftverk_optimizer::{
    create_search_plugin, list_search_plugins, SearchPluginInfo, PLUGIN_HILL_CLIMB,
};
pub use kraftverk_system::{
    capture_snapshot, detect_platform, evaluate_eligibility, exit_code_for, inspect_machine,
    Architecture, CompatibilityStatus, CpuVendor, ExitCode, GpuVendor, HardwareEligibility,
    HardwareFacts, HardwareRejection, HotplugAction, MachineReport, MockPlatform, NativePlatform,
    Platform, SessionGuard, HARDWARE_POLICY,
};

/// Library version string.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Structured unsupported-hardware failure for embedders.
#[derive(Debug, Clone)]
pub struct UnsupportedHardware {
    pub message: String,
    pub eligibility: Box<HardwareEligibility>,
    pub exit_code: i32,
}

impl std::fmt::Display for UnsupportedHardware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for UnsupportedHardware {}

impl From<UnsupportedHardware> for Error {
    fn from(u: UnsupportedHardware) -> Self {
        Error::UnsupportedHardware(u.message)
    }
}

impl UnsupportedHardware {
    pub fn from_eligibility(eligibility: HardwareEligibility) -> Self {
        let exit_code = eligibility
            .primary_rejection()
            .map(|r| exit_code_for(r).as_i32())
            .unwrap_or(25);
        Self {
            message: eligibility.summary(),
            eligibility: Box::new(eligibility),
            exit_code,
        }
    }
}

/// Primary SDK handle. Opening fails on unsupported hardware (amd-only-v1).
pub struct Kraftverk {
    pub eligibility: HardwareEligibility,
    pub fingerprint: String,
    guard: SessionGuard,
}

impl Kraftverk {
    /// Open the default local Kraftverk context after enforcing hardware eligibility.
    pub fn open_default() -> Result<Self> {
        let guard = SessionGuard::start()?;
        let report = inspect_machine(VERSION);
        Ok(Self {
            eligibility: guard.initial.clone(),
            fingerprint: report.fingerprint,
            guard,
        })
    }

    /// Hardware policy identifier (`amd-only-v1`).
    pub fn hardware_policy(&self) -> &'static str {
        HARDWARE_POLICY
    }

    /// Re-check hardware identity before sensitive operations.
    pub fn revalidate(&self) -> Result<HardwareEligibility> {
        self.guard.assert_still_supported()
    }
}

/// Convenience: evaluate eligibility without opening a session.
pub fn check_hardware() -> HardwareEligibility {
    evaluate_eligibility()
}

/// Require supported hardware or return [`UnsupportedHardware`].
pub fn require_supported_hardware() -> std::result::Result<HardwareEligibility, UnsupportedHardware>
{
    let el = evaluate_eligibility();
    if el.supported {
        Ok(el)
    } else {
        Err(UnsupportedHardware::from_eligibility(el))
    }
}

/// Map kraftverk-core errors into a stable SDK alias.
pub type KraftverkError = Error;
