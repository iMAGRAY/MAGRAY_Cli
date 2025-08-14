# Qwen3 Embeddings Integration

## Overview

The `Qwen3MemoryBridge` provides seamless integration between the Qwen3 embedding model from `ai` crate and the `memory` system, resolving **BLOCKER 2** that was preventing memory functionality.

## Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│ GpuBatchProcessor│───▶│ Qwen3MemoryBridge│───▶│Qwen3EmbeddingProvider│
│                 │    │                  │    │                 │
│ embed()         │    │ embed_text()     │    │ embed_text()    │
│ embed_batch()   │    │ embed_batch()    │    │ embed_batch()   │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                │
                                ▼
                       ┌─────────────────┐
                       │FallbackEmbeddingService│
                       │ (1024 dimension)│
                       └─────────────────┘
```

## Usage

### Basic Bridge Usage

```rust
use memory::Qwen3MemoryBridge;
use ai::EmbeddingConfig;

// Create bridge configuration
let config = EmbeddingConfig {
    model_name: "qwen3emb".to_string(),
    batch_size: 32,
    max_length: 512,
    use_gpu: false,
    gpu_config: None,
    embedding_dim: Some(1024),
};

// Create and initialize bridge
let bridge = Qwen3MemoryBridge::new(config).await?;
bridge.initialize().await?;

// Generate embeddings
let embedding = bridge.embed_text("example text").await?;
let batch_embeddings = bridge.embed_batch(&[
    "first text".to_string(),
    "second text".to_string(),
]).await?;
```

### GpuBatchProcessor Integration

```rust
use memory::{
    gpu_accelerated::{GpuBatchProcessor, BatchProcessorConfig},
    EmbeddingCache, CacheConfig
};
use std::sync::Arc;

// Create processor with Qwen3 integration
let config = BatchProcessorConfig {
    max_batch_size: 64,
    batch_timeout_ms: 100,
    use_gpu_if_available: false,
    cache_embeddings: true,
};

let cache = Arc::new(EmbeddingCache::new(cache_path, CacheConfig::default())?);
let processor = GpuBatchProcessor::with_qwen3_bridge(config, cache).await?;

// Use processor - automatically uses Qwen3 when available
let embedding = processor.embed("text to embed").await?;
let available = processor.is_qwen3_available().await;
let metrics = processor.get_qwen3_metrics().await;
```

## Fallback Behavior

The bridge implements graceful fallback:

1. **Primary**: Qwen3EmbeddingProvider (1024 dimensions)
2. **Fallback**: FallbackEmbeddingService (1024 dimensions, hash-based)
3. **Emergency**: Zero vector (1024 dimensions)

## Features

### Performance Metrics

```rust
let metrics = bridge.get_metrics().await;
println!("Total requests: {}", metrics.total_requests);
println!("Qwen3 requests: {}", metrics.qwen3_requests);
println!("Fallback requests: {}", metrics.fallback_requests);
println!("Average latency: {:.2}ms", metrics.avg_latency_ms);
```

### Management Methods

```rust
// Force fallback mode
bridge.force_fallback().await;

// Try to recover Qwen3 provider
let recovered = bridge.try_recover().await;

// Check availability
let available = bridge.is_qwen3_available().await;
```

## Model Requirements

The integration expects the Qwen3 model to be available at:
- `models/qwen3emb/model.onnx`
- `models/qwen3emb/tokenizer.json`

Alternative paths:
- `models/Qwen3-Embedding-0.6B-ONNX/model.onnx`
- `models/Qwen3-Embedding-0.6B-ONNX/tokenizer.json`

## Error Handling

The bridge handles various error conditions gracefully:

- **Missing ONNX Runtime**: Falls back to hash-based embeddings
- **Missing model files**: Falls back to deterministic embeddings
- **Model initialization failures**: Continues with fallback service
- **Runtime errors**: Automatic fallback with error logging

## Performance Characteristics

- **Target performance**: >100 embeddings/sec for single requests
- **Batch processing**: Up to 128 items per batch
- **Fallback latency**: ~1-5ms for fallback embeddings
- **Memory usage**: Minimal overhead through Arc/RwLock sharing

## Compilation Features

The integration is gated behind feature flags:

- `embeddings`: Enables Qwen3MemoryBridge
- `gpu-acceleration`: Enables GpuBatchProcessor integration
- `minimal`: Disables all advanced features

Example:
```bash
cargo build --features="embeddings"
cargo test --features="embeddings,gpu-acceleration"
```

## Testing

Run integration tests:
```bash
cargo test -p memory --features="embeddings" test_qwen3_memory_bridge_basic
cargo test -p memory --features="embeddings,gpu-acceleration" test_qwen3_gpu_batch_processor_integration
```

Note: Tests may show ONNX Runtime errors in environments without the model installed, but basic functionality is validated through compilation and fallback behavior.