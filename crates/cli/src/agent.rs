use anyhow::Result;
use llm::{LlmClient, IntentAnalyzerAgent};
use router::SmartRouter;
// Удален неиспользуемый импорт serde

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
    pub fn new(llm_client: LlmClient) -> Self {
        let smart_router = SmartRouter::new(llm_client.clone());
        let intent_analyzer = IntentAnalyzerAgent::new(llm_client.clone());
        Self { llm_client, smart_router, intent_analyzer }
    }
    
    pub async fn process_message(&self, message: &str) -> Result<AgentResponse> {
        // Используем специализированный агент для анализа намерений
        let decision = self.intent_analyzer.analyze_intent(message).await?;
        
        println!("[AI] Анализ намерения: {} (уверенность: {:.1}%)", 
                decision.action_type, decision.confidence * 100.0);
        
        match decision.action_type.as_str() {
            "chat" => {
                let response = self.llm_client.chat(message).await?;
                Ok(AgentResponse::Chat(response))
            }
            "tools" => {
                let result = self.smart_router.process_smart_request(message).await?;
                Ok(AgentResponse::ToolExecution(result))
            }
            _ => {
                // Fallback на простую эвристику если агент вернул неожиданный тип
                if self.simple_heuristic(message) {
                    let result = self.smart_router.process_smart_request(message).await?;
                    Ok(AgentResponse::ToolExecution(result))
                } else {
                    let response = self.llm_client.chat(message).await?;
                    Ok(AgentResponse::Chat(response))
                }
            }
        }
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
}// Test comment

