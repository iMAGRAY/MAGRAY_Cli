use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::health::{HealthAlert, AlertSeverity};

// @component: {"k":"C","id":"notification_system","t":"Production alert notification system","m":{"cur":95,"tgt":100,"u":"%"},"f":["alerts","notifications","production"]}

/// Типы каналов уведомлений
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationChannel {
    Email {
        smtp_server: String,
        smtp_port: u16,
        username: String,
        password: String,
        from_address: String,
        to_addresses: Vec<String>,
        use_tls: bool,
    },
    Slack {
        webhook_url: String,
        channel: Option<String>,
        mention_users: Vec<String>,
    },
    Webhook {
        url: String,
        method: String,
        headers: HashMap<String, String>,
        auth_token: Option<String>,
    },
    Console {
        colored: bool,
    },
    Log,
}

/// Конфигурация уведомлений
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// Каналы уведомлений
    pub channels: Vec<NotificationChannel>,
    
    /// Маршрутизация алертов по severity
    pub routing: HashMap<AlertSeverity, Vec<String>>,
    
    /// Минимальный интервал между одинаковыми алертами (секунды)
    pub cooldown_seconds: u64,
    
    /// Включить группировку похожих алертов
    pub enable_grouping: bool,
    
    /// Максимальное количество алертов в группе
    pub max_group_size: usize,
    
    /// Интервал отправки сгруппированных алертов (секунды)
    pub group_interval_seconds: u64,
    
    /// Фильтры по компонентам (whitelist)
    pub component_filters: Option<Vec<String>>,
    
    /// Игнорируемые паттерны в описании алертов
    pub ignore_patterns: Vec<String>,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        let mut routing = HashMap::new();
        routing.insert(AlertSeverity::Info, vec!["log".to_string()]);
        routing.insert(AlertSeverity::Warning, vec!["log".to_string(), "console".to_string()]);
        routing.insert(AlertSeverity::Critical, vec!["log".to_string(), "console".to_string(), "email".to_string()]);
        routing.insert(AlertSeverity::Fatal, vec!["*".to_string()]); // All channels
        
        Self {
            channels: vec![
                NotificationChannel::Log,
                NotificationChannel::Console { colored: true },
            ],
            routing,
            cooldown_seconds: 300, // 5 минут
            enable_grouping: true,
            max_group_size: 10,
            group_interval_seconds: 60, // 1 минута
            component_filters: None,
            ignore_patterns: vec![],
        }
    }
}

/// Трейт для отправки уведомлений
#[async_trait]
pub trait NotificationSender: Send + Sync {
    /// Уникальное имя канала
    fn channel_name(&self) -> &str;
    
    /// Отправить одиночное уведомление
    async fn send_single(&self, alert: &HealthAlert) -> Result<()>;
    
    /// Отправить группу уведомлений
    async fn send_batch(&self, alerts: &[HealthAlert]) -> Result<()> {
        // По умолчанию отправляем по одному
        for alert in alerts {
            self.send_single(alert).await?;
        }
        Ok(())
    }
    
    /// Проверить доступность канала
    async fn test_connection(&self) -> Result<()> {
        Ok(())
    }
}

/// Консольный отправитель уведомлений
pub struct ConsoleSender {
    colored: bool,
}

#[async_trait]
impl NotificationSender for ConsoleSender {
    fn channel_name(&self) -> &str {
        "console"
    }
    
    async fn send_single(&self, alert: &HealthAlert) -> Result<()> {
        let icon = match alert.severity {
            AlertSeverity::Info => "ℹ️",
            AlertSeverity::Warning => "⚠️",
            AlertSeverity::Critical => "🚨",
            AlertSeverity::Fatal => "💀",
        };
        
        if self.colored {
            use colored::*;
            let message = format!("{} {} - {}", icon, alert.title, alert.description);
            
            match alert.severity {
                AlertSeverity::Info => println!("{}", message.blue()),
                AlertSeverity::Warning => println!("{}", message.yellow()),
                AlertSeverity::Critical => println!("{}", message.red()),
                AlertSeverity::Fatal => println!("{}", message.red().bold()),
            }
        } else {
            println!("{} {} - {}", icon, alert.title, alert.description);
        }
        
        Ok(())
    }
}

/// Log отправитель уведомлений
pub struct LogSender;

#[async_trait]
impl NotificationSender for LogSender {
    fn channel_name(&self) -> &str {
        "log"
    }
    
    async fn send_single(&self, alert: &HealthAlert) -> Result<()> {
        match alert.severity {
            AlertSeverity::Info => {
                info!("ALERT: {} - {}", alert.title, alert.description);
            }
            AlertSeverity::Warning => {
                warn!("ALERT: {} - {}", alert.title, alert.description);
            }
            AlertSeverity::Critical | AlertSeverity::Fatal => {
                error!("ALERT: {} - {}", alert.title, alert.description);
            }
        }
        Ok(())
    }
}

/// Webhook отправитель уведомлений
pub struct WebhookSender {
    url: String,
    method: String,
    headers: HashMap<String, String>,
    auth_token: Option<String>,
    client: reqwest::Client,
}

impl WebhookSender {
    pub fn new(url: String, method: String, headers: HashMap<String, String>, auth_token: Option<String>) -> Self {
        Self {
            url,
            method,
            headers,
            auth_token,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl NotificationSender for WebhookSender {
    fn channel_name(&self) -> &str {
        "webhook"
    }
    
    async fn send_single(&self, alert: &HealthAlert) -> Result<()> {
        let payload = serde_json::json!({
            "alert": alert,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "source": "magray_memory_system",
        });
        
        let mut request = match self.method.to_uppercase().as_str() {
            "POST" => self.client.post(&self.url),
            "PUT" => self.client.put(&self.url),
            _ => return Err(anyhow!("Unsupported HTTP method: {}", self.method)),
        };
        
        // Добавляем заголовки
        for (key, value) in &self.headers {
            request = request.header(key, value);
        }
        
        // Добавляем авторизацию
        if let Some(token) = &self.auth_token {
            request = request.bearer_auth(token);
        }
        
        let response = request
            .json(&payload)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(anyhow!(
                "Webhook failed with status {}: {}", 
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }
        
        Ok(())
    }
    
    async fn test_connection(&self) -> Result<()> {
        let response = self.client
            .head(&self.url)
            .send()
            .await?;
        
        if !response.status().is_success() && response.status() != reqwest::StatusCode::METHOD_NOT_ALLOWED {
            return Err(anyhow!("Webhook endpoint not accessible: {}", response.status()));
        }
        
        Ok(())
    }
}

/// Slack отправитель уведомлений
pub struct SlackSender {
    webhook_url: String,
    channel: Option<String>,
    mention_users: Vec<String>,
    client: reqwest::Client,
}

impl SlackSender {
    pub fn new(webhook_url: String, channel: Option<String>, mention_users: Vec<String>) -> Self {
        Self {
            webhook_url,
            channel,
            mention_users,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl NotificationSender for SlackSender {
    fn channel_name(&self) -> &str {
        "slack"
    }
    
    async fn send_single(&self, alert: &HealthAlert) -> Result<()> {
        let emoji = match alert.severity {
            AlertSeverity::Info => ":information_source:",
            AlertSeverity::Warning => ":warning:",
            AlertSeverity::Critical => ":rotating_light:",
            AlertSeverity::Fatal => ":skull:",
        };
        
        let color = match alert.severity {
            AlertSeverity::Info => "#36a64f",
            AlertSeverity::Warning => "#ff9900",
            AlertSeverity::Critical => "#ff0000",
            AlertSeverity::Fatal => "#000000",
        };
        
        let mentions = if !self.mention_users.is_empty() {
            format!(" cc: {}", self.mention_users.iter()
<<<<<<< HEAD
                .map(|u| format!("<@{u}>"))
=======
                .map(|u| format!("<@{}>", u))
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
                .collect::<Vec<_>>()
                .join(" "))
        } else {
            String::new()
        };
        
        let payload = serde_json::json!({
            "channel": self.channel,
            "attachments": [{
                "color": color,
                "title": format!("{} {}", emoji, alert.title),
                "text": format!("{}{}", alert.description, mentions),
                "fields": [
                    {
                        "title": "Component",
                        "value": format!("{:?}", alert.component),
                        "short": true
                    },
                    {
                        "title": "Severity",
                        "value": format!("{:?}", alert.severity),
                        "short": true
                    }
                ],
                "footer": "MAGRAY Memory System",
                "ts": alert.timestamp.timestamp()
            }]
        });
        
        let response = self.client
            .post(&self.webhook_url)
            .json(&payload)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(anyhow!(
                "Slack webhook failed: {}", 
                response.text().await.unwrap_or_default()
            ));
        }
        
        Ok(())
    }
    
    async fn send_batch(&self, alerts: &[HealthAlert]) -> Result<()> {
        if alerts.is_empty() {
            return Ok(());
        }
        
        let severity_emoji = |sev: &AlertSeverity| match sev {
            AlertSeverity::Info => ":information_source:",
            AlertSeverity::Warning => ":warning:",
            AlertSeverity::Critical => ":rotating_light:",
            AlertSeverity::Fatal => ":skull:",
        };
        
        let alert_summary = alerts.iter()
            .map(|a| format!("• {} {} - {}", severity_emoji(&a.severity), a.title, a.description))
            .collect::<Vec<_>>()
            .join("\n");
        
        let payload = serde_json::json!({
            "channel": self.channel,
            "text": format!("🚨 *Alert Summary* ({} alerts)\n\n{}", alerts.len(), alert_summary),
            "attachments": []
        });
        
        let response = self.client
            .post(&self.webhook_url)
            .json(&payload)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("Slack batch webhook failed"));
        }
        
        Ok(())
    }
}

/// Главный менеджер уведомлений
pub struct NotificationManager {
    config: NotificationConfig,
    senders: HashMap<String, Arc<dyn NotificationSender>>,
    alert_history: Arc<parking_lot::RwLock<HashMap<String, std::time::Instant>>>,
    grouped_alerts: Arc<parking_lot::RwLock<Vec<HealthAlert>>>,
}

impl NotificationManager {
    /// Создает новый менеджер уведомлений
    pub fn new(config: NotificationConfig) -> Result<Self> {
        let mut senders: HashMap<String, Arc<dyn NotificationSender>> = HashMap::new();
        
        // Инициализируем каналы из конфигурации
        for channel in &config.channels {
            match channel {
                NotificationChannel::Console { colored } => {
                    senders.insert(
                        "console".to_string(),
                        Arc::new(ConsoleSender { colored: *colored })
                    );
                }
                NotificationChannel::Log => {
                    senders.insert("log".to_string(), Arc::new(LogSender));
                }
                NotificationChannel::Webhook { url, method, headers, auth_token } => {
                    senders.insert(
                        "webhook".to_string(),
                        Arc::new(WebhookSender::new(
                            url.clone(),
                            method.clone(),
                            headers.clone(),
                            auth_token.clone()
                        ))
                    );
                }
                NotificationChannel::Slack { webhook_url, channel, mention_users } => {
                    senders.insert(
                        "slack".to_string(),
                        Arc::new(SlackSender::new(
                            webhook_url.clone(),
                            channel.clone(),
                            mention_users.clone()
                        ))
                    );
                }
                NotificationChannel::Email { .. } => {
                    warn!("Email notifications not implemented yet");
                }
            }
        }
        
        let manager = Self {
            config,
            senders,
            alert_history: Arc::new(parking_lot::RwLock::new(HashMap::new())),
            grouped_alerts: Arc::new(parking_lot::RwLock::new(Vec::new())),
        };
        
        // Запускаем фоновую задачу для отправки сгруппированных алертов
        if manager.config.enable_grouping {
            let manager_clone = manager.clone();
            tokio::spawn(async move {
                manager_clone.group_sender_loop().await;
            });
        }
        
        Ok(manager)
    }
    
    /// Обрабатывает входящий алерт
    pub async fn handle_alert(&self, alert: HealthAlert) -> Result<()> {
        // Проверяем фильтры
        if !self.should_send_alert(&alert) {
            return Ok(());
        }
        
        // Проверяем cooldown
        if self.is_in_cooldown(&alert) {
            return Ok(());
        }
        
        // Если включена группировка, добавляем в буфер
        if self.config.enable_grouping && alert.severity != AlertSeverity::Fatal {
            let alerts_to_send = {
                let mut grouped = self.grouped_alerts.write();
                grouped.push(alert);
                
                if grouped.len() >= self.config.max_group_size {
                    grouped.drain(..).collect::<Vec<_>>()
                } else {
                    Vec::new()
                }
            };
            
            if !alerts_to_send.is_empty() {
                self.send_grouped_alerts(alerts_to_send).await?;
            }
            
            return Ok(());
        }
        
        // Иначе отправляем сразу
        self.send_alert(&alert).await?;
        Ok(())
    }
    
    /// Проверяет, должен ли алерт быть отправлен
    fn should_send_alert(&self, alert: &HealthAlert) -> bool {
        // Проверяем фильтр по компонентам
        if let Some(ref filters) = self.config.component_filters {
            let component_str = format!("{:?}", alert.component);
            if !filters.contains(&component_str) {
                return false;
            }
        }
        
        // Проверяем игнорируемые паттерны
        for pattern in &self.config.ignore_patterns {
            if alert.description.contains(pattern) || alert.title.contains(pattern) {
                return false;
            }
        }
        
        true
    }
    
    /// Проверяет cooldown для алерта
    fn is_in_cooldown(&self, alert: &HealthAlert) -> bool {
        let alert_key = format!("{:?}-{}", alert.component, alert.title);
        let mut history = self.alert_history.write();
        
        if let Some(last_sent) = history.get(&alert_key) {
            if last_sent.elapsed().as_secs() < self.config.cooldown_seconds {
                return true;
            }
        }
        
        history.insert(alert_key, std::time::Instant::now());
        false
    }
    
    /// Отправляет алерт через соответствующие каналы
    async fn send_alert(&self, alert: &HealthAlert) -> Result<()> {
        let channels = self.config.routing
            .get(&alert.severity)
            .cloned()
            .unwrap_or_default();
        
        for channel_name in channels {
            if channel_name == "*" {
                // Отправляем через все каналы
                for sender in self.senders.values() {
                    if let Err(e) = sender.send_single(alert).await {
                        error!("Failed to send alert via {}: {}", sender.channel_name(), e);
                    }
                }
            } else if let Some(sender) = self.senders.get(&channel_name) {
                if let Err(e) = sender.send_single(alert).await {
                    error!("Failed to send alert via {}: {}", channel_name, e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Отправляет группу алертов
    async fn send_grouped_alerts(&self, alerts: Vec<HealthAlert>) -> Result<()> {
        if alerts.is_empty() {
            return Ok(());
        }
        
        // Определяем максимальную severity в группе
        let max_severity = alerts.iter()
            .map(|a| &a.severity)
            .max_by_key(|s| match s {
                AlertSeverity::Info => 0,
                AlertSeverity::Warning => 1,
                AlertSeverity::Critical => 2,
                AlertSeverity::Fatal => 3,
            })
            .unwrap_or(&AlertSeverity::Info);
        
        let channels = self.config.routing
            .get(max_severity)
            .cloned()
            .unwrap_or_default();
        
        for channel_name in channels {
            if channel_name == "*" {
                for sender in self.senders.values() {
                    if let Err(e) = sender.send_batch(&alerts).await {
                        error!("Failed to send batch via {}: {}", sender.channel_name(), e);
                    }
                }
            } else if let Some(sender) = self.senders.get(&channel_name) {
                if let Err(e) = sender.send_batch(&alerts).await {
                    error!("Failed to send batch via {}: {}", channel_name, e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Фоновый цикл отправки сгруппированных алертов
    async fn group_sender_loop(&self) {
        let interval = std::time::Duration::from_secs(self.config.group_interval_seconds);
        
        loop {
            tokio::time::sleep(interval).await;
            
            let alerts_to_send = {
                let mut grouped = self.grouped_alerts.write();
                if grouped.is_empty() {
                    continue;
                }
                grouped.drain(..).collect::<Vec<_>>()
            };
            
            if let Err(e) = self.send_grouped_alerts(alerts_to_send).await {
                error!("Failed to send grouped alerts: {}", e);
            }
        }
    }
    
    /// Тестирует все настроенные каналы
    pub async fn test_all_channels(&self) -> HashMap<String, Result<()>> {
        let mut results = HashMap::new();
        
        for (name, sender) in &self.senders {
            results.insert(name.clone(), sender.test_connection().await);
        }
        
        results
    }
}

impl Clone for NotificationManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            senders: self.senders.clone(),
            alert_history: Arc::clone(&self.alert_history),
            grouped_alerts: Arc::clone(&self.grouped_alerts),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_console_sender() {
        let sender = ConsoleSender { colored: false };
        let alert = HealthAlert {
            id: "test-1".to_string(),
            component: crate::health::ComponentType::VectorStore,
            severity: AlertSeverity::Warning,
            title: "Test Alert".to_string(),
            description: "This is a test alert".to_string(),
            metric_value: Some(100.0),
            threshold: Some(80.0),
            timestamp: chrono::Utc::now(),
            resolved: false,
            resolved_at: None,
        };
        
        assert!(sender.send_single(&alert).await.is_ok());
    }
    
    #[tokio::test]
    async fn test_notification_manager() {
        let config = NotificationConfig::default();
        let manager = NotificationManager::new(config).unwrap();
        
        let alert = HealthAlert {
            id: "test-2".to_string(),
            component: crate::health::ComponentType::Cache,
            severity: AlertSeverity::Info,
            title: "Cache Test".to_string(),
            description: "Testing notification system".to_string(),
            metric_value: None,
            threshold: None,
            timestamp: chrono::Utc::now(),
            resolved: false,
            resolved_at: None,
        };
        
        assert!(manager.handle_alert(alert).await.is_ok());
    }
}