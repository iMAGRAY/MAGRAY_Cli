use anyhow::Result;
use llm::LlmClient;
use serde::{Deserialize, Serialize};
use tools::SmartRouter;

pub struct UnifiedAgent {
    llm_client: LlmClient,
    smart_router: SmartRouter,
}

#[derive(Debug, Deserialize)]
struct IntentDecision {
    action_type: ActionType,
    reasoning: String,
    confidence: f32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ActionType {
    SimpleChat,
    UseTools,
}

#[derive(Debug)]
pub enum AgentResponse {
    Chat(String),
    ToolExecution(String),
}

impl UnifiedAgent {
    pub fn new(llm_client: LlmClient) -> Self {
        let smart_router = SmartRouter::new(llm_client.clone());
        Self { llm_client, smart_router }
    }
    
    pub async fn process_message(&self, message: &str) -> Result<AgentResponse> {
        // Всегда сначала спрашиваем у LLM, что делать
        let decision = self.analyze_intent(message).await?;
        
        match decision.action_type {
            ActionType::SimpleChat => {
                let response = self.llm_client.chat(message).await?;
                Ok(AgentResponse::Chat(response))
            }
            ActionType::UseTools => {
                let result = self.smart_router.process_smart_request(message).await?;
                Ok(AgentResponse::ToolExecution(result))
            }
        }
    }
    
    async fn analyze_intent(&self, message: &str) -> Result<IntentDecision> {
        let prompt = format!(
            r#"Analyze this user message and decide if it requires tools or just chat.

Available tools:
- file_read: read files
- file_write: create/write files  
- dir_list: list directory contents
- git_status: check git status
- git_commit: make git commits
- web_search: search the internet
- shell_exec: execute shell commands

Message: "{}"

If the user wants to:
- Read, show, display, or view files → use_tools
- Create, write, save files → use_tools
- List directories, show folder contents → use_tools
- Git operations → use_tools
- Execute commands → use_tools
- Search the internet → use_tools
- Just chat, ask questions, get explanations → simple_chat

Respond with ONLY valid JSON:
{{
    "action_type": "simple_chat",
    "reasoning": "User is just greeting/asking a question",
    "confidence": 0.9
}}

OR

{{
    "action_type": "use_tools", 
    "reasoning": "User wants to read/create files or perform actions",
    "confidence": 0.9
}}"#,
            message
        );
        
        let response = self.llm_client.chat(&prompt).await?;
        
        // Пытаемся извлечь JSON из ответа
        let json_start = response.find('{').unwrap_or(0);
        let json_end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
        let json_str = &response[json_start..json_end];
        
        match serde_json::from_str::<IntentDecision>(json_str) {
            Ok(decision) => Ok(decision),
            Err(_) => {
                // Fallback: если не можем распарсить, используем простую эвристику
                if self.simple_heuristic(message) {
                    Ok(IntentDecision {
                        action_type: ActionType::UseTools,
                        reasoning: "Fallback heuristic detected tool usage".to_string(),
                        confidence: 0.7,
                    })
                } else {
                    Ok(IntentDecision {
                        action_type: ActionType::SimpleChat,
                        reasoning: "Fallback to simple chat".to_string(),
                        confidence: 0.7,
                    })
                }
            }
        }
    }
    
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
}