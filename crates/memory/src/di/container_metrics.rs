//! Container metrics для DI контейнера

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Default)]
pub struct ContainerMetrics {
    pub resolutions: AtomicU64,
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
    pub errors: AtomicU64,
}

#[derive(Debug, Default, Clone)]
pub struct DIContainerStats {
    pub registered_factories: u64,
    pub cached_singletons: u64,
    pub total_resolutions: u64,
    pub cache_hits: u64,
    pub validation_errors: u64,
    pub name: String,
    pub service_count: u64,
    pub failed_resolutions: u64,
}

#[derive(Debug, Default, Clone)]
pub struct DIPerformanceMetrics {
    pub resolution_time_avg_ms: f64,
    pub cache_hit_rate: f64,
    pub memory_usage_mb: f64,
}

#[derive(Debug, Default)]
pub struct MetricsCollector {
    metrics: ContainerMetrics,
}

impl ContainerMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_resolution(&self) {
        self.resolutions.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> HashMap<String, u64> {
        let mut stats = HashMap::new();
        stats.insert(
            "resolutions".to_string(),
            self.resolutions.load(Ordering::Relaxed),
        );
        stats.insert(
            "cache_hits".to_string(),
            self.cache_hits.load(Ordering::Relaxed),
        );
        stats.insert(
            "cache_misses".to_string(),
            self.cache_misses.load(Ordering::Relaxed),
        );
        stats.insert("errors".to_string(), self.errors.load(Ordering::Relaxed));
        stats
    }
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn collect_stats(&self) -> DIContainerStats {
        DIContainerStats::default()
    }

    pub fn collect_performance(&self) -> DIPerformanceMetrics {
        DIPerformanceMetrics::default()
    }
}
