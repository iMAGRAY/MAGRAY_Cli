//! Service layer для разделения ответственностей UnifiedAgent
//!
//! Этот модуль содержит trait-based сервисы, которые разделяют
//! ответственности бывшего God Object UnifiedAgent:
//!
//! - `IntentAnalysisService` - анализ намерений пользователя
//! - `RequestRoutingService` - маршрутизация запросов
//! - `LlmCommunicationService` - взаимодействие с LLM провайдерами
//! - `ResilienceService` - обработка ошибок и retry логика
//! - `ServiceOrchestrator` - координация между сервисами

#![allow(dead_code)] // Allow unused code during development

pub mod di_config;
pub mod intent_analysis;
pub mod llm_communication;
pub mod orchestration_service;
pub mod orchestrator;
pub mod request_routing;
pub mod resilience;

// Re-export главных traits для удобства
pub use intent_analysis::IntentAnalysisService;
pub use llm_communication::LlmCommunicationService;
pub use orchestration_service::OrchestrationService;
pub use orchestrator::ServiceOrchestrator;
pub use request_routing::RequestRoutingService;

/// Общие типы для всех сервисов
pub mod types {
    use anyhow::Result;

    /// Результат анализа намерений пользователя
    #[derive(Debug, Clone)]
    pub struct IntentDecision {
        pub action_type: String,
        pub confidence: f64,
        pub reasoning: String,
        pub extracted_params: std::collections::HashMap<String, String>,
    }

    /// Тип ответа агента
    #[derive(Debug, Clone)]
    pub enum AgentResponse {
        Chat(String),
        ToolExecution(String),
    }

    /// Контекст выполнения запроса
    #[derive(Debug, Clone)]
    pub struct RequestContext {
        pub message: String,
        pub user_id: Option<String>,
        pub session_id: Option<String>,
        pub timestamp: chrono::DateTime<chrono::Utc>,
        pub metadata: std::collections::HashMap<String, String>,
    }

    /// Результат выполнения операции с метриками
    #[derive(Debug)]
    pub struct OperationResult<T> {
        pub result: Result<T>,
        pub duration: std::time::Duration,
        pub retries: u32,
        pub from_cache: bool,
    }
}
