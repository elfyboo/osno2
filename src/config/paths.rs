use directories::ProjectDirs;
use std::path::PathBuf;

pub struct AppPaths {
    pub config_dir: PathBuf,
    pub library_dir: PathBuf,
    pub playlists_dir: PathBuf,
}

impl AppPaths {
    pub fn resolve() -> Self {
        let config_dir = Self::resolve_config_dir();
        let library_dir = config_dir.join("library");
        let playlists_dir = config_dir.join("playlists");

        std::fs::create_dir_all(&config_dir).expect("Failed to create config directory");
        std::fs::create_dir_all(&library_dir).expect("Failed to create library directory");
        std::fs::create_dir_all(&playlists_dir).expect("Failed to create playlists directory");

        Self {
            config_dir,
            library_dir,
            playlists_dir,
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

    pub fn wezterm_config(&self) -> PathBuf {
        self.config_dir.join("wezterm.lua")
    }

    pub fn app_config(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    pub fn index_db(&self) -> PathBuf {
        self.library_dir.join("index.db")
    }

    pub fn tracks_toml(&self) -> PathBuf {
        self.library_dir.join("tracks.toml")
    }
}
