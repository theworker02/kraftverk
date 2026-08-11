//! Soft/hard constraints for optimization sessions.

use serde::{Deserialize, Serialize};

/// Measured or declared limits. Violations cause reject (not fabricated scores).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OptimizeConstraints {
    /// Maximum acceptable CPU temperature (°C) when telemetry provides it.
    pub max_temp_c: Option<f64>,
    /// Maximum acceptable package power (watts) when available.
    pub max_power_w: Option<f64>,
    /// Maximum worker threads the search may propose.
    pub max_workers: Option<usize>,
    /// Maximum rayon threads the search may propose.
    pub max_rayon: Option<usize>,
    /// Reject candidates whose CoV exceeds this (overrides OptimizeConfig.max_cov when set).
    pub max_cov: Option<f64>,
    /// Minimum required Kraft Index improvement fraction (e.g. 0.01 = 1%).
    pub min_improvement: Option<f64>,
}

impl OptimizeConstraints {
    pub fn is_empty(&self) -> bool {
        self.max_temp_c.is_none()
            && self.max_power_w.is_none()
            && self.max_workers.is_none()
            && self.max_rayon.is_none()
            && self.max_cov.is_none()
            && self.min_improvement.is_none()
    }

    /// Check environmental readings against hard limits.
    /// Unknown sensors do not fail — they are recorded as unchecked.
    pub fn check_environment(&self, temp_c: Option<f64>, power_w: Option<f64>) -> ConstraintCheck {
        let mut violations = Vec::new();
        let mut unchecked = Vec::new();

        if let Some(limit) = self.max_temp_c {
            match temp_c {
                Some(t) if t > limit => {
                    violations.push(format!("temp {t:.1}°C exceeds max-temp {limit:.1}°C"));
                }
                None => unchecked.push("max-temp (no temperature sensor)".into()),
                _ => {}
            }
        }
        if let Some(limit) = self.max_power_w {
            match power_w {
                Some(p) if p > limit => {
                    violations.push(format!("power {p:.1}W exceeds max-power {limit:.1}W"));
                }
                None => unchecked.push("max-power (no power sensor)".into()),
                _ => {}
            }
        }

        ConstraintCheck {
            ok: violations.is_empty(),
            violations,
            unchecked,
        }
    }

    pub fn allows_workers(&self, n: usize) -> bool {
        self.max_workers.map(|m| n <= m).unwrap_or(true)
    }

    pub fn allows_rayon(&self, n: usize) -> bool {
        self.max_rayon.map(|m| n <= m).unwrap_or(true)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintCheck {
    pub ok: bool,
    pub violations: Vec<String>,
    pub unchecked: Vec<String>,
}
