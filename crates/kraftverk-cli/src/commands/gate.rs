//! Shared hardware eligibility gate for CLI commands.

use anyhow::{anyhow, Result};
use kraftverk_system::{
    evaluate_eligibility, exit_code_for, HardwareEligibility, SessionGuard, HARDWARE_POLICY,
};

/// Error carrying a documented hardware exit code (20–25).
#[derive(Debug)]
pub struct HardwareGateError {
    pub exit_code: i32,
    pub eligibility: Box<HardwareEligibility>,
    pub message: String,
}

impl std::fmt::Display for HardwareGateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for HardwareGateError {}

impl HardwareGateError {
    pub fn from_eligibility(eligibility: HardwareEligibility) -> Self {
        let exit_code = eligibility
            .primary_rejection()
            .map(|r| exit_code_for(r).as_i32())
            .unwrap_or(25);
        let message = eligibility.summary();
        Self {
            exit_code,
            eligibility: Box::new(eligibility),
            message,
        }
    }
}

/// Enforce AMD-only policy. Returns eligibility snapshot on success.
pub fn require_eligible() -> Result<HardwareEligibility, HardwareGateError> {
    let eligibility = evaluate_eligibility();
    if eligibility.supported {
        Ok(eligibility)
    } else {
        Err(HardwareGateError::from_eligibility(eligibility))
    }
}

/// Start a session guard (for optimizer revalidation / hot-plug).
#[allow(dead_code)]
pub fn start_session_guard() -> Result<SessionGuard> {
    SessionGuard::start().map_err(|e| anyhow!("{e}"))
}

/// Inspect-only summary (never fails the process by itself).
#[allow(dead_code)]
pub fn inspect_eligibility() -> HardwareEligibility {
    evaluate_eligibility()
}

#[allow(dead_code)]
pub fn policy_id() -> &'static str {
    HARDWARE_POLICY
}
