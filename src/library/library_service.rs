use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::fs::fs_entry::{FsEntry, FsEntryKind};
use crate::library::library_playlist::LibraryPlaylist;
use crate::library::library_track::LibraryTrack;
use redb::{ReadableDatabase, ReadableTable, TableDefinition};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia_core::formats::TrackType;
use symphonia_core::formats::probe::Hint;
use symphonia_core::meta::StandardTag;
const TRACKS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("tracks");

pub struct LibraryService {
    db: redb::Database,
    meta_dir: PathBuf,
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
    fn extract_track_metadata(&self, path: &Path) -> Result<LibraryTrack, std::io::Error> {
        let clean_path = dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let file = File::open(&clean_path)?;
        let size_bytes = file.metadata()?.len();

        let ext = clean_path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();

        let name = clean_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();

        // Generate SHA-256 ID out of the canonical path string
        let id: String = "???".to_string();

        // format!(
        //     "{:x}",
        //     sha2::Sha256::digest(clean_path.to_string_lossy().as_bytes())
        // );

        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        let mut hint = Hint::new();
        hint.with_extension(&ext);

        // 1. Initialize demuxer. (Mandatory step to read container headers)
        let mut probe = symphonia::core::formats::probe::Probe::default();
        symphonia::default::register_enabled_formats(&mut probe);

        let mut format_reader = probe
            .probe(
                &hint,
                mss,
                FormatOptions::default(),
                MetadataOptions::default(),
            )
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?; // We extract the `FormatReader` out of the ProbeResult struct here

        let mut duration_secs: u64 = 0;
        let mut sample_rate: Option<u32> = None;
        let mut bitrate: Option<u32> = None;
        let mut codec_name: Option<String> = None;

        // fetch audio track
        if let Some(track) = format_reader.default_track(TrackType::Audio) {
            if let Some(params) = track.codec_params.clone() {
                if params.is_audio() {
                    if let Some(audio_params) = params.audio() {
                        sample_rate = audio_params.sample_rate;

                        if let (Some(frames), Some(sample_rate)) =
                            (track.num_frames, audio_params.sample_rate)
                        {
                            duration_secs = (frames as f64 / sample_rate as f64) as u64;
                        } else {
                            duration_secs = 0;
                        }

                        bitrate = audio_params.bits_per_sample.or_else(|| {
                            if duration_secs > 0 {
                                Some(((size_bytes * 8) / duration_secs) as u32)
                            } else {
                                None
                            }
                        });

                        codec_name = Some(audio_params.codec.to_string());
                    }
                }
            }
        }

        // 4. Extract Text Tags (Artist, Album, Title, Year)
        let mut track_name = name.clone(); // Fallback to filename
        let mut artist = "Unknown Artist".to_string();
        let mut album = "Unknown Album".to_string();
        let mut year: Option<u16> = None;

        if let Some(metadata) = format_reader.metadata().current() {
            for tag in metadata.clone().media.tags {
                if let Some(std_key) = tag.clone().std {
                    match std_key {
                        StandardTag::TrackTitle(val) => track_name = val.to_string(),
                        StandardTag::Artist(val) => artist = val.to_string(),
                        StandardTag::Album(val) => album = val.to_string(),
                        StandardTag::ReleaseYear(val) => year = Some(val),
                        _ => {}
                    }
                }
            }
        }

        let added_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Ok(LibraryTrack {
            id,
            path: clean_path,
            name: track_name,
            artist,
            album,
            year,
            duration_secs,
            size_bytes,
            ext,
            bitrate,
            sample_rate,
            codec: codec_name,
            rating: 0,
            added_at,
        })
    }

    pub fn read_dir(&self, path: &Path) -> Result<Vec<FsEntry>, std::io::Error> {
        let mut entries: Vec<FsEntry> = std::fs::read_dir(path)?
            .filter_map(|e| e.ok())
            .map(|e| {
                let path = e.path();
                let name = e.file_name().to_string_lossy().to_string();
                let is_dir = path.is_dir();
                let size_bytes = e.metadata().map(|m| m.len()).unwrap_or(0);
                let ext = path
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase();
                let kind = if is_dir {
                    FsEntryKind::Directory
                } else {
                    FsEntryKind::File
                };
                FsEntry {
                    path,
                    name,
                    ext,
                    size_bytes,
                    is_dir,
                    kind,
                }
            })
            .collect();

        // Dirs first, then files, both alphabetical
        entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));

        Ok(entries)
    }

    pub fn all_tracks(&self) -> Result<Vec<LibraryTrack>, std::io::Error> {
        let read_txn = self
            .db
            .begin_read()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        let table = match read_txn.open_table(TRACKS_TABLE) {
            Ok(t) => t,
            // Table doesn't exist yet -- library is empty
            Err(_) => return Ok(Vec::new()),
        };

        let mut tracks = Vec::new();
        for entry in table
            .iter()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
        {
            let (_, value) =
                entry.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            let toml_str = std::str::from_utf8(value.value())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            if let Ok(track) = toml::from_str::<LibraryTrack>(toml_str) {
                tracks.push(track);
            }
        }

        Ok(tracks)
    }

    pub fn search(&self, query: &str) -> Result<Vec<LibraryTrack>, std::io::Error> {
        let q = query.to_lowercase();
        let tracks = self.all_tracks()?;
        Ok(tracks
            .into_iter()
            .filter(|t| {
                t.name.to_lowercase().contains(&q)
                    || t.artist.to_lowercase().contains(&q)
                    || t.album.to_lowercase().contains(&q)
            })
            .collect())
    }

    pub fn add_track(&self, path: &Path) -> Result<LibraryTrack, std::io::Error> {
        let track = self.extract_track_metadata(path)?;

        let toml_string = toml::to_string(&track)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let write_txn = self
            .db
            .begin_write()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        {
            let mut table = write_txn
                .open_table(TRACKS_TABLE)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            table
                .insert(track.id.as_str(), toml_string.as_bytes())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        }
        write_txn
            .commit()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        Ok(track)
    }

    pub fn remove_track(&self, id: &str) -> Result<(), std::io::Error> {
        let write_txn = self
            .db
            .begin_write()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        {
            let mut table = write_txn
                .open_table(TRACKS_TABLE)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            table
                .remove(id)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        }
        write_txn
            .commit()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(())
    }

    pub fn update_rating(&self, id: &str, rating: u8) -> Result<(), std::io::Error> {
        let mut track = self
            .all_tracks()?
            .into_iter()
            .find(|t| t.id == id)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Track not found"))?;

        track.rating = rating.min(5);

        let toml_string = toml::to_string(&track)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let write_txn = self
            .db
            .begin_write()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        {
            let mut table = write_txn
                .open_table(TRACKS_TABLE)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            table
                .insert(id, toml_string.as_bytes())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        }
        write_txn
            .commit()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(())
    }

    pub fn reindex(&self) -> Result<(), std::io::Error> {
        // Walk meta_dir for any .toml track files and re-insert into redb
        if !self.meta_dir.exists() {
            return Ok(());
        }

        let write_txn = self
            .db
            .begin_write()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        {
            let mut table = write_txn
                .open_table(TRACKS_TABLE)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

            for entry in std::fs::read_dir(&self.meta_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().unwrap_or_default() == "toml" {
                    if let Ok(contents) = std::fs::read_to_string(&path) {
                        if let Ok(track) = toml::from_str::<LibraryTrack>(&contents) {
                            let _ = table.insert(track.id.as_str(), contents.as_bytes());
                        }
                    }
                }
            }
        }
        write_txn
            .commit()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(())
    }

    pub fn all_playlists(&self) -> Result<Vec<LibraryPlaylist>, std::io::Error> {
        if !self.playlists_dir.exists() {
            return Ok(Vec::new());
        }

        let mut playlists = Vec::new();
        for entry in std::fs::read_dir(&self.playlists_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().unwrap_or_default() == "toml" {
                if let Ok(contents) = std::fs::read_to_string(&path) {
                    if let Ok(playlist) = toml::from_str::<LibraryPlaylist>(&contents) {
                        playlists.push(playlist);
                    }
                }
            }
        }
        Ok(playlists)
    }

    pub fn playlist(&self, name: &str) -> Result<Option<Vec<LibraryTrack>>, std::io::Error> {
        let safe_name = name.replace(|c: char| !c.is_alphanumeric(), "_");
        let file_path = self.playlists_dir.join(format!("{}.toml", safe_name));

        if !file_path.exists() {
            return Ok(None);
        }

        let contents = std::fs::read_to_string(&file_path)?;
        let playlist: LibraryPlaylist = toml::from_str(&contents)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let all = self.all_tracks()?;
        let tracks = playlist
            .track_ids
            .iter()
            .filter_map(|id| all.iter().find(|t| &t.id == id).cloned())
            .collect();

        Ok(Some(tracks))
    }

    pub fn create_playlist(&self, name: &str) -> Result<(), std::io::Error> {
        let safe_name = name.replace(|c: char| !c.is_alphanumeric(), "_");
        let file_path = self.playlists_dir.join(format!("{}.toml", safe_name));

        if file_path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "Playlist already exists",
            ));
        }

        std::fs::create_dir_all(&self.playlists_dir)?;

        let playlist = LibraryPlaylist {
            name: name.to_string(),
            track_ids: Vec::new(),
        };

        let toml_str = toml::to_string(&playlist)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(file_path, toml_str)?;
        Ok(())
    }

    pub fn add_to_playlist(
        &self,
        playlist_name: &str,
        track_id: &str,
    ) -> Result<(), std::io::Error> {
        let safe_name = playlist_name.replace(|c: char| !c.is_alphanumeric(), "_");
        let file_path = self.playlists_dir.join(format!("{}.toml", safe_name));

        let contents = std::fs::read_to_string(&file_path)?;
        let mut playlist: LibraryPlaylist = toml::from_str(&contents)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        if !playlist.track_ids.contains(&track_id.to_string()) {
            playlist.track_ids.push(track_id.to_string());
        }

        let toml_str = toml::to_string(&playlist)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(file_path, toml_str)?;
        Ok(())
    }

    pub fn remove_from_playlist(
        &self,
        playlist_name: &str,
        track_id: &str,
    ) -> Result<(), std::io::Error> {
        let safe_name = playlist_name.replace(|c: char| !c.is_alphanumeric(), "_");
        let file_path = self.playlists_dir.join(format!("{}.toml", safe_name));

        let contents = std::fs::read_to_string(&file_path)?;
        let mut playlist: LibraryPlaylist = toml::from_str(&contents)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        playlist.track_ids.retain(|id| id != track_id);

        let toml_str = toml::to_string(&playlist)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(file_path, toml_str)?;
        Ok(())
    }
}
