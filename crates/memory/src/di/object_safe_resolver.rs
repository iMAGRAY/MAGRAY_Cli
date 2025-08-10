//! Object-Safe DI Resolver
//!
//! Решение проблемы E0038: DIResolver trait не dyn-compatible из-за generic методов.
//!
//! АРХИТЕКТУРНОЕ РЕШЕНИЕ:
//! - Type-erased Service Locator с TypeMap для dynamic dispatch
//! - Сохранение compile-time type safety через wrapper API
//! - SOLID принципы: ISP (разделенные интерфейсы), DIP (зависимость от абстракций)
//!
//! PATTERN: Type Erasure + Service Locator (controlled anti-pattern)

use anyhow::{anyhow, Result};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};
use tracing::{debug, error, warn};

/// Object-safe trait для разрешения зависимостей без generics
///
/// Применяет принцип Interface Segregation (ISP) - минимальный интерфейс
/// для object-safe операций
pub trait ObjectSafeResolver: Send + Sync {
    /// Разрешить зависимость по TypeId (type-erased)
    fn resolve_by_type_id(&self, type_id: TypeId) -> Result<Arc<dyn Any + Send + Sync + 'static>>;

    /// Попытаться разрешить зависимость (возвращает None если не найдена)
    fn try_resolve_by_type_id(
        &self,
        type_id: TypeId,
    ) -> Option<Arc<dyn Any + Send + Sync + 'static>>;

    /// Проверить, зарегистрирован ли тип
    fn is_registered_by_type_id(&self, type_id: TypeId) -> bool;

    /// Получить список всех зарегистрированных типов (для отладки)
    fn get_registered_types(&self) -> Vec<TypeId>;

    /// Получить название типа для диагностики
    fn get_type_name(&self, type_id: TypeId) -> Option<String>;
}

/// Type-safe wrapper для Object-safe resolver
///
/// Предоставляет compile-time type safety поверх type-erased resolver
/// Применяет принцип Dependency Inversion (DIP)
pub struct TypeSafeResolver {
    /// Object-safe resolver для actual dependency resolution
    resolver: Arc<dyn ObjectSafeResolver>,
}

impl TypeSafeResolver {
    /// Создать новый type-safe resolver
    pub fn new(resolver: Arc<dyn ObjectSafeResolver>) -> Self {
        debug!("🔧 TypeSafeResolver: создание wrapper для object-safe resolver");
        Self { resolver }
    }

    /// Разрешить зависимость с compile-time type safety
    pub fn resolve<T>(&self) -> Result<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();

        debug!("🔍 TypeSafeResolver: разрешение типа {}", type_name);

        let any_instance = self
            .resolver
            .resolve_by_type_id(type_id)
            .map_err(|e| anyhow!("Не удалось разрешить тип {}: {}", type_name, e))?;

        // Type-safe downcast с proper error handling
        any_instance
            .downcast::<T>()
            .map_err(|_| {
                error!("❌ TypeSafeResolver: тип {} не соответствует ожидаемому", type_name);
                anyhow!("Type mismatch при разрешении {}: зарегистрированный тип не соответствует запрашиваемому", type_name)
            })
    }

    /// Попытаться разрешить зависимость (безопасная версия)
    pub fn try_resolve<T>(&self) -> Option<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();

        debug!("🔍 TypeSafeResolver: попытка разрешения типа {}", type_name);

        let any_instance = self.resolver.try_resolve_by_type_id(type_id)?;

        // Safe downcast - возвращаем None вместо panic
        match any_instance.downcast::<T>() {
            Ok(typed_instance) => {
                debug!("✅ TypeSafeResolver: тип {} успешно разрешен", type_name);
                Some(typed_instance)
            }
            Err(_) => {
                warn!("⚠️ TypeSafeResolver: type mismatch для {}", type_name);
                None
            }
        }
    }

    /// Проверить, зарегистрирован ли тип
    pub fn is_registered<T>(&self) -> bool
    where
        T: Any + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        self.resolver.is_registered_by_type_id(type_id)
    }

    /// Получить диагностическую информацию
    pub fn get_diagnostic_info(&self) -> ResolverDiagnostics {
        let registered_types = self.resolver.get_registered_types();
        let type_names: Vec<String> = registered_types
            .iter()
            .filter_map(|&type_id| self.resolver.get_type_name(type_id))
            .collect();

        ResolverDiagnostics {
            total_registered: registered_types.len(),
            registered_types,
            type_names,
        }
    }

    /// Получить underlying object-safe resolver (для advanced использования)
    pub fn inner(&self) -> &Arc<dyn ObjectSafeResolver> {
        &self.resolver
    }
}

/// Диагностическая информация о resolver
#[derive(Debug, Clone)]
pub struct ResolverDiagnostics {
    /// Общее количество зарегистрированных типов
    pub total_registered: usize,
    /// Список зарегистрированных TypeId
    pub registered_types: Vec<TypeId>,
    /// Названия типов (если доступны)
    pub type_names: Vec<String>,
}

/// Service Locator реализация для ObjectSafeResolver
///
/// WARNING: Это контролируемое использование Service Locator anti-pattern.
/// Используется только как implementation detail для type erasure,
/// external API остается dependency injection based.
pub struct ServiceLocatorResolver {
    /// Type-erased registry компонентов
    registry: parking_lot::RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync + 'static>>>,
    /// Названия типов для диагностики
    type_names: parking_lot::RwLock<HashMap<TypeId, String>>,
}

impl ServiceLocatorResolver {
    /// Создать новый service locator resolver
    pub fn new() -> Self {
        debug!("🔧 ServiceLocatorResolver: создание нового registry");
        Self {
            registry: parking_lot::RwLock::new(HashMap::new()),
            type_names: parking_lot::RwLock::new(HashMap::new()),
        }
    }

    /// Зарегистрировать компонент с type safety
    pub fn register<T>(&self, instance: Arc<T>) -> Result<()>
    where
        T: Any + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>().to_string();

        debug!("📝 ServiceLocatorResolver: регистрация типа {}", type_name);

        // Проверка на дублирование
        {
            let registry = self.registry.read();
            if registry.contains_key(&type_id) {
                warn!(
                    "⚠️ ServiceLocatorResolver: тип {} уже зарегистрирован, перезапись",
                    type_name
                );
            }
        }

        // Регистрация с type erasure
        {
            let mut registry = self.registry.write();
            let mut type_names = self.type_names.write();

            let any_instance: Arc<dyn Any + Send + Sync + 'static> = instance;
            registry.insert(type_id, any_instance);
            type_names.insert(type_id, type_name.clone());
        }

        debug!(
            "✅ ServiceLocatorResolver: тип {} зарегистрирован",
            type_name
        );
        Ok(())
    }

    /// Разрегистрировать компонент
    pub fn unregister<T>(&self) -> bool
    where
        T: Any + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();

        debug!(
            "🗑️ ServiceLocatorResolver: разрегистрация типа {}",
            type_name
        );

        let mut registry = self.registry.write();
        let mut type_names = self.type_names.write();

        let removed = registry.remove(&type_id).is_some();
        type_names.remove(&type_id);

        if removed {
            debug!(
                "✅ ServiceLocatorResolver: тип {} разрегистрирован",
                type_name
            );
        } else {
            warn!(
                "⚠️ ServiceLocatorResolver: тип {} не найден для разрегистрации",
                type_name
            );
        }

        removed
    }

    /// Очистить весь registry
    pub fn clear(&self) {
        debug!("🧹 ServiceLocatorResolver: очистка registry");

        let mut registry = self.registry.write();
        let mut type_names = self.type_names.write();

        let count = registry.len();
        registry.clear();
        type_names.clear();

        debug!("✅ ServiceLocatorResolver: очищено {} типов", count);
    }

    /// Получить количество зарегистрированных типов
    pub fn count(&self) -> usize {
        self.registry.read().len()
    }
}

impl Default for ServiceLocatorResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectSafeResolver for ServiceLocatorResolver {
    fn resolve_by_type_id(&self, type_id: TypeId) -> Result<Arc<dyn Any + Send + Sync + 'static>> {
        let registry = self.registry.read();
        let type_names = self.type_names.read();

        let type_name = type_names
            .get(&type_id)
            .map(|s| s.as_str())
            .unwrap_or("Unknown");

        match registry.get(&type_id) {
            Some(instance) => {
                debug!("✅ ServiceLocatorResolver: тип {} разрешен", type_name);
                Ok(instance.clone())
            }
            None => {
                let registered: Vec<String> = type_names.values().cloned().collect();
                error!(
                    "❌ ServiceLocatorResolver: тип {} не зарегистрирован. Доступные типы: [{}]",
                    type_name,
                    registered.join(", ")
                );
                Err(anyhow!(
                    "Тип {} не зарегистрирован в DI контейнере. Доступные типы: [{}]",
                    type_name,
                    registered.join(", ")
                ))
            }
        }
    }

    fn try_resolve_by_type_id(
        &self,
        type_id: TypeId,
    ) -> Option<Arc<dyn Any + Send + Sync + 'static>> {
        let registry = self.registry.read();
        registry.get(&type_id).cloned()
    }

    fn is_registered_by_type_id(&self, type_id: TypeId) -> bool {
        let registry = self.registry.read();
        registry.contains_key(&type_id)
    }

    fn get_registered_types(&self) -> Vec<TypeId> {
        let registry = self.registry.read();
        registry.keys().cloned().collect()
    }

    fn get_type_name(&self, type_id: TypeId) -> Option<String> {
        let type_names = self.type_names.read();
        type_names.get(&type_id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test service implementations
    struct TestServiceA {
        pub value: i32,
    }

    struct TestServiceB {
        #[allow(dead_code)]
        pub name: String,
    }

    #[test]
    fn test_service_locator_basic_operations() {
        let resolver = ServiceLocatorResolver::new();

        // Register services
        let service_a = Arc::new(TestServiceA { value: 42 });
        let service_b = Arc::new(TestServiceB {
            name: "test".to_string(),
        });

        resolver.register(service_a.clone()).unwrap();
        resolver.register(service_b.clone()).unwrap();

        // Test registration check
        assert!(resolver.is_registered_by_type_id(TypeId::of::<TestServiceA>()));
        assert!(resolver.is_registered_by_type_id(TypeId::of::<TestServiceB>()));

        // Test count
        assert_eq!(resolver.count(), 2);

        // Test type names
        assert!(resolver
            .get_type_name(TypeId::of::<TestServiceA>())
            .is_some());
    }

    #[test]
    fn test_type_safe_wrapper() {
        let service_locator = Arc::new(ServiceLocatorResolver::new());
        let resolver = TypeSafeResolver::new(service_locator.clone());

        // Register service
        let service = Arc::new(TestServiceA { value: 123 });
        service_locator.register(service.clone()).unwrap();

        // Test type-safe resolution
        let resolved = resolver.resolve::<TestServiceA>().unwrap();
        assert_eq!(resolved.value, 123);

        // Test try_resolve
        let try_resolved = resolver.try_resolve::<TestServiceA>().unwrap();
        assert_eq!(try_resolved.value, 123);

        // Test is_registered
        assert!(resolver.is_registered::<TestServiceA>());
        assert!(!resolver.is_registered::<TestServiceB>());
    }

    #[test]
    fn test_type_mismatch_handling() {
        let service_locator = Arc::new(ServiceLocatorResolver::new());
        let resolver = TypeSafeResolver::new(service_locator.clone());

        // Register service A
        let service_a = Arc::new(TestServiceA { value: 42 });
        service_locator.register(service_a).unwrap();

        // Try to resolve as different type - should fail gracefully
        let result = resolver.try_resolve::<TestServiceB>();
        assert!(result.is_none());

        // resolve() should return error
        let result = resolver.resolve::<TestServiceB>();
        assert!(result.is_err());
    }
}
