use crate::fs::fs_entry::FsEntry;
use crate::library::library_track::LibraryTrack;
use std::path::PathBuf;

pub enum ViewState {
    Filesystem {
        cwd: PathBuf,
        entries: Vec<FsEntry>,
    },
    Tracklist {
        tracks: Vec<LibraryTrack>,
        selected: usize,
    },
    Playlist {
        name: String,
        tracks: Vec<LibraryTrack>,
        selected: usize,
    },
}
