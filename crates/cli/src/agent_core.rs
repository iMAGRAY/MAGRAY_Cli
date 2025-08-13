//! AgentCore - центральное ядро агента с минимальными зависимостями
//!
//! Реализует Single Responsibility Principle - только координация компонентов
//! и базовый lifecycle management без бизнес-логики.

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use tracing::{debug, info};

/// Базовый трейт для всех компонентов агента
#[async_trait]
pub trait AgentComponent: Send + Sync {
    /// Имя компонента для идентификации
    fn component_name(&self) -> &'static str;

    /// Инициализация компонента
    async fn initialize(&mut self) -> Result<()>;

    /// Проверка готовности компонента
    async fn is_ready(&self) -> bool;

    /// Проверка здоровья компонента
    async fn health_check(&self) -> Result<()>;

    /// Graceful shutdown компонента
    async fn shutdown(&self) -> Result<()>;
}

/// Статистика компонента
#[derive(Clone)]
pub struct ComponentStats {
    pub name: String,
    pub ready: bool,
    pub healthy: bool,
    pub requests_processed: u64,
    pub average_response_time_ms: f64,
}

/// Центральное ядро агента - координирует компоненты без бизнес-логики
pub struct AgentCore {
    /// Компоненты агента
    components: HashMap<String, Box<dyn AgentComponent>>,
    /// Состояние инициализации
    initialized: bool,
    /// Статистика
    stats: HashMap<String, ComponentStats>,
}

impl AgentCore {
    /// Создать новое ядро агента
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
            initialized: false,
            stats: HashMap::new(),
        }
    }

    /// Зарегистрировать компонент
    pub fn register_component(&mut self, component: Box<dyn AgentComponent>) {
        let name = component.component_name().to_string();
        info!("Регистрация компонента: {}", name);

        // Инициализируем статистику
        self.stats.insert(
            name.clone(),
            ComponentStats {
                name: name.clone(),
                ready: false,
                healthy: false,
                requests_processed: 0,
                average_response_time_ms: 0.0,
            },
        );

        self.components.insert(name, component);
    }

    /// Получить компонент по имени
    pub fn get_component(&self, name: &str) -> Option<&dyn AgentComponent> {
        self.components.get(name).map(|c| c.as_ref())
    }

    /// Получить мутабельный доступ к компоненту
    pub fn get_component_mut(&mut self, name: &str) -> Option<&mut Box<dyn AgentComponent>> {
        self.components.get_mut(name)
    }

    /// Инициализация всех компонентов
    pub async fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }

        info!(
            "Инициализация AgentCore с {} компонентами",
            self.components.len()
        );

        // Инициализируем компоненты параллельно
        let mut init_results = Vec::new();

        for (name, component) in self.components.iter_mut() {
            debug!("Инициализация компонента: {}", name);
            match component.initialize().await {
                Ok(()) => {
                    info!("Компонент {} успешно инициализирован", name);
                    init_results.push((name.clone(), true));
                }
                Err(e) => {
                    tracing::error!("Ошибка инициализации компонента {}: {}", name, e);
                    init_results.push((name.clone(), false));
                }
            }
        }

        // Проверяем результаты
        let failed_components: Vec<_> = init_results
            .iter()
            .filter(|(_, success)| !success)
            .map(|(name, _)| name.clone())
            .collect();

        if !failed_components.is_empty() {
            return Err(anyhow::anyhow!(
                "Не удалось инициализировать компоненты: {:?}",
                failed_components
            ));
        }

        self.initialized = true;
        info!("AgentCore успешно инициализирован");
        Ok(())
    }

    /// Проверка готовности всех компонентов
    pub async fn is_ready(&self) -> bool {
        if !self.initialized {
            return false;
        }

        // Проверяем готовность всех компонентов параллельно
        let mut all_ready = true;
        for (name, component) in &self.components {
            if !component.is_ready().await {
                debug!("Компонент {} не готов", name);
                all_ready = false;
            }
        }

        all_ready
    }

    /// Комплексная проверка здоровья
    pub async fn health_check(&self) -> Result<HashMap<String, bool>> {
        let mut health_results = HashMap::new();

        for (name, component) in &self.components {
            match component.health_check().await {
                Ok(()) => {
                    health_results.insert(name.clone(), true);
                }
                Err(e) => {
                    debug!("Компонент {} нездоров: {}", name, e);
                    health_results.insert(name.clone(), false);
                }
            }
        }

        Ok(health_results)
    }

    /// Graceful shutdown всех компонентов
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Начинаем shutdown AgentCore");

        // Shutdown компонентов в обратном порядке регистрации
        let component_names: Vec<_> = self.components.keys().cloned().collect();

        for name in component_names.iter().rev() {
            if let Some(component) = self.components.get(name) {
                match component.shutdown().await {
                    Ok(()) => {
                        info!("Компонент {} успешно остановлен", name);
                    }
                    Err(e) => {
                        tracing::warn!("Ошибка при остановке компонента {}: {}", name, e);
                    }
                }
            }
        }

        self.initialized = false;
        info!("AgentCore shutdown завершен");
        Ok(())
    }

    /// Получить статистику всех компонентов
    pub async fn get_component_stats(&mut self) -> HashMap<String, ComponentStats> {
        // Обновляем статистику
        for (name, component) in &self.components {
            if let Some(stats) = self.stats.get_mut(name) {
                stats.ready = component.is_ready().await;
                stats.healthy = component.health_check().await.is_ok();
            }
        }

        self.stats.clone()
    }

    /// Обновить статистику обработки запроса для компонента
    pub fn update_request_stats(&mut self, component_name: &str, response_time_ms: f64) {
        if let Some(stats) = self.stats.get_mut(component_name) {
            stats.requests_processed += 1;

            // Скользящее среднее
            let alpha = 0.1; // коэффициент сглаживания
            stats.average_response_time_ms =
                alpha * response_time_ms + (1.0 - alpha) * stats.average_response_time_ms;
        }
    }

    /// Получить список имен компонентов
    pub fn component_names(&self) -> Vec<String> {
        self.components.keys().cloned().collect()
    }

    /// Количество зарегистрированных компонентов
    pub fn component_count(&self) -> usize {
        self.components.len()
    }
}

impl Default for AgentCore {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder для AgentCore
pub struct AgentCoreBuilder {
    core: AgentCore,
}

impl AgentCoreBuilder {
    pub fn new() -> Self {
        Self {
            core: AgentCore::new(),
        }
    }

    pub fn with_component(mut self, component: Box<dyn AgentComponent>) -> Self {
        self.core.register_component(component);
        self
    }

    pub fn build(self) -> AgentCore {
        self.core
    }
}

impl Default for AgentCoreBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    struct TestComponent {
        name: &'static str,
        initialized: Arc<AtomicBool>,
        ready: Arc<AtomicBool>,
        healthy: Arc<AtomicBool>,
    }

    impl TestComponent {
        fn new(name: &'static str) -> Self {
            Self {
                name,
                initialized: Arc::new(AtomicBool::new(false)),
                ready: Arc::new(AtomicBool::new(false)),
                healthy: Arc::new(AtomicBool::new(true)),
            }
        }
    }

    #[async_trait]
    impl AgentComponent for TestComponent {
        fn component_name(&self) -> &'static str {
            self.name
        }

        async fn initialize(&mut self) -> Result<()> {
            self.initialized.store(true, Ordering::Relaxed);
            self.ready.store(true, Ordering::Relaxed);
            Ok(())
        }

        async fn is_ready(&self) -> bool {
            self.ready.load(Ordering::Relaxed)
        }

        async fn health_check(&self) -> Result<()> {
            if self.healthy.load(Ordering::Relaxed) {
                Ok(())
            } else {
                Err(anyhow::anyhow!("Component is unhealthy"))
            }
        }

        async fn shutdown(&self) -> Result<()> {
            self.ready.store(false, Ordering::Relaxed);
            self.initialized.store(false, Ordering::Relaxed);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_agent_core_basic_lifecycle() {
        let mut core = AgentCore::new();

        // Регистрируем тестовые компоненты
        core.register_component(Box::new(TestComponent::new("test1")));
        core.register_component(Box::new(TestComponent::new("test2")));

        assert_eq!(core.component_count(), 2);
        assert!(!core.is_ready().await);

        // Инициализация
        core.initialize().await.expect("Operation should succeed");
        assert!(core.is_ready().await);

        // Health check
        let health = core.health_check().await.expect("Operation should succeed");
        assert_eq!(health.len(), 2);
        assert!(health.values().all(|&h| h));

        // Shutdown
        core.shutdown().await.expect("Operation should succeed");
        assert!(!core.is_ready().await);
    }

    #[tokio::test]
    async fn test_agent_core_builder() {
        let core = AgentCoreBuilder::new()
            .with_component(Box::new(TestComponent::new("component1")))
            .with_component(Box::new(TestComponent::new("component2")))
            .build();

        assert_eq!(core.component_count(), 2);
        assert!(core.component_names().contains(&"component1".to_string()));
        assert!(core.component_names().contains(&"component2".to_string()));
    }

    #[tokio::test]
    async fn test_component_stats_tracking() {
        let mut core = AgentCore::new();
        core.register_component(Box::new(TestComponent::new("stats_test")));

        core.initialize().await.expect("Operation should succeed");

        // Симулируем обработку запросов
        core.update_request_stats("stats_test", 100.0);
        core.update_request_stats("stats_test", 200.0);

        let stats = core.get_component_stats().await;
        let test_stats = &stats["stats_test"];

        assert_eq!(test_stats.requests_processed, 2);
        assert!(test_stats.average_response_time_ms > 0.0);
        assert!(test_stats.ready);
        assert!(test_stats.healthy);
    }
}
