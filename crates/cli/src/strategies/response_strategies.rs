//! Response Formatting Strategies - различные стратегии для форматирования ответов
//! 
//! Реализует Strategy pattern для кастомизации вывода ответов пользователю

use anyhow::Result;
use async_trait::async_trait;
use tracing::{debug, info};
use serde_json;

use crate::agent_traits::{ResponseFormattingStrategy, RequestContext, AgentResponse, AdminResponse};

// ============================================================================
// ОСНОВНЫЕ RESPONSE STRATEGIES
// ============================================================================

/// Простая стратегия форматирования с базовым текстом
pub struct SimpleResponseFormatter {
    add_emoji: bool,
    add_timestamp: bool,
}

impl SimpleResponseFormatter {
    pub fn new(add_emoji: bool, add_timestamp: bool) -> Self {
        Self {
            add_emoji,
            add_timestamp,
        }
    }
    
    pub fn with_defaults() -> Self {
        Self::new(true, false)
    }
    
    fn add_decorations(&self, content: &str, response_type: &str) -> String {
        let mut result = content.to_string();
        
        if self.add_emoji {
            let emoji = match response_type {
                "chat" => "💬",
                "tools" => "🔧",
                "admin" => "⚙️",
                "error" => "❌",
                _ => "ℹ️",
            };
            result = format!("{} {}", emoji, result);
        }
        
        if self.add_timestamp {
            let timestamp = chrono::Utc::now().format("%H:%M:%S");
            result = format!("[{}] {}", timestamp, result);
        }
        
        result
    }
}

#[async_trait]
impl ResponseFormattingStrategy for SimpleResponseFormatter {
    async fn format_response(&self, response: &AgentResponse, context: &RequestContext) -> Result<String> {
        debug!("SimpleResponseFormatter: форматирование ответа для сессии {}", context.session_id);
        
        let (content, response_type) = match response {
            AgentResponse::Chat(text) => (text.clone(), "chat"),
            AgentResponse::ToolExecution(text) => (text.clone(), "tools"),
            AgentResponse::Admin(admin_resp) => {
                let text = match admin_resp {
                    AdminResponse::SystemStats(stats) => format!("Системная статистика:\n{}", stats),
                    AdminResponse::HealthStatus(status) => format!("Статус здоровья:\n{}", status),
                    AdminResponse::PerformanceMetrics(metrics) => format!("Метрики производительности:\n{}", metrics),
                    AdminResponse::OperationResult(result) => format!("Результат операции:\n{}", result),
                };
                (text, "admin")
            }
            AgentResponse::Error(error) => (error.clone(), "error"),
        };
        
        let formatted = self.add_decorations(&content, response_type);
        
        info!("SimpleResponseFormatter: ответ отформатирован (тип: {})", response_type);
        Ok(formatted)
    }
    
    fn supported_response_types(&self) -> Vec<&'static str> {
        vec!["chat", "tools", "admin", "error"]
    }
}

/// Богатая стратегия форматирования с разметкой
pub struct RichResponseFormatter {
    use_markdown: bool,
    show_context: bool,
    max_line_length: usize,
}

impl RichResponseFormatter {
    pub fn new(use_markdown: bool, show_context: bool, max_line_length: usize) -> Self {
        Self {
            use_markdown,
            show_context,
            max_line_length,
        }
    }
    
    pub fn with_defaults() -> Self {
        Self::new(true, false, 80)
    }
    
    fn wrap_text(&self, text: &str) -> String {
        if !self.use_markdown || text.len() <= self.max_line_length {
            return text.to_string();
        }
        
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut lines = Vec::new();
        let mut current_line = String::new();
        
        for word in words {
            if current_line.len() + word.len() + 1 > self.max_line_length {
                if !current_line.is_empty() {
                    lines.push(current_line.clone());
                    current_line.clear();
                }
            }
            
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        }
        
        if !current_line.is_empty() {
            lines.push(current_line);
        }
        
        lines.join("\n")
    }
    
    fn format_chat_response(&self, text: &str, context: &RequestContext) -> String {
        if !self.use_markdown {
            return text.to_string();
        }
        
        let mut result = String::new();
        
        if self.show_context {
            result.push_str(&format!("**Запрос:** {}\n\n", context.message));
        }
        
        result.push_str("**Ответ:**\n");
        result.push_str(&self.wrap_text(text));
        
        result
    }
    
    fn format_tools_response(&self, text: &str, _context: &RequestContext) -> String {
        if !self.use_markdown {
            return text.to_string();
        }
        
        format!("```\n{}\n```", text)
    }
    
    fn format_admin_response(&self, admin_resp: &AdminResponse, _context: &RequestContext) -> String {
        if !self.use_markdown {
            return match admin_resp {
                AdminResponse::SystemStats(stats) => format!("Системная статистика:\n{}", stats),
                AdminResponse::HealthStatus(status) => format!("Статус здоровья:\n{}", status),
                AdminResponse::PerformanceMetrics(metrics) => format!("Метрики производительности:\n{}", metrics),
                AdminResponse::OperationResult(result) => format!("Результат операции:\n{}", result),
            };
        }
        
        match admin_resp {
            AdminResponse::SystemStats(stats) => {
                format!("## 📊 Системная статистика\n\n```yaml\n{}\n```", stats)
            }
            AdminResponse::HealthStatus(status) => {
                format!("## 🏥 Статус здоровья системы\n\n```\n{}\n```", status)
            }
            AdminResponse::PerformanceMetrics(metrics) => {
                format!("## ⚡ Метрики производительности\n\n```\n{}\n```", metrics)
            }
            AdminResponse::OperationResult(result) => {
                format!("## ✅ Результат операции\n\n```\n{}\n```", result)
            }
        }
    }
    
    fn format_error_response(&self, error: &str, _context: &RequestContext) -> String {
        if !self.use_markdown {
            return format!("❌ Ошибка: {}", error);
        }
        
        format!("## ❌ Ошибка\n\n```\n{}\n```", error)
    }
}

#[async_trait]
impl ResponseFormattingStrategy for RichResponseFormatter {
    async fn format_response(&self, response: &AgentResponse, context: &RequestContext) -> Result<String> {
        debug!("RichResponseFormatter: форматирование rich ответа");
        
        let formatted = match response {
            AgentResponse::Chat(text) => self.format_chat_response(text, context),
            AgentResponse::ToolExecution(text) => self.format_tools_response(text, context),
            AgentResponse::Admin(admin_resp) => self.format_admin_response(admin_resp, context),
            AgentResponse::Error(error) => self.format_error_response(error, context),
        };
        
        info!("RichResponseFormatter: rich ответ отформатирован");
        Ok(formatted)
    }
    
    fn supported_response_types(&self) -> Vec<&'static str> {
        vec!["chat", "tools", "admin", "error"]
    }
}

/// JSON стратегия форматирования для API интеграций
pub struct JsonResponseFormatter {
    pretty_print: bool,
    include_metadata: bool,
}

impl JsonResponseFormatter {
    pub fn new(pretty_print: bool, include_metadata: bool) -> Self {
        Self {
            pretty_print,
            include_metadata,
        }
    }
    
    pub fn with_defaults() -> Self {
        Self::new(true, true)
    }
    
    fn create_json_response(&self, response: &AgentResponse, context: &RequestContext) -> serde_json::Value {
        let mut json = serde_json::json!({
            "response": self.extract_response_data(response),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        
        if self.include_metadata {
            json["metadata"] = serde_json::json!({
                "session_id": context.session_id,
                "original_message": context.message,
                "response_type": self.get_response_type(response),
                "context": context.metadata,
            });
        }
        
        json
    }
    
    fn extract_response_data(&self, response: &AgentResponse) -> serde_json::Value {
        match response {
            AgentResponse::Chat(text) => serde_json::json!({
                "type": "chat",
                "content": text
            }),
            AgentResponse::ToolExecution(text) => serde_json::json!({
                "type": "tool_execution",
                "result": text
            }),
            AgentResponse::Admin(admin_resp) => {
                let (admin_type, content) = match admin_resp {
                    AdminResponse::SystemStats(stats) => ("system_stats", stats),
                    AdminResponse::HealthStatus(status) => ("health_status", status),
                    AdminResponse::PerformanceMetrics(metrics) => ("performance_metrics", metrics),
                    AdminResponse::OperationResult(result) => ("operation_result", result),
                };
                serde_json::json!({
                    "type": "admin",
                    "admin_type": admin_type,
                    "content": content
                })
            }
            AgentResponse::Error(error) => serde_json::json!({
                "type": "error",
                "error_message": error,
                "success": false
            }),
        }
    }
    
    fn get_response_type(&self, response: &AgentResponse) -> &'static str {
        match response {
            AgentResponse::Chat(_) => "chat",
            AgentResponse::ToolExecution(_) => "tool_execution",
            AgentResponse::Admin(_) => "admin",
            AgentResponse::Error(_) => "error",
        }
    }
}

#[async_trait]
impl ResponseFormattingStrategy for JsonResponseFormatter {
    async fn format_response(&self, response: &AgentResponse, context: &RequestContext) -> Result<String> {
        debug!("JsonResponseFormatter: форматирование JSON ответа");
        
        let json_value = self.create_json_response(response, context);
        
        let formatted = if self.pretty_print {
            serde_json::to_string_pretty(&json_value)?
        } else {
            serde_json::to_string(&json_value)?
        };
        
        info!("JsonResponseFormatter: JSON ответ отформатирован");
        Ok(formatted)
    }
    
    fn supported_response_types(&self) -> Vec<&'static str> {
        vec!["chat", "tools", "admin", "error"]
    }
}

/// Адаптивная стратегия форматирования - выбирает формат на основе контекста
pub struct AdaptiveResponseFormatter {
    simple: SimpleResponseFormatter,
    rich: RichResponseFormatter,
    json: JsonResponseFormatter,
}

impl AdaptiveResponseFormatter {
    pub fn new() -> Self {
        Self {
            simple: SimpleResponseFormatter::with_defaults(),
            rich: RichResponseFormatter::with_defaults(),
            json: JsonResponseFormatter::with_defaults(),
        }
    }
    
    fn choose_formatter(&self, context: &RequestContext) -> &dyn ResponseFormattingStrategy {
        // Анализируем контекст для выбора форматировщика
        
        // Если в метаданных есть API флаг, используем JSON
        if context.metadata.get("format").map_or(false, |f| f == "json") ||
           context.metadata.get("api_client").is_some() {
            return &self.json;
        }
        
        // Если сообщение содержит технические термины, используем rich формат
        let message_lower = context.message.to_lowercase();
        let technical_indicators = [
            "статистика", "метрики", "health", "performance",
            "debug", "trace", "error", "exception",
            "api", "json", "xml", "yaml"
        ];
        
        if technical_indicators.iter().any(|&indicator| message_lower.contains(indicator)) {
            return &self.rich;
        }
        
        // По умолчанию используем простой форматировщик
        &self.simple
    }
}

#[async_trait]
impl ResponseFormattingStrategy for AdaptiveResponseFormatter {
    async fn format_response(&self, response: &AgentResponse, context: &RequestContext) -> Result<String> {
        debug!("AdaptiveResponseFormatter: выбор стратегии форматирования");
        
        let formatter = self.choose_formatter(context);
        
        info!("AdaptiveResponseFormatter: выбран форматировщик на основе контекста");
        formatter.format_response(response, context).await
    }
    
    fn supported_response_types(&self) -> Vec<&'static str> {
        vec!["chat", "tools", "admin", "error"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    
    fn create_test_context(message: &str) -> RequestContext {
        RequestContext {
            message: message.to_string(),
            session_id: "test_session".to_string(),
            metadata: HashMap::new(),
        }
    }
    
    #[tokio::test]
    async fn test_simple_formatter() {
        let formatter = SimpleResponseFormatter::with_defaults();
        let context = create_test_context("test message");
        let response = AgentResponse::Chat("Hello world".to_string());
        
        let result = formatter.format_response(&response, &context).await.unwrap();
        assert!(result.contains("💬"));
        assert!(result.contains("Hello world"));
    }
    
    #[tokio::test]
    async fn test_rich_formatter_markdown() {
        let formatter = RichResponseFormatter::with_defaults();
        let context = create_test_context("test message");
        let response = AgentResponse::ToolExecution("ls -la".to_string());
        
        let result = formatter.format_response(&response, &context).await.unwrap();
        assert!(result.contains("```"));
        assert!(result.contains("ls -la"));
    }
    
    #[tokio::test]
    async fn test_json_formatter() {
        let formatter = JsonResponseFormatter::with_defaults();
        let context = create_test_context("test message");
        let response = AgentResponse::Chat("Hello".to_string());
        
        let result = formatter.format_response(&response, &context).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        
        assert_eq!(parsed["response"]["type"], "chat");
        assert_eq!(parsed["response"]["content"], "Hello");
        assert!(parsed["metadata"]["session_id"].is_string());
    }
    
    #[tokio::test]
    async fn test_adaptive_formatter_json_context() {
        let formatter = AdaptiveResponseFormatter::new();
        let mut context = create_test_context("test message");
        context.metadata.insert("format".to_string(), "json".to_string());
        
        let response = AgentResponse::Chat("Hello".to_string());
        let result = formatter.format_response(&response, &context).await.unwrap();
        
        // Должен быть JSON
        assert!(serde_json::from_str::<serde_json::Value>(&result).is_ok());
    }
    
    #[tokio::test]
    async fn test_adaptive_formatter_technical_content() {
        let formatter = AdaptiveResponseFormatter::new();
        let context = create_test_context("покажи статистику системы");
        let response = AgentResponse::Admin(AdminResponse::SystemStats("CPU: 50%".to_string()));
        
        let result = formatter.format_response(&response, &context).await.unwrap();
        
        // Должен быть rich format с markdown
        assert!(result.contains("##"));
        assert!(result.contains("📊"));
    }
    
    #[tokio::test]
    async fn test_all_response_types() {
        let formatter = SimpleResponseFormatter::with_defaults();
        let context = create_test_context("test");
        
        // Тестируем все типы ответов
        let responses = vec![
            AgentResponse::Chat("chat response".to_string()),
            AgentResponse::ToolExecution("tool result".to_string()),
            AgentResponse::Admin(AdminResponse::SystemStats("stats".to_string())),
            AgentResponse::Error("error message".to_string()),
        ];
        
        for response in responses {
            let result = formatter.format_response(&response, &context).await;
            assert!(result.is_ok());
        }
    }
}