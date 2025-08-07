//! Strategy patterns для UnifiedAgent Clean Architecture
//!
//! Реализует различные стратегии для:
//! - Intent Decision (определение намерений)
//! - Fallback (обработка ошибок)
//! - Response Formatting (форматирование ответов)

pub mod circuit_breaker;
pub mod fallback_strategies;
pub mod intent_strategies;
pub mod response_strategies;

pub use circuit_breaker::*;
pub use fallback_strategies::*;
pub use intent_strategies::*;
pub use response_strategies::*;
