//! Shared secret token for local agent auth.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use kraftverk_core::error::{Error, Result};
use rand::RngCore;
use sha2::{Digest, Sha256};

pub fn agent_data_dir() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("org", "Kraftverk", "kraftverk")
        .ok_or_else(|| Error::Storage("could not resolve platform data directory".into()))?;
    let path = dirs.data_dir().join("agent");
    fs::create_dir_all(&path)?;
    Ok(path)
}

pub fn token_path() -> Result<PathBuf> {
    Ok(agent_data_dir()?.join("auth.token"))
}

/// Create or load a 32-byte hex token. Restrictive perms on Unix.
pub fn ensure_agent_token() -> Result<String> {
    let path = token_path()?;
    if path.exists() {
        return load_agent_token();
    }
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let token = hex::encode(bytes);
    let mut f = fs::File::create(&path)?;
    f.write_all(token.as_bytes())?;
    f.flush()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    Ok(token)
}

pub fn load_agent_token() -> Result<String> {
    let path = token_path()?;
    let s = fs::read_to_string(&path)
        .map_err(|e| Error::Storage(format!("agent token missing at {}: {e}", path.display())))?;
    let s = s.trim().to_string();
    if s.len() < 16 {
        return Err(Error::Storage("agent token too short / corrupt".into()));
    }
    Ok(s)
}

pub fn token_fingerprint(token: &str) -> String {
    let mut h = Sha256::new();
    h.update(token.as_bytes());
    hex::encode(h.finalize())[..12].to_string()
}

pub fn tokens_match(a: &str, b: &str) -> bool {
    // Constant-time-ish compare for equal-length secrets.
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        diff |= x ^ y;
    }
    diff == 0
}
