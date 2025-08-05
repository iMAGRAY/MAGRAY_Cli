use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info};

/// Metrics collector for the memory system
// @component: {"k":"C","id":"metrics_collector","t":"Memory system metrics","m":{"cur":85,"tgt":95,"u":"%"},"f":["metrics","monitoring"]}
pub struct MetricsCollector {
    metrics: Arc<RwLock<MemoryMetrics>>,
    start_time: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LatencyMetrics {
    pub count: u64,
    pub sum_ms: f64,
    pub p50_ms: f64,
    pub p90_ms: f64,
    pub p99_ms: f64,
}

impl LatencyMetrics {
    pub fn record(&mut self, ms: f64) {
        self.count += 1;
        self.sum_ms += ms;
        // Simplified percentiles - in production would use a proper histogram
        self.p50_ms = ms;
        self.p90_ms = ms;
        self.p99_ms = ms;
    }
    
    pub fn avg_ms(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum_ms / self.count as f64
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LayerMetrics {
    pub record_count: u64,
    pub total_size_bytes: u64,
    pub avg_embedding_size: f32,
    pub avg_access_count: f32,
    pub oldest_record_age_hours: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryMetrics {
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_entries: u64,
    pub cache_size_bytes: u64,
    pub cache_evictions: u64,
    pub total_embeddings: u64,
    pub vector_searches: u64,
    pub vector_inserts: u64,
    pub vector_deletes: u64,
    pub total_operations: u64,
    pub vector_search_latency_ms: LatencyMetrics,
    pub vector_insert_latency_ms: LatencyMetrics,
    pub promotions_completed: u64,
    pub promotions_interact_to_insights: u64,
    pub promotions_insights_to_assets: u64,
    pub records_expired: u64,
    pub promotion_cycle_duration_ms: LatencyMetrics,
    pub layer_sizes: HashMap<String, LayerMetrics>,
    pub memory_usage_bytes: u64,
    pub avg_response_time_ms: f64,
    pub error_count: u64,
    pub last_error: Option<String>,
    pub uptime_seconds: u64,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(MemoryMetrics::default())),
            start_time: Instant::now(),
        }
    }
    
    /// Record a vector search operation
    pub fn record_vector_search(&self, duration: Duration) {
        let mut metrics = self.metrics.write();
        metrics.vector_searches += 1;
        metrics.total_operations += 1;
        metrics.vector_search_latency_ms.record(duration.as_secs_f64() * 1000.0);
    }
    
    /// Record a vector insert operation
    pub fn record_vector_insert(&self, duration: Duration) {
        let mut metrics = self.metrics.write();
        metrics.vector_inserts += 1;
        metrics.total_operations += 1;
        metrics.vector_insert_latency_ms.record(duration.as_secs_f64() * 1000.0);
    }
    
    /// Record a vector delete operation
    pub fn record_vector_delete(&self) {
        let mut metrics = self.metrics.write();
        metrics.vector_deletes += 1;
        metrics.total_operations += 1;
    }
    
    /// Record cache hit
    pub fn record_cache_hit(&self) {
        let mut metrics = self.metrics.write();
        metrics.cache_hits += 1;
    }
    
    /// Record cache miss
    pub fn record_cache_miss(&self) {
        let mut metrics = self.metrics.write();
        metrics.cache_misses += 1;
    }
    
    /// Record cache eviction
    pub fn record_cache_eviction(&self, count: u64) {
        let mut metrics = self.metrics.write();
        metrics.cache_evictions += count;
    }
    
    /// Update cache stats
    pub fn update_cache_stats(&self, entries: u64, size_bytes: u64) {
        let mut metrics = self.metrics.write();
        metrics.cache_entries = entries;
        metrics.cache_size_bytes = size_bytes;
    }
    
    /// Record promotion
    pub fn record_promotion(&self, from_layer: &str, to_layer: &str, count: u64) {
        let mut metrics = self.metrics.write();
        match (from_layer, to_layer) {
            ("interact", "insights") => metrics.promotions_interact_to_insights += count,
            ("insights", "assets") => metrics.promotions_insights_to_assets += count,
            _ => {}
        }
    }
    
    /// Record expired records
    pub fn record_expired(&self, count: u64) {
        let mut metrics = self.metrics.write();
        metrics.records_expired += count;
    }
    
    /// Record promotion cycle duration
    pub fn record_promotion_cycle(&self, duration: Duration) {
        let mut metrics = self.metrics.write();
        metrics.promotion_cycle_duration_ms.record(duration.as_secs_f64() * 1000.0);
    }
    
    /// Update layer metrics
    pub fn update_layer_metrics(&self, layer: &str, metrics: LayerMetrics) {
        let mut m = self.metrics.write();
        m.layer_sizes.insert(layer.to_string(), metrics);
    }
    
    /// Record an error
    pub fn record_error(&self, error: String) {
        let mut metrics = self.metrics.write();
        metrics.error_count += 1;
        metrics.last_error = Some(error);
    }
    
    /// Record a batch operation
    pub fn record_batch_operation(&self, operation_type: &str, record_count: usize, duration: Duration) {
        match operation_type {
            "batch_insert" => {
                // Записываем каждую вставку как отдельную операцию с усредненным временем
                let avg_duration = duration / record_count as u32;
                for _ in 0..record_count {
                    self.record_vector_insert(avg_duration);
                }
            }
            "batch_search" => {
                // Записываем каждый поиск как отдельную операцию
                let avg_duration = duration / record_count as u32;
                for _ in 0..record_count {
                    self.record_vector_search(avg_duration);
                }
            }
            _ => {
                // Для других типов batch операций просто увеличиваем total_operations
                let mut metrics = self.metrics.write();
                metrics.total_operations += record_count as u64;
            }
        }
        
        debug!("Batch operation '{}': {} records in {:?}", operation_type, record_count, duration);
    }
    
    /// Get current metrics snapshot
    pub fn snapshot(&self) -> MemoryMetrics {
        let mut metrics = self.metrics.read().clone();
        metrics.uptime_seconds = self.start_time.elapsed().as_secs();
        metrics
    }
    
    /// Export metrics in Prometheus format
    pub fn export_prometheus(&self) -> String {
        let metrics = self.snapshot();
        let mut output = String::new();
        
        // Vector metrics
        output.push_str("# HELP memory_vector_searches_total Total number of vector searches\n");
        output.push_str("# TYPE memory_vector_searches_total counter\n");
        output.push_str(&format!("memory_vector_searches_total {}\n", metrics.vector_searches));
        
        output.push_str("# HELP memory_vector_search_latency_ms Vector search latency in milliseconds\n");
        output.push_str("# TYPE memory_vector_search_latency_ms histogram\n");
        output.push_str(&format!("memory_vector_search_latency_ms_sum {}\n", metrics.vector_search_latency_ms.sum_ms));
        output.push_str(&format!("memory_vector_search_latency_ms_count {}\n", metrics.vector_search_latency_ms.count));
        output.push_str(&format!("memory_vector_search_latency_ms{{quantile=\"0.5\"}} {}\n", metrics.vector_search_latency_ms.p50_ms));
        output.push_str(&format!("memory_vector_search_latency_ms{{quantile=\"0.9\"}} {}\n", metrics.vector_search_latency_ms.p90_ms));
        output.push_str(&format!("memory_vector_search_latency_ms{{quantile=\"0.99\"}} {}\n", metrics.vector_search_latency_ms.p99_ms));
        
        // Cache metrics
        output.push_str("# HELP memory_cache_hits_total Total number of cache hits\n");
        output.push_str("# TYPE memory_cache_hits_total counter\n");
        output.push_str(&format!("memory_cache_hits_total {}\n", metrics.cache_hits));
        
        output.push_str("# HELP memory_cache_hit_rate Cache hit rate\n");
        output.push_str("# TYPE memory_cache_hit_rate gauge\n");
        let hit_rate = if metrics.cache_hits + metrics.cache_misses > 0 {
            metrics.cache_hits as f64 / (metrics.cache_hits + metrics.cache_misses) as f64
        } else {
            0.0
        };
        output.push_str(&format!("memory_cache_hit_rate {hit_rate}\n"));
        
        // Layer metrics
        for (layer, layer_metrics) in &metrics.layer_sizes {
            output.push_str("# HELP memory_layer_record_count Number of records in layer\n");
            output.push_str("# TYPE memory_layer_record_count gauge\n");
            output.push_str(&format!("memory_layer_record_count{{layer=\"{}\"}} {}\n", layer, layer_metrics.record_count));
            
            output.push_str(&format!("memory_layer_size_bytes{{layer=\"{}\"}} {}\n", layer, layer_metrics.total_size_bytes));
        }
        
        // System metrics
        output.push_str("# HELP memory_uptime_seconds Uptime in seconds\n");
        output.push_str("# TYPE memory_uptime_seconds gauge\n");
        output.push_str(&format!("memory_uptime_seconds {}\n", metrics.uptime_seconds));
        
        output.push_str("# HELP memory_errors_total Total number of errors\n");
        output.push_str("# TYPE memory_errors_total counter\n");
        output.push_str(&format!("memory_errors_total {}\n", metrics.error_count));
        
        output
    }
    
    /// Log current metrics summary
    pub fn log_summary(&self) {
        let metrics = self.snapshot();
        
        info!("=== Memory System Metrics Summary ===");
        info!("Uptime: {} seconds", metrics.uptime_seconds);
        info!("Total operations: {}", metrics.total_operations);
        
        info!("Vector operations:");
        info!("  Searches: {} (avg: {:.2}ms, p99: {:.2}ms)", 
            metrics.vector_searches, 
            metrics.vector_search_latency_ms.avg_ms(),
            metrics.vector_search_latency_ms.p99_ms
        );
        info!("  Inserts: {} (avg: {:.2}ms)", 
            metrics.vector_inserts, 
            metrics.vector_insert_latency_ms.avg_ms()
        );
        
        let cache_total = metrics.cache_hits + metrics.cache_misses;
        let hit_rate = if cache_total > 0 {
            (metrics.cache_hits as f64 / cache_total as f64) * 100.0
        } else {
            0.0
        };
        info!("Cache performance:");
        info!("  Hit rate: {:.1}% ({} hits, {} misses)", 
            hit_rate, metrics.cache_hits, metrics.cache_misses
        );
        info!("  Entries: {}, Size: {} bytes", 
            metrics.cache_entries, metrics.cache_size_bytes
        );
        
        info!("Promotion stats:");
        info!("  Interact → Insights: {}", metrics.promotions_interact_to_insights);
        info!("  Insights → Assets: {}", metrics.promotions_insights_to_assets);
        info!("  Expired records: {}", metrics.records_expired);
        
        if metrics.error_count > 0 {
            info!("Errors: {} (last: {:?})", metrics.error_count, metrics.last_error);
        }
    }
}

/// Helper to measure operation duration
pub struct TimedOperation<'a> {
    collector: &'a MetricsCollector,
    operation: &'static str,
    start: Instant,
}

impl<'a> TimedOperation<'a> {
    pub fn new(collector: &'a MetricsCollector, operation: &'static str) -> Self {
        Self {
            collector,
            operation,
            start: Instant::now(),
        }
    }
}

impl<'a> Drop for TimedOperation<'a> {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        match self.operation {
            "vector_search" => self.collector.record_vector_search(duration),
            "vector_insert" => self.collector.record_vector_insert(duration),
            _ => {}
        }
        
        if duration.as_millis() > 100 {
            debug!("Slow operation {}: {}ms", self.operation, duration.as_millis());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[test]
    fn test_metrics_collection() {
        let collector = MetricsCollector::new();
        
        // Record some operations
        collector.record_vector_search(Duration::from_millis(10));
        collector.record_vector_search(Duration::from_millis(20));
        collector.record_vector_insert(Duration::from_millis(5));
        
        collector.record_cache_hit();
        collector.record_cache_hit();
        collector.record_cache_miss();
        
        let metrics = collector.snapshot();
        
        assert_eq!(metrics.vector_searches, 2);
        assert_eq!(metrics.vector_inserts, 1);
        assert_eq!(metrics.cache_hits, 2);
        assert_eq!(metrics.cache_misses, 1);
        assert_eq!(metrics.total_operations, 3);
        
        // Check latency metrics
        assert_eq!(metrics.vector_search_latency_ms.count, 2);
        assert!((metrics.vector_search_latency_ms.avg_ms - 15.0).abs() < 0.1);
    }
    
    #[test]
    fn test_prometheus_export() {
        let collector = MetricsCollector::new();
        collector.record_vector_search(Duration::from_millis(10));
        collector.record_cache_hit();
        
        let prometheus = collector.export_prometheus();
        
        assert!(prometheus.contains("memory_vector_searches_total 1"));
        assert!(prometheus.contains("memory_cache_hits_total 1"));
    }
}