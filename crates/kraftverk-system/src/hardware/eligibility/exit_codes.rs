//! Exit codes for hardware eligibility failures.

use super::types::HardwareRejection;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ExitCode {
    UnsupportedArchitecture = 20,
    IntelCpu = 21,
    NvidiaGpu = 22,
    IntelGpu = 23,
    UnknownCpuVendor = 24,
    UnsupportedCombination = 25,
}

impl ExitCode {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

/// Map the primary rejection reason to a documented process exit code.
pub fn exit_code_for(reason: &HardwareRejection) -> ExitCode {
    match reason {
        HardwareRejection::UnsupportedArchitecture { .. } => ExitCode::UnsupportedArchitecture,
        HardwareRejection::IntelCpuDetected => ExitCode::IntelCpu,
        HardwareRejection::NvidiaGpuDetected { .. } => ExitCode::NvidiaGpu,
        HardwareRejection::IntelGpuDetected { .. } => ExitCode::IntelGpu,
        HardwareRejection::UnknownCpuVendor { .. } => ExitCode::UnknownCpuVendor,
        HardwareRejection::UnsupportedGpuVendor { .. }
        | HardwareRejection::MixedGpuVendors { .. }
        | HardwareRejection::UnsupportedHardwareCombination { .. } => {
            ExitCode::UnsupportedCombination
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_match_policy_doc() {
        assert_eq!(ExitCode::UnsupportedArchitecture.as_i32(), 20);
        assert_eq!(ExitCode::IntelCpu.as_i32(), 21);
        assert_eq!(ExitCode::NvidiaGpu.as_i32(), 22);
        assert_eq!(ExitCode::IntelGpu.as_i32(), 23);
        assert_eq!(ExitCode::UnknownCpuVendor.as_i32(), 24);
        assert_eq!(ExitCode::UnsupportedCombination.as_i32(), 25);
    }
}
