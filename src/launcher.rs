use std::env;
use std::process::{Command, Stdio};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let current_exe = env::current_exe()?;

    match which::which("wezterm") {
        Ok(wezterm_path) => spawn_wezterm(&wezterm_path, &current_exe),
        Err(_) => {
            eprintln!("⚠ WezTerm not found on PATH. Running in fallback terminal mode.");
            eprintln!("  Install WezTerm: https://wezfurlong.org/wezterm/installation.html");
            fallback_worker(&current_exe)
        }
    }
}

fn spawn_wezterm(
    wezterm_path: &std::path::Path,
    current_exe: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    Command::new(wezterm_path)
        .arg("start")
        .arg("--no-auto-connect")
        .arg("--")
        .arg(current_exe)
        .arg("--worker")
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
