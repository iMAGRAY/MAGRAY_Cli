use anyhow::Result;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};
use sysinfo::System;

/// Динамическое управление ресурсами памяти с автомасштабированием
// @component: {"k":"C","id":"resource_manager","t":"Dynamic memory resource management","m":{"cur":95,"tgt":100,"u":"%"},"f":["memory","scaling","adaptive"]}
#[derive(Debug)]
pub struct ResourceManager {
    config: ResourceConfig,
    current_limits: Arc<RwLock<CurrentLimits>>,
    system_monitor: SystemMonitor,
    scaling_history: Arc<RwLock<Vec<ScalingEvent>>>,
}

#[derive(Debug, Clone)]
pub struct ResourceConfig {
    /// Базовые лимиты - минимум который всегда доступен
    pub base_max_vectors: usize,
    pub base_cache_size_bytes: usize,
    
    /// Пределы автомасштабирования
    pub scaling_max_vectors: usize,
    pub scaling_max_cache_bytes: usize,
    
    /// Целевое использование системной памяти (%)
    pub target_memory_usage_percent: u8,
    /// Критический порог памяти (%)
    pub critical_memory_usage_percent: u8,
    
    /// Интервал мониторинга
    pub monitoring_interval: Duration,
    /// Время для стабилизации перед масштабированием
    pub scaling_cooldown: Duration,
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            base_max_vectors: 100_000,           // 100K minimum 
            base_cache_size_bytes: 256 * 1024 * 1024, // 256MB minimum
            
            scaling_max_vectors: 5_000_000,      // 5M maximum при хорошей памяти
            scaling_max_cache_bytes: 4 * 1024 * 1024 * 1024, // 4GB maximum
            
            target_memory_usage_percent: 60,     // Целевое использование 60%
            critical_memory_usage_percent: 85,   // Критический порог 85%
            
            monitoring_interval: Duration::from_secs(30),
            scaling_cooldown: Duration::from_secs(300),  // 5 минут
        }
    }
}

#[derive(Debug, Clone)]
pub struct CurrentLimits {
    pub max_vectors: usize,
    pub cache_size_bytes: usize,
    pub last_scaled: Instant,
    pub scaling_factor: f64,
}

#[derive(Debug, Clone)]
pub struct ScalingEvent {
    pub timestamp: Instant,
    pub old_limits: CurrentLimits,
    pub new_limits: CurrentLimits,
    pub trigger: ScalingTrigger,
    pub system_memory_used_percent: f64,
}

#[derive(Debug, Clone)]
pub enum ScalingTrigger {
    MemoryPressure,
    MemoryAvailable,
    UsageGrowth,
    UsageShrink,
    Manual,
}

#[derive(Debug)]
struct SystemMonitor {
    system: System,
    total_memory_bytes: u64,
    last_check: Instant,
    memory_samples: Vec<f64>,
}

impl ResourceManager {
    pub fn new(config: ResourceConfig) -> Result<Self> {
        let system_monitor = SystemMonitor::new()?;
        
        let initial_limits = CurrentLimits {
            max_vectors: config.base_max_vectors,
            cache_size_bytes: config.base_cache_size_bytes,
            last_scaled: Instant::now(),
            scaling_factor: 1.0,
        };
        
        info!("🎯 ResourceManager initialized:");
        info!("  System memory: {:.1} GB", system_monitor.total_memory_bytes as f64 / 1024.0 / 1024.0 / 1024.0);
        info!("  Base limits: {} vectors, {} MB cache", 
              initial_limits.max_vectors, initial_limits.cache_size_bytes / 1024 / 1024);
        
        Ok(Self {
            config,
            current_limits: Arc::new(RwLock::new(initial_limits)),
            system_monitor,
            scaling_history: Arc::new(RwLock::new(Vec::new())),
        })
    }
    
    /// Получить текущие лимиты
    pub fn get_current_limits(&self) -> CurrentLimits {
        self.current_limits.read().clone()
    }
    
    /// Проверить и обновить лимиты на основе текущего состояния системы
    pub fn update_limits_if_needed(&mut self, current_usage: &ResourceUsage) -> Result<bool> {
        let memory_used_percent = self.system_monitor.get_memory_usage_percent()?;
        
        let current_limits = self.current_limits.read().clone();
        
        // Проверяем cooldown период
        if current_limits.last_scaled.elapsed() < self.config.scaling_cooldown {
            return Ok(false);
        }
        
        // Определяем необходимость масштабирования
        let scaling_decision = self.analyze_scaling_need(memory_used_percent, current_usage, &current_limits);
        
        if let Some((new_limits, trigger)) = scaling_decision {
            self.apply_scaling(current_limits, new_limits, trigger, memory_used_percent);
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Анализирует необходимость масштабирования
    fn analyze_scaling_need(
        &self,
        memory_used_percent: f64,
        usage: &ResourceUsage,
        current_limits: &CurrentLimits,
    ) -> Option<(CurrentLimits, ScalingTrigger)> {
        
        // КРИТИЧЕСКАЯ ситуация - необходимо срочно уменьшить лимиты
        if memory_used_percent > self.config.critical_memory_usage_percent as f64 {
            warn!("🚨 Critical memory usage: {:.1}%, scaling down aggressively", memory_used_percent);
            let scale_factor = 0.7; // Уменьшаем на 30%
            return Some((
                self.calculate_new_limits(current_limits, scale_factor),
                ScalingTrigger::MemoryPressure
            ));
        }
        
        // Высокое использование памяти - осторожное уменьшение
        if memory_used_percent > self.config.target_memory_usage_percent as f64 + 15.0 {
            debug!("⚠️ High memory usage: {:.1}%, scaling down conservatively", memory_used_percent);
            let scale_factor = 0.85; // Уменьшаем на 15%
            return Some((
                self.calculate_new_limits(current_limits, scale_factor),
                ScalingTrigger::MemoryPressure
            ));
        }
        
        // Низкое использование памяти И высокое использование индексов - можно увеличить
        if memory_used_percent < self.config.target_memory_usage_percent as f64 - 10.0 
           && usage.vector_usage_percent > 80.0 {
            debug!("📈 Low memory usage {:.1}%, high vector usage {:.1}%, scaling up", 
                   memory_used_percent, usage.vector_usage_percent);
            let scale_factor = 1.3; // Увеличиваем на 30%
            return Some((
                self.calculate_new_limits(current_limits, scale_factor),
                ScalingTrigger::MemoryAvailable
            ));
        }
        
        // Быстрый рост использования - превентивное увеличение лимитов
        if usage.vector_usage_percent > 90.0 && memory_used_percent < self.config.target_memory_usage_percent as f64 {
            debug!("🚀 High vector usage {:.1}%, preemptive scaling up", usage.vector_usage_percent);
            let scale_factor = 1.2; // Увеличиваем на 20%
            return Some((
                self.calculate_new_limits(current_limits, scale_factor),
                ScalingTrigger::UsageGrowth
            ));
        }
        
        None
    }
    
    /// Вычисляет новые лимиты с применением масштабирующего фактора
    fn calculate_new_limits(&self, current: &CurrentLimits, scale_factor: f64) -> CurrentLimits {
        let new_max_vectors = ((current.max_vectors as f64 * scale_factor) as usize)
            .max(self.config.base_max_vectors)
            .min(self.config.scaling_max_vectors);
            
        let new_cache_size = ((current.cache_size_bytes as f64 * scale_factor) as usize)
            .max(self.config.base_cache_size_bytes)
            .min(self.config.scaling_max_cache_bytes);
        
        CurrentLimits {
            max_vectors: new_max_vectors,
            cache_size_bytes: new_cache_size,
            last_scaled: Instant::now(),
            scaling_factor: current.scaling_factor * scale_factor,
        }
    }
    
    /// Применяет новые лимиты и записывает событие
    fn apply_scaling(&self, old_limits: CurrentLimits, new_limits: CurrentLimits, trigger: ScalingTrigger, memory_percent: f64) {
        let scaling_event = ScalingEvent {
            timestamp: Instant::now(),
            old_limits: old_limits.clone(),
            new_limits: new_limits.clone(),
            trigger: trigger.clone(),
            system_memory_used_percent: memory_percent,
        };
        
        info!("🔄 Resource scaling event: {:?}", trigger);
        info!("  Vectors: {} -> {} ({:+.1}%)", 
              old_limits.max_vectors, new_limits.max_vectors,
              ((new_limits.max_vectors as f64 / old_limits.max_vectors as f64) - 1.0) * 100.0);
        info!("  Cache: {:.1}MB -> {:.1}MB ({:+.1}%)", 
              old_limits.cache_size_bytes as f64 / 1024.0 / 1024.0,
              new_limits.cache_size_bytes as f64 / 1024.0 / 1024.0,
              ((new_limits.cache_size_bytes as f64 / old_limits.cache_size_bytes as f64) - 1.0) * 100.0);
        
        // Обновляем лимиты
        *self.current_limits.write() = new_limits;
        
        // Записываем в историю
        self.scaling_history.write().push(scaling_event);
        
        // Ограничиваем историю последними 100 событиями
        let mut history = self.scaling_history.write();
        if history.len() > 100 {
            let drain_count = history.len() - 100;
            history.drain(0..drain_count);
        }
    }
    
    /// Принудительно устанавливает лимиты (для административного управления)
    pub fn set_limits_manual(&mut self, max_vectors: usize, cache_size_bytes: usize) -> Result<()> {
        let old_limits = self.current_limits.read().clone();
        
        let new_limits = CurrentLimits {
            max_vectors: max_vectors.max(self.config.base_max_vectors).min(self.config.scaling_max_vectors),
            cache_size_bytes: cache_size_bytes.max(self.config.base_cache_size_bytes).min(self.config.scaling_max_cache_bytes),
            last_scaled: Instant::now(),
            scaling_factor: max_vectors as f64 / self.config.base_max_vectors as f64,
        };
        
        let memory_percent = self.system_monitor.get_memory_usage_percent().unwrap_or(0.0);
        self.apply_scaling(old_limits, new_limits, ScalingTrigger::Manual, memory_percent);
        
        Ok(())
    }
    
    /// Получить текущее использование ресурсов
    pub fn current_usage(&self) -> ResourceUsage {
        let limits = self.current_limits.read();
        // В реальной ситуации здесь бы был подсчет актуального использования
        // Пока возвращаем базовую статистику
        ResourceUsage::new(
            50_000,  // current_vectors - пример
            limits.max_vectors,
            limits.cache_size_bytes / 2, // current_cache_size - пример 50% использования
            limits.cache_size_bytes,
        )
    }
    
    /// Проверить есть ли давление на память
    pub fn is_memory_pressure(&mut self) -> bool {
        match self.system_monitor.get_memory_usage_percent() {
            Ok(usage) => usage > self.config.critical_memory_usage_percent as f64,
            Err(_) => false,
        }
    }
    
    /// Адаптировать лимиты в зависимости от текущей ситуации
    pub fn adapt_limits(&mut self) {
        let current_usage = self.current_usage();
        if let Err(e) = self.update_limits_if_needed(&current_usage) {
            warn!("Ошибка при адаптации лимитов: {}", e);
        }
    }
    /// Получить статистику масштабирования
    pub fn get_scaling_stats(&self) -> ScalingStats {
        let history = self.scaling_history.read();
        let current = self.current_limits.read();
        
        ScalingStats {
            total_scaling_events: history.len(),
            current_scaling_factor: current.scaling_factor,
            last_scaling_event: history.last().cloned(),
            memory_pressure_events: history.iter().filter(|e| matches!(e.trigger, ScalingTrigger::MemoryPressure)).count(),
            growth_events: history.iter().filter(|e| matches!(e.trigger, ScalingTrigger::MemoryAvailable | ScalingTrigger::UsageGrowth)).count(),
        }
    }
}

impl SystemMonitor {
    fn new() -> Result<Self> {
        let mut system = System::new_all();
        system.refresh_all();
        
        let total_memory = system.total_memory() * 1024; // sysinfo returns KB, convert to bytes
        
        info!("💾 Real system monitoring initialized: {:.1} GB total memory", 
              total_memory as f64 / 1024.0 / 1024.0 / 1024.0);
        
        Ok(Self {
            system,
            total_memory_bytes: total_memory,
            last_check: Instant::now(),
            memory_samples: Vec::with_capacity(10),
        })
    }
    
    fn get_memory_usage_percent(&mut self) -> Result<f64> {
        // Обновляем информацию о системе только если прошло время
        if self.last_check.elapsed() > Duration::from_secs(5) {
            self.system.refresh_memory();
            self.last_check = Instant::now();
        }
        
        let used_memory = self.system.used_memory() * 1024; // KB to bytes
        let usage_percent = (used_memory as f64 / self.total_memory_bytes as f64) * 100.0;
        
        // Сглаживание для предотвращения осцилляций
        self.memory_samples.push(usage_percent);
        if self.memory_samples.len() > 5 {
            self.memory_samples.remove(0);
        }
        
        let avg_usage = self.memory_samples.iter().sum::<f64>() / self.memory_samples.len() as f64;
        
        debug!("💾 Memory usage: {:.1}% (used: {:.1} GB / total: {:.1} GB)", 
               avg_usage, 
               used_memory as f64 / 1024.0 / 1024.0 / 1024.0,
               self.total_memory_bytes as f64 / 1024.0 / 1024.0 / 1024.0);
        
        Ok(avg_usage)
    }
    
    /// Получаем подробную статистику памяти
    #[allow(dead_code)]
    pub fn get_detailed_memory_info(&mut self) -> DetailedMemoryInfo {
        self.system.refresh_memory();
        
        DetailedMemoryInfo {
            total_memory_bytes: self.total_memory_bytes,
            used_memory_bytes: self.system.used_memory() * 1024,
            available_memory_bytes: self.system.available_memory() * 1024,
            usage_percent: (self.system.used_memory() as f64 / self.system.total_memory() as f64) * 100.0,
            swap_total_bytes: self.system.total_swap() * 1024,
            swap_used_bytes: self.system.used_swap() * 1024,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DetailedMemoryInfo {
    pub total_memory_bytes: u64,
    pub used_memory_bytes: u64,
    pub available_memory_bytes: u64,
    pub usage_percent: f64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub current_vectors: usize,
    pub max_vectors: usize,
    pub vector_usage_percent: f64,
    pub current_cache_size: usize,
    pub max_cache_size: usize,
    pub cache_usage_percent: f64,
}

impl ResourceUsage {
    pub fn new(current_vectors: usize, max_vectors: usize, current_cache_size: usize, max_cache_size: usize) -> Self {
        Self {
            current_vectors,
            max_vectors,
            vector_usage_percent: (current_vectors as f64 / max_vectors as f64) * 100.0,
            current_cache_size,
            max_cache_size,
            cache_usage_percent: (current_cache_size as f64 / max_cache_size as f64) * 100.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScalingStats {
    pub total_scaling_events: usize,
    pub current_scaling_factor: f64,
    pub last_scaling_event: Option<ScalingEvent>,
    pub memory_pressure_events: usize,
    pub growth_events: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_resource_manager_creation() {
        let config = ResourceConfig::default();
        let manager = ResourceManager::new(config).unwrap();
        
        let limits = manager.get_current_limits();
        assert_eq!(limits.max_vectors, 100_000);
        assert!(limits.cache_size_bytes > 0);
    }
    
    #[test]
    fn test_scaling_calculation() {
        let config = ResourceConfig::default();
        let manager = ResourceManager::new(config).unwrap();
        
        let current = CurrentLimits {
            max_vectors: 100_000,
            cache_size_bytes: 256 * 1024 * 1024,
            last_scaled: Instant::now(),
            scaling_factor: 1.0,
        };
        
        let scaled = manager.calculate_new_limits(&current, 1.5);
        assert_eq!(scaled.max_vectors, 150_000);
        assert_eq!(scaled.cache_size_bytes, 384 * 1024 * 1024);
    }
}