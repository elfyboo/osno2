use crate::core::model::library_track::LibraryTrack;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    #[error("track not found: {0}")]
    TrackNotFound(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Abstraction over where library data comes from. Production uses
/// `FsLibrary` (real filesystem scan); dev/test uses `SpoofedLibrary`
/// (fixture data, no I/O).
pub trait LibraryBackend: Send + Sync {
    fn list_tracks(&self) -> Result<Vec<LibraryTrack>, LibraryError>;
    fn get_meta(&self, id: &str) -> Result<LibraryTrack, LibraryError>;
}

pub struct FsLibrary {
    root: PathBuf,
}

impl FsLibrary {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl LibraryBackend for FsLibrary {
    fn list_tracks(&self) -> Result<Vec<LibraryTrack>, LibraryError> {
        // Delegate to existing fs/ scanning (jwalk-based, per Cargo.toml)
        // and library/library_track.rs for metadata extraction.
        // Stub left for integration with existing fs::fs_entry module.
        let _ = &self.root;
        Ok(Vec::new())
    }

    fn get_meta(&self, id: &str) -> Result<LibraryTrack, LibraryError> {
        Err(LibraryError::TrackNotFound(id.to_string()))
    }
}
