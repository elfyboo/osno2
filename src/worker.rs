// src/worker.rs
use ansi_control_codes::c0::{CR, ESC, HT};
use ansi_control_codes::control_sequences::{CNL, CPL, CUB, CUD, CUF, CUU};
use std::sync::LazyLock;

// ANSI escape sequences for terminal control characters
static SEQ_BACKSPACE: LazyLock<Vec<u8>> = LazyLock::new(|| vec![0x7F]);
static SEQ_ENTER: LazyLock<Vec<u8>> = LazyLock::new(|| CR.to_string().into_bytes());
static SEQ_TAB: LazyLock<Vec<u8>> = LazyLock::new(|| HT.to_string().into_bytes());
static SEQ_ESC: LazyLock<Vec<u8>> = LazyLock::new(|| ESC.to_string().into_bytes());
static SEQ_UP: LazyLock<Vec<u8>> = LazyLock::new(|| CUU(1.into()).to_string().into_bytes());
static SEQ_DOWN: LazyLock<Vec<u8>> = LazyLock::new(|| CUD(1.into()).to_string().into_bytes());
static SEQ_RIGHT: LazyLock<Vec<u8>> = LazyLock::new(|| CUF(1.into()).to_string().into_bytes());
static SEQ_LEFT: LazyLock<Vec<u8>> = LazyLock::new(|| CUB(1.into()).to_string().into_bytes());
static SEQ_HOME: LazyLock<Vec<u8>> = LazyLock::new(|| CPL(1.into()).to_string().into_bytes());
static SEQ_END: LazyLock<Vec<u8>> = LazyLock::new(|| CNL(1.into()).to_string().into_bytes());

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
use vt100::Parser;

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
    let pty_pair = pty_system.openpty(PtySize {
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

    // Initialize the vt100 Parser state machine.
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
                // todo, persist scrollback history on resize
                // let scrollback: Vec<u8> = {
                //     let guard = vt_parser.read();
                //     let screen = guard.screen();
                //     // Reconstruct visible content as bytes to replay into new parser
                //     (0..screen.rows())
                //         .flat_map(|row| {
                //             let mut line = screen.row_contents(row);
                //             line.push(b'\n');
                //             line
                //         })
                //         .collect()
                // };
                let mut new_parser = Parser::new(rows, cols, 1000);
                // new_parser.process(&scrollback);
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
                        // Global appliance override interception inside terminal view context
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && matches!(key.code, KeyCode::Char('t') | KeyCode::Char('T'))
                        {
                            current_focus = InputFocus::App;
                            terminal.draw(|frame| app.draw(frame, vt_parser.read().screen()))?;
                        } else {
                            // UPDATE: Passes Crossterm keys cleanly into our explicit translation function
                            handle_terminal_key(key, &mut pty_writer);
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

fn handle_terminal_key(key: KeyEvent, pty_writer: &mut Box<dyn std::io::Write + Send>) {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char(c) = key.code {
            let byte = c.to_ascii_uppercase() as u8 - b'A' + 1;
            let _ = pty_writer.write_all(&[byte]);
            let _ = pty_writer.flush();
            return;
        }
    }

    let seq: Option<&[u8]> = match key.code {
        KeyCode::Char(c) => {
            let mut buf = [0u8; 4];
            let s = c.encode_utf8(&mut buf);
            let _ = pty_writer.write_all(s.as_bytes());
            let _ = pty_writer.flush();
            return;
        }
        KeyCode::Enter => Some(&SEQ_ENTER),
        KeyCode::Tab => Some(&SEQ_TAB),
        KeyCode::Backspace => Some(&SEQ_BACKSPACE),
        KeyCode::Esc => Some(&SEQ_ESC),
        KeyCode::Up => Some(&SEQ_UP),
        KeyCode::Down => Some(&SEQ_DOWN),
        KeyCode::Right => Some(&SEQ_RIGHT),
        KeyCode::Left => Some(&SEQ_LEFT),
        KeyCode::Home => Some(&SEQ_HOME),
        KeyCode::End => Some(&SEQ_END),
        _ => None,
    };

    if let Some(bytes) = seq {
        let _ = pty_writer.write_all(bytes);
        let _ = pty_writer.flush();
    }
}

fn should_quit(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C'))
}
