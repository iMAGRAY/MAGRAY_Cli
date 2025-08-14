use crate::components::{
    action_buttons::{ActionButtons, ButtonAction},
    diff_viewer::DiffViewer,
    plan_viewer::PlanViewer,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Copy)]
pub enum AppMode {
    PlanViewing,
    DiffViewing,
    Executing,
    Idle,
}

#[derive(Debug, Clone)]
pub enum FocusedComponent {
    PlanViewer,
    DiffViewer,
    ActionButtons,
}

pub struct AppState {
    pub mode: AppMode,
    pub focused_component: FocusedComponent,
    pub plan_viewer: PlanViewer,
    pub diff_viewer: DiffViewer,
    pub action_buttons: ActionButtons,
    pub status_message: String,
    pub error_message: Option<String>,
    pub should_quit: bool,
    pub orchestration_active: bool,
    pub current_operation: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            mode: AppMode::Idle,
            focused_component: FocusedComponent::PlanViewer,
            plan_viewer: PlanViewer::new(),
            diff_viewer: DiffViewer::new(),
            action_buttons: ActionButtons::new(),
            status_message: "Welcome to MAGRAY TUI. Press 'h' for help.".to_string(),
            error_message: None,
            should_quit: false,
            orchestration_active: false,
            current_operation: None,
        }
    }

    pub fn set_mode(&mut self, mode: AppMode) {
        self.mode = mode;
        match &mode {
            AppMode::PlanViewing => {
                self.status_message = "Plan loaded. Navigate with ↑↓, press Enter for details.".to_string();
                self.focused_component = FocusedComponent::PlanViewer;
            }
            AppMode::DiffViewing => {
                self.status_message = "Diff loaded. Navigate with ↑↓, switch files with ←→.".to_string();
                self.focused_component = FocusedComponent::DiffViewer;
            }
            AppMode::Executing => {
                self.status_message = "Executing plan...".to_string();
                self.focused_component = FocusedComponent::ActionButtons;
            }
            AppMode::Idle => {
                self.status_message = "Ready. Type a command or press 'h' for help.".to_string();
                self.focused_component = FocusedComponent::PlanViewer;
            }
        }
    }

    pub fn set_error(&mut self, error: String) {
        self.error_message = Some(error);
        self.status_message = "Error occurred. Check error panel.".to_string();
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    pub fn set_status(&mut self, message: String) {
        self.status_message = message;
        self.clear_error();
    }

    pub fn cycle_focus(&mut self) {
        self.focused_component = match self.focused_component {
            FocusedComponent::PlanViewer => FocusedComponent::DiffViewer,
            FocusedComponent::DiffViewer => FocusedComponent::ActionButtons,
            FocusedComponent::ActionButtons => FocusedComponent::PlanViewer,
        };

        self.status_message = match self.focused_component {
            FocusedComponent::PlanViewer => "Focused: Plan Viewer".to_string(),
            FocusedComponent::DiffViewer => "Focused: Diff Viewer".to_string(),
            FocusedComponent::ActionButtons => "Focused: Action Buttons".to_string(),
        };
    }

    pub fn start_orchestration(&mut self, operation: String) {
        self.orchestration_active = true;
        self.current_operation = Some(operation.clone());
        self.set_status(format!("Starting orchestration: {}", operation));
    }

    pub fn stop_orchestration(&mut self) {
        self.orchestration_active = false;
        self.current_operation = None;
        self.set_status("Orchestration completed".to_string());
    }

    pub fn handle_button_action(&mut self, action: ButtonAction) -> Option<String> {
        match action {
            ButtonAction::Execute => {
                if self.plan_viewer.has_plan() {
                    self.set_mode(AppMode::Executing);
                    Some("execute_plan".to_string())
                } else {
                    self.set_error("No plan to execute".to_string());
                    None
                }
            }
            ButtonAction::Preview => {
                if self.plan_viewer.has_plan() {
                    self.set_mode(AppMode::DiffViewing);
                    Some("generate_diff".to_string())
                } else {
                    self.set_error("No plan to preview".to_string());
                    None
                }
            }
            ButtonAction::Cancel => {
                self.set_mode(AppMode::Idle);
                self.action_buttons.reset_states();
                Some("cancel_operation".to_string())
            }
            ButtonAction::Modify => {
                if self.plan_viewer.has_plan() {
                    Some("modify_plan".to_string())
                } else {
                    self.set_error("No plan to modify".to_string());
                    None
                }
            }
            ButtonAction::Save => {
                if self.plan_viewer.has_plan() {
                    Some("save_plan".to_string())
                } else {
                    self.set_error("No plan to save".to_string());
                    None
                }
            }
        }
    }

    pub fn is_focused(&self, component: FocusedComponent) -> bool {
        std::mem::discriminant(&self.focused_component) == std::mem::discriminant(&component)
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}