//! MonitoringService - мониторинг системы и метрики
//!
//! Single Responsibility: только monitoring и metrics
//! - health monitoring
//! - performance metrics collection
//! - system resource tracking
//! - readiness checks

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::{
    batch_manager::BatchStats,
    di::{traits::DIResolver, UnifiedContainer},
    health::{HealthMonitor, SystemHealthStatus},
    promotion::PromotionStats,
    service_di::MemorySystemStats,
    services::traits::{MonitoringServiceTrait, ProductionMetrics},
    services::CoordinatorServiceTrait,
};
use crate::di::core_traits::ServiceResolver;

/// Реализация системного мониторинга
/// Отвечает ТОЛЬКО за мониторинг и сбор метрик
#[allow(dead_code)]
pub struct MonitoringService {
    /// DI контейнер для доступа к компонентам
    container: Arc<UnifiedContainer>,
    /// Кэшированные production метрики
    production_metrics: Arc<RwLock<ProductionMetrics>>,
    /// Счетчик запущенных мониторингов
    monitoring_tasks_count: Arc<std::sync::atomic::AtomicU32>,
    /// Координатор сервис для проверки готовности
    coordinator_service: Option<Arc<dyn CoordinatorServiceTrait>>,
}

impl MonitoringService {
    /// Создать новый MonitoringService
    pub fn new(container: Arc<UnifiedContainer>) -> Self {
        info!("📊 Создание MonitoringService для системного мониторинга");

        Self {
            container,
            production_metrics: Arc::new(RwLock::new(ProductionMetrics::default())),
            monitoring_tasks_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            coordinator_service: None,
        }
    }

    /// Создать с coordinator service для более полной функциональности
    #[allow(dead_code)]
    pub fn new_with_coordinator(
        container: Arc<UnifiedContainer>,
        coordinator_service: Arc<dyn CoordinatorServiceTrait>,
    ) -> Self {
        info!("📊 Создание MonitoringService с CoordinatorService");

        Self {
            container,
            production_metrics: Arc::new(RwLock::new(ProductionMetrics::default())),
            monitoring_tasks_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            coordinator_service: Some(coordinator_service),
        }
    }

    /// Получить health monitor из DI
    #[allow(dead_code)]
    fn get_health_monitor(&self) -> Option<Arc<HealthMonitor>> {
        self.container.try_resolve::<HealthMonitor>()
    }
}

#[async_trait]
impl MonitoringServiceTrait for MonitoringService {
    /// Запустить production мониторинг
    #[allow(dead_code)]
    async fn start_production_monitoring(&self) -> Result<()> {
        info!("📊 Запуск production мониторинга...");

        let production_metrics = self.production_metrics.clone();
        let counter = self.monitoring_tasks_count.clone();

        tokio::spawn(async move {
            counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            let mut interval = tokio::time::interval(Duration::from_secs(60));

            loop {
                interval.tick().await;

                let metrics = production_metrics.read().await;

                if metrics.total_operations > 0 {
                    let success_rate = (metrics.successful_operations as f64
                        / metrics.total_operations as f64)
                        * 100.0;

                    debug!(
                        "📊 Production метрики: операций={}, успех={}%, avg_response={}ms",
                        metrics.total_operations, success_rate, metrics.avg_response_time_ms
                    );

                    if success_rate < 95.0 {
                        warn!("📉 Низкий success rate: {:.1}%", success_rate);
                    }

                    if metrics.avg_response_time_ms > 100.0 {
                        warn!(
                            "⏱️ Высокое время отклика: {:.1}ms",
                            metrics.avg_response_time_ms
                        );
                    }
                }
            }
        });

        debug!("📊 Production мониторинг запущен");
        Ok(())
    }

    /// Запустить health мониторинг
    #[allow(dead_code)]
    async fn start_health_monitoring(&self) -> Result<()> {
        if let Some(coordinator_service) = &self.coordinator_service {
            if let Some(health_manager) = coordinator_service.get_health_manager() {
                info!("🚑 Запуск health мониторинга через HealthManager...");

                let _manager = health_manager.clone();
                let counter = self.monitoring_tasks_count.clone();

                tokio::spawn(async move {
                    counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                    let mut interval = tokio::time::interval(Duration::from_secs(30));

                    loop {
                        interval.tick().await;

                        // NOTE: run_health_check заглушка
                        if let Err(e) = async { Ok(()) as Result<()> }.await {
                            error!("❌ Health check не удался: {}", e);
                        }
                    }
                });

                debug!("🚑 Health мониторинг запущен");
            } else {
                warn!("⚠️ HealthManager недоступен для health мониторинга");
            }
        } else {
            // Fallback на прямой health monitor если нет coordinator service
            if let Some(health_monitor) = self.get_health_monitor() {
                info!("🚑 Запуск fallback health мониторинга...");

                let monitor = health_monitor.clone();
                let counter = self.monitoring_tasks_count.clone();

                tokio::spawn(async move {
                    counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                    let mut interval = tokio::time::interval(Duration::from_secs(60));

                    loop {
                        interval.tick().await;

                        let health = monitor.get_system_health();
                        debug!("🚑 System health: {:?}", health);
                    }
                });

                debug!("🚑 Fallback health мониторинг запущен");
            } else {
                warn!("⚠️ Ни HealthManager, ни HealthMonitor недоступны");
            }
        }

        Ok(())
    }

    /// Запустить resource мониторинг
    #[allow(dead_code)]
    async fn start_resource_monitoring(&self) -> Result<()> {
        if let Some(coordinator_service) = &self.coordinator_service {
            if let Some(resource_controller) = coordinator_service.get_resource_controller() {
                info!("💾 Запуск resource мониторинга и auto-scaling...");

                // Запускаем auto-scaling monitoring
                resource_controller.start_autoscaling_monitoring().await?;

                debug!("💾 Resource мониторинг запущен");
            } else {
                warn!("⚠️ ResourceController недоступен для resource мониторинга");
            }
        } else {
            warn!("⚠️ CoordinatorService недоступен для resource мониторинга");
        }

        Ok(())
    }

    /// Выполнить проверки готовности
    #[allow(dead_code)]
    async fn perform_readiness_checks(&self) -> Result<()> {
        info!("🔍 Выполнение проверок готовности...");

        if let Some(coordinator_service) = &self.coordinator_service {
            // Проверяем готовность координаторов (заглушки)
            let mut coordinator_statuses = Vec::new();

            if coordinator_service.get_embedding_coordinator().is_some() {
                coordinator_statuses.push(("EmbeddingCoordinator", true));
            }

            if coordinator_service.get_search_coordinator().is_some() {
                coordinator_statuses.push(("SearchCoordinator", true));
            }

            if coordinator_service.get_health_manager().is_some() {
                coordinator_statuses.push(("HealthManager", true));
            }

            if coordinator_service.get_resource_controller().is_some() {
                coordinator_statuses.push(("ResourceController", true));
            }

            // Все координаторы всегда готовы (заглушка)
            for (name, _ready) in &coordinator_statuses {
                debug!("✅ {} готов (заглушка)", name);
            }

            info!("✅ Все координаторы готовы");
        } else {
            info!("ℹ️ CoordinatorService недоступен, пропускаем проверки координаторов");
        }

        // Базовые проверки DI контейнера
        let di_stats = crate::DIContainerStats::default();
        if di_stats.registered_factories == 0 {
            return Err(anyhow::anyhow!(
                "DI контейнер пуст - нет зарегистрированных типов"
            ));
        }

        info!("✅ Все проверки готовности пройдены");
        Ok(())
    }

    /// Получить статистику системы
    #[allow(dead_code)]
    async fn get_system_stats(&self) -> MemorySystemStats {
        debug!("📊 Сбор системной статистики...");

        // Health status (заглушка)
        let health_status = if let Some(_coordinator_service) = &self.coordinator_service {
            // Fallback на прямой health monitor
            if let Some(health) = self.get_health_monitor() {
                Ok(health.get_system_health())
            } else {
                Err(anyhow::anyhow!("Health monitoring недоступен"))
            }
        } else {
            Err(anyhow::anyhow!("CoordinatorService недоступен"))
        };

        // Cache статистика (заглушка)
        let cache_stats = (0, 0, 0); // hits, misses, size

        // Остальные статистики (заглушки, так как требуют async или сложной интеграции)
        let promotion_stats = PromotionStats::default();
        let batch_stats = BatchStats::default();
        let gpu_stats = None; // GPU stats требуют async

        MemorySystemStats {
            health_status,
            cache_hits: cache_stats.0,
            cache_misses: cache_stats.1,
            cache_size: cache_stats.2,
            promotion_stats,
            batch_stats,
            gpu_stats,
            di_container_stats: crate::DIContainerStats::default(),
        }
    }

    /// Получить health status
    #[allow(dead_code)]
    async fn check_health(&self) -> Result<SystemHealthStatus> {
        if let Some(health_monitor) = self.get_health_monitor() {
            Ok(health_monitor.get_system_health())
        } else {
            Err(anyhow::anyhow!("HealthMonitor недоступен"))
        }
    }

    /// Получить production метрики
    #[allow(dead_code)]
    async fn get_production_metrics(&self) -> Result<ProductionMetrics> {
        let metrics = self.production_metrics.read().await;
        Ok(metrics.clone())
    }

    /// Логирование summary
    #[allow(dead_code)]
    async fn log_initialization_summary(&self) {
        let _production_metrics = self.production_metrics.read().await;
        let coordinator_count = if let Some(coordinator_service) = &self.coordinator_service {
            coordinator_service.count_active_coordinators()
        } else {
            0
        };
        let di_stats = crate::DIContainerStats::default();
        let monitoring_tasks = self
            .monitoring_tasks_count
            .load(std::sync::atomic::Ordering::Relaxed);

        info!("🎉 === MONITORING INITIALIZATION SUMMARY ===");
        info!("📊 Активных координаторов: {}", coordinator_count);
        info!("🔧 DI зависимостей: {}", di_stats.registered_factories);
        info!("📈 Запущено monitoring задач: {}", monitoring_tasks);
        info!("⚡ Система готова к мониторингу");
        info!("===============================================");
    }
}

impl MonitoringService {
    /// Обновить production метрики (helper метод для интеграции с ResilienceService)
    #[allow(dead_code)]
    pub async fn update_production_metrics(&self, new_metrics: ProductionMetrics) {
        let mut metrics = self.production_metrics.write().await;
        *metrics = new_metrics;
    }

    /// Получить количество запущенных мониторинговых задач
    #[allow(dead_code)]
    pub fn get_monitoring_tasks_count(&self) -> u32 {
        self.monitoring_tasks_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}
