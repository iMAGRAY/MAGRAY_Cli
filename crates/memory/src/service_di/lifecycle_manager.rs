//! Lifecycle Manager Module - Single Responsibility для управления жизненным циклом
//! 
//! Этот модуль отвечает ТОЛЬКО за управление жизненным циклом сервиса:
//! инициализация, shutdown, readiness checks.
//! Применяет Single Responsibility и State Machine pattern.

use anyhow::Result;
use std::{
    sync::Arc,
    time::{Duration, Instant},
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
};
use tracing::{info, debug, warn, error};
use tokio::{sync::RwLock, time::timeout};

use crate::{
    storage::VectorStore,
    types::Layer,
};

/// Состояние жизненного цикла сервиса
#[derive(Debug, Clone, PartialEq)]
pub enum LifecycleState {
    /// Не инициализирован
    Uninitialized,
    /// Инициализируется
    Initializing,
    /// Готов к работе
    Ready,
    /// Деградирован (работает с ограничениями)
    Degraded,
    /// Выключается
    ShuttingDown,
    /// Выключен
    Shutdown,
}

/// Конфигурация lifecycle manager
#[derive(Debug, Clone)]
pub struct LifecycleConfig {
    /// Timeout для инициализации
    pub initialization_timeout: Duration,
    /// Timeout для graceful shutdown
    pub shutdown_timeout: Duration,
    /// Интервал проверки readiness
    pub readiness_check_interval: Duration,
    /// Максимальное время ожидания завершения операций
    pub operation_drain_timeout: Duration,
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self {
            initialization_timeout: Duration::from_secs(120),
            shutdown_timeout: Duration::from_secs(30),
            readiness_check_interval: Duration::from_secs(10),
            operation_drain_timeout: Duration::from_secs(15),
        }
    }
}

impl LifecycleConfig {
    pub fn production() -> Self {
        Self {
            initialization_timeout: Duration::from_secs(120),
            shutdown_timeout: Duration::from_secs(30),
            readiness_check_interval: Duration::from_secs(10),
            operation_drain_timeout: Duration::from_secs(15),
        }
    }

    pub fn minimal() -> Self {
        Self {
            initialization_timeout: Duration::from_secs(30),
            shutdown_timeout: Duration::from_secs(5),
            readiness_check_interval: Duration::from_secs(5),
            operation_drain_timeout: Duration::from_secs(3),
        }
    }
}

/// Статистика операций
#[derive(Debug, Clone)]
pub struct OperationStats {
    pub active_operations: u32,
    pub total_operations: u64,
    pub failed_initializations: u32,
    pub graceful_shutdowns: u32,
    pub forced_shutdowns: u32,
}

impl Default for OperationStats {
    fn default() -> Self {
        Self {
            active_operations: 0,
            total_operations: 0,
            failed_initializations: 0,
            graceful_shutdowns: 0,
            forced_shutdowns: 0,
        }
    }
}

/// Lifecycle Manager для управления жизненным циклом сервиса
pub struct LifecycleManager {
    /// Конфигурация
    config: LifecycleConfig,
    /// Текущее состояние
    state: Arc<RwLock<LifecycleState>>,
    /// Готовность к работе
    ready: Arc<AtomicBool>,
    /// Флаг запроса shutdown
    shutdown_requested: Arc<AtomicBool>,
    /// Счетчик активных операций
    active_operations: Arc<AtomicU32>,
    /// Статистика
    stats: Arc<RwLock<OperationStats>>,
    /// Время входа в текущее состояние
    state_entered_at: Arc<RwLock<Instant>>,
}

impl LifecycleManager {
    pub fn new(config: LifecycleConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(LifecycleState::Uninitialized)),
            ready: Arc::new(AtomicBool::new(false)),
            shutdown_requested: Arc::new(AtomicBool::new(false)),
            active_operations: Arc::new(AtomicU32::new(0)),
            stats: Arc::new(RwLock::new(OperationStats::default())),
            state_entered_at: Arc::new(RwLock::new(Instant::now())),
        }
    }

    pub fn with_production_config() -> Self {
        Self::new(LifecycleConfig::production())
    }

    pub fn with_minimal_config() -> Self {
        Self::new(LifecycleConfig::minimal())
    }

    /// Инициализировать сервис
    pub async fn initialize<F, Fut>(&self, init_fn: F) -> Result<()>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        info!("🚀 Начинаем инициализацию сервиса...");
        
        {
            let mut state = self.state.write().await;
            if *state != LifecycleState::Uninitialized {
                return Err(anyhow::anyhow!("Сервис уже инициализируется или инициализирован"));
            }
            *state = LifecycleState::Initializing;
            *self.state_entered_at.write().await = Instant::now();
        }

        let start_time = Instant::now();

        // Выполняем инициализацию с timeout
        let init_result = timeout(
            self.config.initialization_timeout,
            init_fn()
        ).await;

        match init_result {
            Ok(Ok(_)) => {
                // Успешная инициализация
                {
                    let mut state = self.state.write().await;
                    *state = LifecycleState::Ready;
                    *self.state_entered_at.write().await = Instant::now();
                }
                
                self.ready.store(true, Ordering::Relaxed);
                
                let initialization_time = start_time.elapsed();
                info!("✅ Сервис инициализирован за {:?}", initialization_time);
                
                Ok(())
            }
            Ok(Err(e)) => {
                // Ошибка инициализации
                {
                    let mut state = self.state.write().await;
                    *state = LifecycleState::Uninitialized;
                    let mut stats = self.stats.write().await;
                    stats.failed_initializations += 1;
                }
                
                error!("❌ Ошибка инициализации: {}", e);
                Err(e)
            }
            Err(_) => {
                // Timeout инициализации
                {
                    let mut state = self.state.write().await;
                    *state = LifecycleState::Uninitialized;
                    let mut stats = self.stats.write().await;
                    stats.failed_initializations += 1;
                }
                
                let timeout_err = anyhow::anyhow!("Timeout инициализации ({:?})", self.config.initialization_timeout);
                error!("⏱️ {}", timeout_err);
                Err(timeout_err)
            }
        }
    }

    /// Инициализировать слои памяти
    pub async fn initialize_memory_layers(&self, store: Arc<VectorStore>) -> Result<()> {
        info!("🗃️ Инициализация базовых слоев памяти...");

        // Инициализируем все слои с timeout
        for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            let layer_result = timeout(
                Duration::from_secs(30),
                store.init_layer(layer)
            ).await;

            match layer_result {
                Ok(Ok(_)) => {
                    debug!("✓ Слой {:?} инициализирован", layer);
                }
                Ok(Err(e)) => {
                    return Err(anyhow::anyhow!("Ошибка инициализации слоя {:?}: {}", layer, e));
                }
                Err(_) => {
                    return Err(anyhow::anyhow!("Timeout инициализации слоя {:?}", layer));
                }
            }
        }

        info!("✅ Все слои памяти инициализированы");
        Ok(())
    }

    /// Graceful shutdown сервиса
    pub async fn shutdown<F, Fut>(&self, shutdown_fn: F) -> Result<()>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        info!("🛑 Начинаем graceful shutdown...");
        
        {
            let mut state = self.state.write().await;
            if *state == LifecycleState::ShuttingDown || *state == LifecycleState::Shutdown {
                return Ok(()); // Уже в процессе shutdown
            }
            *state = LifecycleState::ShuttingDown;
            *self.state_entered_at.write().await = Instant::now();
        }

        // Помечаем что shutdown запрошен
        self.shutdown_requested.store(true, Ordering::Relaxed);
        self.ready.store(false, Ordering::Relaxed);

        // Ждем завершения активных операций
        let drain_result = self.drain_active_operations().await;
        if let Err(e) = drain_result {
            warn!("⚠️ Не удалось корректно завершить все операции: {}", e);
        }

        // Выполняем shutdown логику
        let shutdown_result = timeout(
            self.config.shutdown_timeout,
            shutdown_fn()
        ).await;

        let success = match shutdown_result {
            Ok(Ok(_)) => {
                info!("✅ Graceful shutdown завершен");
                true
            }
            Ok(Err(e)) => {
                error!("❌ Ошибка во время shutdown: {}", e);
                false
            }
            Err(_) => {
                error!("⏱️ Timeout shutdown - принудительное завершение");
                false
            }
        };

        // Обновляем состояние и статистику
        {
            let mut state = self.state.write().await;
            *state = LifecycleState::Shutdown;
            
            let mut stats = self.stats.write().await;
            if success {
                stats.graceful_shutdowns += 1;
            } else {
                stats.forced_shutdowns += 1;
            }
        }

        if success {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Shutdown не удался"))
        }
    }

    /// Проверить готовность сервиса
    pub async fn is_ready(&self) -> bool {
        let state = self.state.read().await;
        *state == LifecycleState::Ready && self.ready.load(Ordering::Relaxed)
    }

    /// Проверить что сервис не выключается
    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown_requested.load(Ordering::Relaxed)
    }

    /// Получить текущее состояние
    pub async fn get_state(&self) -> LifecycleState {
        let state = self.state.read().await;
        state.clone()
    }

    /// Увеличить счетчик активных операций
    pub fn increment_active_operations(&self) -> u32 {
        self.active_operations.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Уменьшить счетчик активных операций
    pub fn decrement_active_operations(&self) -> u32 {
        self.active_operations.fetch_sub(1, Ordering::Relaxed).saturating_sub(1)
    }

    /// Получить количество активных операций
    pub fn get_active_operations(&self) -> u32 {
        self.active_operations.load(Ordering::Relaxed)
    }

    /// Перевести в деградированное состояние
    pub async fn degrade(&self, reason: &str) {
        warn!("⚠️ Переход в деградированное состояние: {}", reason);
        
        let mut state = self.state.write().await;
        if *state == LifecycleState::Ready {
            *state = LifecycleState::Degraded;
            *self.state_entered_at.write().await = Instant::now();
        }
    }

    /// Восстановиться из деградированного состояния
    pub async fn recover(&self) -> Result<()> {
        info!("🔄 Попытка восстановления из деградированного состояния...");
        
        let mut state = self.state.write().await;
        if *state == LifecycleState::Degraded {
            // Здесь можно добавить дополнительные проверки
            *state = LifecycleState::Ready;
            *self.state_entered_at.write().await = Instant::now();
            self.ready.store(true, Ordering::Relaxed);
            
            info!("✅ Восстановление завершено");
        }
        
        Ok(())
    }

    /// Получить статистику
    pub async fn get_stats(&self) -> OperationStats {
        let mut stats = self.stats.read().await.clone();
        stats.active_operations = self.get_active_operations();
        stats
    }

    /// Получить время в текущем состоянии
    pub async fn time_in_current_state(&self) -> Duration {
        let state_entered_at = self.state_entered_at.read().await;
        state_entered_at.elapsed()
    }

    /// Получить human-readable описание состояния
    pub async fn get_state_description(&self) -> String {
        let state = self.get_state().await;
        let time_in_state = self.time_in_current_state().await;
        let active_ops = self.get_active_operations();
        
        match state {
            LifecycleState::Uninitialized => "🔴 НЕ ИНИЦИАЛИЗИРОВАН".to_string(),
            LifecycleState::Initializing => format!("🟡 ИНИЦИАЛИЗАЦИЯ ({:.1}s)", time_in_state.as_secs_f64()),
            LifecycleState::Ready => format!("🟢 ГОТОВ ({} активных операций)", active_ops),
            LifecycleState::Degraded => format!("🟠 ДЕГРАДИРОВАН ({:.1}s)", time_in_state.as_secs_f64()),
            LifecycleState::ShuttingDown => format!("🔶 ВЫКЛЮЧЕНИЕ ({} операций осталось)", active_ops),
            LifecycleState::Shutdown => "⚫ ВЫКЛЮЧЕН".to_string(),
        }
    }

    /// Принудительный reset (для тестов)
    pub async fn force_reset(&self) {
        let mut state = self.state.write().await;
        *state = LifecycleState::Uninitialized;
        
        self.ready.store(false, Ordering::Relaxed);
        self.shutdown_requested.store(false, Ordering::Relaxed);
        self.active_operations.store(0, Ordering::Relaxed);
        
        *self.state_entered_at.write().await = Instant::now();
        
        info!("♻️ Lifecycle manager сброшен");
    }

    // === Private methods ===

    /// Ждать завершения активных операций
    async fn drain_active_operations(&self) -> Result<()> {
        let start_time = Instant::now();
        
        while start_time.elapsed() < self.config.operation_drain_timeout {
            let active_ops = self.get_active_operations();
            
            if active_ops == 0 {
                debug!("✅ Все активные операции завершены");
                return Ok(());
            }
            
            debug!("⏳ Ожидание завершения {} активных операций...", active_ops);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        let remaining_ops = self.get_active_operations();
        if remaining_ops > 0 {
            warn!("⚠️ Timeout ожидания операций - {} операций будут принудительно прерваны", remaining_ops);
            return Err(anyhow::anyhow!("Не удалось дождаться завершения {} операций", remaining_ops));
        }
        
        Ok(())
    }
}

impl Default for LifecycleManager {
    fn default() -> Self {
        Self::new(LifecycleConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_lifecycle_initialization() {
        let manager = LifecycleManager::with_minimal_config();
        
        assert_eq!(manager.get_state().await, LifecycleState::Uninitialized);
        assert!(!manager.is_ready().await);
        
        // Успешная инициализация
        let result = manager.initialize(|| async { Ok(()) }).await;
        assert!(result.is_ok());
        assert_eq!(manager.get_state().await, LifecycleState::Ready);
        assert!(manager.is_ready().await);
    }

    #[tokio::test]
    async fn test_lifecycle_failed_initialization() {
        let manager = LifecycleManager::with_minimal_config();
        
        // Неудачная инициализация
        let result = manager.initialize(|| async { 
            Err(anyhow::anyhow!("Initialization failed"))
        }).await;
        
        assert!(result.is_err());
        assert_eq!(manager.get_state().await, LifecycleState::Uninitialized);
        assert!(!manager.is_ready().await);
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.failed_initializations, 1);
    }

    #[tokio::test] 
    async fn test_lifecycle_shutdown() {
        let manager = LifecycleManager::with_minimal_config();
        
        // Инициализируем
        manager.initialize(|| async { Ok(()) }).await.unwrap();
        assert_eq!(manager.get_state().await, LifecycleState::Ready);
        
        // Shutdown
        let result = manager.shutdown(|| async { Ok(()) }).await;
        assert!(result.is_ok());
        assert_eq!(manager.get_state().await, LifecycleState::Shutdown);
        assert!(manager.is_shutdown_requested());
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.graceful_shutdowns, 1);
    }

    #[tokio::test]
    async fn test_active_operations_tracking() {
        let manager = LifecycleManager::with_minimal_config();
        
        assert_eq!(manager.get_active_operations(), 0);
        
        let count1 = manager.increment_active_operations();
        assert_eq!(count1, 1);
        assert_eq!(manager.get_active_operations(), 1);
        
        let count2 = manager.increment_active_operations();
        assert_eq!(count2, 2);
        
        let count3 = manager.decrement_active_operations();
        assert_eq!(count3, 1);
        assert_eq!(manager.get_active_operations(), 1);
    }

    #[tokio::test]
    async fn test_degraded_state() {
        let manager = LifecycleManager::with_minimal_config();
        
        // Инициализируем
        manager.initialize(|| async { Ok(()) }).await.unwrap();
        assert_eq!(manager.get_state().await, LifecycleState::Ready);
        
        // Деградируем
        manager.degrade("Test degradation").await;
        assert_eq!(manager.get_state().await, LifecycleState::Degraded);
        
        // Восстанавливаем
        manager.recover().await.unwrap();
        assert_eq!(manager.get_state().await, LifecycleState::Ready);
        assert!(manager.is_ready().await);
    }

    #[tokio::test]
    async fn test_state_description() {
        let manager = LifecycleManager::with_minimal_config();
        
        let desc = manager.get_state_description().await;
        assert!(desc.contains("НЕ ИНИЦИАЛИЗИРОВАН"));
        
        manager.initialize(|| async { Ok(()) }).await.unwrap();
        let desc_ready = manager.get_state_description().await;
        assert!(desc_ready.contains("ГОТОВ"));
        
        manager.increment_active_operations();
        let desc_with_ops = manager.get_state_description().await;
        assert!(desc_with_ops.contains("1 активных"));
    }

    #[tokio::test]
    async fn test_force_reset() {
        let manager = LifecycleManager::with_minimal_config();
        
        // Инициализируем и добавляем операции
        manager.initialize(|| async { Ok(()) }).await.unwrap();
        manager.increment_active_operations();
        
        assert_eq!(manager.get_state().await, LifecycleState::Ready);
        assert_eq!(manager.get_active_operations(), 1);
        
        // Сбрасываем
        manager.force_reset().await;
        
        assert_eq!(manager.get_state().await, LifecycleState::Uninitialized);
        assert_eq!(manager.get_active_operations(), 0);
        assert!(!manager.is_ready().await);
        assert!(!manager.is_shutdown_requested());
    }
}