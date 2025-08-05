use anyhow::Result;
use llm::{LlmClient, IntentAnalyzerAgent};
use router::SmartRouter;
use memory::{DIMemoryService, default_config};
use common::OperationTimer;
use tracing::{info, debug};

// @component: {"k":"C","id":"unified_agent","t":"Main agent orchestrator","m":{"cur":70,"tgt":95,"u":"%"},"d":["llm_client","smart_router","di_memory_service"]}

// @component: UnifiedAgent
// @file: crates/cli/src/agent.rs:6-70
// @status: WORKING
// @performance: O(1) routing, O(n) downstream
// @dependencies: LlmClient(✅), SmartRouter(⚠️), IntentAnalyzerAgent(✅)
// @tests: ❌ No unit tests found
// @production_ready: 60%
// @issues: Missing error handling for LLM failures
// @upgrade_path: Add retry logic, timeout configuration
pub struct UnifiedAgent {
    llm_client: LlmClient,
    smart_router: SmartRouter,
    intent_analyzer: IntentAnalyzerAgent,
    memory_service: DIMemoryService,
}

// Удалены старые типы - теперь используем типы из specialized_agents

#[derive(Debug)]
pub enum AgentResponse {
    Chat(String),
    ToolExecution(String),
}

impl UnifiedAgent {
    pub async fn new() -> Result<Self> {
        info!("🤖 Инициализация UnifiedAgent с DI системой");
        
        let llm_client = LlmClient::from_env()?;
        let smart_router = SmartRouter::new(llm_client.clone());
        let intent_analyzer = IntentAnalyzerAgent::new(llm_client.clone());
        
        // Инициализация DI Memory Service
        let memory_config = default_config()?;
        let memory_service = DIMemoryService::new(memory_config).await
            .map_err(|e| anyhow::anyhow!("Ошибка создания DIMemoryService: {}", e))?;
        
        // Инициализация слоев памяти
        memory_service.initialize().await
            .map_err(|e| anyhow::anyhow!("Ошибка инициализации слоев памяти: {}", e))?;
        
        info!("✅ UnifiedAgent создан с DI мамятью");
        
        Ok(Self { 
            llm_client, 
            smart_router, 
            intent_analyzer,
            memory_service,
        })
    }
    
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
    
    // Удален захардкоженный analyze_intent - теперь используем IntentAnalyzerAgent
    
    // Простая эвристика как fallback
    fn simple_heuristic(&self, message: &str) -> bool {
        let message_lower = message.to_lowercase();
        let tool_indicators = [
            "файл", "file", "папка", "folder", "directory", "dir",
            "git", "commit", "status", "команда", "command", "shell",
            "создай", "create", "покажи", "show", "список", "list",
            "прочитай", "read", "запиши", "write", "найди", "search"
        ];
        
        tool_indicators.iter().any(|&indicator| message_lower.contains(indicator))
    }

    /// Сохранить сообщение пользователя в память (Interact layer)
    pub async fn store_user_message(&self, message: &str) -> Result<()> {
        use memory::{Record, Layer};
        use uuid::Uuid;
        use chrono::Utc;
        
        let record = Record {
            id: Uuid::new_v4(),
            text: message.to_string(),
            embedding: vec![], // Будет создан автоматически
            layer: Layer::Interact,
            kind: "user_message".to_string(),
            tags: vec!["chat".to_string()],
            project: "magray".to_string(),
            session: "current".to_string(),
            ts: Utc::now(),
            score: 0.0,
            access_count: 1,
            last_access: Utc::now(),
        };
        
        self.memory_service.insert(record).await
            .map_err(|e| anyhow::anyhow!("Ошибка сохранения в память: {}", e))?;
        
        debug!("💾 Сообщение сохранено в Interact layer");
        Ok(())
    }

    /// Поиск релевантных сообщений в памяти
    pub async fn search_memory(&self, query: &str) -> Result<Vec<String>> {
        use memory::{Layer, SearchOptions};
        
        let search_options = SearchOptions {
            layers: vec![Layer::Insights],
            top_k: 5,
            score_threshold: 0.7,
            tags: vec![],
            project: Some("magray".to_string()),
        };
        
        // Поиск в слое Insights (наиболее релевантные данные)
        let results = self.memory_service.search(query, Layer::Insights, search_options).await
            .map_err(|e| anyhow::anyhow!("Ошибка поиска в памяти: {}", e))?;
        
        let content: Vec<String> = results.into_iter()
            .map(|record| record.text)
            .collect();
        
        debug!("🔍 Найдено {} релевантных записей в памяти", content.len());
        Ok(content)
    }

    /// Получить статистику DI системы
    pub async fn get_di_stats(&self) -> memory::service_di::MemorySystemStats {
        self.memory_service.get_stats().await
    }

    /// Запустить promotion процесс (перенос данных между слоями)
    pub async fn run_memory_promotion(&self) -> Result<()> {
        let stats = self.memory_service.run_promotion().await
            .map_err(|e| anyhow::anyhow!("Ошибка promotion: {}", e))?;
        
        info!("🔄 Promotion завершен: {} → Insights, {} → Assets", 
              stats.interact_to_insights, stats.insights_to_assets);
        Ok(())
    }

    /// Проверить здоровье системы
    pub async fn check_system_health(&self) -> Result<memory::health::SystemHealthStatus> {
        self.memory_service.check_health().await
            .map_err(|e| anyhow::anyhow!("Ошибка проверки здоровья: {}", e))
    }

    /// Получить performance метрики DI системы
    pub fn get_performance_metrics(&self) -> memory::DIPerformanceMetrics {
        self.memory_service.get_performance_metrics()
    }

    /// Получить краткий отчет о производительности DI системы
    pub fn get_performance_report(&self) -> String {
        self.memory_service.get_performance_report()
    }

    /// Сбросить performance метрики (для тестов/отладки)
    pub fn reset_performance_metrics(&self) {
        self.memory_service.reset_performance_metrics()
    }
}
