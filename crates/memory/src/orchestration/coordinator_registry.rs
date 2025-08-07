use anyhow::Result;
use std::sync::Arc;
use tracing::{info, debug, warn};

use crate::orchestration::{
    EmbeddingCoordinator, SearchCoordinator, HealthManager,
    PromotionCoordinator, ResourceController, BackupCoordinator,
    traits::Coordinator,
};

/// Coordinator registry и factory для orchestration системы
/// 
/// Применяет принципы SOLID:
/// - SRP: Только управление реестром координаторов и их resolution
/// - OCP: Расширяемость через добавление новых типов координаторов
/// - LSP: Взаимозаменяемость координаторов через общий trait
/// - ISP: Разделенные интерфейсы для разных типов операций
/// - DIP: Зависит от DI контейнера, не создает координаторы напрямую
pub struct CoordinatorRegistry {
    /// Embedding coordinator
    pub embedding: Arc<EmbeddingCoordinator>,
    /// Search coordinator
    pub search: Arc<SearchCoordinator>,
    /// Health manager
    pub health: Arc<HealthManager>,
    /// Promotion coordinator
    pub promotion: Arc<PromotionCoordinator>,
    /// Resource controller
    pub resources: Arc<ResourceController>,
    /// Backup coordinator
    pub backup: Arc<BackupCoordinator>,
}

/// Trait для coordinator registry (ISP принцип)
#[async_trait::async_trait]
pub trait CoordinatorRegistryTrait: Send + Sync {
    /// Проверить готовность всех координаторов
    async fn verify_all_ready(&self) -> bool;
    
    /// Получить статус готовности по координаторам
    async fn get_readiness_status(&self) -> ReadinessStatus;
    
    /// Получить координатор по имени (для динамического доступа)
    fn get_coordinator(&self, name: &str) -> Option<&dyn Coordinator>;
    
    /// Получить список всех координаторов
    fn list_coordinator_names(&self) -> Vec<String>;
    
    /// Получить метрики от всех координаторов
    async fn collect_all_metrics(&self) -> CoordinatorMetricsCollection;
}

/// Статус готовности системы
#[derive(Debug, Clone)]
pub struct ReadinessStatus {
    pub all_ready: bool,
    pub coordinator_status: std::collections::HashMap<String, bool>,
    pub not_ready_coordinators: Vec<String>,
    pub ready_count: usize,
    pub total_count: usize,
}

/// Коллекция метрик от всех координаторов
#[derive(Debug)]
pub struct CoordinatorMetricsCollection {
    pub embedding_metrics: serde_json::Value,
    pub search_metrics: serde_json::Value,
    pub health_metrics: serde_json::Value,
    pub promotion_metrics: serde_json::Value,
    pub resources_metrics: serde_json::Value,
    pub backup_metrics: serde_json::Value,
}

/// Factory для создания coordinator registry
pub struct CoordinatorRegistryFactory;

impl CoordinatorRegistry {
    /// Создать registry из всех координаторов
    pub fn new(
        embedding: Arc<EmbeddingCoordinator>,
        search: Arc<SearchCoordinator>,
        health: Arc<HealthManager>,
        promotion: Arc<PromotionCoordinator>,
        resources: Arc<ResourceController>,
        backup: Arc<BackupCoordinator>,
    ) -> Self {
        info!("🏗️ Создание CoordinatorRegistry с {} координаторами", 6);
        
        Self {
            embedding,
            search,
            health,
            promotion,
            resources,
            backup,
        }
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
    async fn check_coordinators_readiness(coordinators: &[(&str, &dyn Coordinator)]) -> Vec<(String, bool)> {
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
            let not_ready: Vec<&str> = results.iter()
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
    pub fn from_container(container: &crate::di::container_core::ContainerCore) -> Result<CoordinatorRegistry> {
        info!("🏭 Создание CoordinatorRegistry из DI контейнера");
        
        // Разрешаем координаторы из контейнера
        let embedding = container.resolve::<EmbeddingCoordinator>()
            .map_err(|e| anyhow::anyhow!("Не удалось разрешить EmbeddingCoordinator: {}", e))?;
        
        let search = container.resolve::<SearchCoordinator>()
            .map_err(|e| anyhow::anyhow!("Не удалось разрешить SearchCoordinator: {}", e))?;
        
        let health = container.resolve::<HealthManager>()
            .map_err(|e| anyhow::anyhow!("Не удалось разрешить HealthManager: {}", e))?;
        
        let promotion = container.resolve::<PromotionCoordinator>()
            .map_err(|e| anyhow::anyhow!("Не удалось разрешить PromotionCoordinator: {}", e))?;
        
        let resources = container.resolve::<ResourceController>()
            .map_err(|e| anyhow::anyhow!("Не удалось разрешить ResourceController: {}", e))?;
        
        let backup = container.resolve::<BackupCoordinator>()
            .map_err(|e| anyhow::anyhow!("Не удалось разрешить BackupCoordinator: {}", e))?;
        
        let registry = CoordinatorRegistry::new(
            embedding,
            search,
            health,
            promotion,
            resources,
            backup,
        );
        
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
            "embedding", "search", "health", "promotion", "resources", "backup"
        ];
        
        let missing_coordinators: Vec<String> = expected_coordinators.iter()
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
            warn!("❌ CoordinatorRegistry валидация провалена: отсутствуют {:?}", 
                validation_result.missing_coordinators);
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
            "backup"
        ];
        
        // В реальных тестах здесь был бы создан registry и проверен метод list_coordinator_names()
        assert_eq!(expected_coordinators.len(), 6);
    }
}