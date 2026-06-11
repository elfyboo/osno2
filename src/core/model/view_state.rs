// model/view_state.rs

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_slider::SliderState;

//use crate::core::model::library_track::LibraryTrack;

#[derive(Default, PartialEq, Debug, Clone, Copy)]
pub enum ActiveView {
    #[default]
    Tracklist,
    Filesystem,
    Visualizer,
    Settings,
    Help,
}

/// Actions the view loop delegates up to the controller/worker.
#[derive(Debug, Clone)]
pub enum AppAction {
    ExecuteCommand(String),
    ToggleTerminalFocus,
    Activate,
    None,
}

/// Pure navigation/selection/input state. Contains no playback,
/// library, or filesystem data — those live in AppState's other
/// fields and are read directly by the renderer.
pub struct ViewState {
    pub active_view: ActiveView,
    pub selected_track: usize,
    pub fs_selected: usize,
    pub volume_state: SliderState,
    pub shell_input: String,
    pub terminal_width: u16,
    pub terminal_height: u16,
}

impl ViewState {
    pub fn new() -> Self {
        Self {
            active_view: ActiveView::default(),
            selected_track: 0,
            fs_selected: 0,
            volume_state: SliderState::new(50.0, 0.0, 100.0),
            shell_input: String::new(),
            terminal_width: 0,
            terminal_height: 0,
        }
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.terminal_width = width;
        self.terminal_height = height;
    }

    /// Processes keyboard input. `track_count` and `fs_entry_count`
    /// are passed in by the reducer (read from AppState's library/fs
    /// fields) since ViewState doesn't own that data.
    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        track_count: usize,
        fs_entry_count: usize,
    ) -> AppAction {
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

        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('t') | KeyCode::Char('T'))
        {
            return AppAction::ToggleTerminalFocus;
        }

        match key.code {
            KeyCode::Up => {
                self.handle_up(fs_entry_count);
                AppAction::None
            }
            KeyCode::Down => {
                self.handle_down(track_count, fs_entry_count);
                AppAction::None
            }
            KeyCode::Tab => {
                self.cycle_view();
                AppAction::None
            }
            KeyCode::Enter => self.handle_enter(),
            _ => {
                self.handle_shell_input(key);
                AppAction::None
            }
        }
    }

    fn handle_up(&mut self, _fs_entry_count: usize) {
        match self.active_view {
            ActiveView::Tracklist => self.selected_track = self.selected_track.saturating_sub(1),
            ActiveView::Filesystem => self.fs_selected = self.fs_selected.saturating_sub(1),
            _ => {}
        }
    }

    fn handle_down(&mut self, track_count: usize, fs_entry_count: usize) {
        match self.active_view {
            ActiveView::Tracklist => {
                if self.selected_track + 1 < track_count {
                    self.selected_track += 1;
                }
            }
            ActiveView::Filesystem => {
                if self.fs_selected + 1 < fs_entry_count {
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

    /// Tracklist/filesystem Enter actions need data ViewState doesn't
    /// own (track id, fs entry path/kind) — those branches return a
    /// marker AppAction that the controller resolves against
    /// AppState before dispatching. See AppAction::Activate.
    fn handle_enter(&mut self) -> AppAction {
        match self.active_view {
            ActiveView::Tracklist | ActiveView::Filesystem => AppAction::Activate,
            _ => {
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
}

impl Default for ViewState {
    fn default() -> Self {
        Self::new()
    }
}
