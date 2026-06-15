use std::env;
use std::process::{Command, Stdio};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let current_exe = env::current_exe()?;
    let paths = crate::config::paths::AppPaths::resolve();
    seed_wezterm_config(&paths.wezterm_config())?;

    match which::which("wezterm") {
        Ok(wezterm_path) => {
            #[cfg(target_os = "windows")]
            {
                let config_path = paths.config_dir.join("wezterm.lua");

                // 1. Build the exact argument array WezTerm needs
                let args: &[&std::ffi::OsStr] = &[
                    std::ffi::OsStr::new("start"),
                    std::ffi::OsStr::new("--no-auto-connect"),
                    std::ffi::OsStr::new("--"),
                    current_exe.as_os_str(),
                    std::ffi::OsStr::new("--worker"),
                ];

                // 2. Set the environment variable explicitly since Win32's CreateProcessW
                // inherits the current environment block by default.
                unsafe {
                    std::env::set_var("WEZTERM_CONFIG_FILE", &config_path);
                }

                spawn_supervised(&wezterm_path, args)?;
            }

            #[cfg(not(target_os = "windows"))]
            {
                // Fall back to standard spawning for Linux/macOS
                spawn_wezterm(&wezterm_path, &current_exe)?;
            }
        }
        Err(_) => {
            eprintln!("⚠ WezTerm not found on PATH. Running in fallback terminal mode.");
            eprintln!("  Install WezTerm: https://wezfurlong.org/wezterm/installation.html");
            fallback_worker(&current_exe)?
        }
    }
    Ok(())
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

#[cfg(target_os = "windows")]
fn spawn_supervised(
    program: &std::path::Path,
    args: &[&std::ffi::OsStr],
) -> windows::core::Result<()> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
        SetInformationJobObject,
    };
    use windows::Win32::System::Threading::{
        CREATE_SUSPENDED, CreateProcessW, INFINITE, PROCESS_INFORMATION, ResumeThread,
        STARTUPINFOW, WaitForSingleObject,
    };
    use windows::core::PWSTR;

    let mut cmdline: Vec<u16> = build_command_line(program, args)
        .encode_utf16()
        .chain(Some(0))
        .collect();

    let startup_info = STARTUPINFOW {
        cb: std::mem::size_of::<STARTUPINFOW>() as u32,
        ..Default::default()
    };
    let mut process_info = PROCESS_INFORMATION::default();

    unsafe {
        CreateProcessW(
            None,
            Some(PWSTR(cmdline.as_mut_ptr())),
            None,
            None,
            false,
            CREATE_SUSPENDED,
            None,
            None,
            &startup_info,
            &mut process_info,
        )?;

        let job = CreateJobObjectW(None, None)?;
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const _,
            std::mem::size_of_val(&info) as u32,
        )?;

        // Suspended process can't spawn anything yet -- guaranteed to join
        // the job before it, or any descendant, runs.
        AssignProcessToJobObject(job, process_info.hProcess)?;
        ResumeThread(process_info.hThread);
        CloseHandle(process_info.hThread);

        // Block until wezterm-gui exits, however it exits.
        WaitForSingleObject(process_info.hProcess, INFINITE);
        CloseHandle(process_info.hProcess);

        // Last handle to the job closes here -- kills any survivors
        // (osno2-worker, OpenConsole.exe, the shell) in one shot.
        CloseHandle(job);
    }

    Ok(())
}

fn build_command_line(program: &std::path::Path, args: &[&std::ffi::OsStr]) -> String {
    fn quote(s: &std::ffi::OsStr) -> String {
        let s = s.to_string_lossy();
        if s.is_empty() || s.contains(' ') || s.contains('"') {
            format!("\"{}\"", s.replace('"', "\\\""))
        } else {
            s.into_owned()
        }
    }

    let mut out = quote(program.as_os_str());
    for arg in args {
        out.push(' ');
        out.push_str(&quote(arg));
    }
    out
}
