// @component: {"k":"C","id":"intelligent_tool_selector","t":"AI-powered tool selection with context analysis","m":{"cur":5,"tgt":90,"u":"%"},"f":["ai","selection","nlp","context","intent"]}

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, info};
use serde::{Serialize, Deserialize};

use crate::ToolSpec;

/// Confidence level for tool selection
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct ToolConfidence {
    pub tool_name: String,
    pub confidence_score: f32, // 0.0 to 1.0
    pub reasoning: String,
    pub context_match: f32,      // 0.0 to 1.0
    pub capability_match: f32,   // 0.0 to 1.0
    pub performance_factor: f32, // 0.0 to 1.0
}

/// Detailed breakdown of scoring for explanation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScoreBreakdown {
    pub name_match: f32,
    pub desc_overlap: f32,
    pub guide_caps: f32,
    pub guide_tags: f32,
    pub guide_good_for: f32,
    pub example_overlap: f32,
    pub urgency_latency_bonus: f32,
    pub low_risk_bonus: f32,
}

/// Matched signals from UsageGuide
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MatchedSignals {
    pub tags: Vec<String>,
    pub capabilities: Vec<String>,
    pub good_for: Vec<String>,
}

/// Public DTO for selection explanation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSelectionExplanation {
    pub tool_name: String,
    pub confidence_score: f32,
    pub context_match: f32,
    pub capability_match: f32,
    pub performance_factor: f32,
    pub breakdown: ScoreBreakdown,
    pub matched: MatchedSignals,
}

/// Context for tool selection
#[derive(Debug, Clone)]
pub struct ToolSelectionContext {
    pub user_query: String,
    pub session_context: HashMap<String, String>,
    pub previous_tools_used: Vec<String>,
    pub task_complexity: TaskComplexity,
    pub urgency_level: UrgencyLevel,
    pub user_expertise: UserExpertise,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskComplexity {
    Simple,  // Single tool operation
    Medium,  // Multiple steps, single domain
    Complex, // Multi-domain, orchestrated
    Expert,  // Advanced workflows, custom logic
}

#[derive(Debug, Clone, PartialEq)]
pub enum UrgencyLevel {
    Low,      // Background task
    Normal,   // Standard priority
    High,     // User waiting
    Critical, // Time-sensitive
}

#[derive(Debug, Clone, PartialEq)]
pub enum UserExpertise {
    Beginner,     // Needs guided assistance
    Intermediate, // Some technical knowledge
    Advanced,     // Comfortable with tools
    Expert,       // Prefers minimal assistance
}

/// Performance metrics for tool selection
#[derive(Debug, Clone, Default)]
pub struct SelectionMetrics {
    pub total_selections: u64,
    pub successful_selections: u64,
    pub average_confidence: f32,
    pub selection_time_ms: f32,
    pub tool_usage_stats: HashMap<String, u64>,
    pub context_hit_rate: f32,
}

/// Intelligent tool selector with AI-powered analysis
pub struct IntelligentToolSelector {
    // Available tools registry
    available_tools: Arc<Mutex<HashMap<String, ToolSpec>>>,

    // Selection performance tracking
    selection_metrics: Arc<Mutex<SelectionMetrics>>,

    // Context analysis patterns
    context_patterns: Arc<Mutex<HashMap<String, Vec<String>>>>, // intent -> tools

    // Performance history for adaptive learning
    performance_history: Arc<Mutex<HashMap<String, ToolPerformanceData>>>,

    // Configuration
    config: SelectorConfig,
}

#[derive(Debug, Clone)]
pub struct ToolPerformanceData {
    pub tool_name: String,
    pub success_rate: f32,
    pub average_execution_time: Duration,
    pub user_satisfaction: f32,
    pub context_relevance: f32,
    pub last_used: std::time::Instant,
    pub usage_count: u64,
}

#[derive(Debug, Clone)]
pub struct SelectorConfig {
    pub min_confidence_threshold: f32,
    pub context_weight: f32,
    pub performance_weight: f32,
    pub recency_weight: f32,
    pub enable_learning: bool,
    pub max_suggestions: usize,
}

impl Default for SelectorConfig {
    fn default() -> Self {
        Self {
            min_confidence_threshold: 0.6,
            context_weight: 0.4,
            performance_weight: 0.3,
            recency_weight: 0.3,
            enable_learning: true,
            max_suggestions: 5,
        }
    }
}

impl IntelligentToolSelector {
    pub fn new(config: SelectorConfig) -> Self {
        info!("🧠 Initializing Intelligent Tool Selector");

        Self {
            available_tools: Arc::new(Mutex::new(HashMap::new())),
            selection_metrics: Arc::new(Mutex::new(SelectionMetrics::default())),
            context_patterns: Arc::new(Mutex::new(Self::init_context_patterns())),
            performance_history: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    /// Initialize common context patterns for tool selection
    fn init_context_patterns() -> HashMap<String, Vec<String>> {
        let mut patterns = HashMap::new();

        // File operations
        patterns.insert("read_file".to_string(), vec!["file_read".to_string()]);
        patterns.insert("write_file".to_string(), vec!["file_write".to_string()]);
        patterns.insert("list_directory".to_string(), vec!["dir_list".to_string()]);
        patterns.insert(
            "search_files".to_string(),
            vec!["file_search".to_string(), "dir_list".to_string()],
        );

        // Git operations
        patterns.insert("git_status".to_string(), vec!["git_status".to_string()]);
        patterns.insert(
            "git_commit".to_string(),
            vec!["git_commit".to_string(), "git_status".to_string()],
        );
        patterns.insert(
            "git_history".to_string(),
            vec!["git_log".to_string(), "git_diff".to_string()],
        );

        // Web operations
        patterns.insert("web_search".to_string(), vec!["web_search".to_string()]);
        patterns.insert("fetch_url".to_string(), vec!["web_fetch".to_string()]);

        // Shell operations
        patterns.insert("run_command".to_string(), vec!["shell_exec".to_string()]);
        patterns.insert("system_info".to_string(), vec!["shell_exec".to_string()]);

        patterns
    }

    /// Register available tool
    pub async fn register_tool(&self, tool_spec: ToolSpec) {
        let mut tools = self.available_tools.lock().await;
        tools.insert(tool_spec.name.clone(), tool_spec.clone());

        // Initialize performance tracking
        let mut history = self.performance_history.lock().await;
        let tool_name = tool_spec.name.clone();
        history.insert(
            tool_name.clone(),
            ToolPerformanceData {
                tool_name: tool_name.clone(),
                success_rate: 1.0, // Start optimistic
                average_execution_time: Duration::from_millis(100),
                user_satisfaction: 0.8,
                context_relevance: 0.7,
                last_used: std::time::Instant::now(),
                usage_count: 0,
            },
        );

        debug!("📝 Registered tool: {}", tool_name);
    }

    /// Select best tool(s) for given context
    pub async fn select_tools(
        &self,
        context: &ToolSelectionContext,
    ) -> Result<Vec<ToolConfidence>> {
        let start_time = std::time::Instant::now();

        debug!("🎯 Selecting tools for query: '{}'", context.user_query);

        // Update metrics
        {
            let mut metrics = self.selection_metrics.lock().await;
            metrics.total_selections += 1;
        }

        // Analyze query and extract intent
        let intent = self.analyze_intent(&context.user_query).await;
        debug!("🧠 Detected intent: {:?}", intent);

        // Get candidate tools based on intent
        let candidates = self.get_candidate_tools(&intent).await;

        // Score each candidate
        let mut scored_tools = Vec::new();
        for candidate in candidates {
            if let Some(confidence) = self.score_tool(&candidate, context).await {
                if confidence.confidence_score >= self.config.min_confidence_threshold {
                    scored_tools.push(confidence);
                }
            }
        }

        // Sort by confidence
        scored_tools.sort_by(|a, b| b.confidence_score.partial_cmp(&a.confidence_score).unwrap());

        // Limit results
        scored_tools.truncate(self.config.max_suggestions);

        // Update metrics
        let selection_time = start_time.elapsed();
        {
            let mut metrics = self.selection_metrics.lock().await;
            metrics.selection_time_ms = selection_time.as_millis() as f32;
            if !scored_tools.is_empty() {
                metrics.successful_selections += 1;
                metrics.average_confidence = scored_tools[0].confidence_score;
            }
        }

        info!(
            "✅ Selected {} tools in {:?}",
            scored_tools.len(),
            selection_time
        );
        Ok(scored_tools)
    }

    /// Select best tools with detailed explanations
    pub async fn select_tools_with_explanations(
        &self,
        context: &ToolSelectionContext,
    ) -> Result<Vec<ToolSelectionExplanation>> {
        let start_time = std::time::Instant::now();

        let intent = self.analyze_intent(&context.user_query).await;
        let candidates = self.get_candidate_tools(&intent).await;
        let mut results: Vec<ToolSelectionExplanation> = Vec::new();
        for candidate in candidates {
            if let Some(exp) = self.explain_tool_score(&candidate, context).await {
                if exp.confidence_score >= self.config.min_confidence_threshold {
                    results.push(exp);
                }
            }
        }
        results.sort_by(|a, b| b.confidence_score.partial_cmp(&a.confidence_score).unwrap());
        results.truncate(self.config.max_suggestions);

        let selection_time = start_time.elapsed();
        {
            let mut metrics = self.selection_metrics.lock().await;
            metrics.selection_time_ms = selection_time.as_millis() as f32;
            if !results.is_empty() {
                metrics.successful_selections += 1;
                metrics.average_confidence = results[0].confidence_score;
            }
        }
        Ok(results)
    }

    /// Analyze user query to extract intent
    async fn analyze_intent(&self, query: &str) -> Vec<String> {
        let query_lower = query.to_lowercase();
        let mut intents = Vec::new();

        // File operations
        if query_lower.contains("read")
            || query_lower.contains("show")
            || query_lower.contains("cat")
            || query_lower.contains("content")
        {
            intents.push("read_file".to_string());
        }

        if query_lower.contains("write")
            || query_lower.contains("create")
            || query_lower.contains("save")
        {
            intents.push("write_file".to_string());
        }

        if query_lower.contains("list")
            || query_lower.contains("ls")
            || query_lower.contains("directory")
            || query_lower.contains("folder")
        {
            intents.push("list_directory".to_string());
        }

        if query_lower.contains("search") || query_lower.contains("find") {
            if query_lower.contains("web") || query_lower.contains("internet") {
                intents.push("web_search".to_string());
            } else {
                intents.push("search_files".to_string());
            }
        }

        // Git operations
        if query_lower.contains("git") {
            if query_lower.contains("status") {
                intents.push("git_status".to_string());
            } else if query_lower.contains("commit") {
                intents.push("git_commit".to_string());
            } else if query_lower.contains("log") || query_lower.contains("history") {
                intents.push("git_history".to_string());
            }
        }

        // Web operations
        if query_lower.contains("fetch")
            || query_lower.contains("download")
            || query_lower.contains("http")
            || query_lower.contains("url")
        {
            intents.push("fetch_url".to_string());
        }

        // Shell operations
        if query_lower.contains("run")
            || query_lower.contains("execute")
            || query_lower.contains("command")
            || query_lower.contains("shell")
        {
            intents.push("run_command".to_string());
        }

        if query_lower.contains("system")
            || query_lower.contains("info")
            || query_lower.contains("status")
        {
            intents.push("system_info".to_string());
        }

        // Default fallback
        if intents.is_empty() {
            intents.push("general".to_string());
        }

        intents
    }

    /// Get candidate tools for given intents
    async fn get_candidate_tools(&self, intents: &[String]) -> Vec<String> {
        let patterns = self.context_patterns.lock().await;
        let mut candidates = Vec::new();

        for intent in intents {
            if let Some(tools) = patterns.get(intent) {
                candidates.extend(tools.clone());
            }
        }

        // Remove duplicates
        candidates.sort();
        candidates.dedup();

        // If no specific matches, return all available tools
        if candidates.is_empty() {
            let tools = self.available_tools.lock().await;
            candidates = tools.keys().cloned().collect();
        }

        // Prefilter by environment/network restrictions
        let net_allow = std::env::var("MAGRAY_NET_ALLOW").unwrap_or_default();
        let net_disabled = net_allow.trim().is_empty();
        if net_disabled {
            candidates.retain(|t| t != "web_fetch" && t != "web_search");
        }

        candidates
    }

    /// Score a tool against the given context
    async fn score_tool(
        &self,
        tool_name: &str,
        context: &ToolSelectionContext,
    ) -> Option<ToolConfidence> {
        let tools = self.available_tools.lock().await;
        let tool_spec = tools.get(tool_name)?;

        // Base scoring components
        let context_match = self.calculate_context_match(tool_spec, context).await;
        let capability_match = self.calculate_capability_match(tool_spec, context).await;
        let performance_factor = self.calculate_performance_factor(tool_name).await;

        // Weighted final score
        let confidence_score = context_match * self.config.context_weight +
            capability_match * 0.4 + // Capability weight
            performance_factor * self.config.performance_weight;

        let reasoning = format!(
            "Context: {:.1}%, Capability: {:.1}%, Performance: {:.1}%",
            context_match * 100.0,
            capability_match * 100.0,
            performance_factor * 100.0
        );

        Some(ToolConfidence {
            tool_name: tool_name.to_string(),
            confidence_score,
            reasoning,
            context_match,
            capability_match,
            performance_factor,
        })
    }

    async fn explain_tool_score(
        &self,
        tool_name: &str,
        context: &ToolSelectionContext,
    ) -> Option<ToolSelectionExplanation> {
        let tools = self.available_tools.lock().await;
        let tool_spec = tools.get(tool_name)?.clone();
        drop(tools);

        let (context_match, mut breakdown, matched) =
            self.calculate_context_match_explained(&tool_spec, context).await;
        let (capability_match, cap_breakdown) =
            self.calculate_capability_match_explained(&tool_spec, context).await;
        breakdown.urgency_latency_bonus = cap_breakdown.urgency_latency_bonus;
        breakdown.low_risk_bonus = cap_breakdown.low_risk_bonus;
        let performance_factor = self.calculate_performance_factor(tool_name).await;

        let confidence_score = context_match * self.config.context_weight
            + capability_match * 0.4
            + performance_factor * self.config.performance_weight;

        Some(ToolSelectionExplanation {
            tool_name: tool_name.to_string(),
            confidence_score,
            context_match,
            capability_match,
            performance_factor,
            breakdown,
            matched,
        })
    }

    /// Calculate how well tool matches context
    async fn calculate_context_match(
        &self,
        tool_spec: &ToolSpec,
        context: &ToolSelectionContext,
    ) -> f32 {
        let (v, _, _) = self.calculate_context_match_explained(tool_spec, context).await;
        v
    }

    /// Calculate capability match based on task complexity
    async fn calculate_capability_match(
        &self,
        tool_spec: &ToolSpec,
        context: &ToolSelectionContext,
    ) -> f32 {
        let (v, _) = self.calculate_capability_match_explained(tool_spec, context).await;
        v
    }

    /// Calculate how well tool matches context
    async fn calculate_context_match_explained(
        &self,
        tool_spec: &ToolSpec,
        context: &ToolSelectionContext,
    ) -> (f32, ScoreBreakdown, MatchedSignals) {
        let mut score = 0.0f32;
        let mut bd = ScoreBreakdown::default();
        let mut matched = MatchedSignals::default();
        let query_lower = context.user_query.to_lowercase();

        // Check tool name relevance
        if query_lower.contains(&tool_spec.name.to_lowercase()) {
            bd.name_match = 0.4;
            score += bd.name_match;
        }

        // Check description relevance
        let desc_lower = tool_spec.description.to_lowercase();
        let desc_words: Vec<&str> = desc_lower.split_whitespace().collect();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        let matching_words = desc_words.iter().filter(|&w| query_words.contains(w)).count();
        if !desc_words.is_empty() {
            bd.desc_overlap = 0.2 * (matching_words as f32 / desc_words.len() as f32);
            score += bd.desc_overlap;
        }

        // UsageGuide relevance boost
        if let Some(guide) = &tool_spec.usage_guide {
            let mut caps_match = 0usize;
            let mut tags_match = 0usize;
            let mut good_for_match = 0usize;
            for c in &guide.capabilities {
                if query_lower.contains(&c.to_lowercase()) {
                    caps_match += 1;
                    matched.capabilities.push(c.clone());
                }
            }
            for t in &guide.tags {
                if query_lower.contains(&t.to_lowercase()) {
                    tags_match += 1;
                    matched.tags.push(t.clone());
                }
            }
            for g in &guide.good_for {
                if query_lower.contains(&g.to_lowercase()) {
                    good_for_match += 1;
                    matched.good_for.push(g.clone());
                }
            }
            let total = (caps_match + tags_match + good_for_match) as f32;
            if total > 0.0 {
                let boost = 0.3 * (1.0 - (1.0 / (1.0 + total)));
                // apportion boost proportional to counts
                let denom = (caps_match + tags_match + good_for_match).max(1) as f32;
                bd.guide_caps = boost * (caps_match as f32 / denom);
                bd.guide_tags = boost * (tags_match as f32 / denom);
                bd.guide_good_for = boost * (good_for_match as f32 / denom);
                score += boost;
            }
        }

        // Check examples relevance (take first overlap)
        for example in &tool_spec.examples {
            let example_lower = example.to_lowercase();
            let example_words: Vec<&str> = example_lower.split_whitespace().collect();
            let common_count = example_words.iter().filter(|&w| query_words.contains(w)).count();
            if !example_words.is_empty() {
                bd.example_overlap = 0.1 * (common_count as f32 / example_words.len() as f32);
                score += bd.example_overlap;
                break;
            }
        }

        (score.min(1.0), bd, matched)
    }

    /// Calculate capability match based on task complexity
    async fn calculate_capability_match_explained(
        &self,
        tool_spec: &ToolSpec,
        context: &ToolSelectionContext,
    ) -> (f32, ScoreBreakdown) {
        let base = match context.task_complexity {
            TaskComplexity::Simple => 0.7,
            TaskComplexity::Medium => 0.6,
            TaskComplexity::Complex => 0.5,
            TaskComplexity::Expert => 0.4,
        };
        let mut bd = ScoreBreakdown::default();
        if let Some(guide) = &tool_spec.usage_guide {
            if guide.risk_score <= 2 { bd.low_risk_bonus = 0.1; }
            if matches!(context.urgency_level, UrgencyLevel::High | UrgencyLevel::Critical)
                && guide.latency_class == "fast" { bd.urgency_latency_bonus = 0.1; }
            return ((base + bd.low_risk_bonus + bd.urgency_latency_bonus).min(1.0), bd);
        }
        (base, bd)
    }

    /// Calculate performance factor based on historical data
    async fn calculate_performance_factor(&self, tool_name: &str) -> f32 {
        let history = self.performance_history.lock().await;

        if let Some(perf_data) = history.get(tool_name) {
            // Weighted performance score
            let success_weight = 0.4;
            let satisfaction_weight = 0.3;
            let recency_weight = 0.3;

            let recency_factor = {
                let elapsed = perf_data.last_used.elapsed().as_secs() as f32;
                // Decay over 30 days
                (1.0 - (elapsed / (30.0 * 24.0 * 3600.0))).max(0.1)
            };

            perf_data.success_rate * success_weight
                + perf_data.user_satisfaction * satisfaction_weight
                + recency_factor * recency_weight
        } else {
            0.5 // Default for unknown tools
        }
    }

    /// Update tool performance after execution
    pub async fn update_tool_performance(
        &self,
        tool_name: &str,
        success: bool,
        execution_time: Duration,
        user_satisfaction: Option<f32>,
    ) {
        let mut history = self.performance_history.lock().await;

        if let Some(perf_data) = history.get_mut(tool_name) {
            // Update success rate (exponential moving average)
            let alpha = 0.1; // Learning rate
            perf_data.success_rate =
                (1.0 - alpha) * perf_data.success_rate + alpha * if success { 1.0 } else { 0.0 };

            // Update execution time
            perf_data.average_execution_time = Duration::from_millis(
                ((1.0 - alpha) * perf_data.average_execution_time.as_millis() as f32
                    + alpha * execution_time.as_millis() as f32) as u64,
            );

            // Update satisfaction if provided
            if let Some(satisfaction) = user_satisfaction {
                perf_data.user_satisfaction =
                    (1.0 - alpha) * perf_data.user_satisfaction + alpha * satisfaction;
            }

            perf_data.last_used = std::time::Instant::now();
            perf_data.usage_count += 1;

            debug!(
                "📊 Updated performance for {}: success={:.1}%, time={:?}",
                tool_name,
                perf_data.success_rate * 100.0,
                perf_data.average_execution_time
            );
        }
    }

    /// Get selection statistics
    pub async fn get_selection_stats(&self) -> String {
        let metrics = self.selection_metrics.lock().await;
        let history = self.performance_history.lock().await;

        let success_rate = if metrics.total_selections > 0 {
            (metrics.successful_selections as f32 / metrics.total_selections as f32) * 100.0
        } else {
            0.0
        };

        let mut stats = format!(
            "🧠 Intelligent Tool Selector Statistics:\n\n\
             📊 Performance Overview:\n\
             • Total selections: {}\n\
             • Success rate: {:.1}%\n\
             • Average confidence: {:.1}%\n\
             • Average selection time: {:.1}ms\n\n\
             🔧 Tool Performance:",
            metrics.total_selections,
            success_rate,
            metrics.average_confidence * 100.0,
            metrics.selection_time_ms
        );

        // Sort tools by usage
        let mut tool_stats: Vec<_> = history.iter().map(|(name, data)| (name, data)).collect();
        tool_stats.sort_by(|a, b| b.1.usage_count.cmp(&a.1.usage_count));

        for (name, data) in tool_stats.iter().take(10) {
            stats.push_str(&format!(
                "\n • {}: {} uses, {:.1}% success, {:.1}/5.0 satisfaction",
                name,
                data.usage_count,
                data.success_rate * 100.0,
                data.user_satisfaction * 5.0
            ));
        }

        if self.config.enable_learning {
            stats.push_str("\n\n🤖 Adaptive Learning: Enabled");
        }

        stats
    }
}

impl Default for IntelligentToolSelector {
    fn default() -> Self {
        Self::new(SelectorConfig::default())
    }
}
