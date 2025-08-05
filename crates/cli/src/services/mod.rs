// @component: {"k":"C","id":"cli_services","t":"Service layer for agent decomposition","m":{"cur":80,"tgt":100,"u":"%"},"f":["services","traits","separation","orchestration"]}
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

pub mod intent_analysis;
pub mod request_routing;
pub mod llm_communication;
pub mod resilience;
pub mod orchestrator;
pub mod di_config;

// Re-export главных traits для удобства
pub use intent_analysis::{IntentAnalysisService, DefaultIntentAnalysisService};
pub use request_routing::{RequestRoutingService, DefaultRequestRoutingService};
pub use llm_communication::{LlmCommunicationService, DefaultLlmCommunicationService};
pub use resilience::{ResilienceService, DefaultResilienceService};
pub use orchestrator::{ServiceOrchestrator, DefaultServiceOrchestrator};
pub use di_config::{register_services, create_services_container};

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