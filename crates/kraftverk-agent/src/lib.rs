//! Privileged agent — authenticated local IPC for elevated, reversible ops.
//!
//! Transport: named pipe (Windows) / Unix domain socket (Linux).
//! No network bind by default. Auth handshake required before any command.
//! Hardware policy `amd-only-v1` is re-checked on start and sensitive ops.

mod apply;
mod auth;
mod client;
mod protocol;
mod server;
mod transport;

pub use apply::PrivilegedOps;
pub use auth::{ensure_agent_token, load_agent_token, token_path};
pub use client::{agent_connected, connect, AgentClient};
pub use protocol::{AgentRequest, AgentResponse, AuthChallenge, FRAME_MAGIC};
pub use server::{run_agent_server, PrivilegedAgent};

use kraftverk_core::error::{Error, Result};
use kraftverk_system::{evaluate_eligibility, HARDWARE_POLICY};

/// Trust boundary notes for documentation / status command.
pub fn trust_boundary_summary() -> &'static str {
    "Safe optimizations run in-process in the kraftverk CLI after amd-only-v1 hardware \
     eligibility passes. Privileged changes (priority/affinity for other processes, power \
     schemes) require kraftverk-agent over authenticated local IPC (named pipe / Unix socket). \
     The agent validates AMD-only hardware on startup and re-checks identity before sensitive ops. \
     Start elevated with: kraftverk agent serve"
}

/// Validate hardware at agent process entry.
pub fn validate_hardware_or_refuse() -> Result<()> {
    let el = evaluate_eligibility();
    if el.supported {
        Ok(())
    } else {
        Err(Error::UnsupportedHardware(el.summary()))
    }
}

/// Default IPC endpoint path/name for this user/session.
pub fn default_endpoint() -> String {
    transport::default_endpoint()
}

pub fn policy_id() -> &'static str {
    HARDWARE_POLICY
}
