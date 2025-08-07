//! Trait абстракции для специализированных сервисов
//!
//! Эти traits обеспечивают:
//! - Dependency Inversion: зависимость от абстракций
//! - Interface Segregation: минимальные специфические интерфейсы
//! - Testability: легкое mock-тестирование
//! - Extensibility: простое добавление новых реализаций

use crate::{
    di::unified_container::UnifiedDIContainer,
    health::SystemHealthStatus,
    orchestration::{
        EmbeddingCoordinator as EmbeddingCoordinatorImpl, HealthManager, ResourceController,
        SearchCoordinator as SearchCoordinatorImpl,
    },
    types::{Layer, Record, SearchOptions},
};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;

/// Основные операции с памятью (CRUD)
/// Single Responsibility: только базовые операции с данными
#[async_trait]
pub trait CoreMemoryServiceTrait: Send + Sync {
    /// Вставить одну запись
    async fn insert(&self, record: Record) -> Result<()>;

    /// Вставить несколько записей батчем
    async fn insert_batch(&self, records: Vec<Record>) -> Result<()>;

    /// Поиск по запросу
    async fn search(
        &self,
        query: &str,
        layer: Layer,
        options: SearchOptions,
    ) -> Result<Vec<Record>>;

    /// Обновить запись
    async fn update(&self, record: Record) -> Result<()>;

    /// Удалить запись
    async fn delete(&self, id: &uuid::Uuid, layer: Layer) -> Result<()>;

    /// Батчевая вставка с результатами
    async fn batch_insert(
        &self,
        records: Vec<Record>,
    ) -> Result<crate::service_di::BatchInsertResult>;

    /// Батчевый поиск
    async fn batch_search(
        &self,
        queries: Vec<String>,
        layer: Layer,
        options: SearchOptions,
    ) -> Result<crate::service_di::BatchSearchResult>;
}

/// Управление координаторами и их инициализация
/// Single Responsibility: только координация между компонентами
#[async_trait]
pub trait CoordinatorServiceTrait: Send + Sync {
    /// Создать все координаторы
    async fn create_coordinators(&self, container: &UnifiedDIContainer) -> Result<()>;

    /// Инициализировать все координаторы
    async fn initialize_coordinators(&self) -> Result<()>;

    /// Получить embedding coordinator
    fn get_embedding_coordinator(&self) -> Option<Arc<EmbeddingCoordinatorImpl>>;

    /// Получить search coordinator
    fn get_search_coordinator(&self) -> Option<Arc<SearchCoordinatorImpl>>;

    /// Получить health manager
    fn get_health_manager(&self) -> Option<Arc<HealthManager>>;

    /// Получить resource controller
    fn get_resource_controller(&self) -> Option<Arc<ResourceController>>;

    /// Shutdown всех координаторов
    async fn shutdown_coordinators(&self) -> Result<()>;

    /// Подсчитать активные координаторы
    fn count_active_coordinators(&self) -> usize;
}

/// Отказоустойчивость и восстановление после ошибок
/// Single Responsibility: только resilience логика
#[async_trait]
pub trait ResilienceServiceTrait: Send + Sync {
    /// Проверить circuit breaker
    async fn check_circuit_breaker(&self) -> Result<()>;

    /// Записать успешную операцию
    async fn record_successful_operation(&self, duration: Duration);

    /// Записать неудачную операцию
    async fn record_failed_operation(&self, duration: Duration);

    /// Получить статус circuit breaker
    async fn get_circuit_breaker_status(&self) -> bool;

    /// Сбросить circuit breaker
    async fn reset_circuit_breaker(&self) -> Result<()>;

    /// Установить threshold для circuit breaker
    async fn set_failure_threshold(&self, threshold: u32) -> Result<()>;

    /// Получить статистику failures
    async fn get_failure_stats(&self) -> (u32, Duration);
}

/// Мониторинг системы и метрики
/// Single Responsibility: только monitoring и metrics
#[async_trait]
pub trait MonitoringServiceTrait: Send + Sync {
    /// Запустить production мониторинг
    async fn start_production_monitoring(&self) -> Result<()>;

    /// Запустить health мониторинг
    async fn start_health_monitoring(&self) -> Result<()>;

    /// Запустить resource мониторинг
    async fn start_resource_monitoring(&self) -> Result<()>;

    /// Выполнить проверки готовности
    async fn perform_readiness_checks(&self) -> Result<()>;

    /// Получить статистику системы
    async fn get_system_stats(&self) -> crate::service_di::MemorySystemStats;

    /// Получить health status
    async fn check_health(&self) -> Result<SystemHealthStatus>;

    /// Получить production метрики
    async fn get_production_metrics(&self) -> Result<ProductionMetrics>;

    /// Логирование summary
    async fn log_initialization_summary(&self);
}

/// Управление кэшированием и оптимизация доступа
/// Single Responsibility: только cache management
#[async_trait]
pub trait CacheServiceTrait: Send + Sync {
    /// Получить embedding из кэша или сгенерировать
    async fn get_or_create_embedding(&self, text: &str) -> Result<Vec<f32>>;

    /// Сгенерировать fallback embedding
    fn generate_fallback_embedding(&self, text: &str) -> Vec<f32>;

    /// Получить статистику кэша
    async fn get_cache_stats(&self) -> (u64, u64, u64); // hits, misses, size

    /// Очистить кэш
    async fn clear_cache(&self) -> Result<()>;

    /// Настроить размер кэша
    async fn set_cache_size(&self, size: usize) -> Result<()>;

    /// Получить cache hit rate
    async fn get_cache_hit_rate(&self) -> f64;
}

/// Production метрики (вынесено из service_di.rs)
#[derive(Debug, Default, Clone)]
pub struct ProductionMetrics {
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub circuit_breaker_trips: u64,
    pub avg_response_time_ms: f64,
    pub peak_memory_usage: f64,
    pub coordinator_health_scores: std::collections::HashMap<String, f64>,
    pub last_health_check: Option<std::time::Instant>,
}
