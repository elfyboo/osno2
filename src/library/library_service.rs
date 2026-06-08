use crate::library::fs_entry::FsEntry;
use crate::library::playlist::Playlist;
use crate::library::track::Track;
use std::path::{Path, PathBuf};

pub struct LibraryService {
    // redb database handle
    db: redb::Database,
    // path to meta/*.toml directory
    meta_dir: PathBuf,
    // path to playlists/*.toml directory
    playlists_dir: PathBuf,
}

impl LibraryService {
    pub fn new(db: redb::Database, meta_dir: PathBuf, playlists_dir: PathBuf) -> Self {
        Self {
            db,
            meta_dir,
            playlists_dir,
        }
    }

    pub fn read_dir(&self, _path: &Path) -> Result<Vec<FsEntry>, std::io::Error> {
        todo!()
    }

    pub fn all_tracks(&self) -> Result<Vec<Track>, std::io::Error> {
        todo!()
    }

    pub fn search(&self, query: &str) -> Result<Vec<Track>, std::io::Error> {
        todo!()
    }

    pub fn add_track(&self, path: &Path) -> Result<Track, std::io::Error> {
        todo!()
    }

    pub fn remove_track(&self, id: &str) -> Result<(), std::io::Error> {
        todo!()
    }

    pub fn update_rating(&self, id: &str, rating: u8) -> Result<(), std::io::Error> {
        todo!()
    }

    pub fn reindex(&self) -> Result<(), std::io::Error> {
        todo!()
    }

    pub fn all_playlists(&self) -> Result<Vec<Playlist>, std::io::Error> {
        todo!()
    }

    pub fn playlist(&self, name: &str) -> Result<Option<Vec<Track>>, std::io::Error> {
        todo!()
    }

    pub fn create_playlist(&self, name: &str) -> Result<(), std::io::Error> {
        todo!()
    }

    pub fn add_to_playlist(&self, playlist: &str, id: &str) -> Result<(), std::io::Error> {
        todo!()
    }

    pub fn remove_from_playlist(&self, playlist: &str, id: &str) -> Result<(), std::io::Error> {
        todo!()
    }
}
