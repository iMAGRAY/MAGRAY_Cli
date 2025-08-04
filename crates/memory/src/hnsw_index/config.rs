use serde::{Deserialize, Serialize};

/// Профессиональная конфигурация для HNSW индекса
/// Основана на рекомендациях из hnsw_rs библиотеки от Jean-Pierre Both
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswConfig {
    /// Размерность векторов
    pub dimension: usize,
    /// Максимальное количество связей на узел (M) - ключевой параметр качества
    /// Рекомендации: 16-48 для большинства случаев, больше = выше качество но больше памяти
    pub max_connections: usize,  
    /// Размер списка кандидатов при построении (ef_construction) - влияет на качество
    /// Рекомендации: 200-800, больше = выше качество но медленнее построение
    pub ef_construction: usize,
    /// Размер списка кандидатов при поиске (ef_search) - баланс скорость/качество  
    /// Рекомендации: равен или больше чем топ-k, можно динамически менять
    pub ef_search: usize,
    /// Максимальное количество элементов в индексе
    pub max_elements: usize,
    /// Максимальное количество слоев в графе
    /// Рекомендации: ln(max_elements) или 16 для большинства случаев
    pub max_layers: usize,
    /// Использовать параллельные операции для больших датасетов
    /// Рекомендации: true для >10k элементов
    pub use_parallel: bool,
}

impl Default for HnswConfig {
    fn default() -> Self {
        Self {
            dimension: 1024,       // Qwen3 фактическая размерность из config.json
            max_connections: 24,   // Оптимальное значение для большинства случаев
            ef_construction: 400,  // Высокое качество построения (200-800 стандарт)
            ef_search: 100,        // Баланс скорость/точность
            max_elements: 1_000_000, // 1M элементов по умолчанию
            max_layers: 16,        // Стандартное значение
            use_parallel: true,    // Многопоточность для больших датасетов
        }
    }
}

impl HnswConfig {
    /// Создать конфигурацию оптимизированную для высокого качества поиска
    pub fn high_quality() -> Self {
        Self {
            max_connections: 48,
            ef_construction: 800,
            ef_search: 200,
            use_parallel: true,
            ..Default::default()
        }
    }
    
    /// Создать конфигурацию оптимизированную для скорости
    pub fn high_speed() -> Self {
        Self {
            max_connections: 16,
            ef_construction: 200,
            ef_search: 50,
            use_parallel: true,
            ..Default::default()
        }
    }
    
    /// Создать конфигурацию для малых датасетов (<10k элементов)
    pub fn small_dataset() -> Self {
        Self {
            max_connections: 16,
            ef_construction: 200,
            ef_search: 50,
            max_elements: 10_000,
            use_parallel: false,
            ..Default::default()
        }
    }
    
    /// Создать конфигурацию для больших датасетов (>1M элементов)
    pub fn large_dataset() -> Self {
        Self {
            max_connections: 32,
            ef_construction: 600,
            ef_search: 150,
            max_elements: 10_000_000,
            use_parallel: true,
            ..Default::default()
        }
    }
    
    /// Валидация конфигурации
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.dimension == 0 {
            return Err(anyhow::anyhow!("dimension must be > 0"));
        }
        
        if self.max_connections == 0 {
            return Err(anyhow::anyhow!("max_connections must be > 0"));
        }
        
        if self.ef_construction < self.max_connections {
            return Err(anyhow::anyhow!("ef_construction should be >= max_connections"));
        }
        
        if self.ef_search == 0 {
            return Err(anyhow::anyhow!("ef_search must be > 0"));
        }
        
        if self.max_elements == 0 {
            return Err(anyhow::anyhow!("max_elements must be > 0"));
        }
        
        if self.max_layers == 0 {
            return Err(anyhow::anyhow!("max_layers must be > 0"));
        }
        
        Ok(())
    }
    
    /// Рассчитать примерное потребление памяти в байтах
    pub fn estimate_memory_usage(&self, element_count: usize) -> u64 {
        let actual_elements = element_count.min(self.max_elements);
        
        // Приблизительная формула для HNSW:
        // - каждый вектор: dimension * 4 bytes (f32)
        // - граф соединений: connections * node_count * 4 bytes (usize)
        // - служебные структуры: ~20% overhead
        
        let vector_data = (actual_elements * self.dimension * 4) as u64;
        let graph_data = (actual_elements * self.max_connections * 4) as u64;
        let overhead = ((vector_data + graph_data) as f64 * 0.2) as u64;
        
        vector_data + graph_data + overhead
    }
    
    /// Рассчитать примерное время построения индекса
    pub fn estimate_build_time_seconds(&self, element_count: usize) -> f64 {
        let actual_elements = element_count.min(self.max_elements);
        
        // Приблизительная формула основана на O(log N) сложности HNSW
        // и экспериментальных данных
        let base_time = (actual_elements as f64).ln() * 0.001; // базовое время в секундах
        
        // Учитываем параметры качества
        let quality_factor = (self.ef_construction as f64 / 200.0).powf(1.2);
        let connection_factor = (self.max_connections as f64 / 16.0).powf(0.8);
        
        // Учитываем параллелизм
        let parallel_factor = if self.use_parallel { 0.3 } else { 1.0 };
        
        base_time * quality_factor * connection_factor * parallel_factor
    }
}