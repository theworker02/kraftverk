//! Safe privileged operations surface (no arbitrary shell).

use kraftverk_core::candidate::ParamValue;
use kraftverk_core::error::{Error, Result};
use kraftverk_system::{evaluate_eligibility, NativePlatform, Platform, RecoveryJournal};
use tracing::{info, warn};

use crate::auth::agent_data_dir;

/// Keys the agent will apply / verify / roll back.
pub const ALLOWED_KEYS: &[&str] = &["process.priority", "process.affinity", "power.scheme"];

pub struct PrivilegedOps {
    platform: NativePlatform,
    journal: RecoveryJournal,
}

impl PrivilegedOps {
    pub fn open() -> Result<Self> {
        let journal_path = agent_data_dir()?.join("recovery_journal.json");
        Ok(Self {
            platform: NativePlatform::detect()?,
            journal: RecoveryJournal::open(journal_path)?,
        })
    }

    pub fn assert_hardware(&self) -> Result<()> {
        let el = evaluate_eligibility();
        if el.supported {
            Ok(())
        } else {
            Err(Error::UnsupportedHardware(el.summary()))
        }
    }

    pub fn get_param(&self, key: &str) -> Result<ParamValue> {
        ensure_allowed(key)?;
        if key == "power.scheme" {
            return Ok(ParamValue::String(read_power_scheme()?));
        }
        self.platform.read_param(key)
    }

    pub fn apply(&mut self, key: &str, previous: ParamValue, next: ParamValue) -> Result<()> {
        ensure_allowed(key)?;
        self.assert_hardware()?;
        if key == "power.scheme" {
            let cur = read_power_scheme()?;
            if let ParamValue::String(exp) = &previous {
                if exp != &cur && !exp.is_empty() {
                    warn!(expected = %exp, actual = %cur, "power.scheme previous mismatch; continuing");
                }
            }
            let ParamValue::String(scheme) = &next else {
                return Err(Error::InvalidConfig("power.scheme must be string".into()));
            };
            set_power_scheme(scheme)?;
            self.journal.begin(
                &format!("agent-{}", uuid::Uuid::new_v4()),
                &kraftverk_core::Candidate {
                    id: "agent-apply".into(),
                    label: format!("power.scheme={scheme}"),
                    changes: vec![kraftverk_core::ParamChange {
                        key: key.into(),
                        previous,
                        next: next.clone(),
                        rationale: "agent apply".into(),
                    }],
                    meta: Default::default(),
                },
            )?;
            self.journal.record_applied(&kraftverk_core::ParamChange {
                key: key.into(),
                previous: ParamValue::String(cur),
                next: next.clone(),
                rationale: "agent apply".into(),
            })?;
            self.journal.complete()?;
            info!(scheme = %scheme, "applied power.scheme via agent");
            return Ok(());
        }

        let change = kraftverk_core::ParamChange {
            key: key.into(),
            previous,
            next,
            rationale: "agent apply".into(),
        };
        self.journal.begin(
            &format!("agent-{}", uuid::Uuid::new_v4()),
            &kraftverk_core::Candidate {
                id: "agent-apply".into(),
                label: change.key.clone(),
                changes: vec![change.clone()],
                meta: Default::default(),
            },
        )?;
        self.platform.apply_change(&change)?;
        if !self.platform.verify_change(&change)? {
            let _ = self.platform.rollback_change(&change);
            let _ = self.journal.fail("verify failed");
            return Err(Error::Platform(format!("verify failed for {key}")));
        }
        self.journal.record_applied(&change)?;
        self.journal.complete()?;
        Ok(())
    }

    pub fn verify(&self, key: &str, expected: &ParamValue) -> Result<bool> {
        ensure_allowed(key)?;
        if key == "power.scheme" {
            let cur = read_power_scheme()?;
            return Ok(matches!(expected, ParamValue::String(s) if s == &cur));
        }
        Ok(self.platform.read_param(key).ok().as_ref() == Some(expected))
    }

    pub fn rollback(&mut self, key: &str, previous: ParamValue) -> Result<()> {
        ensure_allowed(key)?;
        self.assert_hardware()?;
        let current = self.get_param(key)?;
        let change = kraftverk_core::ParamChange {
            key: key.into(),
            previous: current,
            next: previous,
            rationale: "agent rollback".into(),
        };
        if key == "power.scheme" {
            let ParamValue::String(scheme) = &change.next else {
                return Err(Error::InvalidConfig("power.scheme must be string".into()));
            };
            set_power_scheme(scheme)?;
            return Ok(());
        }
        self.platform.apply_change(&change)?;
        Ok(())
    }

    pub fn recover_interrupted(&mut self) -> Result<Option<String>> {
        self.journal.recover_with(&mut self.platform)
    }
}

fn ensure_allowed(key: &str) -> Result<()> {
    if ALLOWED_KEYS.contains(&key) {
        Ok(())
    } else {
        Err(Error::Unsupported(format!(
            "agent refuses key '{key}'; allowed: {}",
            ALLOWED_KEYS.join(", ")
        )))
    }
}

fn read_power_scheme() -> Result<String> {
    #[cfg(windows)]
    {
        let out = std::process::Command::new("powercfg")
            .args(["/getactivescheme"])
            .output()
            .map_err(|e| Error::Platform(format!("powercfg failed: {e}")))?;
        let text = String::from_utf8_lossy(&out.stdout);
        // Example: "Power Scheme GUID: ...  (Balanced)"
        if let Some(start) = text.find('(') {
            if let Some(end) = text[start + 1..].find(')') {
                return Ok(text[start + 1..start + 1 + end].trim().to_string());
            }
        }
        // Fall back to GUID if present.
        for part in text.split_whitespace() {
            if part.len() == 36 && part.contains('-') {
                return Ok(part.to_string());
            }
        }
        Ok(text.trim().to_string())
    }
    #[cfg(target_os = "linux")]
    {
        // Best-effort: cpufreq governor if present.
        let path = "/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor";
        match std::fs::read_to_string(path) {
            Ok(s) => Ok(s.trim().to_string()),
            Err(_) => Ok("unsupported".into()),
        }
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        Err(Error::unsupported("power.scheme unsupported on this OS"))
    }
}

fn set_power_scheme(scheme: &str) -> Result<()> {
    #[cfg(windows)]
    {
        // Accept friendly names mapped to well-known GUIDs, or raw GUID.
        let guid_owned = scheme.to_ascii_lowercase();
        let guid = match guid_owned.as_str() {
            "balanced" => "381b4222-f694-41f0-9685-ff5bb260df2e",
            "high_performance" | "high-performance" | "high" => {
                "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c"
            }
            "power_saver" | "power-saver" | "saver" => "a1841308-3541-4fab-bc81-f71556f20b4a",
            other => other,
        };
        let status = std::process::Command::new("powercfg")
            .args(["/setactive", guid])
            .status()
            .map_err(|e| Error::Platform(format!("powercfg setactive failed: {e}")))?;
        if !status.success() {
            return Err(Error::Platform(format!(
                "powercfg /setactive {guid} failed (may need elevation)"
            )));
        }
        Ok(())
    }
    #[cfg(target_os = "linux")]
    {
        // Only allow known governors — never arbitrary shell.
        let allowed = [
            "performance",
            "powersave",
            "schedutil",
            "ondemand",
            "conservative",
        ];
        if !allowed.contains(&scheme) {
            return Err(Error::InvalidConfig(format!(
                "linux power.scheme must be one of: {}",
                allowed.join(", ")
            )));
        }
        let mut any = false;
        if let Ok(entries) = std::fs::read_dir("/sys/devices/system/cpu") {
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if !name.starts_with("cpu")
                    || !name.chars().nth(3).is_some_and(|c| c.is_ascii_digit())
                {
                    continue;
                }
                let path = e.path().join("cpufreq/scaling_governor");
                if path.exists() {
                    if std::fs::write(&path, scheme).is_ok() {
                        any = true;
                    }
                }
            }
        }
        if !any {
            return Err(Error::Platform(
                "could not write cpufreq governors (need privileges / cpufreq present)".into(),
            ));
        }
        Ok(())
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = scheme;
        Err(Error::unsupported("power.scheme unsupported on this OS"))
    }
}
