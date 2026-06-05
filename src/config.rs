use std::path::PathBuf;
use directories::BaseDirs; // Swapped to BaseDirs for direct home directory targeting

pub struct AppPaths {
    pub config_dir: PathBuf,
    pub library_dir: PathBuf,
}

impl AppPaths {
    pub fn resolve() -> Self {
        if cfg!(debug_assertions) {
            // --- DEVELOPMENT FLOW ---
            // Everything stays sandboxed right inside project directory
            let repo_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            Self {
                config_dir: repo_root.join("dev_env/config"),
                library_dir: repo_root.join("dev_env/library"),
            }
        } else {
            // --- PRODUCTION FLOW ---
            // Resolves universally to the user folder:
            // Windows: C:\Users\Username\.osno2
            // Linux/macOS: /home/username/.osno2 or /Users/username/.osno2
            let base_dirs = BaseDirs::new()
                .expect("Failed to resolve user home directory paths");

            let config_dir = base_dirs.home_dir().join(".osno2");
            let library_dir = config_dir.join("library");

            Self { config_dir, library_dir }
        }
    }
}
