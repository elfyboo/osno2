// src/ui/app.rs
use crate::fs::fs_entry::FsEntry;
use crate::library::library_track::LibraryTrack;
use crate::spoofed::spoof_track_meta::TrackMeta;
use crate::ui::renderer::AppLayout;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;
use tui_slider::SliderState;

#[derive(Default, PartialEq, Debug, Clone, Copy)]
pub enum ActiveView {
    #[default]
    Tracklist,
    Filesystem,
    Visualizer,
    Settings,
    Help,
}

/// Actions that the UI loop delegates up to the orchestration worker
#[derive(Debug, Clone)]
pub enum AppAction {
    ExecuteCommand(String),
    ToggleTerminalFocus,
    None,
}

pub struct App {
    pub active_view: ActiveView,

    // Playback state
    pub now_playing: String,
    //pub volume: u8,
    pub position_secs: u64,
    pub duration_secs: u64,
    pub playing_track: usize,
    pub selected_track: usize,

    // Track list
    pub tracks: Vec<LibraryTrack>,

    // Metadata panel
    pub track_meta: TrackMeta,

    // Slider panel
    pub volume_state: SliderState,

    // Filesystem view [State only]
    pub working_dir: PathBuf,
    pub fs_entries: Vec<FsEntry>,
    pub fs_selected: usize,

    // Sandboxed Application REPL History
    pub shell_history: Vec<String>,
    pub shell_input: String,
}

impl App {
    pub fn new() -> Self {
        // Initialize with default state or empty vectors.
        // Your background engine will populate these asynchronously via workers later.
        let state = SliderState::new(50.0, 0.0, 100.0);
        Self {
            active_view: ActiveView::default(),
            now_playing: "[No Track Playing]".into(),
            volume_state: state,
            position_secs: 0,
            duration_secs: 0,
            playing_track: 0,
            selected_track: 0,
            tracks: Vec::new(),
            track_meta: TrackMeta::default(),
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            fs_entries: Vec::new(),
            fs_selected: 0,
            shell_history: vec!["System operational. Type commands below.".into()],
            shell_input: String::new(),
        }
    }

    pub fn draw(&mut self, frame: &mut ratatui::Frame, vt_screen: &tui_term::vt100::Screen) {
        let layout = AppLayout::new(frame.area());
        // Pass the vt_screen straight down to your AppLayout renderer layout module
        layout.render(frame, self, vt_screen);
    }

    /// Processes keyboard interactions and returns an optional action intent back up to the worker thread loop
    pub fn handle_key(&mut self, key: KeyEvent) -> AppAction {
        // Global App Interception: Alt+1 through Alt+5 changes active layout view tabs
        if key.modifiers.contains(KeyModifiers::ALT) {
            match key.code {
                KeyCode::Char('1') => self.active_view = ActiveView::Tracklist,
                KeyCode::Char('2') => self.active_view = ActiveView::Filesystem,
                KeyCode::Char('3') => self.active_view = ActiveView::Visualizer,
                KeyCode::Char('4') => self.active_view = ActiveView::Settings,
                KeyCode::Char('5') => self.active_view = ActiveView::Help,
                _ => {}
            }
            return AppAction::None;
        }

        // Global Appliance Interception: Ctrl+T signals to flip hardware focus boundaries
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('t') | KeyCode::Char('T'))
        {
            return AppAction::ToggleTerminalFocus;
        }

        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        match (key.code, shift) {
            (KeyCode::Up, _) => {
                self.handle_up();
                AppAction::None
            }
            (KeyCode::Down, _) => {
                self.handle_down();
                AppAction::None
            }
            (KeyCode::Tab, _) => {
                self.cycle_view();
                AppAction::None
            }
            (KeyCode::Enter, _) => self.handle_enter(),
            _ => {
                self.handle_shell_input(key);
                AppAction::None
            }
        }
    }

    fn handle_up(&mut self) {
        match self.active_view {
            ActiveView::Tracklist => self.selected_track = self.selected_track.saturating_sub(1),
            ActiveView::Filesystem => self.fs_selected = self.fs_selected.saturating_sub(1),
            _ => {}
        }
    }

    fn handle_down(&mut self) {
        match self.active_view {
            ActiveView::Tracklist => {
                if self.selected_track + 1 < self.tracks.len() {
                    self.selected_track += 1;
                }
            }
            ActiveView::Filesystem => {
                if self.fs_selected + 1 < self.fs_entries.len() {
                    self.fs_selected += 1;
                }
            }
            _ => {}
        }
    }

    fn cycle_view(&mut self) {
        self.active_view = match self.active_view {
            ActiveView::Tracklist => ActiveView::Filesystem,
            ActiveView::Filesystem => ActiveView::Visualizer,
            ActiveView::Visualizer => ActiveView::Settings,
            ActiveView::Settings => ActiveView::Help,
            ActiveView::Help => ActiveView::Tracklist,
        };
    }

    fn handle_enter(&mut self) -> AppAction {
        match self.active_view {
            ActiveView::Tracklist => {
                if !self.tracks.is_empty() {
                    let track = &self.tracks[self.selected_track];
                    AppAction::ExecuteCommand(format!("play {}", track.id))
                } else {
                    AppAction::None
                }
            }
            ActiveView::Filesystem => {
                if !self.fs_entries.is_empty() {
                    let entry = &self.fs_entries[self.fs_selected];
                    AppAction::ExecuteCommand(format!("cd {}", entry.path.display()))
                } else {
                    AppAction::None
                }
            }
            _ => {
                // If pressing enter on the sandboxed app REPL command bar
                let input = self.shell_input.trim().to_string();
                if !input.is_empty() {
                    self.shell_input.clear();
                    AppAction::ExecuteCommand(input)
                } else {
                    AppAction::None
                }
            }
        }
    }

    fn handle_shell_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Backspace => {
                self.shell_input.pop();
            }
            KeyCode::Char(c) => {
                self.shell_input.push(c);
            }
            _ => {}
        }
    }

    // Helper helpers for duration formatting
    pub fn position_str(&self) -> String {
        format_secs(self.position_secs)
    }
    pub fn duration_str(&self) -> String {
        format_secs(self.duration_secs)
    }
}

fn format_secs(secs: u64) -> String {
    format!("{:02}:{:02}", secs / 60, secs % 60)
}
