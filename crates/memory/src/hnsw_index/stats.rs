use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Профессиональные метрики производительности HNSW индекса
/// Предоставляет детальную статистику для мониторинга и оптимизации
#[derive(Debug, Default)]
pub struct HnswStats {
    // Счетчики операций
    pub total_vectors: AtomicU64,
    pub total_searches: AtomicU64, 
    pub total_insertions: AtomicU64,
    pub total_removals: AtomicU64,
    
    // Временные метрики (в микросекундах)
    pub total_search_time_us: AtomicU64,
    pub total_insert_time_us: AtomicU64,
    pub total_parallel_insert_time_us: AtomicU64,
    
    // Метрики качества
    pub total_distance_calculations: AtomicU64,
    pub parallel_batches_processed: AtomicU64,
    pub failed_operations: AtomicU64,
    
    // Timestamp последней операции
    pub last_operation_timestamp: AtomicU64,
}

/// Моментальный снимок статистики для сериализации и отчетов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswStatsSnapshot {
    pub total_vectors: u64,
    pub total_searches: u64,
    pub total_insertions: u64,
    pub total_removals: u64,
    pub avg_search_time_ms: f64,
    pub avg_insert_time_ms: f64,
    pub search_throughput_per_sec: f64,
    pub insert_throughput_per_sec: f64,
    pub memory_usage_estimate_mb: f64,
    pub distance_calculations_per_search: f64,
    pub parallel_efficiency: f64,
    pub error_rate: f64,
    pub uptime_seconds: f64,
}

impl HnswStats {
    /// Создать новый экземпляр статистики
    pub fn new() -> Self {
        Self {
            last_operation_timestamp: AtomicU64::new(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            ),
            ..Default::default()
        }
    }
    
    /// Записать результат поиска
    pub fn record_search(&self, duration: Duration, distance_calcs: u64) {
        let duration_us = duration.as_micros() as u64;
        
        self.total_searches.fetch_add(1, Ordering::Relaxed);
        self.total_search_time_us.fetch_add(duration_us, Ordering::Relaxed);
        self.total_distance_calculations.fetch_add(distance_calcs, Ordering::Relaxed);
        self.update_timestamp();
    }
    
    /// Записать результат вставки
    pub fn record_insertion(&self, count: u64, duration: Duration, is_parallel: bool) {
        let duration_us = duration.as_micros() as u64;
        
        self.total_insertions.fetch_add(count, Ordering::Relaxed);
        self.total_vectors.fetch_add(count, Ordering::Relaxed);
        
        if is_parallel {
            self.total_parallel_insert_time_us.fetch_add(duration_us, Ordering::Relaxed);
            self.parallel_batches_processed.fetch_add(1, Ordering::Relaxed);
        } else {
            self.total_insert_time_us.fetch_add(duration_us, Ordering::Relaxed);
        }
        
        self.update_timestamp();
    }
    
    /// Записать результат удаления
    pub fn record_removal(&self, success: bool) {
        if success {
            self.total_removals.fetch_add(1, Ordering::Relaxed);
            // Не уменьшаем total_vectors так как это может создать race conditions
        } else {
            self.failed_operations.fetch_add(1, Ordering::Relaxed);
        }
        self.update_timestamp();
    }
    
    /// Записать ошибку операции
    pub fn record_error(&self) {
        self.failed_operations.fetch_add(1, Ordering::Relaxed);
        self.update_timestamp();
    }
    
    /// Получить среднее время поиска в микросекундах
    pub fn avg_search_time_us(&self) -> f64 {
        let searches = self.total_searches.load(Ordering::Relaxed);
        if searches == 0 {
            return 0.0;
        }
        
        let total_time = self.total_search_time_us.load(Ordering::Relaxed);
        total_time as f64 / searches as f64
    }
    
    /// Получить среднее время вставки в микросекундах
    pub fn avg_insert_time_us(&self) -> f64 {
        let insertions = self.total_insertions.load(Ordering::Relaxed);
        if insertions == 0 {
            return 0.0;
        }
        
        let total_time = self.total_insert_time_us.load(Ordering::Relaxed) 
                       + self.total_parallel_insert_time_us.load(Ordering::Relaxed);
        total_time as f64 / insertions as f64
    }
    
    /// Получить пропускную способность поиска (запросов в секунду)
    pub fn search_throughput_per_sec(&self) -> f64 {
        let searches = self.total_searches.load(Ordering::Relaxed);
        let total_time_sec = self.total_search_time_us.load(Ordering::Relaxed) as f64 / 1_000_000.0;
        
        if total_time_sec == 0.0 {
            return 0.0;
        }
        
        searches as f64 / total_time_sec
    }
    
    /// Получить пропускную способность вставки (векторов в секунду)
    pub fn insert_throughput_per_sec(&self) -> f64 {
        let insertions = self.total_insertions.load(Ordering::Relaxed);
        let total_time_sec = (self.total_insert_time_us.load(Ordering::Relaxed) 
                           + self.total_parallel_insert_time_us.load(Ordering::Relaxed)) as f64 / 1_000_000.0;
        
        if total_time_sec == 0.0 {
            return 0.0;
        }
        
        insertions as f64 / total_time_sec
    }
    
    /// Получить текущее количество векторов
    pub fn vector_count(&self) -> u64 {
        self.total_vectors.load(Ordering::Relaxed)
    }
    
    /// Получить среднее время вставки в миллисекундах
    pub fn avg_insertion_time_ms(&self) -> f64 {
        self.avg_insert_time_us() / 1000.0
    }
    
    /// Получить среднее время поиска в миллисекундах
    pub fn avg_search_time_ms(&self) -> f64 {
        self.avg_search_time_us() / 1000.0
    }
    
    /// Примерная оценка использования памяти в KB
    pub fn memory_usage_kb(&self) -> u64 {
        let vectors = self.total_vectors.load(Ordering::Relaxed);
        // Примерная оценка: каждый вектор занимает ~4KB 
        // (1024 dimensions * 4 bytes + overhead для графа)
        vectors * 4
    }
    
    /// Примерная оценка использования памяти в MB
    pub fn memory_usage_mb(&self) -> f64 {
        self.memory_usage_kb() as f64 / 1024.0
    }
    
    /// Получить среднее количество вычислений расстояний на поиск
    pub fn avg_distance_calculations_per_search(&self) -> f64 {
        let searches = self.total_searches.load(Ordering::Relaxed);
        if searches == 0 {
            return 0.0;
        }
        
        let total_calcs = self.total_distance_calculations.load(Ordering::Relaxed);
        total_calcs as f64 / searches as f64
    }
    
    /// Получить эффективность параллельной обработки (0.0 - 1.0)
    pub fn parallel_efficiency(&self) -> f64 {
        let parallel_batches = self.parallel_batches_processed.load(Ordering::Relaxed);
        let total_insertions = self.total_insertions.load(Ordering::Relaxed);
        
        if total_insertions == 0 {
            return 0.0;
        }
        
        // Простая метрика: доля операций выполненных параллельно
        (parallel_batches as f64) / (total_insertions as f64).max(1.0)
    }
    
    /// Получить коэффициент ошибок (0.0 - 1.0)
    pub fn error_rate(&self) -> f64 {
        let failed = self.failed_operations.load(Ordering::Relaxed);
        let total_ops = self.total_searches.load(Ordering::Relaxed) 
                      + self.total_insertions.load(Ordering::Relaxed)
                      + self.total_removals.load(Ordering::Relaxed);
        
        if total_ops == 0 {
            return 0.0;
        }
        
        failed as f64 / total_ops as f64
    }
    
    /// Получить время работы в секундах
    pub fn uptime_seconds(&self) -> f64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        let start = self.last_operation_timestamp.load(Ordering::Relaxed);
        (now.saturating_sub(start)) as f64
    }
    
    /// Создать моментальный снимок статистики
    pub fn snapshot(&self) -> HnswStatsSnapshot {
        HnswStatsSnapshot {
            total_vectors: self.vector_count(),
            total_searches: self.total_searches.load(Ordering::Relaxed),
            total_insertions: self.total_insertions.load(Ordering::Relaxed),
            total_removals: self.total_removals.load(Ordering::Relaxed),
            avg_search_time_ms: self.avg_search_time_ms(),
            avg_insert_time_ms: self.avg_insertion_time_ms(),
            search_throughput_per_sec: self.search_throughput_per_sec(),
            insert_throughput_per_sec: self.insert_throughput_per_sec(),
            memory_usage_estimate_mb: self.memory_usage_mb(),
            distance_calculations_per_search: self.avg_distance_calculations_per_search(),
            parallel_efficiency: self.parallel_efficiency(),
            error_rate: self.error_rate(),
            uptime_seconds: self.uptime_seconds(),
        }
    }
    
    /// Сбросить все статистики
    pub fn reset(&self) {
        self.total_vectors.store(0, Ordering::Relaxed);
        self.total_searches.store(0, Ordering::Relaxed);
        self.total_insertions.store(0, Ordering::Relaxed);
        self.total_removals.store(0, Ordering::Relaxed);
        self.total_search_time_us.store(0, Ordering::Relaxed);
        self.total_insert_time_us.store(0, Ordering::Relaxed);
        self.total_parallel_insert_time_us.store(0, Ordering::Relaxed);
        self.total_distance_calculations.store(0, Ordering::Relaxed);
        self.parallel_batches_processed.store(0, Ordering::Relaxed);
        self.failed_operations.store(0, Ordering::Relaxed);
        self.update_timestamp();
    }
    
    /// Обновить timestamp последней операции
    fn update_timestamp(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.last_operation_timestamp.store(now, Ordering::Relaxed);
    }
}

impl HnswStatsSnapshot {
    /// Форматировать статистику для человеко-читаемого вывода
    pub fn format_human_readable(&self) -> String {
        format!(
            "HNSW Index Statistics:
📊 Vectors: {}, Searches: {}, Insertions: {}, Removals: {}
⏱️  Avg Search: {:.2}ms, Avg Insert: {:.2}ms
🚀 Search Throughput: {:.1}/sec, Insert Throughput: {:.1}/sec  
💾 Memory Usage: {:.1}MB
🎯 Avg Distance Calcs/Search: {:.1}
⚡ Parallel Efficiency: {:.1}%, Error Rate: {:.3}%
⏰ Uptime: {:.1}s",
            self.total_vectors,
            self.total_searches, 
            self.total_insertions,
            self.total_removals,
            self.avg_search_time_ms,
            self.avg_insert_time_ms,
            self.search_throughput_per_sec,
            self.insert_throughput_per_sec,
            self.memory_usage_estimate_mb,
            self.distance_calculations_per_search,
            self.parallel_efficiency * 100.0,
            self.error_rate * 100.0,
            self.uptime_seconds
        )
    }
    
    /// Проверить являются ли метрики здоровыми
    pub fn is_healthy(&self) -> bool {
        // Базовые проверки здоровья индекса
        self.error_rate < 0.01 && // менее 1% ошибок
        (self.total_searches == 0 || self.avg_search_time_ms < 1000.0) && // поиск < 1 секунды
        (self.total_insertions == 0 || self.avg_insert_time_ms < 5000.0) // вставка < 5 секунд
    }
    
    /// Получить предупреждения о производительности
    pub fn performance_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        
        if self.error_rate > 0.05 {
            warnings.push(format!("High error rate: {:.1}%", self.error_rate * 100.0));
        }
        
        if self.avg_search_time_ms > 100.0 {
            warnings.push(format!("Slow search performance: {:.1}ms", self.avg_search_time_ms));
        }
        
        if self.avg_insert_time_ms > 1000.0 {
            warnings.push(format!("Slow insert performance: {:.1}ms", self.avg_insert_time_ms));
        }
        
        if self.memory_usage_estimate_mb > 10000.0 {
            warnings.push(format!("High memory usage: {:.1}MB", self.memory_usage_estimate_mb));
        }
        
        if self.parallel_efficiency < 0.5 && self.total_insertions > 100 {
            warnings.push(format!("Low parallel efficiency: {:.1}%", self.parallel_efficiency * 100.0));
        }
        
        warnings
    }
}