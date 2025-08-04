use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::Arc;
use std::ops::{Deref, DerefMut};
use thread_local::ThreadLocal;
use tracing::{debug, info};

/// Memory pool for reusing buffers across embedding operations
pub struct MemoryPool {
    // Thread-local storage for buffers to avoid contention
    input_buffers: ThreadLocal<RefCell<VecDeque<Vec<i64>>>>,
    output_buffers: ThreadLocal<RefCell<VecDeque<Vec<f32>>>>,
    attention_buffers: ThreadLocal<RefCell<VecDeque<Vec<i64>>>>,
    token_type_buffers: ThreadLocal<RefCell<VecDeque<Vec<i64>>>>,
    
    // Pool statistics
    total_gets: std::sync::atomic::AtomicU64,
    total_returns: std::sync::atomic::AtomicU64,
    cache_hits: std::sync::atomic::AtomicU64,
}

impl MemoryPool {
    /// Create new memory pool
    pub fn new() -> Self {
        info!("Creating optimized memory pool");
        Self {
            input_buffers: ThreadLocal::new(),
            output_buffers: ThreadLocal::new(),
            attention_buffers: ThreadLocal::new(),
            token_type_buffers: ThreadLocal::new(),
            total_gets: std::sync::atomic::AtomicU64::new(0),
            total_returns: std::sync::atomic::AtomicU64::new(0),
            cache_hits: std::sync::atomic::AtomicU64::new(0),
        }
    }
    
    /// Get input buffer (reused if available)
    pub fn get_input_buffer(&self, min_capacity: usize) -> Vec<i64> {
        self.total_gets.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        let mut buffer = {
            let buffers = self.input_buffers.get_or(|| RefCell::new(VecDeque::new()));
            let mut buffers = buffers.borrow_mut();
            
            // Try to find a buffer with sufficient capacity
            for _ in 0..buffers.len() {
                if let Some(buf) = buffers.pop_front() {
                    if buf.capacity() >= min_capacity {
                        self.cache_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        debug!("Reused input buffer with capacity: {}", buf.capacity());
                        return buf;
                    } else {
                        // Put back if too small
                        buffers.push_back(buf);
                        break;
                    }
                }
            }
            
            // No suitable buffer found, create new one
            Vec::with_capacity(min_capacity.max(512))
        };
        
        buffer.clear(); // Ensure it's empty but keep capacity
        buffer
    }
    
    /// Return input buffer to pool
    pub fn return_input_buffer(&self, mut buffer: Vec<i64>) {
        self.total_returns.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        // Don't keep extremely large buffers
        if buffer.capacity() > 10000 {
            debug!("Dropping oversized input buffer: {} capacity", buffer.capacity());
            return;
        }
        
        buffer.clear(); // Clear but keep capacity
        
        {
            let buffers = self.input_buffers.get_or(|| RefCell::new(VecDeque::new()));
            let mut buffers = buffers.borrow_mut();
            
            // Limit pool size per thread
            if buffers.len() < 10 {
                buffers.push_back(buffer);
                debug!("Returned input buffer to pool, pool size: {}", buffers.len());
            }
        }
    }
    
    /// Get output buffer for embeddings
    pub fn get_output_buffer(&self, min_capacity: usize) -> Vec<f32> {
        self.total_gets.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        let mut buffer = {
            let buffers = self.output_buffers.get_or(|| RefCell::new(VecDeque::new()));
            let mut buffers = buffers.borrow_mut();
            
            if let Some(buf) = buffers.pop_front() {
                if buf.capacity() >= min_capacity {
                    self.cache_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    debug!("Reused output buffer with capacity: {}", buf.capacity());
                    return buf;
                }
                // Put back if too small
                buffers.push_back(buf);
            }
            
            Vec::with_capacity(min_capacity.max(1024))
        };
        
        buffer.clear();
        buffer
    }
    
    /// Return output buffer to pool
    pub fn return_output_buffer(&self, mut buffer: Vec<f32>) {
        self.total_returns.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        if buffer.capacity() > 5000 {
            return; // Don't keep very large buffers
        }
        
        buffer.clear();
        
        {
            let buffers = self.output_buffers.get_or(|| RefCell::new(VecDeque::new()));
            let mut buffers = buffers.borrow_mut();
            if buffers.len() < 10 {
                buffers.push_back(buffer);
            }
        }
    }
    
    /// Get attention mask buffer
    pub fn get_attention_buffer(&self, min_capacity: usize) -> Vec<i64> {
        self.total_gets.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        let mut buffer = {
            let buffers = self.attention_buffers.get_or(|| RefCell::new(VecDeque::new()));
            let mut buffers = buffers.borrow_mut();
            
            if let Some(buf) = buffers.pop_front() {
                if buf.capacity() >= min_capacity {
                    self.cache_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    return buf;
                }
                buffers.push_back(buf);
            }
            
            Vec::with_capacity(min_capacity.max(512))
        };
        
        buffer.clear();
        buffer
    }
    
    /// Return attention buffer
    pub fn return_attention_buffer(&self, mut buffer: Vec<i64>) {
        self.total_returns.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        if buffer.capacity() > 10000 {
            return;
        }
        
        buffer.clear();
        
        {
            let buffers = self.attention_buffers.get_or(|| RefCell::new(VecDeque::new()));
            let mut buffers = buffers.borrow_mut();
            if buffers.len() < 10 {
                buffers.push_back(buffer);
            }
        }
    }
    
    /// Get token type buffer
    pub fn get_token_type_buffer(&self, min_capacity: usize) -> Vec<i64> {
        self.total_gets.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        let mut buffer = {
            let buffers = self.token_type_buffers.get_or(|| RefCell::new(VecDeque::new()));
            let mut buffers = buffers.borrow_mut();
            
            if let Some(buf) = buffers.pop_front() {
                if buf.capacity() >= min_capacity {
                    self.cache_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    return buf;
                }
                buffers.push_back(buf);
            }
            
            Vec::with_capacity(min_capacity.max(512))
        };
        
        buffer.clear();
        buffer
    }
    
    /// Return token type buffer
    pub fn return_token_type_buffer(&self, mut buffer: Vec<i64>) {
        self.total_returns.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        if buffer.capacity() > 10000 {
            return;
        }
        
        buffer.clear();
        
        {
            let buffers = self.token_type_buffers.get_or(|| RefCell::new(VecDeque::new()));
            let mut buffers = buffers.borrow_mut();
            if buffers.len() < 10 {
                buffers.push_back(buffer);
            }
        }
    }
    
    /// Get pool statistics
    pub fn get_stats(&self) -> PoolStats {
        let total_gets = self.total_gets.load(std::sync::atomic::Ordering::Relaxed);
        let total_returns = self.total_returns.load(std::sync::atomic::Ordering::Relaxed);
        let cache_hits = self.cache_hits.load(std::sync::atomic::Ordering::Relaxed);
        
        let hit_rate = if total_gets > 0 {
            cache_hits as f64 / total_gets as f64
        } else {
            0.0
        };
        
        PoolStats {
            total_gets,
            total_returns,
            cache_hits,
            hit_rate,
        }
    }
    
    /// Clear all pools (for cleanup)
    pub fn clear(&self) {
        info!("Clearing memory pools");
        
        // Note: ThreadLocal doesn't have a direct clear method,
        // but buffers will be cleaned up when threads end
    }
}

impl Default for MemoryPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_gets: u64,
    pub total_returns: u64,
    pub cache_hits: u64,
    pub hit_rate: f64,
}

/// RAII wrapper for automatic buffer return
pub struct PooledBuffer<T> {
    buffer: Option<Vec<T>>,
    return_fn: Box<dyn FnOnce(Vec<T>)>,
}

impl<T> PooledBuffer<T> {
    pub fn new(buffer: Vec<T>, return_fn: Box<dyn FnOnce(Vec<T>)>) -> Self {
        Self {
            buffer: Some(buffer),
            return_fn,
        }
    }
    
    /// Get mutable reference to the buffer
    pub fn get_mut(&mut self) -> &mut Vec<T> {
        self.buffer.as_mut().unwrap()
    }
    
    /// Get immutable reference to the buffer
    pub fn get_ref(&self) -> &Vec<T> {
        self.buffer.as_ref().unwrap()
    }
    
    /// Take ownership of the buffer (prevents automatic return)
    pub fn take(mut self) -> Vec<T> {
        self.buffer.take().unwrap()
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
        self.buffer.as_ref().unwrap()
    }
}

impl<T> DerefMut for PooledBuffer<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.buffer.as_mut().unwrap()
    }
}

lazy_static::lazy_static! {
    /// Global memory pool instance
    pub static ref GLOBAL_MEMORY_POOL: Arc<MemoryPool> = Arc::new(MemoryPool::new());
}

/// Helper functions for easy access to global pool
pub fn get_input_buffer(min_capacity: usize) -> Vec<i64> {
    GLOBAL_MEMORY_POOL.get_input_buffer(min_capacity)
}

pub fn return_input_buffer(buffer: Vec<i64>) {
    GLOBAL_MEMORY_POOL.return_input_buffer(buffer);
}

pub fn get_output_buffer(min_capacity: usize) -> Vec<f32> {
    GLOBAL_MEMORY_POOL.get_output_buffer(min_capacity)
}

pub fn return_output_buffer(buffer: Vec<f32>) {
    GLOBAL_MEMORY_POOL.return_output_buffer(buffer);
}

pub fn get_pool_stats() -> PoolStats {
    GLOBAL_MEMORY_POOL.get_stats()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_memory_pool_basic() {
        let pool = MemoryPool::new();
        
        // Get buffer
        let buffer = pool.get_input_buffer(100);
        assert!(buffer.capacity() >= 100);
        
        // Return buffer
        pool.return_input_buffer(buffer);
        
        // Get another buffer (should reuse)
        let buffer2 = pool.get_input_buffer(50);
        assert!(buffer2.capacity() >= 50);
        
        let stats = pool.get_stats();
        assert_eq!(stats.total_gets, 2);
        assert_eq!(stats.total_returns, 1);
        assert!(stats.cache_hits >= 1);
    }
    
    #[test]
    fn test_pooled_buffer_raii() {
        let pool = Arc::new(MemoryPool::new());
        let pool_clone = pool.clone();
        
        {
            let buffer = pool.get_input_buffer(100);
            let _pooled = PooledBuffer::new(buffer, Box::new(move |buf| {
                pool_clone.return_input_buffer(buf);
            }));
            
            // Buffer automatically returned when _pooled goes out of scope
        }
        
        let stats = pool.get_stats();
        assert_eq!(stats.total_returns, 1);
    }
}