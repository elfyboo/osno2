// model/state.rs

use std::collections::HashSet;
use std::path::PathBuf;

use crossbeam_channel::Receiver;

use crate::audio::audio_state::AudioState;
use crate::core::model::library::LibraryBackend;
use crate::core::model::library_track::LibraryTrack;
use crate::core::model::scrollback::ScrollbackBuffer;
use crate::core::model::store::Store;
use crate::core::model::view_state::{ActiveView, ViewState};
use crate::fs::fs_entry::FsEntry;

pub struct AppState {
    pub scrollback: ScrollbackBuffer,
    pub library: Box<dyn LibraryBackend>,
    pub library_cache: LibraryCache,
    pub store: Store,
    pub audio: AudioState,
    pub view_state: ViewState,

    // Library/playback data (was on App in ui/app.rs)
    pub now_playing: String,
    pub playing_track: usize,
    pub tracks: Vec<LibraryTrack>,
    pub track_receiver: Option<Receiver<Vec<LibraryTrack>>>,

    // Filesystem browser data
    pub working_dir: PathBuf,
    pub fs_entries: Vec<FsEntry>,

    // Shell/REPL output history (scrollback is for the pty terminal;
    // this is the sandboxed app's own command/response log)
    pub shell_history: Vec<String>,
}

impl AppState {
    pub fn new(library: Box<dyn LibraryBackend>, store: Store, audio: AudioState) -> Self {
        Self {
            scrollback: ScrollbackBuffer::new(),
            library,
            library_cache: LibraryCache::default(),
            store,
            audio,
            view_state: ViewState::new(),
            now_playing: "[No Track Playing]".into(),
            playing_track: 0,
            tracks: Vec::new(),
            track_receiver: None,
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            fs_entries: Vec::new(),
            shell_history: vec!["System operational. Type commands below.".into()],
        }
    }

    /// Drains the background library scanner channel for new tracks.
    /// Call once per tick before rendering.
    pub fn poll_scanner(&mut self) {
        if let Some(rx) = &self.track_receiver {
            let mut new_data = false;
            while let Ok(mut new_tracks) = rx.try_recv() {
                self.tracks.append(&mut new_tracks);
                new_data = true;
            }

            if new_data {
                self.tracks.sort_by(|a, b| {
                    a.artist
                        .cmp(&b.artist)
                        .then(a.album.cmp(&b.album))
                        .then(a.name.cmp(&b.name))
                });
            }
        }
    }

    pub fn start_library_scan(&mut self, directory: PathBuf) {
        let (tx, rx) = crossbeam_channel::unbounded();
        self.track_receiver = Some(rx);
        self.tracks.clear();
        crate::library::scanner::spawn_library_scan(directory, tx);
    }

    /// Resolves AppAction::Activate from ViewState into a concrete
    /// command/state update, since ViewState lacks access to tracks
    /// and fs_entries.
    pub fn resolve_activation(&mut self) -> crate::core::model::view_state::AppAction {
        use crate::core::model::view_state::AppAction;
        use crate::fs::fs_entry::FsEntryKind;

        match self.view_state.active_view {
            ActiveView::Tracklist => {
                if self.tracks.is_empty() {
                    return AppAction::None;
                }
                let track = &self.tracks[self.view_state.selected_track];
                self.playing_track = self.view_state.selected_track;
                self.now_playing = format!("{} - {}", track.artist, track.name);
                AppAction::ExecuteCommand(format!("play {}", track.id))
            }
            ActiveView::Filesystem => {
                if self.fs_entries.is_empty() {
                    return AppAction::None;
                }
                let entry = &self.fs_entries[self.view_state.fs_selected];
                match entry.kind {
                    FsEntryKind::Directory => {
                        AppAction::ExecuteCommand(format!("cd {}", entry.path.display()))
                    }
                    FsEntryKind::AudioFile => {
                        AppAction::ExecuteCommand(format!("play {}", entry.path.display()))
                    }
                    FsEntryKind::File => AppAction::None,
                }
            }
            _ => AppAction::None,
        }
    }

    pub fn position_str(&self) -> String {
        format_secs(self.audio.position.as_secs())
    }

    pub fn duration_str(&self) -> String {
        format_secs(self.audio.duration.map(|d| d.as_secs()).unwrap_or(0))
    }
}

fn format_secs(secs: u64) -> String {
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

#[derive(Default)]
pub struct LibraryCache {
    stale_tracks: HashSet<String>,
    stale_playlists: HashSet<String>,
}

impl LibraryCache {
    pub fn invalidate(&mut self, track_id: &str) {
        self.stale_tracks.insert(track_id.to_string());
    }

    pub fn remove(&mut self, track_id: &str) {
        self.stale_tracks.remove(track_id);
    }

    pub fn invalidate_playlist(&mut self, playlist_id: &str) {
        self.stale_playlists.insert(playlist_id.to_string());
    }

    pub fn is_track_stale(&self, track_id: &str) -> bool {
        self.stale_tracks.contains(track_id)
    }

    pub fn is_playlist_stale(&self, playlist_id: &str) -> bool {
        self.stale_playlists.contains(playlist_id)
    }
}
