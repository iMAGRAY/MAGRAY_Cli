//! Заглушка для совместимости со старой di/ системой
//!
//! Минимальная реализация для предотвращения ошибок компиляции
//! в существующих файлах, пока не будет выполнена полная миграция

use anyhow::Result;
use std::sync::Arc;

// Re-export простых типов из simple_di
pub use crate::simple_di::{DIContainer as UnifiedDIContainer, Lifetime};

/// Заглушка для DIResolver trait
pub trait DIResolver {
    fn resolve<T>(&self) -> Result<Arc<T>>
    where
        T: Send + Sync + 'static;
}

/// Заглушка для DIMemoryService
pub struct DIMemoryService;

impl DIMemoryService {
    pub fn new(_config: impl std::fmt::Debug) -> Self {
        Self
    }
}

// Traits module compatibility
pub mod traits {
    pub use super::DIResolver;
}

// Unified container module compatibility
pub mod unified_container {
    pub use super::UnifiedDIContainer;
}

// Container core compatibility
pub mod container_core {
    pub struct ContainerCore;

    impl ContainerCore {
        pub fn new() -> Self {
            Self
        }
    }
}

// Type safe resolver compatibility
pub struct TypeSafeResolver;

impl TypeSafeResolver {
    pub fn new() -> Self {
        Self
    }
}

// Memory configurator compatibility
pub struct UnifiedMemoryConfigurator;

impl UnifiedMemoryConfigurator {
    pub fn new() -> Self {
        Self
    }
}
