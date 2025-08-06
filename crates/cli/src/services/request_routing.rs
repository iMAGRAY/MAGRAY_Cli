//! Request Routing Service - маршрутизация запросов
//! 
//! Сервис отвечает за принятие решения о том, как обработать запрос:
//! - Чат с LLM 
//! - Выполнение инструментов через SmartRouter
//! - Смешанные операции
//! 
//! Работает на основе результатов IntentAnalysisService и применяет
//! business logic для выбора оптимального способа обработки.

use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};
use router::SmartRouter;
use super::types::{IntentDecision, AgentResponse, RequestContext, OperationResult};

/// Trait для сервиса маршрутизации запросов
#[async_trait::async_trait]
pub trait RequestRoutingService: Send + Sync {
    /// Маршрутизировать запрос на основе анализа намерений
    async fn route_request(
        &self,
        context: &RequestContext,
        intent: &IntentDecision,
    ) -> Result<OperationResult<AgentResponse>>;
    
    /// Получить рекомендацию по маршрутизации без выполнения
    async fn recommend_routing(
        &self,
        context: &RequestContext,
        intent: &IntentDecision,
    ) -> Result<RoutingRecommendation>;
    
    /// Получить статистику маршрутизации
    async fn get_routing_stats(&self) -> RoutingStats;
    
    /// Сбросить статистику (для тестов)
    async fn reset_stats(&self);
}

/// Рекомендация по маршрутизации запроса
#[derive(Debug, Clone)]
pub struct RoutingRecommendation {
    pub route_type: RouteType,
    pub confidence: f64,
    pub reasoning: String,
    pub estimated_duration: Duration,
    pub resource_requirements: ResourceRequirements,
}

/// Тип маршрута для обработки запроса
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteType {
    /// Простой чат с LLM
    DirectChat,
    /// Выполнение инструментов через SmartRouter
    ToolExecution,
    /// Гибридный подход (чат + инструменты)
    Hybrid,
    /// Отклонить запрос (например, из-за нарушения политики)
    Reject,
}

/// Требования к ресурсам для выполнения запроса
#[derive(Debug, Clone)]
pub struct ResourceRequirements {
    pub estimated_memory_mb: u64,
    pub estimated_cpu_cores: f32,
    pub requires_network: bool,
    pub requires_file_system: bool,
    pub estimated_tokens: Option<u64>,
}

/// Статистика работы сервиса маршрутизации
#[derive(Debug, Clone)]
pub struct RoutingStats {
    pub total_requests: u64,
    pub route_distribution: std::collections::HashMap<RouteType, u64>,
    pub avg_routing_time_ms: f64,
    pub successful_routes: u64,
    pub failed_routes: u64,
    pub rejected_routes: u64,
}

/// Реализация сервиса маршрутизации по умолчанию
pub struct DefaultRequestRoutingService {
    /// SmartRouter для выполнения инструментов
    smart_router: SmartRouter,
    
    /// Статистика работы
    stats: parking_lot::RwLock<RoutingStats>,
    
    /// Конфигурация маршрутизации
    config: RoutingConfig,
}

/// Конфигурация сервиса маршрутизации
#[derive(Debug, Clone)]
pub struct RoutingConfig {
    /// Минимальная уверенность для прямого выполнения
    pub min_confidence_direct: f64,
    
    /// Порог для выбора гибридного подхода
    pub hybrid_threshold: f64,
    
    /// Максимальное время выполнения запросов
    pub max_execution_time: Duration,
    
    /// Включить ли интеллектуальную маршрутизацию
    pub enable_smart_routing: bool,
    
    /// Список запрещённых паттернов
    pub blocked_patterns: Vec<String>,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            min_confidence_direct: 0.7,
            hybrid_threshold: 0.5,
            max_execution_time: Duration::from_secs(120),
            enable_smart_routing: true,
            blocked_patterns: vec![
                "rm -rf".to_string(),
                "format c:".to_string(),
                "delete system32".to_string(),
            ],
        }
    }
}

impl DefaultRequestRoutingService {
    /// Создать новый экземпляр сервиса
    pub fn new(smart_router: SmartRouter) -> Self {
        Self::with_config(smart_router, RoutingConfig::default())
    }
    
    /// Создать экземпляр с кастомной конфигурацией
    pub fn with_config(smart_router: SmartRouter, config: RoutingConfig) -> Self {
        Self {
            smart_router,
            stats: parking_lot::RwLock::new(RoutingStats::default()),
            config,
        }
    }
    
    /// Проверить, не заблокирован ли запрос
    fn is_request_blocked(&self, message: &str) -> bool {
        let message_lower = message.to_lowercase();
        self.config.blocked_patterns.iter()
            .any(|pattern| message_lower.contains(&pattern.to_lowercase()))
    }
    
    /// Применить business logic для выбора маршрута
    fn determine_route_type(&self, intent: &IntentDecision, context: &RequestContext) -> RouteType {
        // Проверка на заблокированные запросы
        if self.is_request_blocked(&context.message) {
            warn!("🚫 Запрос заблокирован по безопасности: {}", context.message);
            return RouteType::Reject;
        }
        
        let confidence = intent.confidence;
        let action_type = intent.action_type.as_str();
        
        match action_type {
            "chat" => {
                if confidence >= self.config.min_confidence_direct {
                    RouteType::DirectChat
                } else if confidence >= self.config.hybrid_threshold {
                    // Низкая уверенность в чате - возможно, стоит предложить инструменты
                    RouteType::Hybrid
                } else {
                    RouteType::DirectChat // Fallback
                }
            }
            "tools" => {
                if confidence >= self.config.min_confidence_direct {
                    RouteType::ToolExecution
                } else if confidence >= self.config.hybrid_threshold {
                    // Низкая уверенность в инструментах - добавим чат для уточнения
                    RouteType::Hybrid
                } else {
                    RouteType::ToolExecution // Fallback
                }
            }
            _ => {
                // Неизвестный тип намерения - используем гибридный подход
                warn!("⚠️ Неизвестный тип намерения: {}, используем гибридный подход", action_type);
                RouteType::Hybrid
            }
        }
    }
    
    /// Оценить требования к ресурсам
    fn estimate_resource_requirements(&self, route_type: &RouteType, context: &RequestContext) -> ResourceRequirements {
        match route_type {
            RouteType::DirectChat => ResourceRequirements {
                estimated_memory_mb: 50,
                estimated_cpu_cores: 0.5,
                requires_network: true,
                requires_file_system: false,
                estimated_tokens: Some(context.message.len() as u64 * 2), // Примерная оценка
            },
            RouteType::ToolExecution => ResourceRequirements {
                estimated_memory_mb: 100,
                estimated_cpu_cores: 1.0,
                requires_network: true,
                requires_file_system: true,
                estimated_tokens: Some(context.message.len() as u64 * 4),
            },
            RouteType::Hybrid => ResourceRequirements {
                estimated_memory_mb: 150,
                estimated_cpu_cores: 1.5,
                requires_network: true,
                requires_file_system: true,
                estimated_tokens: Some(context.message.len() as u64 * 6),
            },
            RouteType::Reject => ResourceRequirements {
                estimated_memory_mb: 1,
                estimated_cpu_cores: 0.01,
                requires_network: false,
                requires_file_system: false,
                estimated_tokens: None,
            },
        }
    }
    
    /// Обновить статистику
    fn update_stats(&self, route_type: RouteType, duration: Duration, success: bool) {
        let mut stats = self.stats.write();
        stats.total_requests += 1;
        
        *stats.route_distribution.entry(route_type.clone()).or_insert(0) += 1;
        
        if success {
            stats.successful_routes += 1;
        } else {
            stats.failed_routes += 1;
        }
        
        if route_type == RouteType::Reject {
            stats.rejected_routes += 1;
        }
        
        // Обновляем среднее время маршрутизации
        let duration_ms = duration.as_millis() as f64;
        let total = stats.total_requests as f64;
        stats.avg_routing_time_ms = ((stats.avg_routing_time_ms * (total - 1.0)) + duration_ms) / total;
    }
}

#[async_trait::async_trait]
impl RequestRoutingService for DefaultRequestRoutingService {
    async fn route_request(
        &self,
        context: &RequestContext,
        intent: &IntentDecision,
    ) -> Result<OperationResult<AgentResponse>> {
        use std::time::Instant;
        use tokio::time::timeout;
        
        let start_time = Instant::now();
        
        debug!("🔀 Маршрутизация запроса: {}", context.message);
        
        let route_type = self.determine_route_type(intent, context);
        
        debug!("📍 Выбран маршрут: {:?} (confidence: {:.2})", route_type, intent.confidence);
        
        let result = match route_type {
            RouteType::DirectChat => {
                // Для чата возвращаем специальный ответ, который будет обработан LlmCommunicationService
                Ok(AgentResponse::Chat(format!(
                    "ROUTE_TO_CHAT: {}",
                    context.message
                )))
            }
            RouteType::ToolExecution => {
                // Выполняем через SmartRouter с timeout
                info!("🔧 Выполнение через SmartRouter: {}", context.message);
                let router_future = self.smart_router.process_smart_request(&context.message);
                match timeout(self.config.max_execution_time, router_future).await {
                    Ok(Ok(result)) => Ok(AgentResponse::ToolExecution(result)),
                    Ok(Err(e)) => Err(e),
                    Err(_) => Err(anyhow::anyhow!(
                        "SmartRouter timeout after {:?}",
                        self.config.max_execution_time
                    )),
                }
            }
            RouteType::Hybrid => {
                // Гибридный подход - сначала инструменты, потом чат для объяснения
                info!("🔀 Гибридный подход: {}", context.message);
                let router_future = self.smart_router.process_smart_request(&context.message);
                match timeout(self.config.max_execution_time, router_future).await {
                    Ok(Ok(tool_result)) => {
                        // Комбинируем результат инструментов с запросом на чат
                        let combined_response = format!(
                            "HYBRID_RESULT: {}\nCHAT_FOLLOWUP: {}",
                            tool_result,
                            context.message
                        );
                        Ok(AgentResponse::ToolExecution(combined_response))
                    }
                    Ok(Err(_)) => {
                        // Fallback на чат если инструменты не сработали
                        warn!("⚠️ Инструменты failed в гибридном режиме, fallback на чат");
                        Ok(AgentResponse::Chat(format!(
                            "ROUTE_TO_CHAT: {}",
                            context.message
                        )))
                    }
                    Err(_) => {
                        warn!("⚠️ Timeout в гибридном режиме, fallback на чат");
                        Ok(AgentResponse::Chat(format!(
                            "ROUTE_TO_CHAT: {}",
                            context.message
                        )))
                    }
                }
            }
            RouteType::Reject => {
                warn!("🚫 Запрос отклонён: {}", context.message);
                Err(anyhow::anyhow!(
                    "Запрос отклонён по политике безопасности"
                ))
            }
        };
        
        let duration = start_time.elapsed();
        let success = result.is_ok();
        
        self.update_stats(route_type, duration, success);
        
        Ok(OperationResult {
            result,
            duration,
            retries: 0, // Пока без retry логики на уровне маршрутизации
            from_cache: false,
        })
    }
    
    async fn recommend_routing(
        &self,
        context: &RequestContext,
        intent: &IntentDecision,
    ) -> Result<RoutingRecommendation> {
        let route_type = self.determine_route_type(intent, context);
        let resource_requirements = self.estimate_resource_requirements(&route_type, context);
        
        let estimated_duration = match route_type {
            RouteType::DirectChat => Duration::from_secs(10),
            RouteType::ToolExecution => Duration::from_secs(30),
            RouteType::Hybrid => Duration::from_secs(45),
            RouteType::Reject => Duration::from_millis(1),
        };
        
        let reasoning = match route_type {
            RouteType::DirectChat => "Высокая уверенность в чат намерении".to_string(),
            RouteType::ToolExecution => "Чёткое намерение выполнить инструменты".to_string(),
            RouteType::Hybrid => "Неоднозначное намерение, комбинированный подход".to_string(),
            RouteType::Reject => "Запрос нарушает политику безопасности".to_string(),
        };
        
        Ok(RoutingRecommendation {
            route_type,
            confidence: intent.confidence,
            reasoning,
            estimated_duration,
            resource_requirements,
        })
    }
    
    async fn get_routing_stats(&self) -> RoutingStats {
        let stats = self.stats.read();
        stats.clone()
    }
    
    async fn reset_stats(&self) {
        let mut stats = self.stats.write();
        *stats = RoutingStats::default();
        debug!("🔄 Routing stats reset");
    }
}

impl Default for RoutingStats {
    fn default() -> Self {
        Self {
            total_requests: 0,
            route_distribution: std::collections::HashMap::new(),
            avg_routing_time_ms: 0.0,
            successful_routes: 0,
            failed_routes: 0,
            rejected_routes: 0,
        }
    }
}

/// Factory функция для DI контейнера
pub fn create_request_routing_service(
    smart_router: SmartRouter,
) -> Arc<dyn RequestRoutingService> {
    Arc::new(DefaultRequestRoutingService::new(smart_router))
}

/// Factory функция с кастомной конфигурацией
pub fn create_request_routing_service_with_config(
    smart_router: SmartRouter,
    config: RoutingConfig,
) -> Arc<dyn RequestRoutingService> {
    Arc::new(DefaultRequestRoutingService::with_config(smart_router, config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    
    fn create_test_context(message: &str) -> RequestContext {
        RequestContext {
            message: message.to_string(),
            user_id: Some("test_user".to_string()),
            session_id: Some("test_session".to_string()),  
            timestamp: Utc::now(),
            metadata: std::collections::HashMap::new(),
        }
    }
    
    fn create_test_intent(action_type: &str, confidence: f64) -> IntentDecision {
        IntentDecision {
            action_type: action_type.to_string(),
            confidence,
            reasoning: "Test decision".to_string(),
            extracted_params: std::collections::HashMap::new(),
        }
    }
    
    #[test]
    fn test_blocked_requests() {
        // Создаём минимальный мок SmartRouter
        // В реальном коде это был бы полноценный mock
        let smart_router = SmartRouter::new(llm::LlmClient::from_env().unwrap());
        let service = DefaultRequestRoutingService::new(smart_router);
        
        assert!(service.is_request_blocked("rm -rf /"));
        assert!(service.is_request_blocked("Please format c: drive"));
        assert!(!service.is_request_blocked("Покажи файлы в папке"));
    }
    
    #[test]
    fn test_route_type_determination() {
        let smart_router = SmartRouter::new(llm::LlmClient::from_env().unwrap());
        let service = DefaultRequestRoutingService::new(smart_router);
        
        let context = create_test_context("прочитай файл test.rs");
        
        // Высокая уверенность в инструментах
        let intent = create_test_intent("tools", 0.9);
        let route_type = service.determine_route_type(&intent, &context);
        assert_eq!(route_type, RouteType::ToolExecution);
        
        // Высокая уверенность в чате
        let intent = create_test_intent("chat", 0.9);
        let route_type = service.determine_route_type(&intent, &context);
        assert_eq!(route_type, RouteType::DirectChat);
        
        // Низкая уверенность - гибридный подход
        let intent = create_test_intent("tools", 0.4);
        let route_type = service.determine_route_type(&intent, &context);
        assert_eq!(route_type, RouteType::ToolExecution); // Fallback
    }
    
    #[test]
    fn test_resource_estimation() {
        let smart_router = SmartRouter::new(llm::LlmClient::from_env().unwrap());
        let service = DefaultRequestRoutingService::new(smart_router);
        
        let context = create_test_context("привет мир");
        
        let chat_resources = service.estimate_resource_requirements(&RouteType::DirectChat, &context);
        assert_eq!(chat_resources.estimated_memory_mb, 50);
        assert!(chat_resources.requires_network);
        assert!(!chat_resources.requires_file_system);
        
        let tool_resources = service.estimate_resource_requirements(&RouteType::ToolExecution, &context);
        assert_eq!(tool_resources.estimated_memory_mb, 100);
        assert!(tool_resources.requires_file_system);
    }
}