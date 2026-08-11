//! Privileged agent server loop.

use kraftverk_core::candidate::ParamValue;
use kraftverk_core::error::Result;
use kraftverk_system::{evaluate_eligibility, exit_code_for, SessionGuard, HARDWARE_POLICY};
use tracing::{info, warn};
use uuid::Uuid;

use crate::apply::PrivilegedOps;
use crate::auth::{ensure_agent_token, token_fingerprint};
use crate::protocol::{AgentRequest, AgentResponse};
use crate::transport::{self, default_endpoint};

/// Privileged agent runtime state.
pub struct PrivilegedAgent {
    pub guard: SessionGuard,
    pub ops: PrivilegedOps,
    pub endpoint: String,
}

impl PrivilegedAgent {
    /// Start the agent — refuses unsupported hardware immediately.
    pub fn start() -> Result<Self> {
        let guard = SessionGuard::start()?;
        let mut ops = PrivilegedOps::open()?;
        if let Some(id) = ops.recover_interrupted()? {
            warn!(experiment = %id, "recovered interrupted agent journal");
        }
        tracing::info!(
            policy = HARDWARE_POLICY,
            summary = %guard.initial.summary(),
            "privileged agent hardware gate passed"
        );
        Ok(Self {
            guard,
            ops,
            endpoint: default_endpoint(),
        })
    }
}

pub fn handle_request(
    agent: &mut PrivilegedAgent,
    req: AgentRequest,
    authed: &mut bool,
    expected_token: &str,
) -> AgentResponse {
    // Auth gate: only Auth allowed until handshake succeeds.
    if !*authed {
        match &req {
            AgentRequest::Auth { id, token, .. } => {
                if crate::auth::tokens_match(token, expected_token) {
                    *authed = true;
                    return AgentResponse::Authed {
                        id: *id,
                        agent_version: env!("CARGO_PKG_VERSION").into(),
                    };
                }
                return AgentResponse::Error {
                    id: *id,
                    message: "authentication failed".into(),
                };
            }
            other => {
                let id = request_id(other);
                return AgentResponse::Error {
                    id,
                    message: "unauthenticated: send Auth first".into(),
                };
            }
        }
    }

    match req {
        AgentRequest::Auth { id, .. } => AgentResponse::Authed {
            id,
            agent_version: env!("CARGO_PKG_VERSION").into(),
        },
        AgentRequest::Ping { id } => AgentResponse::Pong { id },
        AgentRequest::Health { id } => {
            let el = evaluate_eligibility();
            AgentResponse::Health {
                id,
                ok: true,
                elevated_hint: is_elevated_hint(),
                endpoint: agent.endpoint.clone(),
                hardware_ok: el.supported,
                summary: el.summary(),
            }
        }
        AgentRequest::GetCapabilities { id } => AgentResponse::Capabilities {
            id,
            caps: serde_json::json!({
                "hardware_policy": HARDWARE_POLICY,
                "eligibility": agent.guard.initial,
                "operational": true,
                "allowed_keys": crate::apply::ALLOWED_KEYS,
                "endpoint": agent.endpoint,
                "token_fp": token_fingerprint(expected_token),
                "note": "Authenticated local IPC agent; no arbitrary shell."
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
        AgentRequest::GetParam { id, key } => match agent.ops.get_param(&key) {
            Ok(v) => AgentResponse::Value {
                id,
                key,
                value: serde_json::to_value(v).unwrap_or(serde_json::Value::Null),
            },
            Err(e) => AgentResponse::Error {
                id,
                message: e.to_string(),
            },
        },
        AgentRequest::VerifyChange { id, key, expected } => {
            let expected = match value_to_param(&expected) {
                Ok(v) => v,
                Err(e) => return AgentResponse::Error { id, message: e },
            };
            match agent.ops.verify(&key, &expected) {
                Ok(ok) => AgentResponse::Verified { id, ok },
                Err(e) => AgentResponse::Error {
                    id,
                    message: e.to_string(),
                },
            }
        }
        AgentRequest::ApplyChange {
            id,
            key,
            previous,
            next,
        } => {
            if let Err(e) = agent.guard.assert_still_supported() {
                return AgentResponse::Error {
                    id,
                    message: e.to_string(),
                };
            }
            let prev = match value_to_param(&previous) {
                Ok(v) => v,
                Err(e) => return AgentResponse::Error { id, message: e },
            };
            let nxt = match value_to_param(&next) {
                Ok(v) => v,
                Err(e) => return AgentResponse::Error { id, message: e },
            };
            match agent.ops.apply(&key, prev, nxt) {
                Ok(()) => AgentResponse::Ok {
                    id,
                    detail: Some(format!("applied {key}")),
                },
                Err(e) => AgentResponse::Error {
                    id,
                    message: e.to_string(),
                },
            }
        }
        AgentRequest::RollbackChange { id, key, previous } => {
            if let Err(e) = agent.guard.assert_still_supported() {
                return AgentResponse::Error {
                    id,
                    message: e.to_string(),
                };
            }
            let prev = match value_to_param(&previous) {
                Ok(v) => v,
                Err(e) => return AgentResponse::Error { id, message: e },
            };
            match agent.ops.rollback(&key, prev) {
                Ok(()) => AgentResponse::Ok {
                    id,
                    detail: Some(format!("rolled back {key}")),
                },
                Err(e) => AgentResponse::Error {
                    id,
                    message: e.to_string(),
                },
            }
        }
    }
}

fn request_id(req: &AgentRequest) -> Uuid {
    match req {
        AgentRequest::Auth { id, .. }
        | AgentRequest::Ping { id }
        | AgentRequest::Health { id }
        | AgentRequest::ApplyChange { id, .. }
        | AgentRequest::VerifyChange { id, .. }
        | AgentRequest::RollbackChange { id, .. }
        | AgentRequest::GetParam { id, .. }
        | AgentRequest::GetCapabilities { id }
        | AgentRequest::CheckHardware { id } => *id,
    }
}

fn value_to_param(v: &serde_json::Value) -> std::result::Result<ParamValue, String> {
    if let Ok(p) = serde_json::from_value::<ParamValue>(v.clone()) {
        return Ok(p);
    }
    match v {
        serde_json::Value::String(s) => Ok(ParamValue::String(s.clone())),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(ParamValue::Int(i))
            } else if let Some(f) = n.as_f64() {
                Ok(ParamValue::Float(f))
            } else {
                Err("invalid number".into())
            }
        }
        serde_json::Value::Bool(b) => Ok(ParamValue::Bool(*b)),
        _ => Err("unsupported param JSON".into()),
    }
}

fn is_elevated_hint() -> bool {
    #[cfg(windows)]
    {
        windows_is_elevated()
    }
    #[cfg(unix)]
    {
        unsafe { libc::geteuid() == 0 }
    }
    #[cfg(not(any(windows, unix)))]
    {
        false
    }
}

#[cfg(windows)]
fn windows_is_elevated() -> bool {
    use std::mem::MaybeUninit;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    #[link(name = "advapi32")]
    extern "system" {
        fn OpenProcessToken(
            process_handle: HANDLE,
            desired_access: u32,
            token_handle: *mut HANDLE,
        ) -> i32;
    }

    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut elev = MaybeUninit::<TOKEN_ELEVATION>::uninit();
        let mut ret_len = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            elev.as_mut_ptr() as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret_len,
        );
        let _ = CloseHandle(token);
        if ok == 0 {
            return false;
        }
        elev.assume_init().TokenIsElevated != 0
    }
}

/// Blocking accept loop.
pub fn run_agent_server() -> Result<()> {
    validate_or_exit()?;
    let token = ensure_agent_token()?;
    let mut agent = PrivilegedAgent::start()?;
    let endpoint = agent.endpoint.clone();
    info!(
        endpoint = %endpoint,
        token_fp = %token_fingerprint(&token),
        "kraftverk-agent listening (local IPC only)"
    );

    #[cfg(unix)]
    {
        let listener = transport::unix_sock::listen(&endpoint)?;
        loop {
            match transport::unix_sock::accept(&listener) {
                Ok(mut stream) => {
                    let mut authed = false;
                    while let Ok(req) = transport::read_json::<AgentRequest>(&mut stream) {
                        let resp = handle_request(&mut agent, req, &mut authed, &token);
                        if transport::write_json(&mut stream, &resp).is_err() {
                            break;
                        }
                    }
                }
                Err(e) => warn!(error = %e, "accept failed"),
            }
        }
    }

    #[cfg(windows)]
    {
        loop {
            match transport::win_pipe::PipeStream::listen_and_accept(&endpoint) {
                Ok(mut stream) => {
                    let mut authed = false;
                    while let Ok(req) = transport::read_json::<AgentRequest>(&mut stream) {
                        let resp = handle_request(&mut agent, req, &mut authed, &token);
                        if transport::write_json(&mut stream, &resp).is_err() {
                            break;
                        }
                    }
                }
                Err(e) => warn!(error = %e, "named pipe accept failed"),
            }
        }
    }

    #[cfg(not(any(windows, unix)))]
    {
        Err(kraftverk_core::Error::unsupported(
            "agent IPC unsupported on this OS",
        ))
    }
}

fn validate_or_exit() -> Result<()> {
    crate::validate_hardware_or_refuse()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::AgentRequest;
    use uuid::Uuid;

    #[test]
    fn rejects_unauthenticated() {
        // SessionGuard requires real AMD hardware — skip on non-eligible CI hosts.
        let el = evaluate_eligibility();
        if !el.supported {
            return;
        }
        let mut agent = PrivilegedAgent::start().expect("start");
        let mut authed = false;
        let resp = handle_request(
            &mut agent,
            AgentRequest::Ping { id: Uuid::new_v4() },
            &mut authed,
            "secret-token-value-32chars-minimum!!",
        );
        assert!(matches!(resp, AgentResponse::Error { .. }));
    }

    #[test]
    fn auth_then_ping() {
        let el = evaluate_eligibility();
        if !el.supported {
            return;
        }
        let token = "secret-token-value-32chars-minimum!!";
        let mut agent = PrivilegedAgent::start().expect("start");
        let mut authed = false;
        let id = Uuid::new_v4();
        let resp = handle_request(
            &mut agent,
            AgentRequest::Auth {
                id,
                token: token.into(),
                client: "test".into(),
            },
            &mut authed,
            token,
        );
        assert!(matches!(resp, AgentResponse::Authed { .. }));
        assert!(authed);
        let resp = handle_request(
            &mut agent,
            AgentRequest::Ping { id: Uuid::new_v4() },
            &mut authed,
            token,
        );
        assert!(matches!(resp, AgentResponse::Pong { .. }));
    }
}
