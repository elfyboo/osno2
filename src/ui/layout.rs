use ratatui::{
    prelude::*,
    //widgets::{Block, Borders, Paragraph},
    widgets::{Block, Paragraph},
};

use crate::ui::app::{App, AppMode};

pub struct AppLayout {
    pub header: Rect,
    pub library: Rect,
    pub visualizer: Rect,
    pub player: Rect,
    pub command: Rect,
}

impl AppLayout {
    pub fn new(area: Rect) -> Self {
        // Vertical split: header / body / player / command
        let vertical = Layout::vertical([
            Constraint::Length(3), // header
            Constraint::Min(0),    // body
            Constraint::Length(3), // player bar
            Constraint::Length(3), // command bar
        ])
        .split(area);

        // Horizontal split of body: library | visualizer
        let body = Layout::horizontal([
            Constraint::Percentage(40), // library
            Constraint::Percentage(60), // visualizer
        ])
        .split(vertical[1]);

        Self {
            header: vertical[0],
            library: body[0],
            visualizer: body[1],
            player: vertical[2],
            command: vertical[3],
        }
    }

    pub fn render(&self, frame: &mut Frame, app: &App) {
        self.render_header(frame);
        self.render_library(frame);
        self.render_visualizer(frame);
        self.render_player(frame);
        self.render_command(frame, app);
    }

    fn render_header(&self, frame: &mut Frame) {
        let block = Block::bordered()
            .title(" osno2 ")
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(Color::Cyan));

        frame.render_widget(block, self.header);
    }

    fn render_library(&self, frame: &mut Frame) {
        let block = Block::bordered()
            .title(" library ")
            .border_style(Style::default().fg(Color::DarkGray));

        frame.render_widget(block, self.library);
    }

    fn render_visualizer(&self, frame: &mut Frame) {
        let block = Block::bordered()
            .title(" visualizer ")
            .border_style(Style::default().fg(Color::DarkGray));

        frame.render_widget(block, self.visualizer);
    }

    fn render_player(&self, frame: &mut Frame) {
        let block = Block::bordered()
            .title(" player ")
            .border_style(Style::default().fg(Color::DarkGray));

        frame.render_widget(block, self.player);
    }

    fn render_command(&self, frame: &mut Frame, app: &App) {
        let (title, content, border_style) = match app.mode {
            AppMode::Command => (
                " command ",
                format!("/{}", app.command_input),
                Style::default().fg(Color::Yellow),
            ),
            AppMode::Normal => (
                " press / to enter a command ",
                String::new(),
                Style::default().fg(Color::DarkGray),
            ),
        };

        let block = Block::bordered().title(title).border_style(border_style);

        let paragraph = Paragraph::new(content).block(block);

        frame.render_widget(paragraph, self.command);
    }
}
