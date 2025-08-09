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

    #[tokio::test]
    async fn multiple_subscribers_receive() {
        let mut rx1 = subscribe(Topic("policy.block")).await;
        let mut rx2 = subscribe(Topic("policy.block")).await;
        publish(Topic("policy.block"), serde_json::json!({"tool": "shell_exec"})).await;
        let e1 = rx1.recv().await.unwrap();
        let e2 = rx2.recv().await.unwrap();
        assert_eq!(e1.payload["tool"], "shell_exec");
        assert_eq!(e2.payload["tool"], "shell_exec");
    }

    #[tokio::test]
    async fn topic_isolation() {
        // Use unique test-only topics to avoid interference from concurrent tests
        const FS_T: &str = "test.fs.isolation";
        const TOOL_T: &str = "test.tool.isolation";
        let mut rx_a = subscribe(Topic(FS_T)).await;
        let mut rx_b = subscribe(Topic(TOOL_T)).await;
        publish(Topic(FS_T), serde_json::json!({"op":"write"})).await;
        let ea = rx_a.recv().await.unwrap();
        // give chance, but tool topic should not receive events for FS_T
        let maybe_b = tokio::time::timeout(Duration::from_millis(120), rx_b.recv()).await;
        assert_eq!(ea.topic.0, FS_T);
        assert!(maybe_b.is_err(), "unexpected cross-topic event on {}", TOOL_T);
    }
}