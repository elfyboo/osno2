// audio/audio_engine.rs

use std::time::Duration;

/// Commands sent from the model layer to the audio worker thread.
/// The worker owns the cpal stream and symphonia decoder; this enum
/// is the only channel through which playback state is mutated.
#[derive(Clone, Debug)]
pub enum AudioCommand {
    Play,
    Pause,
    Stop,
    Seek(Duration),
    SetVolume(f32),
    LoadTrack(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
}
