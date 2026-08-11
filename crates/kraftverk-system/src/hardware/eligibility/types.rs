//! Eligibility types for the AMD-only hardware policy.

use serde::{Deserialize, Serialize};

/// Persisted policy identifier for experiment records and fingerprints.
pub const HARDWARE_POLICY: &str = "amd-only-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Architecture {
    X86,
    X86_64,
    Arm,
    Arm64,
    Riscv,
    Other,
}

impl Architecture {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::X86 => "x86",
            Self::X86_64 => "x86_64",
            Self::Arm => "arm",
            Self::Arm64 => "aarch64",
            Self::Riscv => "riscv",
            Self::Other => "other",
        }
    }

    pub fn is_supported(self) -> bool {
        matches!(self, Self::X86 | Self::X86_64)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CpuVendor {
    Amd,
    Intel,
    Unknown,
}

impl CpuVendor {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Amd => "amd",
            Self::Intel => "intel",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_cpuid_string(s: &str) -> Self {
        let t = s.trim();
        if t.eq_ignore_ascii_case("AuthenticAMD") || t.eq_ignore_ascii_case("AMD") {
            Self::Amd
        } else if t.eq_ignore_ascii_case("GenuineIntel") || t.eq_ignore_ascii_case("Intel") {
            Self::Intel
        } else {
            Self::Unknown
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GpuVendor {
    Amd,
    Nvidia,
    Intel,
    Unknown,
}

impl GpuVendor {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Amd => "amd",
            Self::Nvidia => "nvidia",
            Self::Intel => "intel",
            Self::Unknown => "unknown",
        }
    }

    /// Map PCI vendor ID (preferred) to vendor enum.
    pub fn from_pci_vendor_id(id: u16) -> Self {
        match id {
            0x1002 => Self::Amd, // AMD/ATI
            0x10DE => Self::Nvidia,
            0x8086 => Self::Intel,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CompatibilityStatus {
    Supported,
    Blocked,
}

impl CompatibilityStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Supported => "SUPPORTED",
            Self::Blocked => "BLOCKED",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HardwareRejection {
    UnsupportedArchitecture { arch: String },
    IntelCpuDetected,
    NvidiaGpuDetected { detail: String },
    IntelGpuDetected { detail: String },
    UnknownCpuVendor { raw: String },
    UnsupportedGpuVendor { detail: String },
    MixedGpuVendors { detail: String },
    UnsupportedHardwareCombination { detail: String },
}

impl HardwareRejection {
    pub fn message(&self) -> String {
        match self {
            Self::UnsupportedArchitecture { arch } => {
                format!("Unsupported CPU architecture '{arch}'. Kraftverk requires x86 or x86_64.")
            }
            Self::IntelCpuDetected => {
                "Intel CPU detected. Kraftverk is an AMD-exclusive performance platform \
                 (policy amd-only-v1)."
                    .into()
            }
            Self::NvidiaGpuDetected { detail } => {
                format!(
                    "NVIDIA GPU detected ({detail}). Kraftverk does not support NVIDIA GPUs \
                     (policy amd-only-v1). Mixed AMD+NVIDIA configurations are blocked."
                )
            }
            Self::IntelGpuDetected { detail } => {
                format!(
                    "Intel GPU detected ({detail}). Kraftverk does not support Intel GPUs \
                     (policy amd-only-v1)."
                )
            }
            Self::UnknownCpuVendor { raw } => {
                format!(
                    "Unknown CPU vendor '{raw}'. Kraftverk requires an AMD CPU \
                     (AuthenticAMD) under policy amd-only-v1."
                )
            }
            Self::UnsupportedGpuVendor { detail } => {
                format!(
                    "Unsupported GPU vendor ({detail}). When a GPU is present, only AMD \
                     (PCI 0x1002) is allowed."
                )
            }
            Self::MixedGpuVendors { detail } => {
                format!(
                    "Mixed GPU vendors are not supported ({detail}). All detected GPUs must be AMD."
                )
            }
            Self::UnsupportedHardwareCombination { detail } => {
                format!("Unsupported hardware combination: {detail}")
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareEligibility {
    pub policy: String,
    pub architecture: Architecture,
    pub cpu_vendor: CpuVendor,
    pub cpu_vendor_raw: String,
    pub gpu_vendors: Vec<GpuVendor>,
    pub gpu_details: Vec<String>,
    pub supported: bool,
    pub compatibility: CompatibilityStatus,
    pub rejection_reasons: Vec<HardwareRejection>,
}

impl HardwareEligibility {
    pub fn primary_rejection(&self) -> Option<&HardwareRejection> {
        self.rejection_reasons.first()
    }

    pub fn summary(&self) -> String {
        if self.supported {
            format!(
                "SUPPORTED under {} — arch={} cpu={} gpus={}",
                self.policy,
                self.architecture.as_str(),
                self.cpu_vendor.as_str(),
                if self.gpu_vendors.is_empty() {
                    "none".into()
                } else {
                    self.gpu_vendors
                        .iter()
                        .map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                }
            )
        } else {
            let reasons = self
                .rejection_reasons
                .iter()
                .map(|r| r.message())
                .collect::<Vec<_>>()
                .join("; ");
            format!("BLOCKED under {}: {reasons}", self.policy)
        }
    }
}
