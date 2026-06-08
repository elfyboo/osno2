mod config;
mod launcher;
pub mod library;
pub mod ui;
mod worker;

use clap::Parser;

#[derive(Parser)]
#[command(name = "osno2", about = "freeware terminal audio player")]
struct Cli {
    /// Internal flag: run as TUI worker inside a WezTerm window
    #[arg(long, hide = true)]
    worker: bool,
}

fn main() {
    let cli = Cli::parse();

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
