use crate::event_bus::{EventBus, EventEnvelope, Topic};
use lazy_static::lazy_static;
use std::collections::HashMap;
use tokio::sync::{broadcast, Mutex};
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

// Tool metrics aggregator
#[derive(Default)]
struct ToolMetricsInner {
    invocations: HashMap<String, u64>,
    successes: HashMap<String, u64>,
    asks: HashMap<String, u64>,
    denies: HashMap<String, u64>,
}

lazy_static! {
    static ref TOOL_METRICS: Mutex<ToolMetricsInner> = Mutex::new(ToolMetricsInner::default());
}

pub async fn start_tool_metrics_aggregator() {
    // Subscribe to relevant topics
    let mut rx_invoked = subscribe(Topic("tool.invoked")).await;
    let mut rx_policy = subscribe(Topic("policy.block")).await;
    tokio::spawn(async move {
        loop {
            tokio::select! {
                Ok(env) = rx_invoked.recv() => {
                    let tool = env.payload.get("tool").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let ok = env.payload.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
                    let mut m = TOOL_METRICS.lock().await;
                    *m.invocations.entry(tool.clone()).or_insert(0) += 1;
                    if ok { *m.successes.entry(tool.clone()).or_insert(0) += 1; }
                }
                Ok(env) = rx_policy.recv() => {
                    let tool = env.payload.get("tool").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let action = env.payload.get("action").and_then(|v| v.as_str()).unwrap_or("deny");
                    let mut m = TOOL_METRICS.lock().await;
                    match action {
                        "ask" => *m.asks.entry(tool).or_insert(0) += 1,
                        _ => *m.denies.entry(tool).or_insert(0) += 1,
                    }
                }
                else => { break; }
            }
        }
    });
}

pub async fn tool_metrics_snapshot() -> serde_json::Value {
    let m = TOOL_METRICS.lock().await;
    // Build merged view per tool
    let mut tools: HashMap<String, serde_json::Value> = HashMap::new();
    for (k, v) in &m.invocations {
        tools.insert(k.clone(), serde_json::json!({"invocations": v, "successes": m.successes.get(k).copied().unwrap_or(0), "asks": m.asks.get(k).copied().unwrap_or(0), "denies": m.denies.get(k).copied().unwrap_or(0)}));
    }
    for (k, v) in &m.successes {
        tools.entry(k.clone()).and_modify(|e| { if let Some(obj)=e.as_object_mut(){ obj.insert("successes".into(), serde_json::json!(v)); }}).or_insert(serde_json::json!({"invocations": 0, "successes": v, "asks": m.asks.get(k).copied().unwrap_or(0), "denies": m.denies.get(k).copied().unwrap_or(0)}));
    }
    for (k, v) in &m.asks {
        tools.entry(k.clone()).and_modify(|e| { if let Some(obj)=e.as_object_mut(){ obj.insert("asks".into(), serde_json::json!(v)); }}).or_insert(serde_json::json!({"invocations": 0, "successes": m.successes.get(k).copied().unwrap_or(0), "asks": v, "denies": m.denies.get(k).copied().unwrap_or(0)}));
    }
    for (k, v) in &m.denies {
        tools.entry(k.clone()).and_modify(|e| { if let Some(obj)=e.as_object_mut(){ obj.insert("denies".into(), serde_json::json!(v)); }}).or_insert(serde_json::json!({"invocations": 0, "successes": m.successes.get(k).copied().unwrap_or(0), "asks": m.asks.get(k).copied().unwrap_or(0), "denies": v}));
    }
    serde_json::json!({"tools": tools})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn global_bus_publish_subscribe() {
        let mut rx = subscribe(Topic("tool.invoked")).await;
        publish(
            Topic("tool.invoked"),
            serde_json::json!({"tool": "file_read", "ok": true}),
        )
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
        publish(
            Topic("policy.block"),
            serde_json::json!({"tool": "shell_exec"}),
        )
        .await;
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
        let maybe_b = tokio::time::timeout(Duration::from_millis(120), rx_b.recv()).await;
        assert_eq!(ea.topic.0, FS_T);
        assert!(
            maybe_b.is_err(),
            "unexpected cross-topic event on {}",
            TOOL_T
        );
    }

    #[tokio::test]
    async fn tool_metrics_collects_invokes_and_blocks() {
        start_tool_metrics_aggregator().await;
        publish(
            Topic("tool.invoked"),
            serde_json::json!({"tool":"file_read","success":true}),
        )
        .await;
        publish(
            Topic("policy.block"),
            serde_json::json!({"tool":"shell_exec","action":"deny"}),
        )
        .await;
        publish(
            Topic("policy.block"),
            serde_json::json!({"tool":"web_search","action":"ask"}),
        )
        .await;
        // allow event loop to process
        tokio::time::sleep(Duration::from_millis(50)).await;
        let snap = tool_metrics_snapshot().await;
        let tools = snap["tools"].as_object().unwrap();
        // Use lower-bound assertions due to possible concurrent events in test environment
        assert!(
            tools.get("file_read").unwrap()["invocations"]
                .as_u64()
                .unwrap_or(0)
                >= 1
        );
        assert!(
            tools.get("file_read").unwrap()["successes"]
                .as_u64()
                .unwrap_or(0)
                >= 1
        );
        assert!(
            tools.get("shell_exec").unwrap()["denies"]
                .as_u64()
                .unwrap_or(0)
                >= 1
        );
        assert!(
            tools.get("web_search").unwrap()["asks"]
                .as_u64()
                .unwrap_or(0)
                >= 1
        );
    }
}
