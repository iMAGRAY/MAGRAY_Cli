use serde::{Deserialize, Serialize};

/// Конфигурация flush intervals для различных компонентов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlushConfig {
    /// Интервал flush для основного vector storage (ms)
    pub vector_storage_ms: Option<u64>,
    
    /// Интервал flush для embedding cache (ms)
    pub embedding_cache_ms: Option<u64>,
    
    /// Интервал flush для LRU cache (ms)
    pub lru_cache_ms: Option<u64>,
    
    /// Интервал flush для promotion indices (ms)  
    pub promotion_indices_ms: Option<u64>,
    
    /// Интервал flush для migration database (ms)
    pub migration_db_ms: Option<u64>,
    
    /// Режим производительности
    pub performance_mode: PerformanceMode,
    
    /// Включить ли compression для всех баз
    pub enable_compression: bool,
    
    /// Фактор сжатия (1-19, где 19 максимальное сжатие)
    pub compression_factor: i32,
}

/// Режимы производительности с предустановленными настройками
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PerformanceMode {
    /// Максимальная производительность, редкие flush
    HighPerformance,
    
    /// Баланс производительности и надежности
    Balanced,
    
    /// Максимальная надежность, частые flush
    HighReliability,
    
    /// Пользовательские настройки
    Custom,
}

impl Default for FlushConfig {
    fn default() -> Self {
        Self {
            vector_storage_ms: None,
            embedding_cache_ms: None,
            lru_cache_ms: None,
            promotion_indices_ms: None,
            migration_db_ms: None,
            performance_mode: PerformanceMode::Balanced,
            enable_compression: true,
            compression_factor: 19,
        }
    }
}

impl FlushConfig {
    /// Создать конфигурацию для высокой производительности
    pub fn high_performance() -> Self {
        Self {
            vector_storage_ms: Some(5000),     // 5 секунд
            embedding_cache_ms: Some(10000),   // 10 секунд
            lru_cache_ms: Some(8000),          // 8 секунд
            promotion_indices_ms: Some(3000),  // 3 секунды
            migration_db_ms: Some(2000),       // 2 секунды
            performance_mode: PerformanceMode::HighPerformance,
            enable_compression: true,
            compression_factor: 15, // Меньше сжатие для скорости
        }
    }
    
    /// Создать конфигурацию для высокой надежности
    pub fn high_reliability() -> Self {
        Self {
            vector_storage_ms: Some(500),      // 0.5 секунды
            embedding_cache_ms: Some(1000),    // 1 секунда
            lru_cache_ms: Some(800),           // 0.8 секунды
            promotion_indices_ms: Some(300),   // 0.3 секунды
            migration_db_ms: Some(200),        // 0.2 секунды
            performance_mode: PerformanceMode::HighReliability,
            enable_compression: true,
            compression_factor: 19, // Максимальное сжатие
        }
    }
    
    /// Создать сбалансированную конфигурацию
    pub fn balanced() -> Self {
        Self {
            vector_storage_ms: Some(2000),     // 2 секунды
            embedding_cache_ms: Some(3000),    // 3 секунды  
            lru_cache_ms: Some(2500),          // 2.5 секунды
            promotion_indices_ms: Some(1500),  // 1.5 секунды
            migration_db_ms: Some(1000),       // 1 секунда
            performance_mode: PerformanceMode::Balanced,
            enable_compression: true,
            compression_factor: 17, // Средний уровень сжатия
        }
    }
    
    /// Получить значение flush interval для компонента или дефолт
    pub fn get_vector_storage_ms(&self) -> u64 {
        self.vector_storage_ms.unwrap_or(match self.performance_mode {
            PerformanceMode::HighPerformance => 5000,
            PerformanceMode::Balanced => 2000,
            PerformanceMode::HighReliability => 500,
            PerformanceMode::Custom => 2000,
        })
    }
    
    pub fn get_embedding_cache_ms(&self) -> u64 {
        self.embedding_cache_ms.unwrap_or(match self.performance_mode {
            PerformanceMode::HighPerformance => 10000,
            PerformanceMode::Balanced => 3000,
            PerformanceMode::HighReliability => 1000,
            PerformanceMode::Custom => 3000,
        })
    }
    
    pub fn get_lru_cache_ms(&self) -> u64 {
        self.lru_cache_ms.unwrap_or(match self.performance_mode {
            PerformanceMode::HighPerformance => 8000,
            PerformanceMode::Balanced => 2500,
            PerformanceMode::HighReliability => 800,
            PerformanceMode::Custom => 2500,
        })
    }
    
    pub fn get_promotion_indices_ms(&self) -> u64 {
        self.promotion_indices_ms.unwrap_or(match self.performance_mode {
            PerformanceMode::HighPerformance => 3000,
            PerformanceMode::Balanced => 1500,
            PerformanceMode::HighReliability => 300,
            PerformanceMode::Custom => 1500,
        })
    }
    
    pub fn get_migration_db_ms(&self) -> u64 {
        self.migration_db_ms.unwrap_or(match self.performance_mode {
            PerformanceMode::HighPerformance => 2000,
            PerformanceMode::Balanced => 1000,
            PerformanceMode::HighReliability => 200,
            PerformanceMode::Custom => 1000,
        })
    }
    
    /// Получить compression factor в зависимости от режима
    pub fn get_compression_factor(&self) -> i32 {
        if !self.enable_compression {
            return 0;
        }
        
        match self.performance_mode {
            PerformanceMode::HighPerformance => 15,
            PerformanceMode::Balanced => 17,
            PerformanceMode::HighReliability => 19,
            PerformanceMode::Custom => self.compression_factor,
        }
    }
    
    /// Загрузить конфигурацию из переменных окружения
    pub fn from_env() -> Self {
        let mut config = Self::default();
        
        // Проверяем режим производительности
        if let Ok(mode) = std::env::var("MAGRAY_PERFORMANCE_MODE") {
            config.performance_mode = match mode.to_lowercase().as_str() {
                "high_performance" | "fast" => PerformanceMode::HighPerformance,
                "balanced" | "default" => PerformanceMode::Balanced,
                "high_reliability" | "safe" => PerformanceMode::HighReliability,
                "custom" => PerformanceMode::Custom,
                _ => PerformanceMode::Balanced,
            };
        }
        
        // Загружаем индивидуальные настройки
        if let Ok(ms) = std::env::var("MAGRAY_VECTOR_FLUSH_MS") {
            config.vector_storage_ms = ms.parse().ok();
        }
        
        if let Ok(ms) = std::env::var("MAGRAY_CACHE_FLUSH_MS") {
            config.embedding_cache_ms = ms.parse().ok();
        }
        
        if let Ok(ms) = std::env::var("MAGRAY_LRU_FLUSH_MS") {
            config.lru_cache_ms = ms.parse().ok();
        }
        
        if let Ok(ms) = std::env::var("MAGRAY_PROMOTION_FLUSH_MS") {
            config.promotion_indices_ms = ms.parse().ok();
        }
        
        if let Ok(ms) = std::env::var("MAGRAY_MIGRATION_FLUSH_MS") {
            config.migration_db_ms = ms.parse().ok();
        }
        
        if let Ok(enabled) = std::env::var("MAGRAY_COMPRESSION") {
            config.enable_compression = enabled.to_lowercase() == "true" || enabled == "1";
        }
        
        if let Ok(factor) = std::env::var("MAGRAY_COMPRESSION_FACTOR") {
            if let Ok(f) = factor.parse::<i32>() {
                config.compression_factor = f.clamp(1, 19);
            }
        }
        
        config
    }
    
    /// Сохранить конфигурацию в переменные окружения
    pub fn to_env(&self) {
        std::env::set_var("MAGRAY_PERFORMANCE_MODE", match self.performance_mode {
            PerformanceMode::HighPerformance => "high_performance",
            PerformanceMode::Balanced => "balanced", 
            PerformanceMode::HighReliability => "high_reliability",
            PerformanceMode::Custom => "custom",
        });
        
        if let Some(ms) = self.vector_storage_ms {
            std::env::set_var("MAGRAY_VECTOR_FLUSH_MS", ms.to_string());
        }
        
        if let Some(ms) = self.embedding_cache_ms {
            std::env::set_var("MAGRAY_CACHE_FLUSH_MS", ms.to_string());
        }
        
        if let Some(ms) = self.lru_cache_ms {
            std::env::set_var("MAGRAY_LRU_FLUSH_MS", ms.to_string());
        }
        
        if let Some(ms) = self.promotion_indices_ms {
            std::env::set_var("MAGRAY_PROMOTION_FLUSH_MS", ms.to_string());
        }
        
        if let Some(ms) = self.migration_db_ms {
            std::env::set_var("MAGRAY_MIGRATION_FLUSH_MS", ms.to_string());
        }
        
        std::env::set_var("MAGRAY_COMPRESSION", if self.enable_compression { "true" } else { "false" });
        std::env::set_var("MAGRAY_COMPRESSION_FACTOR", self.compression_factor.to_string());
    }
    
    /// Получить описание текущей конфигурации
    pub fn describe(&self) -> String {
        format!(
            "Performance Mode: {:?}\nVector Storage: {}ms\nEmbedding Cache: {}ms\nLRU Cache: {}ms\nPromotion: {}ms\nMigration: {}ms\nCompression: {} (factor: {})",
            self.performance_mode,
            self.get_vector_storage_ms(),
            self.get_embedding_cache_ms(),
            self.get_lru_cache_ms(),
            self.get_promotion_indices_ms(),
            self.get_migration_db_ms(),
            if self.enable_compression { "enabled" } else { "disabled" },
            self.get_compression_factor()
        )
    }
}

// @component: {"k":"C","id":"flush_config","t":"Configurable flush intervals","m":{"cur":95,"tgt":100,"u":"%"},"f":["config","performance","reliability"]}