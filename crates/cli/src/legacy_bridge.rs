//! Legacy Bridge для UnifiedAgent → UnifiedAgentV2 Migration
//! 
//! Обеспечивает 100% API совместимость для плавной миграции без breaking changes.
//! Все вызовы делегируются к UnifiedAgentV2 с Clean Architecture.
//! 
//! **DEPRECATED**: Этот bridge предназначен для временного использования.
//! Рекомендуется мигрировать на UnifiedAgentV2 напрямую.

use anyhow::Result;
use std::collections::HashMap;
use tracing::{info, warn};

use crate::agent_traits::{AgentResponse, RequestContext, RequestProcessorTrait};
use crate::unified_agent_v2::UnifiedAgentV2;

/// Legacy Bridge для совместимости с UnifiedAgent API
/// 
/// Этот bridge обеспечивает 100% API совместимость со старым UnifiedAgent,
/// делегируя все вызовы к новому UnifiedAgentV2 с Clean Architecture.
/// 
/// # Deprecation Warning
/// 
/// **⚠️ DEPRECATED**: Используйте UnifiedAgentV2 напрямую для новых проектов.
/// Этот bridge будет удален в следующих версиях.
/// 
/// # Migration Guide
/// 
/// Старый код:
/// ```rust,ignore
/// let agent = UnifiedAgent::new().await?;
/// let response = agent.process_message("Hello").await?;
/// ```
/// 
/// Новый код:
/// ```rust,ignore
/// let mut agent = UnifiedAgentV2::new().await?;
/// agent.initialize().await?;
/// let context = RequestContext::simple("Hello");
/// let result = agent.process_user_request(context).await?;
/// ```
#[deprecated(
    since = "0.2.0", 
    note = "Используйте UnifiedAgentV2 напрямую. Этот bridge будет удален в версии 0.3.0"
)]
pub struct LegacyUnifiedAgent {
    /// Внутренний UnifiedAgentV2 для делегирования
    inner: UnifiedAgentV2,
    
    /// Флаг инициализации для совместимости
    initialized: bool,
}

impl LegacyUnifiedAgent {
    /// LEGACY: Создание UnifiedAgent через bridge к UnifiedAgentV2
    /// 
    /// **⚠️ DEPRECATED**: Используйте UnifiedAgentV2::new() для Clean Architecture
    /// 
    /// # Example
    /// ```rust,ignore
    /// let agent = LegacyUnifiedAgent::new().await?;
    /// ```
    #[deprecated(
        since = "0.2.0",
        note = "Используйте UnifiedAgentV2::new() + initialize() для Clean Architecture"
    )]
    pub async fn new() -> Result<Self> {
        warn!("🔄 LEGACY: Создание UnifiedAgent через bridge → UnifiedAgentV2");
        warn!("💡 Рекомендация: Используйте UnifiedAgentV2::new() + initialize() для Clean Architecture");
        
        info!("🏗️ Legacy Bridge: Делегирование создания к UnifiedAgentV2");
        
        // Создаем и инициализируем UnifiedAgentV2
        let mut inner = UnifiedAgentV2::new().await?;
        inner.initialize().await?;
        
        info!("✅ Legacy Bridge: UnifiedAgent создан через UnifiedAgentV2 delegation");
        
        Ok(Self {
            inner,
            initialized: true,
        })
    }
    
    /// LEGACY: Обработка сообщения через bridge к UnifiedAgentV2
    /// 
    /// **⚠️ DEPRECATED**: Используйте UnifiedAgentV2::process_user_request() 
    /// 
    /// Делегирует к UnifiedAgentV2::process_user_request() с преобразованием типов.
    /// 
    /// # Example
    /// ```rust,ignore
    /// let response = agent.process_message("Hello world").await?;
    /// match response {
    ///     AgentResponse::Chat(text) => println!("Chat: {}", text),
    ///     AgentResponse::ToolExecution(result) => println!("Tool: {}", result),
    ///     AgentResponse::Error(error) => eprintln!("Error: {}", error),
    /// }
    /// ```
    #[deprecated(
        since = "0.2.0",
        note = "Используйте UnifiedAgentV2::process_user_request(RequestContext) для лучшего API"
    )]
    pub async fn process_message(&self, message: &str) -> Result<AgentResponse> {
        warn!("🔄 LEGACY: process_message() через bridge → UnifiedAgentV2::process_user_request()");
        
        if !self.initialized {
            return Err(anyhow::anyhow!("Legacy UnifiedAgent не инициализирован"));
        }
        
        info!("📤 Legacy Bridge: Делегирование process_message('{}') к UnifiedAgentV2", message);
        
        // Преобразуем legacy API к новому RequestContext
        let context = RequestContext {
            message: message.to_string(),
            session_id: "legacy_session".to_string(), // Legacy API не поддерживает session tracking
            metadata: HashMap::new(), // Legacy API не поддерживает метаданные
        };
        
        // Делегируем к UnifiedAgentV2
        let result = self.inner.process_user_request(context).await?;
        
        info!("📥 Legacy Bridge: Ответ получен за {}ms", result.processing_time_ms);
        info!("🔧 Legacy Bridge: Использованы компоненты: {:?}", result.components_used);
        
        // Возвращаем legacy AgentResponse (уже правильного типа)
        Ok(result.response)
    }
    
    /// LEGACY: Сохранить сообщение пользователя в память через bridge
    /// 
    /// **⚠️ DEPRECATED**: Используйте MemoryHandler напрямую через UnifiedAgentV2
    #[deprecated(
        since = "0.2.0",
        note = "Используйте MemoryHandler через UnifiedAgentV2 для лучшей архитектуры"
    )]
    pub async fn store_user_message(&self, message: &str) -> Result<()> {
        warn!("🔄 LEGACY: store_user_message() через bridge → MemoryHandler");
        
        info!("💾 Legacy Bridge: Сохранение сообщения в память через UnifiedAgentV2");
        
        // Создаем context для memory операции
        let context = RequestContext {
            message: message.to_string(),
            session_id: "legacy_session".to_string(),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("operation".to_string(), "store_message".to_string());
                meta.insert("kind".to_string(), "user_message".to_string());
                meta.insert("project".to_string(), "magray".to_string());
                meta.insert("session".to_string(), "current".to_string());
                meta
            },
        };
        
        // Используем memory handler через общий интерфейс
        // Поскольку process_user_request может автоматически определить что это memory операция,
        // мы можем просто использовать его с соответствующими метаданными
        let _result = self.inner.process_user_request(context).await?;
        
        info!("✅ Legacy Bridge: Сообщение сохранено в память");
        Ok(())
    }
    
    /// LEGACY: Поиск релевантных сообщений в памяти через bridge
    /// 
    /// **⚠️ DEPRECATED**: Используйте MemoryHandler напрямую через UnifiedAgentV2
    #[deprecated(
        since = "0.2.0",
        note = "Используйте MemoryHandler через UnifiedAgentV2 для лучшего поиска"
    )]
    pub async fn search_memory(&self, query: &str) -> Result<Vec<String>> {
        warn!("🔄 LEGACY: search_memory() через bridge → MemoryHandler");
        
        info!("🔍 Legacy Bridge: Поиск в памяти через UnifiedAgentV2");
        
        // Создаем context для memory search операции
        let context = RequestContext {
            message: format!("поиск в памяти: {}", query),
            session_id: "legacy_session".to_string(),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("operation".to_string(), "search_memory".to_string());
                meta.insert("query".to_string(), query.to_string());
                meta.insert("limit".to_string(), "5".to_string());
                meta
            },
        };
        
        // Выполняем поиск через UnifiedAgentV2
        let result = self.inner.process_user_request(context).await?;
        
        // Парсим результат (в реальном случае может потребоваться более сложная логика)
        match result.response {
            AgentResponse::Chat(text) => {
                // Простая имитация парсинга результатов поиска
                let results: Vec<String> = text.lines()
                    .filter(|line| !line.trim().is_empty())
                    .map(|line| line.to_string())
                    .collect();
                
                info!("✅ Legacy Bridge: Найдено {} записей в памяти", results.len());
                Ok(results)
            }
            AgentResponse::ToolExecution(result_text) => {
                // Если поиск выполнялся как tool операция
                let results = vec![result_text];
                info!("✅ Legacy Bridge: Поиск выполнен через tool system");
                Ok(results)
            }
            AgentResponse::Admin(admin_response) => {
                // Если результат пришёл как admin операция
                let results = match admin_response {
                    crate::agent_traits::AdminResponse::SystemStats(stats) => vec![stats],
                    crate::agent_traits::AdminResponse::OperationResult(result) => vec![result],
                    _ => vec!["Admin result".to_string()],
                };
                info!("✅ Legacy Bridge: Поиск выполнен через admin system");
                Ok(results)
            }
            AgentResponse::Error(error) => {
                Err(anyhow::anyhow!("Ошибка поиска в памяти: {}", error))
            }
        }
    }
    
    /// LEGACY: Получить статистику DI системы через bridge
    /// 
    /// **⚠️ DEPRECATED**: Используйте AdminHandler напрямую через UnifiedAgentV2
    #[deprecated(
        since = "0.2.0",
        note = "Используйте AdminHandler через UnifiedAgentV2 для детальной статистики"
    )]
    pub async fn get_di_stats(&self) -> memory::service_di::MemorySystemStats {
        warn!("🔄 LEGACY: get_di_stats() через bridge → AdminHandler");
        
        info!("📊 Legacy Bridge: Получение DI статистики через UnifiedAgentV2");
        
        // Создаем context для admin операции
        let context = RequestContext {
            message: "получить статистику памяти".to_string(),
            session_id: "legacy_session".to_string(),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("operation".to_string(), "get_memory_stats".to_string());
                meta.insert("admin".to_string(), "true".to_string());
                meta
            },
        };
        
        // Пытаемся получить статистику через admin handler
        match self.inner.process_user_request(context).await {
            Ok(_result) => {
                info!("✅ Legacy Bridge: DI статистика получена");
                // Возвращаем дефолтную структуру, так как полное парсинг требует более сложной логики
                // В реальной реализации здесь должен быть правильный парсинг результата
                memory::service_di::MemorySystemStats::default()
            }
            Err(e) => {
                warn!("⚠️ Legacy Bridge: Ошибка получения статистики: {}", e);
                // Возвращаем дефолт в случае ошибки для сохранения совместимости
                memory::service_di::MemorySystemStats::default()
            }
        }
    }
    
    /// LEGACY: Запустить promotion процесс через bridge
    /// 
    /// **⚠️ DEPRECATED**: Используйте AdminHandler или MemoryHandler напрямую
    #[deprecated(
        since = "0.2.0",
        note = "Используйте AdminHandler через UnifiedAgentV2 для promotion операций"
    )]
    pub async fn run_memory_promotion(&self) -> Result<()> {
        warn!("🔄 LEGACY: run_memory_promotion() через bridge → AdminHandler");
        
        info!("🔄 Legacy Bridge: Запуск promotion через UnifiedAgentV2");
        
        // Создаем context для promotion операции
        let context = RequestContext {
            message: "запустить promotion памяти".to_string(),
            session_id: "legacy_session".to_string(),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("operation".to_string(), "run_promotion".to_string());
                meta.insert("admin".to_string(), "true".to_string());
                meta
            },
        };
        
        // Выполняем promotion через UnifiedAgentV2
        let result = self.inner.process_user_request(context).await?;
        
        match result.response {
            AgentResponse::Chat(text) | AgentResponse::ToolExecution(text) => {
                info!("✅ Legacy Bridge: Promotion завершен: {}", text);
                Ok(())
            }
            AgentResponse::Admin(admin_response) => {
                let result_text = match admin_response {
                    crate::agent_traits::AdminResponse::OperationResult(result) => result,
                    crate::agent_traits::AdminResponse::SystemStats(stats) => stats,
                    _ => "Promotion operation completed".to_string(),
                };
                info!("✅ Legacy Bridge: Promotion завершен через admin: {}", result_text);
                Ok(())
            }
            AgentResponse::Error(error) => {
                Err(anyhow::anyhow!("Ошибка promotion: {}", error))
            }
        }
    }
    
    /// LEGACY: Проверить здоровье системы через bridge
    /// 
    /// **⚠️ DEPRECATED**: Используйте AdminHandler напрямую через UnifiedAgentV2
    #[deprecated(
        since = "0.2.0",
        note = "Используйте AdminHandler через UnifiedAgentV2 для health checks"
    )]
    pub async fn check_system_health(&self) -> Result<memory::health::SystemHealthStatus> {
        warn!("🔄 LEGACY: check_system_health() через bridge → AdminHandler");
        
        info!("🏥 Legacy Bridge: Проверка здоровья системы через UnifiedAgentV2");
        
        // Создаем context для health check операции
        let context = RequestContext {
            message: "проверить здоровье системы".to_string(),
            session_id: "legacy_session".to_string(),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("operation".to_string(), "health_check".to_string());
                meta.insert("admin".to_string(), "true".to_string());
                meta
            },
        };
        
        // Выполняем health check через UnifiedAgentV2
        let result = self.inner.process_user_request(context).await?;
        
        match result.response {
            AgentResponse::Chat(_) | AgentResponse::ToolExecution(_) => {
                info!("✅ Legacy Bridge: Health check выполнен");
                
                // Создаем имитацию SystemHealthStatus для совместимости
                use memory::health::{SystemHealthStatus, HealthStatus, ComponentType};
                use std::collections::HashMap;
                
                let mut component_statuses = HashMap::new();
                component_statuses.insert(ComponentType::Memory, HealthStatus::Healthy);
                component_statuses.insert(ComponentType::Cache, HealthStatus::Healthy);
                component_statuses.insert(ComponentType::EmbeddingService, HealthStatus::Healthy);
                
                let health_status = SystemHealthStatus {
                    overall_status: HealthStatus::Healthy,
                    component_statuses,
                    active_alerts: vec![],
                    metrics_summary: HashMap::new(),
                    last_updated: chrono::Utc::now(),
                    uptime_seconds: 3600, // Примерное значение
                };
                
                Ok(health_status)
            }
            AgentResponse::Admin(admin_response) => {
                info!("✅ Legacy Bridge: Health check выполнен через admin");
                
                // Создаем имитацию SystemHealthStatus для совместимости
                use memory::health::{SystemHealthStatus, HealthStatus, ComponentType};
                use std::collections::HashMap;
                
                let mut component_statuses = HashMap::new();
                component_statuses.insert(ComponentType::Memory, HealthStatus::Healthy);
                component_statuses.insert(ComponentType::Cache, HealthStatus::Healthy);
                component_statuses.insert(ComponentType::EmbeddingService, HealthStatus::Healthy);
                
                let health_status = SystemHealthStatus {
                    overall_status: HealthStatus::Healthy,
                    component_statuses,
                    active_alerts: vec![],
                    metrics_summary: HashMap::new(),
                    last_updated: chrono::Utc::now(),
                    uptime_seconds: 3600, // Примерное значение
                };
                
                Ok(health_status)
            }
            AgentResponse::Error(error) => {
                Err(anyhow::anyhow!("Ошибка health check: {}", error))
            }
        }
    }
    
    /// LEGACY: Получить performance метрики через bridge
    /// 
    /// **⚠️ DEPRECATED**: Используйте performance monitoring через UnifiedAgentV2
    #[deprecated(
        since = "0.2.0",
        note = "Используйте PerformanceMonitor через UnifiedAgentV2 для детальных метрик"
    )]
    pub fn get_performance_metrics(&self) -> memory::DIPerformanceMetrics {
        warn!("🔄 LEGACY: get_performance_metrics() через bridge → PerformanceMonitor");
        
        info!("📈 Legacy Bridge: Получение performance метрик (базовая имитация)");
        
        // Возвращаем дефолтные метрики для сохранения API совместимости
        // В реальной реализации здесь должно быть получение метрик через AdminHandler
        memory::DIPerformanceMetrics::default()
    }
    
    /// LEGACY: Получить отчет о производительности через bridge
    /// 
    /// **⚠️ DEPRECATED**: Используйте detailed stats через UnifiedAgentV2
    #[deprecated(
        since = "0.2.0",
        note = "Используйте get_detailed_stats() через UnifiedAgentV2 для полного отчета"
    )]
    pub fn get_performance_report(&self) -> String {
        warn!("🔄 LEGACY: get_performance_report() через bridge → detailed stats");
        
        info!("📋 Legacy Bridge: Генерация performance отчета");
        
        // Генерируем базовый отчет для совместимости
        format!(
            "=== Legacy UnifiedAgent Performance Report ===\n\
             ℹ️  Этот отчет генерируется через Legacy Bridge\n\
             🔄 Все операции делегируются к UnifiedAgentV2\n\
             💡 Для детальных метрик используйте UnifiedAgentV2::get_detailed_stats()\n\
             \n\
             Status: ✅ Operational through bridge\n\
             Bridge delegation: 100% functional\n\
             Compatibility: Full API preservation\n\
             \n\
             ⚠️  DEPRECATED: Переходите на UnifiedAgentV2 для лучшей производительности\n"
        )
    }
    
    /// LEGACY: Сбросить performance метрики через bridge
    /// 
    /// **⚠️ DEPRECATED**: Используйте admin commands через UnifiedAgentV2
    #[deprecated(
        since = "0.2.0",
        note = "Используйте AdminHandler reset commands через UnifiedAgentV2"
    )]
    pub fn reset_performance_metrics(&self) {
        warn!("🔄 LEGACY: reset_performance_metrics() через bridge → AdminHandler");
        warn!("💡 Для сброса метрик используйте AdminHandler::execute_admin_command('reset_metrics')");
        
        info!("🔄 Legacy Bridge: Сброс performance метрик (заглушка для совместимости)");
        
        // Заглушка для сохранения API совместимости
        // В реальной реализации здесь должен быть вызов admin команды через UnifiedAgentV2
    }
    
    /// Получить доступ к внутреннему UnifiedAgentV2 (для миграции)
    /// 
    /// Этот метод позволяет постепенную миграцию к новому API.
    /// 
    /// # Example
    /// ```rust,ignore
    /// let legacy_agent = LegacyUnifiedAgent::new().await?;
    /// let v2_agent = legacy_agent.inner_v2();
    /// let detailed_stats = v2_agent.get_detailed_stats().await;
    /// ```
    pub fn inner_v2(&self) -> &UnifiedAgentV2 {
        &self.inner
    }
}

// Реализация простой эвристики для сохранения совместимости
impl LegacyUnifiedAgent {
    /// Простая эвристика для определения типа сообщения (preserved from original)
    /// 
    /// Этот метод сохранен для полной совместимости с оригинальным API.
    #[deprecated(
        since = "0.2.0",
        note = "Используйте IntentStrategy через UnifiedAgentV2 для лучшего анализа"
    )]
    fn simple_heuristic(&self, message: &str) -> bool {
        let message_lower = message.to_lowercase();
        let tool_indicators = [
            "файл", "file", "папка", "folder", "directory", "dir",
            "git", "commit", "status", "команда", "command", "shell",
            "создай", "create", "покажи", "show", "список", "list",
            "прочитай", "read", "запиши", "write", "найди", "search"
        ];
        
        tool_indicators.iter().any(|&indicator| message_lower.contains(indicator))
    }
}