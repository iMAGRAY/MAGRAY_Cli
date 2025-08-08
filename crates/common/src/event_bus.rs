use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio::time::{timeout, Duration};
use tracing::{debug, info, warn};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Topic(pub &'static str);

#[derive(Debug, Clone)]
pub struct EventEnvelope<T: Clone + Send + Sync + Debug + 'static> {
    pub topic: Topic,
    pub payload: T,
    pub ts_ms: u128,
}

#[derive(Clone)]
pub struct EventBus<T: Clone + Send + Sync + Debug + 'static> {
    inner: Arc<RwLock<Inner<T>>>,
    publish_timeout: Duration,
    subscribe_buffer: usize,
}

struct Inner<T: Clone + Send + Sync + Debug + 'static> {
    topics: HashMap<&'static str, broadcast::Sender<EventEnvelope<T>>>,
}

impl<T: Clone + Send + Sync + Debug + 'static> Default for EventBus<T> {
    fn default() -> Self { Self::new(1000, Duration::from_secs(1)) }
}

impl<T: Clone + Send + Sync + Debug + 'static> EventBus<T> {
    pub fn new(subscribe_buffer: usize, publish_timeout: Duration) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner { topics: HashMap::new() })),
            publish_timeout,
            subscribe_buffer,
        }
    }

    pub async fn ensure_topic(&self, topic: Topic) {
        let mut inner = self.inner.write().await;
        if !inner.topics.contains_key(topic.0) {
            let (tx, _rx) = broadcast::channel(self.subscribe_buffer);
            inner.topics.insert(topic.0, tx);
            info!(target: "event_bus", topic = topic.0, "created topic");
        }
    }

    pub async fn publish(&self, topic: Topic, payload: T) {
        self.ensure_topic(topic.clone()).await;
        let envelope = EventEnvelope { topic: topic.clone(), payload, ts_ms: current_ts_ms() };
        let tx_opt = { self.inner.read().await.topics.get(topic.0).cloned() };
        if let Some(tx) = tx_opt {
            let timeout_dur = self.publish_timeout;
            let send_fut = async move { tx.send(envelope).map(|_| ()) };
            match timeout(timeout_dur, send_fut).await {
                Ok(Ok(())) => {
                    debug!(target: "event_bus", topic = topic.0, "published");
                }
                Ok(Err(_e)) => {
                    warn!(target: "event_bus", topic = topic.0, "no subscribers or lagging");
                }
                Err(_) => {
                    warn!(target: "event_bus", topic = topic.0, "publish timeout");
                }
            }
        } else {
            warn!(target: "event_bus", topic = topic.0, "no topic sender found");
        }
    }

    pub async fn subscribe(&self, topic: Topic) -> broadcast::Receiver<EventEnvelope<T>> {
        self.ensure_topic(topic.clone()).await;
        let rx = self
            .inner
            .read()
            .await
            .topics
            .get(topic.0)
            .expect("topic must exist")
            .subscribe();
        rx
    }
}

fn current_ts_ms() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn publish_subscribe_basic() {
        let bus: EventBus<String> = EventBus::new(8, Duration::from_millis(100));
        let mut rx = bus.subscribe(Topic("test.topic")).await;
        bus.publish(Topic("test.topic"), "hello".to_string()).await;
        let evt = rx.recv().await.expect("should receive");
        assert_eq!(evt.topic.0, "test.topic");
        assert_eq!(evt.payload, "hello".to_string());
    }

    #[tokio::test]
    async fn publish_to_empty_topic_does_not_panic() {
        let bus: EventBus<u64> = EventBus::default();
        // No subscribers
        bus.publish(Topic("no.subscribers"), 42).await;
    }

    #[tokio::test]
    async fn backpressure_and_timeout() {
        let bus: EventBus<u64> = EventBus::new(1, Duration::from_millis(50));
        let mut rx = bus.subscribe(Topic("bp.topic")).await;

        // Fill buffer with one message, receiver not drained yet
        bus.publish(Topic("bp.topic"), 1).await;
        // This publish may hit timeout or warn due to full buffer
        bus.publish(Topic("bp.topic"), 2).await;

        // Drain one and ensure we at least get the first
        let first = rx.recv().await.expect("recv first");
        assert_eq!(first.payload, 1);
    }
}