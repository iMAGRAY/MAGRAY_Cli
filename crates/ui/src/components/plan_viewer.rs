use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionStep {
    pub id: String,
    pub description: String,
    pub details: String,
    pub status: StepStatus,
    pub dependencies: Vec<String>,
    pub tools: Vec<String>,
    pub estimated_duration: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPlan {
    pub id: String,
    pub title: String,
    pub description: String,
    pub steps: Vec<ActionStep>,
    pub created_at: String,
}

pub struct PlanViewer {
    action_plan: Option<ActionPlan>,
    selected_step_index: usize,
    scroll_offset: usize,
    show_details: bool,
    expanded_steps: Vec<bool>,
}

impl Default for PlanViewer {
    fn default() -> Self {
        Self::new()
    }
}

impl PlanViewer {
    pub fn new() -> Self {
        PlanViewer {
            action_plan: None,
            selected_step_index: 0,
            scroll_offset: 0,
            show_details: false,
            expanded_steps: Vec::new(),
        }
    }

    pub fn set_plan(&mut self, plan: ActionPlan) {
        self.expanded_steps = vec![false; plan.steps.len()];
        self.selected_step_index = 0;
        self.scroll_offset = 0;
        self.action_plan = Some(plan);
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        if let Some(plan) = &self.action_plan {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);

            // Render plan tree
            self.render_plan_tree(f, chunks[0], plan);

            // Render step details if selected
            if self.show_details && self.selected_step_index < plan.steps.len() {
                self.render_step_details(f, chunks[1], &plan.steps[self.selected_step_index]);
            } else {
                self.render_plan_overview(f, chunks[1], plan);
            }
        } else {
            // Render empty state
            let empty_block = Block::default()
                .borders(Borders::ALL)
                .title("Plan Viewer - No Plan Loaded");
            let empty_text = Paragraph::new("No action plan available.\nPress 'L' to load a plan.")
                .block(empty_block)
                .style(Style::default().fg(Color::Gray));
            f.render_widget(empty_text, area);
        }
    }

    fn render_plan_tree(&self, f: &mut Frame, area: Rect, plan: &ActionPlan) {
        let items: Vec<ListItem> = plan
            .steps
            .iter()
            .enumerate()
            .map(|(i, step)| {
                let style = self.get_step_style(&step.status, i == self.selected_step_index);
                let prefix = if *self.expanded_steps.get(i).unwrap_or(&false) {
                    "▼"
                } else {
                    "▶"
                };

                let icon = match step.status {
                    StepStatus::Pending => "⏳",
                    StepStatus::InProgress => "🔄",
                    StepStatus::Completed => "✅",
                    StepStatus::Failed => "❌",
                };

                let content = format!("{} {} {} {}", prefix, icon, i + 1, step.description);
                ListItem::new(content).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(format!(
                "Plan: {} ({} steps)",
                plan.title,
                plan.steps.len()
            )))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        let mut state = ratatui::widgets::ListState::default();
        state.select(Some(self.selected_step_index));
        f.render_stateful_widget(list, area, &mut state);

        // Render navigation help
        let help_area = Rect {
            x: area.x,
            y: area.y + area.height - 3,
            width: area.width,
            height: 2,
        };

        let help_text = Paragraph::new("↑↓: Navigate, Enter: Toggle details, Space: Expand")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::TOP));
        f.render_widget(Clear, help_area);
        f.render_widget(help_text, help_area);
    }

    fn render_step_details(&self, f: &mut Frame, area: Rect, step: &ActionStep) {
        let details = vec![
            Line::from(vec![Span::styled(
                "Step Details",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::raw("")]),
            Line::from(vec![
                Span::styled("ID: ", Style::default().fg(Color::Cyan)),
                Span::raw(&step.id),
            ]),
            Line::from(vec![
                Span::styled("Description: ", Style::default().fg(Color::Cyan)),
                Span::raw(&step.description),
            ]),
            Line::from(vec![
                Span::styled("Details: ", Style::default().fg(Color::Cyan)),
                Span::raw(&step.details),
            ]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!("{:?}", step.status),
                    self.get_status_color(&step.status),
                ),
            ]),
            Line::from(vec![Span::raw("")]),
        ];

        let mut all_details = details;

        if !step.dependencies.is_empty() {
            all_details.push(Line::from(vec![Span::styled(
                "Dependencies:",
                Style::default().fg(Color::Cyan),
            )]));
            for dep in &step.dependencies {
                all_details.push(Line::from(vec![Span::raw(format!("  • {dep}"))]));
            }
            all_details.push(Line::from(vec![Span::raw("")]));
        }

        if !step.tools.is_empty() {
            all_details.push(Line::from(vec![Span::styled(
                "Tools:",
                Style::default().fg(Color::Cyan),
            )]));
            for tool in &step.tools {
                all_details.push(Line::from(vec![Span::raw(format!("  • {tool}"))]));
            }
        }

        if let Some(duration) = step.estimated_duration {
            all_details.push(Line::from(vec![Span::raw("")]));
            all_details.push(Line::from(vec![
                Span::styled("Estimated Duration: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{duration} seconds")),
            ]));
        }

        let details_paragraph = Paragraph::new(all_details)
            .block(Block::default().borders(Borders::ALL).title("Step Details"))
            .wrap(ratatui::widgets::Wrap { trim: true });

        f.render_widget(details_paragraph, area);
    }

    fn render_plan_overview(&self, f: &mut Frame, area: Rect, plan: &ActionPlan) {
        let completed = plan
            .steps
            .iter()
            .filter(|s| matches!(s.status, StepStatus::Completed))
            .count();
        let in_progress = plan
            .steps
            .iter()
            .filter(|s| matches!(s.status, StepStatus::InProgress))
            .count();
        let failed = plan
            .steps
            .iter()
            .filter(|s| matches!(s.status, StepStatus::Failed))
            .count();
        let pending = plan
            .steps
            .iter()
            .filter(|s| matches!(s.status, StepStatus::Pending))
            .count();

        let overview = vec![
            Line::from(vec![Span::styled(
                "Plan Overview",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::raw("")]),
            Line::from(vec![
                Span::styled("Title: ", Style::default().fg(Color::Cyan)),
                Span::raw(&plan.title),
            ]),
            Line::from(vec![
                Span::styled("Description: ", Style::default().fg(Color::Cyan)),
                Span::raw(&plan.description),
            ]),
            Line::from(vec![
                Span::styled("Created: ", Style::default().fg(Color::Cyan)),
                Span::raw(&plan.created_at),
            ]),
            Line::from(vec![Span::raw("")]),
            Line::from(vec![Span::styled(
                "Progress Summary:",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                format!("✅ Completed: {completed}"),
                Style::default().fg(Color::Green),
            )]),
            Line::from(vec![Span::styled(
                format!("🔄 In Progress: {in_progress}"),
                Style::default().fg(Color::Yellow),
            )]),
            Line::from(vec![Span::styled(
                format!("⏳ Pending: {pending}"),
                Style::default().fg(Color::White),
            )]),
            Line::from(vec![Span::styled(
                format!("❌ Failed: {failed}"),
                Style::default().fg(Color::Red),
            )]),
        ];

        let overview_paragraph = Paragraph::new(overview)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Plan Overview"),
            )
            .wrap(ratatui::widgets::Wrap { trim: true });

        f.render_widget(overview_paragraph, area);
    }

    fn get_step_style(&self, status: &StepStatus, is_selected: bool) -> Style {
        let mut style = match status {
            StepStatus::Pending => Style::default().fg(Color::White),
            StepStatus::InProgress => Style::default().fg(Color::Yellow),
            StepStatus::Completed => Style::default().fg(Color::Green),
            StepStatus::Failed => Style::default().fg(Color::Red),
        };

        if is_selected {
            style = style.add_modifier(Modifier::BOLD).bg(Color::DarkGray);
        }

        style
    }

    fn get_status_color(&self, status: &StepStatus) -> Style {
        match status {
            StepStatus::Pending => Style::default().fg(Color::White),
            StepStatus::InProgress => Style::default().fg(Color::Yellow),
            StepStatus::Completed => Style::default().fg(Color::Green),
            StepStatus::Failed => Style::default().fg(Color::Red),
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> bool {
        if let Some(plan) = &self.action_plan {
            match key.code {
                KeyCode::Up => {
                    if self.selected_step_index > 0 {
                        self.selected_step_index -= 1;
                    }
                    true
                }
                KeyCode::Down => {
                    if self.selected_step_index < plan.steps.len().saturating_sub(1) {
                        self.selected_step_index += 1;
                    }
                    true
                }
                KeyCode::Enter => {
                    self.show_details = !self.show_details;
                    true
                }
                KeyCode::Char(' ') => {
                    if self.selected_step_index < self.expanded_steps.len() {
                        self.expanded_steps[self.selected_step_index] =
                            !self.expanded_steps[self.selected_step_index];
                    }
                    true
                }
                KeyCode::Home => {
                    self.selected_step_index = 0;
                    true
                }
                KeyCode::End => {
                    self.selected_step_index = plan.steps.len().saturating_sub(1);
                    true
                }
                _ => false,
            }
        } else {
            false
        }
    }

    pub fn update(&mut self, message: &str) {
        // Handle update messages - could be JSON with new plan data
        if let Ok(plan) = serde_json::from_str::<ActionPlan>(message) {
            self.set_plan(plan);
        }
    }

    pub fn get_selected_step(&self) -> Option<&ActionStep> {
        self.action_plan
            .as_ref()
            .and_then(|plan| plan.steps.get(self.selected_step_index))
    }

    pub fn has_plan(&self) -> bool {
        self.action_plan.is_some()
    }

    pub fn clear_plan(&mut self) {
        self.action_plan = None;
        self.selected_step_index = 0;
        self.scroll_offset = 0;
        self.show_details = false;
        self.expanded_steps.clear();
    }
}
