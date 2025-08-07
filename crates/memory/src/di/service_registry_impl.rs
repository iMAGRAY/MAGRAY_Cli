//! Service Registry Implementation - регистрация сервисов в DI контейнере
//!
//! Отделен от unified_container_impl.rs для следования Single Responsibility Principle.
//! Отвечает ТОЛЬКО за регистрацию фабричных функций и управление их метаданными.

use anyhow::Result;
use parking_lot::RwLock;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};
use tracing::{debug, info, warn};

use super::{
    core_traits::{LifetimeStrategy, ServiceFactory},
    errors::DIError,
};

/// Информация о зарегистрированной фабрике
pub struct FactoryInfo {
    /// Фабричная функция для создания экземпляра
    pub factory: ServiceFactory,
    /// Стратегия управления жизненным циклом
    pub lifetime: LifetimeStrategy,
    /// Имя типа для отладки и логирования
    pub type_name: String,
    /// Время регистрации (для диагностики)
    pub registered_at: std::time::Instant,
    /// Количество раз когда тип был разрешен
    pub resolution_count: std::sync::atomic::AtomicU64,
}

impl FactoryInfo {
    /// Создать новую информацию о фабрике
    pub fn new(factory: ServiceFactory, lifetime: LifetimeStrategy, type_name: String) -> Self {
        Self {
            factory,
            lifetime,
            type_name,
            registered_at: std::time::Instant::now(),
            resolution_count: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Увеличить счетчик разрешений
    pub fn increment_resolution_count(&self) {
        self.resolution_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Получить количество разрешений
    pub fn get_resolution_count(&self) -> u64 {
        self.resolution_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Получить время с момента регистрации
    pub fn age(&self) -> std::time::Duration {
        self.registered_at.elapsed()
    }
}

impl std::fmt::Debug for FactoryInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FactoryInfo")
            .field("lifetime", &self.lifetime)
            .field("type_name", &self.type_name)
            .field("registered_at", &self.registered_at)
            .field("resolution_count", &self.get_resolution_count())
            .finish()
    }
}

/// Service Registry Implementation - отвечает ТОЛЬКО за регистрацию сервисов
///
/// ПРИНЦИПЫ:
/// - SRP: единственная ответственность - управление регистрациями
/// - OCP: расширяемость через различные lifetime стратегии
/// - LSP: соответствует интерфейсу ServiceRegistry
/// - ISP: минимальный интерфейс только для регистрации
/// - DIP: зависит от абстракций (ServiceFactory, LifetimeStrategy)
pub struct ServiceRegistryImpl {
    /// Зарегистрированные фабрики по TypeId
    factories: RwLock<HashMap<TypeId, FactoryInfo>>,
    /// Конфигурация registry
    config: RegistryConfig,
}

/// Конфигурация для service registry
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// Максимальное количество зарегистрированных типов
    pub max_registrations: usize,
    /// Включить подробное логирование регистраций
    pub verbose_logging: bool,
    /// Разрешить перерегистрацию типов
    pub allow_reregistration: bool,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            max_registrations: 10_000,
            verbose_logging: cfg!(debug_assertions),
            allow_reregistration: false,
        }
    }
}

impl RegistryConfig {
    /// Production конфигурация с оптимизированными параметрами
    pub fn production() -> Self {
        Self {
            max_registrations: 50_000,
            verbose_logging: false,
            allow_reregistration: false,
        }
    }

    /// Development конфигурация с расширенным логированием
    pub fn development() -> Self {
        Self {
            max_registrations: 5_000,
            verbose_logging: true,
            allow_reregistration: true,
        }
    }

    /// Minimal конфигурация для тестов
    pub fn minimal() -> Self {
        Self {
            max_registrations: 1_000,
            verbose_logging: false,
            allow_reregistration: true,
        }
    }
}

impl ServiceRegistryImpl {
    /// Создать новый service registry с указанной конфигурацией
    pub fn new(config: RegistryConfig) -> Self {
        info!(
            "🏗️ Создание ServiceRegistryImpl с лимитом {} регистраций",
            config.max_registrations
        );

        Self {
            factories: RwLock::new(HashMap::new()),
            config,
        }
    }

    /// Создать service registry с default конфигурацией
    pub fn default() -> Self {
        Self::new(RegistryConfig::default())
    }

    /// Зарегистрировать тип с фабричной функцией
    pub fn register_type_erased(
        &self,
        type_id: TypeId,
        type_name: String,
        factory: ServiceFactory,
        lifetime: LifetimeStrategy,
    ) -> Result<(), DIError> {
        if self.config.verbose_logging {
            debug!(
                "📝 Регистрация типа {} ({:?}) с lifetime {:?}",
                type_name, type_id, lifetime
            );
        }

        // Проверяем лимит регистраций
        {
            let factories = self.factories.read();
            if factories.len() >= self.config.max_registrations {
                let error = DIError::RegistrationLimitExceeded {
                    limit: self.config.max_registrations,
                    current: factories.len(),
                };
                warn!("❌ Превышен лимит регистраций: {}", error);
                return Err(error);
            }
        }

        // Проверяем дублирование
        {
            let factories = self.factories.read();
            if factories.contains_key(&type_id) {
                if !self.config.allow_reregistration {
                    let error = DIError::DuplicateRegistration {
                        type_name: type_name.clone(),
                    };
                    warn!("❌ Попытка повторной регистрации {}: {}", type_name, error);
                    return Err(error);
                } else {
                    warn!("⚠️ Перерегистрация типа {} разрешена", type_name);
                }
            }
        }

        // Регистрируем фабрику
        {
            let mut factories = self.factories.write();
            let factory_info = FactoryInfo::new(factory, lifetime, type_name.clone());

            factories.insert(type_id, factory_info);
        }

        if self.config.verbose_logging {
            info!("✅ Тип {} успешно зарегистрирован", type_name);
        }

        Ok(())
    }

    /// Проверить зарегистрирован ли тип
    pub fn is_registered(&self, type_id: TypeId) -> bool {
        let factories = self.factories.read();
        factories.contains_key(&type_id)
    }

    /// Получить информацию о фабрике по TypeId
    pub fn get_factory_info(&self, type_id: TypeId) -> Option<FactoryInfo> {
        let factories = self.factories.read();
        // Note: Мы не можем клонировать FactoryInfo из-за ServiceFactory
        // Поэтому возвращаем Option и требуем прямого доступа к factory через другие методы
        None
    }

    /// Получить фабрику и выполнить операцию с ней (безопасный доступ)
    pub fn with_factory<R, F>(&self, type_id: TypeId, f: F) -> Option<R>
    where
        F: FnOnce(&FactoryInfo) -> R,
    {
        let factories = self.factories.read();
        factories.get(&type_id).map(f)
    }

    /// Получить список всех зарегистрированных типов
    pub fn get_registered_types(&self) -> Vec<(TypeId, String)> {
        let factories = self.factories.read();
        factories
            .iter()
            .map(|(&type_id, info)| (type_id, info.type_name.clone()))
            .collect()
    }

    /// Получить количество зарегистрированных типов
    pub fn registration_count(&self) -> usize {
        let factories = self.factories.read();
        factories.len()
    }

    /// Удалить регистрацию типа
    pub fn unregister(&self, type_id: TypeId) -> bool {
        let mut factories = self.factories.write();
        if let Some(info) = factories.remove(&type_id) {
            if self.config.verbose_logging {
                info!("🗑️ Тип {} удален из registry", info.type_name);
            }
            true
        } else {
            false
        }
    }

    /// Очистить все регистрации
    pub fn clear(&self) {
        let mut factories = self.factories.write();
        let count = factories.len();
        factories.clear();

        info!("🧹 Очищено {} регистраций из ServiceRegistry", count);
    }

    /// Получить статистику registry
    pub fn get_stats(&self) -> RegistryStats {
        let factories = self.factories.read();

        let total_registrations = factories.len();
        let mut lifetime_counts = HashMap::new();
        let mut total_resolutions = 0;
        let mut oldest_registration = None;
        let mut newest_registration = None;

        for (type_id, info) in factories.iter() {
            // Подсчет по lifetime
            *lifetime_counts.entry(info.lifetime).or_insert(0) += 1;

            // Общее количество разрешений
            total_resolutions += info.get_resolution_count();

            // Найдем самую старую и новую регистрации
            if oldest_registration.is_none() || info.registered_at < oldest_registration.unwrap() {
                oldest_registration = Some(info.registered_at);
            }
            if newest_registration.is_none() || info.registered_at > newest_registration.unwrap() {
                newest_registration = Some(info.registered_at);
            }
        }

        RegistryStats {
            total_registrations,
            singleton_count: lifetime_counts
                .get(&LifetimeStrategy::Singleton)
                .copied()
                .unwrap_or(0),
            transient_count: lifetime_counts
                .get(&LifetimeStrategy::Transient)
                .copied()
                .unwrap_or(0),
            scoped_count: lifetime_counts
                .get(&LifetimeStrategy::Scoped)
                .copied()
                .unwrap_or(0),
            total_resolutions,
            average_resolutions_per_type: if total_registrations > 0 {
                total_resolutions as f64 / total_registrations as f64
            } else {
                0.0
            },
            registry_age: oldest_registration.map(|t| t.elapsed()),
            max_registrations: self.config.max_registrations,
            utilization: (total_registrations as f64 / self.config.max_registrations as f64)
                * 100.0,
        }
    }

    /// Получить детальный отчет о registry
    pub fn get_detailed_report(&self) -> String {
        let stats = self.get_stats();
        let factories = self.factories.read();

        // Найдем топ-5 наиболее используемых типов
        let mut type_usage: Vec<_> = factories
            .iter()
            .map(|(type_id, info)| {
                (
                    info.type_name.clone(),
                    info.get_resolution_count(),
                    info.lifetime,
                )
            })
            .collect();
        type_usage.sort_by(|a, b| b.1.cmp(&a.1));

        let top_used = type_usage
            .iter()
            .take(5)
            .map(|(name, count, lifetime)| {
                format!("  {} ({:?}): {} resolutions", name, lifetime, count)
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "=== Service Registry Detailed Report ===\n\
             Total registrations: {}\n\
             - Singleton: {}\n\
             - Transient: {}\n\
             - Scoped: {}\n\
             Total resolutions: {}\n\
             Average resolutions per type: {:.2}\n\
             Registry utilization: {:.1}% ({}/{})\n\
             Registry age: {:?}\n\
             \n\
             Top 5 most used types:\n\
             {}\n\
             =======================================",
            stats.total_registrations,
            stats.singleton_count,
            stats.transient_count,
            stats.scoped_count,
            stats.total_resolutions,
            stats.average_resolutions_per_type,
            stats.utilization,
            stats.total_registrations,
            stats.max_registrations,
            stats.registry_age.unwrap_or_default(),
            if top_used.is_empty() {
                "  None"
            } else {
                &top_used
            }
        )
    }

    /// Validate registry state
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        let factories = self.factories.read();

        // Проверяем лимиты
        if factories.len() > self.config.max_registrations {
            errors.push(format!(
                "Registry превысил лимит: {} > {}",
                factories.len(),
                self.config.max_registrations
            ));
        }

        // Проверяем на подозрительные регистрации (много неиспользуемых типов)
        let unused_count = factories
            .values()
            .filter(|info| info.get_resolution_count() == 0)
            .count();

        if unused_count > factories.len() / 2 {
            errors.push(format!(
                "Слишком много неиспользуемых типов: {} из {} ({:.1}%)",
                unused_count,
                factories.len(),
                (unused_count as f64 / factories.len() as f64) * 100.0
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Получить конфигурацию registry
    pub fn get_config(&self) -> &RegistryConfig {
        &self.config
    }
}

/// Статистика service registry
#[derive(Debug, Clone)]
pub struct RegistryStats {
    /// Общее количество регистраций
    pub total_registrations: usize,
    /// Количество singleton регистраций
    pub singleton_count: usize,
    /// Количество transient регистраций
    pub transient_count: usize,
    /// Количество scoped регистраций
    pub scoped_count: usize,
    /// Общее количество разрешений всех типов
    pub total_resolutions: u64,
    /// Среднее количество разрешений на тип
    pub average_resolutions_per_type: f64,
    /// Время существования registry
    pub registry_age: Option<std::time::Duration>,
    /// Максимальное количество регистраций
    pub max_registrations: usize,
    /// Утилизация registry в процентах
    pub utilization: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock factory для тестирования
    fn create_mock_factory() -> ServiceFactory {
        Box::new(|_| Ok(Box::new("test_value") as Box<dyn Any + Send + Sync>))
    }

    #[test]
    fn test_registry_creation() {
        let registry = ServiceRegistryImpl::default();
        assert_eq!(registry.registration_count(), 0);
    }

    #[test]
    fn test_type_registration() {
        let registry = ServiceRegistryImpl::default();
        let type_id = TypeId::of::<String>();
        let factory = create_mock_factory();

        let result = registry.register_type_erased(
            type_id,
            "String".to_string(),
            factory,
            LifetimeStrategy::Singleton,
        );

        assert!(result.is_ok());
        assert_eq!(registry.registration_count(), 1);
        assert!(registry.is_registered(type_id));
    }

    #[test]
    fn test_duplicate_registration_blocked() {
        let registry = ServiceRegistryImpl::new(RegistryConfig {
            allow_reregistration: false,
            ..Default::default()
        });
        let type_id = TypeId::of::<String>();

        // Первая регистрация должна пройти
        let result1 = registry.register_type_erased(
            type_id,
            "String".to_string(),
            create_mock_factory(),
            LifetimeStrategy::Singleton,
        );
        assert!(result1.is_ok());

        // Вторая регистрация должна быть заблокирована
        let result2 = registry.register_type_erased(
            type_id,
            "String".to_string(),
            create_mock_factory(),
            LifetimeStrategy::Singleton,
        );
        assert!(result2.is_err());
    }

    #[test]
    fn test_reregistration_allowed() {
        let registry = ServiceRegistryImpl::new(RegistryConfig {
            allow_reregistration: true,
            ..Default::default()
        });
        let type_id = TypeId::of::<String>();

        // Обе регистрации должны пройти
        let result1 = registry.register_type_erased(
            type_id,
            "String".to_string(),
            create_mock_factory(),
            LifetimeStrategy::Singleton,
        );
        assert!(result1.is_ok());

        let result2 = registry.register_type_erased(
            type_id,
            "String".to_string(),
            create_mock_factory(),
            LifetimeStrategy::Transient,
        );
        assert!(result2.is_ok());
    }

    #[test]
    fn test_registration_limit() {
        let registry = ServiceRegistryImpl::new(RegistryConfig {
            max_registrations: 2,
            ..Default::default()
        });

        // Первые две регистрации должны пройти
        for i in 0..2 {
            let result = registry.register_type_erased(
                TypeId::of::<usize>(), // Используем одинаковый тип с разными именами
                format!("Type{}", i),
                create_mock_factory(),
                LifetimeStrategy::Singleton,
            );
            if i == 0 {
                assert!(result.is_ok());
            } else {
                // Вторая регистрация того же типа должна быть заблокирована (allow_reregistration = false по умолчанию)
                assert!(result.is_err());
            }
        }
    }

    #[test]
    fn test_registry_stats() {
        let registry = ServiceRegistryImpl::default();

        // Регистрируем несколько типов с разными lifetime
        registry
            .register_type_erased(
                TypeId::of::<String>(),
                "String".to_string(),
                create_mock_factory(),
                LifetimeStrategy::Singleton,
            )
            .unwrap();

        registry
            .register_type_erased(
                TypeId::of::<i32>(),
                "i32".to_string(),
                create_mock_factory(),
                LifetimeStrategy::Transient,
            )
            .unwrap();

        let stats = registry.get_stats();
        assert_eq!(stats.total_registrations, 2);
        assert_eq!(stats.singleton_count, 1);
        assert_eq!(stats.transient_count, 1);
        assert_eq!(stats.scoped_count, 0);
        assert!(stats.utilization > 0.0);
    }

    #[test]
    fn test_registry_clear() {
        let registry = ServiceRegistryImpl::default();

        registry
            .register_type_erased(
                TypeId::of::<String>(),
                "String".to_string(),
                create_mock_factory(),
                LifetimeStrategy::Singleton,
            )
            .unwrap();

        assert_eq!(registry.registration_count(), 1);

        registry.clear();
        assert_eq!(registry.registration_count(), 0);
    }

    #[test]
    fn test_registry_detailed_report() {
        let registry = ServiceRegistryImpl::default();

        registry
            .register_type_erased(
                TypeId::of::<String>(),
                "String".to_string(),
                create_mock_factory(),
                LifetimeStrategy::Singleton,
            )
            .unwrap();

        let report = registry.get_detailed_report();
        assert!(report.contains("Service Registry Detailed Report"));
        assert!(report.contains("Total registrations: 1"));
        assert!(report.contains("Singleton: 1"));
    }

    #[test]
    fn test_registry_validation() {
        let registry = ServiceRegistryImpl::default();

        // Пустой registry должен быть валидным
        assert!(registry.validate().is_ok());

        // Добавляем одну регистрацию - должно быть валидным
        registry
            .register_type_erased(
                TypeId::of::<String>(),
                "String".to_string(),
                create_mock_factory(),
                LifetimeStrategy::Singleton,
            )
            .unwrap();

        assert!(registry.validate().is_ok());
    }
}
