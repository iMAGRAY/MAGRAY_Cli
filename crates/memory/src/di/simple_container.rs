//! Упрощенный DI контейнер для восстановления компиляции проекта
//!
//! КРИТИЧЕСКОЕ ИСПРАВЛЕНИЕ: Заменяет сложный UnifiedContainer на простую реализацию
//! - Убирает циклическую сложность 97+
//! - Фиксит ошибки компиляции Clone/trait bounds
//! - Сохраняет только необходимую функциональность
//!
//! АРХИТЕКТУРНЫЕ ПРИНЦИПЫ:
//! - Простота превыше всего
//! - Минимальная функциональность для работы
//! - Легкое расширение в будущем

use anyhow::Result;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Упрощенная ошибка DI контейнера (совместимая со старым DIError)
#[derive(Debug, thiserror::Error, Clone)]
pub enum SimpleDIError {
    #[error("Service not found: {service_type}")]
    ServiceNotFound { service_type: String },

    #[error("Registration failed: {message}")]
    RegistrationFailed { message: String },

    #[error("Container lock error: {message}")]
    LockError { message: String },

    // Совместимость со старым DIError
    #[error("Coordinator error: {message}")]
    Coordinator {
        message: String,
        coordinator_type: String,
        operation: String,
    },

    #[error("Lifecycle error during {operation}: {message}")]
    Lifecycle {
        message: String,
        operation: String,
        coordinator_type: Option<String>,
    },

    #[error("Invalid state: {message}")]
    InvalidState { message: String },

    // Дополнительные варианты для полной совместимости
    #[error("Factory error: {message}")]
    Factory {
        message: String,
        factory_type: String,
    },

    #[error("Dependency validation error: {message}")]
    DependencyValidation {
        message: String,
        dependency_chain: Vec<String>,
    },
}

/// Базовый trait для DI контейнера
pub trait DIContainer: Send + Sync {
    /// Получить сервис по типу
    fn resolve<T: 'static + Send + Sync>(&self) -> Result<Arc<T>, SimpleDIError>;

    /// Зарегистрировать сервис
    fn register<T: 'static + Send + Sync>(&self, instance: T) -> Result<(), SimpleDIError>;

    /// Проверить, зарегистрирован ли сервис
    fn contains<T: 'static + Send + Sync>(&self) -> bool;
}

/// Простая реализация DI контейнера
#[derive(Debug)]
pub struct SimpleDIContainer {
    services: Arc<RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>>,
    name: String,
}

impl SimpleDIContainer {
    /// Создать новый контейнер
    pub fn new(name: String) -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            name,
        }
    }

    /// Создать контейнер по умолчанию
    pub fn default() -> Self {
        Self::new("default".to_string())
    }

    /// Получить количество зарегистрированных сервисов
    pub fn service_count(&self) -> usize {
        match self.services.read() {
            Ok(guard) => guard.len(),
            Err(_) => {
                // В случае poison lock возвращаем 0 - безопасно для статистики
                // Логируем проблему но не паникуем
                eprintln!("Warning: DI Container lock is poisoned, returning 0 service count");
                0
            }
        }
    }

    /// Получить имя контейнера
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl DIContainer for SimpleDIContainer {
    fn resolve<T: 'static + Send + Sync>(&self) -> Result<Arc<T>, SimpleDIError> {
        let type_id = TypeId::of::<T>();
        let services = self.services.read().map_err(|e| SimpleDIError::LockError {
            message: format!("Failed to acquire read lock: {}", e),
        })?;

        let any_service = services
            .get(&type_id)
            .ok_or_else(|| SimpleDIError::ServiceNotFound {
                service_type: std::any::type_name::<T>().to_string(),
            })?;

        let service =
            any_service
                .clone()
                .downcast::<T>()
                .map_err(|_| SimpleDIError::ServiceNotFound {
                    service_type: format!("Failed to downcast {}", std::any::type_name::<T>()),
                })?;

        Ok(service)
    }

    fn register<T: 'static + Send + Sync>(&self, instance: T) -> Result<(), SimpleDIError> {
        let type_id = TypeId::of::<T>();
        let arc_instance = Arc::new(instance);

        let mut services = self
            .services
            .write()
            .map_err(|e| SimpleDIError::LockError {
                message: format!("Failed to acquire write lock: {}", e),
            })?;

        services.insert(type_id, arc_instance);
        Ok(())
    }

    fn contains<T: 'static + Send + Sync>(&self) -> bool {
        let type_id = TypeId::of::<T>();
        match self.services.read() {
            Ok(services) => services.contains_key(&type_id),
            Err(_) => {
                // В случае poison lock считаем что сервис отсутствует - безопасно
                eprintln!("Warning: DI Container lock is poisoned, assuming service not found");
                false
            }
        }
    }
}

impl Clone for SimpleDIContainer {
    fn clone(&self) -> Self {
        Self {
            services: Arc::clone(&self.services),
            name: format!("{}_clone", self.name),
        }
    }
}

/// Статистика контейнера для диагностики
#[derive(Debug, Clone)]
pub struct ContainerStats {
    pub name: String,
    pub service_count: usize,
    pub total_resolutions: u64,
    pub failed_resolutions: u64,
    // Поля для совместимости со старым API
    pub registered_factories: u64,
    pub cached_singletons: u64,
    pub cache_hits: u64,
    pub validation_errors: u64,
}

impl Default for ContainerStats {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            service_count: 0,
            total_resolutions: 0,
            failed_resolutions: 0,
            registered_factories: 0,
            cached_singletons: 0,
            cache_hits: 0,
            validation_errors: 0,
        }
    }
}

impl SimpleDIContainer {
    /// Получить статистику контейнера
    pub fn stats(&self) -> ContainerStats {
        ContainerStats {
            name: self.name.clone(),
            service_count: self.service_count(),
            total_resolutions: 0, // Для простоты не отслеживаем
            failed_resolutions: 0,
            // Заглушки для совместимости
            registered_factories: 0,
            cached_singletons: 0,
            cache_hits: 0,
            validation_errors: 0,
        }
    }
}

// Расширения для RwLock для восстановления от poison
trait RwLockExt<T> {
    fn clear_poison(&self);
}

impl<T> RwLockExt<T> for RwLock<T> {
    fn clear_poison(&self) {
        // В случае poison lock, создаем новый - безопасно для наших данных
        if self.is_poisoned() {
            // Для RwLock нет публичного способа очистки poison,
            // но мы можем продолжить работу
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestService {
        value: i32,
    }

    #[test]
    fn test_register_and_resolve() {
        let container = SimpleDIContainer::new("test".to_string());

        let service = TestService { value: 42 };
        container
            .register(service)
            .expect("Service registration should succeed");

        let resolved = container
            .resolve::<TestService>()
            .expect("Service resolution should succeed");
        assert_eq!(resolved.value, 42);
    }

    #[test]
    fn test_service_not_found() {
        let container = SimpleDIContainer::new("test".to_string());

        let result = container.resolve::<TestService>();
        assert!(result.is_err());

        match result.unwrap_err() {
            SimpleDIError::ServiceNotFound { .. } => {}
            _ => panic!("Expected ServiceNotFound error"),
        }
    }

    #[test]
    fn test_contains() {
        let container = SimpleDIContainer::new("test".to_string());

        assert!(!container.contains::<TestService>());

        container
            .register(TestService { value: 42 })
            .expect("Service registration should succeed");

        assert!(container.contains::<TestService>());
    }

    #[test]
    fn test_clone() {
        let container = SimpleDIContainer::new("original".to_string());
        container
            .register(TestService { value: 42 })
            .expect("Service registration should succeed");

        let cloned = container.clone();
        assert_eq!(cloned.name(), "original_clone");
        assert!(cloned.contains::<TestService>());

        let resolved = cloned
            .resolve::<TestService>()
            .expect("Service resolution should succeed");
        assert_eq!(resolved.value, 42);
    }

    #[test]
    fn test_stats() {
        let container = SimpleDIContainer::new("stats_test".to_string());
        container
            .register(TestService { value: 42 })
            .expect("Service registration should succeed");

        let stats = container.stats();
        assert_eq!(stats.name, "stats_test");
        assert_eq!(stats.service_count, 1);
    }
}
