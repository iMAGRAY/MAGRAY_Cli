use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use anyhow::Result;
use tracing::{info, debug, warn};

/// @component: {"k":"C","id":"gpu_memory_pool","t":"GPU memory pool manager","m":{"cur":90,"tgt":100,"u":"%"}}
pub struct GpuMemoryPool {
    /// Пул буферов различных размеров
    pools: Arc<Mutex<Vec<BufferPool>>>,
    /// Максимальный размер пула в байтах
    max_pool_size: usize,
    /// Текущий размер всех буферов
    current_size: Arc<Mutex<usize>>,
    /// Статистика использования
    stats: Arc<Mutex<PoolStats>>,
}

struct BufferPool {
    size: usize,
    buffers: VecDeque<Vec<u8>>,
    max_buffers: usize,
}

#[derive(Debug, Default, Clone)]
pub struct PoolStats {
    pub allocations: u64,
    pub deallocations: u64,
    pub hits: u64,
    pub misses: u64,
    pub current_buffers: usize,
    pub peak_memory_usage: usize,
}

impl GpuMemoryPool {
    /// Создать новый пул памяти
    pub fn new(max_pool_size: usize) -> Self {
        info!("🏊 Инициализация GPU memory pool (max: {} MB)", max_pool_size / 1024 / 1024);
        
        // Создаём пулы для разных размеров (степени двойки)
        let mut pools = Vec::new();
        let sizes = vec![
            1024,           // 1KB
            4 * 1024,       // 4KB
            16 * 1024,      // 16KB
            64 * 1024,      // 64KB
            256 * 1024,     // 256KB
            1024 * 1024,    // 1MB
            4 * 1024 * 1024,  // 4MB
            16 * 1024 * 1024, // 16MB
        ];
        
        for size in sizes {
            let max_buffers = (max_pool_size / size / 8).max(2); // Делим на 8 для каждого размера
            pools.push(BufferPool {
                size,
                buffers: VecDeque::with_capacity(max_buffers),
                max_buffers,
            });
        }
        
        Self {
            pools: Arc::new(Mutex::new(pools)),
            max_pool_size,
            current_size: Arc::new(Mutex::new(0)),
            stats: Arc::new(Mutex::new(PoolStats::default())),
        }
    }
    
    /// Получить буфер из пула или создать новый
    pub fn acquire_buffer(&self, required_size: usize) -> Result<Vec<u8>> {
        let mut stats = self.stats.lock().unwrap();
        stats.allocations += 1;
        
        // Находим подходящий размер (ближайшая степень двойки)
        let actual_size = required_size.next_power_of_two();
        
        let mut pools = self.pools.lock().unwrap();
        
        // Ищем подходящий пул
        for pool in pools.iter_mut() {
            if pool.size >= actual_size && pool.size <= actual_size * 2 {
                // Пробуем взять из пула
                if let Some(buffer) = pool.buffers.pop_front() {
                    stats.hits += 1;
                    debug!("✅ Взят буфер {}KB из пула", pool.size / 1024);
                    return Ok(buffer);
                }
                
                // Создаём новый буфер если пул пустой
                stats.misses += 1;
                let current = *self.current_size.lock().unwrap();
                
                if current + pool.size <= self.max_pool_size {
                    let buffer = vec![0u8; pool.size];
                    *self.current_size.lock().unwrap() += pool.size;
                    stats.current_buffers += 1;
                    
                    if current + pool.size > stats.peak_memory_usage {
                        stats.peak_memory_usage = current + pool.size;
                    }
                    
                    debug!("🆕 Создан новый буфер {}KB", pool.size / 1024);
                    return Ok(buffer);
                }
            }
        }
        
        // Если не нашли подходящий пул, создаём временный буфер
        warn!("⚠️ Создан временный буфер {}KB (вне пула)", actual_size / 1024);
        Ok(vec![0u8; actual_size])
    }
    
    /// Вернуть буфер в пул
    pub fn release_buffer(&self, mut buffer: Vec<u8>) {
        let mut stats = self.stats.lock().unwrap();
        stats.deallocations += 1;
        
        let size = buffer.capacity();
        buffer.clear(); // Очищаем данные
        
        let mut pools = self.pools.lock().unwrap();
        
        // Находим подходящий пул
        for pool in pools.iter_mut() {
            if pool.size == size && pool.buffers.len() < pool.max_buffers {
                pool.buffers.push_back(buffer);
                debug!("♻️ Буфер {}KB возвращён в пул", size / 1024);
                return;
            }
        }
        
        // Если пул переполнен, просто освобождаем память
        *self.current_size.lock().unwrap() -= size;
        stats.current_buffers -= 1;
        debug!("🗑️ Буфер {}KB удалён (пул переполнен)", size / 1024);
    }
    
    /// Выполнить функцию с временным буфером
    pub fn with_buffer<F, R>(&self, size: usize, f: F) -> Result<R>
    where
        F: FnOnce(&mut Vec<u8>) -> Result<R>,
    {
        let mut buffer = self.acquire_buffer(size)?;
        let result = f(&mut buffer);
        self.release_buffer(buffer);
        result
    }
    
    /// Очистить все неиспользуемые буферы
    pub fn clear_unused(&self) {
        let mut pools = self.pools.lock().unwrap();
        let mut freed = 0;
        
        for pool in pools.iter_mut() {
            let count = pool.buffers.len();
            if count > 0 {
                freed += count * pool.size;
                pool.buffers.clear();
            }
        }
        
        if freed > 0 {
            *self.current_size.lock().unwrap() -= freed;
            let mut stats = self.stats.lock().unwrap();
            stats.current_buffers = 0;
            info!("🧹 Очищено {} MB из пула памяти", freed / 1024 / 1024);
        }
    }
    
    /// Получить статистику использования
    pub fn get_stats(&self) -> PoolStats {
        self.stats.lock().unwrap().clone()
    }
    
    /// Вывести статистику
    pub fn print_stats(&self) {
        let stats = self.get_stats();
        let current = *self.current_size.lock().unwrap();
        
        info!("📊 GPU Memory Pool статистика:");
        info!("  - Текущий размер: {} MB / {} MB", current / 1024 / 1024, self.max_pool_size / 1024 / 1024);
        info!("  - Пиковое использование: {} MB", stats.peak_memory_usage / 1024 / 1024);
        info!("  - Allocations: {} (hits: {}, misses: {})", 
            stats.allocations, stats.hits, stats.misses);
        info!("  - Hit rate: {:.1}%", 
            if stats.allocations > 0 { 
                (stats.hits as f64 / stats.allocations as f64) * 100.0 
            } else { 
                0.0 
            });
        info!("  - Текущих буферов: {}", stats.current_buffers);
    }
}

/// Глобальный GPU memory pool
lazy_static::lazy_static! {
    pub static ref GPU_MEMORY_POOL: GpuMemoryPool = {
        // Определяем размер на основе доступной GPU памяти
        let pool_size = if let Ok(detector) = std::panic::catch_unwind(|| {
            crate::gpu_detector::GpuDetector::detect()
        }) {
            if detector.available {
                // Используем 25% от свободной памяти GPU для пула
                let free_memory = detector.total_free_memory_mb() as usize;
                (free_memory * 1024 * 1024 / 4).min(2 * 1024 * 1024 * 1024) // Максимум 2GB
            } else {
                512 * 1024 * 1024 // 512MB для CPU
            }
        } else {
            512 * 1024 * 1024 // 512MB по умолчанию
        };
        
        GpuMemoryPool::new(pool_size)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_memory_pool() {
        let pool = GpuMemoryPool::new(10 * 1024 * 1024); // 10MB
        
        // Тест простого выделения
        let result = pool.with_buffer(1024, |buffer| {
            buffer.extend_from_slice(&[1, 2, 3, 4]);
            Ok(buffer.len())
        });
        assert!(result.is_ok());
        
        // Тест повторного использования
        let _ = pool.acquire_buffer(1024).unwrap();
        let _ = pool.acquire_buffer(1024).unwrap();
        
        let stats = pool.get_stats();
        assert!(stats.allocations >= 2);
        
        pool.print_stats();
    }
}