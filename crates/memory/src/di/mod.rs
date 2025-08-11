//! ИСПРАВЛЕНО: Упрощенная DI система для восстановления компиляции
//!
//! КРИТИЧЕСКОЕ ИСПРАВЛЕНИЕ: Заменяем сложную SOLID-архитектуру на простую реализацию
//! - Убирает 21 deleted файл и избыточную сложность
//! - Фиксит все ошибки компиляции с Clone/trait bounds  
//! - Сохраняет только необходимое для работы проекта
//!
//! После восстановления компиляции можно постепенно возвращать сложность,
//! но сейчас приоритет - рабочее состояние проекта.

// Публичный API - простой DI контейнер
pub mod simple_container;

// Re-exports для совместимости с существующим кодом
pub use simple_container::{
    DIContainer,
    SimpleDIContainer as UnifiedContainer, 
    SimpleDIContainer as DIContainerBuilder,
    SimpleDIContainer as StandardContainer, // Для совместимости с CLI
    SimpleDIError as DIError,
    ContainerStats as DIContainerStats,
};

// Заглушки для совместимости
pub use simple_container::ContainerStats as DIPerformanceMetrics;

// ВРЕМЕННО ОТКЛЮЧЕНЫ: старые модули для восстановления компиляции
// После стабилизации можно постепенно включать обратно
// #[allow(dead_code)]
// mod core_traits;
// #[allow(dead_code)]  
// mod unified_container_impl;
// #[allow(dead_code)]
// mod container_metrics_impl;
// #[allow(dead_code)]
// mod dependency_graph_validator;

// Модуль errors для совместимости
pub mod errors {
    pub use super::simple_container::SimpleDIError as DIError;
    
    #[derive(Debug, thiserror::Error, Clone)]
    pub enum ValidationError {
        #[error("Validation error: {message}")]
        Generic { message: String },
        
        #[error("Graph operation failed: {message}")]
        GraphOperationFailed { message: String },
        
        #[error("Circular dependency detected: {chain:?}")]
        CircularDependency { chain: Vec<String> },
    }
}

// Простая реализация Lifetime для совместимости
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Lifetime {
    Transient,
    Singleton,
}

impl Default for Lifetime {
    fn default() -> Self {
        Lifetime::Singleton
    }
}

// Простая фабрика контейнеров
pub struct DIContainerFactory;

impl DIContainerFactory {
    /// Создать новый контейнер с именем
    pub fn create_container(name: String) -> UnifiedContainer {
        UnifiedContainer::new(name)
    }
    
    /// Создать контейнер по умолчанию
    pub fn default_container() -> UnifiedContainer {
        UnifiedContainer::default()
    }
}

// Builder для совместимости
pub struct UnifiedContainerBuilder {
    name: String,
}

impl UnifiedContainerBuilder {
    pub fn new() -> Self {
        Self {
            name: "default".to_string(),
        }
    }
    
    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }
    
    pub fn build(self) -> UnifiedContainer {
        UnifiedContainer::new(self.name)
    }
}

impl Default for UnifiedContainerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[derive(Debug)]
    struct TestService {
        value: i32,
    }
    
    #[test]
    fn test_factory_create() {
        let container = DIContainerFactory::create_container("test".to_string());
        assert_eq!(container.name(), "test");
    }
    
    #[test]
    fn test_builder_pattern() {
        let container = UnifiedContainerBuilder::new()
            .with_name("builder_test".to_string())
            .build();
            
        assert_eq!(container.name(), "builder_test");
    }
    
    #[test]
    fn test_di_integration() {
        let container = DIContainerFactory::default_container();
        
        // Регистрируем сервис
        container.register(TestService { value: 42 }).unwrap();
        
        // Разрешаем сервис
        let service = container.resolve::<TestService>().unwrap();
        assert_eq!(service.value, 42);
        
        // Проверяем статистику
        let stats = container.stats();
        assert_eq!(stats.service_count, 1);
    }
}