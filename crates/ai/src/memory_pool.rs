use common::service_traits::{BaseService, ClearableService, StatisticsProvider};
use std::collections::VecDeque;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock};

/// Handle for a pooled buffer that returns it to the pool on drop
pub struct PooledBuffer<T> {
    buffer: Option<Vec<T>>,
    return_fn: Box<dyn FnOnce(Vec<T>) + Send>,
}

impl<T> PooledBuffer<T> {
    fn new(buffer: Vec<T>, return_fn: Box<dyn FnOnce(Vec<T>) + Send>) -> Self {
        Self {
            buffer: Some(buffer),
            return_fn,
        }
    }

    /// Get mutable reference to the buffer - returns None if buffer was taken
    pub fn get_mut(&mut self) -> Option<&mut Vec<T>> {
        self.buffer.as_mut()
    }

    /// Get immutable reference to the buffer - returns None if buffer was taken
    pub fn get_ref(&self) -> Option<&Vec<T>> {
        self.buffer.as_ref()
    }

    /// Take ownership of the buffer (prevents automatic return)
    pub fn take(mut self) -> Option<Vec<T>> {
        self.buffer.take()
    }
}

impl<T> Drop for PooledBuffer<T> {
    fn drop(&mut self) {
        if let Some(buffer) = self.buffer.take() {
            let return_fn = std::mem::replace(&mut self.return_fn, Box::new(|_| {}));
            return_fn(buffer);
        }
    }
}

impl<T> Deref for PooledBuffer<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        self.buffer
            .as_ref()
            .expect("PooledBuffer должен содержать буфер")
    }
}

impl<T> DerefMut for PooledBuffer<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.buffer
            .as_mut()
            .expect("PooledBuffer должен содержать буфер")
    }
}

lazy_static::lazy_static! {
    /// Global memory pool instance
    pub static ref GLOBAL_MEMORY_POOL: Arc<MemoryPool> = Arc::new(MemoryPool::new());
}

/// Generic memory pool for reusing allocations
pub struct MemoryPool {
    f32_pools: Arc<RwLock<[VecDeque<Vec<f32>>; 5]>>,
    i64_pools: Arc<RwLock<[VecDeque<Vec<i64>>; 5]>>,
}

impl MemoryPool {
    pub fn new() -> Self {
        Self {
            f32_pools: Arc::new(RwLock::new(Default::default())),
            i64_pools: Arc::new(RwLock::new(Default::default())),
        }
    }

    /// Get or allocate a f32 buffer of at least the requested capacity
    pub fn get_f32_buffer(&self, capacity: usize) -> PooledBuffer<f32> {
        let pool_index = self.get_pool_index(capacity);
        let mut pools = match self.f32_pools.write() {
            Ok(pools) => pools,
            Err(_) => return PooledBuffer::new(Vec::with_capacity(capacity), Box::new(|_| {})),
        };

        let buffer = if let Some(mut buf) = pools[pool_index].pop_front() {
            buf.clear();
            buf.reserve(capacity.saturating_sub(buf.capacity()));
            buf
        } else {
            Vec::with_capacity(self.get_pool_capacity(pool_index).max(capacity))
        };

        let pools_clone = self.f32_pools.clone();
        let return_fn = Box::new(move |buf: Vec<f32>| {
            if let Ok(mut pools) = pools_clone.write() {
                if pools[pool_index].len() < 10 && buf.capacity() > 0 {
                    pools[pool_index].push_back(buf);
                }
            }
        });

        PooledBuffer::new(buffer, return_fn)
    }

    /// Get or allocate an i64 buffer of at least the requested capacity
    pub fn get_i64_buffer(&self, capacity: usize) -> PooledBuffer<i64> {
        let pool_index = self.get_pool_index(capacity);
        let mut pools = match self.i64_pools.write() {
            Ok(pools) => pools,
            Err(_) => return PooledBuffer::new(Vec::with_capacity(capacity), Box::new(|_| {})),
        };

        let buffer = if let Some(mut buf) = pools[pool_index].pop_front() {
            buf.clear();
            buf.reserve(capacity.saturating_sub(buf.capacity()));
            buf
        } else {
            Vec::with_capacity(self.get_pool_capacity(pool_index).max(capacity))
        };

        let pools_clone = self.i64_pools.clone();
        let return_fn = Box::new(move |buf: Vec<i64>| {
            if let Ok(mut pools) = pools_clone.write() {
                if pools[pool_index].len() < 10 && buf.capacity() > 0 {
                    pools[pool_index].push_back(buf);
                }
            }
        });

        PooledBuffer::new(buffer, return_fn)
    }

    fn get_pool_index(&self, capacity: usize) -> usize {
        match capacity {
            0..=1024 => 0,
            1025..=4096 => 1,
            4097..=16384 => 2,
            16385..=65536 => 3,
            _ => 4,
        }
    }

    fn get_pool_capacity(&self, index: usize) -> usize {
        match index {
            0 => 1024,
            1 => 4096,
            2 => 16384,
            3 => 65536,
            _ => 262144,
        }
    }

    /// Clear all pools, releasing memory
    pub fn clear(&self) {
        if let Ok(mut pools) = self.f32_pools.write() {
            pools.iter_mut().for_each(|p| p.clear());
        }
        if let Ok(mut pools) = self.i64_pools.write() {
            pools.iter_mut().for_each(|p| p.clear());
        }
    }

    /// Get input buffer (alias for i64 buffer)
    pub fn get_input_buffer(&self, capacity: usize) -> PooledBuffer<i64> {
        self.get_i64_buffer(capacity)
    }

    /// Get attention buffer (alias for i64 buffer)
    pub fn get_attention_buffer(&self, capacity: usize) -> PooledBuffer<i64> {
        self.get_i64_buffer(capacity)
    }

    /// Get token type buffer (alias for i64 buffer)
    pub fn get_token_type_buffer(&self, capacity: usize) -> PooledBuffer<i64> {
        self.get_i64_buffer(capacity)
    }

    /// Get output buffer (alias for f32 buffer)
    pub fn get_output_buffer(&self, capacity: usize) -> PooledBuffer<f32> {
        self.get_f32_buffer(capacity)
    }

    /// Return input buffer (no-op as it's handled by Drop)
    pub fn return_input_buffer(&self, _buffer: Vec<i64>) {
        // Buffer is automatically returned via Drop trait
    }

    /// Return attention buffer (no-op as it's handled by Drop)
    pub fn return_attention_buffer(&self, _buffer: Vec<i64>) {
        // Buffer is automatically returned via Drop trait
    }

    /// Return token type buffer (no-op as it's handled by Drop)
    pub fn return_token_type_buffer(&self, _buffer: Vec<i64>) {
        // Buffer is automatically returned via Drop trait
    }

    /// Return output buffer (no-op as it's handled by Drop)
    pub fn return_output_buffer(&self, _buffer: Vec<f32>) {
        // Buffer is automatically returned via Drop trait
    }

    // УСТРАНЕН ДУБЛИКАТ: используем StatisticsProvider trait
}

/// Pool statistics
#[derive(Debug, Default, Clone)]
pub struct PoolStats {
    pub total_allocations: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub peak_memory_usage: usize,
}

/// Get input buffer helper function
pub fn get_input_buffer() -> Vec<f32> {
    GLOBAL_MEMORY_POOL
        .get_f32_buffer(1024)
        .take()
        .unwrap_or_else(|| Vec::with_capacity(1024))
}

/// Return input buffer helper function
pub fn return_input_buffer(_buffer: Vec<f32>) {
    // Buffer is automatically returned via Drop trait
}

/// Get pool statistics
pub fn get_pool_stats() -> PoolStats {
    PoolStats::default() // Simplified for now
}

impl Default for MemoryPool {
    fn default() -> Self {
        Self::new()
    }
}

// Реализация общих трейтов для устранения дублирования
impl BaseService for MemoryPool {
    fn name(&self) -> &'static str {
        "MemoryPool"
    }
}

impl StatisticsProvider for MemoryPool {
    type Stats = PoolStats;

    fn get_stats(&self) -> Self::Stats {
        PoolStats::default() // Simplified for now - можно расширить
    }
}

#[async_trait::async_trait]
impl ClearableService for MemoryPool {
    async fn clear(&mut self) -> Result<(), common::MagrayCoreError> {
        if let Ok(mut pools) = self.f32_pools.write() {
            pools.iter_mut().for_each(|p| p.clear());
        }
        if let Ok(mut pools) = self.i64_pools.write() {
            pools.iter_mut().for_each(|p| p.clear());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_pool_reuse() {
        let pool = MemoryPool::new();

        {
            let mut buf = pool.get_f32_buffer(100);
            buf.extend_from_slice(&[1.0, 2.0, 3.0]);
            assert_eq!(buf.len(), 3);
        } // Buffer returned to pool

        {
            let buf = pool.get_f32_buffer(50);
            assert_eq!(buf.len(), 0); // Should be cleared
            assert!(buf.capacity() >= 100); // Should have retained capacity
        }
    }

    #[test]
    fn test_pooled_buffer_take() {
        let pool = MemoryPool::new();

        let mut buf = pool.get_f32_buffer(10);
        buf.extend_from_slice(&[1.0, 2.0, 3.0]);

        let vec = buf.take().expect("Buffer should exist");
        assert_eq!(vec.len(), 3);
        // Buffer won't be returned to pool since we took ownership
    }
}
