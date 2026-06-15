pub mod audio;
mod config;
pub mod core;
pub mod fs;
mod launcher;
pub mod library;
mod tty;
pub mod ui;
mod worker;

use clap::Parser;

#[derive(Parser)]
#[command(name = "osno2", about = "freeware terminal audio player")]
struct Cli {
    #[arg(long, hide = true)]
    worker: bool,

    /// Internal: becomes the pty's shell, bound to the worker's lifetime.
    #[arg(long, hide = true)]
    pty_helper: bool,

    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    shell_args: Vec<String>,
}

fn main() {
    let cli = Cli::parse();

    if cli.pty_helper {
        tty::pty_helper::run(cli.shell_args); // diverges
    }

    let result = if cli.worker {
        worker::run()
    } else {
        launcher::run()
    };

    if let Err(e) = result {
        eprintln!("osno2: {e}");
        std::process::exit(1);
    }
}

// #[derive(Parser)]
// #[command(name = "osno2", about = "freeware terminal audio player")]
// struct Cli {
//     /// Internal flag: run as TUI worker inside a WezTerm window
//     #[arg(long, hide = true)]
//     worker: bool,
// }

// fn main() {
//     let cli = Cli::parse();

//     let result = if cli.worker {
//         worker::run()
//     } else {
//         launcher::run()
//     };

//     if let Err(e) = result {
//         eprintln!("osno2: {e}");
//         std::process::exit(1);
//     }
// }
