// @component: {"k":"C","id":"tool_metadata_extractor","t":"Tool metadata extraction and semantic analysis","m":{"cur":0,"tgt":100,"u":"%"},"f":["metadata","extraction","semantics","patterns"]}

use super::builder::ToolSelectionRequest;
use super::{Result, ToolContextError};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, instrument};

/// Tool metadata extractor for semantic analysis
pub struct ToolMetadataExtractor {
    // Future: Could include embedding models for semantic analysis
}

/// Extracted metadata from tools and context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedMetadata {
    /// Tool semantic information
    pub semantics: ToolSemantics,

    /// Tool capabilities analysis
    pub capabilities: ToolCapabilities,

    /// Usage patterns detected
    pub usage_patterns: Vec<ToolUsagePattern>,

    /// Contextual information
    pub contextual: ContextualMetadata,
}

/// Semantic information about tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSemantics {
    /// Primary domain/category
    pub primary_domain: String,

    /// Secondary domains
    pub secondary_domains: Vec<String>,

    /// Key concepts/topics
    pub key_concepts: Vec<String>,

    /// Intent categories
    pub intent_categories: Vec<String>,

    /// Semantic embeddings (future)
    pub embeddings: Option<Vec<f32>>,
}

/// Tool capabilities analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCapabilities {
    /// Input types the tool can handle
    pub input_types: Vec<String>,

    /// Output types the tool produces
    pub output_types: Vec<String>,

    /// Operations the tool can perform
    pub operations: Vec<String>,

    /// Platform requirements
    pub platform_requirements: Vec<String>,

    /// Performance characteristics
    pub performance_class: PerformanceClass,
}

/// Performance classification for tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PerformanceClass {
    /// Very fast tools (< 1 second)
    Instant,

    /// Fast tools (1-5 seconds)
    Fast,

    /// Medium speed tools (5-30 seconds)
    Medium,

    /// Slow tools (30+ seconds)
    Slow,

    /// Variable performance
    Variable,
}

/// Tool usage patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsagePattern {
    /// Pattern name/description
    pub pattern_name: String,

    /// Context where this pattern applies
    pub context: String,

    /// Frequency of this pattern
    pub frequency: f32,

    /// Success rate for this pattern
    pub success_rate: f32,

    /// Common parameters for this pattern
    pub common_parameters: HashMap<String, String>,
}

/// Contextual metadata extracted from request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextualMetadata {
    /// Project type detected
    pub project_type: Option<String>,

    /// Programming languages detected
    pub languages: Vec<String>,

    /// File types in context
    pub file_types: Vec<String>,

    /// Current working directory context
    pub working_directory: Option<String>,

    /// Git repository status
    pub git_context: Option<GitContext>,

    /// Suggested tool categories
    pub suggested_categories: Vec<String>,

    /// Intent classification
    pub intent_classification: IntentClassification,

    /// Context confidence score
    pub confidence_score: f32,
}

/// Git repository context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitContext {
    /// Is this a git repository
    pub is_git_repo: bool,

    /// Current branch
    pub current_branch: Option<String>,

    /// Uncommitted changes
    pub has_uncommitted_changes: bool,

    /// Repository status
    pub status: String,
}

/// Intent classification from user query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentClassification {
    /// Primary intent (create, read, update, delete, analyze, etc.)
    pub primary_intent: String,

    /// Intent confidence
    pub confidence: f32,

    /// Sub-intents or secondary actions
    pub sub_intents: Vec<String>,

    /// Urgency level
    pub urgency: UrgencyLevel,

    /// Complexity level
    pub complexity: ComplexityLevel,
}

/// Urgency classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UrgencyLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Complexity classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComplexityLevel {
    Simple,
    Medium,
    Complex,
    Expert,
}

impl ToolMetadataExtractor {
    /// Create a new metadata extractor
    pub fn new() -> Self {
        Self {}
    }

    /// Extract contextual metadata from a tool selection request
    #[instrument(skip(self))]
    pub async fn extract_contextual_metadata(
        &self,
        request: &ToolSelectionRequest,
    ) -> anyhow::Result<ContextualMetadata> {
        debug!("Extracting contextual metadata from request");

        let project_type = self.detect_project_type(&request.context).await?;
        let languages = self.detect_languages(&request.context).await?;
        let file_types = self.detect_file_types(&request.context).await?;
        let git_context = self.extract_git_context(&request.context).await?;
        let intent_classification = self.classify_intent(&request.query).await?;
        let suggested_categories = self
            .suggest_categories(&request.query, &project_type)
            .await?;

        let confidence_score =
            self.calculate_confidence_score(&project_type, &languages, &intent_classification);

        Ok(ContextualMetadata {
            project_type,
            languages,
            file_types,
            working_directory: request.context.get("working_directory").cloned(),
            git_context,
            suggested_categories,
            intent_classification,
            confidence_score,
        })
    }

    /// Extract tool semantics
    pub async fn extract_tool_semantics(
        &self,
        tool_description: &str,
        tool_category: &str,
    ) -> anyhow::Result<ToolSemantics> {
        let primary_domain = self.classify_primary_domain(tool_description, tool_category);
        let secondary_domains = self.extract_secondary_domains(tool_description);
        let key_concepts = self.extract_key_concepts(tool_description);
        let intent_categories = self.classify_intent_categories(tool_description);

        Ok(ToolSemantics {
            primary_domain,
            secondary_domains,
            key_concepts,
            intent_categories,
            embeddings: None, // TODO: Generate embeddings
        })
    }

    /// Detect project type from context
    async fn detect_project_type(
        &self,
        context: &HashMap<String, String>,
    ) -> anyhow::Result<Option<String>> {
        if let Some(working_dir) = context.get("working_directory") {
            let path = Path::new(working_dir);

            // Check for common project files
            if path.join("Cargo.toml").exists() {
                return Ok(Some("rust".to_string()));
            }
            if path.join("package.json").exists() {
                return Ok(Some("javascript".to_string()));
            }
            if path.join("requirements.txt").exists() || path.join("pyproject.toml").exists() {
                return Ok(Some("python".to_string()));
            }
            if path.join("go.mod").exists() {
                return Ok(Some("go".to_string()));
            }
            if path.join("pom.xml").exists() || path.join("build.gradle").exists() {
                return Ok(Some("java".to_string()));
            }
        }

        Ok(None)
    }

    /// Detect programming languages from context
    async fn detect_languages(
        &self,
        context: &HashMap<String, String>,
    ) -> anyhow::Result<Vec<String>> {
        let mut languages = Vec::new();

        if let Some(files) = context.get("files") {
            // Simple extension-based detection
            if files.contains(".rs") {
                languages.push("rust".to_string());
            }
            if files.contains(".js") || files.contains(".ts") {
                languages.push("javascript".to_string());
            }
            if files.contains(".py") {
                languages.push("python".to_string());
            }
            if files.contains(".go") {
                languages.push("go".to_string());
            }
            if files.contains(".java") {
                languages.push("java".to_string());
            }
            if files.contains(".cpp") || files.contains(".cc") {
                languages.push("cpp".to_string());
            }
            if files.contains(".c") {
                languages.push("c".to_string());
            }
        }

        Ok(languages)
    }

    /// Detect file types from context
    async fn detect_file_types(
        &self,
        context: &HashMap<String, String>,
    ) -> anyhow::Result<Vec<String>> {
        let mut file_types = Vec::new();

        if let Some(files) = context.get("files") {
            // Extract unique file extensions
            let extensions: Vec<&str> = files
                .split_whitespace()
                .filter_map(|f| Path::new(f).extension())
                .filter_map(|ext| ext.to_str())
                .collect();

            for ext in extensions {
                if !file_types.contains(&ext.to_string()) {
                    file_types.push(ext.to_string());
                }
            }
        }

        Ok(file_types)
    }

    /// Extract git context if available
    async fn extract_git_context(
        &self,
        context: &HashMap<String, String>,
    ) -> anyhow::Result<Option<GitContext>> {
        if let Some(working_dir) = context.get("working_directory") {
            let git_dir = Path::new(working_dir).join(".git");

            if git_dir.exists() {
                // Basic git context extraction
                // In a real implementation, this would use git2 or similar
                Ok(Some(GitContext {
                    is_git_repo: true,
                    current_branch: context.get("git_branch").cloned(),
                    has_uncommitted_changes: context
                        .get("git_status")
                        .is_some_and(|s| s.contains("modified")),
                    status: context.get("git_status").cloned().unwrap_or_default(),
                }))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Classify user intent from query
    async fn classify_intent(&self, query: &str) -> anyhow::Result<IntentClassification> {
        let query_lower = query.to_lowercase();

        // Simple keyword-based intent classification
        let primary_intent = if query_lower.contains("create")
            || query_lower.contains("make")
            || query_lower.contains("new")
        {
            "create"
        } else if query_lower.contains("read")
            || query_lower.contains("show")
            || query_lower.contains("display")
            || query_lower.contains("view")
        {
            "read"
        } else if query_lower.contains("update")
            || query_lower.contains("modify")
            || query_lower.contains("change")
            || query_lower.contains("edit")
        {
            "update"
        } else if query_lower.contains("delete")
            || query_lower.contains("remove")
            || query_lower.contains("clean")
        {
            "delete"
        } else if query_lower.contains("analyze")
            || query_lower.contains("check")
            || query_lower.contains("test")
            || query_lower.contains("validate")
        {
            "analyze"
        } else if query_lower.contains("search")
            || query_lower.contains("find")
            || query_lower.contains("grep")
        {
            "search"
        } else if query_lower.contains("deploy")
            || query_lower.contains("build")
            || query_lower.contains("compile")
        {
            "build"
        } else {
            "general"
        };

        let urgency = if query_lower.contains("urgent")
            || query_lower.contains("critical")
            || query_lower.contains("immediately")
        {
            UrgencyLevel::Critical
        } else if query_lower.contains("quick")
            || query_lower.contains("fast")
            || query_lower.contains("asap")
        {
            UrgencyLevel::High
        } else if query_lower.contains("when possible") || query_lower.contains("later") {
            UrgencyLevel::Low
        } else {
            UrgencyLevel::Medium
        };

        let complexity = if query_lower.contains("simple")
            || query_lower.contains("basic")
            || query_lower.contains("quick")
        {
            ComplexityLevel::Simple
        } else if query_lower.contains("complex")
            || query_lower.contains("advanced")
            || query_lower.contains("expert")
        {
            ComplexityLevel::Expert
        } else if query_lower.contains("detailed") || query_lower.contains("thorough") {
            ComplexityLevel::Complex
        } else {
            ComplexityLevel::Medium
        };

        // Calculate confidence based on keyword matches
        let keyword_matches = [
            primary_intent != "general",
            query_lower.len() > 10,
            query_lower.split_whitespace().count() > 2,
        ]
        .iter()
        .filter(|&&x| x)
        .count();

        let confidence = (keyword_matches as f32 / 3.0).clamp(0.3, 1.0);

        Ok(IntentClassification {
            primary_intent: primary_intent.to_string(),
            confidence,
            sub_intents: Vec::new(), // TODO: Extract sub-intents
            urgency,
            complexity,
        })
    }

    /// Suggest tool categories based on query and context
    async fn suggest_categories(
        &self,
        query: &str,
        project_type: &Option<String>,
    ) -> anyhow::Result<Vec<String>> {
        let mut categories = Vec::new();
        let query_lower = query.to_lowercase();

        // Query-based suggestions
        if query_lower.contains("file")
            || query_lower.contains("directory")
            || query_lower.contains("folder")
        {
            categories.push("FileSystem".to_string());
        }
        if query_lower.contains("git")
            || query_lower.contains("commit")
            || query_lower.contains("branch")
        {
            categories.push("Git".to_string());
        }
        if query_lower.contains("web")
            || query_lower.contains("http")
            || query_lower.contains("api")
        {
            categories.push("Web".to_string());
        }
        if query_lower.contains("database")
            || query_lower.contains("sql")
            || query_lower.contains("query")
        {
            categories.push("Database".to_string());
        }
        if query_lower.contains("system")
            || query_lower.contains("process")
            || query_lower.contains("service")
        {
            categories.push("System".to_string());
        }
        if query_lower.contains("security")
            || query_lower.contains("auth")
            || query_lower.contains("permission")
        {
            categories.push("Security".to_string());
        }

        // Project-type based suggestions
        if let Some(ref proj_type) = project_type {
            match proj_type.as_str() {
                "rust" | "javascript" | "python" | "go" | "java" => {
                    categories.push("Development".to_string());
                }
                _ => {}
            }
        }

        // Default category if nothing else matches
        if categories.is_empty() {
            categories.push("Analysis".to_string());
        }

        Ok(categories)
    }

    /// Calculate confidence score for metadata extraction
    fn calculate_confidence_score(
        &self,
        project_type: &Option<String>,
        languages: &[String],
        intent: &IntentClassification,
    ) -> f32 {
        let mut score = 0.0;

        // Project type detection adds confidence
        if project_type.is_some() {
            score += 0.3;
        }

        // Language detection adds confidence
        if !languages.is_empty() {
            score += 0.2;
        }

        // Intent classification confidence
        score += intent.confidence * 0.5;

        score.clamp(0.0, 1.0)
    }

    /// Classify primary domain for a tool
    fn classify_primary_domain(&self, description: &str, category: &str) -> String {
        if category.contains("FileSystem") {
            "file_management".to_string()
        } else if category.contains("Git") {
            "version_control".to_string()
        } else if category.contains("Web") {
            "networking".to_string()
        } else if category.contains("Database") {
            "data_management".to_string()
        } else if category.contains("System") {
            "system_administration".to_string()
        } else if category.contains("Development") {
            "software_development".to_string()
        } else if category.contains("Security") {
            "security".to_string()
        } else if category.contains("Analysis") {
            "analysis".to_string()
        } else {
            "general".to_string()
        }
    }

    /// Extract secondary domains from description
    fn extract_secondary_domains(&self, description: &str) -> Vec<String> {
        let mut domains = Vec::new();
        let desc_lower = description.to_lowercase();

        if desc_lower.contains("automation") {
            domains.push("automation".to_string());
        }
        if desc_lower.contains("monitoring") {
            domains.push("monitoring".to_string());
        }
        if desc_lower.contains("deployment") {
            domains.push("deployment".to_string());
        }
        if desc_lower.contains("testing") {
            domains.push("testing".to_string());
        }

        domains
    }

    /// Extract key concepts from description
    fn extract_key_concepts(&self, description: &str) -> Vec<String> {
        // Simple keyword extraction
        // In a real implementation, this would use NLP techniques
        description
            .split_whitespace()
            .filter(|word| word.len() > 4)
            .map(|word| word.to_lowercase())
            .collect()
    }

    /// Classify intent categories for a tool
    fn classify_intent_categories(&self, description: &str) -> Vec<String> {
        let mut categories = Vec::new();
        let desc_lower = description.to_lowercase();

        if desc_lower.contains("create") || desc_lower.contains("generate") {
            categories.push("creation".to_string());
        }
        if desc_lower.contains("read")
            || desc_lower.contains("view")
            || desc_lower.contains("display")
        {
            categories.push("reading".to_string());
        }
        if desc_lower.contains("update")
            || desc_lower.contains("modify")
            || desc_lower.contains("edit")
        {
            categories.push("modification".to_string());
        }
        if desc_lower.contains("delete") || desc_lower.contains("remove") {
            categories.push("deletion".to_string());
        }
        if desc_lower.contains("analyze") || desc_lower.contains("check") {
            categories.push("analysis".to_string());
        }

        categories
    }
}

impl Default for ToolMetadataExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_intent_classification() {
        let extractor = ToolMetadataExtractor::new();

        let intent = extractor
            .classify_intent("create a new file")
            .await
            .expect("Operation failed - converted from unwrap()");
        assert_eq!(intent.primary_intent, "create");
        assert!(intent.confidence > 0.5);

        let intent = extractor
            .classify_intent("show git status")
            .await
            .expect("Async operation should succeed");
        assert_eq!(intent.primary_intent, "read");
    }

    #[tokio::test]
    async fn test_category_suggestion() {
        let extractor = ToolMetadataExtractor::new();

        let categories = extractor
            .suggest_categories("git commit", &Some("rust".to_string()))
            .await
            .expect("Operation failed - converted from unwrap()");
        assert!(categories.contains(&"Git".to_string()));
        assert!(categories.contains(&"Development".to_string()));
    }

    #[test]
    fn test_domain_classification() {
        let extractor = ToolMetadataExtractor::new();

        let domain = extractor.classify_primary_domain("Git repository tool", "Git");
        assert_eq!(domain, "version_control");

        let domain = extractor.classify_primary_domain("File system utility", "FileSystem");
        assert_eq!(domain, "file_management");
    }
}
