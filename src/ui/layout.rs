use ratatui::{
    prelude::*,
    widgets::{Block, Cell, Gauge, Paragraph, Row, Table},
};

use crate::ui::app::{ActiveView, App};

// Hacker green palette
const GREEN_BRIGHT: Color = Color::Rgb(0, 255, 70);
const GREEN_DIM: Color = Color::Rgb(0, 180, 50);
const GREEN_DARK: Color = Color::Rgb(0, 80, 20);
const BG_BLACK: Color = Color::Rgb(5, 5, 5);
const BG_TRACK_ROW: Color = Color::Rgb(0, 140, 40);

pub struct AppLayout {
    pub header: Rect,
    pub main_view: Rect,
    pub progress: Rect,
    pub fft: Rect,
    pub metadata: Rect,
    pub shell: Rect,
}

impl AppLayout {
    pub fn new(area: Rect) -> Self {
        let vertical = Layout::vertical([
            Constraint::Length(3), // header
            Constraint::Min(0),    // main view
            Constraint::Length(3), // progress + fft
            Constraint::Length(7), // metadata grid
            Constraint::Length(6), // shell
        ])
        .split(area);

        let progress_row =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(vertical[2]);

        Self {
            header: vertical[0],
            main_view: vertical[1],
            progress: progress_row[0],
            fft: progress_row[1],
            metadata: vertical[3],
            shell: vertical[4],
        }
    }

    pub fn render(&self, frame: &mut Frame, app: &App) {
        self.render_header(frame, app);
        self.render_main_view(frame, app);
        self.render_progress(frame, app);
        self.render_fft(frame);
        self.render_metadata(frame, app);
        self.render_shell(frame, app);
    }

    fn render_header(&self, frame: &mut Frame, app: &App) {
        let area = self.header;

        let block = Block::bordered()
            .border_style(Style::default().fg(GREEN_DIM))
            .style(Style::default().bg(BG_BLACK));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let now_playing = format!(" Playing: {}", app.now_playing);
        let vol_label = format!("vol {}% [osno2] ", app.volume);

        let header_cols = Layout::horizontal([
            Constraint::Min(0),
            Constraint::Length(vol_label.len() as u16),
        ])
        .split(inner);

        frame.render_widget(
            Paragraph::new(now_playing).style(Style::default().fg(GREEN_BRIGHT).bg(BG_BLACK)),
            header_cols[0],
        );

        frame.render_widget(
            Paragraph::new(vol_label)
                .alignment(Alignment::Right)
                .style(Style::default().fg(GREEN_DIM).bg(BG_BLACK)),
            header_cols[1],
        );
    }

    fn render_main_view(&self, frame: &mut Frame, app: &App) {
        match app.active_view {
            ActiveView::Tracklist => self.render_tracklist(frame, app),
            ActiveView::Filesystem => self.render_filesystem(frame, app),
            ActiveView::Visualizer => self.render_visualizer_fullscreen(frame),
            ActiveView::Settings => self.render_settings(frame),
            ActiveView::Help => self.render_help(frame),
        }
    }

    fn render_tracklist(&self, frame: &mut Frame, app: &App) {
        let area = self.main_view;

        let block = Block::bordered()
            .title(" tracklist ")
            .title_style(Style::default().fg(GREEN_BRIGHT))
            .border_style(Style::default().fg(GREEN_DIM))
            .style(Style::default().bg(BG_TRACK_ROW));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let rows: Vec<Row> = app
            .tracks
            .iter()
            .enumerate()
            .map(|(i, track)| {
                let is_playing = i == app.playing_track;
                let is_selected = i == app.selected_track;

                let row_style = if is_playing {
                    Style::default().fg(GREEN_BRIGHT).bg(BG_BLACK)
                } else if is_selected {
                    Style::default().fg(GREEN_BRIGHT).bg(GREEN_DARK)
                } else {
                    Style::default().fg(BG_BLACK).bg(BG_TRACK_ROW)
                };

                let marker = if is_playing { "▶" } else { " " };

                Row::new(vec![
                    Cell::from(marker),
                    Cell::from(track.length.clone()),
                    Cell::from(format!("{} ({})", track.name, track.year)),
                    Cell::from(track.artist.clone()),
                    Cell::from(track.ext.clone()),
                ])
                .style(row_style)
            })
            .collect();

        let widths = [
            Constraint::Length(2),
            Constraint::Length(6),
            Constraint::Min(0),
            Constraint::Length(20),
            Constraint::Length(6),
        ];

        let table = Table::new(rows, widths)
            .header(
                Row::new(vec!["", "len", "title", "artist", "type"])
                    .style(Style::default().fg(GREEN_BRIGHT).bg(GREEN_DARK).bold()),
            )
            .column_spacing(2);

        frame.render_widget(table, inner);
    }

    fn render_filesystem(&self, frame: &mut Frame, app: &App) {
        let block = Block::bordered()
            .title(format!(" {} ", app.working_dir.display()))
            .title_style(Style::default().fg(GREEN_BRIGHT))
            .border_style(Style::default().fg(GREEN_DIM))
            .style(Style::default().bg(BG_BLACK));

        let inner = block.inner(self.main_view);
        frame.render_widget(block, self.main_view);

        let entries: Vec<Line> = app
            .fs_entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let style = if i == app.fs_selected {
                    Style::default().fg(GREEN_BRIGHT).bg(GREEN_DARK)
                } else {
                    Style::default().fg(GREEN_DIM)
                };
                Line::styled(format!(" {entry}"), style)
            })
            .collect();

        frame.render_widget(
            Paragraph::new(entries).style(Style::default().bg(BG_BLACK)),
            inner,
        );
    }

    fn render_visualizer_fullscreen(&self, frame: &mut Frame) {
        let block = Block::bordered()
            .title(" visualizer ")
            .title_style(Style::default().fg(GREEN_BRIGHT))
            .border_style(Style::default().fg(GREEN_DIM))
            .style(Style::default().bg(BG_BLACK));

        let inner = block.inner(self.main_view);
        frame.render_widget(block, self.main_view);

        let dots = render_dot_grid(inner, &[]);
        frame.render_widget(
            Paragraph::new(dots).style(Style::default().bg(BG_BLACK)),
            inner,
        );
    }

    fn render_settings(&self, frame: &mut Frame) {
        let block = Block::bordered()
            .title(" settings ")
            .title_style(Style::default().fg(GREEN_BRIGHT))
            .border_style(Style::default().fg(GREEN_DIM))
            .style(Style::default().bg(BG_BLACK));

        frame.render_widget(
            Paragraph::new(" settings not yet implemented")
                .style(Style::default().fg(GREEN_DIM).bg(BG_BLACK))
                .block(block),
            self.main_view,
        );
    }

    fn render_help(&self, frame: &mut Frame) {
        let content = vec![
            Line::styled(" keybindings", Style::default().fg(GREEN_BRIGHT).bold()),
            Line::raw(""),
            Line::styled(" Alt+1   tracklist", Style::default().fg(GREEN_DIM)),
            Line::styled(" Alt+2   filesystem", Style::default().fg(GREEN_DIM)),
            Line::styled(" Alt+3   visualizer", Style::default().fg(GREEN_DIM)),
            Line::styled(" Alt+4   settings", Style::default().fg(GREEN_DIM)),
            Line::styled(" Alt+5   help", Style::default().fg(GREEN_DIM)),
            Line::raw(""),
            Line::styled(" shell commands", Style::default().fg(GREEN_BRIGHT).bold()),
            Line::raw(""),
            Line::styled(
                " cd <path>        change directory",
                Style::default().fg(GREEN_DIM),
            ),
            Line::styled(
                " ls               list directory",
                Style::default().fg(GREEN_DIM),
            ),
            Line::styled(
                " play <query>     play track",
                Style::default().fg(GREEN_DIM),
            ),
            Line::styled(
                " queue <query>    add to queue",
                Style::default().fg(GREEN_DIM),
            ),
            Line::styled(
                " volume <0-100>   set volume",
                Style::default().fg(GREEN_DIM),
            ),
            Line::styled(
                " purge            clear library index",
                Style::default().fg(GREEN_DIM),
            ),
            Line::raw(""),
            Line::styled(" q / Ctrl+C       quit", Style::default().fg(GREEN_DIM)),
        ];

        let block = Block::bordered()
            .title(" help ")
            .title_style(Style::default().fg(GREEN_BRIGHT))
            .border_style(Style::default().fg(GREEN_DIM))
            .style(Style::default().bg(BG_BLACK));

        frame.render_widget(
            Paragraph::new(content)
                .style(Style::default().bg(BG_BLACK))
                .block(block),
            self.main_view,
        );
    }

    fn render_progress(&self, frame: &mut Frame, app: &App) {
        let block = Block::bordered()
            .title(format!(" {} / {} ", app.position_str(), app.duration_str()))
            .title_style(Style::default().fg(GREEN_DIM))
            .border_style(Style::default().fg(GREEN_DIM))
            .style(Style::default().bg(BG_BLACK));

        let ratio = if app.duration_secs > 0 {
            (app.position_secs as f64 / app.duration_secs as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let gauge = Gauge::default()
            .block(block)
            .gauge_style(Style::default().fg(GREEN_BRIGHT).bg(GREEN_DARK))
            .ratio(ratio)
            .label("");

        frame.render_widget(gauge, self.progress);
    }

    fn render_fft(&self, frame: &mut Frame) {
        let block = Block::bordered()
            .title(" spectrum ")
            .title_style(Style::default().fg(GREEN_DIM))
            .border_style(Style::default().fg(GREEN_DIM))
            .style(Style::default().bg(BG_BLACK));

        let inner = block.inner(self.fft);
        frame.render_widget(block, self.fft);

        let dots = render_dot_grid(inner, &[]);
        frame.render_widget(
            Paragraph::new(dots).style(Style::default().bg(BG_BLACK)),
            inner,
        );
    }

    fn render_metadata(&self, frame: &mut Frame, app: &App) {
        let block = Block::bordered()
            .title(" metadata ")
            .title_style(Style::default().fg(GREEN_DIM))
            .border_style(Style::default().fg(GREEN_DIM))
            .style(Style::default().bg(BG_BLACK));

        let inner = block.inner(self.metadata);
        frame.render_widget(block, self.metadata);

        let cols = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(inner);

        let t = &app.track_meta;
        let stars = "★".repeat(t.rating) + &"☆".repeat(5_usize.saturating_sub(t.rating));

        let col1 = vec![
            meta_line("Title", &t.title),
            meta_line("Artist", &t.artist),
            meta_line("Album", &t.album),
            meta_line("Year", &t.year),
        ];

        let col2 = vec![
            meta_line("Track", &t.track_num),
            meta_line("Genre", &t.genre),
            meta_line("Time", &t.time),
            meta_line("Size", &t.size),
            meta_line("Rating", &stars),
        ];

        frame.render_widget(
            Paragraph::new(col1).style(Style::default().bg(BG_BLACK)),
            cols[0],
        );
        frame.render_widget(
            Paragraph::new(col2).style(Style::default().bg(BG_BLACK)),
            cols[1],
        );
    }

    fn render_shell(&self, frame: &mut Frame, app: &App) {
        let block = Block::bordered()
            .title(format!(" {} ", app.working_dir.display()))
            .title_style(Style::default().fg(GREEN_DIM))
            .border_style(Style::default().fg(GREEN_DIM))
            .style(Style::default().bg(BG_BLACK));

        let inner = block.inner(self.shell);
        frame.render_widget(block, self.shell);

        let history_height = inner.height.saturating_sub(1) as usize;

        let visible_history: Vec<Line> = app
            .shell_history
            .iter()
            .rev()
            .take(history_height)
            .rev()
            .map(|line| Line::styled(format!(" {line}"), Style::default().fg(GREEN_DIM)))
            .collect();

        let prompt = Line::styled(
            format!(" {}/ {}_", app.working_dir.display(), app.shell_input),
            Style::default().fg(GREEN_BRIGHT),
        );

        let mut lines = visible_history;
        lines.push(prompt);

        frame.render_widget(
            Paragraph::new(lines).style(Style::default().bg(BG_BLACK)),
            inner,
        );
    }
}

// `buckets` is a slice of f32 in [0.0, 1.0] per frequency band.
// Empty slice renders a static placeholder grid.
fn render_dot_grid(area: Rect, buckets: &[f32]) -> Vec<Line<'static>> {
    let cols = area.width as usize;
    let rows = area.height as usize;

    (0..rows)
        .map(|row| {
            let spans: Vec<Span> = (0..cols)
                .map(|col| {
                    let magnitude = if buckets.is_empty() {
                        0.0
                    } else {
                        let idx = (col * buckets.len()) / cols.max(1);
                        buckets.get(idx).copied().unwrap_or(0.0)
                    };

                    let threshold = 1.0 - (row as f32 / rows.saturating_sub(1).max(1) as f32);
                    let lit = magnitude >= threshold;

                    if lit {
                        Span::styled("•", Style::default().fg(GREEN_BRIGHT))
                    } else {
                        Span::styled("·", Style::default().fg(GREEN_DARK))
                    }
                })
                .collect();

            Line::from(spans)
        })
        .collect()
}

fn meta_line<'a>(key: &'a str, value: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!(" {key}: "), Style::default().fg(GREEN_DIM)),
        Span::styled(value.to_string(), Style::default().fg(GREEN_BRIGHT)),
    ])
}
