//! Пример интеграции простой DI системы
//!
//! Показывает как заменить сложную di/ систему на simple_di
//! Реальные примеры для memory crate

use super::{DIContainer, DIContainerBuilder, ServiceFactory, SimpleConfig, SimpleServiceFactory};
use anyhow::Result;
use std::sync::Arc;

// Примеры сервисов из memory crate

/// Упрощенный Memory Service
#[derive(Debug)]
pub struct MemoryService {
    config: SimpleConfig,
}

impl MemoryService {
    pub fn new(config: SimpleConfig) -> Self {
        Self { config }
    }

    pub fn health_check(&self) -> bool {
        !self.config.service_creation_timeout.is_zero()
    }
}

/// Упрощенный Cache Service
#[derive(Debug)]
pub struct CacheService {
    max_size: usize,
}

impl CacheService {
    pub fn new(max_size: usize) -> Self {
        Self { max_size }
    }

    pub fn get_capacity(&self) -> usize {
        self.max_size
    }
}

impl Default for CacheService {
    fn default() -> Self {
        Self::new(1000)
    }
}

/// Упрощенный Orchestrator Service
#[derive(Debug)]
pub struct OrchestratorService {
    memory: Arc<MemoryService>,
    cache: Arc<CacheService>,
}

impl OrchestratorService {
    pub fn new(memory: Arc<MemoryService>, cache: Arc<CacheService>) -> Self {
        Self { memory, cache }
    }

    pub fn orchestrate(&self) -> String {
        format!(
            "Orchestrating with memory health: {} and cache capacity: {}",
            self.memory.health_check(),
            self.cache.get_capacity()
        )
    }
}

/// Пример создания простого DI контейнера для memory crate
pub fn create_memory_container() -> Result<DIContainer> {
    let config = SimpleConfig::development();

    let container = DIContainerBuilder::new()
        // Основные сервисы как singleton
        .register_singleton({
            let config = config.clone();
            move || Ok(MemoryService::new(config.clone()))
        })
        .add_singleton::<CacheService>()
        // Orchestrator с зависимостями
        .register_singleton({
            let container_clone = DIContainer::new();
            // Временный hack - в реальности используем контейнер из builder
            move || {
                // В реальном коде это будет resolve от основного контейнера
                let memory = Arc::new(MemoryService::new(SimpleConfig::development()));
                let cache = Arc::new(CacheService::default());
                Ok(OrchestratorService::new(memory, cache))
            }
        })
        .build();

    // Регистрируем Orchestrator с правильными зависимостями
    container.register_singleton({
        let container = container.clone();
        move || {
            let memory = container.resolve::<MemoryService>()?;
            let cache = container.resolve::<CacheService>()?;
            Ok(OrchestratorService::new(memory, cache))
        }
    })?;

    Ok(container)
}

/// Расширенный пример с использованием фабрик
pub fn create_advanced_memory_container() -> Result<DIContainer> {
    let container = DIContainer::new();

    // Конфигурация
    let config = SimpleConfig::production();
    container.register_singleton(move || Ok(config.clone()))?;

    // Memory Service с фабрикой
    container.register_singleton(SimpleServiceFactory::create_with_dependency(
        container.clone(),
        |config: Arc<SimpleConfig>| Ok(MemoryService::new((*config).clone())),
    ))?;

    // Cache Service как transient (новый экземпляр каждый раз)
    container.register_transient(|| Ok(CacheService::new(2000)))?;

    // Orchestrator с двумя зависимостями
    container.register_singleton(SimpleServiceFactory::create_with_two_dependencies(
        container.clone(),
        |memory: Arc<MemoryService>, cache: Arc<CacheService>| {
            Ok(OrchestratorService::new(memory, cache))
        },
    ))?;

    Ok(container)
}

/// Демонстрация миграции от сложной di/ системы
pub fn migration_example() -> Result<()> {
    println!("=== СТАРАЯ СЛОЖНАЯ СИСТЕМА ===");
    println!("- unified_container.rs (1358 строк)");
    println!("- 25+ файлов в di/ папке");
    println!("- Сложные trait hierarchies");
    println!("- Service Locator anti-pattern");
    println!("- Избыточные абстракции");

    println!("\n=== НОВАЯ ПРОСТАЯ СИСТЕМА ===");

    // Создаем контейнер
    let container = create_advanced_memory_container()?;

    println!(
        "✓ Простой контейнер: {} сервисов зарегистрировано",
        container.service_count()
    );

    // Разрешаем сервисы
    let memory = container.resolve::<MemoryService>()?;
    println!("✓ Memory Service: health = {}", memory.health_check());

    let cache = container.resolve::<CacheService>()?;
    println!("✓ Cache Service: capacity = {}", cache.get_capacity());

    let orchestrator = container.resolve::<OrchestratorService>()?;
    println!("✓ Orchestrator: {}", orchestrator.orchestrate());

    // Проверяем что singleton'ы работают
    let memory2 = container.resolve::<MemoryService>()?;
    println!("✓ Singleton работает: {}", Arc::ptr_eq(&memory, &memory2));

    // Проверяем что transient'ы работают
    let cache2 = container.resolve::<CacheService>()?;
    println!("✓ Transient работает: {}", !Arc::ptr_eq(&cache, &cache2));

    println!("\n=== ПРЕИМУЩЕСТВА ПРОСТОЙ СИСТЕМЫ ===");
    println!("- Менее 200 строк кода вместо 1000+");
    println!("- Явные зависимости");
    println!("- Простые Arc<T> вместо сложных wrapper'ов");
    println!("- Минимальные абстракции");
    println!("- Легко понять и поддерживать");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_container_creation() {
        let container = create_memory_container().unwrap();

        assert!(container.is_registered::<MemoryService>());
        assert!(container.is_registered::<CacheService>());
        assert!(container.is_registered::<OrchestratorService>());

        let orchestrator = container.resolve::<OrchestratorService>().unwrap();
        let result = orchestrator.orchestrate();
        assert!(result.contains("Orchestrating with memory"));
    }

    #[test]
    fn test_advanced_container() {
        let container = create_advanced_memory_container().unwrap();

        // Проверяем все сервисы
        let config = container.resolve::<SimpleConfig>().unwrap();
        assert_eq!(config.max_services, 2000); // production config

        let memory = container.resolve::<MemoryService>().unwrap();
        assert!(memory.health_check());

        let cache1 = container.resolve::<CacheService>().unwrap();
        let cache2 = container.resolve::<CacheService>().unwrap();
        // Transient - разные экземпляры
        assert!(!Arc::ptr_eq(&cache1, &cache2));
        assert_eq!(cache1.get_capacity(), 2000);

        let orchestrator = container.resolve::<OrchestratorService>().unwrap();
        assert!(!orchestrator.orchestrate().is_empty());
    }

    #[test]
    fn test_migration_demo() {
        // Это не должно падать
        migration_example().unwrap();
    }

    #[test]
    fn test_service_lifecycles() {
        let container = create_advanced_memory_container().unwrap();

        // Singleton services
        let config1 = container.resolve::<SimpleConfig>().unwrap();
        let config2 = container.resolve::<SimpleConfig>().unwrap();
        assert!(Arc::ptr_eq(&config1, &config2));

        let memory1 = container.resolve::<MemoryService>().unwrap();
        let memory2 = container.resolve::<MemoryService>().unwrap();
        assert!(Arc::ptr_eq(&memory1, &memory2));

        // Transient services
        let cache1 = container.resolve::<CacheService>().unwrap();
        let cache2 = container.resolve::<CacheService>().unwrap();
        assert!(!Arc::ptr_eq(&cache1, &cache2));
    }

    #[test]
    fn test_dependency_resolution() {
        let container = create_advanced_memory_container().unwrap();

        let orchestrator = container.resolve::<OrchestratorService>().unwrap();

        // Проверяем что зависимости правильно внедрены
        let result = orchestrator.orchestrate();
        assert!(result.contains("true")); // memory health должен быть true
        assert!(result.contains("2000")); // cache capacity
    }
}
