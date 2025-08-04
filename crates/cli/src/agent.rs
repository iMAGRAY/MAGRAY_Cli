use anyhow::Result;
use llm::{LlmClient, IntentAnalyzerAgent};
use router::SmartRouter;
use common::OperationTimer;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

// @component: {"k":"C","id":"unified_agent","t":"Main agent orchestrator","m":{"cur":60,"tgt":90,"u":"%"},"d":["llm_client","smart_router"]}

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
}

// Удалены старые типы - теперь используем типы из specialized_agents

#[derive(Debug)]
pub enum AgentResponse {
    Chat(String),
    ToolExecution(String),
}

impl UnifiedAgent {
    pub async fn new() -> Result<Self> {
        let llm_client = LlmClient::from_env()?;
        let smart_router = SmartRouter::new(llm_client.clone());
        let intent_analyzer = IntentAnalyzerAgent::new(llm_client.clone());
        Ok(Self { llm_client, smart_router, intent_analyzer })
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
    
    // Простой API для тестов
    pub async fn process_query(&self, query: &str) -> Result<String> {
        match self.process_message(query).await? {
            AgentResponse::Chat(response) => Ok(response),
            AgentResponse::ToolExecution(result) => Ok(result),
        }
    }
}

// Структуры для тестов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub llm_provider: String,
    pub model_name: String,
    pub max_tokens: u32,
    pub temperature: f64,
    pub timeout_seconds: u64,
    pub retry_attempts: u32,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            llm_provider: "openai".to_string(),
            model_name: "gpt-4o-mini".to_string(),
            max_tokens: 1000,
            temperature: 0.7,
            timeout_seconds: 30,
            retry_attempts: 3,
        }
    }
}

impl AgentConfig {
    pub fn validate(&self) -> Result<()> {
        if self.max_tokens == 0 {
            anyhow::bail!("max_tokens must be greater than 0");
        }
        if self.temperature < 0.0 || self.temperature > 1.0 {
            anyhow::bail!("temperature must be between 0.0 and 1.0");
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct AgentResponseInfo {
    pub content: String,
    pub confidence: f64,
    pub tokens_used: u32,
    pub processing_time_ms: u64,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AgentMetrics {
    pub total_queries: u64,
    pub successful_queries: u64,
    pub failed_queries: u64,
    pub total_tokens_used: u64,
    pub total_processing_time_ms: u64,
}

impl AgentMetrics {
    pub fn record_query(&mut self, success: bool, tokens: u64, processing_time: u64) {
        self.total_queries += 1;
        if success {
            self.successful_queries += 1;
        } else {
            self.failed_queries += 1;
        }
        self.total_tokens_used += tokens;
        self.total_processing_time_ms += processing_time;
    }
}

#[derive(Debug, Clone)]
pub struct MemoryConfig {
    pub enabled: bool,
    pub max_entries: usize,
    pub ttl_seconds: u64,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_entries: 1000,
            ttl_seconds: 3600,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConversationMessage {
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct AgentContext {
    pub conversation_history: Vec<ConversationMessage>,
}

impl AgentContext {
    pub fn new() -> Self {
        Self {
            conversation_history: Vec::new(),
        }
    }
    
    pub fn add_message(&mut self, role: &str, content: &str) {
        self.conversation_history.push(ConversationMessage {
            role: role.to_string(),
            content: content.to_string(),
            timestamp: Utc::now(),
        });
    }
}

