//! Live hardware detection: CPUID for CPU vendor, PCI IDs for GPUs.
//!
//! Windows: CPUID + PCI Enum registry (VEN_xxxx) for display-class devices.
//! Linux: CPUID + /sys/bus/pci (no lspci required) and /sys/class/drm hints.

use super::evaluate::HardwareFacts;
use super::types::{Architecture, CpuVendor, GpuVendor};

#[derive(Debug, Clone)]
pub struct DetectedGpu {
    pub vendor: GpuVendor,
    pub pci_vendor_id: Option<u16>,
    pub name: String,
    pub bus_id: String,
}

pub fn collect_hardware_facts() -> HardwareFacts {
    let architecture = detect_architecture();
    let (cpu_vendor, cpu_vendor_raw) = detect_cpu_vendor();
    let gpus = detect_gpu_devices();
    let mut vendors = Vec::new();
    let mut details = Vec::new();
    for g in &gpus {
        if !vendors.contains(&g.vendor) {
            vendors.push(g.vendor);
        }
        details.push(format!(
            "{} (pci={:?} bus={})",
            g.name,
            g.pci_vendor_id
                .map(|id| format!("0x{id:04X}"))
                .unwrap_or_else(|| "n/a".into()),
            g.bus_id
        ));
    }
    HardwareFacts {
        architecture,
        cpu_vendor,
        cpu_vendor_raw,
        gpu_vendors: vendors,
        gpu_details: details,
    }
}

pub fn detect_architecture() -> Architecture {
    match std::env::consts::ARCH {
        "x86" => Architecture::X86,
        "x86_64" => Architecture::X86_64,
        "arm" => Architecture::Arm,
        "aarch64" => Architecture::Arm64,
        a if a.starts_with("riscv") => Architecture::Riscv,
        _ => Architecture::Other,
    }
}

/// Prefer CPUID leaf 0 vendor string; fall back to sysinfo /proc hints.
pub fn detect_cpu_vendor() -> (CpuVendor, String) {
    if let Some(raw) = cpuid_vendor_string() {
        return (CpuVendor::from_cpuid_string(&raw), raw);
    }
    // Fallbacks
    #[cfg(target_os = "linux")]
    {
        if let Ok(s) = std::fs::read_to_string("/proc/cpuinfo") {
            for line in s.lines() {
                if let Some(rest) = line.strip_prefix("vendor_id") {
                    let raw = rest.trim().trim_start_matches(':').trim().to_string();
                    if !raw.is_empty() {
                        return (CpuVendor::from_cpuid_string(&raw), raw);
                    }
                }
            }
        }
    }
    let mut sys = sysinfo::System::new();
    sys.refresh_cpu_all();
    let raw = sys
        .cpus()
        .first()
        .map(|c| c.vendor_id().to_string())
        .unwrap_or_else(|| "unknown".into());
    (CpuVendor::from_cpuid_string(&raw), raw)
}

fn cpuid_vendor_string() -> Option<String> {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        // SAFETY: CPUID leaf 0 is universally supported on x86/x86_64.
        #[cfg(target_arch = "x86")]
        use std::arch::x86::{__cpuid, CpuidResult};
        #[cfg(target_arch = "x86_64")]
        use std::arch::x86_64::{__cpuid, CpuidResult};

        // __cpuid is a safe intrinsic on current rustc toolchains.
        let CpuidResult { ebx, ecx, edx, .. } = __cpuid(0);
        let mut bytes = [0u8; 12];
        bytes[0..4].copy_from_slice(&ebx.to_le_bytes());
        bytes[4..8].copy_from_slice(&edx.to_le_bytes());
        bytes[8..12].copy_from_slice(&ecx.to_le_bytes());
        let s = String::from_utf8_lossy(&bytes)
            .trim_end_matches('\0')
            .to_string();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        None
    }
}

pub fn detect_gpu_devices() -> Vec<DetectedGpu> {
    #[cfg(target_os = "linux")]
    {
        detect_gpus_linux()
    }
    #[cfg(windows)]
    {
        detect_gpus_windows()
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        Vec::new()
    }
}

#[cfg(target_os = "linux")]
fn detect_gpus_linux() -> Vec<DetectedGpu> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir("/sys/bus/pci/devices") else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let vendor_path = path.join("vendor");
        let class_path = path.join("class");
        let Ok(vendor_s) = std::fs::read_to_string(&vendor_path) else {
            continue;
        };
        let Ok(class_s) = std::fs::read_to_string(&class_path) else {
            continue;
        };
        let vendor_id = parse_hex_u16(vendor_s.trim());
        let class = parse_hex_u32(class_s.trim()).unwrap_or(0);
        // Display controller class: 0x03xxxx
        let is_display = ((class >> 16) & 0xFF) == 0x03;
        if !is_display {
            continue;
        }
        let Some(vid) = vendor_id else { continue };
        let vendor = GpuVendor::from_pci_vendor_id(vid);
        let bus = entry.file_name().to_string_lossy().to_string();
        let name = std::fs::read_to_string(path.join("device"))
            .ok()
            .map(|d| format!("PCI display {:04x}:{}", vid, d.trim()))
            .unwrap_or_else(|| format!("PCI display vendor=0x{vid:04X}"));
        out.push(DetectedGpu {
            vendor,
            pci_vendor_id: Some(vid),
            name,
            bus_id: bus,
        });
    }
    out
}

#[cfg(windows)]
fn detect_gpus_windows() -> Vec<DetectedGpu> {
    // Walk PCI Enum registry for VEN_xxxx + display-class devices (Class=Display).
    detect_gpus_windows_registry()
}

#[cfg(windows)]
fn detect_gpus_windows_registry() -> Vec<DetectedGpu> {
    use windows_sys::Win32::Foundation::{ERROR_SUCCESS, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, HKEY_LOCAL_MACHINE, KEY_READ,
    };

    let mut out = Vec::new();
    let subkey = to_wide(r"SYSTEM\CurrentControlSet\Enum\PCI");
    let mut hkey = INVALID_HANDLE_VALUE;
    let status =
        unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, subkey.as_ptr(), 0, KEY_READ, &mut hkey) };
    if status != ERROR_SUCCESS {
        return out;
    }

    let mut index = 0u32;
    loop {
        let mut name_buf = [0u16; 256];
        let mut name_len = name_buf.len() as u32;
        let rc = unsafe {
            RegEnumKeyExW(
                hkey,
                index,
                name_buf.as_mut_ptr(),
                &mut name_len,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if rc != ERROR_SUCCESS {
            break;
        }
        index += 1;
        let key_name = String::from_utf16_lossy(&name_buf[..name_len as usize]);
        // Example: VEN_1002&DEV_73BF&SUBSYS_...&REV_C1
        let Some(vid) = parse_ven_from_pci_key(&key_name) else {
            continue;
        };
        // Open first instance under this device to read ClassGUID / DeviceDesc if present.
        let instance_path = format!(r"SYSTEM\CurrentControlSet\Enum\PCI\{key_name}");
        let wide = to_wide(&instance_path);
        let mut device_key = INVALID_HANDLE_VALUE;
        let open = unsafe {
            RegOpenKeyExW(
                HKEY_LOCAL_MACHINE,
                wide.as_ptr(),
                0,
                KEY_READ,
                &mut device_key,
            )
        };
        if open != ERROR_SUCCESS {
            continue;
        }
        // Enumerate first child instance
        let mut child = [0u16; 256];
        let mut child_len = child.len() as u32;
        let child_rc = unsafe {
            RegEnumKeyExW(
                device_key,
                0,
                child.as_mut_ptr(),
                &mut child_len,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        let mut is_display = false;
        let mut desc = format!("PCI VEN_{vid:04X}");
        if child_rc == ERROR_SUCCESS {
            let child_name = String::from_utf16_lossy(&child[..child_len as usize]);
            let inst = format!(r"{instance_path}\{child_name}");
            let inst_w = to_wide(&inst);
            let mut ih = INVALID_HANDLE_VALUE;
            if unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, inst_w.as_ptr(), 0, KEY_READ, &mut ih) }
                == ERROR_SUCCESS
            {
                if let Some(cc) = reg_query_string(ih, "Class") {
                    // "Display" is the Windows class name for GPUs.
                    is_display = cc.eq_ignore_ascii_case("Display");
                }
                if let Some(d) = reg_query_string(ih, "DeviceDesc") {
                    // Often "@oem...;Radeon RX ..." — take after last ';'
                    desc = d.rsplit(';').next().unwrap_or(&d).trim().to_string();
                }
                unsafe { RegCloseKey(ih) };
            }
        }
        unsafe { RegCloseKey(device_key) };

        if !is_display {
            // AMD/NVIDIA own many non-GPU PCI functions (bridges, audio). Never count
            // those as GPUs — require Windows Display class.
            continue;
        }

        let vendor = GpuVendor::from_pci_vendor_id(vid);
        out.push(DetectedGpu {
            vendor,
            pci_vendor_id: Some(vid),
            name: desc,
            bus_id: key_name,
        });
    }
    unsafe { RegCloseKey(hkey) };
    // Deduplicate by bus_id
    let mut seen = std::collections::HashSet::new();
    out.retain(|g| seen.insert(g.bus_id.clone()));
    out
}

#[cfg(windows)]
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
fn reg_query_string(
    hkey: windows_sys::Win32::System::Registry::HKEY,
    name: &str,
) -> Option<String> {
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::RegQueryValueExW;
    let wide_name = to_wide(name);
    let mut ty = 0u32;
    let mut size = 0u32;
    let rc = unsafe {
        RegQueryValueExW(
            hkey,
            wide_name.as_ptr(),
            std::ptr::null_mut(),
            &mut ty,
            std::ptr::null_mut(),
            &mut size,
        )
    };
    if rc != ERROR_SUCCESS || size == 0 {
        return None;
    }
    let mut buf = vec![0u8; size as usize];
    let rc = unsafe {
        RegQueryValueExW(
            hkey,
            wide_name.as_ptr(),
            std::ptr::null_mut(),
            &mut ty,
            buf.as_mut_ptr(),
            &mut size,
        )
    };
    if rc != ERROR_SUCCESS {
        return None;
    }
    let u16s: Vec<u16> = buf
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .filter(|&c| c != 0)
        .collect();
    Some(String::from_utf16_lossy(&u16s))
}

fn parse_ven_from_pci_key(key: &str) -> Option<u16> {
    // VEN_1002&DEV_...
    let upper = key.to_ascii_uppercase();
    let idx = upper.find("VEN_")?;
    let hex = upper.get(idx + 4..idx + 8)?;
    u16::from_str_radix(hex, 16).ok()
}

#[cfg(target_os = "linux")]
fn parse_hex_u16(s: &str) -> Option<u16> {
    let t = s.trim().trim_start_matches("0x").trim_start_matches("0X");
    u16::from_str_radix(t, 16).ok()
}

#[cfg(target_os = "linux")]
fn parse_hex_u32(s: &str) -> Option<u32> {
    let t = s.trim().trim_start_matches("0x").trim_start_matches("0X");
    u32::from_str_radix(t, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ven_key() {
        assert_eq!(
            parse_ven_from_pci_key("VEN_10DE&DEV_2204&SUBSYS_1234&REV_A1"),
            Some(0x10DE)
        );
        assert_eq!(
            parse_ven_from_pci_key("VEN_1002&DEV_73BF&SUBSYS_0000&REV_C1"),
            Some(0x1002)
        );
    }

    #[test]
    fn pci_vendor_map() {
        assert_eq!(GpuVendor::from_pci_vendor_id(0x1002), GpuVendor::Amd);
        assert_eq!(GpuVendor::from_pci_vendor_id(0x10DE), GpuVendor::Nvidia);
        assert_eq!(GpuVendor::from_pci_vendor_id(0x8086), GpuVendor::Intel);
    }
}
