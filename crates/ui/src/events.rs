use tokio::sync::mpsc;

pub enum Event {
    // Define your events here
}

pub struct EventHandler {
    pub sender: mpsc::Sender<Event>,
    pub receiver: mpsc::Receiver<Event>,
}

impl EventHandler {
    pub fn new() -> EventHandler {
        let (sender, receiver) = mpsc::channel(100);
        EventHandler {
            sender,
            receiver,
        }
    }

    pub async fn handle_events(&mut self) {
        while let Some(event) = self.receiver.recv().await {
            match event {
                // Handle events
            }
        }
    }
}
