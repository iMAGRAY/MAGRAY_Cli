use anyhow::Result;
use async_trait::async_trait;

use crate::{
    backup::BackupMetadata,
    health::SystemHealthStatus,
    ml_promotion::MLPromotionStats,
    promotion::PromotionStats,
    resource_manager::ResourceUsage,
    types::{Layer, Record, SearchOptions},
};

/// Базовый trait для всех координаторов
#[async_trait]
pub trait Coordinator: Send + Sync + std::fmt::Debug {
    /// Инициализация координатора
    async fn initialize(&self) -> Result<()>;

    /// Проверка готовности координатора
    async fn is_ready(&self) -> bool;

    /// Health check для мониторинга
    async fn health_check(&self) -> Result<()>;

    /// Graceful shutdown
    async fn shutdown(&self) -> Result<()>;

    /// Получить метрики координатора
    async fn metrics(&self) -> serde_json::Value;
}

/// Координатор поиска
#[async_trait]
pub trait SearchCoordinator: Coordinator {
    /// Поиск с embedding
    async fn search(
        &self,
        query: &str,
        layer: Layer,
        options: SearchOptions,
    ) -> Result<Vec<Record>>;

    /// Векторный поиск
    async fn vector_search(
        &self,
        vector: &[f32],
        layer: Layer,
        options: SearchOptions,
    ) -> Result<Vec<Record>>;

    /// Гибридный поиск (text + vector)
    async fn hybrid_search(
        &self,
        query: &str,
        vector: Option<&[f32]>,
        layer: Layer,
        options: SearchOptions,
    ) -> Result<Vec<Record>>;

    /// Поиск с reranking
    async fn search_with_rerank(
        &self,
        query: &str,
        layer: Layer,
        options: SearchOptions,
        rerank_top_k: usize,
    ) -> Result<Vec<Record>>;
}

/// Координатор embeddings
#[async_trait]
pub trait EmbeddingCoordinator: Coordinator {
    /// Получить embedding для текста
    async fn get_embedding(&self, text: &str) -> Result<Vec<f32>>;

    /// Batch embeddings
    async fn get_embeddings(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;

    /// Проверить кэш
    async fn check_cache(&self, text: &str) -> Option<Vec<f32>>;

    /// Статистика кэша
    async fn cache_stats(&self) -> (u64, u64, u64); // (hits, misses, size)

    /// Очистить кэш
    async fn clear_cache(&self) -> Result<()>;
}

/// Координатор продвижения записей между слоями
#[async_trait]
pub trait PromotionCoordinator: Coordinator {
    /// Запустить процесс продвижения
    async fn run_promotion(&self) -> Result<PromotionStats>;

    /// Запустить ML-based promotion
    async fn run_ml_promotion(&self) -> Result<Option<MLPromotionStats>>;

    /// Проверить нужно ли запускать promotion
    async fn should_promote(&self) -> bool;

    /// Получить статистику promotion
    async fn promotion_stats(&self) -> PromotionStats;
}

/// Координатор здоровья системы
#[async_trait]
pub trait HealthCoordinator: Coordinator {
    /// Общее состояние системы
    async fn system_health(&self) -> Result<SystemHealthStatus>;

    /// Проверка конкретного компонента
    async fn component_health(&self, component: &str) -> Result<bool>;

    /// Запустить проверку здоровья
    async fn run_health_check(&self) -> Result<()>;

    /// Получить алерты
    async fn get_alerts(&self) -> Vec<String>;

    /// Очистить алерты
    async fn clear_alerts(&self) -> Result<()>;
}

/// Координатор ресурсов
#[async_trait]
pub trait ResourceCoordinator: Coordinator {
    /// Текущее использование ресурсов
    async fn resource_usage(&self) -> ResourceUsage;

    /// Проверить доступность ресурсов для операции
    async fn check_resources(&self, operation: &str) -> Result<bool>;

    /// Адаптировать лимиты на основе системы
    async fn adapt_limits(&self) -> Result<()>;

    /// Принудительно освободить ресурсы
    async fn free_resources(&self) -> Result<()>;

    /// Получить текущие лимиты
    async fn get_limits(&self) -> (usize, usize); // (vectors, cache_mb)
}

/// Координатор резервного копирования
#[async_trait]
pub trait BackupCoordinator: Coordinator {
    /// Создать полный backup
    async fn create_backup(&self, path: &str) -> Result<BackupMetadata>;

    /// Создать инкрементальный backup
    async fn create_incremental_backup(&self, path: &str) -> Result<BackupMetadata>;

    /// Восстановить из backup
    async fn restore_backup(&self, path: &str) -> Result<()>;

    /// Список доступных backup'ов
    async fn list_backups(&self) -> Result<Vec<BackupMetadata>>;

    /// Проверить целостность backup
    async fn verify_backup(&self, path: &str) -> Result<bool>;
}

/// Результат координации для цепочки обработки
#[derive(Debug)]
#[allow(dead_code)] // Для будущего orchestration функционала
pub enum CoordinationResult<T> {
    /// Успешный результат
    Success(T),
    /// Частичный успех с предупреждениями
    PartialSuccess(T, Vec<String>),
    /// Fallback результат
    Fallback(T, String),
    /// Ошибка координации
    Error(anyhow::Error),
}

impl<T> CoordinationResult<T> {
    #[allow(dead_code)] // Для будущего orchestration функционала
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success(_) | Self::PartialSuccess(_, _))
    }

    #[allow(dead_code)] // Для будущего orchestration функционала
    pub fn unwrap(self) -> T {
        match self {
            Self::Success(t) | Self::PartialSuccess(t, _) | Self::Fallback(t, _) => t,
            Self::Error(e) => panic!("Called unwrap on Error: {}", e),
        }
    }

    #[allow(dead_code)] // Для будущего orchestration функционала
    pub fn warnings(&self) -> Vec<String> {
        match self {
            Self::PartialSuccess(_, warnings) => warnings.clone(),
            Self::Fallback(_, reason) => vec![reason.clone()],
            _ => vec![],
        }
    }
}
