use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, Paragraph},
    Frame,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ButtonAction {
    Execute,
    Cancel,
    Modify,
    Preview,
    Save,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionState {
    Ready,
    Processing,
    Completed,
    Failed,
    Disabled,
}

#[derive(Debug, Clone)]
pub struct Button {
    pub action: ButtonAction,
    pub label: String,
    pub shortcut: char,
    pub state: ActionState,
    pub description: String,
}

impl Button {
    pub fn new(action: ButtonAction, label: &str, shortcut: char, description: &str) -> Self {
        Self {
            action,
            label: label.to_string(),
            shortcut,
            state: ActionState::Ready,
            description: description.to_string(),
        }
    }

    pub fn is_enabled(&self) -> bool {
        !matches!(self.state, ActionState::Disabled | ActionState::Processing)
    }
}

pub struct ActionButtons {
    buttons: Vec<Button>,
    selected_button_index: usize,
    show_confirmation: bool,
    confirmation_action: Option<ButtonAction>,
    last_action_result: Option<(ButtonAction, bool)>,
    processing_progress: u16,
}

impl Default for ActionButtons {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionButtons {
    pub fn new() -> Self {
        let buttons = vec![
            Button::new(
                ButtonAction::Execute,
                "Execute",
                'e',
                "Execute the current plan",
            ),
            Button::new(
                ButtonAction::Preview,
                "Preview",
                'p',
                "Preview changes before execution",
            ),
            Button::new(
                ButtonAction::Modify,
                "Modify",
                'm',
                "Modify the current plan",
            ),
            Button::new(
                ButtonAction::Cancel,
                "Cancel",
                'c',
                "Cancel current operation",
            ),
            Button::new(ButtonAction::Save, "Save", 's', "Save current plan"),
        ];

        ActionButtons {
            buttons,
            selected_button_index: 0,
            show_confirmation: false,
            confirmation_action: None,
            last_action_result: None,
            processing_progress: 0,
        }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Buttons
                Constraint::Length(2), // Status/Progress
                Constraint::Min(1),    // Description/Help
            ])
            .split(area);

        // Render buttons
        self.render_buttons(f, chunks[0]);

        // Render status/progress
        self.render_status(f, chunks[1]);

        // Render description and help
        self.render_description(f, chunks[2]);

        // Render confirmation dialog if needed
        if self.show_confirmation {
            self.render_confirmation_dialog(f, area);
        }
    }

    fn render_buttons(&self, f: &mut Frame, area: Rect) {
        let button_width = area.width / self.buttons.len() as u16;
        let mut x = area.x;

        for (i, button) in self.buttons.iter().enumerate() {
            let button_area = Rect {
                x,
                y: area.y,
                width: button_width,
                height: area.height,
            };

            let is_selected = i == self.selected_button_index;
            let style = self.get_button_style(button, is_selected);
            let border_style = if is_selected {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Gray)
            };

            let button_text = format!("[{}] {}", button.shortcut.to_uppercase(), button.label);

            let button_widget = Paragraph::new(button_text)
                .style(style)
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(border_style),
                );

            f.render_widget(button_widget, button_area);
            x += button_width;
        }
    }

    fn render_status(&self, f: &mut Frame, area: Rect) {
        let selected_button = &self.buttons[self.selected_button_index];

        match selected_button.state {
            ActionState::Processing => {
                let progress_text = format!("Processing... {}%", self.processing_progress);
                let gauge = Gauge::default()
                    .block(Block::default().borders(Borders::ALL).title("Status"))
                    .gauge_style(Style::default().fg(Color::Yellow))
                    .percent(self.processing_progress)
                    .label(progress_text);
                f.render_widget(gauge, area);
            }
            _ => {
                let status_text = match selected_button.state {
                    ActionState::Ready => "Ready to execute".to_string(),
                    ActionState::Completed => "Action completed successfully".to_string(),
                    ActionState::Failed => "Action failed".to_string(),
                    ActionState::Disabled => "Action disabled".to_string(),
                    ActionState::Processing => unreachable!(),
                };

                let status_color = match selected_button.state {
                    ActionState::Ready => Color::Green,
                    ActionState::Completed => Color::Green,
                    ActionState::Failed => Color::Red,
                    ActionState::Disabled => Color::Gray,
                    ActionState::Processing => Color::Yellow,
                };

                let status_widget = Paragraph::new(status_text)
                    .style(Style::default().fg(status_color))
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL).title("Status"));
                f.render_widget(status_widget, area);
            }
        }
    }

    fn render_description(&self, f: &mut Frame, area: Rect) {
        let selected_button = &self.buttons[self.selected_button_index];

        let mut description_text = vec![
            Line::from(vec![Span::styled(
                "Description:",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::raw(selected_button.description.clone())]),
            Line::from(vec![Span::raw("")]),
            Line::from(vec![Span::styled(
                "Controls:",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::raw(
                "Tab: Switch buttons, Enter: Execute selected, Esc: Cancel",
            )]),
        ];

        if let Some((action, success)) = &self.last_action_result {
            let result_text = if *success {
                format!("Last action: {action:?} completed successfully")
            } else {
                format!("Last action: {action:?} failed")
            };

            let result_color = if *success { Color::Green } else { Color::Red };
            description_text.push(Line::from(vec![Span::raw("")]));
            description_text.push(Line::from(vec![Span::styled(
                result_text,
                Style::default().fg(result_color),
            )]));
        }

        let description_widget = Paragraph::new(description_text)
            .block(Block::default().borders(Borders::ALL).title("Information"))
            .wrap(ratatui::widgets::Wrap { trim: true });

        f.render_widget(description_widget, area);
    }

    fn render_confirmation_dialog(&self, f: &mut Frame, area: Rect) {
        let dialog_width = 50;
        let dialog_height = 8;
        let dialog_area = Rect {
            x: (area.width.saturating_sub(dialog_width)) / 2,
            y: (area.height.saturating_sub(dialog_height)) / 2,
            width: dialog_width,
            height: dialog_height,
        };

        // Clear the background
        f.render_widget(Clear, dialog_area);

        let action_name = self
            .confirmation_action
            .as_ref()
            .map(|a| format!("{a:?}"))
            .unwrap_or_else(|| "Action".to_string());

        let confirmation_text = vec![
            Line::from(vec![Span::styled(
                "Confirmation Required",
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Yellow),
            )]),
            Line::from(vec![Span::raw("")]),
            Line::from(vec![Span::raw(format!(
                "Are you sure you want to {}?",
                action_name.to_lowercase()
            ))]),
            Line::from(vec![Span::raw("")]),
            Line::from(vec![
                Span::styled(
                    "Y",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(": Yes  "),
                Span::styled(
                    "N",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::raw(": No"),
            ]),
        ];

        let dialog = Paragraph::new(confirmation_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title("Confirm Action"),
            )
            .alignment(Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true });

        f.render_widget(dialog, dialog_area);
    }

    fn get_button_style(&self, button: &Button, is_selected: bool) -> Style {
        let base_style = match button.state {
            ActionState::Ready => {
                if is_selected {
                    Style::default().fg(Color::Black).bg(Color::Green)
                } else {
                    Style::default().fg(Color::Green)
                }
            }
            ActionState::Processing => Style::default().fg(Color::Yellow),
            ActionState::Completed => Style::default().fg(Color::Green),
            ActionState::Failed => Style::default().fg(Color::Red),
            ActionState::Disabled => Style::default().fg(Color::DarkGray),
        };

        if is_selected && button.is_enabled() {
            base_style.add_modifier(Modifier::BOLD)
        } else {
            base_style
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Option<ButtonAction> {
        if self.show_confirmation {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    let action = self.confirmation_action.take();
                    self.show_confirmation = false;
                    return action;
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.show_confirmation = false;
                    self.confirmation_action = None;
                }
                _ => {}
            }
            return None;
        }

        match key.code {
            KeyCode::Tab => {
                self.selected_button_index = (self.selected_button_index + 1) % self.buttons.len();
                None
            }
            KeyCode::BackTab => {
                self.selected_button_index = if self.selected_button_index == 0 {
                    self.buttons.len() - 1
                } else {
                    self.selected_button_index - 1
                };
                None
            }
            KeyCode::Enter => {
                let selected_button = &self.buttons[self.selected_button_index];
                if selected_button.is_enabled() {
                    self.request_confirmation(selected_button.action.clone());
                }
                None
            }
            KeyCode::Char(c) => {
                // Handle shortcut keys
                for button in &self.buttons {
                    if button.shortcut == c.to_ascii_lowercase() && button.is_enabled() {
                        self.request_confirmation(button.action.clone());
                        break;
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn request_confirmation(&mut self, action: ButtonAction) {
        // Some actions require confirmation
        match action {
            ButtonAction::Execute | ButtonAction::Cancel => {
                self.confirmation_action = Some(action);
                self.show_confirmation = true;
            }
            _ => {
                // Direct execution for less critical actions
                // This would be handled by the parent component
            }
        }
    }

    pub fn set_button_state(&mut self, action: ButtonAction, state: ActionState) {
        if let Some(button) = self.buttons.iter_mut().find(|b| b.action == action) {
            button.state = state;
        }
    }

    pub fn set_processing_progress(&mut self, progress: u16) {
        self.processing_progress = progress.min(100);
    }

    pub fn set_action_result(&mut self, action: ButtonAction, success: bool) {
        self.last_action_result = Some((action.clone(), success));
        self.set_button_state(
            action,
            if success {
                ActionState::Completed
            } else {
                ActionState::Failed
            },
        );
    }

    pub fn reset_states(&mut self) {
        for button in &mut self.buttons {
            button.state = ActionState::Ready;
        }
        self.last_action_result = None;
        self.processing_progress = 0;
    }

    pub fn enable_button(&mut self, action: ButtonAction) {
        self.set_button_state(action, ActionState::Ready);
    }

    pub fn disable_button(&mut self, action: ButtonAction) {
        self.set_button_state(action, ActionState::Disabled);
    }

    pub fn get_selected_action(&self) -> ButtonAction {
        self.buttons[self.selected_button_index].action.clone()
    }

    pub fn is_processing(&self) -> bool {
        self.buttons
            .iter()
            .any(|b| matches!(b.state, ActionState::Processing))
    }
}
