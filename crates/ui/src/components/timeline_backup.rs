use ratatui::{
    Frame,
    layout::{Rect, Constraint, Direction, Layout, Alignment},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    style::{Color, Style, Modifier},
    text::{Line, Span},
};
use crossterm::event::{KeyCode, KeyEvent};
use serde::{Serialize, Deserialize};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEvent {
    pub id: String,
    pub name: String,
    pub status: TaskStatus,
    pub progress: u16, // 0-100
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub duration: Option<u64>, // seconds
    pub error_message: Option<String>,
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineData {
    pub session_id: String,
    pub title: String,
    pub started_at: String,
    pub tasks: Vec<TaskEvent>,
    pub overall_progress: u16,
    pub estimated_total_duration: Option<u64>,
}

pub struct Timeline {
    timeline_data: Option<TimelineData>,
    selected_task_index: usize,
    scroll_offset: usize,
    show_details: bool,
    auto_scroll: bool,
    last_update: Instant,
    elapsed_time: Duration,
    show_completed: bool,
    show_failed: bool,
}

impl Default for Timeline {
    fn default() -> Self {
        Self::new()
    }
}

impl Timeline {
    pub fn new() -> Self {
        Timeline {
            timeline_data: None,
            selected_task_index: 0,
            scroll_offset: 0,
            show_details: false,
            auto_scroll: true,
            last_update: Instant::now(),
            elapsed_time: Duration::new(0, 0),
            show_completed: true,
            show_failed: true,
        }
    }

    pub fn set_timeline(&mut self, timeline: TimelineData) {
        self.selected_task_index = 0;
        self.scroll_offset = 0;
        self.timeline_data = Some(timeline);
        self.last_update = Instant::now();
        self.elapsed_time = Duration::new(0, 0);
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        match &self.timeline_data {
            Some(timeline) => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),  // Overall progress
                        Constraint::Min(5),     // Task list
                        Constraint::Length(4),  // Details/Stats
                        Constraint::Length(2),  // Controls
                    ])
                    .split(area);

                // Render overall progress
                self.render_overall_progress(f, chunks[0], &timeline);
                
                // Render task list  
                self.render_task_list(f, chunks[1], &timeline);
                
                // Render details/stats
                if self.show_details && self.selected_task_index < timeline.tasks.len() {
                    self.render_task_details(f, chunks[2], &timeline.tasks[self.selected_task_index]);
                } else {
                    self.render_session_stats(f, chunks[2], &timeline);
                }
                
                // Render controls
                self.render_controls(f, chunks[3]);
            }
            None => {
        } else {
            // Render empty state
            let empty_block = Block::default()
                .borders(Borders::ALL)
                .title("Timeline - No Session Active");
            let empty_text = Paragraph::new("No execution timeline available.\nStart an execution to see progress here.")
                .block(empty_block)
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center);
            f.render_widget(empty_text, area);
        }
    }

    fn render_overall_progress(&mut self, f: &mut Frame, area: Rect, timeline: &TimelineData) {
        // Update elapsed time
        self.elapsed_time += self.last_update.elapsed();
        self.last_update = Instant::now();
        
        let progress_label = if timeline.overall_progress == 100 {
            "Completed".to_string()
        } else {
            format!("Progress: {}%", timeline.overall_progress)
        };

        let elapsed_seconds = self.elapsed_time.as_secs();
        let time_label = format!(
            "Elapsed: {:02}:{:02}:{:02}",
            elapsed_seconds / 3600,
            (elapsed_seconds % 3600) / 60,
            elapsed_seconds % 60
        );

        let title = format!("{} | {}", progress_label, time_label);
        
        let progress_color = if timeline.overall_progress == 100 {
            Color::Green
        } else if timeline.tasks.iter().any(|t| matches!(t.status, TaskStatus::Failed)) {
            Color::Red
        } else {
            Color::Yellow
        };

        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title(title))
            .gauge_style(Style::default().fg(progress_color))
            .percent(timeline.overall_progress)
            .label(timeline.title.clone());

        f.render_widget(gauge, area);
    }

    fn render_task_list(&mut self, f: &mut Frame, area: Rect, timeline: &TimelineData) {
        let filtered_tasks: Vec<(usize, &TaskEvent)> = timeline.tasks
            .iter()
            .enumerate()
            .filter(|(_, task)| {
                match task.status {
                    TaskStatus::Completed => self.show_completed,
                    TaskStatus::Failed => self.show_failed,
                    _ => true,
                }
            })
            .collect();

        let items: Vec<ListItem> = filtered_tasks
            .iter()
            .skip(self.scroll_offset)
            .map(|(original_index, task)| {
                let (status_icon, style) = match task.status {
                    TaskStatus::Pending => ("⏳", Style::default().fg(Color::Gray)),
                    TaskStatus::Running => ("🔄", Style::default().fg(Color::Yellow)),
                    TaskStatus::Completed => ("✅", Style::default().fg(Color::Green)),
                    TaskStatus::Failed => ("❌", Style::default().fg(Color::Red)),
                    TaskStatus::Cancelled => ("⛔", Style::default().fg(Color::DarkGray)),
                };

                let progress_bar = if task.progress > 0 {
                    let filled = (task.progress as f32 / 100.0 * 10.0) as usize;
                    let bar = "█".repeat(filled) + &"░".repeat(10 - filled);
                    format!(" [{}] {}%", bar, task.progress)
                } else {
                    String::new()
                };

                let duration_text = task.duration
                    .map(|d| format!(" ({}s)", d))
                    .unwrap_or_default();

                let content = format!(
                    "{} {} {}{}{}",
                    status_icon,
                    task.name,
                    progress_bar,
                    duration_text,
                    if task.error_message.is_some() { " ⚠" } else { "" }
                );

                let item_style = if *original_index == self.selected_task_index {
                    style.add_modifier(Modifier::BOLD).bg(Color::DarkGray)
                } else {
                    style
                };

                ListItem::new(content).style(item_style)
            })
            .collect();

        let visible_count = filtered_tasks.len();
        let title = format!(
            "Tasks ({} visible, {} total)",
            visible_count,
            timeline.tasks.len()
        );

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(title))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol(">> ");

        f.render_widget(list, area);
        
        // Auto-scroll to show running tasks
        if self.auto_scroll {
            if let Some((idx, _)) = filtered_tasks.iter().find(|(_, task)| task.status == TaskStatus::Running) {
                self.selected_task_index = *idx;
            }
        }
    }

    fn render_task_details(&self, f: &mut Frame, area: Rect, task: &TaskEvent) {
        let mut details = vec![
            Line::from(vec![Span::styled("Task Details", Style::default().add_modifier(Modifier::BOLD))]),
            Line::from(vec![Span::raw("")]),
            Line::from(vec![Span::styled("ID: ", Style::default().fg(Color::Cyan)), Span::raw(&task.id)]),
            Line::from(vec![Span::styled("Name: ", Style::default().fg(Color::Cyan)), Span::raw(&task.name)]),
            Line::from(vec![Span::styled("Status: ", Style::default().fg(Color::Cyan)), 
                Span::styled(format!("{:?}", task.status), self.get_status_style(&task.status))]),
            Line::from(vec![Span::styled("Progress: ", Style::default().fg(Color::Cyan)), 
                Span::raw(format!("{}%", task.progress))]),
        ];

        if let Some(start_time) = &task.start_time {
            details.push(Line::from(vec![Span::styled("Started: ", Style::default().fg(Color::Cyan)), 
                Span::raw(start_time)]));
        }

        if let Some(end_time) = &task.end_time {
            details.push(Line::from(vec![Span::styled("Ended: ", Style::default().fg(Color::Cyan)), 
                Span::raw(end_time)]));
        }

        if let Some(duration) = task.duration {
            details.push(Line::from(vec![Span::styled("Duration: ", Style::default().fg(Color::Cyan)), 
                Span::raw(format!("{} seconds", duration))]));
        }

        if let Some(error) = &task.error_message {
            details.push(Line::from(vec![Span::raw("")]));
            details.push(Line::from(vec![Span::styled("Error: ", Style::default().fg(Color::Red)), 
                Span::styled(error, Style::default().fg(Color::Red))]));
        }

        if !task.metadata.is_empty() {
            details.push(Line::from(vec![Span::raw("")]));
            details.push(Line::from(vec![Span::styled("Metadata:", Style::default().fg(Color::Cyan))]));
            for (key, value) in &task.metadata {
                details.push(Line::from(vec![Span::raw(format!("  {}: {}", key, value))]));
            }
        }

        let details_paragraph = Paragraph::new(details)
            .block(Block::default().borders(Borders::ALL).title("Task Details"))
            .wrap(ratatui::widgets::Wrap { trim: true });

        f.render_widget(details_paragraph, area);
    }

    fn render_session_stats(&self, f: &mut Frame, area: Rect, timeline: &TimelineData) {
        let completed = timeline.tasks.iter().filter(|t| t.status == TaskStatus::Completed).count();
        let running = timeline.tasks.iter().filter(|t| t.status == TaskStatus::Running).count();
        let failed = timeline.tasks.iter().filter(|t| t.status == TaskStatus::Failed).count();
        let pending = timeline.tasks.iter().filter(|t| t.status == TaskStatus::Pending).count();

        let avg_duration = if completed > 0 {
            let total_duration: u64 = timeline.tasks
                .iter()
                .filter_map(|t| t.duration)
                .sum();
            Some(total_duration / completed as u64)
        } else {
            None
        };

        let mut stats = vec![
            Line::from(vec![Span::styled("Session Statistics", Style::default().add_modifier(Modifier::BOLD))]),
            Line::from(vec![Span::raw("")]),
            Line::from(vec![Span::styled(format!("✅ Completed: {}", completed), Style::default().fg(Color::Green))]),
            Line::from(vec![Span::styled(format!("🔄 Running: {}", running), Style::default().fg(Color::Yellow))]),
            Line::from(vec![Span::styled(format!("⏳ Pending: {}", pending), Style::default().fg(Color::White))]),
            Line::from(vec![Span::styled(format!("❌ Failed: {}", failed), Style::default().fg(Color::Red))]),
        ];

        if let Some(avg) = avg_duration {
            stats.push(Line::from(vec![Span::raw("")]));
            stats.push(Line::from(vec![Span::styled("Avg Duration: ", Style::default().fg(Color::Cyan)), 
                Span::raw(format!("{} seconds", avg))]));
        }

        if let Some(est_total) = timeline.estimated_total_duration {
            stats.push(Line::from(vec![Span::styled("Est. Total: ", Style::default().fg(Color::Cyan)), 
                Span::raw(format!("{} seconds", est_total))]));
        }

        let stats_paragraph = Paragraph::new(stats)
            .block(Block::default().borders(Borders::ALL).title("Statistics"))
            .wrap(ratatui::widgets::Wrap { trim: true });

        f.render_widget(stats_paragraph, area);
    }

    fn render_controls(&self, f: &mut Frame, area: Rect) {
        let controls = "↑↓: Navigate, Enter: Toggle details, A: Auto-scroll, C/F: Toggle completed/failed";
        let controls_widget = Paragraph::new(controls)
            .block(Block::default().borders(Borders::ALL).title("Controls"))
            .style(Style::default().fg(Color::DarkGray))
            .wrap(ratatui::widgets::Wrap { trim: true });

        f.render_widget(controls_widget, area);
    }

    fn get_status_style(&self, status: &TaskStatus) -> Style {
        match status {
            TaskStatus::Pending => Style::default().fg(Color::Gray),
            TaskStatus::Running => Style::default().fg(Color::Yellow),
            TaskStatus::Completed => Style::default().fg(Color::Green),
            TaskStatus::Failed => Style::default().fg(Color::Red),
            TaskStatus::Cancelled => Style::default().fg(Color::DarkGray),
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> bool {
        if let Some(timeline) = &self.timeline_data {
            match key.code {
                KeyCode::Up => {
                    if self.selected_task_index > 0 {
                        self.selected_task_index -= 1;
                        self.auto_scroll = false;
                    }
                    true
                }
                KeyCode::Down => {
                    if self.selected_task_index < timeline.tasks.len().saturating_sub(1) {
                        self.selected_task_index += 1;
                        self.auto_scroll = false;
                    }
                    true
                }
                KeyCode::Enter => {
                    self.show_details = !self.show_details;
                    true
                }
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    self.auto_scroll = !self.auto_scroll;
                    true
                }
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    self.show_completed = !self.show_completed;
                    true
                }
                KeyCode::Char('f') | KeyCode::Char('F') => {
                    self.show_failed = !self.show_failed;
                    true
                }
                KeyCode::Home => {
                    self.selected_task_index = 0;
                    self.auto_scroll = false;
                    true
                }
                KeyCode::End => {
                    self.selected_task_index = timeline.tasks.len().saturating_sub(1);
                    self.auto_scroll = false;
                    true
                }
                _ => false
            }
        } else {
            false
        }
    }

    pub fn update(&mut self, message: &str) {
        // Handle update messages - could be JSON with new timeline data
        if let Ok(timeline_data) = serde_json::from_str::<TimelineData>(message) {
            self.set_timeline(timeline_data);
        } else if let Ok(task_event) = serde_json::from_str::<TaskEvent>(message) {
            // Update individual task
            self.update_task(task_event);
        }
    }

    pub fn update_task(&mut self, task_event: TaskEvent) {
        if let Some(timeline) = &mut self.timeline_data {
            if let Some(existing_task) = timeline.tasks.iter_mut().find(|t| t.id == task_event.id) {
                *existing_task = task_event;
            } else {
                timeline.tasks.push(task_event);
            }
            
            // Recalculate overall progress
            let completed_tasks = timeline.tasks.iter().filter(|t| matches!(t.status, TaskStatus::Completed)).count();
            let total_tasks = timeline.tasks.len();
            timeline.overall_progress = if total_tasks > 0 {
                ((completed_tasks as f32 / total_tasks as f32) * 100.0) as u16
            } else {
                0
            };
        }
    }

    pub fn has_timeline(&self) -> bool {
        self.timeline_data.is_some()
    }

    pub fn get_selected_task(&self) -> Option<&TaskEvent> {
        self.timeline_data.as_ref()
            .and_then(|timeline| timeline.tasks.get(self.selected_task_index))
    }

    pub fn clear_timeline(&mut self) {
        self.timeline_data = None;
        self.selected_task_index = 0;
        self.scroll_offset = 0;
        self.show_details = false;
        self.elapsed_time = Duration::new(0, 0);
    }

    pub fn get_progress(&self) -> u16 {
        self.timeline_data.as_ref()
            .map(|timeline| timeline.overall_progress)
            .unwrap_or(0)
    }

    pub fn is_completed(&self) -> bool {
        self.get_progress() == 100
    }

    pub fn has_errors(&self) -> bool {
        self.timeline_data.as_ref()
            .map(|timeline| timeline.tasks.iter().any(|t| matches!(t.status, TaskStatus::Failed)))
            .unwrap_or(false)
    }
}