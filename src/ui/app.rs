use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;

use crate::ui::layout::AppLayout;

#[derive(Default, PartialEq)]
pub enum ActiveView {
    #[default]
    Tracklist,
    Filesystem,
    Visualizer,
    Settings,
    Help,
}

pub struct Track {
    pub length: String,
    pub name: String,
    pub year: String,
    pub artist: String,
    pub ext: String,
}

pub struct TrackMeta {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub year: String,
    pub track_num: String,
    pub genre: String,
    pub time: String,
    pub size: String,
    pub rating: usize,
}

pub struct App {
    pub active_view: ActiveView,

    // Playback state
    pub now_playing: String,
    pub volume: u8,
    pub position_secs: u64,
    pub duration_secs: u64,
    pub playing_track: usize,
    pub selected_track: usize,

    // Track list
    pub tracks: Vec<Track>,

    // Metadata panel
    pub track_meta: TrackMeta,

    // Filesystem view
    pub working_dir: PathBuf,
    pub fs_entries: Vec<String>,
    pub fs_selected: usize,

    // Shell
    pub shell_history: Vec<String>,
    pub shell_input: String,
}

impl App {
    pub fn new() -> Self {
        let tracks = vec![
            Track {
                length: "02:20".into(),
                name: "Maintune.Mod".into(),
                year: "1998".into(),
                artist: "s0ren gessele".into(),
                ext: "mod".into(),
            },
            Track {
                length: "02:37".into(),
                name: "Demoseq7".into(),
                year: "1998".into(),
                artist: "s0ren gessele".into(),
                ext: "mod".into(),
            },
            Track {
                length: "03:14".into(),
                name: "Frontline".into(),
                year: "1999".into(),
                artist: "s0ren gessele".into(),
                ext: "mod".into(),
            },
        ];

        let working_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));

        let fs_entries = vec![
            "../".into(),
            "front6/".into(),
            "demos/".into(),
            "Maintune.mod".into(),
            "Demoseq7.mod".into(),
        ];

        Self {
            active_view: ActiveView::default(),

            now_playing: "Demoseq7 (1998)".into(),
            volume: 95,
            position_secs: 83,  // 01:23
            duration_secs: 157, // 02:37
            playing_track: 1,
            selected_track: 1,

            tracks,

            track_meta: TrackMeta {
                title: "Demoseq7".into(),
                artist: "s0ren gessele".into(),
                album: "front6".into(),
                year: "1998".into(),
                track_num: "1".into(),
                genre: "Mod/Tracker".into(),
                time: "02:37".into(),
                size: "86.00 kb".into(),
                rating: 2,
            },

            working_dir,
            fs_entries,
            fs_selected: 0,

            shell_history: vec![
                "~/Music/Others/ cd ./front6".into(),
                "~/Music/Others/front6/ search \"demoseq7\"".into(),
                "~/Music/Others/front6/ play".into(),
            ],
            shell_input: String::new(),
        }
    }

    pub fn draw(&mut self, frame: &mut ratatui::Frame) {
        let layout = AppLayout::new(frame.area());
        layout.render(frame, self);
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        // View switching: Alt+1 through Alt+5
        if key.modifiers.contains(KeyModifiers::ALT) {
            match key.code {
                KeyCode::Char('1') => {
                    self.active_view = ActiveView::Tracklist;
                    return;
                }
                KeyCode::Char('2') => {
                    self.active_view = ActiveView::Filesystem;
                    return;
                }
                KeyCode::Char('3') => {
                    self.active_view = ActiveView::Visualizer;
                    return;
                }
                KeyCode::Char('4') => {
                    self.active_view = ActiveView::Settings;
                    return;
                }
                KeyCode::Char('5') => {
                    self.active_view = ActiveView::Help;
                    return;
                }
                _ => {}
            }
        }

        // Arrow key navigation -- view-specific
        match key.code {
            KeyCode::Up => self.handle_up(),
            KeyCode::Down => self.handle_down(),
            KeyCode::Enter => self.handle_enter(),
            _ => self.handle_shell_input(key),
        }
    }

    fn handle_up(&mut self) {
        match self.active_view {
            ActiveView::Tracklist => {
                self.selected_track = self.selected_track.saturating_sub(1);
            }
            ActiveView::Filesystem => {
                self.fs_selected = self.fs_selected.saturating_sub(1);
            }
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

    fn handle_enter(&mut self) {
        match self.active_view {
            ActiveView::Tracklist => {
                // Emit play command through the command processor
                let name = self.tracks[self.selected_track].name.clone();
                self.execute_command(&format!("play {name}"));
            }
            ActiveView::Filesystem => {
                // Emit cd command -- fs view drives via command processor only
                let entry = self.fs_entries[self.fs_selected].clone();
                self.execute_command(&format!("cd {entry}"));
            }
            _ => {}
        }
    }

    fn handle_shell_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                let input = self.shell_input.trim().to_string();
                if !input.is_empty() {
                    self.execute_command(&input);
                }
            }
            KeyCode::Backspace => {
                self.shell_input.pop();
            }
            KeyCode::Char(c) => {
                self.shell_input.push(c);
            }
            _ => {}
        }
    }

    // Single command throughput. All UI actions and shell input route here.
    pub fn execute_command(&mut self, cmd: &str) {
        let entry = format!("{}/ {}", self.working_dir.display(), cmd);
        self.shell_history.push(entry);
        self.shell_input.clear();

        let parts: Vec<&str> = cmd.trim().splitn(2, ' ').collect();
        match parts.as_slice() {
            ["cd", path] => self.cmd_cd(path),
            ["ls"] => self.cmd_ls(),
            ["play", query] => self.cmd_play(query),
            ["volume", level] => self.cmd_volume(level),
            ["view", target] => self.cmd_view(target),
            _ => {
                self.shell_history.push(format!("  unknown command: {cmd}"));
            }
        }
    }

    fn cmd_cd(&mut self, path: &str) {
        let target = if path.starts_with('/') {
            PathBuf::from(path)
        } else {
            self.working_dir.join(path)
        };

        match std::fs::canonicalize(&target) {
            Ok(resolved) => {
                self.working_dir = resolved;
                self.cmd_ls();
                if self.active_view == ActiveView::Filesystem {
                    self.fs_selected = 0;
                }
            }
            Err(_) => {
                self.shell_history
                    .push(format!("  cd: no such directory: {path}"));
            }
        }
    }

    fn cmd_ls(&mut self) {
        match std::fs::read_dir(&self.working_dir) {
            Ok(entries) => {
                let mut listing: Vec<String> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| {
                        let name = e.file_name().to_string_lossy().to_string();
                        if e.path().is_dir() {
                            format!("{name}/")
                        } else {
                            name
                        }
                    })
                    .collect();

                listing.sort();

                // Prepend parent dir entry
                listing.insert(0, "../".into());

                self.fs_entries = listing.clone();
                for entry in &listing {
                    self.shell_history.push(format!("  {entry}"));
                }
            }
            Err(e) => {
                self.shell_history.push(format!("  ls: {e}"));
            }
        }
    }

    fn cmd_play(&mut self, query: &str) {
        // TODO: resolve against track index and hand off to audio engine
        self.now_playing = query.to_string();
        self.shell_history.push(format!("  playing: {query}"));
    }

    fn cmd_volume(&mut self, level: &str) {
        match level.parse::<u8>() {
            Ok(v) if v <= 100 => {
                self.volume = v;
                self.shell_history.push(format!("  volume set to {v}%"));
            }
            _ => {
                self.shell_history.push("  volume: expected 0-100".into());
            }
        }
    }

    fn cmd_view(&mut self, target: &str) {
        match target {
            "tracklist" | "1" => self.active_view = ActiveView::Tracklist,
            "filesystem" | "2" => self.active_view = ActiveView::Filesystem,
            "visualizer" | "3" => self.active_view = ActiveView::Visualizer,
            "settings" | "4" => self.active_view = ActiveView::Settings,
            "help" | "5" => self.active_view = ActiveView::Help,
            _ => {
                self.shell_history
                    .push(format!("  view: unknown target: {target}"));
            }
        }
    }

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
