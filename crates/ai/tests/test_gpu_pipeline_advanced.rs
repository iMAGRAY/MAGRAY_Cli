use ai::gpu_pipeline::{PipelineConfig, PipelineStats, ProcessedBatch};
use std::time::Duration;

#[test]
fn test_pipeline_config_default() {
    let config = PipelineConfig::default();
    
    assert_eq!(config.num_gpu_streams, 4);
    assert_eq!(config.max_batch_size, 128);
    assert_eq!(config.min_batch_size, 32);
    assert_eq!(config.batch_timeout, Duration::from_secs(30));
    assert!(config.use_pinned_memory);
    assert!(config.enable_prefetch);
    assert_eq!(config.prefetch_count, 2);
}

#[test]
fn test_pipeline_config_clone() {
    let original = PipelineConfig {
        num_gpu_streams: 8,
        max_batch_size: 256,
        min_batch_size: 64,
        batch_timeout: Duration::from_secs(60),
        use_pinned_memory: false,
        enable_prefetch: false,
        prefetch_count: 4,
    };
    
    let cloned = original.clone();
    
    assert_eq!(original.num_gpu_streams, cloned.num_gpu_streams);
    assert_eq!(original.max_batch_size, cloned.max_batch_size);
    assert_eq!(original.min_batch_size, cloned.min_batch_size);
    assert_eq!(original.batch_timeout, cloned.batch_timeout);
    assert_eq!(original.use_pinned_memory, cloned.use_pinned_memory);
    assert_eq!(original.enable_prefetch, cloned.enable_prefetch);
    assert_eq!(original.prefetch_count, cloned.prefetch_count);
}

#[test]
fn test_pipeline_config_custom_values() {
    let config = PipelineConfig {
        num_gpu_streams: 16,
        max_batch_size: 512,
        min_batch_size: 8,
        batch_timeout: Duration::from_millis(5000),
        use_pinned_memory: true,
        enable_prefetch: true,
        prefetch_count: 8,
    };
    
    assert_eq!(config.num_gpu_streams, 16);
    assert_eq!(config.max_batch_size, 512);
    assert_eq!(config.min_batch_size, 8);
    assert_eq!(config.batch_timeout, Duration::from_millis(5000));
    assert!(config.use_pinned_memory);
    assert!(config.enable_prefetch);
    assert_eq!(config.prefetch_count, 8);
}

#[test]
fn test_pipeline_config_edge_cases() {
    // Test minimum values
    let min_config = PipelineConfig {
        num_gpu_streams: 1,
        max_batch_size: 1,
        min_batch_size: 1,
        batch_timeout: Duration::from_millis(1),
        use_pinned_memory: false,
        enable_prefetch: false,
        prefetch_count: 0,
    };
    
    assert_eq!(min_config.num_gpu_streams, 1);
    assert_eq!(min_config.max_batch_size, 1);
    assert_eq!(min_config.min_batch_size, 1);
    assert!(!min_config.use_pinned_memory);
    assert!(!min_config.enable_prefetch);
    assert_eq!(min_config.prefetch_count, 0);
    
    // Test large values
    let max_config = PipelineConfig {
        num_gpu_streams: 1024,
        max_batch_size: 10000,
        min_batch_size: 5000,
        batch_timeout: Duration::from_secs(3600),
        use_pinned_memory: true,
        enable_prefetch: true,
        prefetch_count: 100,
    };
    
    assert_eq!(max_config.num_gpu_streams, 1024);
    assert_eq!(max_config.max_batch_size, 10000);
    assert_eq!(max_config.min_batch_size, 5000);
    assert_eq!(max_config.batch_timeout, Duration::from_secs(3600));
}

#[test]
fn test_pipeline_config_batch_size_relationship() {
    let config = PipelineConfig {
        max_batch_size: 100,
        min_batch_size: 50,
        ..Default::default()
    };
    
    // Min should be less than or equal to max
    assert!(config.min_batch_size <= config.max_batch_size);
    
    // Test range
    let batch_range = config.max_batch_size - config.min_batch_size;
    assert_eq!(batch_range, 50);
}

#[test]
fn test_pipeline_stats_default() {
    let stats = PipelineStats::default();
    
    assert_eq!(stats.total_batches, 0);
    assert_eq!(stats.total_texts, 0);
    assert_eq!(stats.total_gpu_time_ms, 0);
    assert_eq!(stats.total_transfer_time_ms, 0);
    assert_eq!(stats.total_time_ms, 0);
    assert_eq!(stats.avg_batch_size, 0.0);
    assert_eq!(stats.avg_gpu_utilization, 0.0);
    assert_eq!(stats.pipeline_throughput, 0.0);
}

#[test]
fn test_pipeline_stats_clone() {
    let original = PipelineStats {
        total_batches: 1000,
        total_texts: 50000,
        total_gpu_time_ms: 30000,
        total_transfer_time_ms: 5000,
        total_time_ms: 35000,
        avg_batch_size: 50.0,
        avg_gpu_utilization: 0.85,
        pipeline_throughput: 1428.57,
        active_streams: 4,
        gpu_utilization: vec![0.8, 0.9, 0.7, 0.85],
    };
    
    let cloned = original.clone();
    
    assert_eq!(original.total_batches, cloned.total_batches);
    assert_eq!(original.total_texts, cloned.total_texts);
    assert_eq!(original.total_gpu_time_ms, cloned.total_gpu_time_ms);
    assert_eq!(original.total_transfer_time_ms, cloned.total_transfer_time_ms);
    assert_eq!(original.total_time_ms, cloned.total_time_ms);
    assert_eq!(original.avg_batch_size, cloned.avg_batch_size);
    assert_eq!(original.avg_gpu_utilization, cloned.avg_gpu_utilization);
    assert_eq!(original.pipeline_throughput, cloned.pipeline_throughput);
    assert_eq!(original.active_streams, cloned.active_streams);
    assert_eq!(original.gpu_utilization, cloned.gpu_utilization);
}

#[test]
fn test_pipeline_stats_debug() {
    let stats = PipelineStats {
        total_batches: 42,
        total_texts: 2100,
        total_gpu_time_ms: 1000,
        total_transfer_time_ms: 200,
        total_time_ms: 1200,
        avg_batch_size: 50.0,
        avg_gpu_utilization: 0.83,
        pipeline_throughput: 1750.0,
        active_streams: 2,
        gpu_utilization: vec![0.85, 0.81],
    };
    
    let debug_str = format!("{:?}", stats);
    assert!(debug_str.contains("PipelineStats"));
    assert!(debug_str.contains("total_batches: 42"));
    assert!(debug_str.contains("total_texts: 2100"));
    assert!(debug_str.contains("avg_batch_size: 50.0"));
}

#[test]
fn test_pipeline_stats_calculations() {
    let stats = PipelineStats {
        total_batches: 100,
        total_texts: 5000,
        total_gpu_time_ms: 10000,
        total_transfer_time_ms: 2000,
        total_time_ms: 12000,
        avg_batch_size: 50.0,
        avg_gpu_utilization: 0.833, // 10000 / 12000
        pipeline_throughput: 416.67, // 5000 texts / 12 seconds
        active_streams: 1,
        gpu_utilization: vec![0.833],
    };
    
    // Verify calculated average batch size
    let calculated_avg_batch = stats.total_texts as f32 / stats.total_batches as f32;
    assert_eq!(calculated_avg_batch, 50.0);
    
    // Verify GPU utilization calculation
    let calculated_utilization = stats.total_gpu_time_ms as f32 / stats.total_time_ms as f32;
    assert!((calculated_utilization - 0.833).abs() < 0.001);
    
    // Verify throughput calculation (texts per second)
    let calculated_throughput = (stats.total_texts as f32 / stats.total_time_ms as f32) * 1000.0;
    assert!((calculated_throughput - 416.67).abs() < 0.1);
}

#[test]
fn test_processed_batch_creation() {
    let batch = ProcessedBatch {
        id: 12345,
        embeddings: vec![
            vec![0.1, 0.2, 0.3],
            vec![0.4, 0.5, 0.6],
        ],
        processing_time: Duration::from_millis(150),
        gpu_stream_id: 2,
    };
    
    assert_eq!(batch.id, 12345);
    assert_eq!(batch.embeddings.len(), 2);
    assert_eq!(batch.embeddings[0], vec![0.1, 0.2, 0.3]);
    assert_eq!(batch.embeddings[1], vec![0.4, 0.5, 0.6]);
    assert_eq!(batch.processing_time, Duration::from_millis(150));
    assert_eq!(batch.gpu_stream_id, 2);
}

#[test]
fn test_processed_batch_with_large_embeddings() {
    let large_embedding = vec![0.5; 1024]; // 1024-dimensional embedding
    let batch = ProcessedBatch {
        id: 99999,
        embeddings: vec![large_embedding.clone(); 64], // 64 texts
        processing_time: Duration::from_secs(2),
        gpu_stream_id: 7,
    };
    
    assert_eq!(batch.embeddings.len(), 64);
    assert_eq!(batch.embeddings[0].len(), 1024);
    assert_eq!(batch.embeddings[0], large_embedding);
    assert_eq!(batch.processing_time, Duration::from_secs(2));
    assert_eq!(batch.gpu_stream_id, 7);
}

#[test]
fn test_processed_batch_empty_embeddings() {
    let batch = ProcessedBatch {
        id: 0,
        embeddings: Vec::new(),
        processing_time: Duration::from_millis(1),
        gpu_stream_id: 0,
    };
    
    assert_eq!(batch.id, 0);
    assert!(batch.embeddings.is_empty());
    assert_eq!(batch.processing_time, Duration::from_millis(1));
    assert_eq!(batch.gpu_stream_id, 0);
}

#[test]
fn test_pipeline_config_timeout_variations() {
    // Test various timeout configurations
    let short_timeout_config = PipelineConfig {
        batch_timeout: Duration::from_millis(100),
        ..Default::default()
    };
    assert_eq!(short_timeout_config.batch_timeout, Duration::from_millis(100));
    
    let long_timeout_config = PipelineConfig {
        batch_timeout: Duration::from_secs(300), // 5 minutes
        ..Default::default()
    };
    assert_eq!(long_timeout_config.batch_timeout, Duration::from_secs(300));
    
    let zero_timeout_config = PipelineConfig {
        batch_timeout: Duration::from_millis(0),
        ..Default::default()
    };
    assert_eq!(zero_timeout_config.batch_timeout, Duration::from_millis(0));
}

#[test]
fn test_pipeline_config_prefetch_settings() {
    let prefetch_disabled = PipelineConfig {
        enable_prefetch: false,
        prefetch_count: 0,
        ..Default::default()
    };
    assert!(!prefetch_disabled.enable_prefetch);
    assert_eq!(prefetch_disabled.prefetch_count, 0);
    
    let aggressive_prefetch = PipelineConfig {
        enable_prefetch: true,
        prefetch_count: 10,
        ..Default::default()
    };
    assert!(aggressive_prefetch.enable_prefetch);
    assert_eq!(aggressive_prefetch.prefetch_count, 10);
}

#[test]
fn test_pipeline_stats_accumulation_simulation() {
    let mut stats = PipelineStats::default();
    
    // Simulate processing multiple batches
    stats.total_batches += 10;
    stats.total_texts += 500;
    stats.total_gpu_time_ms += 5000;
    stats.total_transfer_time_ms += 1000;
    stats.total_time_ms += 6000;
    
    // Calculate derived metrics
    stats.avg_batch_size = stats.total_texts as f32 / stats.total_batches as f32;
    stats.avg_gpu_utilization = stats.total_gpu_time_ms as f32 / stats.total_time_ms as f32;
    stats.pipeline_throughput = (stats.total_texts as f32 / stats.total_time_ms as f32) * 1000.0;
    
    assert_eq!(stats.total_batches, 10);
    assert_eq!(stats.total_texts, 500);
    assert_eq!(stats.avg_batch_size, 50.0);
    assert!((stats.avg_gpu_utilization - 0.833).abs() < 0.001);
    assert!((stats.pipeline_throughput - 83.333).abs() < 0.1);
}