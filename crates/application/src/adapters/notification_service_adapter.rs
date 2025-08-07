//! Notification Service Adapter
//!
//! Адаптер для интеграции с notification services

use std::sync::Arc;
use async_trait::async_trait;
use crate::{ApplicationResult, ApplicationError};
use crate::ports::{NotificationService, Notification};

/// Simple notification service adapter
pub struct NotificationServiceAdapter {
    notification_service: Arc<dyn NotificationServiceTrait>,
}

#[async_trait]
pub trait NotificationServiceTrait: Send + Sync {
    async fn send_notification(&self, notification: &NotificationMessage) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
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
        Self { notification_service }
    }
}

#[async_trait]
impl NotificationService for NotificationServiceAdapter {
    async fn send_notification(&self, notification: &Notification) -> ApplicationResult<()> {
        let message = NotificationMessage {
            level: format!("{:?}", notification.level),
            title: notification.title.clone(),
            message: notification.message.clone(),
            timestamp: notification.timestamp,
        };

        self.notification_service.send_notification(&message).await
            .map_err(|e| ApplicationError::infrastructure_with_source("Failed to send notification", e))
    }

    async fn send_batch_notifications(&self, notifications: &[Notification]) -> ApplicationResult<()> {
        for notification in notifications {
            self.send_notification(notification).await?;
        }
        Ok(())
    }

    async fn health_check(&self) -> ApplicationResult<crate::ports::NotificationHealth> {
        Ok(crate::ports::NotificationHealth {
            is_healthy: true,
            queued_notifications: 0,
            failed_notifications: 0,
            last_notification_sent: Some(chrono::Utc::now()),
            response_time_ms: 5,
            error_rate: 0.0,
        })
    }
}