// @component: {"k":"C","id":"tool_context_builder","t":"Main ToolContextBuilder for intelligent tool selection","m":{"cur":0,"tgt":100,"u":"%"},"f":["context","builder","selection","orchestration"]}

use super::metadata::{ContextualMetadata, ExtractedMetadata, ToolMetadataExtractor};
use super::reranker::{QwenToolReranker, RerankingRequest, ToolReranker};
use super::{Result, ToolContextError};
use crate::registry::{SecureToolRegistry, ToolMetadata};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, instrument, warn};

/// User preferences for tool selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub prefer_gui_tools: bool,
    pub prefer_command_line: bool,
    pub max_tool_complexity: u8,
    pub preferred_languages: Vec<String>,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            prefer_gui_tools: false,
            prefer_command_line: true,
            max_tool_complexity: 5,
            preferred_languages: vec!["en".to_string()],
        }
    }
}

/// System context information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemContext {
    pub os: String,
    pub architecture: String,
    pub available_memory: u64,
    pub disk_space: u64,
    pub network_available: bool,
}

impl Default for SystemContext {
    fn default() -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            architecture: std::env::consts::ARCH.to_string(),
            available_memory: 0,
            disk_space: 0,
            network_available: true,
        }
    }
}

/// Project context information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub project_type: String,
    pub primary_language: Option<String>,
    pub frameworks: Vec<String>,
    pub dependencies: Vec<String>,
}

impl Default for ProjectContext {
    fn default() -> Self {
        Self {
            project_type: "unknown".to_string(),
            primary_language: None,
            frameworks: vec![],
            dependencies: vec![],
        }
    }
}

/// Performance priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PerformancePriority {
    Speed,
    Quality,
    Balanced,
    Resource,
}

impl Default for PerformancePriority {
    fn default() -> Self {
        Self::Balanced
    }
}

/// Tool selection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSelectionConfig {
    pub max_candidates: usize,
    pub top_n_tools: usize,
    pub similarity_threshold: f64,
    pub performance_priority: PerformancePriority,
    pub user_preferences: UserPreferences,
}

impl Default for ToolSelectionConfig {
    fn default() -> Self {
        Self {
            max_candidates: 50,
            top_n_tools: 10,
            similarity_threshold: 0.1,
            performance_priority: PerformancePriority::default(),
            user_preferences: UserPreferences::default(),
        }
    }
}

/// Enhanced tool selection context
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolSelectionContext {
    pub system: SystemContext,
    pub project: ProjectContext,
    pub user_preferences: UserPreferences,
    pub performance_priority: PerformancePriority,
}

/// Main Tool Context Builder for intelligent tool selection
pub struct ToolContextBuilder {
    registry: Arc<SecureToolRegistry>,
    metadata_extractor: ToolMetadataExtractor,
    reranker: Arc<dyn ToolReranker + Send + Sync>,
    config: ContextBuildingConfig,
}

/// Configuration for context building operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBuildingConfig {
    /// Maximum number of tools to consider for ranking
    pub max_candidate_tools: usize,

    /// Maximum number of tools to return in final context
    pub max_context_tools: usize,

    /// Minimum similarity score threshold
    pub similarity_threshold: f32,

    /// Whether to use semantic reranking
    pub use_semantic_reranking: bool,

    /// Cache context results for performance
    pub enable_caching: bool,

    /// Include usage patterns in context
    pub include_usage_patterns: bool,

    /// Include performance metrics in selection
    pub include_performance_metrics: bool,

    /// Maximum context building time
    pub max_build_time: Duration,
}

impl Default for ContextBuildingConfig {
    fn default() -> Self {
        Self {
            max_candidate_tools: 50,
            max_context_tools: 10,
            similarity_threshold: 0.1,
            use_semantic_reranking: true,
            enable_caching: true,
            include_usage_patterns: true,
            include_performance_metrics: true,
            max_build_time: Duration::from_secs(5),
        }
    }
}

/// Request for tool selection and context building
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSelectionRequest {
    /// User's intent or query
    pub query: String,

    /// Current context (file paths, project type, etc.)
    pub context: HashMap<String, String>,

    /// Required tool categories (optional filter)
    pub required_categories: Option<Vec<String>>,

    /// Exclude specific tools
    pub exclude_tools: Vec<String>,

    /// Platform constraints
    pub platform: Option<String>,

    /// Security constraints
    pub max_security_level: Option<String>,

    /// Performance preferences
    pub prefer_fast_tools: bool,

    /// Include experimental tools
    pub include_experimental: bool,
}

/// Response containing selected tools and context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSelectionResponse {
    /// Selected tools ranked by relevance
    pub tools: Vec<ToolRankingResult>,

    /// Context information for the selection
    pub context: ToolContext,

    /// Performance metrics for the selection process
    pub selection_metrics: SelectionMetrics,
}

/// Individual tool ranking result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRankingResult {
    /// Tool metadata
    pub metadata: ToolMetadata,

    /// Relevance score (0.0 to 1.0)
    pub relevance_score: f32,

    /// Semantic similarity score
    pub semantic_score: f32,

    /// Usage pattern match score
    pub usage_score: f32,

    /// Performance score
    pub performance_score: f32,

    /// Final combined score
    pub combined_score: f32,

    /// Reasoning for the ranking
    pub reasoning: String,
}

/// Built context for tool selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolContext {
    /// Primary query/intent
    pub query: String,

    /// Contextual metadata
    pub metadata: ContextualMetadata,

    /// Relevant tool categories identified
    pub relevant_categories: Vec<String>,

    /// Suggested tool combinations
    pub tool_combinations: Vec<Vec<String>>,

    /// Context building timestamp
    pub built_at: u64,

    /// Context validity duration
    pub valid_for: Duration,
}

/// Metrics for the selection process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionMetrics {
    /// Total time taken for selection
    pub total_time: Duration,

    /// Time spent on metadata extraction
    pub metadata_time: Duration,

    /// Time spent on reranking
    pub reranking_time: Duration,

    /// Number of candidate tools considered
    pub candidates_considered: usize,

    /// Number of tools after filtering
    pub tools_after_filtering: usize,

    /// Whether caching was used
    pub cache_hit: bool,
}

impl ToolContextBuilder {
    /// Create a new ToolContextBuilder with Qwen3 reranker
    pub fn new(registry: Arc<SecureToolRegistry>) -> Result<Self> {
        let config = ContextBuildingConfig::default();
        let metadata_extractor = ToolMetadataExtractor::new();

        // Try to initialize Qwen3 reranker, but allow fallback for testing
        let reranker = Arc::new(QwenToolReranker::new_or_fallback());

        Ok(Self {
            registry,
            metadata_extractor,
            reranker,
            config,
        })
    }

    /// Create ToolContextBuilder with custom configuration
    pub fn with_config(
        registry: Arc<SecureToolRegistry>,
        config: ContextBuildingConfig,
    ) -> Result<Self> {
        let metadata_extractor = ToolMetadataExtractor::new();

        let reranker = Arc::new(QwenToolReranker::new_or_fallback());

        Ok(Self {
            registry,
            metadata_extractor,
            reranker,
            config,
        })
    }

    /// Build tool context and select relevant tools
    #[instrument(skip(self), fields(query_len = request.query.len()))]
    pub async fn build_context(
        &self,
        request: ToolSelectionRequest,
    ) -> Result<ToolSelectionResponse> {
        let start_time = Instant::now();
        info!("Building tool context for query: '{}'", request.query);

        // Extract contextual metadata
        let metadata_start = Instant::now();
        let contextual_metadata = self
            .metadata_extractor
            .extract_contextual_metadata(&request)
            .await
            .map_err(|e| ToolContextError::MetadataExtractionFailed { source: e })?;
        let metadata_time = metadata_start.elapsed();

        // Get candidate tools from registry
        let candidates = self.get_candidate_tools(&request).await?;
        let candidates_count = candidates.len();
        debug!("Found {} candidate tools", candidates_count);

        // Filter tools based on constraints
        let filtered_tools = self.filter_tools(candidates, &request).await?;
        let filtered_count = filtered_tools.len();
        debug!("Filtered to {} tools", filtered_count);

        // Perform semantic reranking if enabled
        let reranking_start = Instant::now();
        let ranked_tools = if self.config.use_semantic_reranking {
            self.rerank_tools(&request.query, filtered_tools).await?
        } else {
            self.simple_rank_tools(filtered_tools, &request).await?
        };
        let reranking_time = reranking_start.elapsed();

        // Build final context
        let context = self
            .build_tool_context(&request, &contextual_metadata)
            .await?;

        let total_time = start_time.elapsed();

        let response = ToolSelectionResponse {
            tools: ranked_tools,
            context,
            selection_metrics: SelectionMetrics {
                total_time,
                metadata_time,
                reranking_time,
                candidates_considered: candidates_count,
                tools_after_filtering: filtered_count,
                cache_hit: false, // TODO: Implement caching
            },
        };

        info!(
            "Tool context built successfully in {:?} with {} tools selected",
            total_time,
            response.tools.len()
        );

        Ok(response)
    }

    /// Get candidate tools from registry
    async fn get_candidate_tools(
        &self,
        request: &ToolSelectionRequest,
    ) -> Result<Vec<ToolMetadata>> {
        // Create a basic security context for tool listing
        // TODO: This should come from the actual user session
        let security_context = crate::registry::SecurityContext {
            user_id: "system".to_string(),
            session_id: "context_builder".to_string(),
            permissions: crate::registry::UserPermissions {
                can_execute_high_risk: false,
                can_install_tools: false,
                can_modify_security: false,
                max_resource_usage: crate::registry::ResourceLimits::default(),
            },
            trust_level: crate::registry::UserTrustLevel::User,
        };

        let all_tools = self.registry.get_available_tools(&security_context).await;

        let mut candidates: Vec<ToolMetadata> = all_tools
            .into_iter()
            .filter(|tool| {
                // Basic filtering
                if request.exclude_tools.contains(&tool.id) {
                    return false;
                }

                // Category filtering
                if let Some(ref required_cats) = request.required_categories {
                    let tool_cat = format!("{:?}", tool.category);
                    if !required_cats.iter().any(|cat| tool_cat.contains(cat)) {
                        return false;
                    }
                }

                // Platform filtering (basic)
                if let Some(ref platform) = request.platform {
                    // TODO: Add platform-specific filtering
                    debug!("Platform filtering for {} not yet implemented", platform);
                }

                true
            })
            .collect();

        // Limit candidates
        candidates.truncate(self.config.max_candidate_tools);

        Ok(candidates)
    }

    /// Filter tools based on security and performance constraints
    async fn filter_tools(
        &self,
        candidates: Vec<ToolMetadata>,
        request: &ToolSelectionRequest,
    ) -> Result<Vec<ToolMetadata>> {
        let filtered: Vec<ToolMetadata> = candidates
            .into_iter()
            .filter(|tool| {
                // Security level filtering
                if let Some(ref max_security) = request.max_security_level {
                    let tool_security = format!("{:?}", tool.security_level);
                    if tool_security > *max_security {
                        return false;
                    }
                }

                // Performance filtering
                if request.prefer_fast_tools
                    && tool.performance_metrics.average_execution_time > Duration::from_secs(10)
                {
                    return false;
                }

                // Experimental tools filtering
                if !request.include_experimental && !tool.trusted {
                    return false;
                }

                // Health check
                if !tool.performance_metrics.is_healthy() {
                    warn!("Excluding unhealthy tool: {}", tool.id);
                    return false;
                }

                true
            })
            .collect();

        Ok(filtered)
    }

    /// Rerank tools using semantic similarity
    async fn rerank_tools(
        &self,
        query: &str,
        tools: Vec<ToolMetadata>,
    ) -> Result<Vec<ToolRankingResult>> {
        debug!("Reranking {} tools using semantic similarity", tools.len());

        // Prepare tool descriptions for reranking
        let tool_descriptions: Vec<String> = tools
            .iter()
            .map(|tool| {
                format!(
                    "{}: {} (Category: {:?})",
                    tool.name, tool.description, tool.category
                )
            })
            .collect();

        // Perform reranking
        let reranking_request = RerankingRequest {
            query: query.to_string(),
            documents: tool_descriptions,
            top_k: Some(self.config.max_context_tools),
        };

        let reranking_response = self
            .reranker
            .rerank(reranking_request)
            .await
            .map_err(|e| ToolContextError::RerankingFailed { source: e })?;

        // Convert to ranking results
        let mut ranked_tools = Vec::new();
        for scored_doc in reranking_response.ranked_documents {
            if let Some(tool) = tools.get(scored_doc.original_index) {
                let ranking_result = ToolRankingResult {
                    metadata: tool.clone(),
                    relevance_score: scored_doc.score,
                    semantic_score: scored_doc.score,
                    usage_score: self.calculate_usage_score(tool),
                    performance_score: self.calculate_performance_score(tool),
                    combined_score: scored_doc.score, // TODO: Combine all scores
                    reasoning: format!("Semantic similarity: {:.3}", scored_doc.score),
                };
                ranked_tools.push(ranking_result);
            }
        }

        Ok(ranked_tools)
    }

    /// Simple ranking without semantic reranking
    async fn simple_rank_tools(
        &self,
        tools: Vec<ToolMetadata>,
        request: &ToolSelectionRequest,
    ) -> Result<Vec<ToolRankingResult>> {
        debug!("Using simple ranking for {} tools", tools.len());

        let mut ranked_tools: Vec<ToolRankingResult> = tools
            .into_iter()
            .map(|tool| {
                let usage_score = self.calculate_usage_score(&tool);
                let performance_score = self.calculate_performance_score(&tool);
                let simple_relevance = self.calculate_simple_relevance(&tool, &request.query);

                let combined_score = (usage_score + performance_score + simple_relevance) / 3.0;

                ToolRankingResult {
                    metadata: tool,
                    relevance_score: simple_relevance,
                    semantic_score: 0.0, // Not used in simple ranking
                    usage_score,
                    performance_score,
                    combined_score,
                    reasoning: "Simple relevance + usage + performance".to_string(),
                }
            })
            .collect();

        // Sort by combined score (handle NaN cases by placing them at end)
        ranked_tools.sort_by(|a, b| {
            b.combined_score
                .partial_cmp(&a.combined_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit results
        ranked_tools.truncate(self.config.max_context_tools);

        Ok(ranked_tools)
    }

    /// Calculate usage score based on tool history
    fn calculate_usage_score(&self, tool: &ToolMetadata) -> f32 {
        if !self.config.include_usage_patterns {
            return 0.5; // Neutral score
        }

        let usage_count = tool.usage_count as f32;
        let success_rate = tool.performance_metrics.success_rate;

        // Combine usage frequency with success rate
        let frequency_score = (usage_count.ln() + 1.0) / 10.0; // Log scale
        let score = (frequency_score + success_rate) / 2.0;

        score.clamp(0.0, 1.0)
    }

    /// Calculate performance score
    fn calculate_performance_score(&self, tool: &ToolMetadata) -> f32 {
        if !self.config.include_performance_metrics {
            return 0.5; // Neutral score
        }

        let success_rate = tool.performance_metrics.success_rate;
        let execution_time = tool
            .performance_metrics
            .average_execution_time
            .as_secs_f32();

        // Prefer tools with high success rate and low execution time
        let time_score = 1.0 / (1.0 + execution_time / 10.0); // Inverse relationship
        let score = (success_rate + time_score) / 2.0;

        score.clamp(0.0, 1.0)
    }

    /// Calculate simple text-based relevance
    fn calculate_simple_relevance(&self, tool: &ToolMetadata, query: &str) -> f32 {
        let query_lower = query.to_lowercase();
        let tool_text = format!(
            "{} {} {:?}",
            tool.name.to_lowercase(),
            tool.description.to_lowercase(),
            tool.category
        );

        // Simple keyword matching
        let keywords: Vec<&str> = query_lower.split_whitespace().collect();
        let matches = keywords
            .iter()
            .filter(|keyword| tool_text.contains(*keyword))
            .count();

        if keywords.is_empty() {
            0.0
        } else {
            matches as f32 / keywords.len() as f32
        }
    }

    /// Build final tool context
    async fn build_tool_context(
        &self,
        request: &ToolSelectionRequest,
        metadata: &ContextualMetadata,
    ) -> Result<ToolContext> {
        Ok(ToolContext {
            query: request.query.clone(),
            metadata: metadata.clone(),
            relevant_categories: metadata.suggested_categories.clone(),
            tool_combinations: Vec::new(), // TODO: Implement tool combination suggestions
            built_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            valid_for: Duration::from_secs(300), // 5 minutes
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{SecurityConfig, UserPermissions, UserTrustLevel};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_tool_context_builder_creation() {
        let security_config = SecurityConfig::default();
        let registry = Arc::new(SecureToolRegistry::new(security_config));

        let builder = ToolContextBuilder::new(registry);
        assert!(builder.is_ok());
    }

    #[tokio::test]
    async fn test_simple_relevance_calculation() {
        let security_config = SecurityConfig::default();
        let registry = Arc::new(SecureToolRegistry::new(security_config));
        let builder =
            ToolContextBuilder::new(registry).expect("Operation failed - converted from unwrap()");

        let tool = ToolMetadata::new(
            "git_status".to_string(),
            "Git Status Tool".to_string(),
            crate::registry::SemanticVersion::new(1, 0, 0),
        )
        .with_description("Check git repository status".to_string());

        let relevance = builder.calculate_simple_relevance(&tool, "git status");
        assert!(relevance > 0.5);

        let irrelevance = builder.calculate_simple_relevance(&tool, "database query");
        assert!(irrelevance < 0.5);
    }
}
