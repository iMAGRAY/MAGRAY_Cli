use memory::{MetricsCollector, LayerMetrics};
use std::time::Duration;

#[test]
fn test_metrics_collector_creation() {
    let collector = MetricsCollector::new();
    let metrics = collector.snapshot();
    
    assert_eq!(metrics.vector_searches, 0);
    assert_eq!(metrics.vector_inserts, 0);
    assert_eq!(metrics.vector_deletes, 0);
    assert_eq!(metrics.cache_hits, 0);
    assert_eq!(metrics.cache_misses, 0);
    assert_eq!(metrics.total_operations, 0);
}

#[test]
fn test_record_vector_search() {
    let collector = MetricsCollector::new();
    
    collector.record_vector_search(Duration::from_millis(5));
    collector.record_vector_search(Duration::from_millis(10));
    
    let metrics = collector.snapshot();
    assert_eq!(metrics.vector_searches, 2);
    assert_eq!(metrics.total_operations, 2);
    assert_eq!(metrics.vector_search_latency_ms.count, 2);
    assert!(metrics.vector_search_latency_ms.avg_ms() > 0.0);
}

#[test]
fn test_record_vector_insert() {
    let collector = MetricsCollector::new();
    
    collector.record_vector_insert(Duration::from_millis(15));
    
    let metrics = collector.snapshot();
    assert_eq!(metrics.vector_inserts, 1);
    assert_eq!(metrics.total_operations, 1);
    assert_eq!(metrics.vector_insert_latency_ms.count, 1);
}

#[test]
fn test_record_vector_delete() {
    let collector = MetricsCollector::new();
    
    collector.record_vector_delete();
    collector.record_vector_delete();
    
    let metrics = collector.snapshot();
    assert_eq!(metrics.vector_deletes, 2);
    assert_eq!(metrics.total_operations, 2);
}

#[test]
fn test_cache_metrics() {
    let collector = MetricsCollector::new();
    
    collector.record_cache_hit();
    collector.record_cache_hit();
    collector.record_cache_miss();
    
    let metrics = collector.snapshot();
    assert_eq!(metrics.cache_hits, 2);
    assert_eq!(metrics.cache_misses, 1);
    
    // Проверяем cache hit rate через Prometheus export
    let prometheus = collector.export_prometheus();
    assert!(prometheus.contains("memory_cache_hit_rate 0.6")); // 2/3 = 0.666...
}

#[test]
fn test_cache_eviction() {
    let collector = MetricsCollector::new();
    
    collector.record_cache_eviction(5);
    collector.record_cache_eviction(3);
    
    let metrics = collector.snapshot();
    assert_eq!(metrics.cache_evictions, 8);
}

#[test]
fn test_update_cache_stats() {
    let collector = MetricsCollector::new();
    
    collector.update_cache_stats(100, 1024 * 1024); // 100 entries, 1MB
    
    let metrics = collector.snapshot();
    assert_eq!(metrics.cache_entries, 100);
    assert_eq!(metrics.cache_size_bytes, 1024 * 1024);
}

#[test]
fn test_promotion_metrics() {
    let collector = MetricsCollector::new();
    
    collector.record_promotion("interact", "insights", 3);
    collector.record_promotion("insights", "assets", 1);
    collector.record_promotion("interact", "insights", 2);
    
    let metrics = collector.snapshot();
    assert_eq!(metrics.promotions_interact_to_insights, 5);
    assert_eq!(metrics.promotions_insights_to_assets, 1);
}

#[test]
fn test_record_expired() {
    let collector = MetricsCollector::new();
    
    collector.record_expired(10);
    collector.record_expired(5);
    
    let metrics = collector.snapshot();
    assert_eq!(metrics.records_expired, 15);
}

#[test]
fn test_promotion_cycle_duration() {
    let collector = MetricsCollector::new();
    
    collector.record_promotion_cycle(Duration::from_millis(100));
    collector.record_promotion_cycle(Duration::from_millis(200));
    
    let metrics = collector.snapshot();
    assert_eq!(metrics.promotion_cycle_duration_ms.count, 2);
    assert_eq!(metrics.promotion_cycle_duration_ms.avg_ms(), 150.0);
}

#[test]
fn test_layer_metrics() {
    let layer_metrics = LayerMetrics {
        record_count: 100,
        total_size_bytes: 1024 * 1024, // 1MB
        avg_embedding_size: 768.0,
        avg_access_count: 5.5,
        oldest_record_age_hours: 24.0,
    };
    
    assert_eq!(layer_metrics.record_count, 100);
    assert_eq!(layer_metrics.total_size_bytes, 1024 * 1024);
    assert_eq!(layer_metrics.avg_embedding_size, 768.0);
}

#[test]
fn test_update_layer_metrics() {
    let collector = MetricsCollector::new();
    
    let layer_metrics = LayerMetrics {
        record_count: 50,
        total_size_bytes: 512 * 1024,
        avg_embedding_size: 768.0,
        avg_access_count: 3.0,
        oldest_record_age_hours: 12.0,
    };
    
    collector.update_layer_metrics("interact", layer_metrics);
    
    let metrics = collector.snapshot();
    assert!(metrics.layer_sizes.contains_key("interact"));
    assert_eq!(metrics.layer_sizes["interact"].record_count, 50);
}

#[test]
fn test_error_tracking() {
    let collector = MetricsCollector::new();
    
    collector.record_error("Failed to connect to database".to_string());
    collector.record_error("Timeout during vector search".to_string());
    
    let metrics = collector.snapshot();
    assert_eq!(metrics.error_count, 2);
    assert_eq!(metrics.last_error, Some("Timeout during vector search".to_string()));
}

#[test]
fn test_batch_operations() {
    let collector = MetricsCollector::new();
    
    // Batch insert
    collector.record_batch_operation("batch_insert", 10, Duration::from_millis(100));
    
    let metrics = collector.snapshot();
    assert_eq!(metrics.vector_inserts, 10);
    assert_eq!(metrics.total_operations, 10);
    
    // Batch search
    collector.record_batch_operation("batch_search", 5, Duration::from_millis(50));
    
    let metrics = collector.snapshot();
    assert_eq!(metrics.vector_searches, 5);
    assert_eq!(metrics.total_operations, 15); // 10 + 5
}

#[test]
fn test_prometheus_export() {
    let collector = MetricsCollector::new();
    
    // Add some test data
    collector.record_vector_search(Duration::from_millis(10));
    collector.record_cache_hit();
    collector.record_error("test error".to_string());
    
    // Export as Prometheus format
    let prometheus = collector.export_prometheus();
    
    // Verify format
    assert!(prometheus.contains("# HELP memory_vector_searches_total"));
    assert!(prometheus.contains("memory_vector_searches_total 1"));
    assert!(prometheus.contains("memory_cache_hits_total 1"));
    assert!(prometheus.contains("memory_errors_total 1"));
}

#[test]
fn test_uptime_calculation() {
    let collector = MetricsCollector::new();
    
    // Sleep a bit to have some uptime - longer to ensure uptime is recorded
    std::thread::sleep(Duration::from_millis(1100));
    
    let metrics = collector.snapshot();
    assert!(metrics.uptime_seconds >= 1);
}

#[test]
fn test_concurrent_metric_updates() {
    use std::sync::Arc;
    use std::thread;
    
    let collector = Arc::new(MetricsCollector::new());
    let mut handles = vec![];
    
    // Spawn multiple threads updating metrics
    for _ in 0..10 {
        let collector_clone = Arc::clone(&collector);
        handles.push(thread::spawn(move || {
            for _ in 0..100 {
                collector_clone.record_vector_search(Duration::from_micros(100));
                collector_clone.record_cache_hit();
            }
        }));
    }
    
    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }
    
    let metrics = collector.snapshot();
    assert_eq!(metrics.vector_searches, 1000);
    assert_eq!(metrics.cache_hits, 1000);
}

#[test]
fn test_log_summary() {
    let collector = MetricsCollector::new();
    
    // Add some metrics
    collector.record_vector_search(Duration::from_millis(10));
    collector.record_cache_hit();
    collector.record_promotion("interact", "insights", 1);
    
    // This should not panic
    collector.log_summary();
}

#[test]
fn test_multiple_layer_metrics() {
    let collector = MetricsCollector::new();
    
    // Update metrics for different layers
    collector.update_layer_metrics("interact", LayerMetrics {
        record_count: 100,
        total_size_bytes: 100_000,
        avg_embedding_size: 768.0,
        avg_access_count: 5.0,
        oldest_record_age_hours: 1.0,
    });
    
    collector.update_layer_metrics("insights", LayerMetrics {
        record_count: 50,
        total_size_bytes: 75_000,
        avg_embedding_size: 768.0,
        avg_access_count: 10.0,
        oldest_record_age_hours: 24.0,
    });
    
    collector.update_layer_metrics("assets", LayerMetrics {
        record_count: 200,
        total_size_bytes: 500_000,
        avg_embedding_size: 768.0,
        avg_access_count: 2.0,
        oldest_record_age_hours: 720.0, // 30 days
    });
    
    let metrics = collector.snapshot();
    assert_eq!(metrics.layer_sizes.len(), 3);
    assert_eq!(metrics.layer_sizes["interact"].record_count, 100);
    assert_eq!(metrics.layer_sizes["insights"].record_count, 50);
    assert_eq!(metrics.layer_sizes["assets"].record_count, 200);
}