use ratatui::widgets::{Block, Borders};
use ratatui::layout::Rect;
use ratatui::Frame;
use crossterm::event::KeyCode;
use tokio::sync::mpsc::Sender;

pub struct PlanViewer {
    // Internal state
}

impl PlanViewer {
    pub fn new() -> Self {
        PlanViewer {
            // Initialize state
        }
    }

    pub fn render<B: ratatui::backend::Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let block = Block::default().title("Plan Viewer").borders(Borders::ALL);
        f.render_widget(block, area);
    }

    pub fn handle_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up => {},
            KeyCode::Down => {},
            KeyCode::Left => {},
            KeyCode::Right => {},
            _ => {}
        }
    }
}
