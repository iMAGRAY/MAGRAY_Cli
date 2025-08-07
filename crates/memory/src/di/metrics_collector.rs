use parking_lot::RwLock;
use std::{
    any::TypeId,
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
use tracing::debug;

use super::traits::{DIContainerStats, DIPerformanceMetrics, MetricsReporter, TypeMetrics};

/// Атомарные счетчики для thread-safe метрик
struct AtomicCounters {
    total_resolutions: AtomicU64,
    total_registrations: AtomicU64,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
}

impl AtomicCounters {
    fn new() -> Self {
        Self {
            total_resolutions: AtomicU64::new(0),
            total_registrations: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
        }
    }

    fn reset(&self) {
        self.total_resolutions.store(0, Ordering::Relaxed);
        self.total_registrations.store(0, Ordering::Relaxed);
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
    }
}

/// Реализация сборщика метрик производительности
/// Применяет принцип Single Responsibility (SRP)
/// Применяет принцип Dependency Inversion (DIP) через trait
pub struct MetricsReporterImpl {
    /// Атомарные счетчики для основных метрик
    counters: AtomicCounters,
    /// Детальные метрики по типам
    type_metrics: RwLock<HashMap<TypeId, TypeMetricsImpl>>,
    /// Общая статистика времени
    timing_stats: RwLock<TimingStats>,
    /// Имена типов для отчетов
    type_names: RwLock<HashMap<TypeId, String>>,
}

#[derive(Debug, Clone)]
struct TypeMetricsImpl {
    resolutions: u64,
    total_time: Duration,
    cache_hits: u64,
    last_resolution: Option<Instant>,
    error_count: u64,
}

impl TypeMetricsImpl {
    fn new() -> Self {
        Self {
            resolutions: 0,
            total_time: Duration::from_nanos(0),
            cache_hits: 0,
            last_resolution: None,
            error_count: 0,
        }
    }

    fn record_resolution(&mut self, duration: Duration, from_cache: bool) {
        self.resolutions += 1;
        self.total_time += duration;
        if from_cache {
            self.cache_hits += 1;
        }
        self.last_resolution = Some(Instant::now());
    }

    fn average_time(&self) -> Duration {
        if self.resolutions > 0 {
            self.total_time / self.resolutions as u32
        } else {
            Duration::from_nanos(0)
        }
    }

    fn to_public(&self) -> TypeMetrics {
        TypeMetrics {
            resolutions: self.resolutions,
            total_time: self.total_time,
            cache_hits: self.cache_hits,
            average_time: self.average_time(),
            last_resolution: self.last_resolution,
            error_count: self.error_count,
        }
    }
}

#[derive(Debug, Default)]
struct TimingStats {
    total_resolution_time: Duration,
    min_resolution_time: Option<Duration>,
    max_resolution_time: Option<Duration>,
    recent_resolutions: Vec<Duration>, // Последние 100 разрешений для расчета percentiles
}

impl TimingStats {
    fn record_duration(&mut self, duration: Duration) {
        self.total_resolution_time += duration;

        // Обновляем min/max
        if let Some(current_min) = self.min_resolution_time {
            if duration < current_min {
                self.min_resolution_time = Some(duration);
            }
        } else {
            self.min_resolution_time = Some(duration);
        }

        if let Some(current_max) = self.max_resolution_time {
            if duration > current_max {
                self.max_resolution_time = Some(duration);
            }
        } else {
            self.max_resolution_time = Some(duration);
        }

        // Сохраняем для percentiles (ограничиваем размер массива)
        self.recent_resolutions.push(duration);
        if self.recent_resolutions.len() > 100 {
            self.recent_resolutions.remove(0);
        }
    }

    fn get_percentile(&self, percentile: f64) -> Option<Duration> {
        if self.recent_resolutions.is_empty() {
            return None;
        }

        let mut sorted = self.recent_resolutions.clone();
        sorted.sort();

        let index = (sorted.len() as f64 * percentile / 100.0) as usize;
        sorted.get(index.min(sorted.len() - 1)).copied()
    }

    fn reset(&mut self) {
        self.total_resolution_time = Duration::from_nanos(0);
        self.min_resolution_time = None;
        self.max_resolution_time = None;
        self.recent_resolutions.clear();
    }
}

impl MetricsReporterImpl {
    pub fn new() -> Self {
        Self {
            counters: AtomicCounters::new(),
            type_metrics: RwLock::new(HashMap::new()),
            timing_stats: RwLock::new(TimingStats::default()),
            type_names: RwLock::new(HashMap::new()),
        }
    }

    /// Получить детальные timing статистики
    pub fn get_timing_stats(&self) -> TimingStatsReport {
        let stats = self.timing_stats.read();

        TimingStatsReport {
            total_time: stats.total_resolution_time,
            min_time: stats.min_resolution_time,
            max_time: stats.max_resolution_time,
            p50: stats.get_percentile(50.0),
            p95: stats.get_percentile(95.0),
            p99: stats.get_percentile(99.0),
            sample_count: stats.recent_resolutions.len(),
        }
    }

    /// Получить топ N самых медленных типов
    pub fn get_slowest_types(&self, limit: usize) -> Vec<(String, TypeMetrics)> {
        let type_metrics = self.type_metrics.read();
        let type_names = self.type_names.read();

        let mut types: Vec<_> = type_metrics
            .iter()
            .map(|(&type_id, metrics)| {
                let name = type_names
                    .get(&type_id)
                    .cloned()
                    .unwrap_or_else(|| format!("Unknown({:?})", type_id));
                (name, metrics.to_public())
            })
            .collect();

        types.sort_by(|a, b| b.1.average_time.cmp(&a.1.average_time));
        types.into_iter().take(limit).collect()
    }

    /// Получить топ N наиболее используемых типов
    pub fn get_most_used_types(&self, limit: usize) -> Vec<(String, TypeMetrics)> {
        let type_metrics = self.type_metrics.read();
        let type_names = self.type_names.read();

        let mut types: Vec<_> = type_metrics
            .iter()
            .map(|(&type_id, metrics)| {
                let name = type_names
                    .get(&type_id)
                    .cloned()
                    .unwrap_or_else(|| format!("Unknown({:?})", type_id));
                (name, metrics.to_public())
            })
            .collect();

        types.sort_by(|a, b| b.1.resolutions.cmp(&a.1.resolutions));
        types.into_iter().take(limit).collect()
    }

    /// Зарегистрировать имя типа для отчетов
    pub fn register_type_name(&self, type_id: TypeId, name: String) {
        let mut type_names = self.type_names.write();
        type_names.insert(type_id, name);
    }

    /// Получить cache hit rate в процентах
    pub fn get_cache_hit_rate(&self) -> f64 {
        let hits = self.counters.cache_hits.load(Ordering::Relaxed);
        let misses = self.counters.cache_misses.load(Ordering::Relaxed);
        let total = hits + misses;

        if total > 0 {
            (hits as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    }
}

impl Default for MetricsReporterImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsReporter for MetricsReporterImpl {
    fn record_resolution(&self, type_id: TypeId, duration: Duration, from_cache: bool) {
        // Обновляем атомарные счетчики
        self.counters
            .total_resolutions
            .fetch_add(1, Ordering::Relaxed);

        if from_cache {
            self.counters.cache_hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.counters.cache_misses.fetch_add(1, Ordering::Relaxed);
        }

        // Обновляем timing статистику
        {
            let mut timing_stats = self.timing_stats.write();
            timing_stats.record_duration(duration);
        }

        // Обновляем метрики для конкретного типа
        {
            let mut type_metrics = self.type_metrics.write();
            let metrics = type_metrics
                .entry(type_id)
                .or_insert_with(TypeMetricsImpl::new);
            metrics.record_resolution(duration, from_cache);
        }

        debug!(
            "Recorded resolution for type {:?}: {:?} (from_cache: {})",
            type_id, duration, from_cache
        );
    }

    fn record_registration(&self, type_id: TypeId) {
        self.counters
            .total_registrations
            .fetch_add(1, Ordering::Relaxed);
        debug!("Recorded registration for type {:?}", type_id);
    }

    fn get_stats(&self) -> DIContainerStats {
        let type_metrics = self.type_metrics.read();

        DIContainerStats {
            registered_factories: self.counters.total_registrations.load(Ordering::Relaxed)
                as usize,
            cached_singletons: type_metrics.len(), // Приблизительная оценка
            total_resolutions: self.counters.total_resolutions.load(Ordering::Relaxed),
            cache_hits: self.counters.cache_hits.load(Ordering::Relaxed),
            validation_errors: 0, // Эта метрика управляется отдельно
        }
    }

    fn get_performance_metrics(&self) -> DIPerformanceMetrics {
        let type_metrics_guard = self.type_metrics.read();
        let timing_stats = self.timing_stats.read();

        let type_metrics: HashMap<TypeId, TypeMetrics> = type_metrics_guard
            .iter()
            .map(|(&type_id, metrics)| (type_id, metrics.to_public()))
            .collect();

        DIPerformanceMetrics {
            total_resolutions: self.counters.total_resolutions.load(Ordering::Relaxed),
            total_resolution_time: timing_stats.total_resolution_time,
            cache_hits: self.counters.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.counters.cache_misses.load(Ordering::Relaxed),
            error_count: 0, // TODO: реализовать отслеживание ошибок
            type_metrics,
            dependency_depth: 0, // Эта метрика требует дополнительного отслеживания
        }
    }

    fn clear_metrics(&self) {
        self.counters.reset();

        {
            let mut type_metrics = self.type_metrics.write();
            type_metrics.clear();
        }

        {
            let mut timing_stats = self.timing_stats.write();
            timing_stats.reset();
        }

        debug!("All metrics cleared");
    }
}

/// Расширенные timing статистики
#[derive(Debug, Clone)]
pub struct TimingStatsReport {
    pub total_time: Duration,
    pub min_time: Option<Duration>,
    pub max_time: Option<Duration>,
    pub p50: Option<Duration>,
    pub p95: Option<Duration>,
    pub p99: Option<Duration>,
    pub sample_count: usize,
}

/// Композитный MetricsReporter для отправки метрик в несколько мест
/// Применяет принцип Open/Closed (OCP)
pub struct CompositeMetricsReporter {
    reporters: Vec<Arc<dyn MetricsReporter>>,
}

use std::sync::Arc;

impl CompositeMetricsReporter {
    pub fn new() -> Self {
        Self {
            reporters: Vec::new(),
        }
    }

    /// Добавить reporter (демонстрация OCP)
    pub fn add_reporter(&mut self, reporter: Arc<dyn MetricsReporter>) {
        self.reporters.push(reporter);
    }
}

impl MetricsReporter for CompositeMetricsReporter {
    fn record_resolution(&self, type_id: TypeId, duration: Duration, from_cache: bool) {
        for reporter in &self.reporters {
            reporter.record_resolution(type_id, duration, from_cache);
        }
    }

    fn record_registration(&self, type_id: TypeId) {
        for reporter in &self.reporters {
            reporter.record_registration(type_id);
        }
    }

    fn get_stats(&self) -> DIContainerStats {
        // Возвращаем статистику от первого reporter'а
        self.reporters
            .first()
            .map(|r| r.get_stats())
            .unwrap_or_else(|| DIContainerStats {
                registered_factories: 0,
                cached_singletons: 0,
                total_resolutions: 0,
                cache_hits: 0,
                validation_errors: 0,
            })
    }

    fn get_performance_metrics(&self) -> DIPerformanceMetrics {
        // Возвращаем метрики от первого reporter'а
        self.reporters
            .first()
            .map(|r| r.get_performance_metrics())
            .unwrap_or_default()
    }

    fn clear_metrics(&self) {
        for reporter in &self.reporters {
            reporter.clear_metrics();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_basic_metrics() {
        let reporter = MetricsReporterImpl::new();
        let type_id = TypeId::of::<String>();

        // Записываем несколько разрешений
        reporter.record_resolution(type_id, Duration::from_millis(10), false);
        reporter.record_resolution(type_id, Duration::from_millis(5), true);
        reporter.record_registration(type_id);

        let stats = reporter.get_stats();
        assert_eq!(stats.total_resolutions, 2);
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(stats.registered_factories, 1);
    }

    #[test]
    fn test_performance_metrics() {
        let reporter = MetricsReporterImpl::new();
        let type_id = TypeId::of::<i32>();

        // Записываем несколько разрешений с разным временем
        reporter.record_resolution(type_id, Duration::from_millis(100), false);
        reporter.record_resolution(type_id, Duration::from_millis(50), true);
        reporter.record_resolution(type_id, Duration::from_millis(75), false);

        let metrics = reporter.get_performance_metrics();
        assert_eq!(metrics.total_resolutions, 3);
        assert_eq!(metrics.cache_hits, 1);
        assert_eq!(metrics.cache_misses, 2);

        // Проверяем, что есть метрики для нашего типа
        assert!(metrics.type_metrics.contains_key(&type_id));

        let type_metrics = &metrics.type_metrics[&type_id];
        assert_eq!(type_metrics.resolutions, 3);
        assert_eq!(type_metrics.cache_hits, 1);
    }

    #[test]
    fn test_cache_hit_rate() {
        let reporter = MetricsReporterImpl::new();
        let type_id = TypeId::of::<f64>();

        // 3 cache hits из 5 разрешений = 60%
        reporter.record_resolution(type_id, Duration::from_millis(10), true);
        reporter.record_resolution(type_id, Duration::from_millis(10), true);
        reporter.record_resolution(type_id, Duration::from_millis(10), true);
        reporter.record_resolution(type_id, Duration::from_millis(10), false);
        reporter.record_resolution(type_id, Duration::from_millis(10), false);

        let hit_rate = reporter.get_cache_hit_rate();
        assert!((hit_rate - 60.0).abs() < 0.01); // Проверяем с небольшой погрешностью
    }

    #[test]
    fn test_timing_stats() {
        let reporter = MetricsReporterImpl::new();
        let type_id = TypeId::of::<Vec<String>>();

        // Добавляем разрешения с разным временем
        let durations = [
            Duration::from_millis(10),
            Duration::from_millis(20),
            Duration::from_millis(30),
            Duration::from_millis(40),
            Duration::from_millis(50),
        ];

        for duration in &durations {
            reporter.record_resolution(type_id, *duration, false);
        }

        let timing_stats = reporter.get_timing_stats();
        assert_eq!(timing_stats.sample_count, 5);
        assert_eq!(timing_stats.min_time, Some(Duration::from_millis(10)));
        assert_eq!(timing_stats.max_time, Some(Duration::from_millis(50)));
        assert!(timing_stats.p50.is_some());
        assert!(timing_stats.p95.is_some());
    }

    #[test]
    fn test_clear_metrics() {
        let reporter = MetricsReporterImpl::new();
        let type_id = TypeId::of::<HashMap<String, i32>>();

        // Добавляем некоторые метрики
        reporter.record_resolution(type_id, Duration::from_millis(10), true);
        reporter.record_registration(type_id);

        let stats_before = reporter.get_stats();
        assert!(stats_before.total_resolutions > 0);

        // Очищаем метрики
        reporter.clear_metrics();

        let stats_after = reporter.get_stats();
        assert_eq!(stats_after.total_resolutions, 0);
        assert_eq!(stats_after.cache_hits, 0);
        assert_eq!(stats_after.registered_factories, 0);
    }

    #[test]
    fn test_composite_reporter() {
        let mut composite = CompositeMetricsReporter::new();
        let reporter1 = Arc::new(MetricsReporterImpl::new());
        let reporter2 = Arc::new(MetricsReporterImpl::new());

        composite.add_reporter(reporter1.clone());
        composite.add_reporter(reporter2.clone());

        let type_id = TypeId::of::<bool>();
        composite.record_resolution(type_id, Duration::from_millis(5), true);

        // Проверяем, что оба reporter'а получили данные
        assert_eq!(reporter1.get_stats().total_resolutions, 1);
        assert_eq!(reporter2.get_stats().total_resolutions, 1);
    }
}
