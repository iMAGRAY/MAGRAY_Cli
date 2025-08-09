use crate::event_bus::Topic;

// Canonical topics used across the platform (see ARCHITECTURE_PLAN_ADVANCED.md)
pub const TOPIC_INTENT: Topic = Topic("intent");
pub const TOPIC_PLAN: Topic = Topic("plan");
pub const TOPIC_TOOL_INVOKED: Topic = Topic("tool.invoked");
pub const TOPIC_FS_DIFF: Topic = Topic("fs.diff");
pub const TOPIC_MEMORY_UPSERT: Topic = Topic("memory.upsert");
pub const TOPIC_MEMORY_SEARCH: Topic = Topic("memory.search");
pub const TOPIC_POLICY_BLOCK: Topic = Topic("policy.block");
pub const TOPIC_JOB_PROGRESS: Topic = Topic("job.progress");
pub const TOPIC_LLM_TOKENS: Topic = Topic("llm.tokens");
pub const TOPIC_ERROR: Topic = Topic("error");