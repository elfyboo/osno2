use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
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

        if crossterm::event::poll(std::time::Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if should_quit(key) {
                    break;
                }
                app.handle_key(key);
            }
        }
    }

    Ok(())
}

fn should_quit(key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => true,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => true,
        _ => false,
    }
}
