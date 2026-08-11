//! Pure eligibility evaluation (testable without live hardware).

use super::types::{
    Architecture, CompatibilityStatus, CpuVendor, GpuVendor, HardwareEligibility,
    HardwareRejection, HARDWARE_POLICY,
};

/// Observable hardware facts used by the eligibility engine.
#[derive(Debug, Clone)]
pub struct HardwareFacts {
    pub architecture: Architecture,
    pub cpu_vendor: CpuVendor,
    pub cpu_vendor_raw: String,
    /// Distinct GPU vendors present. Empty means no GPU detected.
    pub gpu_vendors: Vec<GpuVendor>,
    pub gpu_details: Vec<String>,
}

/// Evaluate live machine facts via platform detection.
pub fn evaluate_eligibility() -> HardwareEligibility {
    let facts = super::detect::collect_hardware_facts();
    evaluate_from_facts(&facts)
}

/// Evaluate eligibility from explicit facts (unit tests / MockPlatform).
pub fn evaluate_from_facts(facts: &HardwareFacts) -> HardwareEligibility {
    let mut reasons: Vec<HardwareRejection> = Vec::new();

    if !facts.architecture.is_supported() {
        reasons.push(HardwareRejection::UnsupportedArchitecture {
            arch: facts.architecture.as_str().to_string(),
        });
    }

    match facts.cpu_vendor {
        CpuVendor::Amd => {}
        CpuVendor::Intel => reasons.push(HardwareRejection::IntelCpuDetected),
        CpuVendor::Unknown => reasons.push(HardwareRejection::UnknownCpuVendor {
            raw: facts.cpu_vendor_raw.clone(),
        }),
    }

    // No GPU → allowed (AMD CPU + supported arch only).
    // Any GPU present → every detected GPU must be AMD-only.
    if !facts.gpu_vendors.is_empty() {
        let has_nvidia = facts.gpu_vendors.contains(&GpuVendor::Nvidia);
        let has_intel = facts.gpu_vendors.contains(&GpuVendor::Intel);
        let has_unknown = facts.gpu_vendors.contains(&GpuVendor::Unknown);
        let has_amd = facts.gpu_vendors.contains(&GpuVendor::Amd);
        let distinct: std::collections::BTreeSet<_> = facts.gpu_vendors.iter().copied().collect();

        if has_nvidia {
            reasons.push(HardwareRejection::NvidiaGpuDetected {
                detail: facts.gpu_details.join("; "),
            });
        }
        if has_intel {
            reasons.push(HardwareRejection::IntelGpuDetected {
                detail: facts.gpu_details.join("; "),
            });
        }
        if has_unknown {
            reasons.push(HardwareRejection::UnsupportedGpuVendor {
                detail: format!("unknown PCI/vendor among: {}", facts.gpu_details.join("; ")),
            });
        }
        if distinct.len() > 1 {
            reasons.push(HardwareRejection::MixedGpuVendors {
                detail: format!(
                    "vendors={:?} details={}",
                    distinct.iter().map(|v| v.as_str()).collect::<Vec<_>>(),
                    facts.gpu_details.join("; ")
                ),
            });
        }
        // AMD+NVIDIA already covered; also catch AMD+Unknown etc. via combination.
        if has_amd && (has_nvidia || has_intel || has_unknown) {
            reasons.push(HardwareRejection::UnsupportedHardwareCombination {
                detail: "AMD GPU mixed with non-AMD GPU(s)".into(),
            });
        }
        if !has_amd && !has_nvidia && !has_intel && has_unknown {
            // already have UnsupportedGpuVendor
        } else if !has_amd && !has_nvidia && !has_intel {
            reasons.push(HardwareRejection::UnsupportedGpuVendor {
                detail: facts.gpu_details.join("; "),
            });
        }
    }

    // Deduplicate by message while preserving order.
    let mut seen = std::collections::HashSet::new();
    reasons.retain(|r| seen.insert(r.message()));

    let supported = reasons.is_empty();
    HardwareEligibility {
        policy: HARDWARE_POLICY.to_string(),
        architecture: facts.architecture,
        cpu_vendor: facts.cpu_vendor,
        cpu_vendor_raw: facts.cpu_vendor_raw.clone(),
        gpu_vendors: facts.gpu_vendors.clone(),
        gpu_details: facts.gpu_details.clone(),
        supported,
        compatibility: if supported {
            CompatibilityStatus::Supported
        } else {
            CompatibilityStatus::Blocked
        },
        rejection_reasons: reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(arch: Architecture, cpu: CpuVendor, raw: &str, gpus: Vec<GpuVendor>) -> HardwareFacts {
        HardwareFacts {
            architecture: arch,
            cpu_vendor: cpu,
            cpu_vendor_raw: raw.into(),
            gpu_details: gpus.iter().map(|g| g.as_str().to_string()).collect(),
            gpu_vendors: gpus,
        }
    }

    #[test]
    fn amd_cpu_amd_gpu_passes() {
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
    fn amd_cpu_no_gpu_passes() {
        let e = evaluate_from_facts(&facts(
            Architecture::X86_64,
            CpuVendor::Amd,
            "AuthenticAMD",
            vec![],
        ));
        assert!(e.supported, "{:?}", e.rejection_reasons);
    }

    #[test]
    fn amd_plus_nvidia_fails() {
        let e = evaluate_from_facts(&facts(
            Architecture::X86_64,
            CpuVendor::Amd,
            "AuthenticAMD",
            vec![GpuVendor::Amd, GpuVendor::Nvidia],
        ));
        assert!(!e.supported);
        assert!(e
            .rejection_reasons
            .iter()
            .any(|r| matches!(r, HardwareRejection::NvidiaGpuDetected { .. })));
    }

    #[test]
    fn intel_cpu_fails() {
        let e = evaluate_from_facts(&facts(
            Architecture::X86_64,
            CpuVendor::Intel,
            "GenuineIntel",
            vec![GpuVendor::Amd],
        ));
        assert!(!e.supported);
        assert!(e
            .rejection_reasons
            .iter()
            .any(|r| matches!(r, HardwareRejection::IntelCpuDetected)));
    }

    #[test]
    fn intel_nvidia_fails() {
        let e = evaluate_from_facts(&facts(
            Architecture::X86_64,
            CpuVendor::Intel,
            "GenuineIntel",
            vec![GpuVendor::Nvidia],
        ));
        assert!(!e.supported);
    }

    #[test]
    fn amd_intel_gpu_fails() {
        let e = evaluate_from_facts(&facts(
            Architecture::X86_64,
            CpuVendor::Amd,
            "AuthenticAMD",
            vec![GpuVendor::Amd, GpuVendor::Intel],
        ));
        assert!(!e.supported);
        assert!(e
            .rejection_reasons
            .iter()
            .any(|r| matches!(r, HardwareRejection::IntelGpuDetected { .. })));
    }

    #[test]
    fn arm_fails() {
        let e = evaluate_from_facts(&facts(
            Architecture::Arm64,
            CpuVendor::Amd,
            "AuthenticAMD",
            vec![],
        ));
        assert!(!e.supported);
        assert!(e
            .rejection_reasons
            .iter()
            .any(|r| matches!(r, HardwareRejection::UnsupportedArchitecture { .. })));
    }

    #[test]
    fn unknown_cpu_fails() {
        let e = evaluate_from_facts(&facts(
            Architecture::X86_64,
            CpuVendor::Unknown,
            "SomeOther",
            vec![],
        ));
        assert!(!e.supported);
        assert!(e
            .rejection_reasons
            .iter()
            .any(|r| matches!(r, HardwareRejection::UnknownCpuVendor { .. })));
    }

    #[test]
    fn nvidia_only_fails() {
        let e = evaluate_from_facts(&facts(
            Architecture::X86_64,
            CpuVendor::Amd,
            "AuthenticAMD",
            vec![GpuVendor::Nvidia],
        ));
        assert!(!e.supported);
        assert_eq!(
            super::super::exit_codes::exit_code_for(e.primary_rejection().unwrap()).as_i32(),
            22
        );
    }

    #[test]
    fn multi_amd_gpu_passes() {
        let e = evaluate_from_facts(&facts(
            Architecture::X86_64,
            CpuVendor::Amd,
            "AuthenticAMD",
            vec![GpuVendor::Amd, GpuVendor::Amd],
        ));
        // Distinct set is still only AMD after BTreeSet in mixed check —
        // we pass duplicate Amd; distinct.len()==1.
        assert!(e.supported, "{:?}", e.rejection_reasons);
    }

    #[test]
    fn x86_amd_passes() {
        let e = evaluate_from_facts(&facts(
            Architecture::X86,
            CpuVendor::Amd,
            "AuthenticAMD",
            vec![],
        ));
        assert!(e.supported);
    }
}
