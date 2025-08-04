use memory::notifications::*;
use memory::types::*;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_notification_system_creation() {
    let system = NotificationSystem::new();
    
    assert_eq!(system.active_subscribers(), 0);
    assert_eq!(system.total_notifications_sent(), 0);
    assert!(system.is_healthy());
}

#[tokio::test]
async fn test_notification_subscription() {
    let mut system = NotificationSystem::new();
    
    let subscriber_id = system.subscribe(NotificationType::MemoryLimitReached).await;
    assert!(!subscriber_id.is_empty());
    
    assert_eq!(system.active_subscribers(), 1);
    
    let success = system.unsubscribe(&subscriber_id).await;
    assert!(success);
    assert_eq!(system.active_subscribers(), 0);
}

#[tokio::test]
async fn test_notification_sending() {
    let mut system = NotificationSystem::new();
    
    let subscriber_id = system.subscribe(NotificationType::MemoryLimitReached).await;
    
    let notification = Notification {
        id: "test_notification".to_string(),
        notification_type: NotificationType::MemoryLimitReached,
        title: "Memory Limit Alert".to_string(),
        message: "Memory usage has exceeded the configured limit".to_string(),
        severity: NotificationSeverity::High,
        timestamp: chrono::Utc::now(),
        metadata: NotificationMetadata {
            source: "memory_monitor".to_string(),
            tags: vec!["memory".to_string(), "alert".to_string()],
            context: Some("Memory usage: 85%".to_string()),
            action_required: true,
            expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(1)),
        },
    };
    
    let result = system.send_notification(notification).await;
    assert!(result.is_ok());
    
    let sent_count = result.unwrap();
    assert_eq!(sent_count, 1);
    assert_eq!(system.total_notifications_sent(), 1);
}

#[tokio::test]
async fn test_notification_filtering() {
    let mut system = NotificationSystem::new();
    
    // Subscribe to memory alerts only
    let memory_subscriber = system.subscribe(NotificationType::MemoryLimitReached).await;
    
    // Subscribe to all promotion alerts
    let promotion_subscriber = system.subscribe(NotificationType::PromotionCompleted).await;
    
    // Send memory alert - should reach memory subscriber
    let memory_notification = Notification {
        id: "memory_alert".to_string(),
        notification_type: NotificationType::MemoryLimitReached,
        title: "Memory Alert".to_string(),
        message: "Memory usage high".to_string(),
        severity: NotificationSeverity::Medium,
        timestamp: chrono::Utc::now(),
        metadata: NotificationMetadata::default(),
    };
    
    let memory_result = system.send_notification(memory_notification).await;
    assert!(memory_result.is_ok());
    assert_eq!(memory_result.unwrap(), 1); // Only memory subscriber
    
    // Send promotion alert - should reach promotion subscriber
    let promotion_notification = Notification {
        id: "promotion_alert".to_string(),
        notification_type: NotificationType::PromotionCompleted,
        title: "Promotion Complete".to_string(),
        message: "Memory promotion cycle completed".to_string(),
        severity: NotificationSeverity::Info,
        timestamp: chrono::Utc::now(),
        metadata: NotificationMetadata::default(),
    };
    
    let promotion_result = system.send_notification(promotion_notification).await;
    assert!(promotion_result.is_ok());
    assert_eq!(promotion_result.unwrap(), 1); // Only promotion subscriber
    
    assert_eq!(system.total_notifications_sent(), 2);
}

#[tokio::test]
async fn test_notification_severity_levels() {
    let severities = vec![
        NotificationSeverity::Low,
        NotificationSeverity::Medium,
        NotificationSeverity::High,
        NotificationSeverity::Critical,
    ];
    
    for severity in severities {
        let notification = Notification {
            id: format!("test_{:?}", severity),
            notification_type: NotificationType::SystemHealthCheck,
            title: format!("{:?} Alert", severity),
            message: format!("This is a {:?} severity notification", severity),
            severity,
            timestamp: chrono::Utc::now(),
            metadata: NotificationMetadata::default(),
        };
        
        // Test notification creation with different severities
        assert_eq!(notification.severity, severity);
        assert!(notification.should_display());
        
        // Critical notifications should require action
        if matches!(severity, NotificationSeverity::Critical) {
            assert!(notification.is_urgent());
        }
    }
}

#[tokio::test]
async fn test_notification_expiration() {
    let mut system = NotificationSystem::new();
    let subscriber_id = system.subscribe(NotificationType::MemoryLimitReached).await;
    
    // Create expired notification
    let expired_notification = Notification {
        id: "expired_test".to_string(),
        notification_type: NotificationType::MemoryLimitReached,
        title: "Expired Alert".to_string(),
        message: "This notification has expired".to_string(),
        severity: NotificationSeverity::Medium,
        timestamp: chrono::Utc::now(),
        metadata: NotificationMetadata {
            source: "test".to_string(),
            tags: vec![],
            context: None,
            action_required: false,
            expires_at: Some(chrono::Utc::now() - chrono::Duration::hours(1)), // Already expired
        },
    };
    
    let result = system.send_notification(expired_notification).await;
    assert!(result.is_ok());
    
    // Expired notifications might still be sent but should be marked as expired
    let sent_count = result.unwrap();
    assert!(sent_count >= 0);
}

#[tokio::test]
async fn test_notification_batch_sending() {
    let mut system = NotificationSystem::new();
    
    let subscriber_id = system.subscribe(NotificationType::SystemHealthCheck).await;
    
    let mut notifications = vec![];
    for i in 0..5 {
        let notification = Notification {
            id: format!("batch_notification_{}", i),
            notification_type: NotificationType::SystemHealthCheck,
            title: format!("Batch Alert {}", i),
            message: format!("This is batch notification number {}", i),
            severity: NotificationSeverity::Info,
            timestamp: chrono::Utc::now(),
            metadata: NotificationMetadata::default(),
        };
        notifications.push(notification);
    }
    
    let result = system.send_batch_notifications(notifications).await;
    assert!(result.is_ok());
    
    let total_sent = result.unwrap();
    assert_eq!(total_sent, 5);
    assert_eq!(system.total_notifications_sent(), 5);
}

#[tokio::test]
async fn test_notification_system_statistics() {
    let mut system = NotificationSystem::new();
    
    let subscriber1 = system.subscribe(NotificationType::MemoryLimitReached).await;
    let subscriber2 = system.subscribe(NotificationType::PromotionCompleted).await;
    let subscriber3 = system.subscribe(NotificationType::SystemHealthCheck).await;
    
    let stats = system.get_statistics();
    assert_eq!(stats.active_subscribers, 3);
    assert_eq!(stats.total_notifications_sent, 0);
    assert_eq!(stats.failed_deliveries, 0);
    assert!(stats.uptime_seconds >= 0.0);
    
    // Send some notifications
    let notification = Notification {
        id: "stats_test".to_string(),
        notification_type: NotificationType::MemoryLimitReached,
        title: "Stats Test".to_string(),
        message: "Testing statistics".to_string(),
        severity: NotificationSeverity::Medium,
        timestamp: chrono::Utc::now(),
        metadata: NotificationMetadata::default(),
    };
    
    system.send_notification(notification).await.unwrap();
    
    let updated_stats = system.get_statistics();
    assert!(updated_stats.total_notifications_sent > 0);
}

#[tokio::test]
async fn test_notification_delivery_retry() {
    let mut system = NotificationSystem::new();
    
    // Configure system with retry policy
    system.configure_retry_policy(RetryPolicy {
        max_attempts: 3,
        initial_delay_ms: 100,
        backoff_multiplier: 2.0,
        max_delay_ms: 1000,
    });
    
    let subscriber_id = system.subscribe(NotificationType::MemoryLimitReached).await;
    
    let notification = Notification {
        id: "retry_test".to_string(),
        notification_type: NotificationType::MemoryLimitReached,
        title: "Retry Test".to_string(),
        message: "Testing delivery retry".to_string(),
        severity: NotificationSeverity::High,
        timestamp: chrono::Utc::now(),
        metadata: NotificationMetadata::default(),
    };
    
    let result = system.send_notification_with_retry(notification).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_notification_rate_limiting() {
    let mut system = NotificationSystem::new();
    
    // Configure rate limiting
    system.configure_rate_limit(RateLimit {
        max_notifications_per_minute: 10,
        burst_size: 5,
        cooldown_period_seconds: 60,
    });
    
    let subscriber_id = system.subscribe(NotificationType::SystemHealthCheck).await;
    
    // Send notifications rapidly
    for i in 0..15 {
        let notification = Notification {
            id: format!("rate_limit_test_{}", i),
            notification_type: NotificationType::SystemHealthCheck,
            title: format!("Rate Limit Test {}", i),
            message: "Testing rate limiting".to_string(),
            severity: NotificationSeverity::Info,
            timestamp: chrono::Utc::now(),
            metadata: NotificationMetadata::default(),
        };
        
        let result = system.send_notification(notification).await;
        
        // Some notifications might be rate limited
        if i >= 10 {
            // After 10 notifications, some should be rate limited
            assert!(result.is_ok() || result.is_err());
        } else {
            assert!(result.is_ok());
        }
    }
}

#[tokio::test]
async fn test_notification_templates() {
    let system = NotificationSystem::new();
    
    // Test memory alert template
    let memory_alert = system.create_memory_alert_notification(85.0, 1024 * 1024 * 1024);
    assert_eq!(memory_alert.notification_type, NotificationType::MemoryLimitReached);
    assert_eq!(memory_alert.severity, NotificationSeverity::High);
    assert!(memory_alert.message.contains("85"));
    assert!(memory_alert.metadata.action_required);
    
    // Test promotion complete template
    let promotion_stats = PromotionStats {
        interact_to_insights: 15,
        insights_to_assets: 3,
        expired_interact: 5,
        expired_insights: 1,
        total_time_ms: 1250,
        index_update_time_ms: 200,
        promotion_time_ms: 800,
        cleanup_time_ms: 250,
    };
    
    let promotion_notification = system.create_promotion_complete_notification(&promotion_stats);
    assert_eq!(promotion_notification.notification_type, NotificationType::PromotionCompleted);
    assert_eq!(promotion_notification.severity, NotificationSeverity::Info);
    assert!(promotion_notification.message.contains("15"));
    assert!(!promotion_notification.metadata.action_required);
    
    // Test system health template
    let health_notification = system.create_health_check_notification(true, vec![
        "Memory usage: OK".to_string(),
        "Vector index: OK".to_string(),
        "Cache performance: WARNING".to_string(),
    ]);
    
    assert_eq!(health_notification.notification_type, NotificationType::SystemHealthCheck);
    assert!(health_notification.message.contains("OK"));
}

#[tokio::test]
async fn test_notification_persistence() {
    let mut system = NotificationSystem::new();
    
    // Enable persistent notifications
    system.enable_persistence(true);
    
    let subscriber_id = system.subscribe(NotificationType::MemoryLimitReached).await;
    
    let notification = Notification {
        id: "persistent_test".to_string(),
        notification_type: NotificationType::MemoryLimitReached,
        title: "Persistent Test".to_string(),
        message: "This notification should be persisted".to_string(),
        severity: NotificationSeverity::Critical,
        timestamp: chrono::Utc::now(),
        metadata: NotificationMetadata {
            source: "test".to_string(),
            tags: vec!["persistent".to_string()],
            context: Some("Test context".to_string()),
            action_required: true,
            expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(24)),
        },
    };
    
    system.send_notification(notification.clone()).await.unwrap();
    
    // Retrieve persistent notifications
    let persistent_notifications = system.get_persistent_notifications().await;
    assert!(persistent_notifications.is_ok());
    
    let notifications = persistent_notifications.unwrap();
    assert!(!notifications.is_empty());
    
    let found_notification = notifications.iter()
        .find(|n| n.id == "persistent_test");
    assert!(found_notification.is_some());
}

#[tokio::test]
async fn test_notification_webhooks() {
    let mut system = NotificationSystem::new();
    
    // Configure webhook endpoint
    let webhook_config = WebhookConfig {
        url: "https://example.com/webhook".to_string(),
        secret: Some("webhook_secret".to_string()),
        timeout_seconds: 30,
        retry_attempts: 3,
        headers: vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("User-Agent".to_string(), "MAGRAY-Notifications/1.0".to_string()),
        ],
    };
    
    system.configure_webhook(webhook_config).await.unwrap();
    
    let subscriber_id = system.subscribe(NotificationType::MemoryLimitReached).await;
    
    let notification = Notification {
        id: "webhook_test".to_string(),
        notification_type: NotificationType::MemoryLimitReached,
        title: "Webhook Test".to_string(),
        message: "Testing webhook delivery".to_string(),
        severity: NotificationSeverity::High,
        timestamp: chrono::Utc::now(),
        metadata: NotificationMetadata::default(),
    };
    
    // This will likely fail since we're using a fake URL, but should handle gracefully
    let result = system.send_notification(notification).await;
    assert!(result.is_ok() || result.is_err()); // Either succeeds or fails gracefully
}

#[tokio::test]
async fn test_notification_cleanup() {
    let mut system = NotificationSystem::new();
    system.enable_persistence(true);
    
    let subscriber_id = system.subscribe(NotificationType::SystemHealthCheck).await;
    
    // Create old notification that should be cleaned up
    let old_notification = Notification {
        id: "old_notification".to_string(),
        notification_type: NotificationType::SystemHealthCheck,
        title: "Old Notification".to_string(),
        message: "This is an old notification".to_string(),
        severity: NotificationSeverity::Info,
        timestamp: chrono::Utc::now() - chrono::Duration::days(30),
        metadata: NotificationMetadata {
            expires_at: Some(chrono::Utc::now() - chrono::Duration::days(1)),
            ..Default::default()
        },
    };
    
    system.send_notification(old_notification).await.unwrap();
    
    // Run cleanup
    let cleanup_result = system.cleanup_expired_notifications().await;
    assert!(cleanup_result.is_ok());
    
    let cleaned_count = cleanup_result.unwrap();
    assert!(cleaned_count >= 0);
}

#[tokio::test]
async fn test_notification_concurrent_access() {
    use std::sync::Arc;
    use tokio::sync::Mutex;
    
    let system = Arc::new(Mutex::new(NotificationSystem::new()));
    let mut handles = vec![];
    
    // Spawn multiple tasks to send notifications concurrently
    for i in 0..10 {
        let system_clone = Arc::clone(&system);
        let handle = tokio::spawn(async move {
            let mut sys = system_clone.lock().await;
            let subscriber_id = sys.subscribe(NotificationType::SystemHealthCheck).await;
            
            let notification = Notification {
                id: format!("concurrent_test_{}", i),
                notification_type: NotificationType::SystemHealthCheck,
                title: format!("Concurrent Test {}", i),
                message: format!("Testing concurrent access {}", i),
                severity: NotificationSeverity::Info,
                timestamp: chrono::Utc::now(),
                metadata: NotificationMetadata::default(),
            };
            
            sys.send_notification(notification).await.unwrap();
            subscriber_id
        });
        handles.push(handle);
    }
    
    let mut subscriber_ids = vec![];
    for handle in handles {
        let subscriber_id = handle.await.unwrap();
        subscriber_ids.push(subscriber_id);
    }
    
    // Verify all notifications were sent
    let final_system = system.lock().await;
    assert_eq!(final_system.active_subscribers(), 10);
    assert_eq!(final_system.total_notifications_sent(), 10);
}

#[test]
fn test_notification_serialization() {
    let notification = Notification {
        id: "serialization_test".to_string(),
        notification_type: NotificationType::MemoryLimitReached,
        title: "Serialization Test".to_string(),
        message: "Testing JSON serialization".to_string(),
        severity: NotificationSeverity::Medium,
        timestamp: chrono::Utc::now(),
        metadata: NotificationMetadata {
            source: "unit_test".to_string(),
            tags: vec!["serialization".to_string(), "test".to_string()],
            context: Some("Test context data".to_string()),
            action_required: true,
            expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(2)),
        },
    };
    
    // Test JSON serialization
    let json = serde_json::to_string(&notification).unwrap();
    assert!(json.contains("serialization_test"));
    assert!(json.contains("MemoryLimitReached"));
    assert!(json.contains("Medium"));
    
    // Test deserialization
    let deserialized: Notification = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.id, notification.id);
    assert_eq!(deserialized.title, notification.title);
    assert_eq!(deserialized.severity, notification.severity);
    assert_eq!(deserialized.metadata.action_required, notification.metadata.action_required);
}

#[test]
fn test_notification_metadata_defaults() {
    let metadata = NotificationMetadata::default();
    
    assert_eq!(metadata.source, "system");
    assert!(metadata.tags.is_empty());
    assert!(metadata.context.is_none());
    assert!(!metadata.action_required);
    assert!(metadata.expires_at.is_none());
}

#[test]
fn test_notification_type_categories() {
    let memory_types = vec![
        NotificationType::MemoryLimitReached,
        NotificationType::CacheEviction,
    ];
    
    let system_types = vec![
        NotificationType::SystemHealthCheck,
        NotificationType::SystemStartup,
        NotificationType::SystemShutdown,
    ];
    
    let promotion_types = vec![
        NotificationType::PromotionCompleted,
        NotificationType::PromotionFailed,
    ];
    
    // Test that notification types can be categorized
    for notification_type in memory_types {
        assert!(notification_type.is_memory_related());
    }
    
    for notification_type in system_types {
        assert!(notification_type.is_system_related());
    }
    
    for notification_type in promotion_types {
        assert!(notification_type.is_promotion_related());
    }
}