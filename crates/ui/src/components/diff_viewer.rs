use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{
        Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
    Frame,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiffLineType {
    Add,
    Remove,
    Context,
    Header,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub line_type: DiffLineType,
    pub old_line_number: Option<usize>,
    pub new_line_number: Option<usize>,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub old_path: String,
    pub new_path: String,
    pub lines: Vec<DiffLine>,
    pub is_binary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffData {
    pub title: String,
    pub files: Vec<FileDiff>,
    pub created_at: String,
}

pub struct DiffViewer {
    diff_data: Option<DiffData>,
    current_file_index: usize,
    scroll_offset: usize,
    line_scroll_offset: usize,
    show_line_numbers: bool,
    show_context: bool,
    syntax_highlighting: bool,
}

impl Default for DiffViewer {
    fn default() -> Self {
        Self::new()
    }
}

impl DiffViewer {
    pub fn new() -> Self {
        DiffViewer {
            diff_data: None,
            current_file_index: 0,
            scroll_offset: 0,
            line_scroll_offset: 0,
            show_line_numbers: true,
            show_context: true,
            syntax_highlighting: true,
        }
    }

    pub fn set_diff(&mut self, diff_data: DiffData) {
        self.current_file_index = 0;
        self.scroll_offset = 0;
        self.line_scroll_offset = 0;
        self.diff_data = Some(diff_data);
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        if let Some(diff) = &self.diff_data {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(1),
                    Constraint::Length(2),
                ])
                .split(area);

            // Render header with file tabs
            self.render_file_tabs(f, chunks[0], diff);

            // Render main diff content
            self.render_diff_content(f, chunks[1], diff);

            // Render navigation help
            self.render_help(f, chunks[2]);
        } else {
            // Render empty state
            let empty_block = Block::default()
                .borders(Borders::ALL)
                .title("Diff Viewer - No Diff Loaded");
            let empty_text =
                Paragraph::new("No diff data available.\nPress 'L' to load diff data.")
                    .block(empty_block)
                    .style(Style::default().fg(Color::Gray));
            f.render_widget(empty_text, area);
        }
    }

    fn render_file_tabs(&self, f: &mut Frame, area: Rect, diff: &DiffData) {
        let file_names: Vec<String> = diff
            .files
            .iter()
            .enumerate()
            .map(|(i, file)| {
                let name = if file.old_path == file.new_path {
                    file.old_path.clone()
                } else {
                    format!("{} -> {}", file.old_path, file.new_path)
                };

                if i == self.current_file_index {
                    format!("[{name}]")
                } else {
                    name
                }
            })
            .collect();

        let tabs_text = file_names.join(" | ");

        let tabs = Paragraph::new(tabs_text)
            .block(Block::default().borders(Borders::ALL).title(format!(
                "Files ({}/{})",
                self.current_file_index + 1,
                diff.files.len()
            )))
            .style(Style::default().fg(Color::Cyan));

        f.render_widget(tabs, area);
    }

    fn render_diff_content(&self, f: &mut Frame, area: Rect, diff: &DiffData) {
        if self.current_file_index >= diff.files.len() {
            return;
        }

        let file = &diff.files[self.current_file_index];

        if file.is_binary {
            let binary_text = Paragraph::new("Binary file - cannot display diff")
                .block(Block::default().borders(Borders::ALL).title("Binary File"))
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(binary_text, area);
            return;
        }

        let visible_lines = area.height.saturating_sub(2).max(1) as usize; // Account for borders
        let start_line = self.scroll_offset;
        let end_line = (start_line + visible_lines).min(file.lines.len());

        let diff_lines: Vec<ListItem> = file.lines[start_line..end_line]
            .iter()
            .map(|line| self.format_diff_line(line))
            .collect();

        let list =
            List::new(diff_lines).block(Block::default().borders(Borders::ALL).title(format!(
                "Diff: {} (Lines {}-{}/{})",
                if file.old_path == file.new_path {
                    &file.old_path
                } else {
                    &format!("{} -> {}", file.old_path, file.new_path)
                },
                start_line + 1,
                end_line,
                file.lines.len()
            )));

        f.render_widget(list, area);

        // Render scrollbar if needed
        if file.lines.len() > visible_lines {
            let scrollbar_area = Rect {
                x: area.x + area.width.saturating_sub(1),
                y: area.y + 1,
                width: 1,
                height: area.height.saturating_sub(2),
            };

            let mut scrollbar_state = ScrollbarState::default()
                .content_length(file.lines.len())
                .position(self.scroll_offset);

            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));

            f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
        }
    }

    fn format_diff_line(&self, line: &DiffLine) -> ListItem<'_> {
        let (style, prefix) = match line.line_type {
            DiffLineType::Add => (Style::default().fg(Color::Green), "+"),
            DiffLineType::Remove => (Style::default().fg(Color::Red), "-"),
            DiffLineType::Context => (Style::default().fg(Color::White), " "),
            DiffLineType::Header => (
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
                "@",
            ),
        };

        let line_number_text = if self.show_line_numbers {
            match (&line.old_line_number, &line.new_line_number) {
                (Some(old), Some(new)) => format!("{old:4}|{new:4}"),
                (Some(old), None) => format!("{old:4}|    "),
                (None, Some(new)) => format!("    |{new:4}"),
                (None, None) => "    |    ".to_string(),
            }
        } else {
            String::new()
        };

        let content = if self.show_line_numbers {
            format!("{} {} {}", line_number_text, prefix, line.content)
        } else {
            format!("{} {}", prefix, line.content)
        };

        ListItem::new(content).style(style)
    }

    fn render_help(&self, f: &mut Frame, area: Rect) {
        let help_text = if self.diff_data.as_ref().map_or(0, |d| d.files.len()) > 1 {
            "←→: Switch files, ↑↓: Scroll, PgUp/PgDn: Page scroll, N: Toggle line numbers, H: Toggle syntax"
        } else {
            "↑↓: Scroll, PgUp/PgDn: Page scroll, N: Toggle line numbers, H: Toggle syntax"
        };

        let help = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL).title("Controls"))
            .style(Style::default().fg(Color::DarkGray))
            .wrap(ratatui::widgets::Wrap { trim: true });

        f.render_widget(help, area);
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> bool {
        if let Some(diff) = &self.diff_data {
            match key.code {
                KeyCode::Up => {
                    if self.scroll_offset > 0 {
                        self.scroll_offset -= 1;
                    }
                    true
                }
                KeyCode::Down => {
                    if let Some(file) = diff.files.get(self.current_file_index) {
                        if self.scroll_offset < file.lines.len().saturating_sub(1) {
                            self.scroll_offset += 1;
                        }
                    }
                    true
                }
                KeyCode::PageUp => {
                    self.scroll_offset = self.scroll_offset.saturating_sub(10);
                    true
                }
                KeyCode::PageDown => {
                    if let Some(file) = diff.files.get(self.current_file_index) {
                        self.scroll_offset =
                            (self.scroll_offset + 10).min(file.lines.len().saturating_sub(1));
                    }
                    true
                }
                KeyCode::Home => {
                    self.scroll_offset = 0;
                    true
                }
                KeyCode::End => {
                    if let Some(file) = diff.files.get(self.current_file_index) {
                        self.scroll_offset = file.lines.len().saturating_sub(1);
                    }
                    true
                }
                KeyCode::Left => {
                    if self.current_file_index > 0 {
                        self.current_file_index -= 1;
                        self.scroll_offset = 0;
                    }
                    true
                }
                KeyCode::Right => {
                    if self.current_file_index < diff.files.len().saturating_sub(1) {
                        self.current_file_index += 1;
                        self.scroll_offset = 0;
                    }
                    true
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    self.show_line_numbers = !self.show_line_numbers;
                    true
                }
                KeyCode::Char('h') | KeyCode::Char('H') => {
                    self.syntax_highlighting = !self.syntax_highlighting;
                    true
                }
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    self.show_context = !self.show_context;
                    true
                }
                _ => false,
            }
        } else {
            false
        }
    }

    pub fn update(&mut self, message: &str) {
        // Handle update messages - could be JSON with new diff data
        if let Ok(diff_data) = serde_json::from_str::<DiffData>(message) {
            self.set_diff(diff_data);
        }
    }

    pub fn has_diff(&self) -> bool {
        self.diff_data.is_some()
    }

    pub fn get_current_file(&self) -> Option<&FileDiff> {
        self.diff_data
            .as_ref()
            .and_then(|diff| diff.files.get(self.current_file_index))
    }

    pub fn clear_diff(&mut self) {
        self.diff_data = None;
        self.current_file_index = 0;
        self.scroll_offset = 0;
        self.line_scroll_offset = 0;
    }

    pub fn get_stats(&self) -> Option<(usize, usize, usize)> {
        self.diff_data.as_ref().map(|diff| {
            let mut additions = 0;
            let mut deletions = 0;
            let files_changed = diff.files.len();

            for file in &diff.files {
                for line in &file.lines {
                    match line.line_type {
                        DiffLineType::Add => additions += 1,
                        DiffLineType::Remove => deletions += 1,
                        _ => {}
                    }
                }
            }

            (files_changed, additions, deletions)
        })
    }
}
