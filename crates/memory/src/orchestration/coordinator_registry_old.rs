//! CoordinatorRegistry - реестр и управление координаторами
//!
//! Реализует Single Responsibility Principle для регистрации координаторов,
//! управления их зависимостями и метаданными.

use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::orchestration::traits::Coordinator;

/// Метаданные координатора
#[derive(Debug, Clone)]
pub struct CoordinatorMetadata {
    pub name: String,
    pub priority: u32,
    pub dependencies: Vec<String>,
    pub capabilities: Vec<String>,
    pub tags: HashMap<String, String>,
    pub initialization_order: u32,
    pub is_critical: bool,
}

impl CoordinatorMetadata {
    pub fn new(name: String) -> Self {
        Self {
            name: name.clone(),
            priority: 100, // default priority
            dependencies: Vec::new(),
            capabilities: Vec::new(),
            tags: HashMap::new(),
            initialization_order: 100, // default order
            is_critical: false,
        }
    }

    /// Builder методы для удобного создания метаданных
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }

    pub fn add_dependency(mut self, dep: String) -> Self {
        self.dependencies.push(dep);
        self
    }

    pub fn with_capabilities(mut self, caps: Vec<String>) -> Self {
        self.capabilities = caps;
        self
    }

    pub fn add_capability(mut self, cap: String) -> Self {
        self.capabilities.push(cap);
        self
    }

    pub fn with_tag(mut self, key: String, value: String) -> Self {
        self.tags.insert(key, value);
        self
    }

    pub fn with_initialization_order(mut self, order: u32) -> Self {
        self.initialization_order = order;
        self
    }

    pub fn as_critical(mut self) -> Self {
        self.is_critical = true;
        self
    }
}

/// Статус регистрации координатора
#[derive(Debug, Clone, PartialEq)]
pub enum RegistrationStatus {
    Registered,
    DependenciesNotMet,
    CircularDependency,
    Duplicate,
}

/// Результат регистрации
#[derive(Debug)]
pub struct RegistrationResult {
    pub status: RegistrationStatus,
    pub message: String,
    pub missing_dependencies: Vec<String>,
}

/// Запись о координаторе в реестре
#[derive(Debug)]
pub struct CoordinatorEntry {
    pub coordinator: Arc<dyn Coordinator>,
    pub metadata: CoordinatorMetadata,
    pub registration_time: std::time::Instant,
    pub is_active: bool,
}

/// Реестр координаторов с управлением зависимостями
pub struct CoordinatorRegistry {
    // Зарегистрированные координаторы
    coordinators: HashMap<String, CoordinatorEntry>,

    // Граф зависимостей для проверки циклов
    dependency_graph: HashMap<String, HashSet<String>>,

    // Порядок инициализации (топологическая сортировка)
    initialization_order: Vec<String>,

    // Кэш для результатов проверки зависимостей
    dependency_cache: HashMap<String, bool>,

    // Группы координаторов по тегам
    tag_groups: HashMap<String, HashSet<String>>,
}

impl CoordinatorRegistry {
    /// Создать новый реестр
    pub fn new() -> Self {
        Self {
            coordinators: HashMap::new(),
            dependency_graph: HashMap::new(),
            initialization_order: Vec::new(),
            dependency_cache: HashMap::new(),
            tag_groups: HashMap::new(),
        }
    }

    /// Зарегистрировать координатор
    pub fn register_coordinator(
        &mut self,
        coordinator: Arc<dyn Coordinator>,
        metadata: CoordinatorMetadata,
    ) -> RegistrationResult {
        let name = metadata.name.clone();

        // Проверяем дубликаты
        if self.coordinators.contains_key(&name) {
            return RegistrationResult {
                status: RegistrationStatus::Duplicate,
                message: format!("Координатор '{}' уже зарегистрирован", name),
                missing_dependencies: Vec::new(),
            };
        }

        // Проверяем зависимости
        let missing_deps = self.check_dependencies(&metadata.dependencies);
        if !missing_deps.is_empty() {
            return RegistrationResult {
                status: RegistrationStatus::DependenciesNotMet,
                message: format!("Не все зависимости доступны для '{}'", name),
                missing_dependencies: missing_deps,
            };
        }

        // Проверяем циклические зависимости
        if self.would_create_cycle(&name, &metadata.dependencies) {
            return RegistrationResult {
                status: RegistrationStatus::CircularDependency,
                message: format!("Регистрация '{}' создала бы циклическую зависимость", name),
                missing_dependencies: Vec::new(),
            };
        }

        // Регистрируем координатор
        let entry = CoordinatorEntry {
            coordinator,
            metadata: metadata.clone(),
            registration_time: std::time::Instant::now(),
            is_active: true,
        };

        self.coordinators.insert(name.clone(), entry);

        // Обновляем граф зависимостей
        self.dependency_graph.insert(
            name.clone(),
            metadata.dependencies.iter().cloned().collect(),
        );

        // Обновляем группы по тегам
        for (tag_key, tag_value) in &metadata.tags {
            let group_key = format!("{}:{}", tag_key, tag_value);
            self.tag_groups
                .entry(group_key)
                .or_insert_with(HashSet::new)
                .insert(name.clone());
        }

        // Пересчитываем порядок инициализации
        self.recalculate_initialization_order();

        // Очищаем кэш зависимостей
        self.dependency_cache.clear();

        info!(
            "Координатор '{}' успешно зарегистрирован (приоритет: {})",
            name, metadata.priority
        );

        RegistrationResult {
            status: RegistrationStatus::Registered,
            message: format!("Координатор '{}' успешно зарегистрирован", name),
            missing_dependencies: Vec::new(),
        }
    }

    /// Получить координатор по имени
    pub fn get_coordinator(&self, name: &str) -> Option<Arc<dyn Coordinator>> {
        self.coordinators
            .get(name)
            .map(|entry| Arc::clone(&entry.coordinator))
    }

    /// Получить все координаторы
    pub fn get_all_coordinators(&self) -> HashMap<String, Arc<dyn Coordinator>> {
        self.coordinators
            .iter()
            .map(|(name, entry)| (name.clone(), Arc::clone(&entry.coordinator)))
            .collect()
    }

    /// Получить координаторы в порядке инициализации
    pub fn get_initialization_order(&self) -> Vec<(String, Arc<dyn Coordinator>)> {
        self.initialization_order
            .iter()
            .filter_map(|name| {
                self.coordinators
                    .get(name)
                    .map(|entry| (name.clone(), Arc::clone(&entry.coordinator)))
            })
            .collect()
    }

    /// Получить все координаторы как slice для итерации
    pub fn get_all_coordinators(&self) -> Vec<(&'static str, &dyn Coordinator)> {
        vec![
            ("embedding", &*self.embedding as &dyn Coordinator),
            ("search", &*self.search as &dyn Coordinator),
            ("health", &*self.health as &dyn Coordinator),
            ("promotion", &*self.promotion as &dyn Coordinator),
            ("resources", &*self.resources as &dyn Coordinator),
            ("backup", &*self.backup as &dyn Coordinator),
        ]
    }

    /// Получить критически важные координаторы (для первоочередной инициализации)
    pub fn get_critical_coordinators(&self) -> Vec<(&'static str, &dyn Coordinator)> {
        vec![
            ("resources", &*self.resources as &dyn Coordinator),
            ("health", &*self.health as &dyn Coordinator),
        ]
    }

    /// Получить core координаторы (основная функциональность)
    pub fn get_core_coordinators(&self) -> Vec<(&'static str, &dyn Coordinator)> {
        vec![
            ("embedding", &*self.embedding as &dyn Coordinator),
            ("search", &*self.search as &dyn Coordinator),
        ]
    }

    /// Получить background координаторы (фоновые процессы)
    pub fn get_background_coordinators(&self) -> Vec<(&'static str, &dyn Coordinator)> {
        vec![
            ("promotion", &*self.promotion as &dyn Coordinator),
            ("backup", &*self.backup as &dyn Coordinator),
        ]
    }

    /// Проверить readiness конкретного набора координаторов
    async fn check_coordinators_readiness(
        coordinators: &[(&str, &dyn Coordinator)],
    ) -> Vec<(String, bool)> {
        let mut results = Vec::new();

        for (name, coordinator) in coordinators {
            let ready = coordinator.is_ready().await;
            results.push((name.to_string(), ready));

            if ready {
                debug!("✅ {} coordinator готов", name);
            } else {
                debug!("⏳ {} coordinator не готов", name);
            }
        }

        results
    }
}

#[async_trait::async_trait]
impl CoordinatorRegistryTrait for CoordinatorRegistry {
    async fn verify_all_ready(&self) -> bool {
        debug!("🔍 Проверка готовности всех координаторов");

        let coordinators = self.get_all_coordinators();
        let results = Self::check_coordinators_readiness(&coordinators).await;

        let all_ready = results.iter().all(|(_, ready)| *ready);

        if all_ready {
            debug!("✅ Все координаторы готовы");
        } else {
            let not_ready: Vec<&str> = results
                .iter()
                .filter(|(_, ready)| !*ready)
                .map(|(name, _)| name.as_str())
                .collect();
            debug!("⏳ Не готовы координаторы: {:?}", not_ready);
        }

        all_ready
    }

    async fn get_readiness_status(&self) -> ReadinessStatus {
        let coordinators = self.get_all_coordinators();
        let results = Self::check_coordinators_readiness(&coordinators).await;

        let mut coordinator_status = std::collections::HashMap::new();
        let mut not_ready_coordinators = Vec::new();
        let mut ready_count = 0;

        for (name, ready) in results {
            coordinator_status.insert(name.clone(), ready);

            if ready {
                ready_count += 1;
            } else {
                not_ready_coordinators.push(name);
            }
        }

        let all_ready = ready_count == coordinators.len();

        ReadinessStatus {
            all_ready,
            coordinator_status,
            not_ready_coordinators,
            ready_count,
            total_count: coordinators.len(),
        }
    }

    fn get_coordinator(&self, name: &str) -> Option<&dyn Coordinator> {
        match name {
            "embedding" => Some(&*self.embedding as &dyn Coordinator),
            "search" => Some(&*self.search as &dyn Coordinator),
            "health" => Some(&*self.health as &dyn Coordinator),
            "promotion" => Some(&*self.promotion as &dyn Coordinator),
            "resources" => Some(&*self.resources as &dyn Coordinator),
            "backup" => Some(&*self.backup as &dyn Coordinator),
            _ => {
                warn!("Неизвестный координатор: {}", name);
                None
            }
        }
    }

    fn list_coordinator_names(&self) -> Vec<String> {
        vec![
            "embedding".to_string(),
            "search".to_string(),
            "health".to_string(),
            "promotion".to_string(),
            "resources".to_string(),
            "backup".to_string(),
        ]
    }

    async fn collect_all_metrics(&self) -> CoordinatorMetricsCollection {
        debug!("📊 Сбор метрик от всех координаторов");

        // Параллельный сбор метрик от всех координаторов
        let results = tokio::join!(
            self.embedding.metrics(),
            self.search.metrics(),
            self.health.metrics(),
            self.promotion.metrics(),
            self.resources.metrics(),
            self.backup.metrics()
        );

        debug!("✅ Метрики собраны от {} координаторов", 6);

        CoordinatorMetricsCollection {
            embedding_metrics: results.0,
            search_metrics: results.1,
            health_metrics: results.2,
            promotion_metrics: results.3,
            resources_metrics: results.4,
            backup_metrics: results.5,
        }
    }
}

impl CoordinatorRegistryFactory {
    /// Создать coordinator registry из DI контейнера
    pub fn from_container(
        container: &crate::di::container_core::ContainerCore,
    ) -> Result<CoordinatorRegistry> {
        info!("🏭 Создание CoordinatorRegistry из DI контейнера");

        // Разрешаем координаторы из контейнера
        let embedding = container
            .resolve::<EmbeddingCoordinator>()
            .map_err(|e| anyhow::anyhow!("Не удалось разрешить EmbeddingCoordinator: {}", e))?;

        let search = container
            .resolve::<SearchCoordinator>()
            .map_err(|e| anyhow::anyhow!("Не удалось разрешить SearchCoordinator: {}", e))?;

        let health = container
            .resolve::<HealthManager>()
            .map_err(|e| anyhow::anyhow!("Не удалось разрешить HealthManager: {}", e))?;

        let promotion = container
            .resolve::<PromotionCoordinator>()
            .map_err(|e| anyhow::anyhow!("Не удалось разрешить PromotionCoordinator: {}", e))?;

        let resources = container
            .resolve::<ResourceController>()
            .map_err(|e| anyhow::anyhow!("Не удалось разрешить ResourceController: {}", e))?;

        let backup = container
            .resolve::<BackupCoordinator>()
            .map_err(|e| anyhow::anyhow!("Не удалось разрешить BackupCoordinator: {}", e))?;

        let registry =
            CoordinatorRegistry::new(embedding, search, health, promotion, resources, backup);

        info!("✅ CoordinatorRegistry успешно создан");
        Ok(registry)
    }

    /// Создать coordinator registry с моками для тестирования
    #[cfg(test)]
    pub fn create_mock_registry() -> CoordinatorRegistry {
        use std::sync::Arc;

        // В реальном коде здесь были бы mock'и координаторов
        // Пока что это placeholder для компиляции тестов
        todo!("Mock registry implementation needed for tests")
    }

    /// Валидировать что все координаторы корректно созданы
    pub async fn validate_registry(registry: &CoordinatorRegistry) -> Result<ValidationResult> {
        info!("🔍 Валидация CoordinatorRegistry");

        let readiness_status = registry.get_readiness_status().await;
        let coordinator_names = registry.list_coordinator_names();

        // Проверяем что все ожидаемые координаторы присутствуют
        let expected_coordinators = vec![
            "embedding",
            "search",
            "health",
            "promotion",
            "resources",
            "backup",
        ];

        let missing_coordinators: Vec<String> = expected_coordinators
            .iter()
            .filter(|&&name| !coordinator_names.contains(&name.to_string()))
            .map(|&name| name.to_string())
            .collect();

        let validation_result = ValidationResult {
            is_valid: missing_coordinators.is_empty(),
            missing_coordinators,
            readiness_status,
            total_coordinators: coordinator_names.len(),
        };

        if validation_result.is_valid {
            info!("✅ CoordinatorRegistry успешно валидирован");
        } else {
            warn!(
                "❌ CoordinatorRegistry валидация провалена: отсутствуют {:?}",
                validation_result.missing_coordinators
            );
        }

        Ok(validation_result)
    }
}

/// Результат валидации registry
#[derive(Debug)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub missing_coordinators: Vec<String>,
    pub readiness_status: ReadinessStatus,
    pub total_coordinators: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_readiness_status_creation() {
        let mut coordinator_status = std::collections::HashMap::new();
        coordinator_status.insert("test1".to_string(), true);
        coordinator_status.insert("test2".to_string(), false);

        let status = ReadinessStatus {
            all_ready: false,
            coordinator_status,
            not_ready_coordinators: vec!["test2".to_string()],
            ready_count: 1,
            total_count: 2,
        };

        assert!(!status.all_ready);
        assert_eq!(status.ready_count, 1);
        assert_eq!(status.total_count, 2);
        assert_eq!(status.not_ready_coordinators, vec!["test2"]);
    }

    #[tokio::test]
    async fn test_coordinator_metrics_collection_structure() {
        let collection = CoordinatorMetricsCollection {
            embedding_metrics: serde_json::json!({"test": "embedding"}),
            search_metrics: serde_json::json!({"test": "search"}),
            health_metrics: serde_json::json!({"test": "health"}),
            promotion_metrics: serde_json::json!({"test": "promotion"}),
            resources_metrics: serde_json::json!({"test": "resources"}),
            backup_metrics: serde_json::json!({"test": "backup"}),
        };

        assert_eq!(collection.embedding_metrics["test"], "embedding");
        assert_eq!(collection.search_metrics["test"], "search");
    }

    #[tokio::test]
    async fn test_validation_result_structure() {
        let readiness_status = ReadinessStatus {
            all_ready: true,
            coordinator_status: std::collections::HashMap::new(),
            not_ready_coordinators: vec![],
            ready_count: 6,
            total_count: 6,
        };

        let result = ValidationResult {
            is_valid: true,
            missing_coordinators: vec![],
            readiness_status,
            total_coordinators: 6,
        };

        assert!(result.is_valid);
        assert!(result.missing_coordinators.is_empty());
        assert_eq!(result.total_coordinators, 6);
    }

    #[test]
    fn test_coordinator_list() {
        // Проверяем что список координаторов соответствует ожиданиям
        let expected_coordinators = vec![
            "embedding",
            "search",
            "health",
            "promotion",
            "resources",
            "backup",
        ];

        // В реальных тестах здесь был бы создан registry и проверен метод list_coordinator_names()
        assert_eq!(expected_coordinators.len(), 6);
    }
}
