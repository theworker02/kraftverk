//! MockPlatform / mock-hardware eligibility matrix (tests only).
//!
//! Enabled via `--features mock-platform`. Never ships as a production bypass.

#![cfg(any(test, feature = "mock-platform"))]

use super::evaluate::{evaluate_from_facts, HardwareFacts};
use super::types::{
    Architecture, CompatibilityStatus, CpuVendor, GpuVendor, HardwareEligibility, HARDWARE_POLICY,
};

/// Injectable hardware identity for deterministic eligibility tests.
#[derive(Debug, Clone)]
pub struct MockHardwareIdentity {
    pub architecture: Architecture,
    pub cpu_vendor: CpuVendor,
    pub cpu_vendor_raw: String,
    pub gpu_vendors: Vec<GpuVendor>,
    pub gpu_details: Vec<String>,
}

impl MockHardwareIdentity {
    pub fn amd_x86_64_no_gpu() -> Self {
        Self {
            architecture: Architecture::X86_64,
            cpu_vendor: CpuVendor::Amd,
            cpu_vendor_raw: "AuthenticAMD".into(),
            gpu_vendors: vec![],
            gpu_details: vec![],
        }
    }

    pub fn to_facts(&self) -> HardwareFacts {
        HardwareFacts {
            architecture: self.architecture,
            cpu_vendor: self.cpu_vendor,
            cpu_vendor_raw: self.cpu_vendor_raw.clone(),
            gpu_vendors: self.gpu_vendors.clone(),
            gpu_details: if self.gpu_details.is_empty() {
                self.gpu_vendors
                    .iter()
                    .map(|v| v.as_str().to_string())
                    .collect()
            } else {
                self.gpu_details.clone()
            },
        }
    }

    pub fn evaluate(&self) -> HardwareEligibility {
        evaluate_from_facts(&self.to_facts())
    }
}

/// Full vendor/arch matrix covering the amd-only-v1 policy surface.
pub fn eligibility_matrix_cases() -> Vec<(&'static str, MockHardwareIdentity, bool)> {
    vec![
        (
            "amd_x86_64_no_gpu",
            MockHardwareIdentity::amd_x86_64_no_gpu(),
            true,
        ),
        (
            "amd_x86_no_gpu",
            MockHardwareIdentity {
                architecture: Architecture::X86,
                cpu_vendor: CpuVendor::Amd,
                cpu_vendor_raw: "AuthenticAMD".into(),
                gpu_vendors: vec![],
                gpu_details: vec![],
            },
            true,
        ),
        (
            "amd_x86_64_amd_gpu",
            MockHardwareIdentity {
                architecture: Architecture::X86_64,
                cpu_vendor: CpuVendor::Amd,
                cpu_vendor_raw: "AuthenticAMD".into(),
                gpu_vendors: vec![GpuVendor::Amd],
                gpu_details: vec!["Radeon (0x1002)".into()],
            },
            true,
        ),
        (
            "amd_x86_64_multi_amd_gpu",
            MockHardwareIdentity {
                architecture: Architecture::X86_64,
                cpu_vendor: CpuVendor::Amd,
                cpu_vendor_raw: "AuthenticAMD".into(),
                gpu_vendors: vec![GpuVendor::Amd, GpuVendor::Amd],
                gpu_details: vec!["GPU0".into(), "GPU1".into()],
            },
            true,
        ),
        (
            "intel_cpu_blocked",
            MockHardwareIdentity {
                architecture: Architecture::X86_64,
                cpu_vendor: CpuVendor::Intel,
                cpu_vendor_raw: "GenuineIntel".into(),
                gpu_vendors: vec![],
                gpu_details: vec![],
            },
            false,
        ),
        (
            "amd_nvidia_blocked",
            MockHardwareIdentity {
                architecture: Architecture::X86_64,
                cpu_vendor: CpuVendor::Amd,
                cpu_vendor_raw: "AuthenticAMD".into(),
                gpu_vendors: vec![GpuVendor::Nvidia],
                gpu_details: vec!["RTX (0x10DE)".into()],
            },
            false,
        ),
        (
            "amd_amd_plus_nvidia_blocked",
            MockHardwareIdentity {
                architecture: Architecture::X86_64,
                cpu_vendor: CpuVendor::Amd,
                cpu_vendor_raw: "AuthenticAMD".into(),
                gpu_vendors: vec![GpuVendor::Amd, GpuVendor::Nvidia],
                gpu_details: vec!["Radeon".into(), "NVIDIA".into()],
            },
            false,
        ),
        (
            "amd_intel_gpu_blocked",
            MockHardwareIdentity {
                architecture: Architecture::X86_64,
                cpu_vendor: CpuVendor::Amd,
                cpu_vendor_raw: "AuthenticAMD".into(),
                gpu_vendors: vec![GpuVendor::Intel],
                gpu_details: vec!["UHD (0x8086)".into()],
            },
            false,
        ),
        (
            "amd_amd_plus_intel_gpu_blocked",
            MockHardwareIdentity {
                architecture: Architecture::X86_64,
                cpu_vendor: CpuVendor::Amd,
                cpu_vendor_raw: "AuthenticAMD".into(),
                gpu_vendors: vec![GpuVendor::Amd, GpuVendor::Intel],
                gpu_details: vec!["Radeon".into(), "Intel".into()],
            },
            false,
        ),
        (
            "unknown_cpu_blocked",
            MockHardwareIdentity {
                architecture: Architecture::X86_64,
                cpu_vendor: CpuVendor::Unknown,
                cpu_vendor_raw: "SomeOther".into(),
                gpu_vendors: vec![],
                gpu_details: vec![],
            },
            false,
        ),
        (
            "arm64_blocked",
            MockHardwareIdentity {
                architecture: Architecture::Arm64,
                cpu_vendor: CpuVendor::Amd,
                cpu_vendor_raw: "AuthenticAMD".into(),
                gpu_vendors: vec![],
                gpu_details: vec![],
            },
            false,
        ),
        (
            "arm_blocked",
            MockHardwareIdentity {
                architecture: Architecture::Arm,
                cpu_vendor: CpuVendor::Amd,
                cpu_vendor_raw: "AuthenticAMD".into(),
                gpu_vendors: vec![],
                gpu_details: vec![],
            },
            false,
        ),
        (
            "riscv_blocked",
            MockHardwareIdentity {
                architecture: Architecture::Riscv,
                cpu_vendor: CpuVendor::Amd,
                cpu_vendor_raw: "AuthenticAMD".into(),
                gpu_vendors: vec![],
                gpu_details: vec![],
            },
            false,
        ),
        (
            "intel_cpu_nvidia_gpu_blocked",
            MockHardwareIdentity {
                architecture: Architecture::X86_64,
                cpu_vendor: CpuVendor::Intel,
                cpu_vendor_raw: "GenuineIntel".into(),
                gpu_vendors: vec![GpuVendor::Nvidia],
                gpu_details: vec!["NVIDIA".into()],
            },
            false,
        ),
        (
            "unknown_gpu_blocked",
            MockHardwareIdentity {
                architecture: Architecture::X86_64,
                cpu_vendor: CpuVendor::Amd,
                cpu_vendor_raw: "AuthenticAMD".into(),
                gpu_vendors: vec![GpuVendor::Unknown],
                gpu_details: vec!["Mystery GPU".into()],
            },
            false,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::eligibility::exit_codes::{exit_code_for, ExitCode};
    use crate::hardware::eligibility::types::HardwareRejection;

    #[test]
    fn mock_platform_matrix_covers_all_combos() {
        let cases = eligibility_matrix_cases();
        assert!(cases.len() >= 14, "matrix too small: {}", cases.len());
        for (name, identity, expect_ok) in cases {
            let e = identity.evaluate();
            assert_eq!(
                e.supported, expect_ok,
                "{name}: expected supported={expect_ok}, got {:?}; reasons={:?}",
                e.compatibility, e.rejection_reasons
            );
            assert_eq!(e.policy, HARDWARE_POLICY);
            if expect_ok {
                assert_eq!(e.compatibility, CompatibilityStatus::Supported);
            } else {
                assert_eq!(e.compatibility, CompatibilityStatus::Blocked);
                assert!(!e.rejection_reasons.is_empty());
            }
        }
    }

    #[test]
    fn matrix_exit_codes_align() {
        let intel = MockHardwareIdentity {
            architecture: Architecture::X86_64,
            cpu_vendor: CpuVendor::Intel,
            cpu_vendor_raw: "GenuineIntel".into(),
            gpu_vendors: vec![],
            gpu_details: vec![],
        }
        .evaluate();
        assert_eq!(
            exit_code_for(intel.primary_rejection().unwrap()),
            ExitCode::IntelCpu
        );

        let nvidia = MockHardwareIdentity {
            architecture: Architecture::X86_64,
            cpu_vendor: CpuVendor::Amd,
            cpu_vendor_raw: "AuthenticAMD".into(),
            gpu_vendors: vec![GpuVendor::Nvidia],
            gpu_details: vec!["nv".into()],
        }
        .evaluate();
        assert_eq!(
            exit_code_for(nvidia.primary_rejection().unwrap()),
            ExitCode::NvidiaGpu
        );

        let arch = MockHardwareIdentity {
            architecture: Architecture::Arm64,
            cpu_vendor: CpuVendor::Amd,
            cpu_vendor_raw: "AuthenticAMD".into(),
            gpu_vendors: vec![],
            gpu_details: vec![],
        }
        .evaluate();
        assert!(matches!(
            arch.primary_rejection().unwrap(),
            HardwareRejection::UnsupportedArchitecture { .. }
        ));
        assert_eq!(
            exit_code_for(arch.primary_rejection().unwrap()),
            ExitCode::UnsupportedArchitecture
        );
    }
}
