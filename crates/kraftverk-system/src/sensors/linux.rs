//! Linux sensors via hwmon sysfs and RAPL powercap.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::{SensorKind, SensorReport, SensorSample, SensorUnit};

pub fn read_sensors() -> SensorReport {
    let mut report = SensorReport::default();
    let mut samples = Vec::new();

    collect_hwmon(&mut samples, &mut report);
    collect_rapl(&mut samples, &mut report);

    // Prefer package/Tctl/edge labels for CPU; amdgpu for GPU.
    for s in &samples {
        if s.kind != SensorKind::Temperature {
            continue;
        }
        let name_l = s.name.to_ascii_lowercase();
        let src_l = s.source.to_ascii_lowercase();
        if report.gpu_temp_c.is_none()
            && (src_l.contains("amdgpu") || name_l.contains("edge") && src_l.contains("amdgpu"))
        {
            report.gpu_temp_c = Some(s.value);
        }
        if report.cpu_temp_c.is_none()
            && (name_l.contains("tctl")
                || name_l.contains("tdie")
                || name_l.contains("package")
                || name_l.contains("k10temp")
                || src_l.contains("k10temp")
                || src_l.contains("zenpower")
                || src_l.contains("coretemp")
                || name_l.contains("cpu"))
        {
            report.cpu_temp_c = Some(s.value);
        }
    }
    // Fallback: any plausible temp sample as CPU if still unset.
    if report.cpu_temp_c.is_none() {
        report.cpu_temp_c = samples
            .iter()
            .filter(|s| s.kind == SensorKind::Temperature)
            .filter(|s| {
                let src = s.source.to_ascii_lowercase();
                !src.contains("amdgpu") && !src.contains("nvme") && !src.contains("acpitz")
            })
            .map(|s| s.value)
            .next();
    }
    if report.gpu_temp_c.is_none() {
        report.gpu_temp_c = samples
            .iter()
            .filter(|s| s.kind == SensorKind::Temperature)
            .filter(|s| s.source.to_ascii_lowercase().contains("amdgpu"))
            .map(|s| s.value)
            .next();
    }

    if report.cpu_temp_c.is_none() && report.gpu_temp_c.is_none() {
        report
            .unavailable
            .push("temperature: no readable hwmon temp*_input".into());
    }
    if report.package_power_w.is_none() {
        report
            .unavailable
            .push("power: no RAPL/powercap energy counter or instantaneous power".into());
    }

    report.notes.push(
        "Linux sensors: /sys/class/hwmon (temps) and /sys/class/powercap (RAPL energy→power)."
            .into(),
    );
    report.samples = samples;
    report
}

fn collect_hwmon(samples: &mut Vec<SensorSample>, report: &mut SensorReport) {
    let Ok(entries) = fs::read_dir("/sys/class/hwmon") else {
        report
            .unavailable
            .push("hwmon: /sys/class/hwmon not present".into());
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let chip = read_trim(path.join("name")).unwrap_or_else(|| {
            path.file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "hwmon".into())
        });
        // Enumerate tempN_input
        for idx in 1..=32 {
            let input = path.join(format!("temp{idx}_input"));
            let Some(raw) = read_trim(&input) else {
                continue;
            };
            let Ok(milli) = raw.parse::<f64>() else {
                continue;
            };
            // Millidegrees Celsius (standard hwmon).
            let celsius = milli / 1000.0;
            if !(celsius > -40.0 && celsius < 150.0) {
                continue;
            }
            let label = read_trim(path.join(format!("temp{idx}_label")))
                .unwrap_or_else(|| format!("temp{idx}"));
            samples.push(SensorSample {
                kind: SensorKind::Temperature,
                name: format!("{chip}/{label}"),
                value: celsius,
                unit: SensorUnit::Celsius,
                source: format!("hwmon:{}", path.display()),
            });
        }
        // Instantaneous power if present (powerN_input in microwatts).
        for idx in 1..=8 {
            let input = path.join(format!("power{idx}_input"));
            let Some(raw) = read_trim(&input) else {
                continue;
            };
            let Ok(uw) = raw.parse::<f64>() else {
                continue;
            };
            let watts = uw / 1_000_000.0;
            if !(0.0..2000.0).contains(&watts) {
                continue;
            }
            let label = read_trim(path.join(format!("power{idx}_label")))
                .unwrap_or_else(|| format!("power{idx}"));
            samples.push(SensorSample {
                kind: SensorKind::Power,
                name: format!("{chip}/{label}"),
                value: watts,
                unit: SensorUnit::Watts,
                source: format!("hwmon:{}", path.display()),
            });
            if report.package_power_w.is_none()
                && (label.to_ascii_lowercase().contains("package")
                    || chip.to_ascii_lowercase().contains("amdgpu")
                    || chip.to_ascii_lowercase().contains("rapl"))
            {
                report.package_power_w = Some(watts);
            }
        }
    }
}

fn collect_rapl(samples: &mut Vec<SensorSample>, report: &mut SensorReport) {
    let root = Path::new("/sys/class/powercap");
    if !root.exists() {
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    // Prefer package-0 domains (intel-rapl:0 or similar AMD energy).
    let mut domains: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            name.contains("rapl") || name.contains("energy")
        })
        .collect();
    domains.sort();

    for domain in domains {
        let energy_path = domain.join("energy_uj");
        if !energy_path.exists() {
            continue;
        }
        let name = read_trim(domain.join("name")).unwrap_or_else(|| {
            domain
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "rapl".into())
        });
        // Two-sample delta for power estimate.
        let Some(e0) = read_u64(&energy_path) else {
            continue;
        };
        let t0 = Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(100));
        let Some(e1) = read_u64(&energy_path) else {
            continue;
        };
        let dt = t0.elapsed().as_secs_f64().max(1e-6);
        let de = if e1 >= e0 {
            (e1 - e0) as f64
        } else {
            // Counter wrap — skip.
            continue;
        };
        let watts = (de / 1_000_000.0) / dt; // µJ → J → W
        if !(0.0..2000.0).contains(&watts) {
            continue;
        }
        samples.push(SensorSample {
            kind: SensorKind::Power,
            name: name.clone(),
            value: watts,
            unit: SensorUnit::Watts,
            source: format!("powercap:{}", domain.display()),
        });
        samples.push(SensorSample {
            kind: SensorKind::Energy,
            name: format!("{name}/energy_uj"),
            value: e1 as f64 / 1_000_000.0,
            unit: SensorUnit::Joules,
            source: format!("powercap:{}", domain.display()),
        });
        let name_l = name.to_ascii_lowercase();
        if report.package_power_w.is_none()
            && (name_l.contains("package") || name_l.contains("psys") || name_l.ends_with(":0"))
        {
            report.package_power_w = Some(watts);
        }
    }
    // If we got any RAPL power but no package specifically, use the first.
    if report.package_power_w.is_none() {
        report.package_power_w = samples
            .iter()
            .filter(|s| s.kind == SensorKind::Power && s.source.contains("powercap"))
            .map(|s| s.value)
            .next();
    }
}

fn read_trim(path: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(path.as_ref())
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn read_u64(path: impl AsRef<Path>) -> Option<u64> {
    read_trim(path)?.parse().ok()
}
