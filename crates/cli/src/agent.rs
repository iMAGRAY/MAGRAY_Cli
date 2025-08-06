//! LEGACY UnifiedAgent - Compatibility Bridge
//! 
//! **⚠️ DEPRECATED**: Этот модуль обеспечивает backward compatibility
//! через bridge pattern к UnifiedAgentV2. 
//! 
//! Для новых проектов используйте UnifiedAgentV2 напрямую.

use crate::legacy_bridge::LegacyUnifiedAgent;

// Re-export bridge as UnifiedAgent для полной совместимости
pub type UnifiedAgent = LegacyUnifiedAgent;

// Re-export AgentResponse (no use)
// pub use crate::agent_traits::AgentResponse as LegacyAgentResponse;

// ============================================================================
// ОРИГИНАЛЬНЫЙ КОД СОХРАНЕН НО ЗАКОММЕНТИРОВАН ДЛЯ ИСТОРИИ
// ============================================================================

/*
// Весь оригинальный код UnifiedAgent закомментирован после миграции на bridge pattern.
// Этот код был заменен на LegacyUnifiedAgent bridge который обеспечивает
// 100% API совместимость, делегируя все вызовы к UnifiedAgentV2.
// 
// ПРИЧИНА МИГРАЦИИ:
// - God Object с 17 зависимостями → Clean Architecture с 4 зависимостями  
// - SOLID принципы нарушались → полное соответствие SOLID
// - Monolithic structure → Dependency Injection + Strategy patterns
// - No circuit breakers → Circuit Breaker для всех компонентов
// - Простая error handling → Comprehensive error handling с fallback
// 
// MIGRATION PATH:
// Old: UnifiedAgent::new().await → LegacyUnifiedAgent::new().await (через bridge)
// New: UnifiedAgentV2::new().await + initialize() (прямое использование)
//
// Оригинальная реализация была ~220 строк с тесными связями между компонентами.
// Bridge pattern сохраняет API но делегирует к Clean Architecture.

use anyhow::Result;
use llm::{LlmClient, IntentAnalyzerAgent};
use router::SmartRouter;
use memory::{DIMemoryService, default_config};
use common::OperationTimer;
use tracing::{info, debug, warn};

#[derive(Debug, Clone)]
pub enum AgentResponse {
    Chat(String),
    ToolExecution(String),
    Error(String),
}

pub struct OriginalUnifiedAgent {
    llm_client: LlmClient,
    smart_router: SmartRouter,
    intent_analyzer: IntentAnalyzerAgent,
    memory_service: DIMemoryService,
}

impl OriginalUnifiedAgent {
    /// ORIGINAL: Создание оригинального UnifiedAgent (GOD OBJECT PATTERN)
    pub async fn new() -> Result<Self> {
        warn!("🤖 Создание LEGACY UnifiedAgent - рекомендуется использовать UnifiedAgentV2");
        info!("🤖 Инициализация UnifiedAgent с DI системой");
        
        let llm_client = LlmClient::from_env_multi()
            .or_else(|_| {
                info!("🔄 Multi-provider setup failed, falling back to single provider");
                LlmClient::from_env()
            })?;
        let smart_router = SmartRouter::new(llm_client.clone());
        let intent_analyzer = IntentAnalyzerAgent::new(llm_client.clone());
        
        // Инициализация DI Memory Service
        let memory_config = default_config()?;
        let memory_service = DIMemoryService::new(memory_config).await
            .map_err(|e| anyhow::anyhow!("Ошибка создания DIMemoryService: {}", e))?;
        
        // Инициализация слоев памяти
        memory_service.initialize().await
            .map_err(|e| anyhow::anyhow!("Ошибка инициализации слоев памяти: {}", e))?;
        
        info!("✅ LEGACY UnifiedAgent создан с DI памятью");
        warn!("💡 Для production используйте UnifiedAgentV2 с Clean Architecture");
        
        Ok(Self { 
            llm_client, 
            smart_router, 
            intent_analyzer,
            memory_service,
        })
    }
    
    /// ORIGINAL: Обработка сообщения (MONOLITHIC PATTERN)
    pub async fn process_message(&self, message: &str) -> Result<AgentResponse> {
        let mut timer = OperationTimer::new("agent_process_message");
        timer.add_field("message_length", message.len());
        
        // Используем специализированный агент для анализа намерений
        let decision = self.intent_analyzer.analyze_intent(message).await?;
        timer.add_field("intent_type", &decision.action_type);
        timer.add_field("confidence", decision.confidence);
        
        println!("[AI] Анализ намерения: {} (уверенность: {:.1}%)", 
                decision.action_type, decision.confidence * 100.0);
        
        let response = match decision.action_type.as_str() {
            "chat" => {
                let chat_timer = OperationTimer::new("llm_chat");
                let response = self.llm_client.chat_simple(message).await?;
                chat_timer.finish();
                Ok(AgentResponse::Chat(response))
            }
            "tools" => {
                let tools_timer = OperationTimer::new("smart_router_process");
                let result = self.smart_router.process_smart_request(message).await?;
                tools_timer.finish();
                Ok(AgentResponse::ToolExecution(result))
            }
            _ => {
                // Fallback на простую эвристику если агент вернул неожиданный тип
                if self.simple_heuristic(message) {
                    let tools_timer = OperationTimer::new("smart_router_fallback");
                    let result = self.smart_router.process_smart_request(message).await?;
                    tools_timer.finish();
                    Ok(AgentResponse::ToolExecution(result))
                } else {
                    let chat_timer = OperationTimer::new("llm_chat_fallback");
                    let response = self.llm_client.chat_simple(message).await?;
                    chat_timer.finish();
                    Ok(AgentResponse::Chat(response))
                }
            }
        };
        
        timer.finish_with_result(response.as_ref().map(|_| ()));
        response
    }
    
    // ... остальные методы были аналогично реализованы в monolithic стиле
    // Полный код сохранен в git history до миграции на bridge pattern.
}
*/