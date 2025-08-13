//! Event topics for MAGRAY system
//!
//! As defined in ARCHITECTURE_PLAN_ADVANCED.md:
//! Topics: intent, plan, tool.invoked, fs.diff, memory.upsert, policy.block, job.progress, llm.tokens, error

/// Standard event topics used throughout MAGRAY
pub struct Topics;

impl Topics {
    /// Intent analysis events
    pub const INTENT: &'static str = "intent";
    pub const INTENT_ANALYZED: &'static str = "intent.analyzed";
    pub const INTENT_FAILED: &'static str = "intent.failed";

    /// Plan events
    pub const PLAN: &'static str = "plan";
    pub const PLAN_CREATED: &'static str = "plan.created";
    pub const PLAN_UPDATED: &'static str = "plan.updated";
    pub const PLAN_STARTED: &'static str = "plan.started";
    pub const PLAN_COMPLETED: &'static str = "plan.completed";
    pub const PLAN_FAILED: &'static str = "plan.failed";

    /// Tool execution events
    pub const TOOL: &'static str = "tool";
    pub const TOOL_INVOKED: &'static str = "tool.invoked";
    pub const TOOL_COMPLETED: &'static str = "tool.completed";
    pub const TOOL_FAILED: &'static str = "tool.failed";

    /// MCP tool audit events (CRITICAL P0.2.6)
    pub const MCP: &'static str = "mcp";
    pub const MCP_TOOL_INVOCATION: &'static str = "mcp.tool_invocation";
    pub const MCP_SECURITY_VIOLATION: &'static str = "mcp.security_violation";
    pub const MCP_AUDIT_TRAIL: &'static str = "mcp.audit_trail";

    /// File system events
    pub const FS: &'static str = "fs";
    pub const FS_DIFF: &'static str = "fs.diff";
    pub const FS_CREATED: &'static str = "fs.created";
    pub const FS_MODIFIED: &'static str = "fs.modified";
    pub const FS_DELETED: &'static str = "fs.deleted";

    /// Memory system events
    pub const MEMORY: &'static str = "memory";
    pub const MEMORY_UPSERT: &'static str = "memory.upsert";
    pub const MEMORY_SEARCH: &'static str = "memory.search";
    pub const MEMORY_PROMOTE: &'static str = "memory.promote";
    pub const MEMORY_INDEXED: &'static str = "memory.indexed";

    /// Policy events
    pub const POLICY: &'static str = "policy";
    pub const POLICY_BLOCK: &'static str = "policy.block";
    pub const POLICY_ALLOW: &'static str = "policy.allow";
    pub const POLICY_CONFIRM: &'static str = "policy.confirm";
    /// CRITICAL SECURITY: Emergency policy disable event
    pub const POLICY_EMERGENCY: &'static str = "policy.emergency";
    /// CRITICAL SECURITY: Policy violation events for audit trail
    pub const POLICY_VIOLATION: &'static str = "policy.violation";
    pub const POLICY_ASK: &'static str = "policy.ask";

    /// Job and progress events
    pub const JOB: &'static str = "job";
    pub const JOB_PROGRESS: &'static str = "job.progress";
    pub const JOB_STARTED: &'static str = "job.started";
    pub const JOB_COMPLETED: &'static str = "job.completed";
    pub const JOB_FAILED: &'static str = "job.failed";

    /// LLM interaction events
    pub const LLM: &'static str = "llm";
    pub const LLM_TOKENS: &'static str = "llm.tokens";
    pub const LLM_REQUEST: &'static str = "llm.request";
    pub const LLM_RESPONSE: &'static str = "llm.response";
    pub const LLM_ERROR: &'static str = "llm.error";

    /// Error events
    pub const ERROR: &'static str = "error";
    pub const ERROR_CRITICAL: &'static str = "error.critical";
    pub const ERROR_RECOVERABLE: &'static str = "error.recoverable";

    /// System events
    pub const SYSTEM: &'static str = "system";
    pub const SYSTEM_STARTUP: &'static str = "system.startup";
    pub const SYSTEM_SHUTDOWN: &'static str = "system.shutdown";
    pub const SYSTEM_HEALTH: &'static str = "system.health";

    /// Agent lifecycle events (P1.1.8 Integration)
    pub const AGENT: &'static str = "agent";
    pub const AGENT_STARTED: &'static str = "agent.started";
    pub const AGENT_STOPPED: &'static str = "agent.stopped";
    pub const AGENT_FAILED: &'static str = "agent.failed";
    pub const AGENT_HEALTH_CHECK: &'static str = "agent.health_check";

    /// Multi-agent workflow events (P1.1.8 Integration)
    pub const WORKFLOW: &'static str = "workflow";
    pub const WORKFLOW_INTENT_ANALYZED: &'static str = "workflow.intent_analyzed";
    pub const WORKFLOW_PLAN_CREATED: &'static str = "workflow.plan_created";
    pub const WORKFLOW_EXECUTION_STARTED: &'static str = "workflow.execution_started";
    pub const WORKFLOW_EXECUTION_COMPLETED: &'static str = "workflow.execution_completed";
    pub const WORKFLOW_EXECUTION_FAILED: &'static str = "workflow.execution_failed";
    pub const WORKFLOW_CRITIQUE_GENERATED: &'static str = "workflow.critique_generated";

    /// Agent communication events (P1.1.8 Integration)
    pub const AGENT_MESSAGE: &'static str = "agent.message";
    pub const AGENT_MESSAGE_SENT: &'static str = "agent.message.sent";
    pub const AGENT_MESSAGE_RECEIVED: &'static str = "agent.message.received";
    pub const AGENT_MESSAGE_FAILED: &'static str = "agent.message.failed";

    /// Scheduler events (P1.1.8 Integration)
    pub const SCHEDULER: &'static str = "scheduler";
    pub const SCHEDULER_JOB_SCHEDULED: &'static str = "scheduler.job_scheduled";
    pub const SCHEDULER_JOB_STARTED: &'static str = "scheduler.job_started";
    pub const SCHEDULER_JOB_COMPLETED: &'static str = "scheduler.job_completed";
    pub const SCHEDULER_JOB_FAILED: &'static str = "scheduler.job_failed";
    pub const SCHEDULER_TASK_SCHEDULED: &'static str = "scheduler.task_scheduled";
    pub const SCHEDULER_TASK_EXECUTED: &'static str = "scheduler.task_executed";

    /// Get all defined topics
    pub fn all() -> Vec<&'static str> {
        vec![
            Self::INTENT,
            Self::INTENT_ANALYZED,
            Self::INTENT_FAILED,
            Self::PLAN,
            Self::PLAN_CREATED,
            Self::PLAN_UPDATED,
            Self::PLAN_STARTED,
            Self::PLAN_COMPLETED,
            Self::PLAN_FAILED,
            Self::TOOL,
            Self::TOOL_INVOKED,
            Self::TOOL_COMPLETED,
            Self::TOOL_FAILED,
            Self::MCP,
            Self::MCP_TOOL_INVOCATION,
            Self::MCP_SECURITY_VIOLATION,
            Self::MCP_AUDIT_TRAIL,
            Self::FS,
            Self::FS_DIFF,
            Self::FS_CREATED,
            Self::FS_MODIFIED,
            Self::FS_DELETED,
            Self::MEMORY,
            Self::MEMORY_UPSERT,
            Self::MEMORY_SEARCH,
            Self::MEMORY_PROMOTE,
            Self::MEMORY_INDEXED,
            Self::POLICY,
            Self::POLICY_BLOCK,
            Self::POLICY_ALLOW,
            Self::POLICY_CONFIRM,
            Self::POLICY_EMERGENCY,
            Self::POLICY_VIOLATION,
            Self::POLICY_ASK,
            Self::JOB,
            Self::JOB_PROGRESS,
            Self::JOB_STARTED,
            Self::JOB_COMPLETED,
            Self::JOB_FAILED,
            Self::LLM,
            Self::LLM_TOKENS,
            Self::LLM_REQUEST,
            Self::LLM_RESPONSE,
            Self::LLM_ERROR,
            Self::ERROR,
            Self::ERROR_CRITICAL,
            Self::ERROR_RECOVERABLE,
            Self::SYSTEM,
            Self::SYSTEM_STARTUP,
            Self::SYSTEM_SHUTDOWN,
            Self::SYSTEM_HEALTH,
            Self::AGENT,
            Self::AGENT_STARTED,
            Self::AGENT_STOPPED,
            Self::AGENT_FAILED,
            Self::AGENT_HEALTH_CHECK,
            Self::WORKFLOW,
            Self::WORKFLOW_INTENT_ANALYZED,
            Self::WORKFLOW_PLAN_CREATED,
            Self::WORKFLOW_EXECUTION_STARTED,
            Self::WORKFLOW_EXECUTION_COMPLETED,
            Self::WORKFLOW_EXECUTION_FAILED,
            Self::WORKFLOW_CRITIQUE_GENERATED,
            Self::AGENT_MESSAGE,
            Self::AGENT_MESSAGE_SENT,
            Self::AGENT_MESSAGE_RECEIVED,
            Self::AGENT_MESSAGE_FAILED,
            Self::SCHEDULER,
            Self::SCHEDULER_JOB_SCHEDULED,
            Self::SCHEDULER_JOB_STARTED,
            Self::SCHEDULER_JOB_COMPLETED,
            Self::SCHEDULER_JOB_FAILED,
            Self::SCHEDULER_TASK_SCHEDULED,
            Self::SCHEDULER_TASK_EXECUTED,
        ]
    }

    /// Check if topic is valid
    pub fn is_valid(topic: &str) -> bool {
        Self::all().contains(&topic)
    }
}

/// Event payload types for different topics
pub mod payloads {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct IntentAnalyzedPayload {
        pub intent: crate::domain::Intent,
        pub processing_time_ms: u64,
        pub confidence: f32,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PlanCreatedPayload {
        pub plan: crate::domain::Plan,
        pub estimated_duration_seconds: Option<u64>,
        pub risk_level: crate::contracts::RiskLevel,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ToolInvokedPayload {
        pub tool_name: String,
        pub command: String,
        pub args: serde_json::Value,
        pub dry_run: bool,
        pub started_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ToolCompletedPayload {
        pub tool_name: String,
        pub command: String,
        pub result: crate::contracts::ToolResult,
        pub completed_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct FsDiffPayload {
        pub file_path: String,
        pub operation: FsOperation,
        pub diff_content: Option<String>,
        pub size_bytes: Option<u64>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum FsOperation {
        Created,
        Modified,
        Deleted,
        Moved { from: String, to: String },
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct MemoryUpsertPayload {
        pub record: crate::domain::MemoryRecord,
        pub operation: MemoryOperation,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum MemoryOperation {
        Insert,
        Update,
        Delete,
        Promote {
            from_layer: String,
            to_layer: String,
        },
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PolicyBlockPayload {
        pub action: crate::contracts::PolicyAction,
        pub reason: String,
        pub risk_level: crate::contracts::RiskLevel,
        pub requires_user_confirmation: bool,
    }

    /// CRITICAL SECURITY: Emergency policy disable event payload
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PolicyEmergencyPayload {
        pub token_hash: String,
        pub activation_time: DateTime<Utc>,
        pub activated_by: String,
        pub operation_context: String,
        pub severity: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct JobProgressPayload {
        pub job_id: Uuid,
        pub job_name: String,
        pub progress_percent: f32,
        pub estimated_remaining_seconds: Option<u64>,
        pub current_step: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LlmTokensPayload {
        pub model: String,
        pub prompt_tokens: u32,
        pub completion_tokens: u32,
        pub total_tokens: u32,
        pub cost_usd: Option<f64>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ErrorPayload {
        pub error_type: String,
        pub message: String,
        pub context: serde_json::Value,
        pub stack_trace: Option<String>,
        pub recoverable: bool,
    }

    /// CRITICAL P0.2.6: MCP tool audit event payload structure
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct McpAuditEvent {
        pub timestamp: DateTime<Utc>,
        pub tool_name: String,
        pub server_url: String,
        pub command: String,
        pub args: serde_json::Value,
        pub user_context: Option<String>,
        pub execution_result: McpExecutionResult,
        pub duration_ms: u64,
        pub resource_usage: McpResourceUsage,
        pub security_checks: McpSecurityCheckResults,
        pub dry_run: bool,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum McpExecutionResult {
        Success,
        Failed { error: String },
        SecurityBlocked { violation: String },
        Timeout,
        ConnectionFailed,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct McpResourceUsage {
        pub memory_peak_bytes: Option<u64>,
        pub cpu_time_ms: Option<u64>,
        pub network_requests: u32,
        pub filesystem_operations: u32,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct McpSecurityCheckResults {
        pub capability_validation: bool,
        pub signature_verification: bool,
        pub server_whitelist_check: bool,
        pub sandbox_policy_check: bool,
        pub policy_engine_decision: String,
    }

    /// CRITICAL P0.2.6: MCP security violation event payload
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct McpSecurityViolationPayload {
        pub timestamp: DateTime<Utc>,
        pub tool_name: String,
        pub server_url: String,
        pub violation_type: String,
        pub violation_details: String,
        pub security_check_failed: String,
        pub risk_level: String,
        pub blocked: bool,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_topics_valid() {
        for topic in Topics::all() {
            assert!(Topics::is_valid(topic), "Topic {topic} should be valid");
        }
    }

    #[test]
    fn test_invalid_topic() {
        assert!(!Topics::is_valid("invalid.topic"));
        assert!(!Topics::is_valid(""));
    }
}
