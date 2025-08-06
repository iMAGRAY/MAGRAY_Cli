//! Layer Orchestrator - Координация между всеми слоями
//!
//! LayerOrchestrator и LayeredDIContainer обеспечивают единую точку доступа
//! ко всем слоям архитектуры с circuit breaker patterns и monitoring.

use anyhow::Result;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, debug};
use chrono::{DateTime, Utc};

use crate::layers::{
    StorageLayer, IndexLayer, QueryLayer, CacheLayer,
    LayerHealth, LayerHealthStatus,
};

/// Главный DI контейнер для слоевой архитектуры
pub struct LayeredDIContainer {
    storage_layer: Arc<dyn StorageLayer>,
    index_layer: Arc<dyn IndexLayer>, 
    query_layer: Arc<dyn QueryLayer>,
    cache_layer: Arc<dyn CacheLayer>,
    orchestrator: Arc<LayerOrchestrator>,
}

/// Orchestrator для координации всех слоев
pub struct LayerOrchestrator {
    storage_layer: Arc<dyn StorageLayer>,
    index_layer: Arc<dyn IndexLayer>,
    query_layer: Arc<dyn QueryLayer>, 
    cache_layer: Arc<dyn CacheLayer>,
    metrics: Arc<RwLock<LayeredMetrics>>,
    circuit_breakers: Arc<RwLock<HashMap<String, SimpleCircuitBreaker>>>,
}

/// Метрики для всех слоев
#[derive(Debug, Default)]
pub struct LayeredMetrics {
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub layer_health_scores: HashMap<String, f64>,
    pub last_health_check: Option<DateTime<Utc>>,
}

/// Простой circuit breaker для демонстрации
#[derive(Debug)]
pub struct SimpleCircuitBreaker {
    failure_count: u32,
    failure_threshold: u32,
    is_open: bool,
    last_failure: Option<DateTime<Utc>>,
}

impl LayeredDIContainer {
    pub fn new(
        storage_layer: Arc<dyn StorageLayer>,
        index_layer: Arc<dyn IndexLayer>,
        query_layer: Arc<dyn QueryLayer>,
        cache_layer: Arc<dyn CacheLayer>,
        orchestrator: Arc<LayerOrchestrator>,
    ) -> Self {
        Self {
            storage_layer,
            index_layer,
            query_layer,
            cache_layer,
            orchestrator,
        }
    }

    /// Получить storage layer
    pub fn storage(&self) -> &Arc<dyn StorageLayer> {
        &self.storage_layer
    }

    /// Получить index layer
    pub fn index(&self) -> &Arc<dyn IndexLayer> {
        &self.index_layer
    }

    /// Получить query layer
    pub fn query(&self) -> &Arc<dyn QueryLayer> {
        &self.query_layer
    }

    /// Получить cache layer
    pub fn cache(&self) -> &Arc<dyn CacheLayer> {
        &self.cache_layer
    }

    /// Получить orchestrator
    pub fn orchestrator(&self) -> &Arc<LayerOrchestrator> {
        &self.orchestrator
    }

    /// Health check всех слоев
    pub async fn health_check(&self) -> Result<HashMap<String, LayerHealthStatus>> {
        let mut health_statuses = HashMap::new();
        
        if let Ok(status) = self.storage_layer.health_check().await {
            health_statuses.insert("storage".to_string(), status);
        }
        
        if let Ok(status) = self.index_layer.health_check().await {
            health_statuses.insert("index".to_string(), status);
        }
        
        if let Ok(status) = self.query_layer.health_check().await {
            health_statuses.insert("query".to_string(), status);
        }
        
        if let Ok(status) = self.cache_layer.health_check().await {
            health_statuses.insert("cache".to_string(), status);
        }

        Ok(health_statuses)
    }

    /// Инициализация всех слоев
    pub async fn initialize(&self) -> Result<()> {
        info!("🚀 Инициализация слоевой архитектуры");
        
        // Инициализируем слои в правильном порядке зависимостей
        self.orchestrator.initialize().await?;
        
        info!("✅ Слоевая архитектура инициализирована");
        Ok(())
    }
}

impl LayerOrchestrator {
    pub async fn new(
        storage_layer: Arc<dyn StorageLayer>,
        index_layer: Arc<dyn IndexLayer>,
        query_layer: Arc<dyn QueryLayer>,
        cache_layer: Arc<dyn CacheLayer>,
    ) -> Result<Arc<Self>> {
        info!("🎭 Инициализация Layer Orchestrator");
        
        let orchestrator = Arc::new(Self {
            storage_layer,
            index_layer,
            query_layer,
            cache_layer,
            metrics: Arc::new(RwLock::new(LayeredMetrics::default())),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
        });

        // Инициализируем circuit breakers для каждого слоя
        orchestrator.initialize_circuit_breakers().await?;
        
        info!("✅ Layer Orchestrator инициализирован");
        Ok(orchestrator)
    }

    async fn initialize_circuit_breakers(&self) -> Result<()> {
        let mut breakers = self.circuit_breakers.write().await;
        
        for layer_name in ["storage", "index", "query", "cache"] {
            breakers.insert(layer_name.to_string(), SimpleCircuitBreaker::new());
        }
        
        debug!("🔌 Circuit breakers инициализированы для всех слоев");
        Ok(())
    }

    pub async fn initialize(&self) -> Result<()> {
        info!("⚡ Запуск orchestrator инициализации...");
        
        // Проверяем готовность всех слоев
        let storage_ready = self.storage_layer.ready_check().await?;
        let index_ready = self.index_layer.ready_check().await?;
        let query_ready = self.query_layer.ready_check().await?;
        let cache_ready = self.cache_layer.ready_check().await?;

        info!("📊 Статус готовности слоев: storage={}, index={}, query={}, cache={}", 
              storage_ready, index_ready, query_ready, cache_ready);

        // Запускаем фоновые задачи мониторинга
        self.start_health_monitoring().await?;

        Ok(())
    }

    async fn start_health_monitoring(&self) -> Result<()> {
        info!("🚑 Запуск health monitoring для всех слоев");
        
        let metrics = Arc::clone(&self.metrics);
        let storage = Arc::clone(&self.storage_layer);
        let index = Arc::clone(&self.index_layer);
        let query = Arc::clone(&self.query_layer);
        let cache = Arc::clone(&self.cache_layer);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                let mut health_scores = HashMap::new();
                
                // Проверяем каждый слой
                if let Ok(health) = storage.health_check().await {
                    health_scores.insert("storage".to_string(), if health.healthy { 1.0 } else { 0.0 });
                }
                
                if let Ok(health) = index.health_check().await {
                    health_scores.insert("index".to_string(), if health.healthy { 1.0 } else { 0.0 });
                }
                
                if let Ok(health) = query.health_check().await {
                    health_scores.insert("query".to_string(), if health.healthy { 1.0 } else { 0.0 });
                }
                
                if let Ok(health) = cache.health_check().await {
                    health_scores.insert("cache".to_string(), if health.healthy { 1.0 } else { 0.0 });
                }

                // Обновляем метрики
                if let Ok(mut metrics) = metrics.try_write() {
                    metrics.layer_health_scores = health_scores;
                    metrics.last_health_check = Some(Utc::now());
                }
            }
        });

        debug!("🚑 Health monitoring запущен");
        Ok(())
    }

    /// Получить текущие метрики
    pub async fn get_metrics(&self) -> LayeredMetrics {
        self.metrics.read().await.clone()
    }
}

impl SimpleCircuitBreaker {
    fn new() -> Self {
        Self {
            failure_count: 0,
            failure_threshold: 5,
            is_open: false,
            last_failure: None,
        }
    }

    #[allow(dead_code)]
    fn check(&mut self) -> bool {
        if self.is_open {
            // Проверяем можем ли восстановиться
            if let Some(last_failure) = self.last_failure {
                if Utc::now().signed_duration_since(last_failure).num_seconds() > 60 {
                    self.is_open = false;
                    self.failure_count = 0;
                    return true;
                }
            }
            return false;
        }
        true
    }

    #[allow(dead_code)]
    fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure = Some(Utc::now());
        
        if self.failure_count >= self.failure_threshold {
            self.is_open = true;
        }
    }

    #[allow(dead_code)]
    fn record_success(&mut self) {
        self.failure_count = 0;
    }
}

impl Clone for LayeredMetrics {
    fn clone(&self) -> Self {
        Self {
            total_operations: self.total_operations,
            successful_operations: self.successful_operations,
            failed_operations: self.failed_operations,
            layer_health_scores: self.layer_health_scores.clone(),
            last_health_check: self.last_health_check,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_creation() {
        let cb = SimpleCircuitBreaker::new();
        assert_eq!(cb.failure_count, 0);
        assert!(!cb.is_open);
        assert_eq!(cb.failure_threshold, 5);
    }

    #[test]
    fn test_layered_metrics_clone() {
        let metrics = LayeredMetrics::default();
        let cloned = metrics.clone();
        assert_eq!(metrics.total_operations, cloned.total_operations);
    }
}