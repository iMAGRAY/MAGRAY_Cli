//! Clean Architecture traits для UnifiedAgent
//!
//! Реализует Dependency Inversion принцип через высокоуровневые абстракции
//! для всех компонентов агента согласно Clean Architecture

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// CORE DOMAIN TYPES
// ============================================================================

/// Результат обработки пользовательского запроса
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentResponse {
    /// Обычный chat ответ от LLM
    Chat(String),
    /// Результат выполнения tools/команд
    ToolExecution(String),
    /// Административный ответ (статистика, здоровье системы)
    Admin(AdminResponse),
    /// Ошибка с пользовательским сообщением
    Error(String),
}

/// Тип административного ответа
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdminResponse {
    /// Системная статистика
    SystemStats(String),
    /// Статус здоровья системы
    HealthStatus(String),
    /// Метрики производительности
    PerformanceMetrics(String),
    /// Результат выполнения операции
    OperationResult(String),
}

/// Результат анализа намерений пользователя
#[derive(Debug, Clone)]
pub struct IntentDecision {
    /// Тип действия: "chat", "tools", "admin", "memory"
    pub action_type: String,
    /// Уверенность в решении (0.0-1.0)
    pub confidence: f32,
    /// Дополнительные параметры для обработки
    pub context: Option<HashMap<String, String>>,
}

/// Контекст для обработки запроса
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Текст пользовательского запроса
    pub message: String,
    /// Метаданные сессии
    pub session_id: String,
    /// Дополнительный контекст
    pub metadata: HashMap<String, String>,
}

/// Результат обработки с метриками
#[derive(Debug)]
pub struct ProcessingResult {
    /// Ответ пользователю
    pub response: AgentResponse,
    /// Время обработки в миллисекундах
    pub processing_time_ms: u64,
    /// Использованные компоненты
    pub components_used: Vec<String>,
    /// Метрики производительности
    pub metrics: HashMap<String, f64>,
}

// ============================================================================
// STRATEGY PATTERNS TRAITS
// ============================================================================

/// Strategy для принятия решений о намерениях пользователя
#[async_trait]
pub trait IntentDecisionStrategy: Send + Sync {
    /// Анализирует запрос и определяет намерение
    async fn analyze_intent(&self, context: &RequestContext) -> Result<IntentDecision>;

    /// Возвращает название стратегии для логирования
    fn strategy_name(&self) -> &'static str;
}

/// Strategy для обработки fallback сценариев
#[async_trait]
pub trait FallbackStrategy: Send + Sync {
    /// Обрабатывает запрос когда основные стратегии не сработали
    async fn handle_fallback(
        &self,
        context: &RequestContext,
        error: &anyhow::Error,
    ) -> Result<AgentResponse>;

    /// Проверяет может ли стратегия обработать данную ошибку
    fn can_handle(&self, error: &anyhow::Error) -> bool;

    /// Приоритет стратегии (выше = важнее)
    fn priority(&self) -> u8;
}

/// Strategy для форматирования ответов
#[async_trait]
pub trait ResponseFormattingStrategy: Send + Sync {
    /// Форматирует ответ для пользователя
    async fn format_response(
        &self,
        response: &AgentResponse,
        context: &RequestContext,
    ) -> Result<String>;

    /// Поддерживаемые типы ответов
    fn supported_response_types(&self) -> Vec<&'static str>;
}

// ============================================================================
// CORE COMPONENT TRAITS (Dependency Inversion)
// ============================================================================

/// Высокоуровневая абстракция для LLM взаимодействия
#[async_trait]
pub trait LlmServiceTrait: Send + Sync {
    /// Простой chat запрос
    async fn chat(&self, message: &str) -> Result<String>;

    /// Chat с контекстом
    async fn chat_with_context(
        &self,
        message: &str,
        context: &HashMap<String, String>,
    ) -> Result<String>;

    /// Проверка доступности сервиса
    async fn health_check(&self) -> Result<()>;
}

/// Высокоуровневая абстракция для интеллектуальной маршрутизации
#[async_trait]
pub trait IntelligentRoutingTrait: Send + Sync {
    /// Обработка умного запроса с планированием и выполнением
    async fn process_request(&self, query: &str) -> Result<String>;

    /// Анализ запроса без выполнения (для предварительной оценки)
    async fn analyze_request(&self, query: &str) -> Result<String>;
}

/// Высокоуровневая абстракция для управления памятью
#[async_trait]
pub trait MemoryManagementTrait: Send + Sync {
    /// Сохранение пользовательского сообщения
    async fn store_message(&self, message: &str, context: &HashMap<String, String>) -> Result<()>;

    /// Поиск релевантной информации
    async fn search_memory(&self, query: &str, limit: usize) -> Result<Vec<String>>;

    /// Запуск процесса продвижения между слоями памяти
    async fn run_promotion(&self) -> Result<String>;

    /// Получение статистики системы памяти
    async fn get_memory_stats(&self) -> Result<String>;

    /// Проверка здоровья системы памяти
    async fn health_check(&self) -> Result<()>;
}

/// Высокоуровневая абстракция для административных операций
#[async_trait]
pub trait AdminServiceTrait: Send + Sync {
    /// Получение системной статистики
    async fn get_system_stats(&self) -> Result<AdminResponse>;

    /// Проверка здоровья всех компонентов
    async fn check_system_health(&self) -> Result<AdminResponse>;

    /// Получение метрик производительности
    async fn get_performance_metrics(&self) -> Result<AdminResponse>;

    /// Выполнение административных команд
    async fn execute_admin_command(
        &self,
        command: &str,
        args: &HashMap<String, String>,
    ) -> Result<AdminResponse>;
}

/// Высокоуровневая абстракция для мониторинга производительности
#[async_trait]
pub trait PerformanceMonitoringTrait: Send + Sync {
    /// Начало измерения операции
    fn start_operation(&self, operation_name: &str) -> String;

    /// Завершение измерения операции
    fn finish_operation(&self, operation_id: &str, success: bool);

    /// Получение метрик за период
    async fn get_metrics(&self, period_minutes: u32) -> Result<HashMap<String, f64>>;

    /// Сброс всех метрик
    fn reset_metrics(&self);
}

// ============================================================================
// HIGH-LEVEL ORCHESTRATION TRAITS
// ============================================================================

/// Главный trait для обработки пользовательских запросов
#[async_trait]
pub trait RequestProcessorTrait: Send + Sync {
    /// Основная точка входа для обработки запросов
    async fn process_user_request(&self, context: RequestContext) -> Result<ProcessingResult>;

    /// Проверка готовности системы к обработке
    async fn is_ready(&self) -> bool;

    /// Graceful shutdown всех компонентов
    async fn shutdown(&self) -> Result<()>;
}

/// Trait для управления жизненным циклом компонентов
#[async_trait]
pub trait ComponentLifecycleTrait: Send + Sync {
    /// Инициализация компонента
    async fn initialize(&self) -> Result<()>;

    /// Проверка здоровья компонента
    async fn health_check(&self) -> Result<()>;

    /// Graceful shutdown компонента
    async fn shutdown(&self) -> Result<()>;

    /// Перезапуск компонента
    async fn restart(&self) -> Result<()> {
        self.shutdown().await?;
        self.initialize().await
    }
}

// ============================================================================
// CIRCUIT BREAKER TRAIT
// ============================================================================

/// Trait для реализации Circuit Breaker паттерна
#[async_trait]
pub trait CircuitBreakerTrait: Send + Sync {
    /// Выполнение операции с Circuit Breaker защитой
    async fn execute<F, T>(&self, operation: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>> + Send,
        T: Send;

    /// Принудительное открытие автомата
    async fn force_open(&self);

    /// Принудительное закрытие автомата
    async fn force_close(&self);

    /// Получение текущего состояния
    async fn get_state(&self) -> String;
}
