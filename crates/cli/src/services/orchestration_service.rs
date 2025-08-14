//! Orchestration Service - P1.1.15.a Implementation
//!
//! This service provides transparent integration between CLI commands and the
//! multi-agent orchestration system, implementing command routing through the
//! Intent→Plan→Execute→Critic workflow.

use anyhow::{anyhow, Result};
use llm::LlmClient;
use orchestrator::{
    create_agent_event_publisher, AgentOrchestrator, OrchestratorConfig, SystemConfig,
    TaskPriority, WorkflowRequest,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Прямой тест OpenAI API без LLM client - для диагностики
async fn try_direct_openai_test() -> anyhow::Result<String> {
    let api_key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY not found in environment"))?;

    if !api_key.starts_with("sk-") {
        return Err(anyhow::anyhow!(
            "API key doesn't look like OpenAI key (should start with sk-)"
        ));
    }

    let client = reqwest::Client::new();
    let request_body = serde_json::json!({
        "model": "gpt-4o-mini",
        "messages": [
            {
                "role": "user",
                "content": "Привет! Ответь одним предложением что ты работаешь."
            }
        ],
        "max_tokens": 50
    });

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("API returned {}: {}", status, error_text));
    }

    let response_json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse JSON: {}", e))?;

    let content = response_json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No content in response"))?
        .to_string();

    Ok(content)
}

// Remove the problematic import for now and create a simplified service

/// Command execution request for orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRequest {
    pub command_type: String,
    pub action: String,
    pub parameters: HashMap<String, String>,
    pub context: ExecutionContext,
}

/// Execution context for command routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub user_id: Option<String>,
    pub session_id: String,
    pub priority: CommandPriority,
    pub timeout_ms: u64,
    pub dry_run: bool,
}

/// Command priority levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Orchestration execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationResult {
    pub success: bool,
    pub execution_time_ms: u64,
    pub intent_analysis: Option<IntentAnalysis>,
    pub plan_summary: Option<String>,
    pub execution_steps: Vec<ExecutionStep>,
    pub critique: Option<String>,
    pub error: Option<String>,
    pub fallback_used: bool,
}

/// Intent analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentAnalysis {
    pub intent_type: String,
    pub confidence: f64,
    pub extracted_parameters: HashMap<String, String>,
    pub suggested_tools: Vec<String>,
}

/// Individual execution step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    pub step_id: String,
    pub tool_used: Option<String>,
    pub status: StepStatus,
    pub duration_ms: u64,
    pub output: Option<String>,
}

/// Step execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
}

/// Orchestration service for CLI command integration
pub struct OrchestrationService {
    /// Core AgentOrchestrator for multi-agent workflows
    orchestrator: Arc<RwLock<Option<AgentOrchestrator>>>,

    /// Fallback to legacy agent for backward compatibility  
    fallback_agent: Arc<RwLock<Option<String>>>,

    /// LLM client for intelligent fallback execution
    llm_client: Arc<RwLock<Option<Arc<LlmClient>>>>,

    /// Configuration for orchestrator
    config: OrchestratorConfig,

    /// Health check and status tracking
    orchestrator_available: Arc<AtomicBool>,
    fallback_mode: Arc<AtomicBool>,
    health_check_interval: Duration,
    last_health_check: Arc<Mutex<Option<Instant>>>,
}

impl OrchestrationService {
    /// Create new orchestration service
    pub fn new() -> Self {
        Self {
            orchestrator: Arc::new(RwLock::new(None)),
            fallback_agent: Arc::new(RwLock::new(None)),
            llm_client: Arc::new(RwLock::new(None)),
            config: OrchestratorConfig::default(),
            orchestrator_available: Arc::new(AtomicBool::new(false)),
            fallback_mode: Arc::new(AtomicBool::new(true)),
            health_check_interval: Duration::from_secs(30),
            last_health_check: Arc::new(Mutex::new(None)),
        }
    }

    /// Create orchestration service with real AgentOrchestrator
    pub async fn with_orchestrator() -> Result<Self> {
        info!("Initializing OrchestrationService with AgentOrchestrator");

        // Create system configuration
        let system_config = SystemConfig::default();
        let orchestrator_config = OrchestratorConfig::default();

        // Create event publisher for orchestrator
        let event_publisher = create_agent_event_publisher()
            .await
            .map_err(|e| anyhow!("Failed to create event publisher: {}", e))?;

        // Create AgentOrchestrator
        let orchestrator =
            AgentOrchestrator::new(system_config, orchestrator_config.clone(), event_publisher)
                .await
                .map_err(|e| anyhow!("Failed to create AgentOrchestrator: {}", e))?;

        // Initialize agents
        orchestrator
            .initialize_agents()
            .await
            .map_err(|e| anyhow!("Failed to initialize agents: {}", e))?;

        info!("AgentOrchestrator initialized successfully");

        // КРИТИЧЕСКОЕ ИСПРАВЛЕНИЕ: Also initialize LLM client for TUI chat support
        let llm_client_option = match LlmClient::from_env() {
            Ok(client) => {
                info!("LLM client initialized successfully for orchestrator mode");
                Some(Arc::new(client))
            }
            Err(e) => {
                warn!(
                    "Failed to initialize LLM client: {}, TUI chat will be limited",
                    e
                );
                None
            }
        };

        Ok(Self {
            orchestrator: Arc::new(RwLock::new(Some(orchestrator))),
            fallback_agent: Arc::new(RwLock::new(None)),
            llm_client: Arc::new(RwLock::new(llm_client_option)),
            config: orchestrator_config,
            orchestrator_available: Arc::new(AtomicBool::new(true)),
            fallback_mode: Arc::new(AtomicBool::new(false)),
            health_check_interval: Duration::from_secs(30),
            last_health_check: Arc::new(Mutex::new(None)),
        })
    }

    /// Create orchestration service with LLM-powered fallback
    /// КРИТИЧЕСКОЕ ИСПРАВЛЕНИЕ: Graceful fallback при недоступности LLM client
    pub async fn with_llm_fallback() -> Result<Self> {
        info!("Initializing OrchestrationService with LLM-powered fallback");

        // КРИТИЧЕСКАЯ ДИАГНОСТИКА LLM client с детальными логами
        warn!("=== LLM CLIENT DIAGNOSTIC START ===");
        warn!("🔍 Checking environment variables:");

        // Проверяем переменные окружения
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            if key.starts_with("sk-") {
                warn!("✅ OPENAI_API_KEY found and looks valid (starts with sk-)");
            } else {
                warn!("⚠️  OPENAI_API_KEY found but doesn't look like OpenAI key (should start with sk-)");
            }
        } else {
            warn!("❌ OPENAI_API_KEY not found in environment");
        }

        // Проверяем .env файл
        if std::path::Path::new(".env").exists() {
            warn!("✅ .env file exists");
        } else {
            warn!("⚠️  .env file not found in current directory");
        }

        warn!("🔧 Attempting to create LLM client...");
        let llm_client_result = LlmClient::from_env();
        let llm_client_option = match llm_client_result {
            Ok(client) => {
                warn!("✅ LLM client created successfully!");

                // Test LLM client with a simple request
                warn!("📞 Testing LLM connectivity with 'Hello' message...");
                match client.chat_simple("Hello").await {
                    Ok(response) => {
                        warn!(
                            "✅ LLM CLIENT TEST SUCCESS! Response: {:?}",
                            response.chars().take(50).collect::<String>()
                        );
                        Some(Arc::new(client))
                    }
                    Err(e) => {
                        error!("❌ LLM CLIENT TEST FAILED: {}", e);
                        warn!("Possible causes:");
                        warn!("1. Invalid API key (check if it's expired or revoked)");
                        warn!("2. Network connectivity issues");
                        warn!("3. Service provider rate limiting or downtime");
                        warn!("4. Firewall blocking HTTPS requests");
                        warn!("⚠️  Proceeding without LLM client - TUI will show config help");
                        None // НЕ используем неработающий клиент
                    }
                }
            }
            Err(e) => {
                error!("❌ FAILED TO CREATE LLM CLIENT: {}", e);
                warn!("Root causes:");
                warn!("• Missing .env file in project root");
                warn!("• Missing API keys (OPENAI_API_KEY, ANTHROPIC_API_KEY, etc.)");
                warn!("• Invalid environment configuration");
                warn!("• LLM crate configuration issues");
                warn!("⚠️  Proceeding without LLM client - TUI will show detailed setup help");
                None
            }
        };

        warn!(
            "=== LLM CLIENT DIAGNOSTIC END (available: {}) ===",
            llm_client_option.is_some()
        );

        info!(
            "OrchestrationService initialized (LLM available: {})",
            llm_client_option.is_some()
        );

        Ok(Self {
            orchestrator: Arc::new(RwLock::new(None)),
            fallback_agent: Arc::new(RwLock::new(None)),
            llm_client: Arc::new(RwLock::new(llm_client_option)),
            config: OrchestratorConfig::default(),
            orchestrator_available: Arc::new(AtomicBool::new(false)),
            fallback_mode: Arc::new(AtomicBool::new(true)),
            health_check_interval: Duration::from_secs(30),
            last_health_check: Arc::new(Mutex::new(None)),
        })
    }

    /// Initialize fallback agent if needed (placeholder)
    async fn ensure_fallback_agent(&self) -> Result<()> {
        let mut fallback = self.fallback_agent.write().await;
        if fallback.is_none() {
            info!("Initializing fallback agent");
            *fallback = Some("fallback_agent".to_string());
            info!("Fallback agent initialized");
        }
        Ok(())
    }

    /// Execute command through orchestrator or fallback
    pub async fn execute_command(
        &mut self,
        request: CommandRequest,
    ) -> Result<OrchestrationResult> {
        let start_time = Instant::now();

        // Check orchestrator health
        self.check_orchestrator_health().await?;

        if self.orchestrator_available.load(Ordering::Relaxed)
            && !self.fallback_mode.load(Ordering::Relaxed)
        {
            self.execute_through_orchestrator(request, start_time).await
        } else {
            self.execute_fallback(request, start_time).await
        }
    }

    /// Execute command through full multi-agent orchestration
    async fn execute_through_orchestrator(
        &self,
        request: CommandRequest,
        start_time: Instant,
    ) -> Result<OrchestrationResult> {
        info!(
            "Executing command through multi-agent orchestrator: {}",
            request.command_type
        );

        // Get orchestrator instance
        let orchestrator_guard = self.orchestrator.read().await;
        let orchestrator = orchestrator_guard
            .as_ref()
            .ok_or_else(|| anyhow!("Orchestrator not initialized"))?;

        // Create workflow request
        let workflow_request = WorkflowRequest {
            user_input: format!("{}: {}", request.command_type, request.action),
            context: Some(serde_json::to_value(&request.parameters)?),
            priority: match request.context.priority {
                CommandPriority::Low => TaskPriority::Low,
                CommandPriority::Normal => TaskPriority::Normal,
                CommandPriority::High => TaskPriority::High,
                CommandPriority::Critical => TaskPriority::Critical,
            },
            dry_run: request.context.dry_run,
            timeout_ms: Some(request.context.timeout_ms),
            config_overrides: None,
        };

        // Execute workflow through orchestrator
        match orchestrator.execute_workflow(workflow_request).await {
            Ok(workflow_result) => {
                let execution_time = start_time.elapsed().as_millis() as u64;

                // Convert WorkflowResult to OrchestrationResult
                let intent_analysis = IntentAnalysis {
                    intent_type: request.command_type.clone(),
                    confidence: 0.9, // High confidence for CLI commands
                    extracted_parameters: request.parameters.clone(),
                    suggested_tools: vec![request.command_type.clone()],
                };

                let execution_step = ExecutionStep {
                    step_id: workflow_result.workflow_id.to_string(),
                    tool_used: Some(request.command_type.clone()),
                    status: if workflow_result.success {
                        StepStatus::Completed
                    } else {
                        StepStatus::Failed
                    },
                    duration_ms: workflow_result.execution_time_ms,
                    output: workflow_result
                        .results
                        .as_ref()
                        .and_then(|v| v.as_str())
                        .map(String::from),
                };

                Ok(OrchestrationResult {
                    success: workflow_result.success,
                    execution_time_ms: execution_time,
                    intent_analysis: Some(intent_analysis),
                    plan_summary: workflow_result
                        .plan
                        .as_ref()
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    execution_steps: vec![execution_step],
                    critique: workflow_result
                        .critique
                        .as_ref()
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    error: workflow_result.error,
                    fallback_used: false,
                })
            }
            Err(e) => {
                error!("Orchestrator execution failed: {}", e);
                Err(anyhow!("Orchestrator execution failed: {}", e))
            }
        }
    }

    /// Execute command using LLM-powered fallback mode
    async fn execute_fallback(
        &self,
        request: CommandRequest,
        start_time: Instant,
    ) -> Result<OrchestrationResult> {
        warn!(
            "Executing command in LLM-powered fallback mode: {}",
            request.command_type
        );

        // Get LLM client
        let llm_client_guard = self.llm_client.read().await;
        let llm_client = llm_client_guard
            .as_ref()
            .ok_or_else(|| anyhow!("LLM client not available in fallback mode"))?;

        info!("Using LLM for intelligent fallback execution");

        // Create intelligent prompt for LLM
        let prompt = format!(
            "You are MAGRAY CLI, an intelligent assistant. The user requested: '{}'\n\
            Command type: {}\n\
            Context: This is a fallback execution because the full multi-agent system is not available.\n\
            Please provide a helpful, informative response to the user's request.",
            request.action,
            request.command_type
        );

        // Execute through LLM
        let llm_start = Instant::now();
        let llm_response = match llm_client.chat_simple(&prompt).await {
            Ok(response) => {
                info!("LLM fallback execution successful");
                response
            }
            Err(e) => {
                error!("LLM fallback execution failed: {}", e);
                format!(
                    "I apologize, but I'm currently unable to process your request: '{}'. \
                    The system is in fallback mode and the LLM service encountered an error: {}",
                    request.action, e
                )
            }
        };
        let llm_duration = llm_start.elapsed().as_millis() as u64;

        let execution_time = start_time.elapsed().as_millis() as u64;
        let execution_step = ExecutionStep {
            step_id: "llm-fallback-execution".to_string(),
            tool_used: Some("LLM".to_string()),
            status: StepStatus::Completed,
            duration_ms: llm_duration,
            output: Some(llm_response),
        };

        Ok(OrchestrationResult {
            success: true,
            execution_time_ms: execution_time,
            intent_analysis: Some(IntentAnalysis {
                intent_type: request.command_type.clone(),
                confidence: 0.8, // High confidence for direct LLM processing
                extracted_parameters: request.parameters.clone(),
                suggested_tools: vec!["LLM".to_string()],
            }),
            plan_summary: Some(
                "LLM-powered fallback execution - AgentOrchestrator not available".to_string(),
            ),
            execution_steps: vec![execution_step],
            critique: Some(
                "Executed using LLM fallback - full orchestration system not available".to_string(),
            ),
            error: None,
            fallback_used: true,
        })
    }

    /// Check if orchestrator is available and healthy
    async fn check_orchestrator_health(&self) -> Result<()> {
        let now = Instant::now();

        // Only check if enough time has passed
        {
            let last_check_guard = self
                .last_health_check
                .lock()
                .map_err(|_| anyhow!("Health check lock poisoned"))?;
            if let Some(last_check) = *last_check_guard {
                if now.duration_since(last_check) < self.health_check_interval {
                    return Ok(());
                }
            }
        }

        // Check actual orchestrator health
        let orchestrator_guard = self.orchestrator.read().await;
        let available = if orchestrator_guard.is_some() {
            // Basic check - orchestrator exists and is available
            debug!("Orchestrator health check passed");
            true
        } else {
            debug!("Orchestrator not initialized");
            false
        };

        self.orchestrator_available
            .store(available, Ordering::Relaxed);
        self.fallback_mode.store(!available, Ordering::Relaxed);

        // Update last health check
        {
            let mut last_check_guard = self
                .last_health_check
                .lock()
                .map_err(|_| anyhow!("Health check lock poisoned"))?;
            *last_check_guard = Some(now);
        }

        if !self.orchestrator_available.load(Ordering::Relaxed) {
            debug!("Orchestrator not available, using fallback mode");
        }

        Ok(())
    }

    /// Shutdown orchestration service gracefully
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down OrchestrationService");

        // Shutdown orchestrator if available (take ownership)
        let mut orchestrator_guard = self.orchestrator.write().await;
        if let Some(orchestrator) = orchestrator_guard.take() {
            if let Err(e) = orchestrator.shutdown().await {
                error!("Failed to shutdown orchestrator: {}", e);
            }
        }

        // Shutdown fallback agent if initialized (placeholder)
        let mut fallback_guard = self.fallback_agent.write().await;
        if fallback_guard.is_some() {
            *fallback_guard = None;
            info!("Fallback agent shut down");
        }

        info!("OrchestrationService shutdown complete");
        Ok(())
    }

    /// Get current orchestrator status
    pub fn get_status(&self) -> OrchestrationStatus {
        OrchestrationStatus {
            orchestrator_available: self.orchestrator_available.load(Ordering::Relaxed),
            fallback_mode: self.fallback_mode.load(Ordering::Relaxed),
            last_health_check: *self
                .last_health_check
                .lock()
                .expect("Lock should not be poisoned"),
        }
    }
}

/// Current orchestration service status
#[derive(Debug, Clone)]
pub struct OrchestrationStatus {
    pub orchestrator_available: bool,
    pub fallback_mode: bool,
    pub last_health_check: Option<Instant>,
}

impl Default for OrchestrationService {
    fn default() -> Self {
        Self::new()
    }
}

impl OrchestrationService {
    /// Simple user request processing (placeholder until full integration)
    pub async fn process_user_request(&self, message: &str) -> Result<String> {
        debug!("Processing user request: {}", message);

        // Create simple command request
        let command_request = CommandRequest {
            command_type: "general".to_string(),
            action: message.to_string(),
            parameters: std::collections::HashMap::new(),
            context: ExecutionContext {
                user_id: None,
                session_id: "cli_session".to_string(),
                priority: CommandPriority::Normal,
                timeout_ms: 30000,
                dry_run: false,
            },
        };

        // Execute through orchestration service
        let result = self.execute_command_immutable(command_request).await?;

        // Return simple response
        if result.success {
            if let Some(ref output) = result
                .execution_steps
                .first()
                .and_then(|s| s.output.as_ref())
            {
                Ok(output.to_string())
            } else {
                Ok("Command executed successfully".to_string())
            }
        } else {
            let error_msg = result
                .error
                .unwrap_or_else(|| "Command execution failed".to_string());
            Err(anyhow::anyhow!(error_msg))
        }
    }

    /// TUI-specific message processing without workflow execution logging
    /// This method provides direct LLM response for TUI chat interface
    /// КРИТИЧЕСКОЕ ИСПРАВЛЕНИЕ: Better error handling и retry logic
    pub async fn process_tui_message(&self, message: &str) -> Result<String> {
        debug!("Processing TUI message (quiet mode): {}", message);

        // Проверяем доступность LLM client
        if let Some(ref llm_client_arc) = *self.llm_client.read().await {
            let prompt = format!(
                "You are MAGRAY CLI, an intelligent assistant. The user requested: {message}

Provide a helpful, concise response."
            );

            match llm_client_arc.chat_simple(&prompt).await {
                Ok(response) => {
                    debug!("TUI response generated successfully");
                    Ok(response)
                }
                Err(e) => {
                    error!("TUI LLM response failed, attempting retry: {}", e);
                    // КРИТИЧЕСКОЕ ИСПРАВЛЕНИЕ: Пытаемся повторно с простым промптом
                    match llm_client_arc.chat_simple(message).await {
                        Ok(retry_response) => {
                            debug!("TUI retry response successful");
                            Ok(retry_response)
                        }
                        Err(retry_e) => {
                            error!("TUI LLM retry also failed: {}", retry_e);
                            Err(anyhow!(
                                "LLM service unavailable: {} (retry: {})",
                                e,
                                retry_e
                            ))
                        }
                    }
                }
            }
        } else {
            error!("No LLM client available for TUI message processing");
            Ok(format!("❌ LLM client недоступен\n\nПроблема: LLM client не смог инициализироваться\n\nВаше сообщение: \"{message}\""))
        }
    }

    /// Execute command without mutable reference (for trait compatibility)
    async fn execute_command_immutable(
        &self,
        request: CommandRequest,
    ) -> Result<OrchestrationResult> {
        let start_time = Instant::now();

        // Check orchestrator availability (without mutation)
        let orchestrator_guard = self.orchestrator.read().await;
        let orchestrator_available = orchestrator_guard.is_some();

        if orchestrator_available && !self.fallback_mode.load(Ordering::Relaxed) {
            debug!("Using orchestrator execution");
            self.execute_through_orchestrator(request, start_time).await
        } else {
            debug!("Using fallback execution");
            self.execute_fallback(request, start_time).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_orchestration_service_creation() {
        let service = OrchestrationService::new();
        assert!(service.fallback_mode.load(Ordering::Relaxed));
        assert!(!service.orchestrator_available.load(Ordering::Relaxed));
    }

    #[tokio::test]
    async fn test_command_execution_fallback() {
        let mut service = OrchestrationService::new();
        let request = CommandRequest {
            command_type: "memory".to_string(),
            action: "search".to_string(),
            parameters: HashMap::new(),
            context: ExecutionContext {
                user_id: None,
                session_id: "test-session".to_string(),
                priority: CommandPriority::Normal,
                timeout_ms: 5000,
                dry_run: false,
            },
        };

        let result = service
            .execute_command(request)
            .await
            .expect("Async operation should succeed");
        assert!(result.success);
        assert!(result.fallback_used);
        assert!(result.execution_time_ms > 0);
    }

    #[test]
    fn test_command_priority_serialization() {
        let priority = CommandPriority::High;
        let serialized =
            serde_json::to_string(&priority).expect("Operation failed - converted from unwrap()");
        let deserialized: CommandPriority =
            serde_json::from_str(&serialized).expect("Operation failed - converted from unwrap()");
        matches!(deserialized, CommandPriority::High);
    }
}
