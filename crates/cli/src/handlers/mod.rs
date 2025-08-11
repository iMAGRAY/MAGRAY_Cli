//! Специализированные обработчики для Clean Architecture UnifiedAgent
//!
//! Каждый handler реализует Single Responsibility Principle
//! и интегрируется через Dependency Injection

pub mod admin_handler;
pub mod chat_handler;
pub mod memory_handler;
pub mod performance_monitor;
pub mod tools_handler;

pub use admin_handler::*;
pub use chat_handler::*;
pub use memory_handler::*;
pub use performance_monitor::*;
pub use tools_handler::*;

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use tracing::info;

/// Общая инициализация компонента с проверкой зависимости
pub async fn standard_component_initialize<DFut, D, E>(
    component_name: &str,
    dep_check: DFut,
) -> Result<()>
where
    DFut: std::future::Future<Output = std::result::Result<D, E>>,
    E: std::fmt::Display,
{
    info!("{}: инициализация начата", component_name);
    dep_check
        .await
        .map(|_| ())
        .map_err(|e| anyhow!(e.to_string()))?;
    info!("{}: инициализация завершена", component_name);
    Ok(())
}

/// Общий graceful shutdown с логоированием
pub async fn standard_component_shutdown(component_name: &str) -> Result<()> {
    info!("{}: начинаем graceful shutdown", component_name);
    // место для общих шагов shutdown при необходимости
    info!("{}: shutdown завершен", component_name);
    Ok(())
}

/// Общая реализация health_check для компонентных обработчиков с Circuit Breaker и зависимостью
/// - Проверяет initialized
/// - Выполняет health_check зависимости (тип результата игнорируется)
/// - Проверяет состояние circuit breaker
pub async fn standard_component_health_check<DFut, SFut, D, E>(
    component_name: &str,
    initialized: bool,
    dep_check: DFut,
    breaker_state: SFut,
) -> Result<()>
where
    DFut: std::future::Future<Output = std::result::Result<D, E>>,
    E: std::fmt::Display + Send + Sync + 'static,
    SFut: std::future::Future<Output = String>,
{
    if !initialized {
        return Err(anyhow!(format!("{} не инициализирован", component_name)));
    }
    // Выполняем проверку зависимости; маппим любую ошибку в anyhow::Error
    dep_check
        .await
        .map(|_| ())
        .map_err(|e| anyhow!(e.to_string()))?;

    let state = breaker_state.await;
    if state == "Open" {
        return Err(anyhow!("Circuit breaker открыт"));
    }
    Ok(())
}

/// Общий хелпер для сборки статистики по ключам со стартовыми значениями
pub fn standard_usage_stats(items: &[(&str, u64)]) -> HashMap<String, u64> {
    let mut stats = HashMap::new();
    for (k, v) in items {
        stats.insert((*k).to_string(), *v);
    }
    stats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_standard_component_health_ok() {
        let dep = async { Ok::<(), &str>(()) };
        let breaker = async { "Closed".to_string() };
        let r = standard_component_health_check("Test", true, dep, breaker).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_standard_component_health_not_initialized() {
        let dep = async { Ok::<(), &str>(()) };
        let breaker = async { "Closed".to_string() };
        let r = standard_component_health_check("Test", false, dep, breaker).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn test_standard_component_health_open_breaker() {
        let dep = async { Ok::<(), &str>(()) };
        let breaker = async { "Open".to_string() };
        let r = standard_component_health_check("Test", true, dep, breaker).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn test_standard_component_initialize_ok() {
        let dep = async { Ok::<(), &str>(()) };
        let r = standard_component_initialize("InitTest", dep).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_standard_component_shutdown_ok() {
        let r = standard_component_shutdown("ShutdownTest").await;
        assert!(r.is_ok());
    }

    #[test]
    fn test_standard_usage_stats() {
        let s = standard_usage_stats(&[("a", 0), ("b", 5)]);
        assert_eq!(s.get("a"), Some(&0));
        assert_eq!(s.get("b"), Some(&5));
    }
}
