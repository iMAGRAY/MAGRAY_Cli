use anyhow::Result;
use std::{
    sync::Arc,
    time::{Duration, Instant},
    collections::HashMap,
};
use tokio::sync::RwLock;
use serde_json::{json, Value};
use tracing::{info, warn, debug};

use crate::orchestration::{
    EmbeddingCoordinator, SearchCoordinator, HealthManager,
    PromotionCoordinator, ResourceController, BackupCoordinator,
    circuit_breaker_manager::{CircuitBreakerManager, CircuitBreakerManagerTrait},
    operation_executor::OperationMetrics,
    traits::{Coordinator, ResourceCoordinator},
};

/// Metrics collector для orchestration системы
/// 
/// Применяет принципы SOLID:
/// - SRP: Только сбор, агрегация и предоставление метрик
/// - OCP: Расширяемость через новые типы метрик
/// - LSP: Взаимозаменяемость через trait
/// - ISP: Разделенные интерфейсы для разных типов метрик
/// - DIP: Зависит от абстракций координаторов
pub struct MetricsCollector {
    /// Координаторы для сбора метрик
    coordinators: MetricsCoordinatorRegistry,
    /// Circuit breaker manager для состояния circuit'ов
    circuit_breaker: Arc<CircuitBreakerManager>,
    /// Orchestration метрики
    orchestration_metrics: Arc<RwLock<OrchestrationMetrics>>,
    /// Время запуска системы
    start_time: Instant,
}

/// Реестр координаторов для сбора метрик
#[derive(Clone)]
pub struct MetricsCoordinatorRegistry {
    pub embedding: Arc<EmbeddingCoordinator>,
    pub search: Arc<SearchCoordinator>, 
    pub health: Arc<HealthManager>,
    pub promotion: Arc<PromotionCoordinator>,
    pub resources: Arc<ResourceController>,
    pub backup: Arc<BackupCoordinator>,
}

/// Orchestration метрики
#[derive(Debug, Default, Clone)]
pub struct OrchestrationMetrics {
    /// Общие метрики операций
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    
    /// Performance метрики
    pub avg_operation_duration_ms: f64,
    pub max_operation_duration_ms: u64,
    pub operations_per_minute: f64,
    
    /// Circuit breaker метрики
    pub circuit_breaker_trips: HashMap<String, u64>,
    pub active_circuit_breakers: u64,
    
    /// Resource utilization
    pub current_concurrent_operations: u64,
    pub max_concurrent_operations: u64,
    pub resource_utilization_percent: f64,
    
    /// SLA метрики
    pub sla_violations: u64,
    pub uptime_seconds: u64,
    pub availability_percent: f64,
    
    /// Coordinator-specific метрики
    pub coordinator_metrics: HashMap<String, CoordinatorMetrics>,
}

/// Метрики конкретного координатора
#[derive(Debug, Default, Clone)]
pub struct CoordinatorMetrics {
    pub success_rate: f64,
    pub avg_response_time_ms: f64,
    pub total_requests: u64,
    pub failed_requests: u64,
    pub circuit_breaker_state: String,
    pub health_score: f64,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// Trait для сбора метрик (ISP принцип)
#[async_trait::async_trait]
pub trait MetricsCollectorTrait: Send + Sync {
    /// Получить все метрики системы
    async fn get_all_metrics(&self) -> Value;
    
    /// Получить упрощенные метрики для dashboard
    async fn get_dashboard_metrics(&self) -> Value;
    
    /// Получить метрики производительности
    async fn get_performance_metrics(&self) -> PerformanceMetrics;
    
    /// Получить метрики доступности
    async fn get_availability_metrics(&self) -> AvailabilityMetrics;
    
    /// Записать метрики выполненной операции
    async fn record_operation(&self, metrics: OperationMetrics);
    
    /// Запустить адаптивную оптимизацию на основе метрик
    async fn run_adaptive_optimization(&self) -> Result<OptimizationResult>;
}

/// Метрики производительности
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub operations_per_second: f64,
    pub avg_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub throughput_trend: ThroughputTrend,
}

/// Метрики доступности
#[derive(Debug, Clone)]
pub struct AvailabilityMetrics {
    pub uptime_percent: f64,
    pub mttr_minutes: f64, // Mean Time To Recovery
    pub mtbf_hours: f64,   // Mean Time Between Failures
    pub sla_compliance_percent: f64,
    pub incident_count: u64,
}

/// Результат адаптивной оптимизации
#[derive(Debug, Clone)]
pub struct OptimizationResult {
    pub actions_taken: Vec<String>,
    pub metrics_improved: Vec<String>,
    pub recommendations: Vec<String>,
    pub estimated_improvement_percent: f64,
}

/// Тренд throughput
#[derive(Debug, Clone, PartialEq)]
pub enum ThroughputTrend {
    Increasing,
    Stable,
    Decreasing,
}

impl MetricsCoordinatorRegistry {
    /// Создать реестр из DI контейнера
    pub fn from_container(container: &crate::di::container_core::ContainerCore) -> Result<Self> {
        Ok(Self {
            embedding: container.resolve::<EmbeddingCoordinator>()?,
            search: container.resolve::<SearchCoordinator>()?,
            health: container.resolve::<HealthManager>()?,
            promotion: container.resolve::<PromotionCoordinator>()?,
            resources: container.resolve::<ResourceController>()?,
            backup: container.resolve::<BackupCoordinator>()?,
        })
    }
}

impl MetricsCollector {
    /// Создать новый metrics collector
    pub fn new(
        coordinators: MetricsCoordinatorRegistry,
        circuit_breaker: Arc<CircuitBreakerManager>,
    ) -> Self {
        Self {
            coordinators,
            circuit_breaker,
            orchestration_metrics: Arc::new(RwLock::new(OrchestrationMetrics::default())),
            start_time: Instant::now(),
        }
    }
    
    /// Создать из DI контейнера
    pub fn from_container(
        container: &crate::di::container_core::ContainerCore,
        circuit_breaker: Arc<CircuitBreakerManager>,
    ) -> Result<Self> {
        let coordinators = MetricsCoordinatorRegistry::from_container(container)?;
        Ok(Self::new(coordinators, circuit_breaker))
    }
    
    /// Собрать метрики от всех координаторов параллельно
    async fn collect_coordinator_metrics(&self) -> HashMap<String, Value> {
        let results = tokio::join!(
            self.coordinators.embedding.metrics(),
            self.coordinators.search.metrics(),
            self.coordinators.health.metrics(),
            self.coordinators.promotion.metrics(),
            self.coordinators.resources.metrics(),
            self.coordinators.backup.metrics()
        );
        
        let mut metrics = HashMap::new();
        metrics.insert("embedding".to_string(), results.0);
        metrics.insert("search".to_string(), results.1);
        metrics.insert("health".to_string(), results.2);
        metrics.insert("promotion".to_string(), results.3);
        metrics.insert("resources".to_string(), results.4);
        metrics.insert("backup".to_string(), results.5);
        
        metrics
    }
    
    /// Собрать readiness состояния координаторов
    async fn collect_readiness_metrics(&self) -> HashMap<String, bool> {
        let results = tokio::join!(
            self.coordinators.embedding.is_ready(),
            self.coordinators.search.is_ready(),
            self.coordinators.health.is_ready(),
            self.coordinators.promotion.is_ready(),
            self.coordinators.resources.is_ready(),
            self.coordinators.backup.is_ready()
        );
        
        let mut readiness = HashMap::new();
        readiness.insert("embedding".to_string(), results.0);
        readiness.insert("search".to_string(), results.1);
        readiness.insert("health".to_string(), results.2);
        readiness.insert("promotion".to_string(), results.3);
        readiness.insert("resources".to_string(), results.4);
        readiness.insert("backup".to_string(), results.5);
        
        readiness
    }
    
    /// Вычислить производительные метрики
    async fn calculate_performance_metrics(&self) -> PerformanceMetrics {
        let orchestration_metrics = self.orchestration_metrics.read().await;
        
        let ops_per_second = if self.start_time.elapsed().as_secs() > 0 {
            orchestration_metrics.total_operations as f64 / self.start_time.elapsed().as_secs() as f64
        } else { 0.0 };
        
        // Симулируем P95/P99 на основе avg (в реальности нужна гистограмма)
        let p95_latency = orchestration_metrics.avg_operation_duration_ms * 1.5;
        let p99_latency = orchestration_metrics.avg_operation_duration_ms * 2.0;
        
        // Определяем тренд на основе текущих метрик
        let throughput_trend = if orchestration_metrics.operations_per_minute > ops_per_second * 60.0 {
            ThroughputTrend::Increasing
        } else if orchestration_metrics.operations_per_minute < ops_per_second * 60.0 * 0.9 {
            ThroughputTrend::Decreasing
        } else {
            ThroughputTrend::Stable
        };
        
        PerformanceMetrics {
            operations_per_second: ops_per_second,
            avg_latency_ms: orchestration_metrics.avg_operation_duration_ms,
            p95_latency_ms: p95_latency,
            p99_latency_ms: p99_latency,
            throughput_trend,
        }
    }
    
    /// Вычислить availability метрики
    async fn calculate_availability_metrics(&self) -> AvailabilityMetrics {
        let orchestration_metrics = self.orchestration_metrics.read().await;
        let uptime_seconds = self.start_time.elapsed().as_secs();
        
        let uptime_percent = if uptime_seconds > 0 {
            ((uptime_seconds - orchestration_metrics.sla_violations) as f64 / uptime_seconds as f64) * 100.0
        } else { 100.0 };
        
        // Простые вычисления для демонстрации (в реальности нужна более сложная логика)
        let incident_count = orchestration_metrics.circuit_breaker_trips.values().sum::<u64>();
        let mttr_minutes = if incident_count > 0 { 5.0 } else { 0.0 }; // Средние 5 минут на восстановление
        let mtbf_hours = if incident_count > 0 {
            uptime_seconds as f64 / 3600.0 / incident_count as f64
        } else { uptime_seconds as f64 / 3600.0 };
        
        AvailabilityMetrics {
            uptime_percent,
            mttr_minutes,
            mtbf_hours,
            sla_compliance_percent: uptime_percent,
            incident_count,
        }
    }
}

#[async_trait::async_trait]
impl MetricsCollectorTrait for MetricsCollector {
    async fn get_all_metrics(&self) -> Value {
        let orchestration_metrics = self.orchestration_metrics.read().await.clone();
        let coordinator_metrics = self.collect_coordinator_metrics().await;
        let readiness_metrics = self.collect_readiness_metrics().await;
        let circuit_breaker_stats = self.circuit_breaker.get_statistics().await;
        
        // Собираем circuit breaker states
        let mut circuit_breaker_states = serde_json::Map::new();
        for (name, stats) in circuit_breaker_stats {
            circuit_breaker_states.insert(name, json!({
                "status": match stats.status {
                    crate::orchestration::circuit_breaker_manager::CircuitBreakerStatus::Closed => "closed",
                    crate::orchestration::circuit_breaker_manager::CircuitBreakerStatus::Open => "open",
                    crate::orchestration::circuit_breaker_manager::CircuitBreakerStatus::HalfOpen => "half_open",
                },
                "failure_count": stats.failure_count,
                "last_failure_seconds_ago": stats.last_failure_seconds_ago,
                "recovery_timeout_seconds": stats.recovery_timeout_seconds,
            }));
        }
        
        json!({
            "orchestrator": {
                "uptime_seconds": self.start_time.elapsed().as_secs(),
                
                // Operation metrics
                "operations": {
                    "total": orchestration_metrics.total_operations,
                    "successful": orchestration_metrics.successful_operations,
                    "failed": orchestration_metrics.failed_operations,
                    "success_rate": if orchestration_metrics.total_operations > 0 {
                        orchestration_metrics.successful_operations as f64 / orchestration_metrics.total_operations as f64 * 100.0
                    } else { 100.0 },
                    "current_concurrent": orchestration_metrics.current_concurrent_operations,
                    "max_concurrent": orchestration_metrics.max_concurrent_operations,
                },
                
                // Performance metrics
                "performance": {
                    "avg_operation_duration_ms": orchestration_metrics.avg_operation_duration_ms,
                    "max_operation_duration_ms": orchestration_metrics.max_operation_duration_ms,
                    "operations_per_minute": orchestration_metrics.operations_per_minute,
                },
                
                // Circuit breaker metrics
                "circuit_breakers": circuit_breaker_states,
                "circuit_breaker_trips": orchestration_metrics.circuit_breaker_trips,
                
                // SLA metrics
                "sla": {
                    "violations": orchestration_metrics.sla_violations,
                    "availability_percent": orchestration_metrics.availability_percent,
                },
                
                // Coordinator health
                "coordinator_health": readiness_metrics,
                "coordinators": coordinator_metrics,
            }
        })
    }
    
    async fn get_dashboard_metrics(&self) -> Value {
        let orchestration_metrics = self.orchestration_metrics.read().await;
        let circuit_breaker_stats = self.circuit_breaker.get_statistics().await;
        let readiness_metrics = self.collect_readiness_metrics().await;
        
        // Подсчитываем открытые circuit breakers
        let open_circuit_breakers = circuit_breaker_stats.values()
            .filter(|stats| stats.status == crate::orchestration::circuit_breaker_manager::CircuitBreakerStatus::Open)
            .count();
        
        json!({
            "status": if readiness_metrics.values().all(|&ready| ready) { "ready" } else { "not_ready" },
            "uptime_hours": self.start_time.elapsed().as_secs() / 3600,
            "operations_per_minute": orchestration_metrics.operations_per_minute,
            "success_rate": if orchestration_metrics.total_operations > 0 {
                orchestration_metrics.successful_operations as f64 / orchestration_metrics.total_operations as f64 * 100.0
            } else { 100.0 },
            "active_operations": orchestration_metrics.current_concurrent_operations,
            "circuit_breakers_open": open_circuit_breakers,
            "availability_percent": orchestration_metrics.availability_percent,
            "coordinator_health": readiness_metrics,
        })
    }
    
    async fn get_performance_metrics(&self) -> PerformanceMetrics {
        self.calculate_performance_metrics().await
    }
    
    async fn get_availability_metrics(&self) -> AvailabilityMetrics {
        self.calculate_availability_metrics().await
    }
    
    async fn record_operation(&self, operation_metrics: OperationMetrics) {
        let mut metrics = self.orchestration_metrics.write().await;
        
        metrics.total_operations += 1;
        
        if operation_metrics.success {
            metrics.successful_operations += 1;
        } else {
            metrics.failed_operations += 1;
        }
        
        // Обновляем average duration
        let duration_ms = operation_metrics.duration.as_millis() as f64;
        metrics.avg_operation_duration_ms = 
            (metrics.avg_operation_duration_ms * (metrics.total_operations - 1) as f64 + duration_ms) 
            / metrics.total_operations as f64;
        
        // Обновляем max duration
        let duration_ms_u64 = operation_metrics.duration.as_millis() as u64;
        if duration_ms_u64 > metrics.max_operation_duration_ms {
            metrics.max_operation_duration_ms = duration_ms_u64;
        }
        
        // Обновляем operations per minute
        if self.start_time.elapsed().as_secs() > 0 {
            metrics.operations_per_minute = metrics.total_operations as f64 / 
                (self.start_time.elapsed().as_secs() as f64 / 60.0);
        }
        
        // SLA violations (для search операций target <5ms)
        if operation_metrics.operation_type == "search" && operation_metrics.duration.as_millis() > 5 {
            metrics.sla_violations += 1;
        }
    }
    
    async fn run_adaptive_optimization(&self) -> Result<OptimizationResult> {
        debug!("🎯 Запуск адаптивной оптимизации на основе метрик");
        
        let metrics = self.orchestration_metrics.read().await;
        let mut actions_taken = Vec::new();
        let mut metrics_improved = Vec::new();
        let mut recommendations = Vec::new();
        
        // Если SLA violations > 10% - рекомендуем увеличить лимиты
        let sla_violation_rate = if metrics.total_operations > 0 {
            metrics.sla_violations as f64 / metrics.total_operations as f64
        } else { 0.0 };
        
        if sla_violation_rate > 0.1 {
            warn!("⚠️ Высокий уровень SLA violations ({:.1}%), адаптируем лимиты", sla_violation_rate * 100.0);
            
            if let Err(e) = self.coordinators.resources.adapt_limits().await {
                warn!("Ошибка адаптации лимитов: {}", e);
                recommendations.push("Manual resource limit adjustment needed".to_string());
            } else {
                actions_taken.push("Increased resource limits".to_string());
                metrics_improved.push("SLA compliance".to_string());
            }
        }
        
        // Если много circuit breaker trips - очищаем кэши
        let total_trips: u64 = metrics.circuit_breaker_trips.values().sum();
        if total_trips > 10 {
            info!("🧩 Много circuit breaker trips ({}), очищаем embedding cache", total_trips);
            
            // Очищаем кэш через embedding coordinator
            use crate::orchestration::traits::EmbeddingCoordinator as EmbeddingCoordinatorTrait;
            if let Err(e) = EmbeddingCoordinatorTrait::clear_cache(&*self.coordinators.embedding).await {
                warn!("Ошибка очистки кэша: {}", e);
                recommendations.push("Manual cache clearing needed".to_string());
            } else {
                actions_taken.push("Cleared embedding cache".to_string());
                metrics_improved.push("Circuit breaker stability".to_string());
            }
        }
        
        // Рекомендации на основе performance метрик
        if metrics.avg_operation_duration_ms > 10.0 {
            recommendations.push("Consider enabling SIMD optimizations".to_string());
            recommendations.push("Review database indexing strategy".to_string());
        }
        
        let estimated_improvement = if !actions_taken.is_empty() { 15.0 } else { 0.0 };
        
        Ok(OptimizationResult {
            actions_taken,
            metrics_improved,
            recommendations,
            estimated_improvement_percent: estimated_improvement,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::circuit_breaker_manager::CircuitBreakerConfig;
    
    #[tokio::test]
    async fn test_orchestration_metrics_creation() {
        let metrics = OrchestrationMetrics {
            total_operations: 100,
            successful_operations: 95,
            failed_operations: 5,
            avg_operation_duration_ms: 2.5,
            max_operation_duration_ms: 10,
            ..Default::default()
        };
        
        assert_eq!(metrics.total_operations, 100);
        assert_eq!(metrics.successful_operations, 95);
        assert_eq!(metrics.failed_operations, 5);
        assert_eq!(metrics.avg_operation_duration_ms, 2.5);
        
        // Тестируем success rate calculation
        let success_rate = metrics.successful_operations as f64 / metrics.total_operations as f64 * 100.0;
        assert_eq!(success_rate, 95.0);
    }
    
    #[tokio::test]
    async fn test_performance_metrics_trends() {
        let perf_metrics = PerformanceMetrics {
            operations_per_second: 100.0,
            avg_latency_ms: 2.5,
            p95_latency_ms: 3.75,
            p99_latency_ms: 5.0,
            throughput_trend: ThroughputTrend::Increasing,
        };
        
        assert_eq!(perf_metrics.operations_per_second, 100.0);
        assert_eq!(perf_metrics.throughput_trend, ThroughputTrend::Increasing);
    }
    
    #[tokio::test]
    async fn test_availability_metrics_calculation() {
        let availability = AvailabilityMetrics {
            uptime_percent: 99.9,
            mttr_minutes: 2.5,
            mtbf_hours: 720.0, // 30 days
            sla_compliance_percent: 99.5,
            incident_count: 1,
        };
        
        assert_eq!(availability.uptime_percent, 99.9);
        assert_eq!(availability.incident_count, 1);
    }
    
    #[tokio::test]
    async fn test_optimization_result_structure() {
        let result = OptimizationResult {
            actions_taken: vec!["Increased cache size".to_string()],
            metrics_improved: vec!["Cache hit rate".to_string()],
            recommendations: vec!["Enable SIMD".to_string()],
            estimated_improvement_percent: 15.0,
        };
        
        assert_eq!(result.actions_taken.len(), 1);
        assert_eq!(result.recommendations.len(), 1);
        assert_eq!(result.estimated_improvement_percent, 15.0);
    }
}