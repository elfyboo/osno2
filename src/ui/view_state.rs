use crate::library::fs_entry::FsEntry;
use crate::library::track::Track;
use std::path::PathBuf;

pub enum ViewState {
    Filesystem {
        cwd: PathBuf,
        entries: Vec<FsEntry>,
    },
    Tracklist {
        tracks: Vec<Track>,
        selected: usize,
    },
    Playlist {
        name: String,
        tracks: Vec<Track>,
        selected: usize,
    },
}
