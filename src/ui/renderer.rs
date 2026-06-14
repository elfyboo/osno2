use crate::ui::app::{ActiveView, App};
use ratatui::{
    prelude::*,
    widgets::{Block, Cell, Gauge, Padding, Paragraph, Row, Table},
};
use tui_term::widget::PseudoTerminal;

// Default Terminal Color Palette
const _COLOR_BRIGHTEST: Color = Color::Rgb(0xfd, 0xeb, 0x18); // brightest
const _COLOR_0: Color = Color::Rgb(0xda, 0xa1, 0x00);
const _COLOR_1: Color = Color::Rgb(0x9b, 0x4c, 0x00);
const _COLOR_2: Color = Color::Rgb(0xfb, 0x12, 0x00);
const _COLOR_DARKEST: Color = Color::Rgb(0x09, 0x00, 0x01); // darkest

// Semantic aliases mapped onto palette
const FG_BRIGHT: Color = _COLOR_BRIGHTEST;
const FG_ACCENT: Color = _COLOR_0;
const FG_DIM: Color = _COLOR_1;
const BORDER_DIM: Color = _COLOR_2;
const BG_BLACK: Color = _COLOR_DARKEST;
const BG_TRACK_ROW: Color = _COLOR_DARKEST;
const BG_FILL: Color = _COLOR_DARKEST;

// Tunables for the player/console band heights (both bands share the same
// height). Default height is `area.height / MIN_BAND_HEIGHT_DIVISOR`,
// capped at `MAX_BAND_HEIGHT_CELLS`. Increase the divisor or lower the cap
// to give more space to the modal view in the middle.
const MIN_BAND_HEIGHT_DIVISOR: u16 = 4;
const MAX_BAND_HEIGHT_CELLS: u16 = 16;

pub struct AppLayout {
    pub main_view: Rect,
    pub snackbar: Rect,
    pub thumbnail: Rect,
    pub snackbar_right: Rect,
    pub progress: Rect,
    pub metadata: Rect,
    pub shell: Rect,
}

impl AppLayout {
    pub fn new(area: Rect) -> Self {
        // 1-cell padding around the entire TUI.
        let area = area.inner(Margin::new(1, 1));

        // Top row: player, bottom row: console. Both share the same height,
        // defaulting to 1/3 of the total screen height and capped at
        // MAX_BAND_HEIGHT. The modal view in between flexes to fill
        // whatever space is left.
        let band_height = (area.height / MIN_BAND_HEIGHT_DIVISOR)
            .min(MAX_BAND_HEIGHT_CELLS)
            .min(area.height / 2);
        let main_view_height = area.height - 2 * band_height;

        let vertical = Layout::vertical([
            Constraint::Length(band_height),
            Constraint::Length(main_view_height),
            Constraint::Length(band_height),
        ])
        .split(area);

        let snackbar = vertical[0];

        // Square thumbnail slot on the left, sized to the snackbar's inner height.
        // Block border consumes 2 rows/cols; terminal cells are ~2:1 (h:w), so
        // width = 2 * inner_height keeps the slot visually square.
        let inner_height = snackbar.height.saturating_sub(2);
        let thumbnail_width = (inner_height * 2).clamp(1, snackbar.width.saturating_sub(1));

        let snackbar_cols =
            Layout::horizontal([Constraint::Length(thumbnail_width), Constraint::Min(0)])
                .split(snackbar);

        let thumbnail = snackbar_cols[0];
        let snackbar_right = snackbar_cols[1];

        let right_rows = Layout::vertical([
            Constraint::Length(1), // duration / progress bar
            Constraint::Min(0),    // metadata grid
        ])
        .split(snackbar_right);

        Self {
            main_view: vertical[1],
            snackbar,
            thumbnail,
            snackbar_right,
            progress: right_rows[0],
            metadata: right_rows[1],
            shell: vertical[2],
        }
    }

    /// UPDATE: Receives the live raw vt100 virtual screen buffer straight from the worker controller
    pub fn render(&self, frame: &mut Frame, app: &mut App, vt_screen: &tui_term::vt100::Screen) {
        self.render_main_view(frame, app);
        self.render_audio_player(frame, app);
        self.render_shell(frame, vt_screen); // UPDATE: Routed to terminal widget execution layer
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
            .title_style(Style::default().fg(FG_BRIGHT))
            .border_style(Style::default().fg(BORDER_DIM))
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
                    Style::default().fg(FG_BRIGHT).bg(BG_BLACK)
                } else if is_selected {
                    Style::default().fg(FG_BRIGHT).bg(BG_FILL)
                } else {
                    Style::default().fg(BG_BLACK).bg(BG_TRACK_ROW)
                };

                let marker = if is_playing { "▶" } else { " " };

                let duration_str = format!(
                    "{:02}:{:02}",
                    track.duration_secs / 60,
                    track.duration_secs % 60
                );
                let year_str = track.year.map(|y| format!("({y})")).unwrap_or_default();

                Row::new(vec![
                    Cell::from(marker),
                    Cell::from(duration_str),
                    Cell::from(format!("{} {}", track.name, year_str)),
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
                    .style(Style::default().fg(FG_BRIGHT).bg(BG_FILL).bold()),
            )
            .column_spacing(2);

        frame.render_widget(table, inner);
    }

    fn render_filesystem(&self, frame: &mut Frame, app: &App) {
        let block = Block::bordered()
            .title(format!(" {} ", app.working_dir.display()))
            .title_style(Style::default().fg(FG_BRIGHT))
            .border_style(Style::default().fg(BORDER_DIM))
            .style(Style::default().bg(BG_BLACK));

        let inner = block.inner(self.main_view);
        frame.render_widget(block, self.main_view);

        let entries: Vec<Line> = app
            .fs_entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let style = if i == app.fs_selected {
                    Style::default().fg(FG_BRIGHT).bg(BG_FILL)
                } else if entry.is_dir {
                    Style::default().fg(FG_BRIGHT)
                } else {
                    Style::default().fg(FG_DIM)
                };

                let display_name = if entry.is_dir {
                    format!("{}/", entry.name)
                } else {
                    entry.name.clone()
                };

                Line::styled(format!(" {display_name}"), style)
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
            .title_style(Style::default().fg(FG_BRIGHT))
            .border_style(Style::default().fg(BORDER_DIM))
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
            .title_style(Style::default().fg(FG_BRIGHT))
            .border_style(Style::default().fg(BORDER_DIM))
            .style(Style::default().bg(BG_BLACK));

        frame.render_widget(
            Paragraph::new(" settings not yet implemented")
                .style(Style::default().fg(FG_DIM).bg(BG_BLACK))
                .block(block),
            self.main_view,
        );
    }

    fn render_help(&self, frame: &mut Frame) {
        let content = vec![
            Line::styled(" keybindings", Style::default().fg(FG_BRIGHT).bold()),
            Line::raw(""),
            Line::styled(" Alt+1   tracklist", Style::default().fg(FG_DIM)),
            Line::styled(" Alt+2   filesystem", Style::default().fg(FG_DIM)),
            Line::styled(" Alt+3   visualizer", Style::default().fg(FG_DIM)),
            Line::styled(" Alt+4   settings", Style::default().fg(FG_DIM)),
            Line::styled(" Alt+5   help", Style::default().fg(FG_DIM)),
            Line::raw(""),
            Line::styled(" shell commands", Style::default().fg(FG_BRIGHT).bold()),
            Line::raw(""),
            Line::styled(
                " cd <path>        change directory",
                Style::default().fg(FG_DIM),
            ),
            Line::styled(
                " ls               list directory",
                Style::default().fg(FG_DIM),
            ),
            Line::styled(" play <query>     play track", Style::default().fg(FG_DIM)),
            Line::styled(
                " queue <query>    add to queue",
                Style::default().fg(FG_DIM),
            ),
            Line::styled(" volume <0-100>   set volume", Style::default().fg(FG_DIM)),
            Line::styled(
                " purge            clear library index",
                Style::default().fg(FG_DIM),
            ),
            Line::raw(""),
            Line::styled(" q / Ctrl+C       quit", Style::default().fg(FG_DIM)),
        ];

        let block = Block::bordered()
            .title(" help ")
            .title_style(Style::default().fg(FG_BRIGHT))
            .border_style(Style::default().fg(BORDER_DIM))
            .style(Style::default().bg(BG_BLACK));

        frame.render_widget(
            Paragraph::new(content)
                .style(Style::default().bg(BG_BLACK))
                .block(block),
            self.main_view,
        );
    }

    fn render_audio_player(&self, frame: &mut Frame, app: &App) {
        let block = Block::bordered()
            .title(" Player ")
            .title_style(Style::default().fg(FG_BRIGHT).bold())
            .border_style(Style::default().fg(BORDER_DIM))
            .style(Style::default().bg(BG_BLACK));

        let inner = block.inner(self.snackbar);
        frame.render_widget(block, self.snackbar);

        let inner_cols =
            Layout::horizontal([Constraint::Length(self.thumbnail.width), Constraint::Min(0)])
                .split(inner);

        // 1-cell gap between thumbnail and the right column
        let right_block = Block::default().padding(Padding::left(1));
        let left_block = Block::default().padding(Padding::left(1));

        let right_area = right_block.inner(inner_cols[1]);
        let left_area = left_block.inner(inner_cols[0]);

        let inner_left =
            Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(left_area);

        let inner_right =
            Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(right_area);

        self.render_thumbnail(frame, inner_left[0]);
        self.render_progress(frame, app, inner_right[0]);
        self.render_metadata(frame, app, inner_right[1]);
    }

    /// Square slot showing either the spectrum visualizer or album art thumbnail.
    /// Currently always renders the visualizer placeholder; swap on `app` state
    /// once album art loading lands.

    fn render_thumbnail(&self, frame: &mut Frame, area: Rect) {
        let dots = render_dot_grid(area, &[]);

        frame.render_widget(
            Paragraph::new(dots).style(Style::default().bg(BG_BLACK)),
            area,
        );
    }

    fn render_progress(&self, frame: &mut Frame, app: &App, area: Rect) {
        let label = format!("{} / {}", app.position_str(), app.duration_str());

        let ratio = if app.duration_secs > 0 {
            (app.position_secs as f64 / app.duration_secs as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(FG_BRIGHT).bg(BG_FILL))
            .ratio(ratio)
            .label(Span::styled(label, Style::default().fg(FG_ACCENT)));

        frame.render_widget(gauge, area);
    }

    fn render_metadata(&self, frame: &mut Frame, app: &App, area: Rect) {
        let block = Block::default().padding(Padding::top(1));
        let area = block.inner(area);

        let cols = Layout::horizontal([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(area);

        let mut col1 = vec![];
        let mut col2 = vec![];
        let mut col3 = vec![];

        if !app.tracks.is_empty() && app.selected_track < app.tracks.len() {
            let t = &app.tracks[app.selected_track];

            let sr = &t.clone().sample_rate.unwrap_or(0).to_string();
            let stars = "★".repeat(t.rating as usize)
                + &"☆".repeat(5_usize.saturating_sub(t.rating as usize));

            col1 = vec![
                meta_line("Title", &t.name),
                meta_line("Artist", &t.artist),
                meta_line("Album", &t.album),
                meta_line("Codec", t.codec.as_deref().unwrap_or("N/A")),
            ];

            col2 = vec![
                meta_line("Track", "0"),
                meta_line("Sample Rate", sr),
                meta_line(
                    "Bitrate",
                    &t.bitrate
                        .map(|b| b.to_string())
                        .unwrap_or_else(|| "N/A".to_string()),
                ),
            ];

            col3 = vec![
                meta_line("Added", &t.added_at.to_string()),
                meta_line("Size", &format!("{} bytes", t.size_bytes)),
                meta_line(
                    "Year",
                    &t.year
                        .map(|y| y.to_string())
                        .unwrap_or_else(|| "N/A".to_string()),
                ),
                meta_line("Rating", &stars),
            ];
        }

        frame.render_widget(
            Paragraph::new(col1).style(Style::default().bg(BG_BLACK)),
            cols[0],
        );
        frame.render_widget(
            Paragraph::new(col2).style(Style::default().bg(BG_BLACK)),
            cols[1],
        );
        frame.render_widget(
            Paragraph::new(col3).style(Style::default().bg(BG_BLACK)),
            cols[2],
        );
    }

    /// UPDATE: Replaces old custom paragraph rendering with the live tui-term widget.
    fn render_shell(&self, frame: &mut Frame, vt_screen: &tui_term::vt100::Screen) {
        let block = Block::new()
            .title(" OS INTERACTIVE CONSOLE (Ctrl+T) ")
            .title_style(Style::default().fg(FG_BRIGHT).bold())
            //.border_style(Style::default().fg(BORDER_DIM))
            .style(Style::default().bg(BG_BLACK));

        let terminal_widget = PseudoTerminal::new(vt_screen).block(block);

        frame.render_widget(terminal_widget, self.shell);
    }
}

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
                        Span::styled("•", Style::default().fg(FG_BRIGHT))
                    } else {
                        Span::styled("·", Style::default().fg(BORDER_DIM))
                    }
                })
                .collect();

            Line::from(spans)
        })
        .collect()
}

fn meta_line(key: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!(" {key}: "), Style::default().fg(FG_DIM)),
        Span::styled(value.to_string(), Style::default().fg(FG_BRIGHT)),
    ])
}
