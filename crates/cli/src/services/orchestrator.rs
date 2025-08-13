//! Service Orchestrator - координация между сервисами
//!
//! Центральный оркестратор, который координирует взаимодействие
//! между всеми специализированными сервисами. Реализует основную
//! бизнес-логику обработки запросов пользователя.

use super::types::{AgentResponse, RequestContext};
use super::{
    resilience::DefaultResilienceService, IntentAnalysisService, LlmCommunicationService,
    RequestRoutingService,
};
use anyhow::Result;
use memory::di::UnifiedContainer as DIMemoryService;
use std::sync::Arc;
use tracing::{debug, info};

/// Trait для оркестратора сервисов
#[async_trait::async_trait]
pub trait ServiceOrchestrator: Send + Sync {
    /// Обработать сообщение пользователя (основной метод)
    async fn process_message(&self, message: &str) -> Result<AgentResponse>;

    /// Получить статистику оркестратора
    async fn get_orchestrator_stats(&self) -> OrchestratorStats;

    /// Проверить здоровье всех сервисов
    async fn health_check(&self) -> SystemHealthStatus;
}

/// Статистика оркестратора
#[derive(Debug, Clone)]
pub struct OrchestratorStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_processing_time_ms: f64,
    pub service_performance: std::collections::HashMap<String, ServicePerformance>,
}

/// Производительность отдельного сервиса
#[derive(Debug, Clone)]
pub struct ServicePerformance {
    pub avg_response_time_ms: f64,
    pub error_rate: f64,
    pub last_error: Option<String>,
}

/// Общий статус здоровья системы
#[derive(Debug, Clone)]
pub struct SystemHealthStatus {
    pub overall_healthy: bool,
    pub service_statuses: std::collections::HashMap<String, bool>,
    pub error_messages: Vec<String>,
}

/// Реализация оркестратора сервисов по умолчанию
pub struct DefaultServiceOrchestrator {
    intent_analysis: Arc<dyn IntentAnalysisService>,
    request_routing: Arc<dyn RequestRoutingService>,
    llm_communication: Arc<dyn LlmCommunicationService>,
    resilience: Arc<DefaultResilienceService>,
    memory_service: DIMemoryService,
    stats: parking_lot::RwLock<OrchestratorStats>,
}

impl DefaultServiceOrchestrator {
    pub fn new(
        intent_analysis: Arc<dyn IntentAnalysisService>,
        request_routing: Arc<dyn RequestRoutingService>,
        llm_communication: Arc<dyn LlmCommunicationService>,
        resilience: Arc<DefaultResilienceService>,
        memory_service: DIMemoryService,
    ) -> Self {
        Self {
            intent_analysis,
            request_routing,
            llm_communication,
            resilience,
            memory_service,
            stats: parking_lot::RwLock::new(OrchestratorStats::default()),
        }
    }

    fn create_request_context(&self, message: &str) -> RequestContext {
        RequestContext {
            message: message.to_string(),
            user_id: Some("current_user".to_string()),
            session_id: Some("current_session".to_string()),
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        }
    }

    fn update_stats(&self, success: bool, duration: std::time::Duration) {
        let mut stats = self.stats.write();
        stats.total_requests += 1;

        if success {
            stats.successful_requests += 1;
        } else {
            stats.failed_requests += 1;
        }

        let duration_ms = duration.as_millis() as f64;
        let total = stats.total_requests as f64;
        stats.avg_processing_time_ms =
            ((stats.avg_processing_time_ms * (total - 1.0)) + duration_ms) / total;
    }
}

#[async_trait::async_trait]
impl ServiceOrchestrator for DefaultServiceOrchestrator {
    async fn process_message(&self, message: &str) -> Result<AgentResponse> {
        use std::time::Instant;
        let start_time = Instant::now();

        info!("🤖 Начало обработки сообщения: {} символов", message.len());

        let context = self.create_request_context(message);

        // 1. Анализ намерений
        debug!("📝 Этап 1: Анализ намерений");
        let intent = self.intent_analysis.analyze_intent(&context).await?;

        // 2. Маршрутизация запроса
        debug!("🔀 Этап 2: Маршрутизация запроса");
        let routing_result = self
            .request_routing
            .route_request(&context, &intent)
            .await?;

        let response = match routing_result.result? {
            AgentResponse::Chat(content) => {
                // 3a. LLM коммуникация для чата
                if content.starts_with("ROUTE_TO_CHAT:") {
                    debug!("💬 Этап 3a: LLM коммуникация");
                    let actual_message =
                        content.strip_prefix("ROUTE_TO_CHAT: ").unwrap_or(&content);
                    let chat_context = RequestContext {
                        message: actual_message.to_string(),
                        ..context
                    };
                    let llm_result = self.llm_communication.chat(&chat_context).await?;
                    AgentResponse::Chat(llm_result.result?)
                } else {
                    AgentResponse::Chat(content)
                }
            }
            AgentResponse::ToolExecution(content) => {
                // 3b. Обработка результатов инструментов
                debug!("🔧 Этап 3b: Результат выполнения инструментов");

                if content.starts_with("HYBRID_RESULT:") {
                    // Гибридный результат - извлекаем части
                    let parts: Vec<&str> = content.splitn(2, "\nCHAT_FOLLOWUP: ").collect();
                    if parts.len() == 2 {
                        let tool_result =
                            parts[0].strip_prefix("HYBRID_RESULT: ").unwrap_or(parts[0]);
                        let _chat_query = parts[1]; // Will be used for followup chat

                        // Делаем followup чат запрос для объяснения результатов
                        let chat_context = RequestContext {
                            message: format!("Объясни результат: {tool_result}"),
                            ..context
                        };
                        let llm_result = self.llm_communication.chat(&chat_context).await?;

                        let combined = format!("{}\n\n{}", tool_result, llm_result.result?);
                        AgentResponse::ToolExecution(combined)
                    } else {
                        AgentResponse::ToolExecution(content)
                    }
                } else {
                    AgentResponse::ToolExecution(content)
                }
            }
        };

        let duration = start_time.elapsed();
        self.update_stats(true, duration);

        info!("✅ Сообщение обработано за {:?}", duration);
        Ok(response)
    }

    async fn get_orchestrator_stats(&self) -> OrchestratorStats {
        let stats = self.stats.read();
        stats.clone()
    }

    async fn health_check(&self) -> SystemHealthStatus {
        let mut service_statuses = std::collections::HashMap::new();
        let mut error_messages = Vec::new();

        // Проверяем каждый сервис
        let llm_health = self.llm_communication.health_check().await;
        service_statuses.insert(
            "llm_communication".to_string(),
            llm_health.primary_provider_healthy,
        );

        if !llm_health.primary_provider_healthy {
            error_messages.push("LLM communication service unhealthy".to_string());
        }

        // Проверяем память (упрощенная проверка для compilability)
        service_statuses.insert("memory".to_string(), true);
        // TODO: Реализовать check_health для DIContainer после добавления метода

        // Общий статус - все сервисы должны быть здоровы
        let overall_healthy = service_statuses.values().all(|&status| status);

        SystemHealthStatus {
            overall_healthy,
            service_statuses,
            error_messages,
        }
    }
}

impl Default for OrchestratorStats {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            avg_processing_time_ms: 0.0,
            service_performance: std::collections::HashMap::new(),
        }
    }
}

/// Factory функция для DI контейнера
pub fn create_service_orchestrator(
    intent_analysis: Arc<dyn IntentAnalysisService>,
    request_routing: Arc<dyn RequestRoutingService>,
    llm_communication: Arc<dyn LlmCommunicationService>,
    resilience: Arc<DefaultResilienceService>,
    memory_service: DIMemoryService,
) -> Arc<dyn ServiceOrchestrator> {
    Arc::new(DefaultServiceOrchestrator::new(
        intent_analysis,
        request_routing,
        llm_communication,
        resilience,
        memory_service,
    ))
}
