use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::Terminal;
use std::error::Error;
use std::io;
use tokio::sync::mpsc;
use crate::plan_viewer::PlanViewer;
use crate::diff_viewer::DiffViewer;
use crate::action_buttons::ActionButtons;
use orchestrator::EventBus;

pub fn run() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (tx, mut rx) = mpsc::channel(100);
    let mut event_bus = EventBus::new(tx.clone());

    let mut plan_viewer = PlanViewer::new();
    let mut diff_viewer = DiffViewer::new();
    let mut action_buttons = ActionButtons::new();

    loop {
        terminal.draw(|f| {
            let size = f.size();
            let chunks = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([
                    ratatui::layout::Constraint::Percentage(50),
                    ratatui::layout::Constraint::Percentage(30),
                    ratatui::layout::Constraint::Percentage(20),
                ]
                .as_ref())
                .split(size);

            plan_viewer.render(f, chunks[0]);
            diff_viewer.render(f, chunks[1]);
            action_buttons.render(f, chunks[2]);
        })?;

        if event::poll(std::time::Duration::from_millis(500))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => break,
                    KeyCode::Tab => {}, // Handle tab switching
                    KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
                        plan_viewer.handle_key(key.code);
                        diff_viewer.handle_key(key.code);
                        action_buttons.handle_key(key.code);
                    }
                    KeyCode::Enter => action_buttons.activate(),
                    _ => {}
                }
            }
        }

        if let Some(message) = rx.recv().await {
            match message {
                // Handle messages from EventBus
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    Ok(())
}