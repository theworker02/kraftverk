//! Length-prefixed JSON IPC protocol.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Magic bytes at the start of every frame (little-endian u32 length follows).
pub const FRAME_MAGIC: &[u8; 4] = b"KVAG";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthChallenge {
    pub protocol: u32,
    pub nonce: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentRequest {
    /// First message after connect; must include shared token.
    Auth {
        id: Uuid,
        token: String,
        client: String,
    },
    Ping {
        id: Uuid,
    },
    Health {
        id: Uuid,
    },
    ApplyChange {
        id: Uuid,
        key: String,
        previous: serde_json::Value,
        next: serde_json::Value,
    },
    VerifyChange {
        id: Uuid,
        key: String,
        expected: serde_json::Value,
    },
    RollbackChange {
        id: Uuid,
        key: String,
        previous: serde_json::Value,
    },
    GetParam {
        id: Uuid,
        key: String,
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
    Authed {
        id: Uuid,
        agent_version: String,
    },
    Ok {
        id: Uuid,
        detail: Option<String>,
    },
    Value {
        id: Uuid,
        key: String,
        value: serde_json::Value,
    },
    Verified {
        id: Uuid,
        ok: bool,
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
    Health {
        id: Uuid,
        ok: bool,
        elevated_hint: bool,
        endpoint: String,
        hardware_ok: bool,
        summary: String,
    },
}
