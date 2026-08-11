//! Stable machine fingerprint without unnecessary PII.
//!
//! Includes: OS family/version, CPU brand/cores, total memory bucket, hostname hash,
//! eligibility architecture/vendor/compatibility (amd-only-v1).
//! Excludes: username, home path, MAC addresses, serial numbers, full hostname plaintext.

use sha2::{Digest, Sha256};

use crate::hardware::eligibility::{CompatibilityStatus, HARDWARE_POLICY};
use crate::hardware::inspect::MachineReport;

pub fn machine_fingerprint(report: &MachineReport) -> String {
    let mut hasher = Sha256::new();
    hasher.update(report.os_family.as_bytes());
    hasher.update(b"|");
    hasher.update(report.os_version.as_bytes());
    hasher.update(b"|");
    hasher.update(report.arch.as_bytes());
    hasher.update(b"|");
    hasher.update(report.cpu.brand.as_bytes());
    hasher.update(b"|");
    hasher.update(report.cpu.logical_cpus.to_string().as_bytes());
    hasher.update(b"|");
    hasher.update(report.cpu.physical_cores.to_string().as_bytes());
    hasher.update(b"|");
    // Memory rounded to 1 GiB bucket to reduce uniqueness from tiny diffs.
    let mem_gib = report.memory.total_bytes / (1 << 30);
    hasher.update(mem_gib.to_string().as_bytes());
    hasher.update(b"|");
    hasher.update(report.hostname_hash.as_bytes());
    hasher.update(b"|");
    hasher.update(HARDWARE_POLICY.as_bytes());
    hasher.update(b"|");
    if let Some(el) = &report.eligibility {
        hasher.update(el.architecture.as_str().as_bytes());
        hasher.update(b"|");
        hasher.update(el.cpu_vendor.as_str().as_bytes());
        hasher.update(b"|");
        hasher.update(el.compatibility.as_str().as_bytes());
        for g in &el.gpu_vendors {
            hasher.update(b"|g:");
            hasher.update(g.as_str().as_bytes());
        }
    } else {
        hasher.update(CompatibilityStatus::Blocked.as_str().as_bytes());
    }
    let digest = hasher.finalize();
    format!("kv-{}", &hex::encode(digest)[..16])
}
