use std::time::Duration;

use crossbeam_channel::Sender;

pub use crate::audio::audio_engine::{AudioCommand, PlaybackStatus};

/// Model-side view of audio playback state. Mutated by the reducer in
/// response to AudioEvents; the actual decode/output runs on a worker
/// thread (see `worker.rs` / `audio/mod.rs`) driven via `commands`.
pub struct AudioState {
    commands: Sender<AudioCommand>,
    pub status: PlaybackStatus,
    pub current_track: Option<String>,
    pub position: Duration,
    pub duration: Option<Duration>,
    pub volume: f32,
}

impl AudioState {
    pub fn new(commands: Sender<AudioCommand>) -> Self {
        Self {
            commands,
            status: PlaybackStatus::Stopped,
            current_track: None,
            position: Duration::ZERO,
            duration: None,
            volume: 1.0,
        }
    }

    pub fn play(&mut self) {
        if self.current_track.is_none() {
            return;
        }
        self.status = PlaybackStatus::Playing;
        let _ = self.commands.send(AudioCommand::Play);
    }

    pub fn pause(&mut self) {
        if self.status != PlaybackStatus::Playing {
            return;
        }
        self.status = PlaybackStatus::Paused;
        let _ = self.commands.send(AudioCommand::Pause);
    }

    pub fn stop(&mut self) {
        self.status = PlaybackStatus::Stopped;
        self.position = Duration::ZERO;
        let _ = self.commands.send(AudioCommand::Stop);
    }

    pub fn seek(&mut self, position_ms: u64) {
        let position = Duration::from_millis(position_ms);
        self.position = position;
        let _ = self.commands.send(AudioCommand::Seek(position));
    }

    pub fn set_volume(&mut self, level: f32) {
        self.volume = level.clamp(0.0, 1.0);
        let _ = self.commands.send(AudioCommand::SetVolume(self.volume));
    }

    pub fn load_track(&mut self, track_id: &str) {
        self.current_track = Some(track_id.to_string());
        self.position = Duration::ZERO;
        self.duration = None;
        self.status = PlaybackStatus::Stopped;
        let _ = self
            .commands
            .send(AudioCommand::LoadTrack(track_id.to_string()));
    }

    /// Called by the controller when the worker reports updated
    /// position/duration/status (separate from AppEvent dispatch,
    /// since this is high-frequency and shouldn't go through the bus).
    pub fn sync_from_engine(
        &mut self,
        position: Duration,
        duration: Option<Duration>,
        status: PlaybackStatus,
    ) {
        self.position = position;
        self.duration = duration;
        self.status = status;
    }
}
