//! HealthChecker - централизованный компонент проверки здоровья системы
//!
//! Реализует Single Responsibility Principle для мониторинга здоровья,
//! диагностики проблем и предоставления статуса готовности координаторов.

use anyhow::Result;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::orchestration::traits as _traits_mod;
use common::{service_macros::CoordinatorMacroHelpers, service_traits::*, MagrayCoreError};

/// Уровни здоровья компонентов
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HealthLevel {
    Critical = 0,  // Критическая ошибка, требует немедленного внимания
    Warning = 1,   // Предупреждение, возможна деградация
    Healthy = 2,   // Нормальное состояние
    Excellent = 3, // Оптимальное состояние
}

impl std::fmt::Display for HealthLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthLevel::Critical => write!(f, "critical"),
            HealthLevel::Warning => write!(f, "warning"),
            HealthLevel::Healthy => write!(f, "healthy"),
            HealthLevel::Excellent => write!(f, "excellent"),
        }
    }
}

/// Детальная информация о состоянии здоровья
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub level: HealthLevel,
    pub score: f64, // 0.0 - 100.0
    pub message: String,
    pub details: HashMap<String, Value>,
    pub last_check: Instant,
    pub check_duration_ms: u64,
}

impl HealthStatus {
    /// Создать новый статус здоровья
    pub fn new(level: HealthLevel, score: f64, message: String) -> Self {
        Self {
            level,
            score,
            message,
            details: HashMap::new(),
            last_check: Instant::now(),
            check_duration_ms: 0,
        }
    }

    /// Добавить дополнительную информацию
    pub fn with_detail(mut self, key: String, value: Value) -> Self {
        self.details.insert(key, value);
        self
    }

    /// Установить длительность проверки
    pub fn with_check_duration(mut self, duration_ms: u64) -> Self {
        self.check_duration_ms = duration_ms;
        self
    }
}

/// Результат системной диагностики
#[derive(Debug, Clone)]
pub struct SystemDiagnostics {
    pub overall_health: HealthLevel,
    pub overall_score: f64,
    pub coordinator_statuses: HashMap<String, HealthStatus>,
    pub system_issues: Vec<String>,
    pub recommendations: Vec<String>,
    pub check_timestamp: Instant,
    pub total_check_duration_ms: u64,
}

impl SystemDiagnostics {
    /// Получить количество координаторов по уровням здоровья
    pub fn get_health_distribution(&self) -> HashMap<HealthLevel, usize> {
        let mut distribution = HashMap::new();

        for status in self.coordinator_statuses.values() {
            *distribution.entry(status.level.clone()).or_insert(0) += 1;
        }

        distribution
    }

    /// Получить критические проблемы
    pub fn get_critical_issues(&self) -> Vec<(&String, &HealthStatus)> {
        self.coordinator_statuses
            .iter()
            .filter(|(_, status)| status.level == HealthLevel::Critical)
            .collect()
    }

    /// Проверить готовность системы к работе
    pub fn is_system_ready(&self) -> bool {
        // Система готова если нет критических проблем и общий score > 70
        !self
            .coordinator_statuses
            .values()
            .any(|status| status.level == HealthLevel::Critical)
            && self.overall_score >= 70.0
    }
}

/// Параметры проверки здоровья
#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    /// Интервал между проверками
    pub check_interval: Duration,
    /// Таймаут для каждой проверки
    pub check_timeout: Duration,
    /// Минимальный score для healthy статуса
    pub healthy_threshold: f64,
    /// Минимальный score для warning статуса
    pub warning_threshold: f64,
    /// Включить глубокую диагностику
    pub deep_diagnostics: bool,
    /// Максимальное количество повторных попыток
    pub max_retries: u32,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(30),
            check_timeout: Duration::from_secs(5),
            healthy_threshold: 80.0,
            warning_threshold: 60.0,
            deep_diagnostics: false,
            max_retries: 3,
        }
    }
}

/// Централизованный проверяльщик здоровья
#[derive(Clone, Debug)]
pub struct HealthChecker {
    /// Конфигурация проверок
    config: HealthCheckConfig,

    /// Последние результаты диагностики
    last_diagnostics: Arc<RwLock<Option<SystemDiagnostics>>>,

    /// История результатов проверок
    diagnostics_history: Arc<RwLock<Vec<SystemDiagnostics>>>,

    /// Максимальный размер истории
    max_history_size: usize,

    /// Флаг активности checker'а
    active: Arc<AtomicBool>,

    /// Счетчики для статистики
    total_checks: Arc<AtomicU64>,
    failed_checks: Arc<AtomicU64>,

    /// Время запуска для uptime расчетов
    start_time: Instant,

    /// Кэш результатов для оптимизации
    results_cache: Arc<RwLock<HashMap<String, (HealthStatus, Instant)>>>,

    /// TTL для кэша результатов
    cache_ttl: Duration,
}

impl CoordinatorMacroHelpers for HealthChecker {
    async fn perform_coordinator_init(&self) -> anyhow::Result<()> {
        info!("🚀 Инициализация HealthChecker");
        self.active.store(true, Ordering::Relaxed);
        Ok(())
    }

    async fn check_readiness(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    async fn perform_health_check(&self) -> anyhow::Result<()> {
        if !self.is_active() {
            return Err(anyhow::anyhow!("HealthChecker не активен"));
        }
        Ok(())
    }

    async fn perform_coordinator_shutdown(&self) -> anyhow::Result<()> {
        info!("🛑 Остановка HealthChecker");
        self.active.store(false, Ordering::Relaxed);
        self.clear_cache().await;
        info!("✅ HealthChecker остановлен");
        Ok(())
    }

    async fn collect_coordinator_metrics(&self) -> serde_json::Value {
        self.get_health_checker_stats().await
    }
}

impl HealthChecker {
    /// Создать новый HealthChecker
    pub fn new(config: HealthCheckConfig, max_history_size: usize) -> Self {
        Self {
            config,
            last_diagnostics: Arc::new(RwLock::new(None)),
            diagnostics_history: Arc::new(RwLock::new(Vec::with_capacity(max_history_size))),
            max_history_size,
            active: Arc::new(AtomicBool::new(true)),
            total_checks: Arc::new(AtomicU64::new(0)),
            failed_checks: Arc::new(AtomicU64::new(0)),
            start_time: Instant::now(),
            results_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: Duration::from_secs(10), // 10 секунд TTL для кэша
        }
    }

    /// Выполнить полную системную диагностику
    pub async fn perform_system_diagnostics<T>(
        &self,
        coordinators: &HashMap<String, Arc<T>>,
    ) -> SystemDiagnostics
    where
        T: _traits_mod::Coordinator + ?Sized + Send + Sync + 'static,
    {
        let check_start = Instant::now();
        self.total_checks.fetch_add(1, Ordering::Relaxed);

        debug!(
            "🔍 Начало системной диагностики ({} coordinators)",
            coordinators.len()
        );

        let mut coordinator_statuses = HashMap::new();
        let mut system_issues = Vec::new();
        let mut recommendations = Vec::new();

        // Параллельная проверка всех координаторов
        let mut check_tasks = Vec::new();

        for (name, coordinator) in coordinators {
            let coordinator_clone = Arc::clone(coordinator);
            let name_clone = name.clone();
            let config = self.config.clone();

            let task = tokio::spawn(async move {
                let result =
                    Self::check_coordinator_health(&name_clone, &coordinator_clone, &config).await;
                (name_clone, result)
            });

            check_tasks.push(task);
        }

        // Собираем результаты
        for task in check_tasks {
            match task.await {
                Ok((name, health_result)) => {
                    match health_result {
                        Ok(status) => {
                            // Добавляем проблемы в системный список
                            if status.level == HealthLevel::Critical {
                                system_issues.push(format!(
                                    "Критическая проблема с {}: {}",
                                    name, status.message
                                ));
                            } else if status.level == HealthLevel::Warning {
                                system_issues.push(format!(
                                    "Предупреждение для {}: {}",
                                    name, status.message
                                ));
                            }

                            coordinator_statuses.insert(name, status);
                        }
                        Err(e) => {
                            error!("❌ Ошибка проверки здоровья для {}: {}", name, e);
                            self.failed_checks.fetch_add(1, Ordering::Relaxed);

                            let critical_status = HealthStatus::new(
                                HealthLevel::Critical,
                                0.0,
                                format!("Health check failed: {}", e),
                            );

                            coordinator_statuses.insert(name.clone(), critical_status);
                            system_issues.push(format!("Координатор {} недоступен: {}", name, e));
                        }
                    }
                }
                Err(e) => {
                    error!("❌ Таск проверки здоровья завершился с ошибкой: {}", e);
                    self.failed_checks.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        // Рассчитываем общий health score и уровень
        let (overall_score, overall_health) = self.calculate_overall_health(&coordinator_statuses);

        // Генерируем рекомендации на основе диагностики
        self.generate_recommendations(&coordinator_statuses, &mut recommendations);

        let total_duration = check_start.elapsed().as_millis() as u64;

        let diagnostics = SystemDiagnostics {
            overall_health: overall_health.clone(),
            overall_score,
            coordinator_statuses,
            system_issues,
            recommendations,
            check_timestamp: check_start,
            total_check_duration_ms: total_duration,
        };

        // Сохраняем результаты
        self.save_diagnostics_results(&diagnostics).await;

        info!(
            "✅ Системная диагностика завершена за {}ms (overall: {}, score: {:.1})",
            total_duration, overall_health, overall_score
        );

        diagnostics
    }

    /// Проверить здоровье конкретного координатора
    pub async fn check_coordinator_health<T>(
        name: &str,
        coordinator: &Arc<T>,
        config: &HealthCheckConfig,
    ) -> Result<HealthStatus>
    where
        T: _traits_mod::Coordinator + ?Sized,
    {
        let check_start = Instant::now();

        debug!("🔍 Проверка здоровья координатора: {}", name);

        // Выполняем health check с таймаутом и retry логикой
        let health_result = tokio::time::timeout(
            config.check_timeout,
            Self::perform_health_check_with_retry(coordinator, config.max_retries),
        )
        .await;

        let check_duration = check_start.elapsed().as_millis() as u64;

        match health_result {
            Ok(Ok(())) => {
                // Дополнительные проверки
                let readiness = coordinator.is_ready().await;
                let metrics = coordinator.metrics().await;

                let (score, level) = Self::calculate_health_score(readiness, &metrics, config);

                let status =
                    HealthStatus::new(level, score, format!("Coordinator {} operational", name))
                        .with_detail("ready".to_string(), Value::Bool(readiness))
                        .with_detail("metrics".to_string(), metrics)
                        .with_check_duration(check_duration);

                debug!(
                    "✅ {} health check completed: {} (score: {:.1})",
                    name, status.level, score
                );
                Ok(status)
            }
            Ok(Err(e)) => {
                warn!("⚠️ {} health check failed: {}", name, e);

                let status = HealthStatus::new(
                    HealthLevel::Critical,
                    0.0,
                    format!("Health check failed: {}", e),
                )
                .with_check_duration(check_duration);

                Ok(status)
            }
            Err(_) => {
                warn!(
                    "⏱️ {} health check timed out after {:?}",
                    name, config.check_timeout
                );

                let status = HealthStatus::new(
                    HealthLevel::Critical,
                    0.0,
                    format!("Health check timed out after {:?}", config.check_timeout),
                )
                .with_check_duration(check_duration);

                Ok(status)
            }
        }
    }

    /// Получить последние результаты диагностики
    pub async fn get_last_diagnostics(&self) -> Option<SystemDiagnostics> {
        self.last_diagnostics.read().await.clone()
    }

    /// Получить историю диагностики
    pub async fn get_diagnostics_history(&self) -> Vec<SystemDiagnostics> {
        self.diagnostics_history.read().await.clone()
    }

    /// Получить статистику работы HealthChecker
    pub async fn get_health_checker_stats(&self) -> Value {
        let total_checks = self.total_checks.load(Ordering::Relaxed);
        let failed_checks = self.failed_checks.load(Ordering::Relaxed);
        let success_rate = if total_checks > 0 {
            ((total_checks - failed_checks) as f64 / total_checks as f64) * 100.0
        } else {
            100.0
        };

        let last_diagnostics = self.get_last_diagnostics().await;
        let uptime_seconds = self.start_time.elapsed().as_secs();

        json!({
            "uptime_seconds": uptime_seconds,
            "total_checks": total_checks,
            "failed_checks": failed_checks,
            "success_rate": success_rate,
            "active": self.active.load(Ordering::Relaxed),
            "config": {
                "check_interval_secs": self.config.check_interval.as_secs(),
                "check_timeout_secs": self.config.check_timeout.as_secs(),
                "healthy_threshold": self.config.healthy_threshold,
                "warning_threshold": self.config.warning_threshold,
                "deep_diagnostics": self.config.deep_diagnostics,
                "max_retries": self.config.max_retries,
            },
            "last_check": last_diagnostics.as_ref().map(|d| {
                json!({
                    "timestamp": d.check_timestamp.elapsed().as_secs(),
                    "overall_health": d.overall_health.to_string(),
                    "overall_score": d.overall_score,
                    "duration_ms": d.total_check_duration_ms,
                    "coordinators_count": d.coordinator_statuses.len(),
                    "system_issues_count": d.system_issues.len(),
                })
            }),
            "history_size": self.diagnostics_history.read().await.len(),
            "max_history_size": self.max_history_size,
        })
    }

    /// Получить упрощенный статус для быстрого доступа
    pub async fn get_quick_health_status<T>(&self, coordinators: &HashMap<String, Arc<T>>) -> Value
    where
        T: _traits_mod::Coordinator + ?Sized + Send + Sync + 'static,
    {
        // Проверяем кэш
        if let Some(cached) = self.get_cached_quick_status().await {
            return cached;
        }

        debug!("🔍 Быстрая проверка статуса системы");

        let mut ready_count = 0;
        let total_count = coordinators.len();

        // Параллельная проверка readiness
        let readiness_futures: Vec<_> = coordinators
            .iter()
            .map(|(name, coordinator)| {
                let name = name.clone();
                let coordinator = Arc::clone(coordinator);
                tokio::spawn(async move { (name, coordinator.is_ready().await) })
            })
            .collect();

        let mut coordinator_readiness = HashMap::new();

        for future in readiness_futures {
            if let Ok((name, ready)) = future.await {
                coordinator_readiness.insert(name, ready);
                if ready {
                    ready_count += 1;
                }
            }
        }

        let readiness_percentage = if total_count > 0 {
            (ready_count as f64 / total_count as f64) * 100.0
        } else {
            100.0
        };

        let status = if readiness_percentage >= 100.0 {
            "healthy"
        } else if readiness_percentage >= 75.0 {
            "warning"
        } else {
            "critical"
        };

        let result = json!({
            "status": status,
            "readiness_percentage": readiness_percentage,
            "ready_coordinators": ready_count,
            "total_coordinators": total_count,
            "coordinator_readiness": coordinator_readiness,
            "timestamp": Instant::now().elapsed().as_secs(),
        });

        // Кэшируем результат
        self.cache_quick_status(result.clone()).await;

        result
    }

    /// Остановить HealthChecker (legacy method, используйте perform_coordinator_shutdown)
    #[deprecated(note = "Используйте coordinator shutdown из trait")]
    pub async fn shutdown(&self) {
        let _ = self.perform_coordinator_shutdown().await;
    }

    /// Проверить активность checker'а
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    /// Обновить конфигурацию
    pub async fn update_config(&mut self, new_config: HealthCheckConfig) {
        info!("🔧 Обновление конфигурации HealthChecker");
        self.config = new_config;
        // Очищаем кэш при изменении конфигурации
        self.clear_cache().await;
    }

    // === Приватные методы ===

    /// Выполнить health check с повторными попытками
    async fn perform_health_check_with_retry<T>(
        coordinator: &Arc<T>,
        max_retries: u32,
    ) -> Result<()>
    where
        T: _traits_mod::Coordinator + ?Sized,
    {
        let mut last_error = None;

        for attempt in 0..=max_retries {
            match coordinator.health_check().await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < max_retries {
                        let delay = Duration::from_millis(100 * (attempt + 1) as u64);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            anyhow::anyhow!("Health check failed after {} retries", max_retries)
        }))
    }

    /// Рассчитать health score на основе метрик
    fn calculate_health_score(
        readiness: bool,
        metrics: &Value,
        config: &HealthCheckConfig,
    ) -> (f64, HealthLevel) {
        let mut score = if readiness { 100.0 } else { 0.0 };

        // Анализируем метрики для дополнительной оценки
        if let Some(health_score) = metrics.get("health_score").and_then(|v| v.as_f64()) {
            score = (score + health_score) / 2.0;
        }

        // Определяем уровень на основе score и пороговых значений
        let level = if score >= config.healthy_threshold {
            if score >= 95.0 {
                HealthLevel::Excellent
            } else {
                HealthLevel::Healthy
            }
        } else if score >= config.warning_threshold {
            HealthLevel::Warning
        } else {
            HealthLevel::Critical
        };

        (score, level)
    }

    /// Рассчитать общий health для системы
    fn calculate_overall_health(
        &self,
        statuses: &HashMap<String, HealthStatus>,
    ) -> (f64, HealthLevel) {
        if statuses.is_empty() {
            return (0.0, HealthLevel::Critical);
        }

        // Рассчитываем средний score с весами по критичности
        let mut weighted_sum = 0.0;
        let mut total_weight = 0.0;
        let mut has_critical = false;
        let mut warning_count = 0;

        for status in statuses.values() {
            let weight = match status.level {
                HealthLevel::Critical => {
                    has_critical = true;
                    2.0 // Критические проблемы имеют больший вес
                }
                HealthLevel::Warning => {
                    warning_count += 1;
                    1.5
                }
                HealthLevel::Healthy => 1.0,
                HealthLevel::Excellent => 1.0,
            };

            weighted_sum += status.score * weight;
            total_weight += weight;
        }

        let overall_score = if total_weight > 0.0 {
            weighted_sum / total_weight
        } else {
            0.0
        };

        // Определяем общий уровень
        let overall_level = if has_critical {
            HealthLevel::Critical
        } else if warning_count > statuses.len() / 2 {
            HealthLevel::Warning
        } else if overall_score >= 95.0 {
            HealthLevel::Excellent
        } else if overall_score >= self.config.healthy_threshold {
            HealthLevel::Healthy
        } else {
            HealthLevel::Warning
        };

        (overall_score, overall_level)
    }

    /// Сгенерировать рекомендации на основе диагностики
    fn generate_recommendations(
        &self,
        statuses: &HashMap<String, HealthStatus>,
        recommendations: &mut Vec<String>,
    ) {
        let critical_count = statuses
            .values()
            .filter(|s| s.level == HealthLevel::Critical)
            .count();

        let warning_count = statuses
            .values()
            .filter(|s| s.level == HealthLevel::Warning)
            .count();

        if critical_count > 0 {
            recommendations.push(format!(
                "СРОЧНО: {} координаторов в критическом состоянии требуют немедленного внимания",
                critical_count
            ));
        }

        if warning_count > 0 {
            recommendations.push(format!(
                "ВНИМАНИЕ: {} координаторов показывают предупреждения, рекомендуется проверка",
                warning_count
            ));
        }

        if critical_count == 0 && warning_count == 0 {
            recommendations.push("Система работает в нормальном режиме".to_string());
        }

        // Специфичные рекомендации на основе анализа метрик
        for (name, status) in statuses {
            if status.level == HealthLevel::Critical {
                recommendations.push(format!(
                    "Проверить логи и перезапустить координатор {}",
                    name
                ));
            } else if status.score < 70.0 {
                recommendations.push(format!(
                    "Мониторить производительность координатора {}",
                    name
                ));
            }
        }
    }

    /// Сохранить результаты диагностики
    async fn save_diagnostics_results(&self, diagnostics: &SystemDiagnostics) {
        // Обновляем последние результаты
        let mut last_diagnostics = self.last_diagnostics.write().await;
        *last_diagnostics = Some(diagnostics.clone());

        // Добавляем в историю
        let mut history = self.diagnostics_history.write().await;

        // Ring buffer логика
        if history.len() >= self.max_history_size {
            history.remove(0);
        }

        history.push(diagnostics.clone());

        debug!(
            "💾 Результаты диагностики сохранены (история: {} записей)",
            history.len()
        );
    }

    /// Получить кэшированный быстрый статус
    async fn get_cached_quick_status(&self) -> Option<Value> {
        let cache = self.results_cache.read().await;
        if let Some((cached_status, cached_time)) = cache.get("quick_status") {
            if cached_time.elapsed() < self.cache_ttl {
                if let Ok(value) = serde_json::from_str(&cached_status.message) {
                    return Some(value);
                }
            }
        }
        None
    }

    /// Кэшировать быстрый статус
    async fn cache_quick_status(&self, status: Value) {
        let mut cache = self.results_cache.write().await;
        let cached_status = HealthStatus::new(
            HealthLevel::Healthy,
            100.0,
            serde_json::to_string(&status).unwrap_or_default(),
        );
        cache.insert("quick_status".to_string(), (cached_status, Instant::now()));
    }

    /// Очистить кэш
    async fn clear_cache(&self) {
        let mut cache = self.results_cache.write().await;
        cache.clear();
    }
}

#[async_trait::async_trait]
impl _traits_mod::Coordinator for HealthChecker {
    async fn initialize(&self) -> anyhow::Result<()> {
        self.perform_coordinator_init().await
    }

    async fn is_ready(&self) -> bool {
        self.check_readiness().await
    }

    async fn health_check(&self) -> anyhow::Result<()> {
        if !self.is_ready().await {
            return Err(anyhow::anyhow!("HealthChecker не готов"));
        }
        self.perform_health_check().await
    }

    async fn shutdown(&self) -> anyhow::Result<()> {
        self.perform_coordinator_shutdown().await
    }

    async fn metrics(&self) -> serde_json::Value {
        self.collect_coordinator_metrics().await
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new(HealthCheckConfig::default(), 100)
    }
}

#[cfg(all(test, feature = "legacy-orchestrator"))]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[derive(Debug)]
    struct MockCoordinator {
        ready: Arc<AtomicBool>,
        should_fail_health_check: Arc<AtomicBool>,
    }

    impl MockCoordinator {
        fn new(ready: bool) -> Self {
            Self {
                ready: Arc::new(AtomicBool::new(ready)),
                should_fail_health_check: Arc::new(AtomicBool::new(false)),
            }
        }

        fn set_health_check_fail(&self, should_fail: bool) {
            self.should_fail_health_check
                .store(should_fail, Ordering::Relaxed);
        }
    }

    #[async_trait]
    impl _traits_mod::Coordinator for MockCoordinator {
        async fn initialize(&self) -> Result<()> {
            Ok(())
        }

        async fn is_ready(&self) -> bool {
            self.ready.load(Ordering::Relaxed)
        }

        async fn health_check(&self) -> Result<()> {
            if self.should_fail_health_check.load(Ordering::Relaxed) {
                Err(anyhow::anyhow!("Mock health check failure"))
            } else {
                Ok(())
            }
        }

        async fn shutdown(&self) -> Result<()> {
            Ok(())
        }

        async fn metrics(&self) -> Value {
            json!({
                "health_score": if self.ready.load(Ordering::Relaxed) { 95.0 } else { 30.0 },
                "mock": true,
            })
        }
    }

    #[tokio::test]
    async fn test_health_checker_creation() {
        let checker = HealthChecker::default();
        assert!(checker.is_active());
        assert_eq!(checker.max_history_size, 100);
    }

    #[tokio::test]
    async fn test_coordinator_health_check() {
        let config = HealthCheckConfig::default();
        let coordinator = Arc::new(MockCoordinator::new(true));

        let result = HealthChecker::check_coordinator_health("test", &coordinator, &config).await;

        assert!(result.is_ok());
        let status = result.unwrap();
        assert_eq!(status.level, HealthLevel::Excellent);
        assert!(status.score >= 95.0);
    }

    #[tokio::test]
    async fn test_coordinator_health_check_failure() {
        let config = HealthCheckConfig::default();
        let coordinator = Arc::new(MockCoordinator::new(false));
        coordinator.set_health_check_fail(true);

        let result = HealthChecker::check_coordinator_health("test", &coordinator, &config).await;

        assert!(result.is_ok());
        let status = result.unwrap();
        assert_eq!(status.level, HealthLevel::Critical);
        assert_eq!(status.score, 0.0);
    }

    #[tokio::test]
    async fn test_system_diagnostics() {
        let checker = HealthChecker::default();
        let mut coordinators = HashMap::new();

        coordinators.insert("healthy".to_string(), Arc::new(MockCoordinator::new(true)));
        coordinators.insert("warning".to_string(), Arc::new(MockCoordinator::new(false)));

        let diagnostics = checker.perform_system_diagnostics(&coordinators).await;

        assert_eq!(diagnostics.coordinator_statuses.len(), 2);
        assert!(diagnostics.overall_score > 0.0);
        // С одним coordinator в warning состоянии, общий уровень должен быть warning
        assert!(matches!(
            diagnostics.overall_health,
            HealthLevel::Warning | HealthLevel::Healthy
        ));
    }

    #[tokio::test]
    async fn test_quick_health_status() {
        let checker = HealthChecker::default();
        let mut coordinators = HashMap::new();

        coordinators.insert("test1".to_string(), Arc::new(MockCoordinator::new(true)));
        coordinators.insert("test2".to_string(), Arc::new(MockCoordinator::new(true)));

        let status = checker.get_quick_health_status(&coordinators).await;

        assert_eq!(status["status"], "healthy");
        assert_eq!(status["readiness_percentage"], 100.0);
        assert_eq!(status["ready_coordinators"], 2);
        assert_eq!(status["total_coordinators"], 2);
    }

    #[tokio::test]
    async fn test_health_level_ordering() {
        assert!(HealthLevel::Excellent > HealthLevel::Healthy);
        assert!(HealthLevel::Healthy > HealthLevel::Warning);
        assert!(HealthLevel::Warning > HealthLevel::Critical);
    }

    #[tokio::test]
    async fn test_diagnostics_history() {
        let checker = HealthChecker::new(HealthCheckConfig::default(), 2); // Маленький размер
        let mut coordinators = HashMap::new();
        coordinators.insert("test".to_string(), Arc::new(MockCoordinator::new(true)));

        // Выполняем несколько диагностик
        checker.perform_system_diagnostics(&coordinators).await;
        checker.perform_system_diagnostics(&coordinators).await;
        checker.perform_system_diagnostics(&coordinators).await;

        let history = checker.get_diagnostics_history().await;

        // Ring buffer должен содержать только последние 2 записи
        assert_eq!(history.len(), 2);
    }

    #[tokio::test]
    async fn test_shutdown() {
        let checker = HealthChecker::default();
        assert!(checker.is_active());

        checker.shutdown().await;
        assert!(!checker.is_active());
    }
}
