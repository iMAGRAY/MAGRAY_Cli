//! SearchQuery - Domain entity for memory search operations
//!
//! Pure business logic for search queries

use serde::{Deserialize, Serialize};
use crate::value_objects::{LayerType, ScoreThreshold};
use crate::entities::EmbeddingVector;
use crate::errors::{DomainError, DomainResult};
use crate::RecordCount;

/// Domain entity representing a search query in the memory system
/// 
/// Contains business rules and validation for search operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Text query (business input)
    query_text: String,
    
    /// Vector representation (computed from text)
    query_vector: Option<EmbeddingVector>,
    
    /// Layers to search in (business constraint)
    target_layers: Vec<LayerType>,
    
    /// Maximum results to return (business limit)
    max_results: RecordCount,
    
    /// Minimum similarity score (business threshold)
    score_threshold: ScoreThreshold,
    
    /// Filter by project (business context)
    project_filter: Option<String>,
    
    /// Filter by tags (business categorization)
    tag_filters: Vec<String>,
    
    /// Filter by content kind (business classification)
    kind_filter: Option<String>,
    
    /// Include only recent records (business temporal filter)
    max_age_hours: Option<u64>,
}

impl SearchQuery {
    /// Create new search query with business validation
    pub fn new(query_text: String) -> DomainResult<Self> {
        if query_text.trim().is_empty() {
            return Err(DomainError::InvalidSearchQuery("Query text cannot be empty".to_string()));
        }
        
        Ok(Self {
            query_text,
            query_vector: None,
            target_layers: vec![LayerType::Interact, LayerType::Insights], // Default business layers
            max_results: 10, // Default business limit
            score_threshold: ScoreThreshold::medium(),
            project_filter: None,
            tag_filters: Vec::new(),
            kind_filter: None,
            max_age_hours: None,
        })
    }
    
    /// Builder pattern for complex queries
    pub fn with_layers(mut self, layers: Vec<LayerType>) -> Self {
        self.target_layers = layers;
        self
    }
    
    pub fn with_max_results(mut self, max_results: RecordCount) -> DomainResult<Self> {
        if max_results == 0 {
            return Err(DomainError::InvalidSearchQuery("Max results must be greater than 0".to_string()));
        }
        
        // Business rule: reasonable limits
        if max_results > 1000 {
            return Err(DomainError::SearchLimitExceeded {
                requested: max_results,
                max_allowed: 1000,
            });
        }
        
        self.max_results = max_results;
        Ok(self)
    }
    
    pub fn with_score_threshold(mut self, threshold: ScoreThreshold) -> Self {
        self.score_threshold = threshold;
        self
    }
    
    pub fn with_project(mut self, project: String) -> DomainResult<Self> {
        if project.trim().is_empty() {
            return Err(DomainError::InvalidProjectName("Project name cannot be empty".to_string()));
        }
        self.project_filter = Some(project);
        Ok(self)
    }
    
    pub fn with_tags(mut self, tags: Vec<String>) -> DomainResult<Self> {
        // Business validation: no empty tags
        for tag in &tags {
            if tag.trim().is_empty() {
                return Err(DomainError::InvalidTag("Tag cannot be empty".to_string()));
            }
        }
        self.tag_filters = tags;
        Ok(self)
    }
    
    pub fn with_kind(mut self, kind: String) -> DomainResult<Self> {
        if kind.trim().is_empty() {
            return Err(DomainError::InvalidKind("Kind cannot be empty".to_string()));
        }
        self.kind_filter = Some(kind);
        Ok(self)
    }
    
    pub fn with_max_age_hours(mut self, hours: u64) -> Self {
        self.max_age_hours = Some(hours);
        self
    }
    
    pub fn with_vector(mut self, vector: EmbeddingVector) -> Self {
        self.query_vector = Some(vector);
        self
    }
    
    // Getters for query components
    pub fn query_text(&self) -> &str {
        &self.query_text
    }
    
    pub fn query_vector(&self) -> Option<&EmbeddingVector> {
        self.query_vector.as_ref()
    }
    
    pub fn target_layers(&self) -> &[LayerType] {
        &self.target_layers
    }
    
    pub fn max_results(&self) -> RecordCount {
        self.max_results
    }
    
    pub fn score_threshold(&self) -> ScoreThreshold {
        self.score_threshold
    }
    
    pub fn project_filter(&self) -> Option<&str> {
        self.project_filter.as_deref()
    }
    
    pub fn tag_filters(&self) -> &[String] {
        &self.tag_filters
    }
    
    pub fn kind_filter(&self) -> Option<&str> {
        self.kind_filter.as_deref()
    }
    
    pub fn max_age_hours(&self) -> Option<u64> {
        self.max_age_hours
    }
    
    // Business logic methods
    
    /// Check if query requires vector computation
    pub fn needs_vector_computation(&self) -> bool {
        self.query_vector.is_none()
    }
    
    /// Check if query has any filters applied
    pub fn has_filters(&self) -> bool {
        self.project_filter.is_some() || 
        !self.tag_filters.is_empty() || 
        self.kind_filter.is_some() ||
        self.max_age_hours.is_some()
    }
    
    /// Get business priority of this query
    pub fn priority(&self) -> QueryPriority {
        if self.target_layers.contains(&LayerType::Interact) {
            QueryPriority::High // Hot data queries are high priority
        } else if self.target_layers.contains(&LayerType::Insights) {
            QueryPriority::Medium
        } else {
            QueryPriority::Low
        }
    }
    
    /// Check if query is simple (text-only, no filters)
    pub fn is_simple(&self) -> bool {
        !self.has_filters() && 
        self.target_layers.len() <= 2 &&
        self.max_results <= 20
    }
    
    /// Get estimated complexity for resource planning
    pub fn complexity_score(&self) -> f32 {
        let mut score = 1.0;
        
        // More layers = more complexity
        score += self.target_layers.len() as f32 * 0.1;
        
        // More results = more complexity
        score += (self.max_results as f32 / 100.0).min(2.0);
        
        // Filters add complexity
        if self.has_filters() {
            score += 0.5;
        }
        
        // Lower thresholds = more results to process
        score += (1.0 - self.score_threshold.value()) * 0.5;
        
        score
    }
    
    /// Validate query for business rules
    pub fn validate(&self) -> DomainResult<()> {
        if self.query_text.trim().is_empty() {
            return Err(DomainError::InvalidSearchQuery("Query text cannot be empty".to_string()));
        }
        
        if self.target_layers.is_empty() {
            return Err(DomainError::InvalidSearchQuery("Must specify at least one layer".to_string()));
        }
        
        if self.max_results == 0 {
            return Err(DomainError::InvalidSearchQuery("Max results must be greater than 0".to_string()));
        }
        
        Ok(())
    }
}

/// Business priority levels for query processing
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QueryPriority {
    Low = 1,
    Medium = 2,
    High = 3,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            query_text: String::new(),
            query_vector: None,
            target_layers: vec![LayerType::Interact, LayerType::Insights],
            max_results: 10,
            score_threshold: ScoreThreshold::medium(),
            project_filter: None,
            tag_filters: Vec::new(),
            kind_filter: None,
            max_age_hours: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_query_creation() {
        let query = SearchQuery::new("test query".to_string()).unwrap();
        assert_eq!(query.query_text(), "test query");
        assert_eq!(query.max_results(), 10);
        assert!(!query.has_filters());
    }
    
    #[test]
    fn test_empty_query_validation() {
        let result = SearchQuery::new("".to_string());
        assert!(result.is_err());
        
        let result = SearchQuery::new("   ".to_string());
        assert!(result.is_err());
    }
    
    #[test]
    fn test_builder_pattern() {
        let query = SearchQuery::new("test".to_string())
            .unwrap()
            .with_layers(vec![LayerType::Assets])
            .with_max_results(5)
            .unwrap()
            .with_project("my_project".to_string())
            .unwrap();
        
        assert_eq!(query.target_layers(), &[LayerType::Assets]);
        assert_eq!(query.max_results(), 5);
        assert_eq!(query.project_filter(), Some("my_project"));
        assert!(query.has_filters());
    }
    
    #[test]
    fn test_max_results_validation() {
        let query = SearchQuery::new("test".to_string()).unwrap();
        
        // Zero results should fail
        let result = query.clone().with_max_results(0);
        assert!(result.is_err());
        
        // Too many results should fail
        let result = query.with_max_results(2000);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_query_priority() {
        let interact_query = SearchQuery::new("test".to_string())
            .unwrap()
            .with_layers(vec![LayerType::Interact]);
        assert_eq!(interact_query.priority(), QueryPriority::High);
        
        let assets_query = SearchQuery::new("test".to_string())
            .unwrap()
            .with_layers(vec![LayerType::Assets]);
        assert_eq!(assets_query.priority(), QueryPriority::Low);
    }
    
    #[test]
    fn test_complexity_score() {
        let simple_query = SearchQuery::new("test".to_string()).unwrap();
        let complex_query = SearchQuery::new("test".to_string())
            .unwrap()
            .with_max_results(100)
            .unwrap()
            .with_project("project".to_string())
            .unwrap()
            .with_tags(vec!["tag1".to_string(), "tag2".to_string()])
            .unwrap();
        
        assert!(complex_query.complexity_score() > simple_query.complexity_score());
    }
}