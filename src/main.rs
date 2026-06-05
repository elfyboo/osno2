mod prompt;

use std::io;
use std::process::Command as SysCommand;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};

// Pull in the isolated prompt parsing layer
use prompt::{Command, LoopMode, OsnoPrompt};

// =============================================================================
// 1. Application Runtime State
// =============================================================================

struct AppState {
    input_buffer: String,
    logs: Vec<String>,
    current_track: Option<String>,
    volume: u8,
    loop_mode: LoopMode,
    should_quit: bool,
}

impl AppState {
    fn new() -> Self {
        Self {
            input_buffer: String::new(),
            logs: vec!["osno2 engine online. Type /play <song> to start.".to_string()],
            current_track: None,
            volume: 70,
            loop_mode: LoopMode::Off,
            should_quit: false,
        }
    }

    fn execute_command(&mut self, cmd: Command) {
        match cmd {
            Command::Play { query } => {
                let track = query.join(" ");
                self.current_track = Some(track.clone());
                self.logs.push(format!("::▶ Playing: {}", track));
            }
            Command::Volume { level } => {
                self.volume = level;
                self.logs.push(format!("::Volume set to {}%", level));
            }
            Command::Loop { mode } => {
                self.loop_mode = mode;
                self.logs.push(format!("::Loop mode: {:?}", mode));
            }
            Command::Purge => {
                self.logs.push("::Purging broken library track links...".to_string());
            }
            other => {
                self.logs.push(format!("::Command recognized but not handled: {:?}", other));
            }
        }
    }
}

// =============================================================================
// 2. System Entrypoint (Launcher / Worker Orchestrator)
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.contains(&"--worker".to_string()) {
        // --- WORKER MODE ---
        // Runs inside the pristine Ghostty window
        run_worker_appliance().await?;
    } else {
        // --- LAUNCHER MODE ---
        // Runs when the user triggers "osno2" inside their active shell
        let current_exe = std::env::current_exe()?;

        println!("Spawning dedicated osno2 window frame context...");
        SysCommand::new("ghostty")
            .arg("-e")
            .arg(current_exe)
            .arg("--worker")
            .spawn()?;
    }

    Ok(())
}

// =============================================================================
// 3. Worker Application Loop
// =============================================================================

async fn run_worker_appliance() -> Result<(), Box<dyn std::error::Error>> {
    // Terminal Initialization Configuration
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app_state = AppState::new();

    // Primary Interactive Loop Frame
    while !app_state.should_quit {
        terminal.draw(|f| ui_layout(f, &app_state))?;

        // 50ms polling timeout keeps the app responsive for background tasks
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Enter => {
                            if !app_state.input_buffer.is_empty() {
                                let input = app_state.input_buffer.clone();
                                app_state.input_buffer.clear();

                                if input == "/quit" || input == "/exit" {
                                    app_state.should_quit = true;
                                } else {
                                    // Interface with the prompt parsing module cleanly
                                    match OsnoPrompt::parse_line(&input) {
                                        Ok(prompt) => app_state.execute_command(prompt.command),
                                        Err(err) => {
                                            app_state.logs.push(format!("❌ Error: {}", err.raw_message()));
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Char(c) => {
                            app_state.input_buffer.push(c);
                        }
                        KeyCode::Backspace => {
                            app_state.input_buffer.pop();
                        }
                        KeyCode::Esc => {
                            app_state.input_buffer.clear();
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Clean TUI Recovery Handshake
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

// =============================================================================
// 4. UI Rendering Functions
// =============================================================================

fn ui_layout(f: &mut ratatui::Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Status Header Panel
            Constraint::Min(5),    // Main Console/Visualizer Screen
            Constraint::Length(3), // Slash Command Input Bar
        ])
        .split(f.size());

    // Header Widget
    let track_status = state.current_track.as_deref().unwrap_or("[No Track Loaded]");
    let header_text = format!(
        " osno2 // Active: {}  |  Vol: {}%  |  Loop: {:?}",
        track_status, state.volume, state.loop_mode
    );
    let header = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)))
        .style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(header, chunks[0]);

    // System Monitor Console Log Widget
    let logs_content = state.logs.join("\n");
    let display_pane = Paragraph::new(logs_content)
        .block(Block::default().title(" System Status Monitor ").borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    f.render_widget(display_pane, chunks[1]);

    // Command Prompt Input Field
    let prompt_text = format!(" osno2 > {}", state.input_buffer);
    let input_field = Paragraph::new(prompt_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta)),
    );
    f.render_widget(input_field, chunks[2]);
}
