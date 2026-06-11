// model/scrollback.rs

use std::collections::VecDeque;

/// Fixed-capacity ring buffer for terminal scrollback. Oldest lines
/// are dropped once `capacity` is exceeded.
pub struct ScrollbackBuffer {
    lines: VecDeque<String>,
    capacity: usize,
    /// Offset from the bottom (0 = pinned to latest output).
    scroll_offset: usize,
}

impl ScrollbackBuffer {
    pub const DEFAULT_CAPACITY: usize = 10_000;

    pub fn new() -> Self {
        Self::with_capacity(Self::DEFAULT_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            lines: VecDeque::with_capacity(capacity.min(1024)),
            capacity,
            scroll_offset: 0,
        }
    }

    pub fn push_line(&mut self, line: String) {
        if self.lines.len() == self.capacity {
            self.lines.pop_front();
        }
        self.lines.push_back(line);

        // New output resets the view to the bottom unless the user
        // has scrolled back, in which case preserve their position
        // relative to the new tail.
        if self.scroll_offset > 0 {
            self.scroll_offset = (self.scroll_offset + 1).min(self.lines.len().saturating_sub(1));
        }
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.scroll_offset = 0;
    }

    pub fn scroll_up(&mut self, n: u16) {
        let max_offset = self.lines.len().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + n as usize).min(max_offset);
    }

    pub fn scroll_down(&mut self, n: u16) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n as usize);
    }

    /// Returns up to `viewport_height` lines visible at the current
    /// scroll position, oldest first.
    pub fn visible_iter(&self, viewport_height: usize) -> impl Iterator<Item = &String> {
        let total = self.lines.len();
        let end = total.saturating_sub(self.scroll_offset);
        let start = end.saturating_sub(viewport_height);
        self.lines
            .iter()
            .skip(start)
            .take(end.saturating_sub(start))
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }
}

impl Default for ScrollbackBuffer {
    fn default() -> Self {
        Self::new()
    }
}
