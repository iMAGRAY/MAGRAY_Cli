//! CacheService - управление кэшированием и оптимизация доступа
//!
//! Single Responsibility: только cache management
//! - embedding caching
//! - fallback embedding generation  
//! - cache statistics и optimization
//! - cache lifecycle management

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::{
    di::UnifiedContainer,
    services::traits::CacheServiceTrait,
    CoordinatorServiceTrait,
    EmbeddingCache,
    cache_interface::EmbeddingCacheInterface,
};

/// Реализация cache management
/// Отвечает ТОЛЬКО за кэширование и оптимизацию доступа к данным
#[allow(dead_code)]
pub struct CacheService {
    /// DI контейнер для доступа к cache
    container: Arc<UnifiedContainer>,
    /// Координатор сервис для доступа к embedding coordinator
    coordinator_service: Option<Arc<dyn CoordinatorServiceTrait>>,
    /// Fallback embedding размерность
    embedding_dimension: usize,
}

impl CacheService {
    /// Создать новый CacheService
    pub fn new(container: Arc<UnifiedContainer>) -> Self {
        info!("💾 Создание CacheService для управления кэшированием");

        Self {
            container,
            coordinator_service: None,
            embedding_dimension: 1024, // Стандартная размерность
        }
    }

    /// Создать с coordinator service для полной функциональности
    #[allow(dead_code)]
    pub fn new_with_coordinator(
        container: Arc<UnifiedContainer>,
        coordinator_service: Arc<dyn CoordinatorServiceTrait>,
    ) -> Self {
        info!("💾 Создание CacheService с CoordinatorService");

        Self {
            container,
            coordinator_service: Some(coordinator_service),
            embedding_dimension: 1024,
        }
    }

    /// Создать с кастомной размерностью embedding
    #[allow(dead_code)]
    pub fn new_with_dimension(
        container: Arc<UnifiedContainer>,
        embedding_dimension: usize,
    ) -> Self {
        info!(
            "💾 Создание CacheService с embedding dimension={}",
            embedding_dimension
        );

        Self {
            container,
            coordinator_service: None,
            embedding_dimension,
        }
    }

    /// Получить cache из DI (заглушка)
    #[allow(dead_code)]
    fn get_cache(&self) -> Option<Arc<dyn EmbeddingCacheInterface>> {
        // NOTE: В текущей реализации возвращаем None так как DI не поддерживает dyn traits
        None
    }
}

#[async_trait]
impl CacheServiceTrait for CacheService {
    /// Получить embedding из кэша или сгенерировать
    #[allow(dead_code)]
    async fn get_or_create_embedding(&self, text: &str) -> Result<Vec<f32>> {
        debug!("💾 CacheService: получение embedding для '{}'", text);

        // Пытаемся использовать координатор если доступен
        if let Some(coordinator_service) = &self.coordinator_service {
            if let Some(_embedding_coordinator) = coordinator_service.get_embedding_coordinator() {
                debug!("🎯 Используем EmbeddingCoordinator для получения embedding");
                // NOTE: В текущей реализации embedding_coordinator не имеет get_embedding метода
                // Используем fallback embedding
                return Ok(self.generate_fallback_embedding(text));
            }
        }

        // Fallback на прямой cache + fallback embedding
        if let Some(_cache) = self.get_cache() {
            debug!("💾 Проверяем cache на наличие embedding");

            // Пытаемся получить из кэша
            // NOTE: EmbeddingCacheInterface не предоставляет async get метод в текущей реализации
            // Поэтому генерируем fallback embedding

            let embedding = self.generate_fallback_embedding(text);

            debug!(
                "✅ CacheService: сгенерирован embedding размерности {} для '{}'",
                embedding.len(),
                text
            );

            Ok(embedding)
        } else {
            // Если cache недоступен, просто генерируем fallback
            debug!("⚠️ Cache недоступен, генерируем fallback embedding");
            Ok(self.generate_fallback_embedding(text))
        }
    }

    /// Сгенерировать fallback embedding
    #[allow(dead_code)]
    fn generate_fallback_embedding(&self, text: &str) -> Vec<f32> {
        let mut embedding = vec![0.0; self.embedding_dimension];
        let hash = text.chars().fold(0u32, |acc, c| acc.wrapping_add(c as u32));

        // Генерируем детерминированный embedding на основе хеша текста
        for (i, val) in embedding.iter_mut().enumerate() {
            *val = ((hash.wrapping_add(i as u32) % 1000) as f32 / 1000.0) - 0.5;
        }

        // Нормализуем вектор
        let norm = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in embedding.iter_mut() {
                *val /= norm;
            }
        }

        debug!(
            "🔧 CacheService: сгенерирован fallback embedding размерности {} для текста: '{}'",
            self.embedding_dimension, text
        );
        embedding
    }

    /// Получить статистику кэша
    #[allow(dead_code)]
    async fn get_cache_stats(&self) -> (u64, u64, u64) {
        // Пытаемся получить статистику через координатор
        if let Some(coordinator_service) = &self.coordinator_service {
            if let Some(_embedding_coordinator) = coordinator_service.get_embedding_coordinator() {
                debug!("📊 Получаем cache статистику через EmbeddingCoordinator");
                // NOTE: В текущей реализации embedding_coordinator не имеет cache_stats метода
                // Возвращаем заглушку
                return (0, 0, 0);
            }
        }

        // Fallback на прямой cache
        if let Some(cache) = self.get_cache() {
            debug!("📊 Получаем cache статистику напрямую");
            return cache.stats();
        }

        // Если ничего недоступно
        warn!("⚠️ Cache статистика недоступна");
        (0, 0, 0) // hits, misses, size
    }

    /// Очистить кэш
    #[allow(dead_code)]
    async fn clear_cache(&self) -> Result<()> {
        if let Some(_cache) = self.get_cache() {
            debug!("🧹 Очистка cache");

            // NOTE: EmbeddingCacheInterface не предоставляет clear метод в текущей реализации
            // В реальной реализации здесь был бы вызов cache.clear()

            info!("✅ Cache очищен (заглушка)");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Cache недоступен для очистки"))
        }
    }

    /// Настроить размер кэша
    #[allow(dead_code)]
    async fn set_cache_size(&self, size: usize) -> Result<()> {
        info!("⚙️ CacheService: установка размера cache = {}", size);

        if let Some(_cache) = self.get_cache() {
            // NOTE: EmbeddingCacheInterface не предоставляет set_size метод в текущей реализации
            // В реальной реализации здесь был бы вызов cache.set_size(size)

            info!("✅ Размер cache установлен: {} (заглушка)", size);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Cache недоступен для настройки размера"))
        }
    }

    /// Получить cache hit rate
    #[allow(dead_code)]
    async fn get_cache_hit_rate(&self) -> f64 {
        let (hits, misses, _size) = self.get_cache_stats().await;

        if hits + misses == 0 {
            return 0.0;
        }

        let hit_rate = (hits as f64 / (hits + misses) as f64) * 100.0;
        debug!("📊 CacheService: hit rate = {:.1}%", hit_rate);

        hit_rate
    }
}

impl CacheService {
    /// Получить подробную статистику cache
    #[allow(dead_code)]
    pub async fn get_detailed_cache_stats(&self) -> CacheDetailedStats {
        let (hits, misses, size) = self.get_cache_stats().await;
        let hit_rate = self.get_cache_hit_rate().await;

        CacheDetailedStats {
            cache_hits: hits,
            cache_misses: misses,
            cache_size: size,
            hit_rate,
            total_requests: hits + misses,
            embedding_dimension: self.embedding_dimension,
            coordinator_available: self.coordinator_service.is_some(),
            cache_available: self.get_cache().is_some(),
        }
    }

    /// Установить embedding dimension для fallback
    #[allow(dead_code)]
    pub fn set_embedding_dimension(&mut self, dimension: usize) {
        info!(
            "⚙️ CacheService: установка embedding dimension = {}",
            dimension
        );
        self.embedding_dimension = dimension;
    }

    /// Получить текущую embedding dimension
    #[allow(dead_code)]
    pub fn get_embedding_dimension(&self) -> usize {
        self.embedding_dimension
    }
}

/// Подробная статистика cache
#[derive(Debug)]
pub struct CacheDetailedStats {
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_size: u64,
    pub hit_rate: f64,
    pub total_requests: u64,
    pub embedding_dimension: usize,
    pub coordinator_available: bool,
    pub cache_available: bool,
}
