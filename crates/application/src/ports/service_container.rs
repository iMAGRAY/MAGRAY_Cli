//! Service Container Port
//!
//! Абстракция для Dependency Injection container

use crate::ApplicationResult;
use std::any::Any;
use std::sync::Arc;

/// Trait for Dependency Injection containers
pub trait ServiceContainer: Send + Sync {
    /// Register a singleton service
    fn register_singleton<T>(&mut self, instance: Arc<T>) -> ApplicationResult<()>
    where
        T: Any + Send + Sync + 'static;

    /// Register a transient service factory
    fn register_transient<T, F>(&mut self, factory: F) -> ApplicationResult<()>
    where
        T: Any + Send + Sync + 'static,
        F: Fn() -> Arc<T> + Send + Sync + 'static;

    /// Resolve a service instance
    fn resolve<T>(&self) -> ApplicationResult<Arc<T>>
    where
        T: Any + Send + Sync + 'static;

    /// Check if a service is registered
    fn is_registered<T>(&self) -> bool
    where
        T: Any + Send + Sync + 'static;
}

/// Mock service container for testing
#[cfg(feature = "test-utils")]
pub struct MockServiceContainer;

#[cfg(feature = "test-utils")]
impl ServiceContainer for MockServiceContainer {
    fn register_singleton<T>(&mut self, _instance: Arc<T>) -> ApplicationResult<()>
    where
        T: Any + Send + Sync + 'static,
    {
        Ok(())
    }

    fn register_transient<T, F>(&mut self, _factory: F) -> ApplicationResult<()>
    where
        T: Any + Send + Sync + 'static,
        F: Fn() -> Arc<T> + Send + Sync + 'static,
    {
        Ok(())
    }

    fn resolve<T>(&self) -> ApplicationResult<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        Err(crate::ApplicationError::not_found(
            "Service",
            "not registered",
        ))
    }

    fn is_registered<T>(&self) -> bool
    where
        T: Any + Send + Sync + 'static,
    {
        false
    }
}
