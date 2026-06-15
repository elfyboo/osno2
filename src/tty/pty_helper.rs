//! Re-exec target spawned inside the pty in place of the user's shell.
//! Binds the shell's lifetime to the worker process so it can never be
//! orphaned, using the strongest primitive each platform offers. Not
//! used on Windows -- the worker's Job Object already covers this case.

#[cfg(target_os = "linux")]
pub fn run(shell_args: Vec<String>) -> ! {
    use std::os::unix::process::CommandExt;

    // Kernel delivers SIGKILL the instant our parent (the worker) dies,
    // for any reason -- crash, SIGKILL, panic. Preserved across execve
    // for non-setuid binaries.
    unsafe {
        libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL);
    }

    let (program, args) = shell_args
        .split_first()
        .expect("pty-helper requires a command");
    let err = std::process::Command::new(program).args(args).exec();
    panic!("exec failed: {err}");
}

#[cfg(target_os = "macos")]
pub fn run(shell_args: Vec<String>) -> ! {
    use std::time::Duration;

    let (program, args) = shell_args
        .split_first()
        .expect("pty-helper requires a command");

    let mut child = std::process::Command::new(program)
        .args(args)
        .spawn()
        .expect("failed to spawn shell");

    let child_pid = child.id() as libc::pid_t;
    let worker_pid = unsafe { libc::getppid() };

    // No PR_SET_PDEATHSIG on macOS; poll for the worker's death and take
    // the shell down with us.
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_millis(200));
            if unsafe { libc::getppid() } != worker_pid {
                unsafe { libc::kill(child_pid, libc::SIGKILL) };
                std::process::exit(1);
            }
        }
    });

    let status = child.wait().expect("failed to wait on shell");
    std::process::exit(status.code().unwrap_or(1));
}

#[cfg(target_os = "windows")]
pub fn run(shell_args: Vec<String>) -> ! {
    // Unreachable in normal operation -- the worker's Job Object handles
    // this platform. Kept only so the binary builds uniformly.
    let (program, args) = shell_args
        .split_first()
        .expect("pty-helper requires a command");
    let status = std::process::Command::new(program)
        .args(args)
        .status()
        .expect("failed to run shell");
    std::process::exit(status.code().unwrap_or(1));
}
