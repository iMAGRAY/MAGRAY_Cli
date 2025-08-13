use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use uuid::Uuid;

// LLM integration for enhanced intent analysis
use llm::{LlmProvider, LlmRequest};

// Health monitoring integration
use crate::reliability::health::{HealthChecker, HealthReport, HealthStatus};

/// Intent represents the user's structured intention
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Intent {
    pub id: Uuid,
    pub intent_type: IntentType,
    pub parameters: HashMap<String, serde_json::Value>,
    pub confidence: f64,
    pub context: IntentContext,
}

/// Types of intents the system can recognize
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum IntentType {
    /// Execute a specific tool or command
    ExecuteTool { tool_name: String },
    /// Ask a question or request information
    AskQuestion { question: String },
    /// Perform file operations
    FileOperation { operation: String, path: String },
    /// Memory operations (search, store, analyze)
    MemoryOperation { operation: String },
    /// Complex workflow or recipe execution
    WorkflowExecution { workflow_name: String },
    /// System management commands
    SystemCommand { command: String },
    /// Unknown intent requiring further analysis
    Unknown { raw_input: String },
}

/// Context information for intent analysis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntentContext {
    pub session_id: Uuid,
    pub user_id: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub environment: HashMap<String, String>,
    pub conversation_history: Vec<String>,
}

/// Trait for intent analysis functionality
#[async_trait]
pub trait IntentAnalyzerTrait: Send + Sync {
    /// Analyze user input and extract structured intent
    async fn analyze_intent(&self, input: &str, context: &IntentContext) -> Result<Intent>;

    /// Update confidence based on execution results
    async fn update_confidence(&mut self, intent_id: Uuid, success: bool) -> Result<()>;

    /// Get intent analysis statistics
    async fn get_statistics(&self) -> Result<IntentAnalysisStats>;
}

/// Statistics for intent analysis performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentAnalysisStats {
    pub total_analyzed: u64,
    pub successful_predictions: u64,
    pub average_confidence: f64,
    pub intent_type_distribution: HashMap<String, u64>,
}

/// Intent Analyzer implementation
pub struct IntentAnalyzer {
    agent_id: Uuid,
    stats: IntentAnalysisStats,
    confidence_threshold: f64,
    llm_provider: Option<Box<dyn LlmProvider>>,
    use_llm_fallback: bool,
    // Health monitoring fields
    last_heartbeat: Arc<RwLock<Option<DateTime<Utc>>>>,
    error_count: Arc<RwLock<u32>>,
    start_time: Instant,
}

impl IntentAnalyzer {
    /// Create new IntentAnalyzer instance
    pub fn new() -> Self {
        Self {
            agent_id: Uuid::new_v4(),
            stats: IntentAnalysisStats {
                total_analyzed: 0,
                successful_predictions: 0,
                average_confidence: 0.0,
                intent_type_distribution: HashMap::new(),
            },
            confidence_threshold: 0.7,
            llm_provider: None,
            use_llm_fallback: false,
            last_heartbeat: Arc::new(RwLock::new(Some(Utc::now()))),
            error_count: Arc::new(RwLock::new(0)),
            start_time: Instant::now(),
        }
    }

    /// Create IntentAnalyzer with LLM provider for enhanced analysis
    pub fn with_llm_provider(mut self, provider: Box<dyn LlmProvider>) -> Self {
        self.llm_provider = Some(provider);
        self.use_llm_fallback = true;
        self
    }

    /// Start automatic heartbeat loop for health monitoring
    /// This prevents timeout issues by sending heartbeat every 30 seconds
    pub fn start_heartbeat_loop(&self) {
        let last_heartbeat = Arc::clone(&self.last_heartbeat);
        let agent_id = self.agent_id;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                interval.tick().await;

                // Update heartbeat timestamp
                {
                    let mut heartbeat = last_heartbeat.write().await;
                    *heartbeat = Some(Utc::now());
                }

                tracing::debug!(
                    agent_id = %agent_id,
                    agent_type = "IntentAnalyzer",
                    "Heartbeat sent"
                );
            }
        });

        tracing::info!(
            agent_id = %self.agent_id,
            agent_type = "IntentAnalyzer",
            "Heartbeat loop started with 30s interval"
        );
    }

    /// Parse basic intent patterns from user input
    fn parse_intent_type(&self, input: &str) -> IntentType {
        let input = input.to_lowercase();

        // Tool execution patterns
        if input.starts_with("run ") || input.starts_with("execute ") {
            let tool_name = input
                .split_whitespace()
                .nth(1)
                .unwrap_or("unknown")
                .to_string();
            return IntentType::ExecuteTool { tool_name };
        }

        // Question patterns
        if input.starts_with("what ")
            || input.starts_with("how ")
            || input.starts_with("why ")
            || input.contains("?")
        {
            return IntentType::AskQuestion {
                question: input.clone(),
            };
        }

        // File operation patterns
        if input.contains("file")
            || input.contains("read")
            || input.contains("write")
            || input.contains("delete")
        {
            let operation = if input.contains("read") {
                "read"
            } else if input.contains("write") {
                "write"
            } else {
                "unknown"
            };
            let path = "unknown".to_string(); // Would extract from input in real implementation
            return IntentType::FileOperation {
                operation: operation.to_string(),
                path,
            };
        }

        // Memory operation patterns
        if input.contains("remember") || input.contains("recall") || input.contains("search") {
            let operation = if input.contains("remember") {
                "store"
            } else {
                "search"
            };
            return IntentType::MemoryOperation {
                operation: operation.to_string(),
            };
        }

        // System command patterns
        if input.starts_with("system ") || input.contains("status") || input.contains("health") {
            return IntentType::SystemCommand {
                command: input.clone(),
            };
        }

        // Default to unknown
        IntentType::Unknown { raw_input: input }
    }

    /// Calculate confidence based on pattern matching and context
    fn calculate_confidence(
        &self,
        intent_type: &IntentType,
        input: &str,
        context: &IntentContext,
    ) -> f64 {
        let mut confidence: f64 = 0.5; // Base confidence

        match intent_type {
            IntentType::Unknown { .. } => confidence = 0.1,
            IntentType::ExecuteTool { tool_name } => {
                if !tool_name.is_empty() && tool_name != "unknown" {
                    confidence = 0.9;
                } else {
                    confidence = 0.3;
                }
            }
            IntentType::AskQuestion { .. } => {
                if input.contains("?") {
                    confidence = 0.8
                } else {
                    confidence = 0.6
                }
            }
            _ => confidence = 0.7, // Default for recognized patterns
        }

        // Adjust based on context
        if !context.conversation_history.is_empty() {
            confidence += 0.1; // Slight boost for context
        }

        confidence.min(1.0f64)
    }

    /// Enhanced intent analysis using LLM when available
    async fn llm_enhanced_analysis(
        &self,
        input: &str,
        context: &IntentContext,
    ) -> Result<(IntentType, f64)> {
        if let Some(ref provider) = self.llm_provider {
            let prompt = self.build_intent_analysis_prompt(input, context);

            let request = LlmRequest {
                prompt,
                system_prompt: None,
                stream: false,
                temperature: Some(0.1),
                max_tokens: Some(200),
            };

            match provider.complete(request).await {
                Ok(response) => {
                    return self.parse_llm_response(&response.content);
                }
                Err(e) => {
                    tracing::warn!("LLM analysis failed, falling back to rule-based: {}", e);
                }
            }
        }

        // Fallback to rule-based analysis
        let intent_type = self.parse_intent_type(input);
        let confidence = self.calculate_confidence(&intent_type, input, context);
        Ok((intent_type, confidence))
    }

    /// Build structured prompt for LLM intent analysis
    fn build_intent_analysis_prompt(&self, input: &str, context: &IntentContext) -> String {
        let context_info = if !context.conversation_history.is_empty() {
            format!(
                "Previous context: {:?}",
                context.conversation_history.last()
            )
        } else {
            "No previous context".to_string()
        };

        format!(
            r#"Analyze the following user input and classify the intent. 
User input: "{}"
Context: {}

Please respond with JSON in this format:
{{
  "intent_type": "ExecuteTool|AskQuestion|FileOperation|MemoryOperation|WorkflowExecution|SystemCommand|Unknown",
  "confidence": 0.9,
  "parameters": {{"tool_name": "example"}}
}}

Available intent types:
- ExecuteTool: Running a specific tool or command
- AskQuestion: Asking for information or help
- FileOperation: File/directory operations (read, write, delete)
- MemoryOperation: Memory search, storage, or analysis
- WorkflowExecution: Complex multi-step workflows
- SystemCommand: System management commands
- Unknown: Unclear or ambiguous intent

Respond only with valid JSON."#,
            input, context_info
        )
    }

    /// Parse LLM response into intent type and confidence
    fn parse_llm_response(&self, response: &str) -> Result<(IntentType, f64)> {
        #[derive(Deserialize)]
        struct LlmIntentResponse {
            intent_type: String,
            confidence: f64,
            parameters: Option<HashMap<String, String>>,
        }

        match serde_json::from_str::<LlmIntentResponse>(response) {
            Ok(parsed) => {
                let intent_type = match parsed.intent_type.as_str() {
                    "ExecuteTool" => {
                        let tool_name = parsed
                            .parameters
                            .and_then(|p| p.get("tool_name").cloned())
                            .unwrap_or_else(|| "unknown".to_string());
                        IntentType::ExecuteTool { tool_name }
                    }
                    "AskQuestion" => IntentType::AskQuestion {
                        question: parsed
                            .parameters
                            .and_then(|p| p.get("question").cloned())
                            .unwrap_or_else(|| "".to_string()),
                    },
                    "FileOperation" => {
                        let params = parsed.parameters.unwrap_or_default();
                        IntentType::FileOperation {
                            operation: params
                                .get("operation")
                                .cloned()
                                .unwrap_or_else(|| "unknown".to_string()),
                            path: params
                                .get("path")
                                .cloned()
                                .unwrap_or_else(|| "unknown".to_string()),
                        }
                    }
                    "MemoryOperation" => IntentType::MemoryOperation {
                        operation: parsed
                            .parameters
                            .and_then(|p| p.get("operation").cloned())
                            .unwrap_or_else(|| "unknown".to_string()),
                    },
                    "WorkflowExecution" => IntentType::WorkflowExecution {
                        workflow_name: parsed
                            .parameters
                            .and_then(|p| p.get("workflow_name").cloned())
                            .unwrap_or_else(|| "unknown".to_string()),
                    },
                    "SystemCommand" => IntentType::SystemCommand {
                        command: parsed
                            .parameters
                            .and_then(|p| p.get("command").cloned())
                            .unwrap_or_else(|| "unknown".to_string()),
                    },
                    _ => IntentType::Unknown {
                        raw_input: "llm_unknown".to_string(),
                    },
                };

                Ok((intent_type, parsed.confidence.clamp(0.0, 1.0)))
            }
            Err(_) => {
                // Failed to parse LLM response, return unknown intent
                Ok((
                    IntentType::Unknown {
                        raw_input: "llm_parse_error".to_string(),
                    },
                    0.2,
                ))
            }
        }
    }
}

#[async_trait]
impl IntentAnalyzerTrait for IntentAnalyzer {
    async fn analyze_intent(&self, input: &str, context: &IntentContext) -> Result<Intent> {
        // Use LLM enhancement if available, otherwise fall back to rule-based
        let (intent_type, confidence) = if self.use_llm_fallback {
            self.llm_enhanced_analysis(input, context).await?
        } else {
            let intent_type = self.parse_intent_type(input);
            let confidence = self.calculate_confidence(&intent_type, input, context);
            (intent_type, confidence)
        };

        let intent = Intent {
            id: Uuid::new_v4(),
            intent_type,
            parameters: HashMap::new(), // Enhanced parameter extraction in LLM mode
            confidence,
            context: context.clone(),
        };

        tracing::debug!(
            "Analyzed intent: type={:?}, confidence={:.2}, method={}",
            intent.intent_type,
            intent.confidence,
            if self.use_llm_fallback {
                "LLM+Rules"
            } else {
                "Rules"
            }
        );

        Ok(intent)
    }

    async fn update_confidence(&mut self, intent_id: Uuid, success: bool) -> Result<()> {
        if success {
            self.stats.successful_predictions += 1;
        }
        self.stats.total_analyzed += 1;

        // Recalculate average confidence
        if self.stats.total_analyzed > 0 {
            let success_rate =
                self.stats.successful_predictions as f64 / self.stats.total_analyzed as f64;
            self.stats.average_confidence = success_rate;
        }

        tracing::debug!(
            "Updated confidence for intent {}: success={}, new_avg={:.2}",
            intent_id,
            success,
            self.stats.average_confidence
        );

        Ok(())
    }

    async fn get_statistics(&self) -> Result<IntentAnalysisStats> {
        Ok(self.stats.clone())
    }
}

impl Default for IntentAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// HealthChecker implementation for IntentAnalyzer
#[async_trait]
impl HealthChecker for IntentAnalyzer {
    async fn check_health(&self) -> Result<HealthReport> {
        let last_heartbeat = *self.last_heartbeat.read().await;
        let error_count = *self.error_count.read().await;
        let uptime = self.start_time.elapsed().as_secs();

        // Determine health status based on errors and activity
        let status = if error_count > 10 {
            HealthStatus::Unhealthy {
                reason: format!("High error count: {}", error_count),
            }
        } else if error_count > 5 {
            HealthStatus::Degraded {
                reason: format!("Moderate error count: {}", error_count),
            }
        } else {
            HealthStatus::Healthy
        };

        Ok(HealthReport {
            agent_id: self.agent_id,
            agent_name: "IntentAnalyzer".to_string(),
            agent_type: "IntentAnalyzer".to_string(),
            status,
            timestamp: Utc::now(),
            last_heartbeat,
            response_time_ms: Some(5), // Intent analysis is typically fast
            memory_usage_mb: Some(50), // Estimated memory usage
            cpu_usage_percent: Some(2.0), // Low CPU usage for rule-based analysis
            active_tasks: self.stats.total_analyzed as u32,
            error_count,
            restart_count: 0, // Track restarts in future implementation
            uptime_seconds: uptime,
            metadata: serde_json::json!({
                "total_analyzed": self.stats.total_analyzed,
                "successful_predictions": self.stats.successful_predictions,
                "average_confidence": self.stats.average_confidence,
                "confidence_threshold": self.confidence_threshold,
                "llm_enabled": self.use_llm_fallback
            }),
        })
    }

    fn agent_id(&self) -> Uuid {
        self.agent_id
    }

    fn agent_name(&self) -> &str {
        "IntentAnalyzer"
    }

    fn agent_type(&self) -> &str {
        "IntentAnalyzer"
    }

    async fn heartbeat(&self) -> Result<()> {
        let mut heartbeat = self.last_heartbeat.write().await;
        *heartbeat = Some(Utc::now());
        Ok(())
    }

    fn last_heartbeat(&self) -> Option<DateTime<Utc>> {
        // Use try_read for synchronous access needed by the trait
        self.last_heartbeat.try_read().ok().and_then(|guard| *guard)
    }

    fn is_healthy(&self) -> bool {
        let error_count = self
            .error_count
            .try_read()
            .map(|guard| *guard)
            .unwrap_or(u32::MAX);
        error_count <= 10
    }

    async fn restart(&self) -> Result<()> {
        // Reset error count and update heartbeat
        {
            let mut error_count = self.error_count.write().await;
            *error_count = 0;
        }
        self.heartbeat().await?;
        tracing::info!("IntentAnalyzer restarted successfully");
        Ok(())
    }
}

/// BaseActor implementation for IntentAnalyzer
#[async_trait]
impl crate::actors::BaseActor for IntentAnalyzer {
    fn id(&self) -> crate::actors::ActorId {
        crate::actors::ActorId::new()
    }

    fn actor_type(&self) -> &'static str {
        "IntentAnalyzer"
    }

    async fn handle_message(
        &mut self,
        message: crate::actors::ActorMessage,
        _context: &crate::actors::ActorContext,
    ) -> Result<(), crate::actors::ActorError> {
        match message {
            crate::actors::ActorMessage::Agent(agent_msg) => match agent_msg {
                crate::actors::AgentMessage::AnalyzeIntent {
                    user_input,
                    context: _,
                } => {
                    let intent_context = IntentContext {
                        session_id: uuid::Uuid::new_v4(),
                        user_id: None,
                        timestamp: chrono::Utc::now(),
                        environment: std::collections::HashMap::new(),
                        conversation_history: vec![],
                    };

                    match self.analyze_intent(&user_input, &intent_context).await {
                        Ok(intent) => {
                            tracing::info!(
                                "Intent analyzed successfully: {:?}",
                                intent.intent_type
                            );
                            Ok(())
                        }
                        Err(e) => {
                            tracing::error!("Intent analysis failed: {}", e);
                            Err(crate::actors::ActorError::MessageHandlingFailed(
                                self.id(),
                                format!("Intent analysis failed: {}", e),
                            ))
                        }
                    }
                }
                _ => {
                    tracing::warn!("Unsupported agent message type for IntentAnalyzer");
                    Ok(())
                }
            },
            _ => {
                tracing::warn!("Unsupported message type for IntentAnalyzer");
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_context() -> IntentContext {
        IntentContext {
            session_id: Uuid::new_v4(),
            user_id: Some("test_user".to_string()),
            timestamp: chrono::Utc::now(),
            environment: HashMap::new(),
            conversation_history: vec![],
        }
    }

    #[tokio::test]
    async fn test_analyze_tool_execution_intent() {
        let analyzer = IntentAnalyzer::new();
        let context = create_test_context();

        let intent = analyzer
            .analyze_intent("run file_reader", &context)
            .await
            .expect("Operation failed - converted from unwrap()");

        match intent.intent_type {
            IntentType::ExecuteTool { tool_name } => {
                assert_eq!(tool_name, "file_reader");
                assert!(intent.confidence > 0.8);
            }
            _ => panic!("Expected ExecuteTool intent"),
        }
    }

    #[tokio::test]
    async fn test_analyze_question_intent() {
        let analyzer = IntentAnalyzer::new();
        let context = create_test_context();

        let intent = analyzer
            .analyze_intent("What is the current time?", &context)
            .await
            .expect("Operation failed - converted from unwrap()");

        match intent.intent_type {
            IntentType::AskQuestion { question } => {
                assert!(question.contains("what is the current time?"));
                assert!(intent.confidence > 0.7);
            }
            _ => panic!("Expected AskQuestion intent"),
        }
    }

    #[tokio::test]
    async fn test_analyze_unknown_intent() {
        let analyzer = IntentAnalyzer::new();
        let context = create_test_context();

        let intent = analyzer
            .analyze_intent("random gibberish xyz", &context)
            .await
            .expect("Operation failed - converted from unwrap()");

        match intent.intent_type {
            IntentType::Unknown { raw_input } => {
                assert_eq!(raw_input, "random gibberish xyz");
                assert!(intent.confidence < 0.3);
            }
            _ => panic!("Expected Unknown intent"),
        }
    }

    #[tokio::test]
    async fn test_update_confidence() {
        let mut analyzer = IntentAnalyzer::new();
        let intent_id = Uuid::new_v4();

        analyzer
            .update_confidence(intent_id, true)
            .await
            .expect("Async operation should succeed");

        let stats = analyzer
            .get_statistics()
            .await
            .expect("Async operation should succeed");
        assert_eq!(stats.total_analyzed, 1);
        assert_eq!(stats.successful_predictions, 1);
        assert_eq!(stats.average_confidence, 1.0);
    }
}
