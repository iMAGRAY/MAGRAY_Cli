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
    fn default() -> Self {
        Self::new(1000, Duration::from_secs(1))
    }
}

impl<T: Clone + Send + Sync + Debug + 'static> EventBus<T> {
    pub fn new(subscribe_buffer: usize, publish_timeout: Duration) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner {
                topics: HashMap::new(),
            })),
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
        let envelope = EventEnvelope {
            topic: topic.clone(),
            payload,
            ts_ms: current_ts_ms(),
        };
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
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
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

        match rx.recv().await {
            Ok(evt) => {
                assert!(evt.payload == 1 || evt.payload == 2);
            }
            Err(broadcast::error::RecvError::Lagged(_)) => {
                // After lag, next received should be the latest
                let evt = rx.recv().await.expect("recv after lag");
                assert_eq!(evt.payload, 2);
            }
            Err(e) => panic!("unexpected recv error: {e:?}"),
        }
    }

    #[tokio::test]
    async fn multi_subscribers_receive_all() {
        let bus: EventBus<&'static str> = EventBus::new(8, Duration::from_millis(100));
        let mut rx1 = bus.subscribe(Topic("fanout.topic")).await;
        let mut rx2 = bus.subscribe(Topic("fanout.topic")).await;

        bus.publish(Topic("fanout.topic"), "a").await;
        bus.publish(Topic("fanout.topic"), "b").await;

        let e1a = rx1
            .recv()
            .await
            .expect("Async operation should succeed")
            .payload;
        let e2a = rx2
            .recv()
            .await
            .expect("Async operation should succeed")
            .payload;
        let e1b = rx1
            .recv()
            .await
            .expect("Async operation should succeed")
            .payload;
        let e2b = rx2
            .recv()
            .await
            .expect("Async operation should succeed")
            .payload;

        assert_eq!(e1a, "a");
        assert_eq!(e2a, "a");
        assert_eq!(e1b, "b");
        assert_eq!(e2b, "b");
    }

    #[tokio::test]
    async fn slow_subscriber_does_not_block_fast_one() {
        let bus: EventBus<&'static str> = EventBus::new(1, Duration::from_millis(50));
        let mut rx_fast = bus.subscribe(Topic("nonblock.topic")).await;
        let mut _rx_slow = bus.subscribe(Topic("nonblock.topic")).await;

        // Ensure publish completes quickly (non-blocking semantics)
        let t0 = std::time::Instant::now();
        bus.publish(Topic("nonblock.topic"), "x1").await;
        bus.publish(Topic("nonblock.topic"), "x2").await;
        assert!(t0.elapsed() < std::time::Duration::from_millis(200));

        // Fast receiver should eventually get a message; tolerate lag
        match rx_fast.recv().await {
            Ok(evt) => {
                assert!(evt.payload == "x1" || evt.payload == "x2");
            }
            Err(broadcast::error::RecvError::Lagged(_)) => {
                let evt = rx_fast.recv().await.expect("recv after lag");
                assert_eq!(evt.payload, "x2");
            }
            Err(e) => panic!("unexpected recv error: {e:?}"),
        }
    }

    #[tokio::test]
    async fn unsubscribe_drop_receiver() {
        let bus: EventBus<u8> = EventBus::new(4, Duration::from_millis(50));
        let rx = bus.subscribe(Topic("drop.topic")).await;
        drop(rx);
        // Publish should not panic when no receivers remain
        bus.publish(Topic("drop.topic"), 7).await;
    }

    #[tokio::test]
    async fn multi_topics_independent() {
        let bus: EventBus<&'static str> = EventBus::new(8, Duration::from_millis(100));
        let mut rx_a = bus.subscribe(Topic("topic.a")).await;
        let mut rx_b = bus.subscribe(Topic("topic.b")).await;

        bus.publish(Topic("topic.a"), "A1").await;
        bus.publish(Topic("topic.b"), "B1").await;

        let ea = rx_a
            .recv()
            .await
            .expect("Async operation should succeed")
            .payload;
        let eb = rx_b
            .recv()
            .await
            .expect("Async operation should succeed")
            .payload;
        assert_eq!(ea, "A1");
        assert_eq!(eb, "B1");
    }
}
