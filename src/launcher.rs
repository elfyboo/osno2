use std::env;
use std::process::{Command, Stdio};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let current_exe = env::current_exe()?;
    let paths = crate::config::paths::AppPaths::resolve();
    seed_wezterm_config(&paths.wezterm_config())?;
    match which::which("wezterm") {
        Ok(wezterm_path) => spawn_wezterm(&wezterm_path, &current_exe),
        Err(_) => {
            eprintln!("⚠ WezTerm not found on PATH. Running in fallback terminal mode.");
            eprintln!("  Install WezTerm: https://wezfurlong.org/wezterm/installation.html");
            fallback_worker(&current_exe)
        }
    }
}

fn seed_wezterm_config(dest: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    if dest.exists() {
        return Ok(());
    }

    // Embedded at compile time -- no external file dependency at runtime
    let lua = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/wezterm.lua"));
    std::fs::write(dest, lua)?;
    Ok(())
}

fn spawn_wezterm(
    wezterm_path: &std::path::Path,
    current_exe: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = crate::config::paths::AppPaths::resolve()
        .config_dir
        .join("wezterm.lua");

    Command::new(wezterm_path)
        .arg("start")
        .arg("--no-auto-connect")
        .arg("--")
        .arg(current_exe)
        .arg("--worker")
        .env("WEZTERM_CONFIG_FILE", &config_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    Ok(())
}

fn fallback_worker(current_exe: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut child = Command::new(current_exe).arg("--worker").spawn()?;

    child.wait()?;
    Ok(())
}
