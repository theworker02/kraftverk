//! OS-backed temperature and power sensors.
//!
//! Readings are never invented. When no supported API exposes a value, fields
//! remain `None` / `Unsupported` with an explicit reason.

use serde::{Deserialize, Serialize};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(windows)]
mod windows;

/// A single sensor sample with provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorSample {
    pub kind: SensorKind,
    pub name: String,
    pub value: f64,
    pub unit: SensorUnit,
    pub source: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensorKind {
    Temperature,
    Power,
    Energy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensorUnit {
    Celsius,
    Watts,
    Joules,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SensorReport {
    pub samples: Vec<SensorSample>,
    /// Best-effort CPU package / die temperature (°C).
    pub cpu_temp_c: Option<f64>,
    /// Best-effort AMD GPU temperature (°C) when hwmon/DRM exposes it.
    pub gpu_temp_c: Option<f64>,
    /// Instantaneous or averaged package power (W) when energy counters exist.
    pub package_power_w: Option<f64>,
    pub notes: Vec<String>,
    pub unavailable: Vec<String>,
}

impl SensorReport {
    pub fn primary_temp_c(&self) -> Option<f64> {
        self.cpu_temp_c.or(self.gpu_temp_c)
    }

    pub fn primary_power_w(&self) -> Option<f64> {
        self.package_power_w
    }
}

/// Read all available sensors for the current OS.
pub fn read_sensors() -> SensorReport {
    #[cfg(target_os = "linux")]
    {
        linux::read_sensors()
    }
    #[cfg(windows)]
    {
        windows::read_sensors()
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        SensorReport {
            notes: vec!["Sensor backends are implemented for Linux and Windows only.".into()],
            unavailable: vec![
                "temperature (no portable backend on this OS)".into(),
                "power (no portable backend on this OS)".into(),
            ],
            ..Default::default()
        }
    }
}

/// Convenience: CPU package temp and package power for telemetry/constraints.
pub fn read_temp_and_power() -> (Option<f64>, Option<f64>) {
    let r = read_sensors();
    (r.primary_temp_c(), r.primary_power_w())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_does_not_fabricate() {
        let r = read_sensors();
        if let Some(t) = r.cpu_temp_c {
            assert!(t.is_finite());
            assert!(t > -40.0 && t < 150.0, "implausible temp {t}");
        }
        if let Some(p) = r.package_power_w {
            assert!(p.is_finite());
            assert!((0.0..2000.0).contains(&p), "implausible power {p}");
        }
        let _ = &r.notes;
        let _ = &r.unavailable;
        let _ = &r.samples;
    }
}
