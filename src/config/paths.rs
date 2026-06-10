use directories::{ProjectDirs, UserDirs};
use std::path::PathBuf;

pub struct AppPaths {
    pub config_dir: PathBuf,
    pub _library_dir: PathBuf,
    pub _playlists_dir: PathBuf,
    pub _music_root: PathBuf,
}

impl AppPaths {
    pub fn resolve() -> Self {
        let config_dir = Self::resolve_config_dir();
        let _library_dir = config_dir.join("library");
        let _playlists_dir = config_dir.join("playlists");
        let _music_root = Self::resolve_music_root();

        std::fs::create_dir_all(&config_dir).expect("Failed to create config directory");
        std::fs::create_dir_all(&_library_dir).expect("Failed to create library directory");
        std::fs::create_dir_all(&_playlists_dir).expect("Failed to create playlists directory");

        Self {
            config_dir,
            _library_dir,
            _playlists_dir,
            _music_root,
        }
    }

    fn resolve_config_dir() -> PathBuf {
        if cfg!(debug_assertions) {
            // Sandboxed inside the repo during development
            std::env::current_dir()
                .expect("Failed to resolve working directory")
                .join("dev_env")
                .join("config")
        } else {
            // Production: platform-correct config directory
            // Windows: %APPDATA%\osno2
            // macOS:   ~/Library/Application Support/osno2
            // Linux:   ~/.config/osno2
            ProjectDirs::from("", "", "osno2")
                .expect("Failed to resolve platform config directory")
                .config_dir()
                .to_path_buf()
        }
    }

    fn resolve_music_root() -> PathBuf {
        UserDirs::new()
            .and_then(|dirs| dirs.audio_dir().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."))
    }

    pub fn wezterm_config(&self) -> PathBuf {
        self.config_dir.join("wezterm.lua")
    }

    pub fn _app_config(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    pub fn _index_db(&self) -> PathBuf {
        self._library_dir.join("index.db")
    }

    pub fn _tracks_toml(&self) -> PathBuf {
        self._library_dir.join("tracks.toml")
    }

    pub fn _music_root(&self) -> &PathBuf {
        &self._music_root
    }
}
