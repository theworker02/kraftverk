//! Hardware / OS discovery for `kraftverk inspect`.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sysinfo::System;

use crate::hardware::eligibility::{evaluate_eligibility, HardwareEligibility};
use crate::hardware::fingerprint::machine_fingerprint;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensorStatus {
    Available,
    Unavailable { reason: String },
    Unsupported { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    pub brand: String,
    pub vendor_id: String,
    pub physical_cores: usize,
    pub logical_cpus: usize,
    pub frequency_mhz: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageVolume {
    pub name: String,
    pub mount_point: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub file_system: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub name: String,
    pub status: SensorStatus,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineReport {
    pub os_family: String,
    pub os_version: String,
    pub arch: String,
    pub hostname_hash: String,
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub storage: Vec<StorageVolume>,
    pub gpus: Vec<GpuInfo>,
    pub temperature: SensorStatus,
    pub environment: EnvironmentMeta,
    pub fingerprint: String,
    pub unsupported: Vec<String>,
    /// AMD-only hardware eligibility (policy amd-only-v1).
    pub eligibility: Option<HardwareEligibility>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentMeta {
    pub kraftverk_version: String,
    pub rustc_host: String,
    pub endian: String,
    pub pointer_width: String,
}

pub fn inspect_machine(version: &str) -> MachineReport {
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu_brand = sys
        .cpus()
        .first()
        .map(|c| c.brand().to_string())
        .unwrap_or_else(|| "unknown".into());
    let vendor = sys
        .cpus()
        .first()
        .map(|c| c.vendor_id().to_string())
        .unwrap_or_else(|| "unknown".into());
    let freq = sys.cpus().first().map(|c| c.frequency()).unwrap_or(0);
    let logical = sys.cpus().len().max(1);
    let physical = sys.physical_core_count().unwrap_or(logical);

    let hostname = System::host_name().unwrap_or_default();
    let hostname_hash = {
        let mut h = Sha256::new();
        h.update(hostname.as_bytes());
        hex::encode(h.finalize())[..12].to_string()
    };

    let disks = sysinfo::Disks::new_with_refreshed_list();
    let storage: Vec<StorageVolume> = disks
        .list()
        .iter()
        .map(|d| StorageVolume {
            name: d.name().to_string_lossy().to_string(),
            mount_point: d.mount_point().to_string_lossy().to_string(),
            total_bytes: d.total_space(),
            available_bytes: d.available_space(),
            file_system: d.file_system().to_string_lossy().to_string(),
        })
        .collect();

    let gpus = detect_gpus();
    let eligibility = evaluate_eligibility();
    let sensor = crate::sensors::read_sensors();
    let temperature = match sensor.primary_temp_c() {
        Some(_) => SensorStatus::Available,
        None => {
            let reason = sensor
                .unavailable
                .iter()
                .find(|u| u.to_ascii_lowercase().contains("temp"))
                .cloned()
                .unwrap_or_else(|| {
                    "Temperature sensors unavailable on this host (not fabricated)".into()
                });
            SensorStatus::Unavailable { reason }
        }
    };

    let mut unsupported = vec![
        "FAN RPM".into(),
        "GPU clocks / utilization (vendor SDK)".into(),
    ];
    if sensor.primary_temp_c().is_none() {
        unsupported.push("CPU package temperature (no readable OS sensor)".into());
    }
    if sensor.primary_power_w().is_none() {
        unsupported.push("Power draw (watts) — no readable OS counter".into());
    }
    if gpus.iter().all(|g| {
        matches!(
            g.status,
            SensorStatus::Unsupported { .. } | SensorStatus::Unavailable { .. }
        )
    }) {
        unsupported.push("Detailed GPU enumeration".into());
    }

    let mut report = MachineReport {
        os_family: std::env::consts::OS.to_string(),
        os_version: System::os_version().unwrap_or_else(|| "unknown".into()),
        arch: std::env::consts::ARCH.to_string(),
        hostname_hash,
        cpu: CpuInfo {
            brand: cpu_brand,
            vendor_id: vendor,
            physical_cores: physical,
            logical_cpus: logical,
            frequency_mhz: freq,
        },
        memory: MemoryInfo {
            total_bytes: sys.total_memory(),
            available_bytes: sys.available_memory(),
        },
        storage,
        gpus,
        temperature,
        environment: EnvironmentMeta {
            kraftverk_version: version.to_string(),
            rustc_host: std::env::consts::ARCH.to_string(),
            endian: if cfg!(target_endian = "little") {
                "little".into()
            } else {
                "big".into()
            },
            pointer_width: (std::mem::size_of::<usize>() * 8).to_string(),
        },
        fingerprint: String::new(),
        unsupported,
        eligibility: Some(eligibility),
    };
    report.fingerprint = machine_fingerprint(&report);
    report
}

fn detect_gpus() -> Vec<GpuInfo> {
    // Prefer PCI-based enumeration from the eligibility detector; fall back honestly.
    let detected = crate::hardware::eligibility::detect_gpu_devices();
    if !detected.is_empty() {
        return detected
            .into_iter()
            .map(|g| GpuInfo {
                name: g.name,
                status: SensorStatus::Available,
                notes: format!(
                    "vendor={} pci={:?} bus={}",
                    g.vendor.as_str(),
                    g.pci_vendor_id.map(|id| format!("0x{id:04X}")),
                    g.bus_id
                ),
            })
            .collect();
    }
    #[cfg(windows)]
    {
        vec![GpuInfo {
            name: "none-detected".into(),
            status: SensorStatus::Unavailable {
                reason: "No display-class PCI devices enumerated via registry".into(),
            },
            notes: "Eligibility still evaluates CPU; no GPU present is allowed for AMD CPUs."
                .into(),
        }]
    }
    #[cfg(target_os = "linux")]
    {
        vec![GpuInfo {
            name: "none-detected".into(),
            status: SensorStatus::Unavailable {
                reason: "No display-class devices under /sys/bus/pci".into(),
            },
            notes: "Eligibility still evaluates CPU; no GPU present is allowed for AMD CPUs."
                .into(),
        }]
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        vec![GpuInfo {
            name: "unsupported-os".into(),
            status: SensorStatus::Unsupported {
                reason: format!("GPU discovery not implemented for {}", std::env::consts::OS),
            },
            notes: String::new(),
        }]
    }
}
