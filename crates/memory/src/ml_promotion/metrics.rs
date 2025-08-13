use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tracing::{debug, info};

use super::traits::PromotionMetrics;
use super::types::MLPromotionStats;
use common::MemoryError;

/// Реализация системы метрик для ML promotion
#[derive(Debug, Clone)]
pub struct MLPromotionMetricsCollector {
    stats: Arc<Mutex<InternalStats>>,
    config: MetricsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Размер sliding window для метрик
    pub window_size: usize,
    /// Интервал агрегации метрик (в секундах)
    pub aggregation_interval_secs: u64,
    /// Максимальное количество сохраняемых исторических точек
    pub max_history_points: usize,
    /// Включить детальное логирование
    pub detailed_logging: bool,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            window_size: 100,
            aggregation_interval_secs: 60,
            max_history_points: 1000,
            detailed_logging: false,
        }
    }
}

impl MetricsConfig {
    pub fn production() -> Self {
        Self {
            window_size: 500,
            aggregation_interval_secs: 30,
            max_history_points: 5000,
            detailed_logging: false,
        }
    }

    pub fn debug() -> Self {
        Self {
            window_size: 50,
            aggregation_interval_secs: 10,
            max_history_points: 100,
            detailed_logging: true,
        }
    }
}

/// Внутренняя структура для хранения метрик
#[derive(Debug)]
struct InternalStats {
    // Inference metrics
    inference_times: VecDeque<u64>,
    accuracy_scores: VecDeque<f32>,
    total_inferences: usize,

    // Feature extraction metrics
    feature_extraction_times: VecDeque<u64>,
    total_extractions: usize,

    // Cache metrics
    cache_hit_rates: VecDeque<f32>,
    cache_hits: usize,
    cache_requests: usize,

    // GPU metrics
    gpu_utilization_samples: VecDeque<f32>,
    gpu_memory_usage: VecDeque<f32>,

    // Promotion metrics
    total_analyzed: usize,
    promoted_interact_to_insights: usize,
    promoted_insights_to_assets: usize,

    // Temporal metrics
    start_time: DateTime<Utc>,
    last_reset: DateTime<Utc>,

    // Historical data
    historical_snapshots: Vec<HistoricalSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalSnapshot {
    timestamp: DateTime<Utc>,
    avg_inference_time: f32,
    avg_accuracy: f32,
    cache_hit_rate: f32,
    gpu_utilization: f32,
    promotion_rate: f32,
}

impl Default for InternalStats {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            inference_times: VecDeque::new(),
            accuracy_scores: VecDeque::new(),
            total_inferences: 0,
            feature_extraction_times: VecDeque::new(),
            total_extractions: 0,
            cache_hit_rates: VecDeque::new(),
            cache_hits: 0,
            cache_requests: 0,
            gpu_utilization_samples: VecDeque::new(),
            gpu_memory_usage: VecDeque::new(),
            total_analyzed: 0,
            promoted_interact_to_insights: 0,
            promoted_insights_to_assets: 0,
            start_time: now,
            last_reset: now,
            historical_snapshots: Vec::new(),
        }
    }
}

#[async_trait]
impl PromotionMetrics for MLPromotionMetricsCollector {
    fn record_inference(&mut self, inference_time_ms: u64, accuracy: f32) {
        let mut stats = match self.safe_lock() {
            Ok(guard) => guard,
            Err(e) => {
                tracing::error!("Failed to acquire metrics lock: {}", e);
                return;
            }
        };

        // Добавляем новые данные
        stats.inference_times.push_back(inference_time_ms);
        stats.accuracy_scores.push_back(accuracy);
        stats.total_inferences += 1;

        // Поддерживаем размер sliding window
        if stats.inference_times.len() > self.config.window_size {
            stats.inference_times.pop_front();
            stats.accuracy_scores.pop_front();
        }

        if self.config.detailed_logging {
            debug!(
                "📊 Inference recorded: {}ms, accuracy: {:.2}%",
                inference_time_ms,
                accuracy * 100.0
            );
        }
    }

    fn record_feature_extraction(&mut self, extraction_time_ms: u64) {
        let mut stats = match self.safe_lock() {
            Ok(guard) => guard,
            Err(e) => {
                tracing::error!(
                    "Failed to acquire metrics lock for feature extraction: {}",
                    e
                );
                return;
            }
        };

        stats.feature_extraction_times.push_back(extraction_time_ms);
        stats.total_extractions += 1;

        if stats.feature_extraction_times.len() > self.config.window_size {
            stats.feature_extraction_times.pop_front();
        }

        if self.config.detailed_logging {
            debug!("🔬 Feature extraction recorded: {}ms", extraction_time_ms);
        }
    }

    fn update_cache_stats(&mut self, hit_rate: f32) {
        let mut stats = match self.safe_lock() {
            Ok(guard) => guard,
            Err(e) => {
                tracing::error!("Failed to acquire metrics lock for cache stats: {}", e);
                return;
            }
        };

        stats.cache_hit_rates.push_back(hit_rate);
        stats.cache_requests += 1;

        // Приблизительный расчет hits
        stats.cache_hits += (hit_rate * 100.0) as usize;

        if stats.cache_hit_rates.len() > self.config.window_size {
            stats.cache_hit_rates.pop_front();
        }

        if self.config.detailed_logging {
            debug!("💾 Cache stats updated: hit rate {:.1}%", hit_rate * 100.0);
        }
    }

    fn update_gpu_stats(&mut self, utilization: f32) {
        let mut stats = self.stats.lock().expect("Lock should not be poisoned");

        stats.gpu_utilization_samples.push_back(utilization);

        if stats.gpu_utilization_samples.len() > self.config.window_size {
            stats.gpu_utilization_samples.pop_front();
        }

        if self.config.detailed_logging {
            debug!("🚀 GPU utilization updated: {:.1}%", utilization * 100.0);
        }
    }

    fn get_stats(&self) -> MLPromotionStats {
        let stats = self.stats.lock().expect("Lock should not be poisoned");

        // Вычисляем агрегированные метрики
        let avg_inference_time = if stats.inference_times.is_empty() {
            0
        } else {
            stats.inference_times.iter().sum::<u64>() / stats.inference_times.len() as u64
        };

        let avg_feature_extraction_time = if stats.feature_extraction_times.is_empty() {
            0
        } else {
            stats.feature_extraction_times.iter().sum::<u64>()
                / stats.feature_extraction_times.len() as u64
        };

        let model_accuracy = if stats.accuracy_scores.is_empty() {
            0.0
        } else {
            stats.accuracy_scores.iter().sum::<f32>() / stats.accuracy_scores.len() as f32
        };

        let cache_hit_rate = if stats.cache_hit_rates.is_empty() {
            0.0
        } else {
            stats.cache_hit_rates.iter().sum::<f32>() / stats.cache_hit_rates.len() as f32
        };

        let gpu_utilization = if stats.gpu_utilization_samples.is_empty() {
            0.0
        } else {
            stats.gpu_utilization_samples.iter().sum::<f32>()
                / stats.gpu_utilization_samples.len() as f32
        };

        // Вычисляем общее время работы
        let total_processing_time = (Utc::now() - stats.start_time).num_milliseconds() as f64;

        MLPromotionStats {
            total_analyzed: stats.total_analyzed,
            promoted_interact_to_insights: stats.promoted_interact_to_insights,
            promoted_insights_to_assets: stats.promoted_insights_to_assets,
            ml_inference_time_ms: avg_inference_time,
            feature_extraction_time_ms: avg_feature_extraction_time,
            model_accuracy,
            avg_confidence_score: model_accuracy, // Используем accuracy как proxy
            cache_hit_rate,
            gpu_utilization,
            analyzed_records: stats.total_analyzed,
            promoted_records: stats.promoted_interact_to_insights
                + stats.promoted_insights_to_assets,
            processing_time_ms: total_processing_time,
            algorithm_used: "ml".to_string(),
        }
    }

    fn reset_metrics(&mut self) {
        // Сначала создаем копию данных для снимка
        let snapshot_data = {
            let stats = self.stats.lock().expect("Lock should not be poisoned");
            // Копируем данные вместо передачи ссылки
            InternalStats {
                inference_times: stats.inference_times.clone(),
                accuracy_scores: stats.accuracy_scores.clone(),
                feature_extraction_times: stats.feature_extraction_times.clone(),
                cache_hit_rates: stats.cache_hit_rates.clone(),
                gpu_utilization_samples: stats.gpu_utilization_samples.clone(),
                gpu_memory_usage: stats.gpu_memory_usage.clone(),
                ..Default::default()
            }
        };

        // Сохраняем снимок в историю перед сбросом
        self.take_historical_snapshot(&snapshot_data);

        // Теперь безопасно сбрасываем все метрики
        {
            let mut stats = self.stats.lock().expect("Lock should not be poisoned");
            stats.inference_times.clear();
            stats.accuracy_scores.clear();
            stats.feature_extraction_times.clear();
            stats.cache_hit_rates.clear();
            stats.gpu_utilization_samples.clear();
            stats.gpu_memory_usage.clear();

            stats.total_inferences = 0;
            stats.total_extractions = 0;
            stats.cache_hits = 0;
            stats.cache_requests = 0;
            stats.total_analyzed = 0;
            stats.promoted_interact_to_insights = 0;
            stats.promoted_insights_to_assets = 0;

            stats.last_reset = Utc::now();
        }

        info!("🔄 ML promotion metrics reset");
    }
}

impl MLPromotionMetricsCollector {
    pub fn new(config: MetricsConfig) -> Self {
        info!("📊 Инициализация ML promotion metrics collector");
        info!("  - Window size: {}", config.window_size);
        info!(
            "  - Aggregation interval: {}s",
            config.aggregation_interval_secs
        );
        info!("  - Detailed logging: {}", config.detailed_logging);

        Self {
            stats: Arc::new(Mutex::new(InternalStats::default())),
            config,
        }
    }

    /// Safe mutex lock with error handling
    fn safe_lock(
        &self,
    ) -> std::result::Result<std::sync::MutexGuard<'_, InternalStats>, MemoryError> {
        self.stats.lock().map_err(|_| MemoryError::Promotion {
            reason: "ML metrics mutex poisoned".to_string(),
        })
    }

    /// Записывает успешную promotion
    pub fn record_promotion(
        &mut self,
        from_layer: crate::types::Layer,
        to_layer: crate::types::Layer,
    ) {
        let mut stats = self.stats.lock().expect("Lock should not be poisoned");
        stats.total_analyzed += 1;

        match (from_layer, to_layer) {
            (crate::types::Layer::Interact, crate::types::Layer::Insights) => {
                stats.promoted_interact_to_insights += 1;
            }
            (crate::types::Layer::Insights, crate::types::Layer::Assets) => {
                stats.promoted_insights_to_assets += 1;
            }
            _ => {} // Другие типы промоции
        }

        if self.config.detailed_logging {
            debug!("⬆️ Promotion recorded: {:?} → {:?}", from_layer, to_layer);
        }
    }

    /// Получает детальные метрики производительности
    pub fn get_performance_breakdown(&self) -> PerformanceBreakdown {
        let stats = self.stats.lock().expect("Lock should not be poisoned");

        PerformanceBreakdown {
            inference_p50: self.calculate_percentile(&stats.inference_times, 0.5),
            inference_p90: self.calculate_percentile(&stats.inference_times, 0.9),
            inference_p99: self.calculate_percentile(&stats.inference_times, 0.99),
            feature_extraction_avg: if stats.feature_extraction_times.is_empty() {
                0.0
            } else {
                stats.feature_extraction_times.iter().sum::<u64>() as f32
                    / stats.feature_extraction_times.len() as f32
            },
            accuracy_trend: self.calculate_trend(&stats.accuracy_scores),
            cache_efficiency: if stats.cache_requests > 0 {
                stats.cache_hits as f32 / stats.cache_requests as f32
            } else {
                0.0
            },
            uptime_hours: (Utc::now() - stats.start_time).num_hours() as f32,
        }
    }

    /// Получает исторические данные
    pub fn get_historical_data(&self) -> Vec<HistoricalSnapshot> {
        let stats = self.stats.lock().expect("Lock should not be poisoned");
        stats.historical_snapshots.clone()
    }

    fn calculate_percentile(&self, values: &VecDeque<u64>, percentile: f32) -> f32 {
        if values.is_empty() {
            return 0.0;
        }

        let mut sorted: Vec<u64> = values.iter().cloned().collect();
        sorted.sort();

        let index = ((sorted.len() - 1) as f32 * percentile) as usize;
        sorted[index] as f32
    }

    fn calculate_trend(&self, values: &VecDeque<f32>) -> f32 {
        if values.len() < 2 {
            return 0.0;
        }

        // Простой расчет тренда: сравниваем первую и вторую половину
        let mid = values.len() / 2;
        let first_half_avg: f32 = values.iter().take(mid).sum::<f32>() / mid as f32;
        let second_half_avg: f32 =
            values.iter().skip(mid).sum::<f32>() / (values.len() - mid) as f32;

        second_half_avg - first_half_avg
    }

    fn take_historical_snapshot(&mut self, stats: &InternalStats) {
        let snapshot = HistoricalSnapshot {
            timestamp: Utc::now(),
            avg_inference_time: if stats.inference_times.is_empty() {
                0.0
            } else {
                stats.inference_times.iter().sum::<u64>() as f32
                    / stats.inference_times.len() as f32
            },
            avg_accuracy: if stats.accuracy_scores.is_empty() {
                0.0
            } else {
                stats.accuracy_scores.iter().sum::<f32>() / stats.accuracy_scores.len() as f32
            },
            cache_hit_rate: if stats.cache_hit_rates.is_empty() {
                0.0
            } else {
                stats.cache_hit_rates.iter().sum::<f32>() / stats.cache_hit_rates.len() as f32
            },
            gpu_utilization: if stats.gpu_utilization_samples.is_empty() {
                0.0
            } else {
                stats.gpu_utilization_samples.iter().sum::<f32>()
                    / stats.gpu_utilization_samples.len() as f32
            },
            promotion_rate: if stats.total_analyzed > 0 {
                (stats.promoted_interact_to_insights + stats.promoted_insights_to_assets) as f32
                    / stats.total_analyzed as f32
            } else {
                0.0
            },
        };

        // Добавляем в историю и поддерживаем максимальный размер
        let mut stats_mut = self.stats.lock().expect("Lock should not be poisoned");
        stats_mut.historical_snapshots.push(snapshot);

        if stats_mut.historical_snapshots.len() > self.config.max_history_points {
            stats_mut.historical_snapshots.remove(0);
        }
    }

    /// Экспортирует метрики в JSON формат
    pub fn export_metrics(&self) -> serde_json::Value {
        let stats = self.stats.lock().expect("Lock should not be poisoned");
        let current_stats = self.get_stats();
        let performance = self.get_performance_breakdown();

        serde_json::json!({
            "current_stats": current_stats,
            "performance_breakdown": {
                "inference_p50": performance.inference_p50,
                "inference_p90": performance.inference_p90,
                "inference_p99": performance.inference_p99,
                "feature_extraction_avg": performance.feature_extraction_avg,
                "accuracy_trend": performance.accuracy_trend,
                "cache_efficiency": performance.cache_efficiency,
                "uptime_hours": performance.uptime_hours,
            },
            "historical_data": stats.historical_snapshots,
            "collection_config": self.config,
        })
    }
}

/// Детальная разбивка производительности
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceBreakdown {
    pub inference_p50: f32,
    pub inference_p90: f32,
    pub inference_p99: f32,
    pub feature_extraction_avg: f32,
    pub accuracy_trend: f32,
    pub cache_efficiency: f32,
    pub uptime_hours: f32,
}

/// Реализация thread-safe клона для использования в многопоточной среде
impl Clone for InternalStats {
    fn clone(&self) -> Self {
        Self {
            inference_times: self.inference_times.clone(),
            accuracy_scores: self.accuracy_scores.clone(),
            total_inferences: self.total_inferences,
            feature_extraction_times: self.feature_extraction_times.clone(),
            total_extractions: self.total_extractions,
            cache_hit_rates: self.cache_hit_rates.clone(),
            cache_hits: self.cache_hits,
            cache_requests: self.cache_requests,
            gpu_utilization_samples: self.gpu_utilization_samples.clone(),
            gpu_memory_usage: self.gpu_memory_usage.clone(),
            total_analyzed: self.total_analyzed,
            promoted_interact_to_insights: self.promoted_interact_to_insights,
            promoted_insights_to_assets: self.promoted_insights_to_assets,
            start_time: self.start_time,
            last_reset: self.last_reset,
            historical_snapshots: self.historical_snapshots.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector_creation() {
        let config = MetricsConfig::default();
        let collector = MLPromotionMetricsCollector::new(config);
        let stats = collector.get_stats();

        assert_eq!(stats.total_analyzed, 0);
        assert_eq!(stats.ml_inference_time_ms, 0);
    }

    #[test]
    fn test_inference_recording() {
        let config = MetricsConfig::default();
        let mut collector = MLPromotionMetricsCollector::new(config);

        collector.record_inference(100, 0.85);
        collector.record_inference(120, 0.90);

        let stats = collector.get_stats();
        assert_eq!(stats.ml_inference_time_ms, 110); // Average
        assert_eq!(stats.model_accuracy, 0.875); // Average accuracy
    }

    #[test]
    fn test_performance_breakdown() {
        let config = MetricsConfig::debug();
        let mut collector = MLPromotionMetricsCollector::new(config);

        // Добавляем тестовые данные
        for i in 0..10 {
            collector.record_inference(100 + i * 10, 0.8 + i as f32 * 0.01);
        }

        let breakdown = collector.get_performance_breakdown();
        assert!(breakdown.inference_p50 > 0.0);
        assert!(breakdown.accuracy_trend > 0.0); // Должен показывать положительный тренд
    }
}
