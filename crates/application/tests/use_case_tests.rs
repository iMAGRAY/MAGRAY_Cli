#![allow(unused_imports)]
#![allow(unused_attributes)]
#![allow(clippy::empty_line_after_doc_comments)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::len_zero)]
//! Unit tests for Application Layer Use Cases

use application::dtos::*;
use application::use_cases::*;
use application::{ApplicationResult, RequestContext, RequestSource};
use std::sync::Arc;

/// Mock implementations for testing
#[derive(Clone)]
struct MockStoreMemoryUseCase {
    should_fail: bool,
}

#[async_trait::async_trait]
impl StoreMemoryUseCase for MockStoreMemoryUseCase {
    async fn store_memory(
        &self,
        _request: StoreMemoryRequest,
        _context: RequestContext,
    ) -> ApplicationResult<StoreMemoryResponse> {
        if self.should_fail {
            return Err(application::ApplicationError::infrastructure(
                "Mock failure",
            ));
        }

        Ok(StoreMemoryResponse {
            record_id: "test-record-123".to_string(),
            layer: domain::value_objects::layer_type::LayerType::Interact,
            embedding_dimensions: 384,
            processing_time_ms: 100,
            estimated_retrieval_time_ms: 5,
        })
    }

    async fn store_batch_memory(
        &self,
        request: BatchStoreMemoryRequest,
        _context: RequestContext,
    ) -> ApplicationResult<BatchStoreMemoryResponse> {
        if self.should_fail {
            return Err(application::ApplicationError::infrastructure(
                "Mock batch failure",
            ));
        }

        let total = request.records.len();
        let results: Vec<BatchStoreResult> = (0..total)
            .map(|i| BatchStoreResult {
                index: i,
                success: true,
                record_id: Some(format!("test-record-{}", i)),
                error: None,
                layer: Some(domain::value_objects::layer_type::LayerType::Interact),
            })
            .collect();

        Ok(BatchStoreMemoryResponse {
            total_requested: total,
            successful: total,
            failed: 0,
            results,
            total_processing_time_ms: 200,
        })
    }

    async fn retrieve_memory(
        &self,
        request: RetrieveMemoryRequest,
        _context: RequestContext,
    ) -> ApplicationResult<RetrieveMemoryResponse> {
        if self.should_fail {
            return Err(application::ApplicationError::not_found(
                "Record",
                "not found",
            ));
        }

        Ok(RetrieveMemoryResponse {
            record_id: request.record_id,
            content: "Test content".to_string(),
            metadata: None,
            layer: domain::value_objects::layer_type::LayerType::Interact,
            created_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
            access_count: 1,
            embedding: if request.include_embedding {
                Some(vec![0.1; 384])
            } else {
                None
            },
            stats: if request.include_stats {
                Some(RecordStats {
                    retrieval_time_ms: 10,
                    cache_hit: true,
                    layer_promotion_candidate: false,
                    similarity_scores: None,
                })
            } else {
                None
            },
        })
    }
}

fn create_test_context() -> RequestContext {
    RequestContext::new(RequestSource::Internal)
}

#[tokio::test]
async fn test_store_memory_use_case_success() {
    let use_case = MockStoreMemoryUseCase { should_fail: false };
    let context = create_test_context();

    let request = StoreMemoryRequest {
        content: "Test content".to_string(),
        metadata: None,
        project: "test-project".to_string(),
        kind: Some("test".to_string()),
        session: Some("test-session".to_string()),
        target_layer: None,
        priority: Some(2),
        tags: vec!["test".to_string()],
    };

    let result = use_case.store_memory(request, context).await;

    let response = result.expect("Use case operation should succeed");
    assert_eq!(response.record_id, "test-record-123");
    assert_eq!(response.embedding_dimensions, 384);
}

#[tokio::test]
async fn test_store_memory_use_case_failure() {
    let use_case = MockStoreMemoryUseCase { should_fail: true };
    let context = create_test_context();

    let request = StoreMemoryRequest {
        content: "Test content".to_string(),
        metadata: None,
        project: "test".to_string(),
        kind: None,
        session: None,
        target_layer: None,
        priority: None,
        tags: vec![],
    };

    let result = use_case.store_memory(request, context).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_batch_store_memory_use_case() {
    let use_case = MockStoreMemoryUseCase { should_fail: false };
    let context = create_test_context();

    let records = vec![
        StoreMemoryRequest {
            content: "Content 1".to_string(),
            metadata: None,
            project: "test".to_string(),
            kind: None,
            session: None,
            target_layer: None,
            priority: None,
            tags: vec![],
        },
        StoreMemoryRequest {
            content: "Content 2".to_string(),
            metadata: None,
            project: "test".to_string(),
            kind: None,
            session: None,
            target_layer: None,
            priority: None,
            tags: vec![],
        },
    ];

    let batch_request = BatchStoreMemoryRequest {
        records,
        options: BatchOptions {
            parallel_processing: true,
            failure_tolerance: FailureTolerance::Partial,
            progress_reporting: false,
        },
    };

    let result = use_case.store_batch_memory(batch_request, context).await;

    let response = result.expect("Use case operation should succeed");
    assert_eq!(response.total_requested, 2);
    assert_eq!(response.successful, 2);
    assert_eq!(response.failed, 0);
}

#[tokio::test]
async fn test_retrieve_memory_use_case() {
    let use_case = MockStoreMemoryUseCase { should_fail: false };
    let context = create_test_context();

    let request = RetrieveMemoryRequest {
        record_id: "test-123".to_string(),
        include_embedding: true,
        include_stats: true,
    };

    let result = use_case.retrieve_memory(request, context).await;

    let response = result.expect("Use case operation should succeed");
    assert_eq!(response.record_id, "test-123");
    assert_eq!(response.content, "Test content");
    assert!(response.embedding.is_some());
    assert!(response.stats.is_some());
}

#[tokio::test]
async fn test_retrieve_memory_not_found() {
    let use_case = MockStoreMemoryUseCase { should_fail: true };
    let context = create_test_context();

    let request = RetrieveMemoryRequest {
        record_id: "nonexistent".to_string(),
        include_embedding: false,
        include_stats: false,
    };

    let result = use_case.retrieve_memory(request, context).await;

    assert!(result.is_err());
}

#[derive(Clone)]
struct MockSearchMemoryUseCase;

#[async_trait::async_trait]
impl SearchMemoryUseCase for MockSearchMemoryUseCase {
    async fn search_memory(
        &self,
        request: SearchMemoryRequest,
        _context: RequestContext,
    ) -> ApplicationResult<SearchMemoryResponse> {
        Ok(SearchMemoryResponse {
            results: vec![SearchResult {
                record_id: "result-1".to_string(),
                content: "Matching content 1".to_string(),
                content_preview: "Matching content 1".to_string(),
                score: 0.95,
                similarity_score: 0.95,
                layer: domain::value_objects::layer_type::LayerType::Interact,
                project: None,
                metadata: None,
                created_at: chrono::Utc::now(),
                last_accessed: chrono::Utc::now(),
                tags: vec!["content".to_string()],
                explanation: None,
                embedding: None,
            }],
            total_results: 1,
            search_time_ms: 50,
            query_hash: "hash123".to_string(),
            layers_searched: request.layers.unwrap_or_default(),
        })
    }

    async fn similarity_search(
        &self,
        request: SimilaritySearchRequest,
        _context: RequestContext,
    ) -> ApplicationResult<SimilaritySearchResponse> {
        Ok(SimilaritySearchResponse {
            results: vec![SimilarityResult {
                record_id: "sim-1".to_string(),
                embedding: if request.include_vectors {
                    vec![0.1; 384]
                } else {
                    vec![]
                },
                similarity_score: 0.89,
                metadata: None,
            }],
            total_results: 1,
            search_time_ms: 25,
            embedding_dimensions: 384,
        })
    }

    async fn get_cached_search(
        &self,
        _query_hash: &str,
        _context: RequestContext,
    ) -> ApplicationResult<Option<SearchMemoryResponse>> {
        Ok(None)
    }
}

#[tokio::test]
async fn test_search_memory_use_case() {
    let use_case = MockSearchMemoryUseCase;
    let context = create_test_context();

    let request = SearchMemoryRequest {
        query: "test query".to_string(),
        limit: Some(10),
        similarity_threshold: Some(0.7),
        layers: Some(vec![
            domain::value_objects::layer_type::LayerType::Interact,
            domain::value_objects::layer_type::LayerType::Insights,
        ]),
        project: None,
        project_filter: None,
        filters: None,
        include_embeddings: false,
        use_cache: true,
    };

    let result = use_case.search_memory(request, context).await;

    let response = result.expect("Use case operation should succeed");
    assert_eq!(response.results.len(), 1);
    assert_eq!(response.results[0].record_id, "result-1");
    assert_eq!(response.query_hash, "hash123");
}

#[tokio::test]
async fn test_similarity_search_use_case() {
    let use_case = MockSearchMemoryUseCase;
    let context = create_test_context();

    let request = SimilaritySearchRequest {
        query_embedding: vec![0.1; 384],
        limit: Some(5),
        similarity_threshold: Some(0.8),
        threshold: Some(0.8),
        layers: Some(vec![domain::value_objects::layer_type::LayerType::Insights]),
        include_vectors: true,
        metadata_filters: None,
    };

    let result = use_case.similarity_search(request, context).await;

    let response = result.expect("Use case operation should succeed");
    assert_eq!(response.results.len(), 1);
    assert_eq!(response.results[0].record_id, "sim-1");
    assert!(!response.results[0].embedding.is_empty());
}

#[tokio::test]
async fn test_request_context_creation() {
    let context = RequestContext::new(RequestSource::Cli);

    assert!(context.request_id.to_string().len() > 0);
    assert!(context.correlation_id.len() > 0);
    assert!(context.user_id.is_none());
    assert!(matches!(context.source, RequestSource::Cli));
}

#[tokio::test]
async fn test_request_context_with_user() {
    let context = RequestContext::new(RequestSource::Api)
        .with_user("user-123".to_string())
        .with_correlation_id("corr-456".to_string());

    assert_eq!(context.user_id, Some("user-123".to_string()));
    assert_eq!(context.correlation_id, "corr-456");
    assert!(matches!(context.source, RequestSource::Api));
}
