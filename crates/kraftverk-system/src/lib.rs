//! Kraftverk system layer: platform, hardware inspection, and telemetry.
//!
//! Consolidated from kraftverk-platform, kraftverk-hardware, and kraftverk-telemetry
//! to keep the first-party crate count within the product limit.

// Build-time architecture gate (vendor checks remain runtime-only).
#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
compile_error!(
    "Kraftverk only builds for x86 / x86_64 targets (AMD-exclusive platform). \
     ARM, ARM64, RISC-V, and other architectures are unsupported."
);

pub mod hardware;
pub mod platform;
pub mod telemetry;

// Platform surface (primary)
pub use platform::{capabilities, host, mock, native, recovery, topology, tuner};
pub use platform::{
    detect_platform, ApplyGuard, Capabilities, Capability, CpuTopology, FeatureSupport,
    MockPlatform, NativePlatform, Platform, RecoveryJournal, RecoveryRecord, Topology, TunerEffect,
};

// Hardware convenience re-exports
pub use hardware::{
    detect_architecture, detect_cpu_vendor, detect_gpu_devices, evaluate_eligibility,
    evaluate_from_facts, exit_code_for, inspect_machine, machine_fingerprint,
    probe_amd_capabilities, recheck_eligibility, Architecture, CompatibilityStatus, CpuVendor,
    DetectedGpu, ExitCode, GpuInfo, GpuVendor, HardwareEligibility, HardwareFacts,
    HardwareRejection, HotplugAction, MachineReport, SensorStatus, SessionGuard, HARDWARE_POLICY,
};

// Telemetry convenience
pub use telemetry::{
    capture_snapshot, environment_suitable_for_bench, snapshot_json, NoiseEstimate,
    TelemetrySnapshot,
};
