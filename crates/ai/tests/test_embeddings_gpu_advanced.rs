#[cfg(feature = "gpu")]
use ai::embeddings_gpu::PerformanceMetrics;

#[cfg(feature = "gpu")]
#[test]
fn test_performance_metrics_creation() {
    let metrics = PerformanceMetrics::default();

    assert_eq!(metrics.total_requests, 0);
    assert_eq!(metrics.total_tokens, 0);
    assert_eq!(metrics.total_time_ms, 0);
    assert_eq!(metrics.gpu_time_ms, 0);
    assert_eq!(metrics.cpu_time_ms, 0);
    assert_eq!(metrics.avg_batch_size, 0.0);
    assert_eq!(metrics.cache_hits, 0);
    assert_eq!(metrics.cache_misses, 0);
}

#[cfg(feature = "gpu")]
#[test]
fn test_performance_metrics_tokens_per_second_zero_time() {
    let metrics = PerformanceMetrics {
        total_tokens: 1000,
        total_time_ms: 0,
        ..Default::default()
    };

    assert_eq!(metrics.tokens_per_second(), 0.0);
}

#[cfg(feature = "gpu")]
#[test]
fn test_performance_metrics_tokens_per_second_calculation() {
    let metrics = PerformanceMetrics {
        total_tokens: 1000,
        total_time_ms: 500, // 0.5 seconds
        ..Default::default()
    };

    // Should be 1000 tokens / 0.5 seconds = 2000 tokens/second
    assert_eq!(metrics.tokens_per_second(), 2000.0);
}

#[cfg(feature = "gpu")]
#[test]
fn test_performance_metrics_tokens_per_second_edge_cases() {
    // Zero tokens
    let metrics_zero_tokens = PerformanceMetrics {
        total_tokens: 0,
        total_time_ms: 1000,
        ..Default::default()
    };
    assert_eq!(metrics_zero_tokens.tokens_per_second(), 0.0);

    // Very small time
    let metrics_small_time = PerformanceMetrics {
        total_tokens: 10,
        total_time_ms: 1, // 1ms
        ..Default::default()
    };
    assert_eq!(metrics_small_time.tokens_per_second(), 10000.0);

    // Large numbers
    let metrics_large = PerformanceMetrics {
        total_tokens: 1_000_000,
        total_time_ms: 2_000, // 2 seconds
        ..Default::default()
    };
    assert_eq!(metrics_large.tokens_per_second(), 500_000.0);
}

#[cfg(feature = "gpu")]
#[test]
fn test_performance_metrics_clone() {
    let original = PerformanceMetrics {
        total_requests: 42,
        total_tokens: 1337,
        total_time_ms: 999,
        gpu_time_ms: 500,
        cpu_time_ms: 499,
        avg_batch_size: 8.5,
        cache_hits: 100,
        cache_misses: 20,
    };

    let cloned = original.clone();

    assert_eq!(original.total_requests, cloned.total_requests);
    assert_eq!(original.total_tokens, cloned.total_tokens);
    assert_eq!(original.total_time_ms, cloned.total_time_ms);
    assert_eq!(original.gpu_time_ms, cloned.gpu_time_ms);
    assert_eq!(original.cpu_time_ms, cloned.cpu_time_ms);
    assert_eq!(original.avg_batch_size, cloned.avg_batch_size);
    assert_eq!(original.cache_hits, cloned.cache_hits);
    assert_eq!(original.cache_misses, cloned.cache_misses);
}

#[cfg(feature = "gpu")]
#[test]
fn test_performance_metrics_debug() {
    let metrics = PerformanceMetrics {
        total_requests: 5,
        total_tokens: 150,
        total_time_ms: 100,
        gpu_time_ms: 80,
        cpu_time_ms: 20,
        avg_batch_size: 10.0,
        cache_hits: 4,
        cache_misses: 1,
    };

    let debug_str = format!("{:?}", metrics);
    assert!(debug_str.contains("PerformanceMetrics"));
    assert!(debug_str.contains("total_requests: 5"));
    assert!(debug_str.contains("total_tokens: 150"));
}

#[cfg(feature = "gpu")]
#[test]
fn test_performance_metrics_with_realistic_values() {
    let metrics = PerformanceMetrics {
        total_requests: 250,
        total_tokens: 50_000,
        total_time_ms: 5_000, // 5 seconds
        gpu_time_ms: 4_500,
        cpu_time_ms: 500,
        avg_batch_size: 16.5,
        cache_hits: 180,
        cache_misses: 70,
    };

    // Should calculate realistic tokens per second
    let tps = metrics.tokens_per_second();
    assert_eq!(tps, 10_000.0); // 50k tokens / 5 seconds

    // Verify all fields are preserved
    assert_eq!(metrics.total_requests, 250);
    assert_eq!(metrics.gpu_time_ms, 4_500);
    assert_eq!(metrics.cpu_time_ms, 500);
    assert_eq!(metrics.avg_batch_size, 16.5);

    // Verify cache statistics
    assert_eq!(metrics.cache_hits, 180);
    assert_eq!(metrics.cache_misses, 70);
}

#[cfg(feature = "gpu")]
#[test]
fn test_performance_metrics_accumulation_simulation() {
    let mut metrics = PerformanceMetrics::default();

    // Simulate accumulating metrics over multiple requests
    metrics.total_requests += 1;
    metrics.total_tokens += 100;
    metrics.total_time_ms += 50;
    metrics.gpu_time_ms += 45;
    metrics.cpu_time_ms += 5;
    metrics.cache_hits += 1;

    assert_eq!(metrics.total_requests, 1);
    assert_eq!(metrics.total_tokens, 100);
    assert_eq!(metrics.tokens_per_second(), 2000.0); // 100 tokens / 0.05 seconds

    // Add another request
    metrics.total_requests += 1;
    metrics.total_tokens += 150;
    metrics.total_time_ms += 75;
    metrics.cache_misses += 1;

    assert_eq!(metrics.total_requests, 2);
    assert_eq!(metrics.total_tokens, 250);
    assert_eq!(metrics.total_time_ms, 125);
    assert_eq!(metrics.tokens_per_second(), 2000.0); // 250 tokens / 0.125 seconds
    assert_eq!(metrics.cache_hits, 1);
    assert_eq!(metrics.cache_misses, 1);
}

#[cfg(feature = "gpu")]
#[test]
fn test_performance_metrics_timing_precision() {
    // Test with very precise timing measurements
    let metrics = PerformanceMetrics {
        total_tokens: 1,
        total_time_ms: 1,
        ..Default::default()
    };

    assert_eq!(metrics.tokens_per_second(), 1000.0);

    // Test fractional calculations
    let metrics_fractional = PerformanceMetrics {
        total_tokens: 333,
        total_time_ms: 111,
        ..Default::default()
    };

    let tps = metrics_fractional.tokens_per_second();
    assert!((tps - 3000.0).abs() < 0.1); // Should be ~3000 tokens/second
}

#[cfg(feature = "gpu")]
#[test]
fn test_performance_metrics_gpu_vs_cpu_time() {
    let metrics = PerformanceMetrics {
        total_time_ms: 1000,
        gpu_time_ms: 800,
        cpu_time_ms: 200,
        ..Default::default()
    };

    // Verify GPU and CPU time components
    assert_eq!(
        metrics.gpu_time_ms + metrics.cpu_time_ms,
        metrics.total_time_ms
    );

    // Calculate GPU utilization ratio
    let gpu_ratio = metrics.gpu_time_ms as f32 / metrics.total_time_ms as f32;
    assert_eq!(gpu_ratio, 0.8);

    let cpu_ratio = metrics.cpu_time_ms as f32 / metrics.total_time_ms as f32;
    assert_eq!(cpu_ratio, 0.2);
}

#[cfg(feature = "gpu")]
#[test]
fn test_performance_metrics_cache_statistics() {
    let metrics = PerformanceMetrics {
        cache_hits: 85,
        cache_misses: 15,
        ..Default::default()
    };

    let total_cache_requests = metrics.cache_hits + metrics.cache_misses;
    assert_eq!(total_cache_requests, 100);

    let hit_rate = metrics.cache_hits as f32 / total_cache_requests as f32;
    assert_eq!(hit_rate, 0.85);

    let miss_rate = metrics.cache_misses as f32 / total_cache_requests as f32;
    assert_eq!(miss_rate, 0.15);
}

#[cfg(feature = "gpu")]
#[test]
fn test_performance_metrics_batch_size_tracking() {
    let metrics = PerformanceMetrics {
        total_requests: 10,
        avg_batch_size: 8.5,
        ..Default::default()
    };

    // Estimate total items processed
    let estimated_total_items = metrics.total_requests as f32 * metrics.avg_batch_size;
    assert_eq!(estimated_total_items, 85.0);

    // Test various batch sizes
    let small_batch_metrics = PerformanceMetrics {
        avg_batch_size: 1.0,
        ..Default::default()
    };
    assert_eq!(small_batch_metrics.avg_batch_size, 1.0);

    let large_batch_metrics = PerformanceMetrics {
        avg_batch_size: 64.0,
        ..Default::default()
    };
    assert_eq!(large_batch_metrics.avg_batch_size, 64.0);
}
