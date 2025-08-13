// @component: {"k":"M","id":"tool_context_module","t":"Tool Context Builder module with intelligent selection","m":{"cur":0,"tgt":100,"u":"%"},"f":["context","builder","reranker","metadata"]}

//! Tool Context Builder - Intelligent tool selection and context building
//!
//! This module provides intelligent tool selection capabilities using AI-powered
//! reranking and context building for optimal tool recommendations.

pub mod builder;
pub mod metadata;
pub mod reranker;
pub mod similarity;
pub mod usage_guide;

pub use builder::{
    ContextBuildingConfig, PerformancePriority, ProjectContext, SystemContext, ToolContext,
    ToolContextBuilder, ToolRankingResult, ToolSelectionConfig, ToolSelectionContext,
    ToolSelectionRequest, ToolSelectionResponse, UserPreferences,
};

pub use metadata::{
    ContextualMetadata, ExtractedMetadata, ToolCapabilities, ToolMetadataExtractor, ToolSemantics,
    ToolUsagePattern,
};

pub use reranker::{
    QwenToolReranker, RerankingConfig, RerankingRequest, RerankingResponse, ToolReranker,
    ToolSimilarityScore,
};

pub use usage_guide::{
    CacheStats, CompactGuide, GuideOptimizer, TelemetryData, UsageGuideConfig, UsageGuideManager,
};

pub use similarity::{MockEmbeddingGenerator, SimilarityCalculator};

/// Error types for tool context operations
#[derive(Debug, thiserror::Error)]
pub enum ToolContextError {
    #[error("Tool not found: {tool_id}")]
    ToolNotFound { tool_id: String },

    #[error("Reranking failed: {source}")]
    RerankingFailed { source: anyhow::Error },

    #[error("Metadata extraction failed: {source}")]
    MetadataExtractionFailed { source: anyhow::Error },

    #[error("Context building failed: {reason}")]
    ContextBuildingFailed { reason: String },

    #[error("Configuration error: {message}")]
    ConfigurationError { message: String },

    #[error("Model loading error: {source}")]
    ModelLoadingError { source: anyhow::Error },
}

pub type Result<T> = std::result::Result<T, ToolContextError>;
