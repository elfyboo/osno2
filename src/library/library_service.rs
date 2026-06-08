use crate::library::fs_entry::FsEntry;
use crate::library::library_playlist::Playlist;
use crate::library::library_track::Track;
use std::path::{Path, PathBuf};

pub struct LibraryService {
    // redb database handle
    db: redb::Database,
    // path to meta/*.toml directory
    meta_dir: PathBuf,
    // path to playlists/*.toml directory
    playlists_dir: PathBuf,
}

use redb::{ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};

// Define the Tables.
// Key: Track ID (e.g., a hash of the path or a UUID string)
// Value: The serialized `Track` struct as a byte array
const TRACKS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("tracks");

// Example of an index table for fast search later:
// Key: Search term (lowercase), Value: Comma-separated Track IDs
const SEARCH_INDEX: TableDefinition<&str, &str> = TableDefinition::new("search_index");

impl LibraryService {
    pub fn new(db: redb::Database, meta_dir: PathBuf, playlists_dir: PathBuf) -> Self {
        Self {
            db,
            meta_dir,
            playlists_dir,
        }
    }

    fn extract_track_metadata(&self, path: &Path) -> Result<Track, std::io::Error> {
        let clean_path = dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let file = File::open(&clean_path)?;
        let file_metadata = file.metadata()?;

        // 1. Gather baseline filesystem facts
        let size_bytes = file_metadata.len();
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

        // Generate the SHA256 ID based on the canonical path string
        let id = format!(
            "{:x}",
            sha2::Sha256::digest(clean_path.to_string_lossy().as_bytes())
        );

        // 2. Setup Symphonia Media Source Stream
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        let mut hint = Hint::new();
        hint.with_extension(&ext);

        let format_opts = FormatOptions::default();
        let metadata_opts = MetadataOptions::default();

        // Probe the file format
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

        let mut format_reader = probed.format;

        // 3. Extract Audio Properties (Codec, Sample Rate, Bitrate, Duration)
        let mut duration_secs = 0;
        let mut sample_rate = None;
        let mut bitrate = None;
        let mut codec = None;

        // Inspect the primary audio track
        if let Some(track) = format_reader.default_track() {
            let params = &track.codec_params;
            sample_rate = params.sample_rate;
            bitrate = params.bits_per_sample.or_else(|| {
                if duration_secs > 0 {
                    // (Bytes * 8 bits) / seconds = bits per second
                    Some(((size_bytes * 8) / duration_secs) as u32)
                } else {
                    None
                }
            });
            // Map codec type to readable string
            codec = Some(format!("{:?}", params.codec).to_lowercase());

            // Calculate duration using TimeBase and number of frames
            if let (Some(n_frames), Some(tb)) = (params.n_frames, params.time_base) {
                let time = tb.calc_time(n_frames);
                duration_secs = time.seconds;
            }
        }

        // 4. Extract Text Tags (Artist, Album, Title, Year)
        let mut track_name = name.clone(); // Fallback to filename if no Title tag exists
        let mut artist = "Unknown Artist".to_string();
        let mut album = "Unknown Album".to_string();
        let mut year = None;

        // Symphonia tracks metadata in a metadata queue layer
        if let Some(mut metadata) = format_reader.metadata().current() {
            // Alternatively, inspect container-level metadata if present
            if let Some(revision) = metadata.tags() {
                for tag in revision {
                    if let Some(standard_key) = tag.std_key {
                        match standard_key {
                            StandardTagKey::TrackTitle => track_name = tag.value.to_string(),
                            StandardTagKey::Artist => artist = tag.value.to_string(),
                            StandardTagKey::Album => album = tag.value.to_string(),
                            StandardTagKey::Date => {
                                // Attempt to parse out a 4-digit year from date strings
                                if let Ok(parsed_year) =
                                    tag.value.to_string().get(0..4).unwrap_or("").parse::<u16>()
                                {
                                    year = Some(parsed_year);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        let added_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Ok(Track {
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
            codec,
            rating: 0, // Unrated by default
            added_at,
        })
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
        // 1. Process and build the Track using Symphonia parsing pipelines
        let track = self.extract_track_metadata(path)?;

        // 2. Serialize to raw TOML text representation
        let toml_string = toml::to_string(&track).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("TOML Serialization Failure: {e}"),
            )
        })?;

        // 3. Atomically insert right into redb tables
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
        todo!()
    }

    pub fn update_rating(&self, id: &str, rating: u8) -> Result<(), std::io::Error> {
        todo!()
    }

    pub fn reindex(&self) -> Result<(), std::io::Error> {
        todo!()
    }

    pub fn all_playlists(&self) -> Result<Vec<Playlist>, std::io::Error> {
        let mut playlists = Vec::new();

        if !self.playlists_dir.exists() {
            return Ok(playlists);
        }

        for entry in std::fs::read_dir(&self.playlists_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().unwrap_or_default() == "toml" {
                if let Ok(contents) = std::fs::read_to_string(&path) {
                    if let Ok(playlist) = toml::from_str::<Playlist>(&contents) {
                        playlists.push(playlist);
                    }
                }
            }
        }

        Ok(playlists)
    }

    pub fn playlist(&self, name: &str) -> Result<Option<Vec<Track>>, std::io::Error> {
        todo!()
    }

    pub fn create_playlist(&self, name: &str) -> Result<(), std::io::Error> {
        let safe_name = name.replace(|c: char| !c.is_alphanumeric(), "_");
        let file_path = self.playlists_dir.join(format!("{}.toml", safe_name));

        if file_path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "Playlist exists",
            ));
        }

        let playlist = Playlist {
            name: name.to_string(),
            track_ids: Vec::new(),
        };

        let toml_str = toml::to_string(&playlist).unwrap();
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

        // 1. Read existing
        let contents = std::fs::read_to_string(&file_path)?;
        let mut playlist: Playlist = toml::from_str(&contents)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // 2. Modify (Prevent duplicates)
        if !playlist.track_ids.contains(&track_id.to_string()) {
            playlist.track_ids.push(track_id.to_string());
        }

        // 3. Write back
        let toml_str = toml::to_string(&playlist).unwrap();
        std::fs::write(file_path, toml_str)?;

        Ok(())
    }

    pub fn remove_from_playlist(&self, playlist: &str, id: &str) -> Result<(), std::io::Error> {
        todo!()
    }
}
