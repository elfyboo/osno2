use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryTrack {
    pub id: String, // sha256 of canonical path
    pub path: PathBuf,
    pub name: String,
    pub artist: String,
    pub album: String,
    pub year: Option<u16>,
    pub duration_secs: u64,
    pub size_bytes: u64,
    pub ext: String,
    pub bitrate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub codec: Option<String>,
    pub rating: u8,    // 0-5
    pub added_at: u64, // unix timestamp
}
