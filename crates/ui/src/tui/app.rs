use super::{
    events::{should_quit, EventHandler, TUIEvent},
    state::{AppMode, AppState, FocusedComponent},
};
use crate::components::action_buttons::ButtonAction;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::io::{self, Stdout};
use std::time::Duration;

type Backend = CrosstermBackend<Stdout>;

pub struct TUIApp {
    terminal: Terminal<Backend>,
    event_handler: EventHandler,
    state: AppState,
}

impl TUIApp {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        let event_handler = EventHandler::new(Duration::from_millis(100));
        let state = AppState::new();

        Ok(TUIApp {
            terminal,
            event_handler,
            state,
        })
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        while !self.state.should_quit {
            // Draw UI
            let state = &mut self.state;
            self.terminal.draw(|f| Self::render_ui(f, state))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn render_ui(f: &mut Frame, state: &mut AppState) {
        let size = f.area();

        // Create main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                [
                    Constraint::Length(3), // Header
                    Constraint::Min(1),    // Main content
                    Constraint::Length(3), // Status bar
                ]
                .as_ref(),
            )
            .split(size);

        // Render header
        Self::render_header(f, chunks[0], state);

        // Create main content layout
        let content_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Percentage(45), // Plan viewer
                    Constraint::Percentage(35), // Diff viewer
                    Constraint::Percentage(20), // Action buttons
                ]
                .as_ref(),
            )
            .split(chunks[1]);

        // Render components with focus indication
        Self::render_plan_viewer(f, content_chunks[0], state);
        Self::render_diff_viewer(f, content_chunks[1], state);
        Self::render_action_buttons(f, content_chunks[2], state);

        // Render status bar
        Self::render_status_bar(f, chunks[2], state);

        // Render help dialog if needed
        if matches!(state.mode, AppMode::Idle) {
            // Could show help overlay here
        }
    }

    fn render_header(f: &mut Frame, area: Rect, state: &AppState) {
        let mode_text = format!("Mode: {:?}", state.mode);
        let orchestration_text = if state.orchestration_active {
            format!(
                " | Orchestration: Active ({})",
                state.current_operation.as_deref().unwrap_or("Unknown")
            )
        } else {
            " | Orchestration: Idle".to_string()
        };

        let title = Line::from(vec![
            Span::styled(
                "MAGRAY CLI - TUI Interface",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" | "),
            Span::styled(mode_text, Style::default().fg(Color::Yellow)),
            Span::styled(
                orchestration_text,
                Style::default().fg(if state.orchestration_active {
                    Color::Green
                } else {
                    Color::Gray
                }),
            ),
        ]);

        let header = Paragraph::new(title)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::White));

        f.render_widget(header, area);
    }

    fn render_plan_viewer(f: &mut Frame, area: Rect, state: &mut AppState) {
        let is_focused = state.is_focused(FocusedComponent::PlanViewer);

        // Create a block with different style based on focus
        let block = if is_focused {
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title("Plan Viewer (Focused)")
        } else {
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Gray))
                .title("Plan Viewer")
        };

        // Render the plan viewer content within the block
        let inner_area = block.inner(area);
        f.render_widget(block, area);
        state.plan_viewer.render(f, inner_area);
    }

    fn render_diff_viewer(f: &mut Frame, area: Rect, state: &mut AppState) {
        let is_focused = state.is_focused(FocusedComponent::DiffViewer);

        let block = if is_focused {
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title("Diff Viewer (Focused)")
        } else {
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Gray))
                .title("Diff Viewer")
        };

        let inner_area = block.inner(area);
        f.render_widget(block, area);
        state.diff_viewer.render(f, inner_area);
    }

    fn render_action_buttons(f: &mut Frame, area: Rect, state: &mut AppState) {
        let is_focused = state.is_focused(FocusedComponent::ActionButtons);

        let block = if is_focused {
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title("Actions (Focused)")
        } else {
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Gray))
                .title("Actions")
        };

        let inner_area = block.inner(area);
        f.render_widget(block, area);
        state.action_buttons.render(f, inner_area);
    }

    fn render_status_bar(f: &mut Frame, area: Rect, state: &AppState) {
        let status_text = if let Some(ref error) = state.error_message {
            Line::from(vec![
                Span::styled(
                    "ERROR: ",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::styled(error.clone(), Style::default().fg(Color::Red)),
            ])
        } else {
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Cyan)),
                Span::raw(&state.status_message),
                Span::raw(" | "),
                Span::styled(
                    "Press 'Tab' to switch focus, 'h' for help, 'q' to quit",
                    Style::default().fg(Color::DarkGray),
                ),
            ])
        };

        let status = Paragraph::new(status_text)
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .style(Style::default().fg(Color::White));

        f.render_widget(status, area);
    }

    fn handle_events(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match self.event_handler.next()? {
            TUIEvent::Key(key) => {
                if should_quit(&key) {
                    self.state.quit();
                    return Ok(());
                }

                // Handle global keys
                match key.code {
                    KeyCode::Tab => {
                        self.state.cycle_focus();
                        return Ok(());
                    }
                    KeyCode::Char('h') => {
                        self.show_help();
                        return Ok(());
                    }
                    KeyCode::F(5) => {
                        self.refresh_ui();
                        return Ok(());
                    }
                    _ => {}
                }

                // Handle component-specific keys
                match self.state.focused_component {
                    FocusedComponent::PlanViewer => {
                        self.state.plan_viewer.handle_key_event(key);
                    }
                    FocusedComponent::DiffViewer => {
                        self.state.diff_viewer.handle_key_event(key);
                    }
                    FocusedComponent::ActionButtons => {
                        if let Some(action) = self.state.action_buttons.handle_key_event(key) {
                            self.handle_button_action(action);
                        }
                    }
                }
            }
            TUIEvent::Tick => {
                // Handle periodic updates
            }
            TUIEvent::Resize(width, height) => {
                self.terminal.resize(Rect::new(0, 0, width, height))?;
            }
            TUIEvent::OrchestrationUpdate(message) => {
                self.state.set_status(format!("Orchestration: {}", message));
            }
            TUIEvent::PlanGenerated(plan_json) => {
                self.state.plan_viewer.update(&plan_json);
                self.state.set_mode(AppMode::PlanViewing);
                self.state
                    .set_status("Plan generated successfully".to_string());
            }
            TUIEvent::ExecutionProgress(progress) => {
                self.state.set_status(format!("Execution: {}", progress));
            }
            TUIEvent::ExecutionComplete(result) => {
                self.state.stop_orchestration();
                self.state
                    .set_status(format!("Execution complete: {}", result));
            }
            TUIEvent::Error(error) => {
                self.state.set_error(error);
            }
        }

        Ok(())
    }

    fn handle_button_action(&mut self, action: ButtonAction) {
        if let Some(command) = self.state.handle_button_action(action.clone()) {
            // Here we would send the command to the orchestrator
            // For now, we'll simulate the command execution
            self.simulate_command_execution(command);
        }
    }

    fn simulate_command_execution(&mut self, command: String) {
        // This would be replaced with actual orchestrator integration
        match command.as_str() {
            "execute_plan" => {
                self.state.start_orchestration("Executing plan".to_string());
                // In real implementation: send to orchestrator
            }
            "generate_diff" => {
                self.state.set_status("Generating diff...".to_string());
                // In real implementation: generate diff from plan
            }
            "cancel_operation" => {
                self.state.stop_orchestration();
            }
            "modify_plan" => {
                self.state
                    .set_status("Plan modification not yet implemented".to_string());
            }
            "save_plan" => {
                self.state.set_status("Plan saved (simulation)".to_string());
            }
            _ => {
                self.state
                    .set_error(format!("Unknown command: {}", command));
            }
        }
    }

    fn show_help(&mut self) {
        self.state.set_status(
            "Help: Tab=Focus, ↑↓=Navigate, Enter=Select, Space=Expand, q=Quit".to_string(),
        );
    }

    fn refresh_ui(&mut self) {
        self.state.set_status("UI refreshed".to_string());
    }

    pub fn get_event_handler(&self) -> &EventHandler {
        &self.event_handler
    }

    pub fn load_plan(&mut self, plan_json: String) {
        self.state.plan_viewer.update(&plan_json);
        self.state.set_mode(AppMode::PlanViewing);
    }

    pub fn load_diff(&mut self, diff_json: String) {
        self.state.diff_viewer.update(&diff_json);
        self.state.set_mode(AppMode::DiffViewing);
    }

    pub fn set_orchestration_active(&mut self, active: bool, operation: Option<String>) {
        if active {
            if let Some(op) = operation {
                self.state.start_orchestration(op);
            }
        } else {
            self.state.stop_orchestration();
        }
    }
}

impl Drop for TUIApp {
    fn drop(&mut self) {
        // Cleanup terminal
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = self.terminal.show_cursor();
    }
}
