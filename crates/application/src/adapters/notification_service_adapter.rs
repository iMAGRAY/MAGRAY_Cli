//! Notification Service Adapter
//!
//! Адаптер для интеграции с notification services

use crate::ports::{Notification, NotificationService};
use crate::{ApplicationError, ApplicationResult};
use async_trait::async_trait;
use std::sync::Arc;

/// Simple notification service adapter
pub struct NotificationServiceAdapter {
    notification_service: Arc<dyn NotificationServiceTrait>,
}

#[async_trait]
pub trait NotificationServiceTrait: Send + Sync {
    async fn send_notification(
        &self,
        notification: &NotificationMessage,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

#[derive(Debug, Clone)]
pub struct NotificationMessage {
    pub level: String,
    pub title: String,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl NotificationServiceAdapter {
    pub fn new(notification_service: Arc<dyn NotificationServiceTrait>) -> Self {
        Self {
            notification_service,
        }
    }
}

#[async_trait]
impl NotificationService for NotificationServiceAdapter {
    async fn send_notification(
        &self,
        notification: &Notification,
    ) -> ApplicationResult<crate::ports::NotificationResult> {
        let message = NotificationMessage {
            level: format!("{:?}", notification.level),
            title: notification.title.clone(),
            message: notification.message.clone(),
            timestamp: notification.metadata.timestamp,
        };

        self.notification_service
            .send_notification(&message)
            .await
            .map_err(|e| {
                ApplicationError::infrastructure(format!("Failed to send notification: {}", e))
            })?;

        Ok(crate::ports::NotificationResult {
            notification_id: uuid::Uuid::new_v4().to_string(),
            status: crate::ports::DeliveryStatus::Delivered,
            delivery_attempts: vec![],
            total_processing_time_ms: 5,
        })
    }

    async fn send_batch_notifications(
        &self,
        notifications: &[Notification],
    ) -> ApplicationResult<crate::ports::BatchNotificationResult> {
        let mut results = Vec::new();

        for notification in notifications {
            let result = self.send_notification(notification).await?;
            results.push(result);
        }

        Ok(crate::ports::BatchNotificationResult {
            batch_id: uuid::Uuid::new_v4().to_string(),
            total_notifications: notifications.len(),
            successful_deliveries: results.len(),
            failed_deliveries: 0,
            results,
            processing_time_ms: 50,
        })
    }

    async fn subscribe(
        &self,
        _subscription: &crate::ports::NotificationSubscription,
    ) -> ApplicationResult<crate::ports::SubscriptionId> {
        // Mock implementation
        Ok(uuid::Uuid::new_v4().to_string())
    }

    async fn unsubscribe(
        &self,
        _subscription_id: &crate::ports::SubscriptionId,
    ) -> ApplicationResult<()> {
        // Mock implementation
        Ok(())
    }

    async fn get_delivery_status(
        &self,
        _notification_id: &str,
    ) -> ApplicationResult<crate::ports::DeliveryStatus> {
        // Mock implementation
        Ok(crate::ports::DeliveryStatus::Delivered)
    }

    async fn health_check(&self) -> ApplicationResult<crate::ports::NotificationServiceHealth> {
        Ok(crate::ports::NotificationServiceHealth {
            is_healthy: true,
            active_subscriptions: 0,
            pending_notifications: 0,
            delivery_success_rate: 1.0,
            average_delivery_time_ms: 5,
            last_error: None,
            target_health: vec![],
        })
    }
}
