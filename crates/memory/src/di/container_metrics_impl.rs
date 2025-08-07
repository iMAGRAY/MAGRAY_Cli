//! Container Metrics Implementation - сбор метрик производительности DI контейнера
//!
//! Отделен от unified_container_impl.rs для следования Single Responsibility Principle.
//! Отвечает ТОЛЬКО за сбор, агрегацию и предоставление метрик производительности.

use parking_lot::RwLock;
use std::{
    any::TypeId,
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
use tracing::{debug, info};

use super::{
    core_traits::{CacheStats, ContainerMetrics, ResolutionStats},
    errors::DIError,
};

/// Container Metrics Implementation - отвечает ТОЛЬКО за сбор метрик производительности
///
/// ПРИНЦИПЫ:
/// - SRP: единственная ответственность - сбор и агрегация метрик
/// - OCP: расширяемость через различные типы метрик
/// - LSP: соответствует интерфейсу ContainerMetrics
/// - ISP: минимальный интерфейс только для метрик
/// - DIP: не зависит от конкретных реализаций контейнера
pub struct ContainerMetricsImpl {
    /// Конфигурация метрик
    config: MetricsConfig,
    /// Общие счетчики (lock-free для производительности)
    counters: MetricsCounters,
    /// Детальные метрики по типам (с блокировкой для консистентности)
    type_metrics: RwLock<HashMap<TypeId, TypeMetrics>>,
    /// Статистика производительности
    performance_stats: RwLock<PerformanceStats>,
    /// Время создания metrics collector
    created_at: Instant,
}

/// Конфигурация сбора метрик
#[derive(Debug, Clone)]
pub struct MetricsConfig {
    /// Включить сбор детальных метрик по типам
    pub collect_type_metrics: bool,
    /// Включить сбор timing информации
    pub collect_timing: bool,
    /// Включить сбор статистики кэша
    pub collect_cache_stats: bool,
    /// Максимальное количество типов для детальных метрик
    pub max_tracked_types: usize,
    /// Интервал агрегации метрик
    pub aggregation_interval: Duration,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            collect_type_metrics: true,
            collect_timing: true,
            collect_cache_stats: true,
            max_tracked_types: 10_000,
            aggregation_interval: Duration::from_secs(60),
        }
    }
}

impl MetricsConfig {
    /// Production конфигурация с минимальным overhead
    pub fn production() -> Self {
        Self {
            collect_type_metrics: false, // Отключаем для производительности
            collect_timing: true,
            collect_cache_stats: true,
            max_tracked_types: 1_000,
            aggregation_interval: Duration::from_secs(300), // 5 минут
        }
    }

    /// Development конфигурация с полным набором метрик
    pub fn development() -> Self {
        Self {
            collect_type_metrics: true,
            collect_timing: true,
            collect_cache_stats: true,
            max_tracked_types: 50_000,
            aggregation_interval: Duration::from_secs(30),
        }
    }

    /// Minimal конфигурация для тестов
    pub fn minimal() -> Self {
        Self {
            collect_type_metrics: false,
            collect_timing: false,
            collect_cache_stats: false,
            max_tracked_types: 100,
            aggregation_interval: Duration::from_secs(10),
        }
    }
}

/// Lock-free счетчики для high-performance метрик
#[derive(Debug)]
struct MetricsCounters {
    /// Общее количество разрешений
    total_resolutions: AtomicU64,
    /// Успешные разрешения
    successful_resolutions: AtomicU64,
    /// Неудачные разрешения
    failed_resolutions: AtomicU64,
    /// Cache hits
    cache_hits: AtomicU64,
    /// Cache misses  
    cache_misses: AtomicU64,
    /// Количество ошибок
    error_count: AtomicU64,
}

impl Default for MetricsCounters {
    fn default() -> Self {
        Self {
            total_resolutions: AtomicU64::new(0),
            successful_resolutions: AtomicU64::new(0),
            failed_resolutions: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
        }
    }
}

/// Метрики для конкретного типа
#[derive(Debug, Clone)]
pub struct TypeMetrics {
    /// Количество разрешений этого типа
    pub resolutions: u64,
    /// Количество неудач при разрешении
    pub failures: u64,
    /// Общее время всех разрешений
    pub total_time: Duration,
    /// Минимальное время разрешения
    pub min_time: Duration,
    /// Максимальное время разрешения
    pub max_time: Duration,
    /// Последнее время разрешения
    pub last_resolution: Option<Instant>,
    /// Cache hits для этого типа
    pub cache_hits: u64,
    /// Имя типа для отладки
    pub type_name: String,
}

impl TypeMetrics {
    /// Создать новые метрики для типа
    pub fn new(type_name: String) -> Self {
        Self {
            resolutions: 0,
            failures: 0,
            total_time: Duration::ZERO,
            min_time: Duration::MAX,
            max_time: Duration::ZERO,
            last_resolution: None,
            cache_hits: 0,
            type_name,
        }
    }

    /// Записать успешное разрешение
    pub fn record_success(&mut self, duration: Duration) {
        self.resolutions += 1;
        self.total_time += duration;
        self.last_resolution = Some(Instant::now());

        if duration < self.min_time {
            self.min_time = duration;
        }
        if duration > self.max_time {
            self.max_time = duration;
        }
    }

    /// Записать неудачное разрешение
    pub fn record_failure(&mut self, duration: Duration) {
        self.failures += 1;
        self.total_time += duration;
        self.last_resolution = Some(Instant::now());
    }

    /// Записать cache hit
    pub fn record_cache_hit(&mut self) {
        self.cache_hits += 1;
    }

    /// Получить среднее время разрешения
    pub fn avg_time(&self) -> Duration {
        if self.resolutions > 0 {
            self.total_time / self.resolutions as u32
        } else {
            Duration::ZERO
        }
    }

    /// Получить success rate для этого типа
    pub fn success_rate(&self) -> f64 {
        let total = self.resolutions + self.failures;
        if total > 0 {
            (self.resolutions as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    }
}

/// Общая статистика производительности
#[derive(Debug, Default)]
struct PerformanceStats {
    /// Общее время работы collector
    total_runtime: Duration,
    /// Общее время всех разрешений
    total_resolution_time: Duration,
    /// Максимальное время одного разрешения
    max_single_resolution_time: Duration,
    /// Количество агрегаций
    aggregation_count: u64,
    /// Время последней агрегации
    last_aggregation: Option<Instant>,
}

impl ContainerMetricsImpl {
    /// Создать новый metrics collector
    pub fn new(config: MetricsConfig) -> Self {
        info!(
            "📊 Создание ContainerMetricsImpl с конфигурацией: {:?}",
            config
        );

        Self {
            config,
            counters: MetricsCounters::default(),
            type_metrics: RwLock::new(HashMap::new()),
            performance_stats: RwLock::new(PerformanceStats::default()),
            created_at: Instant::now(),
        }
    }

    /// Создать metrics collector с default конфигурацией
    pub fn default() -> Self {
        Self::new(MetricsConfig::default())
    }

    /// Записать успешное разрешение
    pub fn record_resolution_success(&self, type_id: TypeId, duration_ns: u64) {
        // Обновляем общие счетчики (lock-free)
        self.counters
            .total_resolutions
            .fetch_add(1, Ordering::Relaxed);
        self.counters
            .successful_resolutions
            .fetch_add(1, Ordering::Relaxed);

        let duration = Duration::from_nanos(duration_ns);

        // Если включен сбор timing информации
        if self.config.collect_timing {
            let mut perf_stats = self.performance_stats.write();
            perf_stats.total_resolution_time += duration;

            if duration > perf_stats.max_single_resolution_time {
                perf_stats.max_single_resolution_time = duration;
            }
        }

        // Если включен сбор метрик по типам
        if self.config.collect_type_metrics {
            self.update_type_metrics(type_id, |metrics| {
                metrics.record_success(duration);
            });
        }

        if self.config.collect_timing {
            debug!(
                "📊 Записано успешное разрешение {:?} за {:?}",
                type_id, duration
            );
        }
    }

    /// Записать неудачное разрешение
    pub fn record_resolution_failure(&self, type_id: TypeId, error: &DIError) {
        // Обновляем общие счетчики
        self.counters
            .total_resolutions
            .fetch_add(1, Ordering::Relaxed);
        self.counters
            .failed_resolutions
            .fetch_add(1, Ordering::Relaxed);
        self.counters.error_count.fetch_add(1, Ordering::Relaxed);

        // Если включен сбор метрик по типам
        if self.config.collect_type_metrics {
            self.update_type_metrics(type_id, |metrics| {
                metrics.record_failure(Duration::ZERO); // Не знаем время для failed resolution
            });
        }

        debug!("📊 Записана ошибка разрешения {:?}: {}", type_id, error);
    }

    /// Записать cache hit
    pub fn record_cache_hit(&self, type_id: TypeId) {
        // Обновляем общие счетчики
        self.counters.cache_hits.fetch_add(1, Ordering::Relaxed);

        // Если включены метрики по типам
        if self.config.collect_type_metrics {
            self.update_type_metrics(type_id, |metrics| {
                metrics.record_cache_hit();
            });
        }

        if self.config.collect_cache_stats {
            debug!("📊 Записан cache hit для {:?}", type_id);
        }
    }

    /// Получить статистику разрешений
    pub fn get_resolution_stats(&self) -> ResolutionStats {
        let total_resolutions = self.counters.total_resolutions.load(Ordering::Relaxed);
        let successful_resolutions = self.counters.successful_resolutions.load(Ordering::Relaxed);
        let failed_resolutions = self.counters.failed_resolutions.load(Ordering::Relaxed);

        let (avg_resolution_time_ns, max_resolution_time_ns) = if self.config.collect_timing {
            let perf_stats = self.performance_stats.read();
            let avg = if total_resolutions > 0 {
                perf_stats.total_resolution_time.as_nanos() as u64 / total_resolutions
            } else {
                0
            };
            (avg, perf_stats.max_single_resolution_time.as_nanos() as u64)
        } else {
            (0, 0)
        };

        let resolutions_by_type = if self.config.collect_type_metrics {
            let type_metrics = self.type_metrics.read();
            type_metrics
                .iter()
                .map(|(&type_id, metrics)| (type_id, metrics.resolutions))
                .collect()
        } else {
            HashMap::new()
        };

        ResolutionStats {
            total_resolutions,
            successful_resolutions,
            failed_resolutions,
            avg_resolution_time_ns,
            max_resolution_time_ns,
            resolutions_by_type,
        }
    }

    /// Получить статистику кэша
    pub fn get_cache_stats(&self) -> CacheStats {
        let cache_hits = self.counters.cache_hits.load(Ordering::Relaxed);
        let cache_misses = self.counters.cache_misses.load(Ordering::Relaxed);
        let total_requests = cache_hits + cache_misses;

        CacheStats {
            cache_hits,
            cache_misses,
            cache_size: 0, // TODO: Получить из ContainerCache если нужно
            cache_hit_rate: if total_requests > 0 {
                (cache_hits as f64 / total_requests as f64) * 100.0
            } else {
                0.0
            },
        }
    }

    /// Сбросить все метрики
    pub fn reset_metrics(&self) {
        // Сбрасываем счетчики
        self.counters.total_resolutions.store(0, Ordering::Relaxed);
        self.counters
            .successful_resolutions
            .store(0, Ordering::Relaxed);
        self.counters.failed_resolutions.store(0, Ordering::Relaxed);
        self.counters.cache_hits.store(0, Ordering::Relaxed);
        self.counters.cache_misses.store(0, Ordering::Relaxed);
        self.counters.error_count.store(0, Ordering::Relaxed);

        // Сбрасываем детальные метрики
        if self.config.collect_type_metrics {
            let mut type_metrics = self.type_metrics.write();
            type_metrics.clear();
        }

        // Сбрасываем статистику производительности
        if self.config.collect_timing {
            let mut perf_stats = self.performance_stats.write();
            *perf_stats = PerformanceStats::default();
        }

        info!("🔄 Все метрики ContainerMetrics сброшены");
    }

    /// Получить детальный отчет о метриках
    pub fn get_detailed_report(&self) -> String {
        let resolution_stats = self.get_resolution_stats();
        let cache_stats = self.get_cache_stats();
        let uptime = self.created_at.elapsed();

        let mut report = format!(
            "=== Container Metrics Detailed Report ===\n\
             Uptime: {:?}\n\
             \n\
             Resolution Statistics:\n\
             - Total resolutions: {}\n\
             - Successful: {} ({:.1}%)\n\
             - Failed: {} ({:.1}%)\n\
             - Average time: {:.2}μs\n\
             - Max time: {:.2}μs\n\
             \n\
             Cache Statistics:\n\
             - Hits: {} ({:.1}%)\n\
             - Misses: {}\n\
             - Hit rate: {:.1}%\n",
            uptime,
            resolution_stats.total_resolutions,
            resolution_stats.successful_resolutions,
            if resolution_stats.total_resolutions > 0 {
                (resolution_stats.successful_resolutions as f64
                    / resolution_stats.total_resolutions as f64)
                    * 100.0
            } else {
                0.0
            },
            resolution_stats.failed_resolutions,
            if resolution_stats.total_resolutions > 0 {
                (resolution_stats.failed_resolutions as f64
                    / resolution_stats.total_resolutions as f64)
                    * 100.0
            } else {
                0.0
            },
            resolution_stats.avg_resolution_time_ns as f64 / 1000.0, // ns to μs
            resolution_stats.max_resolution_time_ns as f64 / 1000.0, // ns to μs
            cache_stats.cache_hits,
            cache_stats.cache_hit_rate,
            cache_stats.cache_misses,
            cache_stats.cache_hit_rate
        );

        // Добавляем топ типов по использованию если включены детальные метрики
        if self.config.collect_type_metrics {
            let type_metrics = self.type_metrics.read();

            if !type_metrics.is_empty() {
                report.push_str("\nTop 10 Most Used Types:\n");

                let mut sorted_types: Vec<_> = type_metrics.values().collect();
                sorted_types.sort_by(|a, b| b.resolutions.cmp(&a.resolutions));

                for (i, metrics) in sorted_types.iter().take(10).enumerate() {
                    report.push_str(&format!(
                        "{}. {} - {} resolutions (avg: {:?})\n",
                        i + 1,
                        metrics.type_name,
                        metrics.resolutions,
                        metrics.avg_time()
                    ));
                }
            }
        }

        report.push_str("========================================");
        report
    }

    /// Запустить background задачу агрегации метрик
    pub fn start_aggregation_task(metrics: std::sync::Arc<Self>) -> tokio::task::JoinHandle<()> {
        let aggregation_interval = metrics.config.aggregation_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(aggregation_interval);

            loop {
                interval.tick().await;
                metrics.aggregate_metrics();
            }
        })
    }

    /// Validate состояние метрик
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut warnings = Vec::new();
        let stats = self.get_resolution_stats();

        // Проверяем success rate
        if stats.total_resolutions > 100 {
            let success_rate =
                (stats.successful_resolutions as f64 / stats.total_resolutions as f64) * 100.0;
            if success_rate < 95.0 {
                warnings.push(format!(
                    "Низкий success rate: {:.1}% (рекомендуется > 95%)",
                    success_rate
                ));
            }
        }

        // Проверяем производительность
        if self.config.collect_timing && stats.avg_resolution_time_ns > 10_000_000 {
            // 10ms
            warnings.push(format!(
                "Медленные разрешения: среднее время {:.2}ms (рекомендуется < 10ms)",
                stats.avg_resolution_time_ns as f64 / 1_000_000.0
            ));
        }

        // Проверяем cache hit rate
        let cache_stats = self.get_cache_stats();
        let total_cache_requests = cache_stats.cache_hits + cache_stats.cache_misses;
        if total_cache_requests > 100 && cache_stats.cache_hit_rate < 70.0 {
            warnings.push(format!(
                "Низкий cache hit rate: {:.1}% (рекомендуется > 70%)",
                cache_stats.cache_hit_rate
            ));
        }

        if warnings.is_empty() {
            Ok(())
        } else {
            Err(warnings)
        }
    }

    /// Получить конфигурацию метрик
    pub fn get_config(&self) -> &MetricsConfig {
        &self.config
    }

    // === PRIVATE HELPER METHODS ===

    /// Обновить метрики для конкретного типа
    fn update_type_metrics<F>(&self, type_id: TypeId, updater: F)
    where
        F: FnOnce(&mut TypeMetrics),
    {
        let mut type_metrics = self.type_metrics.write();

        // Проверяем лимит на количество отслеживаемых типов
        if type_metrics.len() >= self.config.max_tracked_types
            && !type_metrics.contains_key(&type_id)
        {
            // Удаляем наименее используемые типы если превышен лимит
            self.evict_least_used_types(&mut type_metrics);
        }

        let metrics = type_metrics
            .entry(type_id)
            .or_insert_with(|| TypeMetrics::new(format!("Type({:?})", type_id)));

        updater(metrics);
    }

    /// Удалить наименее используемые типы из метрик
    fn evict_least_used_types(&self, type_metrics: &mut HashMap<TypeId, TypeMetrics>) {
        let target_size = (self.config.max_tracked_types as f64 * 0.8) as usize;

        if type_metrics.len() <= target_size {
            return;
        }

        // Сортируем по количеству использований
        let mut types_by_usage: Vec<_> = type_metrics
            .iter()
            .map(|(&type_id, metrics)| (type_id, metrics.resolutions))
            .collect();

        types_by_usage.sort_by(|a, b| a.1.cmp(&b.1)); // Сортировка по возрастанию

        // Удаляем наименее используемые
        let to_remove = type_metrics.len() - target_size;
        for (type_id, _) in types_by_usage.into_iter().take(to_remove) {
            type_metrics.remove(&type_id);
        }

        debug!(
            "🧹 Удалено {} наименее используемых типов из метрик",
            to_remove
        );
    }

    /// Выполнить агрегацию метрик
    fn aggregate_metrics(&self) {
        debug!("📊 Выполняется агрегация метрик...");

        let mut perf_stats = self.performance_stats.write();
        perf_stats.aggregation_count += 1;
        perf_stats.last_aggregation = Some(Instant::now());
        perf_stats.total_runtime = self.created_at.elapsed();

        // TODO: Добавить дополнительную логику агрегации при необходимости
        // Например, расчет процентилей, сброс временных метрик и т.д.
    }
}

// Реализация ContainerMetrics trait
impl ContainerMetrics for ContainerMetricsImpl {
    fn record_resolution_success(&self, type_id: TypeId, duration_ns: u64) {
        self.record_resolution_success(type_id, duration_ns);
    }

    fn record_resolution_failure(&self, type_id: TypeId, error: &DIError) {
        self.record_resolution_failure(type_id, error);
    }

    fn record_cache_hit(&self, type_id: TypeId) {
        self.record_cache_hit(type_id);
    }

    fn get_resolution_stats(&self) -> ResolutionStats {
        self.get_resolution_stats()
    }

    fn get_cache_stats(&self) -> CacheStats {
        self.get_cache_stats()
    }

    fn reset_metrics(&self) {
        self.reset_metrics();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = ContainerMetricsImpl::new(MetricsConfig::minimal());
        let stats = metrics.get_resolution_stats();

        assert_eq!(stats.total_resolutions, 0);
        assert_eq!(stats.successful_resolutions, 0);
        assert_eq!(stats.failed_resolutions, 0);
    }

    #[test]
    fn test_record_success() {
        let metrics = ContainerMetricsImpl::new(MetricsConfig::development());
        let type_id = TypeId::of::<String>();

        metrics.record_resolution_success(type_id, 1_000_000); // 1ms in ns

        let stats = metrics.get_resolution_stats();
        assert_eq!(stats.total_resolutions, 1);
        assert_eq!(stats.successful_resolutions, 1);
        assert_eq!(stats.failed_resolutions, 0);
        assert_eq!(stats.avg_resolution_time_ns, 1_000_000);
    }

    #[test]
    fn test_record_failure() {
        let metrics = ContainerMetricsImpl::new(MetricsConfig::development());
        let type_id = TypeId::of::<String>();
        let error = DIError::TypeNotRegistered { type_id };

        metrics.record_resolution_failure(type_id, &error);

        let stats = metrics.get_resolution_stats();
        assert_eq!(stats.total_resolutions, 1);
        assert_eq!(stats.successful_resolutions, 0);
        assert_eq!(stats.failed_resolutions, 1);
    }

    #[test]
    fn test_cache_hit_recording() {
        let metrics = ContainerMetricsImpl::new(MetricsConfig::development());
        let type_id = TypeId::of::<String>();

        metrics.record_cache_hit(type_id);

        let cache_stats = metrics.get_cache_stats();
        assert_eq!(cache_stats.cache_hits, 1);
        assert_eq!(cache_stats.cache_misses, 0);
    }

    #[test]
    fn test_metrics_reset() {
        let metrics = ContainerMetricsImpl::new(MetricsConfig::development());
        let type_id = TypeId::of::<String>();

        // Записываем некоторые метрики
        metrics.record_resolution_success(type_id, 1_000_000);
        metrics.record_cache_hit(type_id);

        // Проверяем что они записались
        let stats_before = metrics.get_resolution_stats();
        assert_eq!(stats_before.total_resolutions, 1);

        let cache_stats_before = metrics.get_cache_stats();
        assert_eq!(cache_stats_before.cache_hits, 1);

        // Сбрасываем
        metrics.reset_metrics();

        // Проверяем что сбросились
        let stats_after = metrics.get_resolution_stats();
        assert_eq!(stats_after.total_resolutions, 0);

        let cache_stats_after = metrics.get_cache_stats();
        assert_eq!(cache_stats_after.cache_hits, 0);
    }

    #[test]
    fn test_detailed_report() {
        let metrics = ContainerMetricsImpl::new(MetricsConfig::development());
        let type_id = TypeId::of::<String>();

        // Добавляем некоторые метрики
        metrics.record_resolution_success(type_id, 1_000_000);
        metrics.record_cache_hit(type_id);

        let report = metrics.get_detailed_report();
        assert!(report.contains("Container Metrics Detailed Report"));
        assert!(report.contains("Total resolutions: 1"));
        assert!(report.contains("Hits: 1"));
    }

    #[test]
    fn test_metrics_validation() {
        let metrics = ContainerMetricsImpl::new(MetricsConfig::minimal());

        // Новые метрики должны быть валидными
        assert!(metrics.validate().is_ok());
    }

    #[test]
    fn test_type_metrics_with_limit() {
        let config = MetricsConfig {
            collect_type_metrics: true,
            max_tracked_types: 2,
            ..MetricsConfig::minimal()
        };
        let metrics = ContainerMetricsImpl::new(config);

        // Записываем метрики для 3 разных типов
        metrics.record_resolution_success(TypeId::of::<String>(), 1_000_000);
        metrics.record_resolution_success(TypeId::of::<i32>(), 1_000_000);
        metrics.record_resolution_success(TypeId::of::<f64>(), 1_000_000);

        // Проверяем что система справляется с превышением лимита
        let stats = metrics.get_resolution_stats();
        assert_eq!(stats.total_resolutions, 3);
    }
}
