//! Lightweight playback session, separate from the music library so position
//! updates never rewrite the whole library file.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::storage;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Session {
    #[serde(default)]
    pub schema: u32,
    #[serde(default)]
    pub current: Option<u64>,
    #[serde(default)]
    pub position: f64,
    #[serde(default)]
    pub queue: Vec<u64>,
    #[serde(default)]
    pub queue_index: usize,
    #[serde(default)]
    pub view: String,
    #[serde(default)]
    pub volume: Option<f32>,
    #[serde(default)]
    pub saved_at: u64,
}

fn session_path() -> PathBuf {
    storage::data_dir().join("session.json")
}

pub fn load() -> Option<Session> {
    let bytes = fs::read(session_path()).ok()?;
    let session: Session = serde_json::from_slice(&bytes).ok()?;
    Some(session)
}

pub fn save(session: &Session) -> std::io::Result<()> {
    let path = session_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(session)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))?;
    fs::write(&temp, bytes)?;
    fs::rename(temp, path)?;
    Ok(())
}
