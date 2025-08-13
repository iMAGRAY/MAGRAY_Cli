# Memory System API - Reference Documentation

## 📚 Overview

Memory System предоставляет hybrid vector search capabilities с GPU acceleration, dependency injection, и intelligent memory orchestration для MAGRAY CLI's local-first AI assistant.

## 🏗️ Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     Memory System                          │
├─────────────────┬───────────────┬───────────────────────────┤
│   Vector Store  │  DI Container │    Memory Orchestrator    │
│                 │               │                           │
│ ┌─────────────┐ │ ┌───────────┐ │ ┌─────────────────────────┐ │
│ │    HNSW     │ │ │ Service   │ │ │   Background Tasks      │ │
│ │   Index     │ │ │ Registry  │ │ │   Health Monitoring     │ │
│ └─────────────┘ │ └───────────┘ │ │   Resource Management   │ │
│ ┌─────────────┐ │ ┌───────────┐ │ │   Event Coordination    │ │
│ │    BM25     │ │ │ Factory   │ │ └─────────────────────────┘ │
│ │   Search    │ │ │ Manager   │ │                           │
│ └─────────────┘ │ └───────────┘ │                           │
├─────────────────┴───────────────┴───────────────────────────┤
│                 GPU Acceleration Layer                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   CUDA      │  │   OpenCL    │  │   CPU Fallback      │  │
│  │ Embeddings  │  │  Compute    │  │   SIMD Optimized    │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## 📊 System Components Status

| Component | Documentation | Implementation | Testing | Production Ready |
|-----------|---------------|----------------|---------|------------------|
| **Vector Store** | ✅ Complete | ✅ 90% | ✅ 85% | ⚠️ Near Ready |
| **DI Container** | ✅ Complete | ✅ 85% | ✅ 80% | 🔧 Development |
| **Memory Orchestrator** | ✅ Complete | ✅ 80% | ✅ 75% | 🔧 Development |
| **Embedding Service** | ✅ Complete | ✅ 75% | ✅ 70% | 🔧 Development |
| **GPU Acceleration** | ✅ Complete | ✅ 70% | ✅ 65% | 🔧 Development |
| **Search API** | ✅ Complete | ✅ 80% | ✅ 75% | 🔧 Development |

## 📖 API Reference Documents

### Core Components
- [**vector-store-api.md**](vector-store-api.md) - Hybrid HNSW + BM25 vector storage
- [**search-api.md**](search-api.md) - Vector similarity search и full-text search
- [**embedding-api.md**](embedding-api.md) - AI embedding generation service

### Dependency Injection System
- [**di-container-api.md**](di-container-api.md) - Service container и dependency resolution
- [**orchestrator-api.md**](orchestrator-api.md) - Memory operation orchestration

### Performance & Acceleration
- [**gpu-acceleration-api.md**](gpu-acceleration-api.md) - GPU computing integration
- [**performance-api.md**](performance-api.md) - Performance monitoring и optimization

### Development Guides
- [**memory-integration.md**](../guides/memory-integration.md) - Integration patterns
- [**performance-tuning.md**](../guides/performance-tuning.md) - Performance optimization

## 🚀 Quick Start Guide

### Basic Memory Operations

```rust
use memory::unified_memory_service::UnifiedMemoryService;
use memory::types::{MemoryRecord, EmbeddingVector};

#[tokio::main] 
async fn main() -> anyhow::Result<()> {
    // Initialize unified memory service
    let service = UnifiedMemoryService::new().await?;
    
    // Store memory with automatic embedding generation
    let record_id = service.store_memory(
        "Important project documentation about API design patterns"
    ).await?;
    
    println!("Stored record: {}", record_id);
    
    // Search similar memories
    let results = service.search_memory("API patterns", 5).await?;
    
    for result in results {
        println!("Found: {} (score: {:.3})", 
            result.content, 
            result.similarity_score
        );
    }
    
    Ok(())
}
```

### Advanced Vector Search

```rust
use memory::{
    vector_index_hnswlib::HNSWIndex,
    types::{EmbeddingVector, SearchQuery, SearchResult},
};

// Initialize HNSW index with custom configuration
let mut index = HNSWIndex::new(384, 1000)?; // 384-dim vectors, 1000 max elements

// Add vectors to index
let embedding = EmbeddingVector::new(vec![0.1, 0.2, 0.3, /* ... 384 dimensions */]);
let record_id = index.add_vector(embedding, "document_1".to_string())?;

// Perform similarity search
let query_vector = EmbeddingVector::new(vec![0.15, 0.25, 0.35, /* ... */]);
let results = index.search(&query_vector, 10)?; // Top 10 results

for result in results {
    println!("ID: {}, Score: {:.4}", result.id, result.score);
}
```

### Dependency Injection Setup

```rust
use memory::di::{
    OptimizedUnifiedContainer, 
    MemoryServiceConfig,
    ServiceRegistry
};

// Configure DI container
let config = MemoryServiceConfig {
    enable_gpu: true,
    cache_size: 1000,
    batch_size: 100,
    embedding_model: "qwen3".to_string(),
};

// Create container with services
let container = OptimizedUnifiedContainer::new(config).await?;

// Register custom services
container.register::<CustomEmbeddingService>()?;
container.register::<CustomCacheService>()?;

// Resolve services with automatic dependency injection
let memory_service = container.resolve::<UnifiedMemoryService>().await?;
let embedding_service = container.resolve::<EmbeddingService>().await?;
```

### GPU-Accelerated Operations

```rust
use memory::gpu_ultra_accelerated::{GpuMemoryService, GpuConfig};

// Configure GPU acceleration
let gpu_config = GpuConfig {
    enable_tensorrt: true,
    memory_limit: 2 * 1024 * 1024 * 1024, // 2GB
    batch_size: 64,
    precision: "fp16".to_string(),
};

// Initialize GPU service
let gpu_service = GpuMemoryService::new(gpu_config).await?;

// Batch processing with GPU acceleration
let texts = vec![
    "First document content",
    "Second document content", 
    "Third document content",
];

let embeddings = gpu_service.generate_embeddings_batch(texts).await?;
println!("Generated {} embeddings using GPU", embeddings.len());

// GPU-accelerated search
let query = "search query text";
let results = gpu_service.search_gpu_accelerated(query, 10).await?;
```

### Memory Orchestration

```rust
use memory::orchestration::{
    MemoryOrchestrator,
    BackgroundTaskManager,
    HealthChecker
};

// Initialize orchestrator with background tasks
let orchestrator = MemoryOrchestrator::new().await?;

// Configure health monitoring
orchestrator.configure_health_checks(vec![
    "embedding_service_health",
    "vector_index_health", 
    "cache_service_health",
]).await?;

// Start background maintenance tasks
orchestrator.start_background_tasks().await?;

// Monitor system health
let health_status = orchestrator.get_system_health().await?;
println!("Memory system health: {:?}", health_status);

// Graceful shutdown
orchestrator.shutdown().await?;
```

## ⚙️ Configuration

### Environment Variables

```bash
# Memory System Configuration
MAGRAY_MEMORY_CACHE_SIZE=1000          # Memory record cache size
MAGRAY_MEMORY_INDEX_SIZE=10000         # Max vectors in index
MAGRAY_MEMORY_BATCH_SIZE=100           # Batch processing size

# Embedding Configuration  
MAGRAY_EMBEDDING_MODEL=qwen3           # Embedding model to use
MAGRAY_EMBEDDING_DIMENSION=384         # Vector dimension
MAGRAY_EMBEDDING_CACHE_TTL=3600        # Cache TTL (seconds)

# GPU Acceleration
MAGRAY_GPU_ACCELERATION=true           # Enable GPU acceleration
MAGRAY_GPU_MEMORY_LIMIT=2147483648     # GPU memory limit (bytes)
MAGRAY_GPU_BATCH_SIZE=64               # GPU batch size
MAGRAY_TENSORRT_ENABLED=true           # Enable TensorRT optimization

# Search Configuration
MAGRAY_SEARCH_HYBRID_WEIGHT=0.7        # Vector vs text search weight
MAGRAY_SEARCH_MAX_RESULTS=50           # Maximum search results
MAGRAY_SEARCH_TIMEOUT=30000            # Search timeout (ms)

# Performance Tuning
MAGRAY_MEMORY_THREADS=4                # Processing thread count  
MAGRAY_MEMORY_PREFETCH=true            # Enable prefetching
MAGRAY_SIMD_OPTIMIZATION=true          # Enable SIMD optimizations
```

### Feature Flags

```toml
[features]
default = ["vector-search", "dependency-injection"]

# Core features
vector-search = ["hnsw-index", "bm25-search"]
dependency-injection = ["service-container", "factory-pattern"]
memory-orchestration = ["background-tasks", "health-monitoring"]

# Performance features  
gpu-acceleration = ["cuda", "tensorrt", "opencl"]
simd-optimization = ["avx2", "sse4", "neon"]
multi-threading = ["rayon", "tokio-parallel"]

# Storage features
persistence = ["sqlite", "file-storage"]
compression = ["lz4", "snappy"]
encryption = ["aes-256", "key-derivation"]

# Monitoring features
performance-monitoring = ["metrics", "profiling"]
health-checks = ["system-monitoring", "alerting"]
audit-logging = ["structured-logging"]
```

### Service Configuration

```rust
use memory::di::MemoryServiceConfig;
use std::time::Duration;

let config = MemoryServiceConfig {
    // Core settings
    cache_size: 2000,
    max_memory_records: 50000,
    batch_processing_size: 200,
    
    // Embedding settings
    embedding_model: "qwen3-embedding-0.6b".to_string(),
    embedding_dimension: 384,
    embedding_cache_ttl: Duration::from_secs(7200),
    
    // Search settings
    search_timeout: Duration::from_secs(30),
    max_search_results: 100,
    hybrid_search_weight: 0.8, // Favor vector search
    
    // Performance settings
    enable_gpu: true,
    enable_simd: true,
    worker_threads: 6,
    enable_prefetch: true,
    
    // Health monitoring
    enable_health_checks: true,
    health_check_interval: Duration::from_secs(60),
    
    // Background tasks
    enable_background_optimization: true,
    cleanup_interval: Duration::from_secs(300),
};
```

## 📊 Performance Characteristics

### Vector Operations

| Operation | Latency | Throughput | Memory Usage |
|-----------|---------|------------|--------------|
| **Single Embedding** | 5-20ms | 1000+ ops/sec | ~50MB |
| **Batch Embedding (64)** | 100-300ms | 5000+ ops/sec | ~200MB |
| **Vector Search (k=10)** | 1-5ms | 10000+ ops/sec | ~100MB |
| **Hybrid Search** | 5-15ms | 2000+ ops/sec | ~150MB |

### GPU Acceleration Impact

| Metric | CPU Only | GPU Accelerated | Improvement |
|--------|----------|-----------------|-------------|
| **Embedding Generation** | 20ms | 5ms | 4x faster |
| **Batch Processing** | 500ms | 100ms | 5x faster |
| **Large Dataset Search** | 100ms | 20ms | 5x faster |
| **Memory Usage** | 200MB | 500MB | Higher usage |

### Scalability Limits

| Component | Single Node Limit | Performance Notes |
|-----------|-------------------|-------------------|
| **Vector Index** | 1M vectors | Linear search degradation after 100k |
| **Memory Cache** | 10k records | LRU eviction with hit rate > 90% |
| **Concurrent Searches** | 100 threads | Lock contention beyond 50 threads |
| **GPU Batch Size** | 256 items | Memory bandwidth limit |

## 🔒 Memory Management & Security

### Memory Safety

```rust
use memory::resource_manager::{ResourceMonitor, MemoryBudget};

// Configure memory limits
let budget = MemoryBudget {
    max_heap_size: 1024 * 1024 * 1024, // 1GB
    max_vector_cache: 500 * 1024 * 1024, // 500MB
    max_embedding_cache: 200 * 1024 * 1024, // 200MB
    warn_threshold: 0.8, // Warn at 80% usage
    emergency_cleanup: 0.95, // Emergency cleanup at 95%
};

let monitor = ResourceMonitor::new(budget);

// Monitor memory usage
let usage = monitor.current_usage().await?;
println!("Memory usage: {:.1}% ({} MB)", 
    usage.percentage * 100.0,
    usage.used_bytes / (1024 * 1024)
);

// Automatic cleanup when limits approached
if usage.percentage > 0.9 {
    monitor.emergency_cleanup().await?;
}
```

### Data Privacy

```rust
use memory::privacy::{DataSanitizer, PiiDetector};

// Configure PII detection
let pii_detector = PiiDetector::new()
    .with_patterns(vec![
        r"\b\d{3}-\d{2}-\d{4}\b", // SSN
        r"\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b", // Credit card
        r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b", // Email
    ])
    .with_confidence_threshold(0.8);

// Sanitize content before storing
let sanitizer = DataSanitizer::new(pii_detector);

let content = "User email: john.doe@example.com, SSN: 123-45-6789";
let sanitized = sanitizer.sanitize(content).await?;
println!("Sanitized: {}", sanitized); // "User email: [EMAIL], SSN: [SSN]"

// Store sanitized content
service.store_memory(sanitized).await?;
```

## 🔍 Advanced Features

### Hybrid Search Implementation

```rust
use memory::search::{HybridSearcher, SearchConfig, WeightConfig};

// Configure hybrid search weights
let search_config = SearchConfig {
    vector_weight: 0.7,
    text_weight: 0.3,
    max_results: 20,
    min_similarity_threshold: 0.1,
    enable_reranking: true,
};

let searcher = HybridSearcher::new(search_config);

// Perform hybrid search
let query = "machine learning algorithms for data analysis";
let results = searcher.search_hybrid(query).await?;

for result in results {
    println!("Hybrid Score: {:.3} (Vector: {:.3}, Text: {:.3})", 
        result.combined_score,
        result.vector_score, 
        result.text_score
    );
}
```

### Memory Promotion & Optimization

```rust
use memory::ml_promotion::{
    MemoryPromoter, 
    PromotionCriteria,
    AccessPattern
};

// Configure memory promotion
let criteria = PromotionCriteria {
    min_access_frequency: 5,
    recency_weight: 0.4,
    frequency_weight: 0.6,
    relevance_threshold: 0.8,
};

let promoter = MemoryPromoter::new(criteria);

// Analyze access patterns
let patterns = promoter.analyze_access_patterns().await?;
for pattern in patterns {
    if pattern.should_promote() {
        println!("Promoting memory: {} (score: {:.3})", 
            pattern.memory_id, 
            pattern.promotion_score
        );
    }
}

// Execute promotion batch
let promoted = promoter.promote_batch().await?;
println!("Promoted {} memories to high-priority cache", promoted.len());
```

### Real-time Memory Sync

```rust
use memory::streaming::{MemoryStreamer, StreamConfig};
use tokio_stream::StreamExt;

// Configure real-time memory streaming
let stream_config = StreamConfig {
    buffer_size: 1000,
    batch_timeout: Duration::from_millis(100),
    enable_deduplication: true,
};

let mut streamer = MemoryStreamer::new(stream_config).await?;

// Process memory updates in real-time
while let Some(batch) = streamer.next().await {
    match batch {
        Ok(memory_batch) => {
            println!("Processing {} new memories", memory_batch.len());
            
            // Batch process embeddings
            let embeddings = embedding_service
                .generate_batch_embeddings(&memory_batch)
                .await?;
            
            // Update vector index
            for (memory, embedding) in memory_batch.iter().zip(embeddings.iter()) {
                index.add_vector(embedding.clone(), memory.id.clone())?;
            }
        },
        Err(e) => eprintln!("Stream error: {}", e),
    }
}
```

## 🐛 Error Handling & Recovery

### Memory System Errors

```rust
use memory::errors::{
    MemoryError, 
    EmbeddingError, 
    SearchError,
    ResourceError
};

async fn handle_memory_operations() -> Result<()> {
    match service.store_memory("content").await {
        Ok(record_id) => println!("Stored: {}", record_id),
        Err(MemoryError::Embedding(EmbeddingError::ModelNotFound)) => {
            eprintln!("Embedding model not available, falling back to simple storage");
            // Fallback to text-only storage
            service.store_text_only("content").await?;
        },
        Err(MemoryError::Resource(ResourceError::MemoryExhausted)) => {
            eprintln!("Memory limit reached, triggering cleanup");
            service.emergency_cleanup().await?;
            // Retry after cleanup
            service.store_memory("content").await?;
        },
        Err(MemoryError::Search(SearchError::IndexCorrupted)) => {
            eprintln!("Search index corrupted, rebuilding");
            service.rebuild_index().await?;
        },
        Err(e) => return Err(e.into()),
    }
    
    Ok(())
}
```

### Graceful Degradation

```rust
use memory::fallback::{FallbackStrategy, OperationMode};

// Configure fallback behavior
let fallback = FallbackStrategy::new()
    .with_gpu_fallback(OperationMode::CpuOnly)
    .with_embedding_fallback(OperationMode::TextOnly)
    .with_search_fallback(OperationMode::SimpleSearch);

// Execute with automatic fallback
let result = fallback.execute_with_fallback(|| async {
    // Try GPU-accelerated embedding + vector search
    service.smart_search_gpu("query", 10).await
}).await;

match result.mode {
    OperationMode::GpuAccelerated => println!("Full GPU acceleration used"),
    OperationMode::CpuOnly => println!("Fell back to CPU processing"),  
    OperationMode::TextOnly => println!("Fell back to text search only"),
    OperationMode::SimpleSearch => println!("Used basic string matching"),
}
```

## 📋 Testing Framework

### Unit Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use memory::testing::{MockEmbeddingService, TestMemoryData};
    
    #[tokio::test]
    async fn test_memory_storage_and_retrieval() {
        let service = UnifiedMemoryService::new_test().await?;
        
        // Store test memory
        let content = "Test memory content for unit testing";
        let record_id = service.store_memory(content).await?;
        
        assert!(!record_id.is_empty());
        
        // Search for stored memory
        let results = service.search_memory("test memory", 5).await?;
        assert!(!results.is_empty());
        
        let found = results.iter()
            .find(|r| r.content.contains("unit testing"));
        assert!(found.is_some());
    }
    
    #[tokio::test]
    async fn test_vector_similarity_search() {
        let mut index = HNSWIndex::new(384, 1000)?;
        
        // Add test vectors
        let vectors = TestMemoryData::generate_test_vectors(100, 384);
        for (i, vector) in vectors.iter().enumerate() {
            index.add_vector(vector.clone(), format!("test_{}", i))?;
        }
        
        // Test search accuracy
        let query = &vectors[0]; // Use first vector as query
        let results = index.search(query, 10)?;
        
        assert_eq!(results[0].id, "test_0"); // Should find itself first
        assert!(results[0].score > 0.99); // Near perfect similarity
    }
    
    #[tokio::test]
    async fn test_dependency_injection() {
        let container = OptimizedUnifiedContainer::new_test().await?;
        
        // Test service resolution
        let memory_service = container.resolve::<UnifiedMemoryService>().await?;
        let embedding_service = container.resolve::<EmbeddingService>().await?;
        
        assert!(memory_service.is_healthy().await?);
        assert!(embedding_service.is_available().await?);
    }
}
```

### Integration Testing

```rust
#[tokio::test]
async fn test_end_to_end_memory_workflow() {
    // Initialize full system
    let config = MemoryServiceConfig::test_config();
    let container = OptimizedUnifiedContainer::new(config).await?;
    let service = container.resolve::<UnifiedMemoryService>().await?;
    
    // Test data pipeline
    let documents = vec![
        "Machine learning algorithms for data analysis",
        "Natural language processing with transformers",
        "Computer vision and image recognition systems",
        "Distributed systems and microservices architecture",
    ];
    
    // Store documents  
    let mut record_ids = Vec::new();
    for doc in &documents {
        let id = service.store_memory(doc).await?;
        record_ids.push(id);
    }
    
    // Test semantic search
    let results = service.search_memory("AI and ML techniques", 10).await?;
    assert!(!results.is_empty());
    
    // Verify relevant results returned
    let ml_results = results.iter()
        .filter(|r| r.content.to_lowercase().contains("machine learning"))
        .count();
    assert!(ml_results > 0);
    
    // Test batch operations
    let batch_results = service.search_memory_batch(vec![
        "algorithms",
        "transformers", 
        "microservices"
    ]).await?;
    
    assert_eq!(batch_results.len(), 3);
    
    // Cleanup
    for id in record_ids {
        service.delete_memory(&id).await?;
    }
}
```

## 🔗 Integration Patterns

### Event Bus Integration

```rust
use common::event_bus::{EventBus, EventPublisher};
use memory::events::{MemoryEvent, MemoryEventType};

// Memory service with event publishing
pub struct EventAwareMemoryService {
    service: UnifiedMemoryService,
    event_publisher: EventPublisher,
}

impl EventAwareMemoryService {
    pub async fn store_memory_with_events(&self, content: &str) -> Result<String> {
        // Publish storage start event
        self.event_publisher.publish(
            "memory.storage.started",
            MemoryEvent {
                event_type: MemoryEventType::StorageStarted,
                content: content.to_string(),
                timestamp: chrono::Utc::now(),
                metadata: HashMap::new(),
            }
        ).await?;
        
        // Store memory
        let result = self.service.store_memory(content).await;
        
        // Publish completion event
        match &result {
            Ok(record_id) => {
                self.event_publisher.publish(
                    "memory.storage.completed",
                    MemoryEvent {
                        event_type: MemoryEventType::StorageCompleted,
                        content: record_id.clone(),
                        timestamp: chrono::Utc::now(),
                        metadata: [("record_id", record_id.clone())].into_iter().collect(),
                    }
                ).await?;
            },
            Err(error) => {
                self.event_publisher.publish(
                    "memory.storage.failed", 
                    MemoryEvent {
                        event_type: MemoryEventType::StorageFailed,
                        content: error.to_string(),
                        timestamp: chrono::Utc::now(),
                        metadata: HashMap::new(),
                    }
                ).await?;
            }
        }
        
        result
    }
}
```

## 🔗 Related Documentation

- [Multi-Agent Integration](../agents/integration-guide.md) - Agent system integration
- [Tools Integration](../tools/README.md) - Tool Context Builder integration
- [Security Configuration](../security/policy-api.md) - Security policy integration
- [Performance Tuning](../guides/performance-tuning.md) - Optimization guides

---

**API Version**: 1.0  
**Implementation Status**: 75% Complete  
**Production Readiness**: Development Phase  
**GPU Acceleration**: Available (CUDA/OpenCL)  
**Next Milestone**: Production-ready memory orchestration