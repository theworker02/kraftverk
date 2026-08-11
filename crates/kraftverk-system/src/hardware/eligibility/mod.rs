//! Central AMD-only hardware eligibility subsystem.
//!
//! All product surfaces (CLI, desktop, SDK, agent, optimizer) must consult this
//! module — do not scatter ad-hoc vendor checks elsewhere.

pub mod amd;
pub mod detect;
pub mod evaluate;
pub mod exit_codes;
pub mod hotplug;
#[cfg(any(test, feature = "mock-platform"))]
pub mod mock_matrix;
pub mod types;

pub use amd::{probe_amd_capabilities, AmdCapabilities, AmdCpuTopologyHints};
pub use detect::{detect_architecture, detect_cpu_vendor, detect_gpu_devices, DetectedGpu};
pub use evaluate::{evaluate_eligibility, evaluate_from_facts, HardwareFacts};
pub use exit_codes::{exit_code_for, ExitCode};
pub use hotplug::{recheck_eligibility, HotplugAction, SessionGuard};
#[cfg(any(test, feature = "mock-platform"))]
pub use mock_matrix::{eligibility_matrix_cases, MockHardwareIdentity};
pub use types::{
    Architecture, CompatibilityStatus, CpuVendor, GpuVendor, HardwareEligibility,
    HardwareRejection, HARDWARE_POLICY,
};
