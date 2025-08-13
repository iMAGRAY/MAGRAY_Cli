use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{error::Error, io};
use tokio::sync::broadcast;

use crate::components::{
    action_buttons::ActionButtons, diff_viewer::DiffViewer, plan_viewer::PlanViewer,
};

pub async fn run_app(mut receiver: broadcast::Receiver<String>) -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut plan_viewer = PlanViewer::new();
    let mut diff_viewer = DiffViewer::new();
    let mut action_buttons = ActionButtons::new();

    loop {
        terminal.draw(|f| {
            let size = f.area();
            let chunks = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        ratatui::layout::Constraint::Percentage(40),
                        ratatui::layout::Constraint::Percentage(40),
                        ratatui::layout::Constraint::Percentage(20),
                    ]
                    .as_ref(),
                )
                .split(size);

            plan_viewer.render(f, chunks[0]);
            diff_viewer.render(f, chunks[1]);
            action_buttons.render(f, chunks[2]);
        })?;

        if let Ok(message) = receiver.try_recv() {
            plan_viewer.update(&message);
            diff_viewer.update(&message);
        }

        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('c') => break,
                    KeyCode::Tab => { /* switch focus */ }
                    KeyCode::Up => { /* navigate up */ }
                    KeyCode::Down => { /* navigate down */ }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
