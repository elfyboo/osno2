use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryPlaylist {
    pub name: String,
    pub track_ids: Vec<String>,
}
