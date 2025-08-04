use ai::{MemoryPool, PoolStats, get_input_buffer, return_input_buffer, get_pool_stats};
use std::sync::Arc;
use std::thread;

#[test]
fn test_memory_pool_creation() {
    let pool = MemoryPool::new();
    let stats = pool.get_stats();
    
    assert_eq!(stats.total_gets, 0);
    assert_eq!(stats.total_returns, 0);
    assert_eq!(stats.cache_hits, 0);
    assert_eq!(stats.hit_rate, 0.0);
}

#[test]
fn test_memory_pool_input_buffer() {
    let pool = MemoryPool::new();
    
    // Get buffer
    let buffer = pool.get_input_buffer(100);
    assert!(buffer.capacity() >= 100);
    assert!(buffer.is_empty());
    
    // Return buffer
    pool.return_input_buffer(buffer);
    
    // Get another buffer (should reuse)
    let buffer2 = pool.get_input_buffer(50);
    assert!(buffer2.capacity() >= 50);
    
    let stats = pool.get_stats();
    assert_eq!(stats.total_gets, 2);
    assert_eq!(stats.total_returns, 1);
    assert_eq!(stats.cache_hits, 1);
    assert_eq!(stats.hit_rate, 0.5);
}

#[test]
fn test_memory_pool_output_buffer() {
    let pool = MemoryPool::new();
    
    let buffer = pool.get_output_buffer(1024);
    assert!(buffer.capacity() >= 1024);
    assert!(buffer.is_empty());
    
    pool.return_output_buffer(buffer);
    
    // Should reuse buffer
    let buffer2 = pool.get_output_buffer(512);
    assert!(buffer2.capacity() >= 512);
    
    let stats = pool.get_stats();
    assert!(stats.cache_hits > 0);
}

#[test]
fn test_memory_pool_attention_buffer() {
    let pool = MemoryPool::new();
    
    let buffer = pool.get_attention_buffer(256);
    assert!(buffer.capacity() >= 256);
    
    pool.return_attention_buffer(buffer);
    
    let buffer2 = pool.get_attention_buffer(128);
    assert!(buffer2.capacity() >= 128);
    
    let stats = pool.get_stats();
    assert!(stats.cache_hits > 0);
}

#[test]
fn test_memory_pool_token_type_buffer() {
    let pool = MemoryPool::new();
    
    let buffer = pool.get_token_type_buffer(256);
    assert!(buffer.capacity() >= 256);
    
    pool.return_token_type_buffer(buffer);
    
    let buffer2 = pool.get_token_type_buffer(128);
    assert!(buffer2.capacity() >= 128);
    
    let stats = pool.get_stats();
    assert!(stats.cache_hits > 0);
}

#[test]
fn test_memory_pool_oversized_buffer() {
    let pool = MemoryPool::new();
    
    // Create oversized buffer
    let mut buffer = Vec::with_capacity(20000);
    buffer.resize(20000, 0i64);
    
    pool.return_input_buffer(buffer);
    
    // Oversized buffer should not be kept
    let buffer2 = pool.get_input_buffer(100);
    assert!(buffer2.capacity() < 20000);
    
    let stats = pool.get_stats();
    assert_eq!(stats.cache_hits, 0); // No reuse of oversized buffer
}

#[test]
fn test_memory_pool_thread_local() {
    let pool = Arc::new(MemoryPool::new());
    let mut handles = vec![];
    
    // Each thread should have its own buffer pool
    for _ in 0..5 {
        let pool_clone = Arc::clone(&pool);
        let handle = thread::spawn(move || {
            for _ in 0..10 {
                let buffer = pool_clone.get_input_buffer(100);
                pool_clone.return_input_buffer(buffer);
            }
        });
        handles.push(handle);
    }
    
    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }
    
    let stats = pool.get_stats();
    assert_eq!(stats.total_gets, 50); // 5 threads * 10 gets
    assert_eq!(stats.total_returns, 50);
    assert!(stats.cache_hits > 0); // Should have reuses
}

#[test]
fn test_pool_stats() {
    let stats = PoolStats {
        total_gets: 100,
        total_returns: 90,
        cache_hits: 80,
        hit_rate: 0.8,
    };
    
    assert_eq!(stats.total_gets, 100);
    assert_eq!(stats.total_returns, 90);
    assert_eq!(stats.cache_hits, 80);
    assert_eq!(stats.hit_rate, 0.8);
}

#[test]
fn test_global_memory_pool() {
    // Test global pool instance
    let buffer1 = get_input_buffer(100);
    assert!(buffer1.capacity() >= 100);
    
    return_input_buffer(buffer1);
    
    let buffer2 = get_input_buffer(50);
    assert!(buffer2.capacity() >= 50);
    
    let stats = get_pool_stats();
    assert!(stats.total_gets >= 2);
    assert!(stats.total_returns >= 1);
}

#[test]
fn test_pooled_buffer_raii() {
    use ai::memory_pool::PooledBuffer;
    
    let pool = Arc::new(MemoryPool::new());
    let pool_clone = pool.clone();
    
    {
        let buffer = pool.get_input_buffer(100);
        let mut pooled = PooledBuffer::new(buffer, Box::new(move |buf| {
            pool_clone.return_input_buffer(buf);
        }));
        
        // Use buffer
        pooled.as_mut().push(42);
        assert_eq!(pooled.as_ref()[0], 42);
        
        // Buffer automatically returned when pooled goes out of scope
    }
    
    let stats = pool.get_stats();
    assert_eq!(stats.total_returns, 1);
}

#[test]
fn test_pooled_buffer_take() {
    use ai::memory_pool::PooledBuffer;
    
    let pool = Arc::new(MemoryPool::new());
    let pool_clone = pool.clone();
    
    let buffer = pool.get_input_buffer(100);
    let pooled = PooledBuffer::new(buffer, Box::new(move |buf| {
        pool_clone.return_input_buffer(buf);
    }));
    
    // Take ownership - buffer won't be returned
    let _owned = pooled.take();
    
    let stats = pool.get_stats();
    assert_eq!(stats.total_returns, 0); // Not returned
}