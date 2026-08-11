//! MockPlatform-style hardware combination tests for AMD-only eligibility.
//!
//! These do not require live AMD hardware — they inject HardwareFacts.

use kraftverk_system::{
    evaluate_from_facts, Architecture, CompatibilityStatus, CpuVendor, GpuVendor, HardwareFacts,
    HARDWARE_POLICY,
};

fn facts(arch: Architecture, cpu: CpuVendor, raw: &str, gpus: Vec<GpuVendor>) -> HardwareFacts {
    HardwareFacts {
        architecture: arch,
        cpu_vendor: cpu,
        cpu_vendor_raw: raw.into(),
        gpu_details: gpus
            .iter()
            .map(|g| format!("mock-{}", g.as_str()))
            .collect(),
        gpu_vendors: gpus,
    }
}

#[test]
fn mock_amd_amd_x86_64_passes() {
    let e = evaluate_from_facts(&facts(
        Architecture::X86_64,
        CpuVendor::Amd,
        "AuthenticAMD",
        vec![GpuVendor::Amd],
    ));
    assert!(e.supported);
    assert_eq!(e.compatibility, CompatibilityStatus::Supported);
    assert_eq!(e.policy, HARDWARE_POLICY);
}

#[test]
fn mock_amd_no_gpu_passes() {
    let e = evaluate_from_facts(&facts(
        Architecture::X86_64,
        CpuVendor::Amd,
        "AuthenticAMD",
        vec![],
    ));
    assert!(e.supported, "{:?}", e.rejection_reasons);
}

#[test]
fn mock_amd_nvidia_fails() {
    let e = evaluate_from_facts(&facts(
        Architecture::X86_64,
        CpuVendor::Amd,
        "AuthenticAMD",
        vec![GpuVendor::Amd, GpuVendor::Nvidia],
    ));
    assert!(!e.supported);
}

#[test]
fn mock_intel_amd_fails() {
    let e = evaluate_from_facts(&facts(
        Architecture::X86_64,
        CpuVendor::Intel,
        "GenuineIntel",
        vec![GpuVendor::Amd],
    ));
    assert!(!e.supported);
}

#[test]
fn mock_intel_nvidia_fails() {
    let e = evaluate_from_facts(&facts(
        Architecture::X86_64,
        CpuVendor::Intel,
        "GenuineIntel",
        vec![GpuVendor::Nvidia],
    ));
    assert!(!e.supported);
}

#[test]
fn mock_amd_intel_gpu_fails() {
    let e = evaluate_from_facts(&facts(
        Architecture::X86_64,
        CpuVendor::Amd,
        "AuthenticAMD",
        vec![GpuVendor::Amd, GpuVendor::Intel],
    ));
    assert!(!e.supported);
}

#[test]
fn mock_arm_fails() {
    let e = evaluate_from_facts(&facts(
        Architecture::Arm64,
        CpuVendor::Unknown,
        "unknown",
        vec![],
    ));
    assert!(!e.supported);
}

#[test]
fn mock_unknown_cpu_fails() {
    let e = evaluate_from_facts(&facts(
        Architecture::X86_64,
        CpuVendor::Unknown,
        "SomeCpu",
        vec![],
    ));
    assert!(!e.supported);
}

#[test]
fn mock_multi_amd_gpu_passes() {
    let e = evaluate_from_facts(&facts(
        Architecture::X86_64,
        CpuVendor::Amd,
        "AuthenticAMD",
        vec![GpuVendor::Amd, GpuVendor::Amd],
    ));
    assert!(e.supported, "{:?}", e.rejection_reasons);
}

#[test]
fn mock_nvidia_only_exit_22() {
    let e = evaluate_from_facts(&facts(
        Architecture::X86_64,
        CpuVendor::Amd,
        "AuthenticAMD",
        vec![GpuVendor::Nvidia],
    ));
    assert!(!e.supported);
    assert_eq!(
        kraftverk_system::exit_code_for(e.primary_rejection().unwrap()).as_i32(),
        22
    );
}

#[test]
fn mock_intel_cpu_exit_21() {
    let e = evaluate_from_facts(&facts(
        Architecture::X86_64,
        CpuVendor::Intel,
        "GenuineIntel",
        vec![],
    ));
    assert_eq!(
        kraftverk_system::exit_code_for(e.primary_rejection().unwrap()).as_i32(),
        21
    );
}
