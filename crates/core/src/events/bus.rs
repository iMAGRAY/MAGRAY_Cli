//! EventBus implementation with backpressure and resilience

use super::*;
use std::time::Instant;
use tokio::sync::broadcast;
use tokio::time::{timeout, Duration};
use tracing::{debug, error, info, warn};

/// High-performance EventBus implementation
pub struct EventBus {
    /// Topic-based subscription management
    subscriptions: Arc<RwLock<HashMap<String, Vec<Arc<dyn EventHandler>>>>>,

    /// Event publisher for broadcasting
    publisher: broadcast::Sender<Event>,

    /// Backpressure configuration
    config: BackpressureConfig,

    /// Statistics tracking
    stats: Arc<RwLock<EventBusStats>>,

    /// Background task handles for cleanup
    _background_tasks: Vec<tokio::task::JoinHandle<()>>,
}

impl EventBus {
    /// Create new EventBus with configuration
    pub fn new(config: BackpressureConfig) -> Self {
        let (publisher, _) = broadcast::channel(config.max_queue_size);

        let stats = Arc::new(RwLock::new(EventBusStats {
            total_events_published: 0,
            total_events_delivered: 0,
            failed_deliveries: 0,
            active_subscriptions: 0,
            queue_sizes: HashMap::new(),
            slow_consumers: Vec::new(),
        }));

        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            publisher,
            config,
            stats,
            _background_tasks: Vec::new(),
        }
    }

    /// Create EventBus with default configuration
    pub fn default() -> Self {
        Self::new(BackpressureConfig::default())
    }

    /// Start background monitoring and cleanup tasks
    pub async fn start_background_tasks(&mut self) {
        let stats_monitor = self.start_stats_monitoring();
        let slow_consumer_detector = self.start_slow_consumer_detection();

        self._background_tasks.push(stats_monitor);
        self._background_tasks.push(slow_consumer_detector);
    }

    /// Background task for statistics monitoring
    fn start_stats_monitoring(&self) -> tokio::task::JoinHandle<()> {
        let stats = Arc::clone(&self.stats);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));

            loop {
                interval.tick().await;

                let stats = stats.read().await;
                info!(
                    "EventBus stats: published={}, delivered={}, failed={}",
                    stats.total_events_published,
                    stats.total_events_delivered,
                    stats.failed_deliveries
                );
            }
        })
    }

    /// Background task for detecting slow consumers
    fn start_slow_consumer_detection(&self) -> tokio::task::JoinHandle<()> {
        let stats = Arc::clone(&self.stats);
        let _threshold_ms = self.config.slow_consumer_threshold_ms;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));

            loop {
                interval.tick().await;

                // Monitor queue sizes and detect slow consumers
                let mut stats = stats.write().await;
                let slow_topics: Vec<String> = stats
                    .queue_sizes
                    .iter()
                    .filter(|(_, &size)| size > 1000) // Large queue indicates slow consumer
                    .map(|(topic, _)| topic.clone())
                    .collect();

                if !slow_topics.is_empty() {
                    warn!("Detected slow consumers on topics: {:?}", slow_topics);
                    stats.slow_consumers = slow_topics;
                }
            }
        })
    }

    /// Get current statistics
    pub async fn get_stats(&self) -> EventBusStats {
        self.stats.read().await.clone()
    }

    /// Shutdown EventBus gracefully
    pub async fn shutdown(&mut self) {
        info!("Shutting down EventBus gracefully...");

        // Cancel background tasks
        for task in self._background_tasks.drain(..) {
            task.abort();
        }

        // Clear subscriptions
        let mut subs = self.subscriptions.write().await;
        subs.clear();

        info!("EventBus shutdown completed");
    }

    /// Internal helper to update statistics
    async fn update_stats<F>(&self, update_fn: F)
    where
        F: FnOnce(&mut EventBusStats),
    {
        let mut stats = self.stats.write().await;
        update_fn(&mut stats);
    }

    /// Internal helper to deliver event to handler safely
    async fn deliver_event_safely(
        &self,
        handler: Arc<dyn EventHandler>,
        event: Event,
        topic: &str,
    ) {
        Self::deliver_event_safely_static(
            Arc::clone(&self.stats),
            self.config.clone(),
            handler,
            event,
            topic,
        )
        .await;
    }

    /// Static version of deliver_event_safely for use in spawned tasks
    async fn deliver_event_safely_static(
        stats: Arc<RwLock<EventBusStats>>,
        config: BackpressureConfig,
        handler: Arc<dyn EventHandler>,
        event: Event,
        topic: &str,
    ) {
        let handler_name = handler.name();
        let start_time = Instant::now();

        // Apply timeout to prevent blocking
        let delivery_timeout = Duration::from_millis(config.slow_consumer_threshold_ms);

        match timeout(delivery_timeout, handler.handle(event.clone())).await {
            Ok(Ok(())) => {
                let duration = start_time.elapsed();
                debug!(
                    "Event delivered to handler '{}' on topic '{}' in {:?}",
                    handler_name, topic, duration
                );

                let mut stats_guard = stats.write().await;
                stats_guard.total_events_delivered += 1;
            }
            Ok(Err(e)) => {
                error!(
                    "Handler '{}' failed to process event on topic '{}': {}",
                    handler_name, topic, e
                );

                let mut stats_guard = stats.write().await;
                stats_guard.failed_deliveries += 1;
            }
            Err(_timeout) => {
                warn!(
                    "Handler '{}' timed out processing event on topic '{}' (>{:?})",
                    handler_name, topic, delivery_timeout
                );

                let mut stats_guard = stats.write().await;
                stats_guard.failed_deliveries += 1;
                stats_guard
                    .slow_consumers
                    .push(format!("{}:{}", topic, handler_name));
            }
        }
    }
}

#[async_trait]
impl EventPublisher for EventBus {
    async fn publish(&self, topic: &str, payload: serde_json::Value, source: &str) -> Result<()> {
        self.publish_correlated(topic, payload, source, Uuid::new_v4())
            .await
    }

    async fn publish_correlated(
        &self,
        topic: &str,
        payload: serde_json::Value,
        source: &str,
        correlation_id: Uuid,
    ) -> Result<()> {
        let event = Event {
            id: Uuid::new_v4(),
            topic: topic.to_string(),
            payload,
            timestamp: Utc::now(),
            source: source.to_string(),
            correlation_id: Some(correlation_id),
        };

        // Publish to broadcast channel
        match self.publisher.send(event.clone()) {
            Ok(subscriber_count) => {
                debug!(
                    "Published event to topic '{}' - {} subscribers",
                    topic, subscriber_count
                );

                self.update_stats(|stats| {
                    stats.total_events_published += 1;
                })
                .await;
            }
            Err(_) => {
                warn!("No subscribers for topic '{}'", topic);
            }
        }

        // Deliver to topic-specific handlers
        let subscriptions = self.subscriptions.read().await;
        if let Some(handlers) = subscriptions.get(topic) {
            for handler in handlers {
                let handler_clone = Arc::clone(handler);
                let event_clone = event.clone();
                let stats_ref = Arc::clone(&self.stats);
                let config = self.config.clone();
                let topic_string = topic.to_string();

                // Spawn task to handle delivery asynchronously
                tokio::spawn(async move {
                    Self::deliver_event_safely_static(
                        stats_ref,
                        config,
                        handler_clone,
                        event_clone,
                        &topic_string,
                    )
                    .await;
                });
            }
        }

        Ok(())
    }
}

#[async_trait]
impl EventSubscriber for EventBus {
    async fn subscribe(&self, topic: &str, handler: Arc<dyn EventHandler>) -> Result<()> {
        let mut subscriptions = self.subscriptions.write().await;

        let handlers = subscriptions
            .entry(topic.to_string())
            .or_insert_with(Vec::new);
        handlers.push(handler);

        info!(
            "Handler '{}' subscribed to topic '{}'",
            handlers.last().unwrap().name(),
            topic
        );

        self.update_stats(|stats| {
            stats.active_subscriptions = subscriptions.len();
        })
        .await;

        Ok(())
    }

    async fn unsubscribe(&self, topic: &str, handler_name: &str) -> Result<()> {
        let mut subscriptions = self.subscriptions.write().await;

        if let Some(handlers) = subscriptions.get_mut(topic) {
            let original_len = handlers.len();
            handlers.retain(|h| h.name() != handler_name);

            let removed_count = original_len - handlers.len();
            if removed_count > 0 {
                info!(
                    "Unsubscribed handler '{}' from topic '{}'",
                    handler_name, topic
                );
            }

            // Remove empty topic entries
            if handlers.is_empty() {
                subscriptions.remove(topic);
            }
        }

        self.update_stats(|stats| {
            stats.active_subscriptions = subscriptions.len();
        })
        .await;

        Ok(())
    }

    async fn topics(&self) -> Vec<String> {
        let subscriptions = self.subscriptions.read().await;
        subscriptions.keys().cloned().collect()
    }
}

/// Convenience macro for creating events
#[macro_export]
macro_rules! event {
    ($topic:expr, $payload:expr, $source:expr) => {
        Event {
            id: uuid::Uuid::new_v4(),
            topic: $topic.to_string(),
            payload: serde_json::json!($payload),
            timestamp: chrono::Utc::now(),
            source: $source.to_string(),
            correlation_id: None,
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHandler {
        name: String,
    }

    #[async_trait]
    impl EventHandler for TestHandler {
        async fn handle(&self, event: Event) -> Result<()> {
            println!("Handler '{}' received event: {:?}", self.name, event);
            Ok(())
        }

        fn topics(&self) -> Vec<String> {
            vec!["test".to_string()]
        }

        fn name(&self) -> String {
            self.name.clone()
        }
    }

    #[tokio::test]
    async fn test_event_bus_basic_flow() {
        let mut bus = EventBus::default();
        bus.start_background_tasks().await;

        // Subscribe handler
        let handler = Arc::new(TestHandler {
            name: "test_handler".to_string(),
        });

        bus.subscribe("test", handler).await.unwrap();

        // Publish event
        bus.publish(
            "test",
            serde_json::json!({"message": "hello"}),
            "test_source",
        )
        .await
        .unwrap();

        // Allow some time for async delivery
        tokio::time::sleep(Duration::from_millis(100)).await;

        let stats = bus.get_stats().await;
        assert!(stats.total_events_published > 0);

        bus.shutdown().await;
    }
}
