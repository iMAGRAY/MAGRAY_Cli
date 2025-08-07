//! Заглушка для совместимости со старой di/ системой
//!
//! Минимальная реализация для предотвращения ошибок компиляции
//! в существующих файлах, пока не будет выполнена полная миграция

use anyhow::Result;
use std::sync::Arc;

/// Заглушка для DIResolver trait
pub trait DIResolver {
    fn resolve<T>(&self) -> Result<Arc<T>>
    where
        T: Send + Sync + 'static;
}

/// Минимальная Legacy конфигурация, ожидаемая CLI
#[derive(Debug, Clone)]
pub struct LegacyMemoryConfig {
    pub health_enabled: bool,
}

impl Default for LegacyMemoryConfig {
    fn default() -> Self {
        Self { health_enabled: true }
    }
}

/// Минимальный статус здоровья для совместимости в минимальной сборке
#[derive(Debug, Clone, Default)]
pub struct SystemHealthStatusStub {
    pub healthy: bool,
}

/// Заглушка для DIMemoryService
pub struct DIMemoryService;

impl DIMemoryService {
    pub async fn new(_config: LegacyMemoryConfig) -> Result<Self> {
        Ok(Self)
    }

    pub async fn initialize(&self) -> Result<()> {
        Ok(())
    }

    pub async fn check_health(&self) -> Result<SystemHealthStatusStub> {
        Ok(SystemHealthStatusStub { healthy: true })
    }
}

// Traits module compatibility
pub mod traits {
    pub use super::DIResolver;
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

// Пустой конфигуратор (не используется в минимальной сборке)
pub struct UnifiedMemoryConfigurator;

impl UnifiedMemoryConfigurator {
    pub fn new() -> Self {
        Self
    }
}
