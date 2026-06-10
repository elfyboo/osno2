use crate::library::library_track::LibraryTrack;
use crossbeam_channel::Sender;
use jwalk::WalkDir;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

// Core imports
use symphonia_core::formats::FormatOptions;
use symphonia_core::formats::probe::{Hint, Probe};
use symphonia_core::io::MediaSourceStream;
use symphonia_core::meta::{MetadataOptions, StandardTag};

pub fn spawn_library_scan(dir: PathBuf, tx: Sender<Vec<LibraryTrack>>) {
    std::thread::spawn(move || {
        let mut batch = Vec::new();
        let probe = Probe::new();

        for entry in WalkDir::new(&dir)
            .sort(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() {
                if let Some(track) = parse_symphonia_to_library_track(&path, &probe) {
                    batch.push(track);
                    if batch.len() >= 50 {
                        if tx.send(batch.clone()).is_err() {
                            return;
                        }
                        batch.clear();
                    }
                }
            }
        }
        if !batch.is_empty() {
            let _ = tx.send(batch);
        }
    });
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

    let id = "abc".to_string();

    let mut name = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut artist = "Unknown Artist".to_string();
    let mut album = "Unknown Album".to_string();
    let mut year = None;

    if let Some(metadata) = format_reader.metadata().current() {
        for tag in metadata.media.tags.clone() {
            if let Some(std_tag) = tag.std {
                match std_tag {
                    StandardTag::TrackTitle(val) => name = val.to_string(),
                    StandardTag::Artist(val) => artist = val.to_string(),
                    StandardTag::Album(val) => album = val.to_string(),
                    StandardTag::RecordingYear(y) => year = Some(y),
                    StandardTag::OriginalReleaseYear(y) => year = Some(y),
                    _ => {}
                }
            }
        }
    }

    let sample_rate = None;
    let codec = None;

    if let Some(track) = format_reader.tracks().first() {
        if let Some(codec) = track.codec_params.clone() {
            if codec.is_audio() {
                if let Some(audio_params) = codec.audio() {
                    audio_params.sample_rate.unwrap_or(0);
                }
            }
        }
    }

    Some(LibraryTrack {
        id,
        path: path.to_path_buf(),
        name,
        artist,
        album,
        year,
        duration_secs: 0,
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
