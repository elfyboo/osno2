use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;

use crate::ui::layout::AppLayout;

pub struct App {
    pub mode: AppMode,
    pub command_input: String,
}

#[derive(Default, PartialEq)]
pub enum AppMode {
    #[default]
    Normal,
    Command,
}

impl App {
    pub fn new() -> Self {
        Self {
            mode: AppMode::default(),
            command_input: String::new(),
        }
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let layout = AppLayout::new(frame.area());
        layout.render(frame, self);
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match self.mode {
            AppMode::Normal => self.handle_normal(key),
            AppMode::Command => self.handle_command(key),
        }
    }

    fn handle_normal(&mut self, key: KeyEvent) {
        if let KeyCode::Char('/') = key.code {
            self.mode = AppMode::Command;
            self.command_input.clear();
        }
    }

    fn handle_command(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.mode = AppMode::Normal;
                self.command_input.clear();
            }
            KeyCode::Enter => {
                // TODO: route self.command_input to clap parser
                self.mode = AppMode::Normal;
                self.command_input.clear();
            }
            KeyCode::Backspace => {
                self.command_input.pop();
            }
            KeyCode::Char(c) => {
                self.command_input.push(c);
            }
            _ => {}
        }
    }
}
