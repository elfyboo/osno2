use clap::{Parser, Subcommand, ValueEnum};

/// Represents the looping behavior for audio playback
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopMode {
    /// No looping
    Off,
    /// Loop the current track
    Track,
    /// Loop the entire queue
    Queue,
}

/// The root command structure for the osno2 internal prompt
#[derive(Parser, Debug)]
#[command(name = "", bin_name = "/", disable_help_flag = true)]
pub struct OsnoPrompt {
    #[command(subcommand)]
    pub command: Command,
}

/// Available commands for the osno2 prompt
#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// Play a track directly from the library
    Play {
        /// The track name, artist, or album keywords to fuzzy search
        #[arg(trailing_var_arg = true, required = true)]
        query: Vec<String>,
    },
    /// Load a specific playlist
    Playlist {
        /// The exact name or path of the playlist TOML file
        name: String,
    },
    /// Add a song or a playlist to the active playback queue
    Add {
        /// The track keywords or the playlist path to append
        #[arg(trailing_var_arg = true, required = true)]
        target: Vec<String>,
    },
    /// Remove an item from the active playback queue
    Remove {
        /// The 0-indexed position of the track in the queue
        index: usize,
    },
    /// Set the master volume level
    Volume {
        /// Volume integer from 0 (mute) to 100 (max)
        #[arg(value_parser = clap::value_parser!(u8).range(0..=100))]
        level: u8,
    },
    /// Configure crossfade timing between tracks
    Crossfade {
        /// Duration in seconds for overlapping track transitions
        seconds: u32,
    },
    /// Change the audio playback repetition constraints
    Loop {
        /// The looping target behavior
        #[arg(value_enum)]
        mode: LoopMode,
    },
    /// Wipe broken external track pointers from the local library directory
    Purge,
}

impl OsnoPrompt {
    /// Parses an interactive terminal string like "/play hyperlight laura"
    pub fn parse_line(input: &str) -> Result<Self, clap::Error> {
        if !input.starts_with('/') {
            return Err(clap::Error::new(clap::error::ErrorKind::InvalidSubcommand));
        }

        let clean_input = input.trim_start_matches('/');
        let mut args = vec!["/"];
        args.extend(clean_input.split_whitespace());

        Self::try_parse_from(args)
    }
}
