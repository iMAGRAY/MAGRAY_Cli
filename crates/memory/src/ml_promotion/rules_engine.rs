use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

use super::traits::PromotionRulesEngine;
use super::types::PromotionDecision;
use crate::types::{Layer, Record};

/// Configurable promotion rules engine
#[derive(Debug, Clone)]
pub struct ConfigurableRulesEngine {
    config: RulesConfig,
    layer_strategies: HashMap<Layer, LayerStrategy>,
    business_rules: Vec<BusinessRule>,
    promotion_history: PromotionHistory,
}

/// Конфигурация для business rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesConfig {
    /// Минимальный период между повторными promotion одной записи (часы)
    pub min_repromotion_interval_hours: u64,
    /// Максимальное количество promotion записи за день
    pub max_promotions_per_record_per_day: u32,
    /// Минимальное время жизни в текущем layer перед promotion (часы)
    pub min_layer_residence_time_hours: u64,
    /// Включить strict validation rules
    pub strict_validation: bool,
    /// Максимальная нагрузка системы для promotion (0.0-1.0)
    pub max_system_load_for_promotion: f32,
    /// Временные окна для aggressive/conservative promotion
    pub time_windows: TimeWindowConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeWindowConfig {
    /// Час начала aggressive promotion (0-23)
    pub aggressive_start_hour: u8,
    /// Час окончания aggressive promotion (0-23)
    pub aggressive_end_hour: u8,
    /// Множитель threshold во время aggressive периода
    pub aggressive_threshold_multiplier: f32,
    /// Множитель threshold во время conservative периода
    pub conservative_threshold_multiplier: f32,
}

impl Default for RulesConfig {
    fn default() -> Self {
        Self {
            min_repromotion_interval_hours: 24,
            max_promotions_per_record_per_day: 3,
            min_layer_residence_time_hours: 1,
            strict_validation: false,
            max_system_load_for_promotion: 0.8,
            time_windows: TimeWindowConfig {
                aggressive_start_hour: 9,
                aggressive_end_hour: 17,
                aggressive_threshold_multiplier: 0.8,
                conservative_threshold_multiplier: 1.2,
            },
        }
    }
}

impl RulesConfig {
    pub fn production() -> Self {
        Self {
            min_repromotion_interval_hours: 48,
            max_promotions_per_record_per_day: 2,
            min_layer_residence_time_hours: 6,
            strict_validation: true,
            max_system_load_for_promotion: 0.7,
            time_windows: TimeWindowConfig {
                aggressive_start_hour: 10,
                aggressive_end_hour: 16,
                aggressive_threshold_multiplier: 0.9,
                conservative_threshold_multiplier: 1.1,
            },
        }
    }

    pub fn development() -> Self {
        Self {
            min_repromotion_interval_hours: 1,
            max_promotions_per_record_per_day: 10,
            min_layer_residence_time_hours: 0,
            strict_validation: false,
            max_system_load_for_promotion: 1.0,
            time_windows: TimeWindowConfig {
                aggressive_start_hour: 0,
                aggressive_end_hour: 23,
                aggressive_threshold_multiplier: 0.5,
                conservative_threshold_multiplier: 1.0,
            },
        }
    }
}

/// Стратегия для каждого layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerStrategy {
    /// Минимальный access_count для promotion из этого layer
    pub min_access_count: u32,
    /// Минимальный confidence score для promotion
    pub min_confidence_score: f32,
    /// Максимальный возраст записи в часах для promotion
    pub max_age_hours: Option<u64>,
    /// Приоритет layer при выборе кандидатов
    pub priority: u8,
    /// Специальные условия для layer
    pub special_conditions: Vec<SpecialCondition>,
}

/// Специальные условия для promotion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpecialCondition {
    /// Требует минимальную длину текста
    MinTextLength(usize),
    /// Требует наличие ключевых слов
    RequiredKeywords(Vec<String>),
    /// Исключает записи с определенными словами
    BlacklistKeywords(Vec<String>),
    /// Требует минимальную семантическую важность
    MinSemanticImportance(f32),
    /// Ограничение по времени суток
    TimeOfDayRestriction { start_hour: u8, end_hour: u8 },
}

/// Бизнес-правило
#[derive(Debug, Clone)]
pub enum BusinessRule {
    /// Максимальное количество promotion в час
    MaxPromotionsPerHour(u32),
    /// Балансировка между layers
    LayerBalance {
        min_interact_ratio: f32,
        max_assets_ratio: f32,
    },
    /// Содержание не должно быть дублированным
    NoDuplicateContent,
    /// Пользовательские правила на основе access patterns
    UserAccessPatterns,
}

/// История promotion для предотвращения дубликатов
#[derive(Debug, Clone)]
pub struct PromotionHistory {
    recent_promotions: HashMap<uuid::Uuid, Vec<DateTime<Utc>>>,
    daily_counts: HashMap<uuid::Uuid, u32>,
    last_cleanup: DateTime<Utc>,
}

impl Default for PromotionHistory {
    fn default() -> Self {
        Self {
            recent_promotions: HashMap::new(),
            daily_counts: HashMap::new(),
            last_cleanup: Utc::now(),
        }
    }
}

impl PromotionHistory {
    fn cleanup_old_entries(&mut self) {
        let now = Utc::now();
        let cutoff = now - Duration::hours(48);

        // Очищаем старые promotion записи
        self.recent_promotions.retain(|_, timestamps| {
            timestamps.retain(|&ts| ts > cutoff);
            !timestamps.is_empty()
        });

        // Сбрасываем daily counts если прошел день
        if (now - self.last_cleanup).num_hours() >= 24 {
            self.daily_counts.clear();
            self.last_cleanup = now;
        }
    }
}

#[async_trait]
impl PromotionRulesEngine for ConfigurableRulesEngine {
    async fn can_promote(&self, record: &Record) -> bool {
        debug!("🔍 Проверка возможности promotion для записи {}", record.id);

        // 1. Базовые проверки
        if !self.check_basic_requirements(record) {
            debug!("❌ Базовые требования не выполнены");
            return false;
        }

        // 2. Проверка layer strategy
        if !self.check_layer_strategy(record) {
            debug!("❌ Layer strategy не выполнена");
            return false;
        }

        // 3. Проверка promotion history
        if !self.check_promotion_history(record) {
            debug!("❌ Promotion history ограничения");
            return false;
        }

        // 4. Проверка business rules
        if !self.check_business_rules(record).await {
            debug!("❌ Business rules не выполнены");
            return false;
        }

        // 5. Проверка времени и нагрузки системы
        if !self.check_system_conditions().await {
            debug!("❌ Системные условия не подходят");
            return false;
        }

        debug!("✅ Запись {} может быть promoted", record.id);
        true
    }

    fn determine_target_layer(&self, record: &Record, confidence: f32) -> Layer {
        let adjusted_confidence = self.adjust_confidence_for_time_window(confidence);

        match record.layer {
            Layer::Interact => {
                if adjusted_confidence > 0.9 {
                    // Высокий confidence - можно сразу в Assets
                    Layer::Assets
                } else if adjusted_confidence > 0.7 {
                    // Средний confidence - в Insights
                    Layer::Insights
                } else {
                    // Низкий confidence - остается в Interact
                    Layer::Interact
                }
            }
            Layer::Insights => {
                if adjusted_confidence > 0.8 {
                    Layer::Assets
                } else {
                    Layer::Insights
                }
            }
            Layer::Assets => Layer::Assets, // Уже на верхнем уровне
        }
    }

    async fn filter_candidates(&self, candidates: Vec<Record>) -> Result<Vec<Record>> {
        let total_candidates = candidates.len();
        info!(
            "🔬 Фильтрация {} кандидатов по business rules",
            total_candidates
        );

        let mut filtered = Vec::new();
        let mut processed = 0;

        for record in candidates {
            processed += 1;

            if self.can_promote(&record).await {
                filtered.push(record);
            }

            // Периодически логируем прогресс
            if processed % 100 == 0 {
                debug!("Обработано {}/{} кандидатов", processed, total_candidates);
            }
        }

        info!(
            "✅ Отфильтровано {}/{} кандидатов",
            filtered.len(),
            total_candidates
        );
        Ok(filtered)
    }

    fn validate_promotion(&self, decision: &PromotionDecision) -> bool {
        debug!(
            "✅ Валидация promotion decision для записи {}",
            decision.record_id
        );

        // 1. Проверяем что target layer логичен
        if !self.is_valid_layer_transition(decision.current_layer, decision.target_layer) {
            warn!(
                "❌ Недопустимый переход {:?} → {:?}",
                decision.current_layer, decision.target_layer
            );
            return false;
        }

        // 2. Проверяем confidence score
        if decision.confidence < self.get_min_confidence_for_layer(decision.current_layer) {
            warn!(
                "❌ Недостаточный confidence: {} < {}",
                decision.confidence,
                self.get_min_confidence_for_layer(decision.current_layer)
            );
            return false;
        }

        // 3. Strict validation если включено
        if self.config.strict_validation {
            if !self.strict_validation_checks(decision) {
                warn!("❌ Strict validation failed");
                return false;
            }
        }

        debug!("✅ Promotion decision валидна");
        true
    }
}

impl ConfigurableRulesEngine {
    pub fn new(config: RulesConfig) -> Self {
        info!("🎯 Инициализация ConfigurableRulesEngine");
        info!("  - Strict validation: {}", config.strict_validation);
        info!(
            "  - Max system load: {:.1}%",
            config.max_system_load_for_promotion * 100.0
        );
        info!(
            "  - Min repromotion interval: {}h",
            config.min_repromotion_interval_hours
        );

        let mut layer_strategies = HashMap::new();

        // Настройка стратегий для каждого layer
        layer_strategies.insert(
            Layer::Interact,
            LayerStrategy {
                min_access_count: 2,
                min_confidence_score: 0.6,
                max_age_hours: Some(168), // 1 неделя
                priority: 1,
                special_conditions: vec![
                    SpecialCondition::MinTextLength(10),
                    SpecialCondition::TimeOfDayRestriction {
                        start_hour: 6,
                        end_hour: 22,
                    },
                ],
            },
        );

        layer_strategies.insert(
            Layer::Insights,
            LayerStrategy {
                min_access_count: 5,
                min_confidence_score: 0.75,
                max_age_hours: Some(720), // 1 месяц
                priority: 2,
                special_conditions: vec![
                    SpecialCondition::MinTextLength(20),
                    SpecialCondition::MinSemanticImportance(0.7),
                ],
            },
        );

        layer_strategies.insert(
            Layer::Assets,
            LayerStrategy {
                min_access_count: 10,
                min_confidence_score: 0.9,
                max_age_hours: None, // Без ограничений
                priority: 3,
                special_conditions: vec![
                    SpecialCondition::MinTextLength(50),
                    SpecialCondition::MinSemanticImportance(0.8),
                ],
            },
        );

        let business_rules = vec![
            BusinessRule::MaxPromotionsPerHour(100),
            BusinessRule::LayerBalance {
                min_interact_ratio: 0.6,
                max_assets_ratio: 0.1,
            },
            BusinessRule::NoDuplicateContent,
            BusinessRule::UserAccessPatterns,
        ];

        Self {
            config,
            layer_strategies,
            business_rules,
            promotion_history: PromotionHistory::default(),
        }
    }

    pub fn with_custom_strategies(mut self, strategies: HashMap<Layer, LayerStrategy>) -> Self {
        self.layer_strategies = strategies;
        self
    }

    pub fn add_business_rule(&mut self, rule: BusinessRule) {
        self.business_rules.push(rule);
        info!("➕ Добавлено новое business rule");
    }

    fn check_basic_requirements(&self, record: &Record) -> bool {
        // Проверка минимального времени в текущем layer
        let age_in_layer = Utc::now().signed_duration_since(record.ts);
        if age_in_layer.num_hours() < self.config.min_layer_residence_time_hours as i64 {
            debug!(
                "Запись {} слишком новая в layer: {}h < {}h",
                record.id,
                age_in_layer.num_hours(),
                self.config.min_layer_residence_time_hours
            );
            return false;
        }

        true
    }

    fn check_layer_strategy(&self, record: &Record) -> bool {
        let strategy = match self.layer_strategies.get(&record.layer) {
            Some(s) => s,
            None => {
                warn!("Нет стратегии для layer {:?}", record.layer);
                return false;
            }
        };

        // Проверка access_count
        if record.access_count < strategy.min_access_count {
            debug!(
                "Недостаточно accesses: {} < {}",
                record.access_count, strategy.min_access_count
            );
            return false;
        }

        // Проверка возраста
        if let Some(max_age) = strategy.max_age_hours {
            let age = Utc::now().signed_duration_since(record.ts);
            if age.num_hours() > max_age as i64 {
                debug!("Запись слишком старая: {}h > {}h", age.num_hours(), max_age);
                return false;
            }
        }

        // Проверка специальных условий
        self.check_special_conditions(record, &strategy.special_conditions)
    }

    fn check_special_conditions(&self, record: &Record, conditions: &[SpecialCondition]) -> bool {
        for condition in conditions {
            match condition {
                SpecialCondition::MinTextLength(min_len) => {
                    if record.text.len() < *min_len {
                        debug!(
                            "Текст слишком короткий: {} < {}",
                            record.text.len(),
                            min_len
                        );
                        return false;
                    }
                }
                SpecialCondition::RequiredKeywords(keywords) => {
                    let text_lower = record.text.to_lowercase();
                    let has_keyword = keywords
                        .iter()
                        .any(|kw| text_lower.contains(&kw.to_lowercase()));
                    if !has_keyword {
                        debug!("Отсутствуют обязательные keywords");
                        return false;
                    }
                }
                SpecialCondition::BlacklistKeywords(keywords) => {
                    let text_lower = record.text.to_lowercase();
                    let has_blacklisted = keywords
                        .iter()
                        .any(|kw| text_lower.contains(&kw.to_lowercase()));
                    if has_blacklisted {
                        debug!("Содержит запрещенные keywords");
                        return false;
                    }
                }
                SpecialCondition::MinSemanticImportance(min_importance) => {
                    // Заглушка для семантической важности
                    let semantic_score = self.calculate_semantic_importance(&record.text);
                    if semantic_score < *min_importance {
                        debug!(
                            "Недостаточная семантическая важность: {} < {}",
                            semantic_score, min_importance
                        );
                        return false;
                    }
                }
                SpecialCondition::TimeOfDayRestriction {
                    start_hour,
                    end_hour,
                } => {
                    let current_hour = Utc::now().time().hour() as u8;
                    if current_hour < *start_hour || current_hour > *end_hour {
                        debug!(
                            "Вне разрешенного времени: {}h не в [{}h, {}h]",
                            current_hour, start_hour, end_hour
                        );
                        return false;
                    }
                }
            }
        }
        true
    }

    fn check_promotion_history(&self, record: &Record) -> bool {
        // Проверка интервала между re-promotion
        if let Some(timestamps) = self.promotion_history.recent_promotions.get(&record.id) {
            let min_interval = Duration::hours(self.config.min_repromotion_interval_hours as i64);
            let now = Utc::now();

            if let Some(&last_promotion) = timestamps.last() {
                if now - last_promotion < min_interval {
                    debug!("Слишком рано для re-promotion записи {}", record.id);
                    return false;
                }
            }
        }

        // Проверка дневного лимита
        if let Some(&daily_count) = self.promotion_history.daily_counts.get(&record.id) {
            if daily_count >= self.config.max_promotions_per_record_per_day {
                debug!("Достигнут дневной лимит promotion для записи {}", record.id);
                return false;
            }
        }

        true
    }

    async fn check_business_rules(&self, record: &Record) -> bool {
        for rule in &self.business_rules {
            let rule_passed = match rule {
                BusinessRule::MaxPromotionsPerHour(_max_per_hour) => {
                    // Заглушка для проверки rate limiting
                    true
                }
                BusinessRule::LayerBalance {
                    min_interact_ratio: _,
                    max_assets_ratio: _,
                } => {
                    // Заглушка для проверки баланса между layers
                    true
                }
                BusinessRule::NoDuplicateContent => {
                    // Заглушка для проверки дублирования контента
                    !self.is_duplicate_content(&record.text)
                }
                BusinessRule::UserAccessPatterns => {
                    // Заглушка для анализа пользовательских паттернов
                    true
                }
            };

            if !rule_passed {
                return false;
            }
        }

        true
    }

    async fn check_system_conditions(&self) -> bool {
        // Заглушка для проверки нагрузки системы
        let current_system_load = self.get_current_system_load().await;

        if current_system_load > self.config.max_system_load_for_promotion {
            debug!(
                "Системная нагрузка слишком высока: {:.2} > {:.2}",
                current_system_load, self.config.max_system_load_for_promotion
            );
            return false;
        }

        true
    }

    fn adjust_confidence_for_time_window(&self, confidence: f32) -> f32 {
        let current_hour = Utc::now().time().hour() as u8;
        let time_config = &self.config.time_windows;

        if current_hour >= time_config.aggressive_start_hour
            && current_hour <= time_config.aggressive_end_hour
        {
            // Aggressive period - понижаем threshold
            confidence * time_config.aggressive_threshold_multiplier
        } else {
            // Conservative period - повышаем threshold
            confidence * time_config.conservative_threshold_multiplier
        }
    }

    fn is_valid_layer_transition(&self, from: Layer, to: Layer) -> bool {
        match (from, to) {
            (Layer::Interact, Layer::Insights) => true,
            (Layer::Interact, Layer::Assets) => true, // Skip level promotion allowed
            (Layer::Insights, Layer::Assets) => true,
            (layer, target) if layer == target => true, // Stay in same layer
            _ => false,                                 // Backwards promotion not allowed
        }
    }

    fn get_min_confidence_for_layer(&self, layer: Layer) -> f32 {
        self.layer_strategies
            .get(&layer)
            .map(|s| s.min_confidence_score)
            .unwrap_or(0.5)
    }

    fn strict_validation_checks(&self, _decision: &PromotionDecision) -> bool {
        // Заглушка для строгих проверок
        true
    }

    fn calculate_semantic_importance(&self, text: &str) -> f32 {
        // Простая заглушка - в реальности здесь был бы ML анализ
        let word_count = text.split_whitespace().count();
        (word_count as f32 / 100.0).min(1.0)
    }

    fn is_duplicate_content(&self, _text: &str) -> bool {
        // Заглушка для проверки дублирования
        false
    }

    async fn get_current_system_load(&self) -> f32 {
        // Заглушка для получения нагрузки системы
        0.3 // 30% load
    }

    /// Записывает успешную promotion в историю
    pub fn record_promotion(&mut self, record_id: uuid::Uuid) {
        let now = Utc::now();

        // Очищаем старые записи
        self.promotion_history.cleanup_old_entries();

        // Добавляем новую promotion
        self.promotion_history
            .recent_promotions
            .entry(record_id)
            .or_insert_with(Vec::new)
            .push(now);

        // Увеличиваем дневной счетчик
        *self
            .promotion_history
            .daily_counts
            .entry(record_id)
            .or_insert(0) += 1;

        debug!("📝 Записана promotion для {}", record_id);
    }

    /// Получает статистику promotion rules
    pub fn get_rules_statistics(&self) -> RulesStatistics {
        RulesStatistics {
            total_records_in_history: self.promotion_history.recent_promotions.len(),
            business_rules_count: self.business_rules.len(),
            layer_strategies_count: self.layer_strategies.len(),
            last_cleanup: self.promotion_history.last_cleanup,
        }
    }
}

/// Статистика работы rules engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesStatistics {
    pub total_records_in_history: usize,
    pub business_rules_count: usize,
    pub layer_strategies_count: usize,
    pub last_cleanup: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn create_test_record(layer: Layer, access_count: u32, text: &str) -> Record {
        Record {
            id: Uuid::new_v4(),
            text: text.to_string(),
            embedding: vec![0.1; 384],
            ts: Utc::now() - Duration::hours(2), // 2 hours old
            layer,
            access_count,
            score: 0.0,
            kind: "test".to_string(),
            tags: vec!["test".to_string()],
            project: "test_project".to_string(),
            session: "test_session".to_string(),
            last_access: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_basic_promotion_rules() {
        let config = RulesConfig::development();
        let rules_engine = ConfigurableRulesEngine::new(config);

        let record = create_test_record(
            Layer::Interact,
            5,
            "This is a test record with sufficient text",
        );

        let can_promote = rules_engine.can_promote(&record).await;
        assert!(can_promote);
    }

    #[tokio::test]
    async fn test_insufficient_access_count() {
        let config = RulesConfig::default();
        let rules_engine = ConfigurableRulesEngine::new(config);

        let record = create_test_record(Layer::Interact, 1, "Short text"); // Insufficient access count

        let can_promote = rules_engine.can_promote(&record).await;
        assert!(!can_promote);
    }

    #[test]
    fn test_target_layer_determination() {
        let config = RulesConfig::default();
        let rules_engine = ConfigurableRulesEngine::new(config);

        let record = create_test_record(Layer::Interact, 5, "Test record");

        let target_high_confidence = rules_engine.determine_target_layer(&record, 0.95);
        assert_eq!(target_high_confidence, Layer::Assets);

        let target_medium_confidence = rules_engine.determine_target_layer(&record, 0.75);
        assert_eq!(target_medium_confidence, Layer::Insights);
    }

    #[tokio::test]
    async fn test_filter_candidates() {
        let config = RulesConfig::development();
        let rules_engine = ConfigurableRulesEngine::new(config);

        let candidates = vec![
            create_test_record(Layer::Interact, 5, "Good record with enough text"),
            create_test_record(Layer::Interact, 1, "Bad"), // Too few accesses
            create_test_record(
                Layer::Interact,
                3,
                "Another good record with sufficient content",
            ),
        ];

        let filtered = rules_engine.filter_candidates(candidates).await.unwrap();
        assert_eq!(filtered.len(), 2); // Should filter out the record with too few accesses
    }
}
