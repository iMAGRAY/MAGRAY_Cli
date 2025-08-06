//! UnifiedAgent V2 - Clean Architecture Implementation
//! 
//! Полная реализация Clean Architecture с применением:
//! - SOLID принципов
//! - Dependency Inversion через DI контейнер
//! - Strategy patterns для различных стратегий
//! - Circuit Breaker patterns для resilience
//! - Comprehensive error handling

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, debug, error, warn};

use crate::agent_traits::*;
use crate::handlers::*;
use crate::strategies::*;
use crate::orchestrator::*;
use tools::enhanced_tool_system::{EnhancedToolSystem, EnhancedToolSystemConfig};

// ============================================================================
// ADAPTER IMPLEMENTATIONS FOR EXISTING SERVICES
// ============================================================================

/// Adapter для LlmClient -> LlmServiceTrait  
pub struct LlmServiceAdapter {
    llm_client: llm::LlmClient,
}

impl LlmServiceAdapter {
    pub fn new() -> Result<Self> {
        let llm_client = llm::LlmClient::from_env_multi()
            .or_else(|_| {
                info!("🔄 Multi-provider setup failed, falling back to single provider");
                llm::LlmClient::from_env()
            })?;
        
        if llm_client.is_multi_provider() {
            info!("✅ LlmServiceAdapter using multi-provider orchestration");
        } else {
            info!("✅ LlmServiceAdapter using single provider");
        }
        
        Ok(Self { llm_client })
    }
    
    /// Get LLM status report if available
    pub async fn get_status_report(&self) -> Option<String> {
        self.llm_client.get_status_report().await
    }
}

#[async_trait]
impl LlmServiceTrait for LlmServiceAdapter {
    async fn chat(&self, message: &str) -> Result<String> {
        self.llm_client.chat_simple(message).await
    }
    
    async fn chat_with_context(&self, message: &str, context: &HashMap<String, String>) -> Result<String> {
        // Простая реализация - добавляем контекст к сообщению
        let context_str = context.iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join(", ");
        
        let full_message = if context_str.is_empty() {
            message.to_string()
        } else {
            format!("{}\n\nКонтекст: {}", message, context_str)
        };
        
        self.llm_client.chat_simple(&full_message).await
    }
    
    async fn health_check(&self) -> Result<()> {
        // Простая проверка через тестовый запрос
        self.llm_client.chat_simple("ping").await?;
        Ok(())
    }
}

/// Adapter для SmartRouter -> IntelligentRoutingTrait
pub struct IntelligentRoutingAdapter {
    smart_router: router::SmartRouter,
}

impl IntelligentRoutingAdapter {
    pub fn new(smart_router: router::SmartRouter) -> Self {
        Self { smart_router }
    }
}

#[async_trait]
impl IntelligentRoutingTrait for IntelligentRoutingAdapter {
    async fn process_request(&self, query: &str) -> Result<String> {
        self.smart_router.process_smart_request(query).await
    }
    
    async fn analyze_request(&self, query: &str) -> Result<String> {
        let plan = self.smart_router.analyze_and_plan(query).await?;
        Ok(format!("Plan: {} (confidence: {:.2})", plan.reasoning, plan.confidence))
    }
    
    async fn health_check(&self) -> Result<()> {
        // Здесь можно добавить специфичную проверку для SmartRouter
        Ok(())
    }
}

/// Adapter для DIMemoryService -> MemoryManagementTrait
pub struct MemoryManagementAdapter {
    memory_service: memory::DIMemoryService,
}

impl MemoryManagementAdapter {
    pub fn new(memory_service: memory::DIMemoryService) -> Self {
        Self { memory_service }
    }
}

#[async_trait]
impl MemoryManagementTrait for MemoryManagementAdapter {
    async fn store_message(&self, message: &str, context: &HashMap<String, String>) -> Result<()> {
        use memory::{Record, Layer};
        use uuid::Uuid;
        use chrono::Utc;
        
        let record = Record {
            id: Uuid::new_v4(),
            text: message.to_string(),
            embedding: vec![], // Будет создан автоматически
            layer: Layer::Interact,
            kind: context.get("kind").unwrap_or(&"user_message".to_string()).clone(),
            tags: vec!["chat".to_string()],
            project: context.get("project").unwrap_or(&"magray".to_string()).clone(),
            session: context.get("session").unwrap_or(&"current".to_string()).clone(),
            ts: Utc::now(),
            score: 0.0,
            access_count: 1,
            last_access: Utc::now(),
        };
        
        self.memory_service.insert(record).await
            .map_err(|e| anyhow::anyhow!("Ошибка сохранения в память: {}", e))
    }
    
    async fn search_memory(&self, query: &str, limit: usize) -> Result<Vec<String>> {
        use memory::{Layer, SearchOptions};
        
        let search_options = SearchOptions {
            layers: vec![Layer::Insights],
            top_k: limit,
            score_threshold: 0.7,
            tags: vec![],
            project: Some("magray".to_string()),
        };
        
        let results = self.memory_service.search(query, Layer::Insights, search_options).await
            .map_err(|e| anyhow::anyhow!("Ошибка поиска в памяти: {}", e))?;
        
        Ok(results.into_iter().map(|record| record.text).collect())
    }
    
    async fn run_promotion(&self) -> Result<String> {
        let stats = self.memory_service.run_promotion().await
            .map_err(|e| anyhow::anyhow!("Ошибка promotion: {}", e))?;
        
        Ok(format!("Promotion завершен: {} → Insights, {} → Assets", 
                   stats.interact_to_insights, stats.insights_to_assets))
    }
    
    async fn get_memory_stats(&self) -> Result<String> {
        let stats = self.memory_service.get_stats().await;
        Ok(format!("Memory Stats: {:?}", stats))
    }
    
    async fn health_check(&self) -> Result<()> {
        let health = self.memory_service.check_health().await
            .map_err(|e| anyhow::anyhow!("Memory health check failed: {}", e))?;
        
        if !matches!(health.overall_status, memory::health::HealthStatus::Healthy) {
            return Err(anyhow::anyhow!("Memory system unhealthy: {:?}", health.overall_status));
        }
        
        Ok(())
    }
}

/// Простая реализация AdminServiceTrait
pub struct BasicAdminService {
    performance_monitor: Arc<PerformanceMonitor>,
}

impl BasicAdminService {
    pub fn new(performance_monitor: Arc<PerformanceMonitor>) -> Self {
        Self { performance_monitor }
    }
}

#[async_trait]
impl AdminServiceTrait for BasicAdminService {
    async fn get_system_stats(&self) -> Result<AdminResponse> {
        let stats = format!(
            "System Statistics:\n\
             ├─ Active operations: {:?}\n\
             ├─ Performance metrics available\n\
             └─ System healthy",
            self.performance_monitor.get_active_operations()?
        );
        Ok(AdminResponse::SystemStats(stats))
    }
    
    async fn check_system_health(&self) -> Result<AdminResponse> {
        // Проверяем компоненты
        self.performance_monitor.health_check().await?;
        
        let status = "All systems operational ✅";
        Ok(AdminResponse::HealthStatus(status.to_string()))
    }
    
    async fn get_performance_metrics(&self) -> Result<AdminResponse> {
        let metrics = self.performance_monitor.get_detailed_metrics(60).await?;
        Ok(AdminResponse::PerformanceMetrics(metrics))
    }
    
    async fn execute_admin_command(&self, command: &str, args: &HashMap<String, String>) -> Result<AdminResponse> {
        let result = match command {
            "reset_metrics" => {
                self.performance_monitor.reset_metrics();
                "Metrics reset successfully".to_string()
            }
            "cleanup_old_metrics" => {
                let hours = args.get("hours")
                    .and_then(|h| h.parse::<u64>().ok())
                    .unwrap_or(24);
                let cleaned = self.performance_monitor.cleanup_old_metrics(hours)?;
                format!("Cleaned {} old metrics", cleaned)
            }
            _ => format!("Unknown command: {}", command)
        };
        
        Ok(AdminResponse::OperationResult(result))
    }
}

// ============================================================================
// UNIFIED AGENT V2 - CLEAN ARCHITECTURE
// ============================================================================

/// UnifiedAgent V2 с Clean Architecture
pub struct UnifiedAgentV2 {
    // Specialized handlers (Dependency Injection)
    chat_handler: ChatHandler<LlmServiceAdapter, BasicCircuitBreaker>,
    tools_handler: ToolsHandler<IntelligentRoutingAdapter, BasicCircuitBreaker>,
    memory_handler: MemoryHandler<MemoryManagementAdapter, BasicCircuitBreaker>,
    admin_handler: AdminHandler<BasicAdminService, BasicCircuitBreaker>,
    
    // Strategy patterns
    intent_strategy: Box<dyn IntentDecisionStrategy>,
    fallback_strategy: CompositeFallbackStrategy,
    response_strategy: Box<dyn ResponseFormattingStrategy>,
    
    // Integrated Tool Orchestrator (replaces simple task orchestrator)
    tool_orchestrator: ToolOrchestrator,
    
    // Performance monitoring
    performance_monitor: Arc<PerformanceMonitor>,
    
    // Circuit breakers for each component
    chat_circuit_breaker: BasicCircuitBreaker,
    tools_circuit_breaker: BasicCircuitBreaker,
    memory_circuit_breaker: BasicCircuitBreaker,
    admin_circuit_breaker: BasicCircuitBreaker,
    
    // State
    initialized: bool,
}

impl UnifiedAgentV2 {
    /// Создание нового UnifiedAgentV2 через DI
    pub async fn new() -> Result<Self> {
        info!("🏗️ Создание UnifiedAgentV2 с Clean Architecture");
        
        // Создаем performance monitor
        let performance_monitor = Arc::new(PerformanceMonitor::new());
        
        // Создаем circuit breakers для каждого компонента
        let chat_circuit_breaker = BasicCircuitBreaker::with_defaults("chat_handler".to_string());
        let tools_circuit_breaker = BasicCircuitBreaker::with_defaults("tools_handler".to_string());
        let memory_circuit_breaker = BasicCircuitBreaker::with_defaults("memory_handler".to_string());
        let admin_circuit_breaker = BasicCircuitBreaker::with_defaults("admin_handler".to_string());
        
        // Создаем адаптеры для существующих сервисов
        let llm_adapter = LlmServiceAdapter::new()?;
        
        // Создаем SmartRouter с новым LLM client
        let llm_client_for_router = llm::LlmClient::from_env_multi()
            .or_else(|_| llm::LlmClient::from_env())?;
        let smart_router = router::SmartRouter::new(llm_client_for_router);
        let routing_adapter = IntelligentRoutingAdapter::new(smart_router);
        
        let memory_config = memory::default_config()?;
        let memory_service = memory::DIMemoryService::new(memory_config).await
            .map_err(|e| anyhow::anyhow!("Ошибка создания DIMemoryService: {}", e))?;
        memory_service.initialize().await
            .map_err(|e| anyhow::anyhow!("Ошибка инициализации памяти: {}", e))?;
        let memory_adapter = MemoryManagementAdapter::new(memory_service);
        
        let admin_service = BasicAdminService::new(performance_monitor.clone());
        
        // Создаем strategy patterns
        let intent_llm_adapter = LlmServiceAdapter::new()?;
        let intent_strategy: Box<dyn IntentDecisionStrategy> = Box::new(
            HybridIntentStrategy::new(intent_llm_adapter, 0.7)
        );
        
        // Создаем отдельный адаптер для fallback strategy
        let fallback_llm_adapter = LlmServiceAdapter::new()?;
        let mut fallback_strategy = CompositeFallbackStrategy::new();
        fallback_strategy = fallback_strategy
            .add_strategy(Box::new(CircuitBreakerFallbackStrategy::new(5, 30)))
            .add_strategy(Box::new(SmartFallbackStrategy::new(Some(fallback_llm_adapter), 3)))
            .add_strategy(Box::new(SimpleFallbackStrategy::new()));
        
        // Создаем specialized handlers
        let chat_handler = ChatHandler::new(llm_adapter, chat_circuit_breaker.clone());
        let tools_handler = ToolsHandler::new(routing_adapter, tools_circuit_breaker.clone());
        let memory_handler = MemoryHandler::new(memory_adapter, memory_circuit_breaker.clone());
        let admin_handler = AdminHandler::new(admin_service, admin_circuit_breaker.clone());
        
        let response_strategy: Box<dyn ResponseFormattingStrategy> = Box::new(
            AdaptiveResponseFormatter::new()
        );
        
        // Создаем Integrated Tool Orchestrator
        let orchestrator_config = crate::orchestrator::OrchestrationConfig::default();
        let tool_system_config = EnhancedToolSystemConfig::default();
        let tool_orchestrator_config = ToolOrchestratorConfig {
            orchestration_config: orchestrator_config,
            tool_system_config,
            enable_cross_system_optimization: true,
            performance_monitoring_interval: std::time::Duration::from_secs(30),
        };
        let tool_orchestrator = ToolOrchestrator::new(tool_orchestrator_config).await?;
        
        let agent = Self {
            chat_handler,
            tools_handler,
            memory_handler,
            admin_handler,
            intent_strategy,
            fallback_strategy,
            response_strategy,
            tool_orchestrator,
            performance_monitor,
            chat_circuit_breaker,
            tools_circuit_breaker,
            memory_circuit_breaker,
            admin_circuit_breaker,
            initialized: false,
        };
        
        info!("✅ UnifiedAgentV2 создан с Clean Architecture");
        Ok(agent)
    }
    
    /// Инициализация всех компонентов
    pub async fn initialize(&mut self) -> Result<()> {
        let op_id = self.performance_monitor.start_operation("agent_initialization");
        
        info!("🚀 Инициализация UnifiedAgentV2");
        
        // Инициализируем performance monitor
        self.performance_monitor.initialize().await?;
        
        // Инициализируем все handlers
        self.chat_handler.initialize().await
            .map_err(|e| anyhow::anyhow!("Ошибка инициализации ChatHandler: {}", e))?;
        
        self.tools_handler.initialize().await
            .map_err(|e| anyhow::anyhow!("Ошибка инициализации ToolsHandler: {}", e))?;
        
        self.memory_handler.initialize().await
            .map_err(|e| anyhow::anyhow!("Ошибка инициализации MemoryHandler: {}", e))?;
        
        self.admin_handler.initialize().await
            .map_err(|e| anyhow::anyhow!("Ошибка инициализации AdminHandler: {}", e))?;
        
        // Инициализируем Integrated Tool Orchestrator
        // Note: ToolOrchestrator doesn't need separate initialization as it's initialized in constructor
        
        self.initialized = true;
        self.performance_monitor.finish_operation(&op_id, true);
        
        info!("✅ UnifiedAgentV2 полностью инициализирован с Integrated Tool Orchestrator");
        Ok(())
    }
}

#[async_trait]
impl RequestProcessorTrait for UnifiedAgentV2 {
    async fn process_user_request(&self, context: RequestContext) -> Result<ProcessingResult> {
        if !self.initialized {
            return Err(anyhow::anyhow!("UnifiedAgentV2 не инициализирован"));
        }
        
        let op_id = self.performance_monitor.start_operation("user_request_processing");
        let start_time = std::time::Instant::now();
        let mut components_used = Vec::new();
        let mut metrics = HashMap::new();
        
        debug!("UnifiedAgentV2: обработка запроса '{}'", context.message);
        
        // Шаг 1: Integrated Orchestration - анализ задачи и интеллектуальное выполнение
        let orchestration_result = self.tool_orchestrator.process_request(
            &context.message, 
            Some(context.metadata.clone())
        ).await;
        
        // Анализируем результат интегрированной оркестрации
        let (execution_strategy, orchestration_response) = match orchestration_result {
            Ok(result) => {
                info!("🎯 Integrated orchestrator завершил обработку: handler={}, optimization={}", 
                      result.orchestration_result.handler_used, result.optimization_applied);
                      
                // Если задача была обработана через tool system, возвращаем результат
                if let Some(tool_result) = &result.tool_result {
                    // Завершаем операцию с performance metrics
                    let execution_time = start_time.elapsed();
                    self.performance_monitor.finish_operation(&op_id, tool_result.execution_result.output.success);
                    
                    return Ok(ProcessingResult {
                        response: AgentResponse::ToolExecution(tool_result.execution_result.output.result.clone()),
                        processing_time_ms: execution_time.as_millis() as u64,
                        components_used: vec!["integrated_orchestrator".to_string(), "enhanced_tool_system".to_string()],
                        metrics: {
                            let mut m = HashMap::new();
                            m.insert("orchestration_time".to_string(), result.performance_metrics.orchestration_time.as_millis() as f64);
                            m.insert("tool_execution_time".to_string(), result.performance_metrics.tool_execution_time.as_millis() as f64);
                            m.insert("optimization_applied".to_string(), if result.optimization_applied { 1.0 } else { 0.0 });
                            m
                        },
                    });
                }
                
                // Если не через tool system, продолжаем стандартную обработку
                (None::<ExecutionStrategy>, Some(result.orchestration_result.response))
            }
            Err(e) => {
                warn!("⚠️ Integrated orchestrator недоступен: {}, используем стандартную обработку", e);
                (None::<ExecutionStrategy>, None::<String>)
            }
        };
        
        // Шаг 2: Определение намерения (Intent Strategy)
        let intent_decision = match self.intent_strategy.analyze_intent(&context).await {
            Ok(decision) => {
                components_used.push("intent_analyzer".to_string());
                decision
            }
            Err(e) => {
                warn!("Ошибка анализа намерений: {}, используем fallback", e);
                // Fallback к эвристической стратегии
                let heuristic = HeuristicIntentStrategy::new(0.5);
                heuristic.analyze_intent(&context).await?
            }
        };
        
        info!("Intent: {} (уверенность: {:.1}%)", 
              intent_decision.action_type, intent_decision.confidence * 100.0);
        
        // Добавляем orchestration metrics
        if execution_strategy.is_some() {
            components_used.push("adaptive_orchestrator".to_string());
            metrics.insert("orchestration_enabled".to_string(), 1.0);
        }
        
        // Шаг 3: Делегирование специализированному handler'у
        let response = match intent_decision.action_type.as_str() {
            "chat" => {
                if self.chat_handler.can_handle(&context).await {
                    components_used.push("chat_handler".to_string());
                    self.chat_handler.handle_chat(&context).await
                } else {
                    // Fallback
                    self.fallback_strategy.handle_fallback(&context, 
                        &anyhow::anyhow!("ChatHandler не может обработать запрос")).await
                }
            }
            "tools" => {
                if self.tools_handler.can_handle(&context).await {
                    components_used.push("tools_handler".to_string());
                    self.tools_handler.handle_tools(&context).await
                } else {
                    self.fallback_strategy.handle_fallback(&context,
                        &anyhow::anyhow!("ToolsHandler не может обработать запрос")).await
                }
            }
            "memory" => {
                if self.memory_handler.can_handle(&context).await {
                    components_used.push("memory_handler".to_string());
                    self.memory_handler.store_user_message(&context).await
                } else {
                    self.fallback_strategy.handle_fallback(&context,
                        &anyhow::anyhow!("MemoryHandler не может обработать запрос")).await
                }
            }
            "admin" => {
                if self.admin_handler.can_handle(&context).await {
                    components_used.push("admin_handler".to_string());
                    self.admin_handler.handle_admin_request(&context).await
                } else {
                    self.fallback_strategy.handle_fallback(&context,
                        &anyhow::anyhow!("AdminHandler не может обработать запрос")).await
                }
            }
            _ => {
                // Неизвестный тип, используем fallback
                warn!("Неизвестный тип намерения: {}", intent_decision.action_type);
                self.fallback_strategy.handle_fallback(&context,
                    &anyhow::anyhow!("Неизвестный тип намерения: {}", intent_decision.action_type)).await
            }
        };
        
        let processing_time = start_time.elapsed();
        
        // Обрабатываем результат или ошибку
        let final_response = match response {
            Ok(resp) => {
                self.performance_monitor.finish_operation(&op_id, true);
                
                // Note: Tool orchestrator handles task completion internally
                
                resp
            }
            Err(e) => {
                error!("Ошибка обработки запроса: {}", e);
                self.performance_monitor.finish_operation(&op_id, false);
                
                // Note: Tool orchestrator handles task completion internally
                
                // Используем fallback strategy
                components_used.push("fallback_strategy".to_string());
                self.fallback_strategy.handle_fallback(&context, &e).await?
            }
        };
        
        // Добавляем метрики
        metrics.insert("processing_time_ms".to_string(), processing_time.as_millis() as f64);
        metrics.insert("intent_confidence".to_string(), intent_decision.confidence as f64);
        metrics.insert("components_count".to_string(), components_used.len() as f64);
        
        info!("UnifiedAgentV2: запрос обработан за {:?}", processing_time);
        
        Ok(ProcessingResult {
            response: final_response,
            processing_time_ms: processing_time.as_millis() as u64,
            components_used,
            metrics,
        })
    }
    
    async fn is_ready(&self) -> bool {
        if !self.initialized {
            return false;
        }
        
        // Проверяем здоровье всех компонентов
        self.chat_handler.health_check().await.is_ok() &&
        self.tools_handler.health_check().await.is_ok() &&
        self.memory_handler.health_check().await.is_ok() &&
        self.admin_handler.health_check().await.is_ok()
    }
    
    async fn shutdown(&self) -> Result<()> {
        info!("🛑 Начинаем graceful shutdown UnifiedAgentV2");
        
        // Последовательно останавливаем все компоненты
        if let Err(e) = self.tool_orchestrator.shutdown().await {
            warn!("Ошибка при shutdown tool orchestrator: {}", e);
        }
        self.admin_handler.shutdown().await?;
        self.memory_handler.shutdown().await?;
        self.tools_handler.shutdown().await?;
        self.chat_handler.shutdown().await?;
        self.performance_monitor.shutdown().await?;
        
        info!("✅ UnifiedAgentV2 shutdown завершен");
        Ok(())
    }
}

impl UnifiedAgentV2 {
    /// Получение подробной статистики
    pub async fn get_detailed_stats(&self) -> String {
        let mut stats = String::new();
        
        stats.push_str("=== UnifiedAgentV2 Detailed Statistics ===\n\n");
        
        // Performance metrics
        if let Ok(metrics) = self.performance_monitor.get_detailed_metrics(60).await {
            stats.push_str(&metrics);
            stats.push_str("\n\n");
        }
        
        // Circuit breaker states
        stats.push_str("Circuit Breaker States:\n");
        stats.push_str(&format!("├─ Chat: {}\n", self.chat_circuit_breaker.get_state().await));
        stats.push_str(&format!("├─ Tools: {}\n", self.tools_circuit_breaker.get_state().await));
        stats.push_str(&format!("├─ Memory: {}\n", self.memory_circuit_breaker.get_state().await));
        stats.push_str(&format!("└─ Admin: {}\n", self.admin_circuit_breaker.get_state().await));
        
        // Component readiness
        stats.push_str("\nComponent Health:\n");
        stats.push_str(&format!("├─ Chat Handler: {}\n", 
            if self.chat_handler.health_check().await.is_ok() { "✅ Healthy" } else { "❌ Unhealthy" }));
        stats.push_str(&format!("├─ Tools Handler: {}\n", 
            if self.tools_handler.health_check().await.is_ok() { "✅ Healthy" } else { "❌ Unhealthy" }));
        stats.push_str(&format!("├─ Memory Handler: {}\n", 
            if self.memory_handler.health_check().await.is_ok() { "✅ Healthy" } else { "❌ Unhealthy" }));
        stats.push_str(&format!("├─ Admin Handler: {}\n", 
            if self.admin_handler.health_check().await.is_ok() { "✅ Healthy" } else { "❌ Unhealthy" }));
        stats.push_str(&format!("└─ Tool Orchestrator: {}\n", 
            if self.tool_orchestrator.health_check().await.is_ok() { "✅ Healthy" } else { "❌ Unhealthy" }));
        
        // Integrated Tool Orchestrator Statistics
        stats.push_str("\n");
        stats.push_str(&self.tool_orchestrator.get_comprehensive_stats().await);
        
        stats
    }
    
    /// Форматирование ответа с помощью response strategy
    pub async fn format_response(&self, response: &AgentResponse, context: &RequestContext) -> Result<String> {
        self.response_strategy.format_response(response, context).await
    }
}