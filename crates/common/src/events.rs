use crate::event_bus::{EventBus, EventEnvelope, Topic};
use lazy_static::lazy_static;
use tokio::sync::broadcast;
use tokio::time::Duration;

lazy_static! {
    pub static ref GLOBAL_EVENT_BUS: EventBus<serde_json::Value> =
        EventBus::new(4096, Duration::from_millis(250));
}

pub async fn publish(topic: Topic, payload: serde_json::Value) {
    GLOBAL_EVENT_BUS.publish(topic, payload).await;
}

pub async fn subscribe(topic: Topic) -> broadcast::Receiver<EventEnvelope<serde_json::Value>> {
    GLOBAL_EVENT_BUS.subscribe(topic).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn global_bus_publish_subscribe() {
        let mut rx = subscribe(Topic("tool.invoked")).await;
        publish(Topic("tool.invoked"), serde_json::json!({"tool": "file_read", "ok": true}))
            .await;
        let evt = rx.recv().await.expect("receive");
        assert_eq!(evt.topic.0, "tool.invoked");
        assert_eq!(evt.payload["tool"], "file_read");
        assert_eq!(evt.payload["ok"], true);
    }
}