//! Hot-plug / mid-session hardware revalidation.

use kraftverk_core::error::{Error, Result};

use super::evaluate::evaluate_eligibility;
use super::types::{GpuVendor, HardwareEligibility, HardwareRejection};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotplugAction {
    Continue,
    /// NVIDIA (or other forbidden GPU) appeared — stop, restore, block.
    AbortBlock {
        reason: String,
    },
}

/// Re-check live hardware identity. If NVIDIA appears mid-session, abort.
pub fn recheck_eligibility(previous: &HardwareEligibility) -> (HardwareEligibility, HotplugAction) {
    let current = evaluate_eligibility();
    if current.supported {
        return (current, HotplugAction::Continue);
    }

    let nvidia_now = current
        .rejection_reasons
        .iter()
        .any(|r| matches!(r, HardwareRejection::NvidiaGpuDetected { .. }))
        || current.gpu_vendors.contains(&GpuVendor::Nvidia);

    let nvidia_before = previous.gpu_vendors.contains(&GpuVendor::Nvidia);

    if nvidia_now && !nvidia_before {
        let reason = current
            .primary_rejection()
            .map(|r| r.message())
            .unwrap_or_else(|| "NVIDIA GPU appeared mid-session".into());
        return (
            current,
            HotplugAction::AbortBlock {
                reason: format!(
                    "Hot-plug policy: {reason}. Stopping experiments, reverting managed config, \
                     blocking further execution."
                ),
            },
        );
    }

    // Any newly unsupported configuration also blocks.
    let reason = current.summary();
    (
        current,
        HotplugAction::AbortBlock {
            reason: format!("Hardware eligibility lost mid-session: {reason}"),
        },
    )
}

/// Session-scoped guard that tracks the eligibility snapshot at session start
/// and revalidates before sensitive operations.
pub struct SessionGuard {
    pub initial: HardwareEligibility,
}

impl SessionGuard {
    pub fn start() -> Result<Self> {
        let initial = evaluate_eligibility();
        if !initial.supported {
            return Err(Error::UnsupportedHardware(initial.summary()));
        }
        Ok(Self { initial })
    }

    pub fn assert_still_supported(&self) -> Result<HardwareEligibility> {
        let (current, action) = recheck_eligibility(&self.initial);
        match action {
            HotplugAction::Continue => Ok(current),
            HotplugAction::AbortBlock { reason } => Err(Error::UnsupportedHardware(reason)),
        }
    }
}
