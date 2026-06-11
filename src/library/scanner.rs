// library/scanner.rs

use crate::core::model::library_track::LibraryTrack;
use crossbeam_channel::Sender;
use jwalk::WalkDir;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use symphonia_core::formats::FormatOptions;
use symphonia_core::formats::probe::{Hint, Probe};
use symphonia_core::io::MediaSourceStream;
use symphonia_core::meta::{MetadataOptions, StandardTag};

const BATCH_SIZE: usize = 50;

pub fn spawn_library_scan(dir: PathBuf, tx: Sender<Vec<LibraryTrack>>) {
    std::thread::spawn(move || {
        let mut batch = Vec::with_capacity(BATCH_SIZE);
        let probe = Probe::new();

        for entry in WalkDir::new(&dir)
            .sort(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            if let Some(track) = parse_symphonia_to_library_track(&path, &probe) {
                batch.push(track);
                if batch.len() >= BATCH_SIZE {
                    if tx.send(std::mem::take(&mut batch)).is_err() {
                        return;
                    }
                    batch.reserve(BATCH_SIZE);
                }
            }
        }

        if !batch.is_empty() {
            let _ = tx.send(batch);
        }
    });
}

/// Derives a stable track ID from the canonical file path. Stable
/// across rescans (unlike a random/sequential id), which is required
/// for LibraryCache invalidation and redb keying to work correctly.

fn track_id_for_path(path: &Path) -> String {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mut hasher = Sha256::new();
    hasher.update(canonical.to_string_lossy().as_bytes());
    let hash_bytes = hasher.finalize();

    // Convert the hash to a hex string and take the first 16 characters
    hash_bytes
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<String>()
        .chars()
        .take(16)
        .collect()
}

fn parse_symphonia_to_library_track(path: &Path, probe: &Probe) -> Option<LibraryTrack> {
    let file = fs::File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let mut format_reader = probe
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .ok()?;

    let mut name = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut artist = "Unknown Artist".to_string();
    let mut album = "Unknown Album".to_string();
    let mut year = None;

    if let Some(metadata) = format_reader.metadata().current() {
        for tag in &metadata.media.tags {
            let Some(std_tag) = &tag.std else {
                continue;
            };
            match std_tag {
                StandardTag::TrackTitle(val) => name = val.to_string(),
                StandardTag::Artist(val) => artist = val.to_string(),
                StandardTag::Album(val) => album = val.to_string(),
                StandardTag::RecordingYear(y) => year = Some(*y),
                StandardTag::OriginalReleaseYear(y) => year = Some(*y),
                _ => {}
            }
        }
    }

    let mut sample_rate = None;
    let mut codec = None;
    let mut duration_secs = 0;

    if let Some(track) = format_reader.tracks().first() {
        if let Some(params) = &track.codec_params {
            if let Some(audio_params) = params.audio() {
                sample_rate = audio_params.sample_rate;
                codec = Some(format!("{:?}", audio_params.codec));
                if let (Some(frames), Some(sample_rate)) = (track.num_frames, sample_rate) {
                    if sample_rate > 0 {
                        duration_secs = frames / sample_rate as u64;
                    }
                }
            }
        }
    }

    Some(LibraryTrack {
        id: track_id_for_path(path),
        path: path.to_path_buf(),
        name,
        artist,
        album,
        year,
        duration_secs,
        size_bytes: path.metadata().ok()?.len(),
        ext: path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .into(),
        bitrate: None,
        sample_rate,
        codec,
        rating: 0,
        added_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    })
}
