// Usage guide generation and caching for effective tool descriptions

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::sync::RwLock;

use crate::registry::{SecurityLevel, ToolCategory, ToolMetadata};
use crate::{ToolSpec, UsageGuide};

/// Usage guide generator and cache manager
#[derive(Debug)]
pub struct UsageGuideManager {
    /// In-memory cache for quick access
    cache: RwLock<HashMap<String, CachedUsageGuide>>,

    /// Generator configuration
    config: UsageGuideConfig,

    /// SQLite cache path (future implementation)
    cache_path: Option<String>,
}

/// Cached usage guide with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedUsageGuide {
    pub guide: UsageGuide,
    pub embedding: Option<Vec<f32>>,
    pub generated_at: u64,
    pub usage_count: u64,
    pub last_used: u64,
    pub telemetry_data: TelemetryData,
}

/// Usage guide generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageGuideConfig {
    /// Enable LLM-based guide enhancement
    pub enable_llm_enhancement: bool,

    /// Include performance metrics in guides
    pub include_performance_metrics: bool,

    /// Update guides based on telemetry
    pub enable_telemetry_updates: bool,

    /// Cache expiration time in seconds
    pub cache_expiration_secs: u64,

    /// Maximum cache size
    pub max_cache_entries: usize,

    /// Auto-generate missing guides
    pub auto_generate: bool,
}

impl Default for UsageGuideConfig {
    fn default() -> Self {
        Self {
            enable_llm_enhancement: false, // Disabled by default for privacy
            include_performance_metrics: true,
            enable_telemetry_updates: true,
            cache_expiration_secs: 86400, // 24 hours
            max_cache_entries: 1000,
            auto_generate: true,
        }
    }
}

/// Telemetry data for improving usage guides
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TelemetryData {
    /// Success/failure rates
    pub success_rate: f64,

    /// Common error patterns
    pub common_errors: Vec<String>,

    /// Typical usage patterns
    pub usage_patterns: Vec<String>,

    /// Performance observations
    pub avg_execution_time_ms: u64,

    /// User feedback (warnings, improvements)
    pub user_warnings: Vec<String>,

    /// Platform-specific issues
    pub platform_issues: HashMap<String, Vec<String>>,
}

impl UsageGuideManager {
    /// Create new usage guide manager
    pub fn new(config: UsageGuideConfig) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            config,
            cache_path: None,
        }
    }

    /// Create with SQLite cache backing (future implementation)
    pub fn with_sqlite_cache<P: AsRef<Path>>(config: UsageGuideConfig, cache_path: P) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            config,
            cache_path: Some(cache_path.as_ref().to_string_lossy().to_string()),
        }
    }

    /// Generate or retrieve usage guide for a tool
    pub async fn get_usage_guide(
        &self,
        tool_name: &str,
        spec: &ToolSpec,
        metadata: &ToolMetadata,
    ) -> Result<UsageGuide> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(tool_name) {
                if !self.is_cache_expired(cached) {
                    let guide = cached.guide.clone();
                    // Drop cache lock before async operation
                    drop(cache);
                    // Update usage statistics
                    self.record_usage(tool_name).await?;
                    return Ok(guide);
                }
            }
        }

        // Generate new guide
        let guide = self
            .generate_usage_guide(spec, metadata)
            .await
            .context("Failed to generate usage guide")?;

        // Cache the generated guide
        self.cache_guide(tool_name.to_string(), guide.clone())
            .await?;

        Ok(guide)
    }

    /// Generate effective usage guide from tool spec and metadata
    pub async fn generate_usage_guide(
        &self,
        spec: &ToolSpec,
        metadata: &ToolMetadata,
    ) -> Result<UsageGuide> {
        // Start with existing guide or generate basic one
        let mut guide = spec
            .usage_guide
            .clone()
            .unwrap_or_else(|| crate::generate_usage_guide(spec));

        // Enhance with metadata information
        self.enhance_with_metadata(&mut guide, metadata).await?;

        // Add security and risk information
        self.add_security_info(&mut guide, metadata).await?;

        // Add platform compatibility
        self.add_platform_info(&mut guide, metadata, spec).await?;

        // Add performance information if enabled
        if self.config.include_performance_metrics {
            self.add_performance_info(&mut guide, metadata).await?;
        }

        // Enhance with LLM if enabled (placeholder)
        if self.config.enable_llm_enhancement {
            // Future: LLM-based enhancement
            // guide = self.enhance_with_llm(&guide).await?;
        }

        Ok(guide)
    }

    /// Enhance guide with metadata information
    async fn enhance_with_metadata(
        &self,
        guide: &mut UsageGuide,
        metadata: &ToolMetadata,
    ) -> Result<()> {
        // Update title and summary with metadata
        if guide.usage_title.is_empty() {
            guide.usage_title = metadata.name.clone();
        }

        if guide.usage_summary.is_empty() {
            guide.usage_summary = metadata.description.clone();
        }

        // Add category-specific information
        match metadata.category {
            ToolCategory::FileSystem => {
                if !guide.good_for.contains(&"file operations".to_string()) {
                    guide.good_for.push("file operations".to_string());
                }
            }
            ToolCategory::Git => {
                if !guide.good_for.contains(&"version control".to_string()) {
                    guide.good_for.push("version control".to_string());
                }
            }
            ToolCategory::Web => {
                if !guide.good_for.contains(&"web requests".to_string()) {
                    guide.good_for.push("web requests".to_string());
                }
            }
            ToolCategory::System => {
                if !guide.good_for.contains(&"system operations".to_string()) {
                    guide.good_for.push("system operations".to_string());
                }
            }
            _ => {}
        }

        // Add dependency information
        if !metadata.dependencies.is_empty() {
            let deps: Vec<String> = metadata
                .dependencies
                .iter()
                .map(|d| d.tool_id.clone())
                .collect();
            guide
                .preconditions
                .push(format!("requires dependencies: {}", deps.join(", ")));
        }

        Ok(())
    }

    /// Add security and risk information
    async fn add_security_info(
        &self,
        guide: &mut UsageGuide,
        metadata: &ToolMetadata,
    ) -> Result<()> {
        // Set risk score
        guide.risk_score = match metadata.security_level {
            SecurityLevel::Safe => 1,
            SecurityLevel::LowRisk => 2,
            SecurityLevel::MediumRisk => 4,
            SecurityLevel::HighRisk => 7,
            SecurityLevel::Critical => 9,
        };

        // Add security constraints
        match metadata.security_level {
            SecurityLevel::HighRisk | SecurityLevel::Critical => {
                guide
                    .constraints
                    .push("requires user confirmation".to_string());
                if metadata.security_level == SecurityLevel::Critical {
                    guide
                        .constraints
                        .push("admin privileges required".to_string());
                }
            }
            _ => {}
        }

        // Add permission-based constraints
        match &metadata.permissions.file_system {
            crate::registry::FileSystemPermissions::ReadWrite => {
                guide.side_effects.push("modifies files".to_string());
            }
            crate::registry::FileSystemPermissions::FullAccess => {
                guide
                    .side_effects
                    .push("full file system access".to_string());
                guide
                    .constraints
                    .push("file system access required".to_string());
            }
            _ => {}
        }

        match &metadata.permissions.network {
            crate::registry::NetworkPermissions::Internet => {
                guide
                    .constraints
                    .push("internet access required".to_string());
                guide
                    .side_effects
                    .push("makes network requests".to_string());
            }
            crate::registry::NetworkPermissions::Restricted { allowed_hosts } => {
                guide
                    .constraints
                    .push(format!("network access to: {}", allowed_hosts.join(", ")));
            }
            _ => {}
        }

        Ok(())
    }

    /// Add platform compatibility information
    async fn add_platform_info(
        &self,
        guide: &mut UsageGuide,
        _metadata: &ToolMetadata,
        _spec: &ToolSpec,
    ) -> Result<()> {
        // Default to common platforms if not specified
        if guide.platforms.is_empty() {
            guide.platforms = vec!["linux".to_string(), "mac".to_string(), "win".to_string()];
        }

        Ok(())
    }

    /// Add performance information
    async fn add_performance_info(
        &self,
        guide: &mut UsageGuide,
        metadata: &ToolMetadata,
    ) -> Result<()> {
        // Set cost class based on resource requirements
        guide.cost_class = if metadata.resource_requirements.requires_gpu {
            "expensive".to_string()
        } else if let Some(memory) = metadata.resource_requirements.max_memory_mb {
            if memory > 1024 {
                "moderate".to_string()
            } else {
                "free".to_string()
            }
        } else {
            "free".to_string()
        };

        // Set latency class based on performance metrics
        guide.latency_class = if metadata
            .performance_metrics
            .average_execution_time
            .as_secs()
            > 10
        {
            "slow".to_string()
        } else if metadata
            .performance_metrics
            .average_execution_time
            .as_millis()
            > 1000
        {
            "moderate".to_string()
        } else {
            "fast".to_string()
        };

        // Add resource constraints
        if let Some(memory) = metadata.resource_requirements.max_memory_mb {
            if memory > 512 {
                guide
                    .constraints
                    .push(format!("requires {memory}MB+ memory"));
            }
        }

        if metadata.resource_requirements.requires_gpu {
            guide.constraints.push("GPU required".to_string());
        }

        if metadata.resource_requirements.requires_network {
            guide
                .constraints
                .push("network access required".to_string());
        }

        Ok(())
    }

    /// Update guide with telemetry data
    pub async fn update_with_telemetry(
        &self,
        tool_name: &str,
        telemetry: TelemetryData,
    ) -> Result<()> {
        if !self.config.enable_telemetry_updates {
            return Ok(());
        }

        let mut cache = self.cache.write().await;
        if let Some(cached) = cache.get_mut(tool_name) {
            cached.telemetry_data = telemetry.clone();

            // Update guide based on telemetry
            self.apply_telemetry_updates(&mut cached.guide, &telemetry)
                .await?;

            cached.generated_at = self.current_timestamp();
        }

        Ok(())
    }

    /// Apply telemetry data to improve usage guide
    async fn apply_telemetry_updates(
        &self,
        guide: &mut UsageGuide,
        telemetry: &TelemetryData,
    ) -> Result<()> {
        // Add warnings based on common errors
        for error in &telemetry.common_errors {
            let warning = format!("common issue: {error}");
            if !guide.constraints.contains(&warning) {
                guide.constraints.push(warning);
            }
        }

        // Add user warnings
        for warning in &telemetry.user_warnings {
            if !guide.not_for.contains(warning) {
                guide.not_for.push(warning.clone());
            }
        }

        // Update performance class based on observed metrics
        if telemetry.avg_execution_time_ms > 10000 {
            guide.latency_class = "slow".to_string();
        } else if telemetry.avg_execution_time_ms > 1000 {
            guide.latency_class = "moderate".to_string();
        } else {
            guide.latency_class = "fast".to_string();
        }

        // Add platform-specific warnings
        for (platform, issues) in &telemetry.platform_issues {
            for issue in issues {
                let warning = format!("on {platform}: {issue}");
                if !guide.constraints.contains(&warning) {
                    guide.constraints.push(warning);
                }
            }
        }

        Ok(())
    }

    /// Cache a usage guide
    async fn cache_guide(&self, tool_name: String, guide: UsageGuide) -> Result<()> {
        let mut cache = self.cache.write().await;

        // Enforce cache size limit
        if cache.len() >= self.config.max_cache_entries {
            self.evict_oldest_entries(&mut cache).await?;
        }

        let cached_guide = CachedUsageGuide {
            guide,
            embedding: None, // Will be filled by embedding system
            generated_at: self.current_timestamp(),
            usage_count: 1,
            last_used: self.current_timestamp(),
            telemetry_data: TelemetryData::default(),
        };

        cache.insert(tool_name, cached_guide);

        Ok(())
    }

    /// Record usage of a cached guide
    async fn record_usage(&self, tool_name: &str) -> Result<()> {
        let mut cache = self.cache.write().await;
        if let Some(cached) = cache.get_mut(tool_name) {
            cached.usage_count += 1;
            cached.last_used = self.current_timestamp();
        }
        Ok(())
    }

    /// Check if cached guide is expired
    fn is_cache_expired(&self, cached: &CachedUsageGuide) -> bool {
        let current_time = self.current_timestamp();
        current_time - cached.generated_at > self.config.cache_expiration_secs
    }

    /// Evict oldest cache entries
    async fn evict_oldest_entries(
        &self,
        cache: &mut HashMap<String, CachedUsageGuide>,
    ) -> Result<()> {
        if cache.is_empty() {
            return Ok(());
        }

        // Find entries to remove (remove 25% of cache)
        let remove_count = (cache.len() / 4).max(1);

        let mut entries: Vec<_> = cache
            .iter()
            .map(|(name, cached)| (name.clone(), cached.last_used))
            .collect();

        // Sort by last used time (oldest first)
        entries.sort_by(|a, b| a.1.cmp(&b.1));

        // Remove oldest entries
        for (name, _) in entries.into_iter().take(remove_count) {
            cache.remove(&name);
        }

        Ok(())
    }

    /// Get current timestamp
    fn current_timestamp(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> CacheStats {
        let cache = self.cache.read().await;

        let total_usage = cache.values().map(|c| c.usage_count).sum();
        let avg_usage = if cache.is_empty() {
            0.0
        } else {
            total_usage as f64 / cache.len() as f64
        };

        let current_time = self.current_timestamp();
        let expired_count = cache
            .values()
            .filter(|c| current_time - c.generated_at > self.config.cache_expiration_secs)
            .count();

        CacheStats {
            total_entries: cache.len(),
            expired_entries: expired_count,
            total_usage_count: total_usage,
            average_usage_per_entry: avg_usage,
            cache_hit_rate: 0.0, // Would be calculated over time
            memory_usage_estimate: cache.len() * 1024, // Rough estimate
        }
    }

    /// Clear expired entries
    pub async fn cleanup_expired(&self) -> Result<usize> {
        let mut cache = self.cache.write().await;
        let current_time = self.current_timestamp();

        let initial_size = cache.len();

        cache.retain(|_, cached| {
            current_time - cached.generated_at <= self.config.cache_expiration_secs
        });

        Ok(initial_size - cache.len())
    }

    /// Get all cached guides
    pub async fn get_all_guides(&self) -> HashMap<String, UsageGuide> {
        let cache = self.cache.read().await;
        cache
            .iter()
            .map(|(name, cached)| (name.clone(), cached.guide.clone()))
            .collect()
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub total_usage_count: u64,
    pub average_usage_per_entry: f64,
    pub cache_hit_rate: f64,
    pub memory_usage_estimate: usize,
}

/// Usage guide optimization utilities
pub struct GuideOptimizer;

impl GuideOptimizer {
    /// Optimize guide for LLM context (reduce token usage)
    pub fn optimize_for_llm(guide: &UsageGuide) -> CompactGuide {
        CompactGuide {
            title: guide.usage_title.clone(),
            summary: Self::truncate_text(&guide.usage_summary, 100),
            example: guide.examples.first().cloned().unwrap_or_default(),
            constraints: guide
                .constraints
                .iter()
                .take(3)
                .map(|c| Self::truncate_text(c, 50))
                .collect(),
            risk_level: match guide.risk_score {
                1..=2 => "safe".to_string(),
                3..=5 => "medium".to_string(),
                6..=10 => "high".to_string(),
                _ => "unknown".to_string(),
            },
        }
    }

    /// Generate compact context string for multiple tools
    pub fn generate_context_string(guides: &[CompactGuide]) -> String {
        let mut context = String::new();

        for (i, guide) in guides.iter().enumerate() {
            context.push_str(&format!(
                "{}. {}: {} Example: {} Risk: {}\n",
                i + 1,
                guide.title,
                guide.summary,
                guide.example,
                guide.risk_level
            ));
        }

        context
    }

    fn truncate_text(text: &str, max_length: usize) -> String {
        if text.len() <= max_length {
            text.to_string()
        } else {
            format!("{}...", &text[..max_length.saturating_sub(3)])
        }
    }
}

/// Compact guide for LLM context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactGuide {
    pub title: String,
    pub summary: String,
    pub example: String,
    pub constraints: Vec<String>,
    pub risk_level: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::SemanticVersion;
    use std::time::Duration;

    #[tokio::test]
    async fn test_usage_guide_generation() {
        let manager = UsageGuideManager::new(UsageGuideConfig::default());

        let spec = ToolSpec {
            name: "file_reader".to_string(),
            description: "Read files from disk".to_string(),
            usage: "file_reader --path <path>".to_string(),
            examples: vec!["file_reader --path /tmp/file.txt".to_string()],
            input_schema: "{}".to_string(),
            usage_guide: None,
            permissions: None,
            supports_dry_run: true,
        };

        let metadata = ToolMetadata::new(
            "file_reader".to_string(),
            "File Reader".to_string(),
            SemanticVersion::new(1, 0, 0),
        )
        .with_category(ToolCategory::FileSystem);

        let guide = manager
            .generate_usage_guide(&spec, &metadata)
            .await
            .expect("Operation failed - converted from unwrap()");

        assert_eq!(guide.usage_title, "file_reader");
        assert_eq!(guide.usage_summary, "Read files from disk");
        assert!(guide.good_for.contains(&"file operations".to_string()));
        assert_eq!(guide.risk_score, 1); // Safe by default
    }

    #[tokio::test]
    async fn test_usage_guide_caching() {
        let manager = UsageGuideManager::new(UsageGuideConfig::default());

        let spec = ToolSpec {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            usage: "test_tool --help".to_string(),
            examples: vec![],
            input_schema: "{}".to_string(),
            usage_guide: None,
            permissions: None,
            supports_dry_run: true,
        };

        let metadata = ToolMetadata::new(
            "test_tool".to_string(),
            "Test Tool".to_string(),
            SemanticVersion::new(1, 0, 0),
        );

        // First call should generate and cache
        let guide1 = manager
            .get_usage_guide("test_tool", &spec, &metadata)
            .await
            .expect("Operation failed - converted from unwrap()");

        // Second call should use cache
        let guide2 = manager
            .get_usage_guide("test_tool", &spec, &metadata)
            .await
            .expect("Operation failed - converted from unwrap()");

        assert_eq!(guide1.usage_title, guide2.usage_title);

        let stats = manager.get_cache_stats().await;
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.total_usage_count, 2);
    }

    #[tokio::test]
    async fn test_telemetry_updates() {
        let manager = UsageGuideManager::new(UsageGuideConfig::default());

        // Cache an initial guide
        let guide = UsageGuide {
            usage_title: "Test Tool".to_string(),
            usage_summary: "A test tool".to_string(),
            preconditions: vec![],
            arguments_brief: HashMap::new(),
            good_for: vec![],
            not_for: vec![],
            constraints: vec![],
            examples: vec![],
            platforms: vec!["linux".to_string()],
            cost_class: "free".to_string(),
            latency_class: "fast".to_string(),
            side_effects: vec![],
            risk_score: 1,
            capabilities: vec![],
            tags: vec![],
        };

        manager
            .cache_guide("test_tool".to_string(), guide)
            .await
            .expect("Operation failed - converted from unwrap()");

        // Update with telemetry
        let telemetry = TelemetryData {
            success_rate: 0.9,
            common_errors: vec!["file not found".to_string()],
            usage_patterns: vec![],
            avg_execution_time_ms: 5000, // Slow execution
            user_warnings: vec!["avoid large files".to_string()],
            platform_issues: {
                let mut issues = HashMap::new();
                issues.insert(
                    "windows".to_string(),
                    vec!["path separator issues".to_string()],
                );
                issues
            },
        };

        manager
            .update_with_telemetry("test_tool", telemetry)
            .await
            .expect("Operation failed - converted from unwrap()");

        // Check that guide was updated
        let cache = manager.cache.read().await;
        let cached = cache
            .get("test_tool")
            .expect("Operation failed - converted from unwrap()");

        assert!(cached
            .guide
            .constraints
            .contains(&"common issue: file not found".to_string()));
        assert!(cached
            .guide
            .not_for
            .contains(&"avoid large files".to_string()));
        assert_eq!(cached.guide.latency_class, "moderate");
        assert!(cached
            .guide
            .constraints
            .iter()
            .any(|c| c.contains("windows")));
    }

    #[test]
    fn test_guide_optimizer() {
        let guide = UsageGuide {
            usage_title: "Test Tool".to_string(),
            usage_summary: "A very long description that should be truncated because it exceeds the maximum length allowed for compact guides".to_string(),
            preconditions: vec![],
            arguments_brief: HashMap::new(),
            good_for: vec![],
            not_for: vec![],
            constraints: vec![
                "constraint 1".to_string(),
                "constraint 2".to_string(),
                "constraint 3".to_string(),
                "constraint 4".to_string(), // Should be truncated
            ],
            examples: vec!["test_tool --example".to_string()],
            platforms: vec!["linux".to_string()],
            cost_class: "free".to_string(),
            latency_class: "fast".to_string(),
            side_effects: vec![],
            risk_score: 5,
            capabilities: vec![],
            tags: vec![],
        };

        let compact = GuideOptimizer::optimize_for_llm(&guide);

        assert_eq!(compact.title, "Test Tool");
        assert!(compact.summary.len() <= 100);
        assert!(compact.summary.ends_with("..."));
        assert_eq!(compact.example, "test_tool --example");
        assert_eq!(compact.constraints.len(), 3); // Only first 3
        assert_eq!(compact.risk_level, "medium");
    }
}
