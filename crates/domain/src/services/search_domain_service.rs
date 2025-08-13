//! SearchDomainService - Business logic for search operations
//!
//! Coordinates complex search scenarios with business rules

use crate::entities::{EmbeddingVector, MemoryRecord, SearchQuery};
use crate::errors::DomainResult;
use crate::repositories::{EmbeddingRepository, MemoryRepository, SearchRepository, SearchResults};
use crate::value_objects::{LayerType, ScoreThreshold};
use async_trait::async_trait;
use std::sync::Arc;

/// Domain service for search business operations
pub struct SearchDomainService<S, M, E>
where
    S: SearchRepository,
    M: MemoryRepository,
    E: EmbeddingRepository,
{
    search_repo: Arc<S>,
    memory_repo: Arc<M>,
    embedding_repo: Arc<E>,
}

impl<S, M, E> SearchDomainService<S, M, E>
where
    S: SearchRepository,
    M: MemoryRepository,
    E: EmbeddingRepository,
{
    pub fn new(search_repo: Arc<S>, memory_repo: Arc<M>, embedding_repo: Arc<E>) -> Self {
        Self {
            search_repo,
            memory_repo,
            embedding_repo,
        }
    }

    /// Execute intelligent search with business logic
    pub async fn search_with_intelligence(
        &self,
        mut query: SearchQuery,
    ) -> DomainResult<SearchResults> {
        // Business validation
        query.validate()?;

        // Apply business intelligence to query
        self.optimize_query(&mut query).await?;

        // Execute search based on query characteristics
        let results = if query.is_simple() {
            self.search_repo.text_search(query).await?
        } else if query.needs_vector_computation() {
            // Compute vector first, then search
            let vector = self.compute_query_vector(&query).await?;
            let query_with_vector = query.with_vector(vector);
            self.search_repo.search(query_with_vector).await?
        } else {
            self.search_repo.search(query).await?
        };

        // Apply business post-processing
        self.post_process_results(results).await
    }

    /// Search with automatic fallback strategies
    pub async fn search_with_fallback(&self, query: SearchQuery) -> DomainResult<SearchResults> {
        // Try smart search first
        match self.search_with_intelligence(query.clone()).await {
            Ok(results) if !results.is_empty() => Ok(results),
            _ => {
                // Fallback: broaden search criteria
                let fallback_query = self.create_fallback_query(query)?;
                self.search_repo.search(fallback_query).await
            }
        }
    }

    /// Search within business context (project-aware)
    pub async fn context_aware_search(
        &self,
        query: SearchQuery,
        project: &str,
        session: &str,
    ) -> DomainResult<SearchResults> {
        // Get context-relevant records first
        let context_records = self.memory_repo.find_by_project(project).await?;
        let session_records = self.memory_repo.find_by_session(session).await?;

        // Execute context search
        let results = self
            .search_repo
            .context_search(query, project, session)
            .await?;

        self.boost_context_results(results, &context_records, &session_records)
            .await
    }

    // Private helper methods

    async fn optimize_query(&self, query: &mut SearchQuery) -> DomainResult<()> {
        // Business logic: optimize query based on patterns
        if query.target_layers().contains(&LayerType::Interact) && query.max_results() < 5 {
            // For hot data queries, get more results to ensure relevance
            *query = query.clone().with_max_results(10)?;
        }

        // Adjust threshold based on query complexity
        if query.complexity_score() > 2.0 {
            // For complex queries, be more permissive
            *query = query.clone().with_score_threshold(ScoreThreshold::low());
        }

        Ok(())
    }

    async fn compute_query_vector(&self, _query: &SearchQuery) -> DomainResult<EmbeddingVector> {
        // This would normally call an embedding service
        let expected_dims = self.embedding_repo.expected_dimensions();
        Ok(EmbeddingVector::zero(expected_dims))
    }

    fn create_fallback_query(&self, original_query: SearchQuery) -> DomainResult<SearchQuery> {
        let fallback = SearchQuery::new(original_query.query_text().to_string())?
            .with_layers(LayerType::all_layers())
            .with_score_threshold(ScoreThreshold::low())
            .with_max_results(original_query.max_results() * 2)?;

        Ok(fallback)
    }

    async fn post_process_results(
        &self,
        mut results: SearchResults,
    ) -> DomainResult<SearchResults> {
        // Apply business rules to search results
        for result_record in &mut results.records {
            // Update access count (business event)
            let record_id = result_record.record.id();
            let _ = self.memory_repo.find_by_id(record_id).await; // This would trigger access recording
        }

        // Sort by business relevance (combination of similarity and business score)
        results.records.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .expect("Operation failed - converted from unwrap()")
        });

        // Update ranks after sorting
        for (index, result_record) in results.records.iter_mut().enumerate() {
            result_record.rank = index + 1;
        }

        Ok(results)
    }

    async fn boost_context_results(
        &self,
        mut results: SearchResults,
        _context_records: &[MemoryRecord],
        _session_records: &[MemoryRecord],
    ) -> DomainResult<SearchResults> {
        // Business logic: boost results that are contextually relevant
        for result_record in &mut results.records {
            // Boost records from same project/session
            if result_record.record.project() == result_record.record.project() {
                result_record.relevance_score *= 1.2;
            }
        }

        Ok(results)
    }

    /// Search across specific memory layers with filtering
    pub async fn search_across_layers(
        &self,
        query: SearchQuery,
        layers: &[LayerType],
        limit: usize,
        threshold: Option<&ScoreThreshold>,
        project: Option<&str>,
        filters: Option<&std::collections::HashMap<String, String>>,
    ) -> DomainResult<SearchResults> {
        // Create search query with layer restrictions
        let mut layer_query = query.with_layers(layers.to_vec());

        if let Some(threshold) = threshold {
            layer_query = layer_query.with_score_threshold(*threshold);
        }

        layer_query = layer_query.with_max_results(limit)?;

        // Apply project filter if provided
        if let Some(project) = project {
            layer_query = layer_query.with_project(project.to_string())?;
        }

        // Apply additional filters if provided
        if let Some(_filters) = filters {
            // Filters would need specific implementation in SearchQuery
            // For now, just use the query as-is
        }

        self.search_with_intelligence(layer_query).await
    }
}

/// Trait for search domain service operations
#[async_trait]
#[allow(dead_code)]
pub trait SearchDomainServiceTrait: Send + Sync {
    async fn search_with_intelligence(&self, query: SearchQuery) -> DomainResult<SearchResults>;
    async fn search_with_fallback(&self, query: SearchQuery) -> DomainResult<SearchResults>;
    async fn context_aware_search(
        &self,
        query: SearchQuery,
        project: &str,
        session: &str,
    ) -> DomainResult<SearchResults>;
    async fn search_across_layers(
        &self,
        query: SearchQuery,
        layers: &[LayerType],
        limit: usize,
        threshold: Option<&ScoreThreshold>,
        project: Option<&str>,
        filters: Option<&std::collections::HashMap<String, String>>,
    ) -> DomainResult<SearchResults>;
}

#[async_trait]
impl<S, M, E> SearchDomainServiceTrait for SearchDomainService<S, M, E>
where
    S: SearchRepository,
    M: MemoryRepository,
    E: EmbeddingRepository,
{
    async fn search_with_intelligence(&self, query: SearchQuery) -> DomainResult<SearchResults> {
        self.search_with_intelligence(query).await
    }

    async fn search_with_fallback(&self, query: SearchQuery) -> DomainResult<SearchResults> {
        self.search_with_fallback(query).await
    }

    async fn context_aware_search(
        &self,
        query: SearchQuery,
        project: &str,
        session: &str,
    ) -> DomainResult<SearchResults> {
        self.context_aware_search(query, project, session).await
    }

    async fn search_across_layers(
        &self,
        query: SearchQuery,
        layers: &[LayerType],
        limit: usize,
        threshold: Option<&ScoreThreshold>,
        project: Option<&str>,
        filters: Option<&std::collections::HashMap<String, String>>,
    ) -> DomainResult<SearchResults> {
        self.search_across_layers(query, layers, limit, threshold, project, filters)
            .await
    }
}
