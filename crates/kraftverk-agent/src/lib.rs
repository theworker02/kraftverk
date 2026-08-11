//! Privileged agent scaffold.
//!
//! Full elevated agent is not required for safe in-process opts.
//! This crate defines the authenticated IPC surface for privileged work.
//! Hardware eligibility (amd-only-v1) is validated on startup and before
//! sensitive apply/rollback operations.

use kraftverk_core::error::{Error, Result};
use kraftverk_system::{evaluate_eligibility, exit_code_for, SessionGuard, HARDWARE_POLICY};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Messages the unprivileged CLI may send to a future privileged agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentRequest {
    Ping {
        id: Uuid,
    },
    ApplyChange {
        id: Uuid,
        key: String,
        previous: serde_json::Value,
        next: serde_json::Value,
    },
    RollbackChange {
        id: Uuid,
        key: String,
        previous: serde_json::Value,
    },
    GetCapabilities {
        id: Uuid,
    },
    CheckHardware {
        id: Uuid,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentResponse {
    Pong {
        id: Uuid,
    },
    Ok {
        id: Uuid,
    },
    Error {
        id: Uuid,
        message: String,
    },
    Capabilities {
        id: Uuid,
        caps: serde_json::Value,
    },
    Hardware {
        id: Uuid,
        supported: bool,
        policy: String,
        summary: String,
        exit_code: Option<i32>,
    },
}

/// Privileged agent runtime state (scaffold).
pub struct PrivilegedAgent {
    guard: SessionGuard,
}

impl PrivilegedAgent {
    /// Start the agent — refuses unsupported hardware immediately.
    pub fn start() -> Result<Self> {
        let guard = SessionGuard::start()?;
        tracing::info!(
            policy = HARDWARE_POLICY,
            summary = %guard.initial.summary(),
            "privileged agent hardware gate passed"
        );
        Ok(Self { guard })
    }

    pub fn handle(&self, req: AgentRequest) -> AgentResponse {
        match req {
            AgentRequest::Ping { id } => AgentResponse::Pong { id },
            AgentRequest::GetCapabilities { id } => AgentResponse::Capabilities {
                id,
                caps: serde_json::json!({
                    "hardware_policy": HARDWARE_POLICY,
                    "eligibility": self.guard.initial,
                    "operational": false,
                    "note": "Agent IPC scaffold — privileged applies not yet live."
                }),
            },
            AgentRequest::CheckHardware { id } => {
                let el = evaluate_eligibility();
                AgentResponse::Hardware {
                    id,
                    supported: el.supported,
                    policy: el.policy.clone(),
                    summary: el.summary(),
                    exit_code: el.primary_rejection().map(|r| exit_code_for(r).as_i32()),
                }
            }
            AgentRequest::ApplyChange { id, .. } | AgentRequest::RollbackChange { id, .. } => {
                match self.guard.assert_still_supported() {
                    Ok(_) => AgentResponse::Error {
                        id,
                        message: "privileged apply/rollback not operational in this build; \
                                  hardware identity re-check passed"
                            .into(),
                    },
                    Err(e) => AgentResponse::Error {
                        id,
                        message: e.to_string(),
                    },
                }
            }
        }
    }
}

/// Validate hardware at agent process entry (for future binary main).
pub fn validate_hardware_or_refuse() -> Result<()> {
    let el = evaluate_eligibility();
    if el.supported {
        Ok(())
    } else {
        Err(Error::UnsupportedHardware(el.summary()))
    }
}

/// Trust boundary notes for documentation / status command.
pub fn trust_boundary_summary() -> &'static str {
    "Safe optimizations run in-process in the kraftverk CLI after amd-only-v1 hardware \
     eligibility passes. Privileged changes (power plans, system-wide affinity for other \
     processes, firmware) require kraftverk-agent over authenticated local IPC. The agent \
     validates AMD-only hardware on startup and re-checks identity before sensitive ops. \
     The agent is scaffolded but not fully operational in 0.2."
}

/// Returns whether a live agent is connected (always false until agent ships).
pub fn agent_connected() -> bool {
    false
}
