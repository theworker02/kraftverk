//! Platform adapter that routes privileged keys through the local agent IPC
//! when available, otherwise falls back to in-process native ops (except
//! `power.scheme`, which always requires the agent).

use kraftverk_core::candidate::{ParamChange, ParamValue};
use kraftverk_core::error::{Error, Result};
use kraftverk_core::OptimizeMode;
use kraftverk_system::{
    Capabilities, Capability, FeatureSupport, NativePlatform, Platform, Topology,
};
use uuid::Uuid;

use crate::apply::ALLOWED_KEYS;
use crate::client::{agent_connected, AgentClient};
use crate::protocol::{AgentRequest, AgentResponse};

/// Keys that may be elevated via the privileged agent.
pub fn is_privileged_key(key: &str) -> bool {
    ALLOWED_KEYS.contains(&key)
}

/// Native + optional agent-backed platform used by the CLI optimize path.
pub struct AgentBackedPlatform {
    native: NativePlatform,
}

impl AgentBackedPlatform {
    pub fn detect() -> Result<Self> {
        Ok(Self {
            native: NativePlatform::detect()?,
        })
    }

    pub fn agent_reachable(&self) -> bool {
        agent_connected()
    }

    fn require_agent(key: &str) -> Result<AgentClient> {
        AgentClient::connect_default().map_err(|e| {
            Error::Platform(format!(
                "privileged key '{key}' requires a running kraftverk agent \
                 (`kraftverk agent serve`; elevate on Windows for power schemes). \
                 Connect failed: {e}"
            ))
        })
    }

    fn route_via_agent(key: &str) -> bool {
        match key {
            "power.scheme" => true,
            "process.priority" | "process.affinity" => agent_connected(),
            _ => false,
        }
    }
}

impl Platform for AgentBackedPlatform {
    fn name(&self) -> &str {
        self.native.name()
    }

    fn capabilities(&self) -> Capabilities {
        let mut caps = self.native.capabilities();
        let agent = agent_connected();
        if let Some(c) = caps
            .features
            .iter_mut()
            .find(|f| f.id == "power.scheme" || f.id == "power.plan")
        {
            c.id = "power.scheme".into();
            c.name = "OS power scheme / plan".into();
            if agent {
                c.support = FeatureSupport::Partial;
                c.notes = "Available via privileged agent (authenticated local IPC).".into();
            } else {
                c.support = FeatureSupport::RequiresPrivilege;
                c.notes = "Start `kraftverk agent serve` (elevated on Windows) to enable.".into();
            }
        } else {
            caps.features.push(Capability {
                id: "power.scheme".into(),
                name: "OS power scheme / plan".into(),
                support: if agent {
                    FeatureSupport::Partial
                } else {
                    FeatureSupport::RequiresPrivilege
                },
                notes: if agent {
                    "Available via privileged agent.".into()
                } else {
                    "Start `kraftverk agent serve` (elevated on Windows) to enable.".into()
                },
            });
        }
        if agent {
            for id in ["process.priority", "process.affinity"] {
                if let Some(c) = caps.features.iter_mut().find(|f| f.id == id) {
                    c.notes = format!(
                        "{}; privileged agent connected — applies via authenticated IPC.",
                        c.notes
                    );
                }
            }
        }
        caps
    }

    fn topology(&self) -> Result<Topology> {
        self.native.topology()
    }

    fn read_param(&self, key: &str) -> Result<ParamValue> {
        if key == "power.scheme" {
            let mut client = Self::require_agent(key)?;
            let id = Uuid::new_v4();
            return match client.request(AgentRequest::GetParam {
                id,
                key: key.into(),
            })? {
                AgentResponse::Value { value, .. } => serde_json::from_value(value)
                    .map_err(|e| Error::Platform(format!("agent param decode: {e}"))),
                AgentResponse::Error { message, .. } => Err(Error::Platform(message)),
                _ => Err(Error::Platform("unexpected agent GetParam response".into())),
            };
        }
        self.native.read_param(key)
    }

    fn apply_change(&mut self, change: &ParamChange) -> Result<()> {
        if Self::route_via_agent(&change.key) {
            let mut client = Self::require_agent(&change.key)?;
            let id = Uuid::new_v4();
            let resp = client.request(AgentRequest::ApplyChange {
                id,
                key: change.key.clone(),
                previous: serde_json::to_value(&change.previous)
                    .map_err(|e| Error::Platform(e.to_string()))?,
                next: serde_json::to_value(&change.next)
                    .map_err(|e| Error::Platform(e.to_string()))?,
            })?;
            return match resp {
                AgentResponse::Ok { .. } => Ok(()),
                AgentResponse::Error { message, .. } => Err(Error::Platform(message)),
                _ => Err(Error::Platform(
                    "unexpected agent ApplyChange response".into(),
                )),
            };
        }
        if change.key == "power.scheme" {
            return Err(Error::Platform(
                "power.scheme requires kraftverk agent serve (elevate on Windows for powercfg)"
                    .into(),
            ));
        }
        self.native.apply_change(change)
    }

    fn verify_change(&self, change: &ParamChange) -> Result<bool> {
        if Self::route_via_agent(&change.key) {
            let mut client = Self::require_agent(&change.key)?;
            let id = Uuid::new_v4();
            let resp = client.request(AgentRequest::VerifyChange {
                id,
                key: change.key.clone(),
                expected: serde_json::to_value(&change.next)
                    .map_err(|e| Error::Platform(e.to_string()))?,
            })?;
            return match resp {
                AgentResponse::Verified { ok, .. } => Ok(ok),
                AgentResponse::Error { message, .. } => Err(Error::Platform(message)),
                _ => Err(Error::Platform(
                    "unexpected agent VerifyChange response".into(),
                )),
            };
        }
        self.native.verify_change(change)
    }

    fn rollback_change(&mut self, change: &ParamChange) -> Result<()> {
        if Self::route_via_agent(&change.key) {
            let mut client = Self::require_agent(&change.key)?;
            let id = Uuid::new_v4();
            let resp = client.request(AgentRequest::RollbackChange {
                id,
                key: change.key.clone(),
                previous: serde_json::to_value(&change.previous)
                    .map_err(|e| Error::Platform(e.to_string()))?,
            })?;
            return match resp {
                AgentResponse::Ok { .. } => Ok(()),
                AgentResponse::Error { message, .. } => Err(Error::Platform(message)),
                _ => Err(Error::Platform(
                    "unexpected agent RollbackChange response".into(),
                )),
            };
        }
        self.native.rollback_change(change)
    }

    fn score_multiplier(&self) -> f64 {
        self.native.score_multiplier()
    }

    fn mode_allowed(&self, mode: OptimizeMode) -> Result<()> {
        let _ = mode;
        Ok(())
    }

    fn safe_param_keys(&self) -> Vec<String> {
        let mut keys = self.native.safe_param_keys();
        if agent_connected() && !keys.iter().any(|k| k == "power.scheme") {
            keys.push("power.scheme".into());
        }
        keys
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn privileged_key_set() {
        assert!(is_privileged_key("process.priority"));
        assert!(is_privileged_key("process.affinity"));
        assert!(is_privileged_key("power.scheme"));
        assert!(!is_privileged_key("bench.worker_threads"));
        assert!(!is_privileged_key("shell.exec"));
    }
}
