//! Windows sensors via thermal zone WMI (when available).
//!
//! AMD GPU temperature/power through proprietary ADL is intentionally not linked.
//! When OS APIs do not expose readings, fields stay unset (never fabricated).

use std::cmp::Ordering;
use std::process::Command;

use super::{SensorKind, SensorReport, SensorSample, SensorUnit};

pub fn read_sensors() -> SensorReport {
    let mut report = SensorReport::default();
    let mut samples = Vec::new();

    if let Some(temps) = query_thermal_zones() {
        for (i, t) in temps.into_iter().enumerate() {
            if t > -40.0 && t < 150.0 {
                samples.push(SensorSample {
                    kind: SensorKind::Temperature,
                    name: format!("thermal_zone_{i}"),
                    value: t,
                    unit: SensorUnit::Celsius,
                    source: "wmi:MSAcpi_ThermalZoneTemperature".into(),
                });
            }
        }
    }

    // Prefer median of thermal zones as a coarse package proxy when present.
    let zone_temps: Vec<f64> = samples
        .iter()
        .filter(|s| s.kind == SensorKind::Temperature)
        .map(|s| s.value)
        .collect();
    if !zone_temps.is_empty() {
        let mut sorted = zone_temps;
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        report.cpu_temp_c = Some(sorted[sorted.len() / 2]);
        report.notes.push(
            "Windows temperature from ACPI thermal zones (WMI). Often coarse or unavailable \
             depending on OEM ACPI tables."
                .into(),
        );
    } else {
        report.unavailable.push(
            "temperature: ACPI thermal zones unavailable (OEM/driver dependent; not fabricated)"
                .into(),
        );
    }

    // Package power is not reliably exposed via documented free Windows APIs for AMD.
    report.unavailable.push(
        "power: package wattage not exposed via portable Windows APIs without vendor SDKs \
         (ADL not linked)"
            .into(),
    );
    report.notes.push(
        "Windows: no AMD ADL binding in this build; GPU temp/power remain unset unless OS exposes them."
            .into(),
    );

    report.samples = samples;
    report
}

/// Query ACPI thermal zones via PowerShell/WMI. Returns Celsius values.
fn query_thermal_zones() -> Option<Vec<f64>> {
    // Keep this dependency-light: one short PowerShell call. Failures → None.
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-CimInstance -Namespace root/wmi -ClassName MSAcpi_ThermalZoneTemperature \
             -ErrorAction SilentlyContinue | ForEach-Object { $_.CurrentTemperature }",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut temps = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(raw) = line.parse::<f64>() {
            // WMI reports tenths of Kelvin.
            let celsius = raw / 10.0 - 273.15;
            if celsius.is_finite() {
                temps.push(celsius);
            }
        }
    }
    if temps.is_empty() {
        None
    } else {
        Some(temps)
    }
}
