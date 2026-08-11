//! AMD capability / topology insights (honest — no fabricated telemetry).

use serde::{Deserialize, Serialize};

use super::detect::{detect_cpu_vendor, detect_gpu_devices};
use super::types::CpuVendor;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmdCpuTopologyHints {
    pub physical_cores: Option<usize>,
    pub logical_cpus: Option<usize>,
    pub smt_likely: Option<bool>,
    pub packages: Option<usize>,
    /// CCD/CCX counts are only filled when reliably detectable; otherwise None.
    pub ccd_count: Option<usize>,
    pub ccx_per_ccd: Option<usize>,
    pub preferred_cores_note: String,
    pub detection_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmdCapabilities {
    pub cpu_is_amd: bool,
    pub cpu_brand: String,
    pub topology: AmdCpuTopologyHints,
    pub amd_gpus: Vec<String>,
    pub platform_profile_note: String,
    pub cache_aware_search_hooks: Vec<String>,
    pub unsupported_surfaces: Vec<String>,
}

/// Probe AMD-facing capability surfaces after eligibility has already passed
/// (or for inspect-only commands). Does not invent sensors.
pub fn probe_amd_capabilities() -> AmdCapabilities {
    let (vendor, _) = detect_cpu_vendor();
    let mut sys = sysinfo::System::new();
    sys.refresh_cpu_all();
    let brand = sys
        .cpus()
        .first()
        .map(|c| c.brand().to_string())
        .unwrap_or_else(|| "unknown".into());
    let logical = sys.cpus().len();
    let physical = sys.physical_core_count();
    let smt_likely = physical.map(|p| logical > p);

    let mut notes: Vec<String> = vec![
        "CCD/CCX enumeration requires vendor topology interfaces not yet wired — left unset rather than guessed.".into(),
        "Preferred-core ranking is not fabricated; use OS scheduler hints when available.".into(),
    ];
    append_linux_topology_note(&mut notes);

    let amd_gpus: Vec<String> = detect_gpu_devices()
        .into_iter()
        .filter(|g| g.vendor == super::types::GpuVendor::Amd)
        .map(|g| format!("{} [{}]", g.name, g.bus_id))
        .collect();

    AmdCapabilities {
        cpu_is_amd: vendor == CpuVendor::Amd,
        cpu_brand: brand,
        topology: AmdCpuTopologyHints {
            physical_cores: physical,
            logical_cpus: Some(logical.max(1)),
            smt_likely,
            packages: Some(1),
            ccd_count: None,
            ccx_per_ccd: None,
            preferred_cores_note: "Preferred cores not exposed portably; Kraftverk will not invent rankings.".into(),
            detection_notes: notes,
        },
        amd_gpus,
        platform_profile_note: "AMD Platform Profile characterization is optional and not assumed; \
             when unavailable it is reported as unsupported rather than simulated."
            .into(),
        cache_aware_search_hooks: vec![
            "chase: optional worker/affinity candidates may bias toward topology-local threads when physical_cores known".into(),
            "optimizer: empirical only — no assumed cache-size wins without measurement".into(),
        ],
        unsupported_surfaces: vec![
            "undocumented MSR / SMU writes".into(),
            "fabricated CCD/CCX maps".into(),
            "GPU clock / power telemetry without a real backend".into(),
        ],
    }
}

fn append_linux_topology_note(notes: &mut Vec<String>) {
    #[cfg(target_os = "linux")]
    if std::path::Path::new("/sys/devices/system/cpu/cpu0/topology").exists() {
        notes.push(
            "Linux sysfs CPU topology present; package/core IDs usable for affinity experiments."
                .into(),
        );
    }
    #[cfg(not(target_os = "linux"))]
    let _ = notes;
}
