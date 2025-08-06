//! Strategy Factory Pattern - динамическое создание и конфигурация стратегий
//! 
//! Реализует Abstract Factory для создания различных типов стратегий
//! с гибкой конфигурацией и возможностью runtime переключения.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

use crate::agent_traits::{
    IntentDecisionStrategy, FallbackStrategy, ResponseFormattingStrategy, LlmServiceTrait
};
use crate::strategies::{
    intent_strategies::{HeuristicIntentStrategy, LlmIntentStrategy, HybridIntentStrategy},
    fallback_strategies::{SimpleFallbackStrategy, SmartFallbackStrategy, CompositeFallbackStrategy, CircuitBreakerFallbackStrategy},
    response_strategies::{SimpleResponseFormatter, RichResponseFormatter, AdaptiveResponseFormatter}
};

// ============================================================================
// CONFIGURATION STRUCTS
// ============================================================================

/// Конфигурация для Intent Strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IntentStrategyConfig {
    /// Эвристическая стратегия на основе ключевых слов
    Heuristic {
        /// Минимальный порог уверенности
        confidence_threshold: f32,
        /// Дополнительные ключевые слова по категориям
        custom_keywords: Option<HashMap<String, Vec<String>>>,
    },
    /// LLM-based стратегия
    Llm {
        /// Системный prompt для классификации
        system_prompt: Option<String>,
        /// Максимальное время ожидания ответа
        timeout_seconds: Option<u64>,
        /// Использовать кэш для повторяющихся запросов
        enable_cache: bool,
    },
    /// Гибридная стратегия (эвристика + LLM)
    Hybrid {
        /// Порог для переключения с эвристики на LLM
        llm_threshold: f32,
        /// Конфигурация эвристической части
        heuristic: Box<IntentStrategyConfig>,
        /// Конфигурация LLM части
        llm: Box<IntentStrategyConfig>,
        /// Стратегия объединения результатов
        combination_strategy: CombinationStrategy,
    },
}

/// Стратегия объединения результатов в гибридном режиме
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CombinationStrategy {
    /// Выбрать результат с наибольшей уверенностью
    MaxConfidence,
    /// Взвешенное среднее
    WeightedAverage { heuristic_weight: f32, llm_weight: f32 },
    /// LLM приоритет при высокой уверенности, иначе эвристика
    LlmPriorityWithFallback,
}

/// Конфигурация для Fallback Strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FallbackStrategyConfig {
    /// Простая стратегия с предопределенными ответами
    Simple {
        /// Кастомные ответы для типов ошибок
        custom_responses: Option<HashMap<String, String>>,
    },
    /// Умная стратегия с альтернативными подходами
    Smart {
        /// Максимальное количество попыток
        max_retry_attempts: u32,
        /// Использовать альтернативные LLM провайдеры
        enable_provider_fallback: bool,
        /// Время ожидания для каждой попытки
        retry_timeout_seconds: u64,
    },
    /// Композитная стратегия с несколькими уровнями
    Composite {
        /// Конфигурации стратегий по приоритету (высший → низший)
        strategies: Vec<FallbackStrategyConfig>,
        /// Максимальное время для всего fallback процесса
        total_timeout_seconds: u64,
    },
    /// Circuit Breaker стратегия
    CircuitBreaker {
        /// Максимальное количество ошибок до открытия
        failure_threshold: u32,
        /// Время восстановления в секундах
        recovery_timeout_seconds: u64,
        /// Стратегия для open состояния
        open_state_strategy: Box<FallbackStrategyConfig>,
    },
}

/// Конфигурация для Response Formatting Strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseFormattingConfig {
    /// Простое форматирование
    Simple {
        /// Префикс для разных типов ответов
        prefixes: Option<HashMap<String, String>>,
        /// Включать временные метки
        include_timestamps: bool,
    },
    /// Богатое форматирование с markdown
    Rich {
        /// Использовать markdown разметку
        enable_markdown: bool,
        /// Включать эмодзи
        include_emojis: bool,
        /// Максимальная длина ответа
        max_length: Option<usize>,
    },
    /// Адаптивное форматирование на основе контекста
    Adaptive {
        /// Правила выбора форматирования
        rules: Vec<FormattingRule>,
        /// Форматтер по умолчанию
        default_formatter: Box<ResponseFormattingConfig>,
    },
}

/// Правило для адаптивного форматирования
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattingRule {
    /// Условие применения правила
    pub condition: FormattingCondition,
    /// Конфигурация форматтера для этого условия
    pub formatter_config: ResponseFormattingConfig,
    /// Приоритет правила (больше = выше приоритет)
    pub priority: u32,
}

/// Условие для применения правила форматирования
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FormattingCondition {
    /// На основе типа ответа
    ResponseType { response_types: Vec<String> },
    /// На основе длины сообщения
    MessageLength { min_length: Option<usize>, max_length: Option<usize> },
    /// На основе метаданных запроса
    Metadata { key: String, value: String },
    /// Комбинация условий
    And { conditions: Vec<FormattingCondition> },
    Or { conditions: Vec<FormattingCondition> },
}

// ============================================================================
// STRATEGY FACTORY TRAIT
// ============================================================================

/// Абстрактная фабрика для создания стратегий
pub trait StrategyFactory: Send + Sync {
    /// Создание Intent Decision Strategy
    fn create_intent_strategy(
        &self, 
        config: IntentStrategyConfig,
        llm_service: Option<Arc<dyn LlmServiceTrait>>,
    ) -> Result<Box<dyn IntentDecisionStrategy>>;

    /// Создание Fallback Strategy
    fn create_fallback_strategy(
        &self,
        config: FallbackStrategyConfig,
        llm_service: Option<Arc<dyn LlmServiceTrait>>,
    ) -> Result<Box<dyn FallbackStrategy>>;

    /// Создание Response Formatting Strategy
    fn create_response_strategy(
        &self,
        config: ResponseFormattingConfig,
    ) -> Result<Box<dyn ResponseFormattingStrategy>>;

    /// Валидация конфигурации перед созданием
    fn validate_config(&self, config: &StrategyFactoryConfig) -> Result<ValidationReport>;

    /// Получение доступных стратегий и их описаний
    fn get_available_strategies(&self) -> AvailableStrategies;
}

/// Полная конфигурация фабрики стратегий
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyFactoryConfig {
    pub intent_strategy: IntentStrategyConfig,
    pub fallback_strategy: FallbackStrategyConfig,
    pub response_strategy: ResponseFormattingConfig,
    /// Глобальные настройки
    pub global_settings: GlobalStrategySettings,
}

/// Глобальные настройки стратегий
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalStrategySettings {
    /// Включить подробное логирование
    pub enable_detailed_logging: bool,
    /// Включить метрики производительности
    pub enable_performance_metrics: bool,
    /// Максимальное время выполнения любой стратегии
    pub global_timeout_seconds: u64,
    /// Включить A/B тестирование стратегий
    pub enable_ab_testing: bool,
}

/// Отчет валидации конфигурации
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub suggestions: Vec<ValidationSuggestion>,
}

/// Ошибка валидации
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub severity: ErrorSeverity,
}

/// Предупреждение валидации
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    pub field: String,
    pub message: String,
    pub recommendation: String,
}

/// Предложение по улучшению
#[derive(Debug, Clone)]
pub struct ValidationSuggestion {
    pub category: String,
    pub suggestion: String,
    pub expected_improvement: String,
}

/// Severity уровень ошибки
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Критическая ошибка - стратегия не будет работать
    Critical,
    /// Важная ошибка - снижение производительности
    Major,
    /// Минорная ошибка - косметические проблемы
    Minor,
}

/// Информация о доступных стратегиях
#[derive(Debug, Clone)]
pub struct AvailableStrategies {
    pub intent_strategies: Vec<StrategyInfo>,
    pub fallback_strategies: Vec<StrategyInfo>,
    pub response_strategies: Vec<StrategyInfo>,
}

/// Информация о конкретной стратегии
#[derive(Debug, Clone)]
pub struct StrategyInfo {
    pub name: String,
    pub description: String,
    pub performance_characteristics: PerformanceCharacteristics,
    pub required_dependencies: Vec<String>,
    pub configuration_options: Vec<ConfigOption>,
}

/// Характеристики производительности стратегии
#[derive(Debug, Clone)]
pub struct PerformanceCharacteristics {
    /// Средняя латентность в миллисекундах
    pub avg_latency_ms: u64,
    /// Потребление памяти
    pub memory_usage: MemoryUsage,
    /// Нагрузка на CPU
    pub cpu_usage: CpuUsage,
    /// Точность (если применимо)
    pub accuracy: Option<f32>,
}

/// Уровень потребления памяти
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryUsage {
    Low,
    Medium,
    High,
}

/// Уровень нагрузки на CPU
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CpuUsage {
    Low,
    Medium,
    High,
}

/// Опция конфигурации стратегии
#[derive(Debug, Clone)]
pub struct ConfigOption {
    pub name: String,
    pub description: String,
    pub option_type: ConfigOptionType,
    pub default_value: Option<String>,
    pub required: bool,
}

/// Тип опции конфигурации
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigOptionType {
    String,
    Integer,
    Float,
    Boolean,
    Enum { options: Vec<String> },
}

// ============================================================================
// DEFAULT STRATEGY FACTORY IMPLEMENTATION
// ============================================================================

/// Стандартная реализация фабрики стратегий
pub struct DefaultStrategyFactory {
    global_settings: GlobalStrategySettings,
}

impl DefaultStrategyFactory {
    pub fn new(global_settings: GlobalStrategySettings) -> Self {
        Self { global_settings }
    }

    pub fn with_defaults() -> Self {
        Self {
            global_settings: GlobalStrategySettings {
                enable_detailed_logging: false,
                enable_performance_metrics: false,
                global_timeout_seconds: 30,
                enable_ab_testing: false,
            }
        }
    }

    fn create_heuristic_strategy(&self, config: IntentStrategyConfig) -> Result<Box<dyn IntentDecisionStrategy>> {
        match config {
            IntentStrategyConfig::Heuristic { confidence_threshold, custom_keywords: _ } => {
                Ok(Box::new(HeuristicIntentStrategy::new(confidence_threshold)))
            }
            _ => Err(anyhow::anyhow!("Invalid config for heuristic strategy")),
        }
    }

    fn create_llm_strategy(
        &self, 
        config: IntentStrategyConfig, 
        llm_service: Arc<dyn LlmServiceTrait>
    ) -> Result<Box<dyn IntentDecisionStrategy>> {
        match config {
            IntentStrategyConfig::Llm { system_prompt: _, timeout_seconds: _, enable_cache: _ } => {
                Ok(Box::new(LlmIntentStrategy::new(llm_service)))
            }
            _ => Err(anyhow::anyhow!("Invalid config for LLM strategy")),
        }
    }

    fn create_hybrid_strategy(
        &self,
        config: IntentStrategyConfig,
        llm_service: Arc<dyn LlmServiceTrait>
    ) -> Result<Box<dyn IntentDecisionStrategy>> {
        match config {
            IntentStrategyConfig::Hybrid { llm_threshold, heuristic: _, llm: _, combination_strategy: _ } => {
                Ok(Box::new(HybridIntentStrategy::new(llm_service, llm_threshold)))
            }
            _ => Err(anyhow::anyhow!("Invalid config for hybrid strategy")),
        }
    }
}

impl StrategyFactory for DefaultStrategyFactory {
    fn create_intent_strategy(
        &self,
        config: IntentStrategyConfig,
        llm_service: Option<Arc<dyn LlmServiceTrait>>,
    ) -> Result<Box<dyn IntentDecisionStrategy>> {
        match &config {
            IntentStrategyConfig::Heuristic { .. } => {
                self.create_heuristic_strategy(config)
            }
            IntentStrategyConfig::Llm { .. } => {
                let llm = llm_service.ok_or_else(|| 
                    anyhow::anyhow!("LLM service required for LLM strategy"))?;
                self.create_llm_strategy(config, llm)
            }
            IntentStrategyConfig::Hybrid { .. } => {
                let llm = llm_service.ok_or_else(|| 
                    anyhow::anyhow!("LLM service required for hybrid strategy"))?;
                self.create_hybrid_strategy(config, llm)
            }
        }
    }

    fn create_fallback_strategy(
        &self,
        config: FallbackStrategyConfig,
        llm_service: Option<Arc<dyn LlmServiceTrait>>,
    ) -> Result<Box<dyn FallbackStrategy>> {
        match config {
            FallbackStrategyConfig::Simple { custom_responses: _ } => {
                Ok(Box::new(SimpleFallbackStrategy::new()))
            }
            FallbackStrategyConfig::Smart { max_retry_attempts, enable_provider_fallback: _, retry_timeout_seconds: _ } => {
                let llm = llm_service.ok_or_else(|| 
                    anyhow::anyhow!("LLM service required for smart fallback strategy"))?;
                Ok(Box::new(SmartFallbackStrategy::new(Some(llm), max_retry_attempts)))
            }
            FallbackStrategyConfig::Composite { strategies, total_timeout_seconds: _ } => {
                let mut composite = CompositeFallbackStrategy::new();
                
                // Создаем каждую стратегию в композите
                for strategy_config in strategies {
                    let strategy = self.create_fallback_strategy(strategy_config, llm_service.clone())?;
                    composite = composite.add_strategy(strategy);
                }
                
                Ok(Box::new(composite))
            }
            FallbackStrategyConfig::CircuitBreaker { failure_threshold, recovery_timeout_seconds, open_state_strategy: _ } => {
                Ok(Box::new(CircuitBreakerFallbackStrategy::new(
                    failure_threshold, 
                    recovery_timeout_seconds
                )))
            }
        }
    }

    fn create_response_strategy(
        &self,
        config: ResponseFormattingConfig,
    ) -> Result<Box<dyn ResponseFormattingStrategy>> {
        match config {
            ResponseFormattingConfig::Simple { prefixes: _, include_timestamps: _ } => {
                Ok(Box::new(SimpleResponseFormatter::with_defaults()))
            }
            ResponseFormattingConfig::Rich { enable_markdown: _, include_emojis: _, max_length: _ } => {
                Ok(Box::new(RichResponseFormatter::new()))
            }
            ResponseFormattingConfig::Adaptive { rules: _, default_formatter: _ } => {
                Ok(Box::new(AdaptiveResponseFormatter::new()))
            }
        }
    }

    fn validate_config(&self, config: &StrategyFactoryConfig) -> Result<ValidationReport> {
        let mut report = ValidationReport {
            is_valid: true,
            errors: vec![],
            warnings: vec![],
            suggestions: vec![],
        };

        // Валидация intent strategy
        self.validate_intent_config(&config.intent_strategy, &mut report);
        
        // Валидация fallback strategy
        self.validate_fallback_config(&config.fallback_strategy, &mut report);
        
        // Валидация response strategy
        self.validate_response_config(&config.response_strategy, &mut report);
        
        // Валидация глобальных настроек
        self.validate_global_settings(&config.global_settings, &mut report);

        // Определяем общую валидность
        report.is_valid = report.errors.iter().all(|e| e.severity != ErrorSeverity::Critical);

        Ok(report)
    }

    fn get_available_strategies(&self) -> AvailableStrategies {
        AvailableStrategies {
            intent_strategies: vec![
                StrategyInfo {
                    name: "heuristic".to_string(),
                    description: "Keyword-based intent analysis with configurable confidence thresholds".to_string(),
                    performance_characteristics: PerformanceCharacteristics {
                        avg_latency_ms: 1,
                        memory_usage: MemoryUsage::Low,
                        cpu_usage: CpuUsage::Low,
                        accuracy: Some(0.75),
                    },
                    required_dependencies: vec![],
                    configuration_options: vec![
                        ConfigOption {
                            name: "confidence_threshold".to_string(),
                            description: "Minimum confidence threshold for decisions".to_string(),
                            option_type: ConfigOptionType::Float,
                            default_value: Some("0.5".to_string()),
                            required: false,
                        }
                    ],
                },
                StrategyInfo {
                    name: "llm".to_string(),
                    description: "LLM-powered intent analysis with natural language understanding".to_string(),
                    performance_characteristics: PerformanceCharacteristics {
                        avg_latency_ms: 500,
                        memory_usage: MemoryUsage::Medium,
                        cpu_usage: CpuUsage::Medium,
                        accuracy: Some(0.92),
                    },
                    required_dependencies: vec!["llm_service".to_string()],
                    configuration_options: vec![
                        ConfigOption {
                            name: "system_prompt".to_string(),
                            description: "Custom system prompt for intent classification".to_string(),
                            option_type: ConfigOptionType::String,
                            default_value: None,
                            required: false,
                        }
                    ],
                },
                StrategyInfo {
                    name: "hybrid".to_string(),
                    description: "Combines heuristic and LLM approaches for optimal accuracy and speed".to_string(),
                    performance_characteristics: PerformanceCharacteristics {
                        avg_latency_ms: 100,
                        memory_usage: MemoryUsage::Medium,
                        cpu_usage: CpuUsage::Medium,
                        accuracy: Some(0.88),
                    },
                    required_dependencies: vec!["llm_service".to_string()],
                    configuration_options: vec![
                        ConfigOption {
                            name: "llm_threshold".to_string(),
                            description: "Threshold for switching from heuristic to LLM analysis".to_string(),
                            option_type: ConfigOptionType::Float,
                            default_value: Some("0.7".to_string()),
                            required: false,
                        }
                    ],
                },
            ],
            fallback_strategies: vec![
                StrategyInfo {
                    name: "simple".to_string(),
                    description: "Basic fallback with predefined error responses".to_string(),
                    performance_characteristics: PerformanceCharacteristics {
                        avg_latency_ms: 1,
                        memory_usage: MemoryUsage::Low,
                        cpu_usage: CpuUsage::Low,
                        accuracy: None,
                    },
                    required_dependencies: vec![],
                    configuration_options: vec![],
                },
                StrategyInfo {
                    name: "smart".to_string(),
                    description: "Intelligent fallback with retry logic and alternative approaches".to_string(),
                    performance_characteristics: PerformanceCharacteristics {
                        avg_latency_ms: 50,
                        memory_usage: MemoryUsage::Medium,
                        cpu_usage: CpuUsage::Medium,
                        accuracy: None,
                    },
                    required_dependencies: vec!["llm_service".to_string()],
                    configuration_options: vec![
                        ConfigOption {
                            name: "max_retry_attempts".to_string(),
                            description: "Maximum number of retry attempts".to_string(),
                            option_type: ConfigOptionType::Integer,
                            default_value: Some("3".to_string()),
                            required: false,
                        }
                    ],
                },
            ],
            response_strategies: vec![
                StrategyInfo {
                    name: "simple".to_string(),
                    description: "Plain text responses with optional prefixes".to_string(),
                    performance_characteristics: PerformanceCharacteristics {
                        avg_latency_ms: 1,
                        memory_usage: MemoryUsage::Low,
                        cpu_usage: CpuUsage::Low,
                        accuracy: None,
                    },
                    required_dependencies: vec![],
                    configuration_options: vec![],
                },
                StrategyInfo {
                    name: "rich".to_string(),
                    description: "Rich formatting with markdown support and emojis".to_string(),
                    performance_characteristics: PerformanceCharacteristics {
                        avg_latency_ms: 5,
                        memory_usage: MemoryUsage::Low,
                        cpu_usage: CpuUsage::Low,
                        accuracy: None,
                    },
                    required_dependencies: vec![],
                    configuration_options: vec![
                        ConfigOption {
                            name: "enable_markdown".to_string(),
                            description: "Enable markdown formatting".to_string(),
                            option_type: ConfigOptionType::Boolean,
                            default_value: Some("true".to_string()),
                            required: false,
                        }
                    ],
                },
                StrategyInfo {
                    name: "adaptive".to_string(),
                    description: "Context-aware formatting that adapts to request characteristics".to_string(),
                    performance_characteristics: PerformanceCharacteristics {
                        avg_latency_ms: 10,
                        memory_usage: MemoryUsage::Medium,
                        cpu_usage: CpuUsage::Low,
                        accuracy: None,
                    },
                    required_dependencies: vec![],
                    configuration_options: vec![],
                },
            ],
        }
    }
}

impl DefaultStrategyFactory {
    fn validate_intent_config(&self, config: &IntentStrategyConfig, report: &mut ValidationReport) {
        match config {
            IntentStrategyConfig::Heuristic { confidence_threshold, .. } => {
                if *confidence_threshold < 0.0 || *confidence_threshold > 1.0 {
                    report.errors.push(ValidationError {
                        field: "intent_strategy.confidence_threshold".to_string(),
                        message: "Confidence threshold must be between 0.0 and 1.0".to_string(),
                        severity: ErrorSeverity::Critical,
                    });
                }
            }
            IntentStrategyConfig::Llm { timeout_seconds, .. } => {
                if let Some(timeout) = timeout_seconds {
                    if *timeout == 0 {
                        report.errors.push(ValidationError {
                            field: "intent_strategy.timeout_seconds".to_string(),
                            message: "Timeout must be greater than 0".to_string(),
                            severity: ErrorSeverity::Major,
                        });
                    }
                }
            }
            IntentStrategyConfig::Hybrid { llm_threshold, .. } => {
                if *llm_threshold < 0.0 || *llm_threshold > 1.0 {
                    report.errors.push(ValidationError {
                        field: "intent_strategy.llm_threshold".to_string(),
                        message: "LLM threshold must be between 0.0 and 1.0".to_string(),
                        severity: ErrorSeverity::Critical,
                    });
                }
            }
        }
    }

    fn validate_fallback_config(&self, config: &FallbackStrategyConfig, report: &mut ValidationReport) {
        match config {
            FallbackStrategyConfig::Smart { max_retry_attempts, .. } => {
                if *max_retry_attempts == 0 {
                    report.warnings.push(ValidationWarning {
                        field: "fallback_strategy.max_retry_attempts".to_string(),
                        message: "Zero retry attempts may reduce resilience".to_string(),
                        recommendation: "Consider setting at least 1-3 retry attempts".to_string(),
                    });
                }
                if *max_retry_attempts > 10 {
                    report.warnings.push(ValidationWarning {
                        field: "fallback_strategy.max_retry_attempts".to_string(),
                        message: "High retry count may cause excessive delays".to_string(),
                        recommendation: "Consider reducing to 3-5 attempts for better user experience".to_string(),
                    });
                }
            }
            FallbackStrategyConfig::CircuitBreaker { failure_threshold, .. } => {
                if *failure_threshold == 0 {
                    report.errors.push(ValidationError {
                        field: "fallback_strategy.failure_threshold".to_string(),
                        message: "Failure threshold must be greater than 0".to_string(),
                        severity: ErrorSeverity::Critical,
                    });
                }
            }
            FallbackStrategyConfig::Composite { strategies, .. } => {
                if strategies.is_empty() {
                    report.errors.push(ValidationError {
                        field: "fallback_strategy.strategies".to_string(),
                        message: "Composite strategy must have at least one strategy".to_string(),
                        severity: ErrorSeverity::Critical,
                    });
                }
            }
            _ => {}
        }
    }

    fn validate_response_config(&self, _config: &ResponseFormattingConfig, _report: &mut ValidationReport) {
        // Response formatting validation (если потребуется)
    }

    fn validate_global_settings(&self, settings: &GlobalStrategySettings, report: &mut ValidationReport) {
        if settings.global_timeout_seconds == 0 {
            report.errors.push(ValidationError {
                field: "global_settings.global_timeout_seconds".to_string(),
                message: "Global timeout must be greater than 0".to_string(),
                severity: ErrorSeverity::Critical,
            });
        }

        if settings.global_timeout_seconds > 300 {
            report.warnings.push(ValidationWarning {
                field: "global_settings.global_timeout_seconds".to_string(),
                message: "Very high global timeout may impact user experience".to_string(),
                recommendation: "Consider reducing to 30-60 seconds for better responsiveness".to_string(),
            });
        }

        if settings.enable_performance_metrics {
            report.suggestions.push(ValidationSuggestion {
                category: "performance".to_string(),
                suggestion: "Consider implementing metrics aggregation for better observability".to_string(),
                expected_improvement: "Better debugging and performance optimization capabilities".to_string(),
            });
        }
    }
}

impl Default for DefaultStrategyFactory {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Создание конфигурации для production окружения
pub fn create_production_config() -> StrategyFactoryConfig {
    StrategyFactoryConfig {
        intent_strategy: IntentStrategyConfig::Hybrid {
            llm_threshold: 0.7,
            heuristic: Box::new(IntentStrategyConfig::Heuristic {
                confidence_threshold: 0.6,
                custom_keywords: None,
            }),
            llm: Box::new(IntentStrategyConfig::Llm {
                system_prompt: None,
                timeout_seconds: Some(10),
                enable_cache: true,
            }),
            combination_strategy: CombinationStrategy::LlmPriorityWithFallback,
        },
        fallback_strategy: FallbackStrategyConfig::Composite {
            strategies: vec![
                FallbackStrategyConfig::CircuitBreaker {
                    failure_threshold: 5,
                    recovery_timeout_seconds: 30,
                    open_state_strategy: Box::new(FallbackStrategyConfig::Simple {
                        custom_responses: None,
                    }),
                },
                FallbackStrategyConfig::Smart {
                    max_retry_attempts: 3,
                    enable_provider_fallback: true,
                    retry_timeout_seconds: 5,
                },
                FallbackStrategyConfig::Simple {
                    custom_responses: None,
                },
            ],
            total_timeout_seconds: 30,
        },
        response_strategy: ResponseFormattingConfig::Adaptive {
            rules: vec![
                FormattingRule {
                    condition: FormattingCondition::ResponseType {
                        response_types: vec!["ToolExecution".to_string()],
                    },
                    formatter_config: ResponseFormattingConfig::Rich {
                        enable_markdown: true,
                        include_emojis: false,
                        max_length: Some(2000),
                    },
                    priority: 10,
                },
            ],
            default_formatter: Box::new(ResponseFormattingConfig::Simple {
                prefixes: None,
                include_timestamps: false,
            }),
        },
        global_settings: GlobalStrategySettings {
            enable_detailed_logging: false,
            enable_performance_metrics: true,
            global_timeout_seconds: 30,
            enable_ab_testing: false,
        },
    }
}

/// Создание конфигурации для development окружения
pub fn create_development_config() -> StrategyFactoryConfig {
    StrategyFactoryConfig {
        intent_strategy: IntentStrategyConfig::Heuristic {
            confidence_threshold: 0.5,
            custom_keywords: None,
        },
        fallback_strategy: FallbackStrategyConfig::Simple {
            custom_responses: None,
        },
        response_strategy: ResponseFormattingConfig::Rich {
            enable_markdown: true,
            include_emojis: true,
            max_length: None,
        },
        global_settings: GlobalStrategySettings {
            enable_detailed_logging: true,
            enable_performance_metrics: true,
            global_timeout_seconds: 60,
            enable_ab_testing: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_production_config_creation() {
        let config = create_production_config();
        
        // Production должен использовать hybrid strategy
        match config.intent_strategy {
            IntentStrategyConfig::Hybrid { .. } => {},
            _ => panic!("Production should use hybrid intent strategy"),
        }
        
        // Production должен иметь композитную fallback стратегию
        match config.fallback_strategy {
            FallbackStrategyConfig::Composite { .. } => {},
            _ => panic!("Production should use composite fallback strategy"),
        }
    }

    #[test]
    fn test_development_config_creation() {
        let config = create_development_config();
        
        // Development может использовать простую heuristic strategy
        match config.intent_strategy {
            IntentStrategyConfig::Heuristic { .. } => {},
            _ => panic!("Development can use heuristic intent strategy"),
        }
        
        assert!(config.global_settings.enable_detailed_logging);
        assert!(config.global_settings.enable_ab_testing);
    }

    #[test]
    fn test_validation_report_creation() {
        let factory = DefaultStrategyFactory::with_defaults();
        let config = create_production_config();
        
        let report = factory.validate_config(&config).expect("Валидация конфигурации в тестах");
        assert!(report.is_valid);
        assert!(report.errors.is_empty() || report.errors.iter().all(|e| e.severity != ErrorSeverity::Critical));
    }

    #[tokio::test]
    async fn test_strategy_factory_available_strategies() {
        let factory = DefaultStrategyFactory::with_defaults();
        let strategies = factory.get_available_strategies();
        
        assert!(!strategies.intent_strategies.is_empty());
        assert!(!strategies.fallback_strategies.is_empty());
        assert!(!strategies.response_strategies.is_empty());
        
        // Проверим что heuristic стратегия присутствует
        let heuristic_found = strategies.intent_strategies.iter()
            .any(|s| s.name == "heuristic");
        assert!(heuristic_found);
    }
}