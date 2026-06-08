// src/worker.rs
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use parking_lot::RwLock;
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use ratatui::prelude::*;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;

// Pull the exact 0.2.0 components out of tui-term
use tui_term::vt100::Parser;

use crate::library::library_service::LibraryService;
use crate::ui::app::{ActiveView, App, AppAction};

/// The conceptual focus boundaries for our input dispatch matrix
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputFocus {
    App,
    Terminal,
}

/// Unified event tracking enum passing up from background threads to the main UI thread
pub enum AppEvent {
    KeyEvent(KeyEvent),
    TerminalOutput,
    Resize(u16, u16),
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    let _guard = rt.enter();
    let mut terminal = setup_terminal()?;

    let app_paths = crate::config::paths::AppPaths::resolve();
    let db = redb::Database::create(app_paths.config_dir.join("library/index.db"))?;
    let library_service = LibraryService::new(
        db,
        app_paths.config_dir.join("library"),
        app_paths.config_dir.join("playlists"),
    );

    let result = event_loop(&mut terminal, library_service, rt.handle().clone());
    teardown_terminal(&mut terminal)?;
    result
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>, io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn teardown_terminal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), io::Error> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    library_service: LibraryService,
    tokio_handle: tokio::runtime::Handle,
) -> Result<(), Box<dyn std::error::Error>> {
    let (event_tx, event_rx) = crossbeam_channel::unbounded::<AppEvent>();

    let pty_system = NativePtySystem::default();
    let size = terminal.size()?;
    let pty_pair = pty_system.open_pty(PtySize {
        rows: size.height,
        cols: size.width,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    #[cfg(target_os = "windows")]
    let cmd = CommandBuilder::new("powershell.exe");
    #[cfg(not(target_os = "windows"))]
    let cmd = CommandBuilder::with_argv(vec![
        std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string()),
        "-i".to_string(),
    ]);

    let _child = pty_pair.slave.spawn_command(cmd)?;
    let mut pty_writer = pty_pair.master.take_writer()?;
    let pty_reader = pty_pair.master.try_clone_reader()?;

    // 0.2.0 FIX: Initialize the vt100 Parser state machine.
    // Arguments: (rows, cols, scrollback_lines) -> 1000 lines of scrollback memory
    let vt_parser = Arc::new(RwLock::new(Parser::new(size.height, size.width, 1000)));

    // Background Worker Thread: Pipe stdout directly into the vt100 parser
    let vt_writer_clone = Arc::clone(&vt_parser);
    let tx_terminal = event_tx.clone();
    std::thread::spawn(move || {
        let mut reader = std::io::BufReader::new(pty_reader);
        let mut buffer = [0u8; 8192];
        while let Ok(n) = std::io::Read::read(&mut reader, &mut buffer) {
            if n == 0 {
                break;
            }

            // vt100::Parser implements std::io::Write
            let _ = vt_writer_clone.write().write_all(&buffer[..n]);
            let _ = tx_terminal.send(AppEvent::TerminalOutput);
        }
    });

    let tx_crossterm = event_tx.clone();
    std::thread::spawn(move || {
        loop {
            if let Ok(true) = event::poll(std::time::Duration::from_millis(200)) {
                match event::read() {
                    Ok(Event::Key(key)) => {
                        if key.kind == KeyEventKind::Press {
                            let _ = tx_crossterm.send(AppEvent::KeyEvent(key));
                        }
                    }
                    Ok(Event::Resize(cols, rows)) => {
                        let _ = tx_crossterm.send(AppEvent::Resize(cols, rows));
                    }
                    _ => {}
                }
            }
        }
    });

    let mut app = App::new();
    let mut current_focus = InputFocus::App;

    if let Ok(entries) = library_service.read_dir(&app.working_dir) {
        app.fs_entries = entries;
    }

    // 0.2.0 FIX: Pass the parsed .screen() down to the UI drawing layer
    terminal.draw(|frame| app.draw(frame, vt_parser.read().screen()))?;

    for event in event_rx {
        match event {
            AppEvent::TerminalOutput => {
                if event_rx.is_empty() {
                    terminal.draw(|frame| app.draw(frame, vt_parser.read().screen()))?;
                }
            }
            AppEvent::Resize(cols, rows) => {
                let _ = pty_pair.master.resize(PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                });
                // 0.2.0 FIX: vt100::Parser uses .set_size() for re-allocations
                vt_parser.write().set_size(rows, cols);
                terminal.draw(|frame| app.draw(frame, vt_parser.read().screen()))?;
            }
            AppEvent::KeyEvent(key) => {
                if should_quit(key) {
                    return Ok(());
                }

                match current_focus {
                    InputFocus::App => match app.handle_key(key) {
                        AppAction::ToggleTerminalFocus => {
                            current_focus = InputFocus::Terminal;
                            terminal.draw(|frame| app.draw(frame, vt_parser.read().screen()))?;
                        }
                        AppAction::ExecuteCommand(cmd_str) => {
                            execute_app_command(
                                &mut app,
                                &cmd_str,
                                &library_service,
                                &tokio_handle,
                            );
                            terminal.draw(|frame| app.draw(frame, vt_parser.read().screen()))?;
                        }
                        AppAction::None => {
                            terminal.draw(|frame| app.draw(frame, vt_parser.read().screen()))?;
                        }
                    },
                    InputFocus::Terminal => {
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && matches!(key.code, KeyCode::Char('t') | KeyCode::Char('T'))
                        {
                            current_focus = InputFocus::App;
                            terminal.draw(|frame| app.draw(frame, vt_parser.read().screen()))?;
                        } else {
                            if let Some(bytes) = tui_term::types::Input::from(key).to_bytes() {
                                let _ = pty_writer.write_all(bytes);
                                let _ = pty_writer.flush();
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn execute_app_command(
    app: &mut App,
    cmd: &str,
    library: &LibraryService,
    _rt: &tokio::runtime::Handle,
) {
    let history_line = format!("{}/ {}", app.working_dir.display(), cmd);
    app.shell_history.push(history_line);

    let parts: Vec<&str> = cmd.trim().splitn(2, ' ').collect();
    match parts.as_slice() {
        ["cd", target_path] => {
            let raw_path = PathBuf::from(target_path);
            let target = if raw_path.starts_with("/") {
                raw_path
            } else {
                app.working_dir.join(raw_path)
            };
            match library.read_dir(&target) {
                Ok(new_entries) => {
                    app.working_dir = dunce::canonicalize(&target).unwrap_or(target);
                    app.fs_entries = new_entries;
                    if app.active_view == ActiveView::Filesystem {
                        app.fs_selected = 0;
                    }
                }
                Err(e) => app.shell_history.push(format!("  cd error: {e}")),
            }
        }
        ["ls"] => {
            if let Ok(entries) = library.read_dir(&app.working_dir) {
                app.fs_entries = entries;
            }
        }
        ["play", query] => {
            app.now_playing = query.to_string();
        }
        ["volume", level] => {
            if let Ok(v) = level.parse::<u8>() {
                if v <= 100 {
                    app.volume = v;
                }
            }
        }
        ["view", target] => match *target {
            "tracklist" | "1" => app.active_view = ActiveView::Tracklist,
            "filesystem" | "2" => app.active_view = ActiveView::Filesystem,
            "visualizer" | "3" => app.active_view = ActiveView::Visualizer,
            "settings" | "4" => app.active_view = ActiveView::Settings,
            "help" | "5" => app.active_view = ActiveView::Help,
            _ => {}
        },
        _ => {}
    }
}

// fn event_loop(
//     terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
// ) -> Result<(), Box<dyn std::error::Error>> {
//     let mut app = App::new();

//     loop {
//         terminal.draw(|frame| app.draw(frame))?;

//         // Drain all pending events before the next frame to avoid input lag
//         // accumulating across slow renders. Filter to Press only -- crossterm
//         // emits Repeat and Release on some terminals, causing doubled input.
//         while crossterm::event::poll(std::time::Duration::from_millis(0))? {
//             if let Event::Key(key) = event::read()? {
//                 if key.kind != KeyEventKind::Press {
//                     continue;
//                 }
//                 if should_quit(key) {
//                     return Ok(());
//                 }
//                 app.handle_key(key);
//             }
//         }

//         std::thread::sleep(std::time::Duration::from_millis(16));
//     }
// }

fn should_quit(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C'))
}
