use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum TUIEvent {
    Key(KeyEvent),
    Tick,
    Resize(u16, u16),
    OrchestrationUpdate(String),
    PlanGenerated(String),
    ExecutionProgress(String),
    ExecutionComplete(String),
    Error(String),
}

pub struct EventHandler {
    sender: mpsc::Sender<TUIEvent>,
    receiver: mpsc::Receiver<TUIEvent>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (sender, receiver) = mpsc::channel();
        let event_sender = sender.clone();

        thread::spawn(move || {
            let mut last_tick = Instant::now();
            loop {
                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or_else(|| Duration::from_secs(0));

                if crossterm::event::poll(timeout).unwrap() {
                    match crossterm::event::read().unwrap() {
                        Event::Key(key) => {
                            if let Err(_) = event_sender.send(TUIEvent::Key(key)) {
                                break;
                            }
                        }
                        Event::Resize(width, height) => {
                            if let Err(_) = event_sender.send(TUIEvent::Resize(width, height)) {
                                break;
                            }
                        }
                        _ => {}
                    }
                }

                if last_tick.elapsed() >= tick_rate {
                    if let Err(_) = event_sender.send(TUIEvent::Tick) {
                        break;
                    }
                    last_tick = Instant::now();
                }
            }
        });

        EventHandler { sender, receiver }
    }

    pub fn next(&self) -> Result<TUIEvent, mpsc::RecvError> {
        self.receiver.recv()
    }

    pub fn send_orchestration_update(&self, message: String) {
        let _ = self.sender.send(TUIEvent::OrchestrationUpdate(message));
    }

    pub fn send_plan_generated(&self, plan_json: String) {
        let _ = self.sender.send(TUIEvent::PlanGenerated(plan_json));
    }

    pub fn send_execution_progress(&self, progress: String) {
        let _ = self.sender.send(TUIEvent::ExecutionProgress(progress));
    }

    pub fn send_execution_complete(&self, result: String) {
        let _ = self.sender.send(TUIEvent::ExecutionComplete(result));
    }

    pub fn send_error(&self, error: String) {
        let _ = self.sender.send(TUIEvent::Error(error));
    }
}

pub fn should_quit(key: &KeyEvent) -> bool {
    matches!(
        (key.code, key.modifiers),
        (KeyCode::Char('q'), KeyModifiers::NONE)
            | (KeyCode::Char('c'), KeyModifiers::CONTROL)
            | (KeyCode::Esc, KeyModifiers::NONE)
    )
}