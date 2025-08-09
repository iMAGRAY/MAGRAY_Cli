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
    dep_check.await.map(|_| ()).map_err(|e| anyhow!(e.to_string()))?;
    info!("{}: инициализация завершена", component_name);
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
