//! Notification Service Port
//!
//! Абстракция для отправки уведомлений о состоянии системы, ошибках и событиях.

use async_trait::async_trait;
use crate::ApplicationResult;
use serde::{Deserialize, Serialize};

/// Trait для notification services
#[async_trait]
pub trait NotificationService: Send + Sync {
    /// Send single notification
    async fn send_notification(&self, notification: &Notification) -> ApplicationResult<NotificationResult>;
    
    /// Send batch notifications
    async fn send_batch_notifications(&self, notifications: &[Notification]) -> ApplicationResult<BatchNotificationResult>;
    
    /// Subscribe to notification events
    async fn subscribe(&self, subscription: &NotificationSubscription) -> ApplicationResult<SubscriptionId>;
    
    /// Unsubscribe from notifications
    async fn unsubscribe(&self, subscription_id: &SubscriptionId) -> ApplicationResult<()>;
    
    /// Get notification delivery status
    async fn get_delivery_status(&self, notification_id: &str) -> ApplicationResult<DeliveryStatus>;
    
    /// Health check for notification service
    async fn health_check(&self) -> ApplicationResult<NotificationServiceHealth>;
}

/// Notification message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: Option<String>,
    pub title: String,
    pub message: String,
    pub level: NotificationLevel,
    pub category: NotificationCategory,
    pub targets: Vec<NotificationTarget>,
    pub metadata: NotificationMetadata,
    pub delivery_options: DeliveryOptions,
}

/// Notification severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum NotificationLevel {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

/// Notification categories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationCategory {
    SystemHealth,
    Performance,
    Security,
    UserAction,
    DataProcessing,
    MLPromotion,
    SearchAnalytics,
    ResourceUsage,
    Custom(String),
}

/// Notification targets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationTarget {
    /// Email notification
    Email {
        address: String,
        name: Option<String>,
    },
    /// Slack channel/user
    Slack {
        channel: String,
        webhook_url: Option<String>,
    },
    /// Discord webhook
    Discord {
        webhook_url: String,
        channel_name: Option<String>,
    },
    /// Teams webhook
    Teams {
        webhook_url: String,
    },
    /// SMS notification
    Sms {
        phone_number: String,
    },
    /// Push notification
    Push {
        device_token: String,
        platform: PushPlatform,
    },
    /// Webhook callback
    Webhook {
        url: String,
        method: HttpMethod,
        headers: std::collections::HashMap<String, String>,
    },
    /// Log file
    Log {
        level: log::Level,
        target: String,
    },
    /// Custom target
    Custom {
        target_type: String,
        config: serde_json::Value,
    },
}

/// Push notification platforms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PushPlatform {
    Ios,
    Android,
    Web,
}

/// HTTP methods for webhooks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    PATCH,
}

/// Notification metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationMetadata {
    pub source: String,
    pub correlation_id: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub tags: Vec<String>,
    pub custom_fields: std::collections::HashMap<String, serde_json::Value>,
}

/// Delivery options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryOptions {
    pub priority: DeliveryPriority,
    pub retry_policy: RetryPolicy,
    pub batching: BatchingOptions,
    pub rate_limiting: RateLimitingOptions,
    pub formatting: FormattingOptions,
}

/// Delivery priority levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum DeliveryPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Retry policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
    pub retry_on: Vec<RetryCondition>,
}

/// Conditions that trigger retries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetryCondition {
    NetworkError,
    Timeout,
    ServerError, // 5xx responses
    RateLimit,
    ServiceUnavailable,
}

/// Batching options for notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchingOptions {
    pub enabled: bool,
    pub max_batch_size: usize,
    pub max_wait_time_ms: u64,
    pub batch_by_target: bool,
}

/// Rate limiting options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitingOptions {
    pub enabled: bool,
    pub max_per_minute: u32,
    pub max_per_hour: u32,
    pub burst_limit: u32,
}

/// Message formatting options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattingOptions {
    pub format: MessageFormat,
    pub include_metadata: bool,
    pub include_timestamp: bool,
    pub template: Option<String>,
    pub custom_formatting: Option<serde_json::Value>,
}

/// Message formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageFormat {
    PlainText,
    Html,
    Markdown,
    Json,
    Custom(String),
}

/// Notification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationResult {
    pub notification_id: String,
    pub status: DeliveryStatus,
    pub delivery_attempts: Vec<DeliveryAttempt>,
    pub total_processing_time_ms: u64,
}

/// Batch notification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchNotificationResult {
    pub batch_id: String,
    pub total_notifications: usize,
    pub successful_deliveries: usize,
    pub failed_deliveries: usize,
    pub results: Vec<NotificationResult>,
    pub processing_time_ms: u64,
}

/// Delivery status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeliveryStatus {
    Pending,
    Delivered,
    Failed,
    Retrying,
    RateLimited,
    Cancelled,
}

/// Individual delivery attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryAttempt {
    pub attempt_number: u32,
    pub target: NotificationTarget,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub status: DeliveryStatus,
    pub response_code: Option<u16>,
    pub error_message: Option<String>,
    pub delivery_time_ms: u64,
}

/// Notification subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSubscription {
    pub id: Option<String>,
    pub name: String,
    pub filters: SubscriptionFilters,
    pub targets: Vec<NotificationTarget>,
    pub delivery_options: DeliveryOptions,
    pub active: bool,
}

/// Subscription filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionFilters {
    pub levels: Vec<NotificationLevel>,
    pub categories: Vec<NotificationCategory>,
    pub sources: Vec<String>,
    pub tags: Vec<String>,
    pub custom_conditions: Option<serde_json::Value>,
}

/// Subscription identifier
pub type SubscriptionId = String;

/// Notification service health
#[derive(Debug, Clone)]
pub struct NotificationServiceHealth {
    pub is_healthy: bool,
    pub active_subscriptions: usize,
    pub pending_notifications: usize,
    pub delivery_success_rate: f32,
    pub average_delivery_time_ms: u64,
    pub last_error: Option<String>,
    pub target_health: Vec<TargetHealth>,
}

/// Health status of individual targets
#[derive(Debug, Clone)]
pub struct TargetHealth {
    pub target_type: String,
    pub is_healthy: bool,
    pub success_rate: f32,
    pub average_response_time_ms: u64,
    pub last_failure: Option<chrono::DateTime<chrono::Utc>>,
}

impl Notification {
    /// Create a simple info notification
    pub fn info(title: &str, message: &str) -> Self {
        Self {
            id: None,
            title: title.to_string(),
            message: message.to_string(),
            level: NotificationLevel::Info,
            category: NotificationCategory::SystemHealth,
            targets: vec![],
            metadata: NotificationMetadata {
                source: "application".to_string(),
                correlation_id: None,
                timestamp: chrono::Utc::now(),
                tags: vec![],
                custom_fields: std::collections::HashMap::new(),
            },
            delivery_options: DeliveryOptions::default(),
        }
    }
    
    /// Create an error notification
    pub fn error(title: &str, message: &str) -> Self {
        Self {
            level: NotificationLevel::Error,
            ..Self::info(title, message)
        }
    }
    
    /// Create a critical notification
    pub fn critical(title: &str, message: &str) -> Self {
        Self {
            level: NotificationLevel::Critical,
            delivery_options: DeliveryOptions {
                priority: DeliveryPriority::Critical,
                ..DeliveryOptions::default()
            },
            ..Self::info(title, message)
        }
    }
    
    /// Add target to notification
    pub fn with_target(mut self, target: NotificationTarget) -> Self {
        self.targets.push(target);
        self
    }
    
    /// Add email target
    pub fn with_email(mut self, email: &str) -> Self {
        self.targets.push(NotificationTarget::Email {
            address: email.to_string(),
            name: None,
        });
        self
    }
    
    /// Add Slack target
    pub fn with_slack(mut self, channel: &str) -> Self {
        self.targets.push(NotificationTarget::Slack {
            channel: channel.to_string(),
            webhook_url: None,
        });
        self
    }
}

impl Default for DeliveryOptions {
    fn default() -> Self {
        Self {
            priority: DeliveryPriority::Normal,
            retry_policy: RetryPolicy::default(),
            batching: BatchingOptions::default(),
            rate_limiting: RateLimitingOptions::default(),
            formatting: FormattingOptions::default(),
        }
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
            retry_on: vec![
                RetryCondition::NetworkError,
                RetryCondition::Timeout,
                RetryCondition::ServerError,
            ],
        }
    }
}

impl Default for BatchingOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            max_batch_size: 10,
            max_wait_time_ms: 5000,
            batch_by_target: true,
        }
    }
}

impl Default for RateLimitingOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            max_per_minute: 60,
            max_per_hour: 1000,
            burst_limit: 10,
        }
    }
}

impl Default for FormattingOptions {
    fn default() -> Self {
        Self {
            format: MessageFormat::PlainText,
            include_metadata: false,
            include_timestamp: true,
            template: None,
            custom_formatting: None,
        }
    }
}