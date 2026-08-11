//! Default data directory resolution.

use std::path::PathBuf;

use directories::ProjectDirs;
use kraftverk_core::error::{Error, Result};

pub fn default_data_dir() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("org", "Kraftverk", "kraftverk")
        .ok_or_else(|| Error::Storage("could not resolve platform data directory".into()))?;
    let path = dirs.data_dir().to_path_buf();
    std::fs::create_dir_all(&path)?;
    Ok(path)
}

pub fn default_db_path() -> Result<PathBuf> {
    Ok(default_data_dir()?.join("kraftverk.db"))
}

pub fn recovery_journal_path() -> Result<PathBuf> {
    Ok(default_data_dir()?.join("recovery_journal.json"))
}

pub fn bench_scratch_dir() -> Result<PathBuf> {
    let p = default_data_dir()?.join("bench_scratch");
    std::fs::create_dir_all(&p)?;
    Ok(p)
}
