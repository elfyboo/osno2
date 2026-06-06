use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use std::io;

use crate::ui::app::App;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = setup_terminal()?;

    let result = event_loop(&mut terminal);

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
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();

    loop {
        terminal.draw(|frame| app.draw(frame))?;

        // Drain all pending events before the next frame to avoid input lag
        // accumulating across slow renders. Filter to Press only -- crossterm
        // emits Repeat and Release on some terminals, causing doubled input.
        while crossterm::event::poll(std::time::Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if should_quit(key) {
                    return Ok(());
                }
                app.handle_key(key);
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}

fn should_quit(key: KeyEvent) -> bool {
    if !key.modifiers.contains(KeyModifiers::CONTROL) {
        return false;
    }
    match key.code {
        KeyCode::Char('c') | KeyCode::Char('C') => true,
        _ => false,
    }
}
