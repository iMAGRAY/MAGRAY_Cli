// Tool selection algorithms and strategies

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::registry::{SecurityLevel, ToolCategory, ToolMetadata};
use crate::ToolSpec;

/// Tool selection strategy interface
pub trait SelectionStrategy {
    /// Select top tools based on query and context
    async fn select_tools(
        &self,
        query: &str,
        available_tools: &[(String, ToolMetadata, ToolSpec)],
        context: &super::ToolSelectionContext,
    ) -> Result<Vec<super::SelectedTool>>;
}

/// Embedding-based selection strategy
#[derive(Debug, Clone)]
pub struct EmbeddingSelectionStrategy {
    pub similarity_threshold: f64,
    pub max_candidates: usize,
    pub boost_factors: HashMap<String, f64>,
}

impl EmbeddingSelectionStrategy {
    pub fn new(similarity_threshold: f64, max_candidates: usize) -> Self {
        Self {
            similarity_threshold,
            max_candidates,
            boost_factors: HashMap::new(),
        }
    }

    pub fn with_boost_factor(mut self, tool_name: String, boost: f64) -> Self {
        self.boost_factors.insert(tool_name, boost);
        self
    }
}

impl SelectionStrategy for EmbeddingSelectionStrategy {
    async fn select_tools(
        &self,
        query: &str,
        available_tools: &[(String, ToolMetadata, ToolSpec)],
        _context: &super::ToolSelectionContext,
    ) -> Result<Vec<super::SelectedTool>> {
        // Placeholder implementation
        // Real embedding calculation would happen here
        let mut selected = Vec::new();

        for (name, metadata, spec) in available_tools {
            // Mock similarity calculation based on keyword matching
            let similarity = self.calculate_mock_similarity(query, spec);

            if similarity >= self.similarity_threshold {
                let boosted_similarity = similarity * self.boost_factors.get(name).unwrap_or(&1.0);

                let usage_guide = spec
                    .usage_guide
                    .clone()
                    .unwrap_or_else(|| crate::generate_usage_guide(spec));

                let selected_tool = super::SelectedTool {
                    name: name.clone(),
                    description: spec.description.clone(),
                    usage_guide,
                    similarity_score: boosted_similarity,
                    ranking_score: boosted_similarity,
                    selection_reason: format!("Keyword match similarity: {:.3}", similarity),
                    usage_conditions: vec![],
                };

                selected.push(selected_tool);
            }
        }

        // Sort by similarity and take top candidates
        selected.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).expect("Operation failed - converted from unwrap()"));
        selected.truncate(self.max_candidates);

        Ok(selected)
    }
}

impl EmbeddingSelectionStrategy {
    /// Mock similarity calculation for testing
    fn calculate_mock_similarity(&self, query: &str, spec: &ToolSpec) -> f64 {
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        let tool_text = format!(
            "{} {} {}",
            spec.name.to_lowercase(),
            spec.description.to_lowercase(),
            spec.usage.to_lowercase()
        );

        let matches = query_words
            .iter()
            .map(|word| if tool_text.contains(word) { 1.0 } else { 0.0 })
            .sum::<f64>();

        matches / query_words.len() as f64
    }
}

/// Hybrid selection combining multiple strategies
#[derive(Debug)]
pub struct HybridSelectionStrategy {
    strategies: Vec<ConcreteSelectionStrategy>,
    weights: Vec<f64>,
}

// Note: Due to trait object limitations, we'll use an enum for concrete strategies
#[derive(Debug, Clone)]
pub enum ConcreteSelectionStrategy {
    Embedding(EmbeddingSelectionStrategy),
    Keyword(KeywordSelectionStrategy),
    Category(CategorySelectionStrategy),
}

impl SelectionStrategy for ConcreteSelectionStrategy {
    async fn select_tools(
        &self,
        query: &str,
        available_tools: &[(String, ToolMetadata, ToolSpec)],
        context: &super::ToolSelectionContext,
    ) -> Result<Vec<super::SelectedTool>> {
        match self {
            ConcreteSelectionStrategy::Embedding(strategy) => {
                strategy.select_tools(query, available_tools, context).await
            }
            ConcreteSelectionStrategy::Keyword(strategy) => {
                strategy.select_tools(query, available_tools, context).await
            }
            ConcreteSelectionStrategy::Category(strategy) => {
                strategy.select_tools(query, available_tools, context).await
            }
        }
    }
}

/// Keyword-based selection strategy
#[derive(Debug, Clone)]
pub struct KeywordSelectionStrategy {
    pub keyword_weights: HashMap<String, f64>,
    pub max_candidates: usize,
}

impl KeywordSelectionStrategy {
    pub fn new(max_candidates: usize) -> Self {
        Self {
            keyword_weights: HashMap::new(),
            max_candidates,
        }
    }

    pub fn with_keyword_weight(mut self, keyword: String, weight: f64) -> Self {
        self.keyword_weights.insert(keyword, weight);
        self
    }
}

impl SelectionStrategy for KeywordSelectionStrategy {
    async fn select_tools(
        &self,
        query: &str,
        available_tools: &[(String, ToolMetadata, ToolSpec)],
        _context: &super::ToolSelectionContext,
    ) -> Result<Vec<super::SelectedTool>> {
        let mut selected = Vec::new();
        let query_lower = query.to_lowercase();

        for (name, _metadata, spec) in available_tools {
            let score = self.calculate_keyword_score(&query_lower, spec);

            if score > 0.0 {
                let usage_guide = spec
                    .usage_guide
                    .clone()
                    .unwrap_or_else(|| crate::generate_usage_guide(spec));

                let selected_tool = super::SelectedTool {
                    name: name.clone(),
                    description: spec.description.clone(),
                    usage_guide,
                    similarity_score: score,
                    ranking_score: score,
                    selection_reason: format!("Keyword match score: {:.3}", score),
                    usage_conditions: vec![],
                };

                selected.push(selected_tool);
            }
        }

        selected.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).expect("Operation failed - converted from unwrap()"));
        selected.truncate(self.max_candidates);

        Ok(selected)
    }
}

impl KeywordSelectionStrategy {
    fn calculate_keyword_score(&self, query: &str, spec: &ToolSpec) -> f64 {
        let tool_text = format!(
            "{} {} {}",
            spec.name.to_lowercase(),
            spec.description.to_lowercase(),
            spec.usage.to_lowercase()
        );

        self.keyword_weights
            .iter()
            .map(|(keyword, weight)| {
                if query.contains(keyword) && tool_text.contains(keyword) {
                    *weight
                } else {
                    0.0
                }
            })
            .sum()
    }
}

/// Category-based selection strategy
#[derive(Debug, Clone)]
pub struct CategorySelectionStrategy {
    pub category_preferences: HashMap<ToolCategory, f64>,
    pub max_candidates: usize,
}

impl CategorySelectionStrategy {
    pub fn new(max_candidates: usize) -> Self {
        Self {
            category_preferences: HashMap::new(),
            max_candidates,
        }
    }

    pub fn with_category_preference(mut self, category: ToolCategory, score: f64) -> Self {
        self.category_preferences.insert(category, score);
        self
    }
}

impl SelectionStrategy for CategorySelectionStrategy {
    async fn select_tools(
        &self,
        _query: &str,
        available_tools: &[(String, ToolMetadata, ToolSpec)],
        _context: &super::ToolSelectionContext,
    ) -> Result<Vec<super::SelectedTool>> {
        let mut selected = Vec::new();

        for (name, metadata, spec) in available_tools {
            if let Some(&score) = self.category_preferences.get(&metadata.category) {
                let usage_guide = spec
                    .usage_guide
                    .clone()
                    .unwrap_or_else(|| crate::generate_usage_guide(spec));

                let selected_tool = super::SelectedTool {
                    name: name.clone(),
                    description: spec.description.clone(),
                    usage_guide,
                    similarity_score: score,
                    ranking_score: score,
                    selection_reason: format!("Category preference: {:?}", metadata.category),
                    usage_conditions: vec![],
                };

                selected.push(selected_tool);
            }
        }

        selected.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).expect("Operation failed - converted from unwrap()"));
        selected.truncate(self.max_candidates);

        Ok(selected)
    }
}

/// Tool filtering utilities
pub struct ToolFilter;

impl ToolFilter {
    /// Filter tools by security level
    pub fn by_security_level(
        tools: Vec<(String, ToolMetadata, ToolSpec)>,
        max_level: SecurityLevel,
    ) -> Vec<(String, ToolMetadata, ToolSpec)> {
        tools
            .into_iter()
            .filter(|(_, metadata, _)| metadata.security_level <= max_level)
            .collect()
    }

    /// Filter tools by platform compatibility
    pub fn by_platform(
        tools: Vec<(String, ToolMetadata, ToolSpec)>,
        platform: &str,
    ) -> Vec<(String, ToolMetadata, ToolSpec)> {
        tools
            .into_iter()
            .filter(|(_, _, spec)| {
                spec.usage_guide
                    .as_ref()
                    .map(|guide| guide.platforms.contains(&platform.to_string()))
                    .unwrap_or(true) // Include tools without platform restrictions
            })
            .collect()
    }

    /// Filter tools by resource requirements
    pub fn by_resources(
        tools: Vec<(String, ToolMetadata, ToolSpec)>,
        available_memory_mb: u64,
        has_network: bool,
        has_gpu: bool,
    ) -> Vec<(String, ToolMetadata, ToolSpec)> {
        tools
            .into_iter()
            .filter(|(_, metadata, _)| {
                let reqs = &metadata.resource_requirements;

                // Check memory requirement
                if let Some(required_memory) = reqs.max_memory_mb {
                    if required_memory > available_memory_mb {
                        return false;
                    }
                }

                // Check network requirement
                if reqs.requires_network && !has_network {
                    return false;
                }

                // Check GPU requirement
                if reqs.requires_gpu && !has_gpu {
                    return false;
                }

                true
            })
            .collect()
    }

    /// Filter tools by user preferences
    pub fn by_user_preferences(
        tools: Vec<(String, ToolMetadata, ToolSpec)>,
        preferences: &super::UserPreferences,
    ) -> Vec<(String, ToolMetadata, ToolSpec)> {
        tools
            .into_iter()
            .filter(|(name, metadata, _)| {
                // Check if tool is avoided
                if preferences.avoided_tools.contains(name) {
                    return false;
                }

                // Check security risk tolerance
                if metadata.security_level > preferences.risk_tolerance {
                    return false;
                }

                true
            })
            .collect()
    }

    /// Apply boost to preferred tools
    pub fn boost_preferred_tools(
        mut tools: Vec<super::SelectedTool>,
        preferences: &super::UserPreferences,
        boost_factor: f64,
    ) -> Vec<super::SelectedTool> {
        for tool in &mut tools {
            if preferences.preferred_tools.contains(&tool.name) {
                tool.ranking_score *= boost_factor;
                tool.selection_reason = format!(
                    "{} (preferred tool boost: {:.1}x)",
                    tool.selection_reason, boost_factor
                );
            }
        }

        // Re-sort after boosting
        tools.sort_by(|a, b| b.ranking_score.partial_cmp(&a.ranking_score).expect("Operation failed - converted from unwrap()"));

        tools
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::SemanticVersion;
    use crate::{SystemContext, ToolSelectionContext};

    #[tokio::test]
    async fn test_embedding_selection_strategy() {
        let strategy = EmbeddingSelectionStrategy::new(0.1, 5);

        let spec = ToolSpec {
            name: "file_reader".to_string(),
            description: "Read files from filesystem".to_string(),
            usage: "file_reader --path <path>".to_string(),
            examples: vec![],
            input_schema: "{}".to_string(),
            usage_guide: None,
            permissions: None,
            supports_dry_run: true,
        };

        let metadata = ToolMetadata::new(
            "file_reader".to_string(),
            "File Reader".to_string(),
            SemanticVersion::new(1, 0, 0),
        );

        let tools = vec![("file_reader".to_string(), metadata, spec)];
        let context = ToolSelectionContext {
            query: "read a file".to_string(),
            project_context: None,
            system_context: SystemContext {
                platform: "linux".to_string(),
                has_network: false,
                has_gpu: false,
                available_memory_mb: 1024,
                max_execution_time_secs: Some(30),
            },
            user_preferences: None,
        };

        let selected = strategy
            .select_tools("read file", &tools, &context)
            .await
            .expect("Operation failed - converted from unwrap()");

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].name, "file_reader");
        assert!(selected[0].similarity_score > 0.0);
    }

    #[test]
    fn test_tool_filter_by_security_level() {
        let spec = ToolSpec {
            name: "test_tool".to_string(),
            description: "Test".to_string(),
            usage: "test".to_string(),
            examples: vec![],
            input_schema: "{}".to_string(),
            usage_guide: None,
            permissions: None,
            supports_dry_run: true,
        };

        let high_risk_metadata = ToolMetadata::new(
            "high_risk".to_string(),
            "High Risk Tool".to_string(),
            SemanticVersion::new(1, 0, 0),
        )
        .with_permissions(crate::registry::ToolPermissions {
            file_system: crate::registry::FileSystemPermissions::FullAccess,
            network: crate::registry::NetworkPermissions::Internet,
            system: crate::registry::SystemPermissions::FullAccess,
            custom: HashMap::new(),
        });

        let safe_metadata = ToolMetadata::new(
            "safe".to_string(),
            "Safe Tool".to_string(),
            SemanticVersion::new(1, 0, 0),
        );

        let tools = vec![
            ("high_risk".to_string(), high_risk_metadata, spec.clone()),
            ("safe".to_string(), safe_metadata, spec),
        ];

        let filtered = ToolFilter::by_security_level(tools, SecurityLevel::MediumRisk);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "safe");
    }

    #[tokio::test]
    async fn test_keyword_selection_strategy() {
        let strategy = KeywordSelectionStrategy::new(10)
            .with_keyword_weight("file".to_string(), 1.0)
            .with_keyword_weight("read".to_string(), 0.8);

        let spec = ToolSpec {
            name: "file_reader".to_string(),
            description: "Read files from disk".to_string(),
            usage: "file_reader --path <path>".to_string(),
            examples: vec![],
            input_schema: "{}".to_string(),
            usage_guide: None,
            permissions: None,
            supports_dry_run: true,
        };

        let metadata = ToolMetadata::new(
            "file_reader".to_string(),
            "File Reader".to_string(),
            SemanticVersion::new(1, 0, 0),
        );

        let tools = vec![("file_reader".to_string(), metadata, spec)];
        let context = ToolSelectionContext {
            query: "read file content".to_string(),
            project_context: None,
            system_context: SystemContext {
                platform: "linux".to_string(),
                has_network: false,
                has_gpu: false,
                available_memory_mb: 1024,
                max_execution_time_secs: Some(30),
            },
            user_preferences: None,
        };

        let selected = strategy
            .select_tools("read file content", &tools, &context)
            .await
            .expect("Operation failed - converted from unwrap()");

        assert_eq!(selected.len(), 1);
        assert!(selected[0].similarity_score > 0.0);
    }
}
