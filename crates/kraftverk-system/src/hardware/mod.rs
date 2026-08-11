//! Machine inspection, fingerprinting, and AMD-only eligibility.

pub mod eligibility;
pub mod fingerprint;
pub mod inspect;

pub use eligibility::{
    detect_architecture, detect_cpu_vendor, detect_gpu_devices, evaluate_eligibility,
    evaluate_from_facts, exit_code_for, probe_amd_capabilities, recheck_eligibility, Architecture,
    CompatibilityStatus, CpuVendor, DetectedGpu, ExitCode, GpuVendor, HardwareEligibility,
    HardwareFacts, HardwareRejection, HotplugAction, SessionGuard, HARDWARE_POLICY,
};
pub use fingerprint::machine_fingerprint;
pub use inspect::{inspect_machine, GpuInfo, MachineReport, SensorStatus};
