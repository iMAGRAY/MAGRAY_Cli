use memory::cache_lru::*;
use std::sync::Arc;

#[test]
fn test_lru_cache_creation() {
    let cache = LruEmbeddingCache::new(100, 1000);
    
    assert_eq!(cache.max_size(), 100);
    assert_eq!(cache.current_size(), 0);
    assert!(cache.is_empty());
}

#[test]
fn test_lru_cache_insert_and_get() {
    let cache = LruEmbeddingCache::new(10, 1000);
    let key = "test_key".to_string();
    let embedding = vec![0.1, 0.2, 0.3, 0.4, 0.5];
    
    cache.insert(key.clone(), embedding.clone());
    
    assert_eq!(cache.current_size(), 1);
    assert!(!cache.is_empty());
    
    let retrieved = cache.get(&key);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap(), embedding);
}

#[test]
fn test_lru_cache_miss() {
    let cache = LruEmbeddingCache::new(10, 1000);
    
    let result = cache.get("nonexistent_key");
    assert!(result.is_none());
}

#[test]
fn test_lru_cache_eviction() {
    let cache = LruEmbeddingCache::new(3, 1000); // Small cache
    
    // Fill cache
    cache.insert("key1".to_string(), vec![1.0]);
    cache.insert("key2".to_string(), vec![2.0]);
    cache.insert("key3".to_string(), vec![3.0]);
    
    assert_eq!(cache.current_size(), 3);
    
    // Add one more to trigger eviction
    cache.insert("key4".to_string(), vec![4.0]);
    
    assert_eq!(cache.current_size(), 3);
    
    // key1 should be evicted (least recently used)
    assert!(cache.get("key1").is_none());
    
    // Other keys should still be present
    assert!(cache.get("key2").is_some());
    assert!(cache.get("key3").is_some());
    assert!(cache.get("key4").is_some());
}

#[test]
fn test_lru_cache_update_order() {
    let cache = LruEmbeddingCache::new(3, 1000);
    
    cache.insert("key1".to_string(), vec![1.0]);
    cache.insert("key2".to_string(), vec![2.0]);
    cache.insert("key3".to_string(), vec![3.0]);
    
    // Access key1 to make it most recently used
    let _ = cache.get("key1");
    
    // Add new key - key2 should be evicted now (not key1)
    cache.insert("key4".to_string(), vec![4.0]);
    
    assert!(cache.get("key1").is_some()); // Still present
    assert!(cache.get("key2").is_none());  // Evicted
    assert!(cache.get("key3").is_some()); // Still present
    assert!(cache.get("key4").is_some()); // New entry
}

#[test]
fn test_lru_cache_clear() {
    let cache = LruEmbeddingCache::new(10, 1000);
    
    cache.insert("key1".to_string(), vec![1.0]);
    cache.insert("key2".to_string(), vec![2.0]);
    
    assert_eq!(cache.current_size(), 2);
    
    cache.clear();
    
    assert_eq!(cache.current_size(), 0);
    assert!(cache.is_empty());
    assert!(cache.get("key1").is_none());
    assert!(cache.get("key2").is_none());
}

#[test]
fn test_lru_cache_contains() {
    let cache = LruEmbeddingCache::new(10, 1000);
    
    cache.insert("exists".to_string(), vec![1.0]);
    
    assert!(cache.contains_key("exists"));
    assert!(!cache.contains_key("not_exists"));
}

#[test]
fn test_lru_cache_remove() {
    let cache = LruEmbeddingCache::new(10, 1000);
    
    cache.insert("key1".to_string(), vec![1.0]);
    cache.insert("key2".to_string(), vec![2.0]);
    
    assert_eq!(cache.current_size(), 2);
    
    let removed = cache.remove("key1");
    assert!(removed.is_some());
    assert_eq!(removed.unwrap(), vec![1.0]);
    
    assert_eq!(cache.current_size(), 1);
    assert!(!cache.contains_key("key1"));
    assert!(cache.contains_key("key2"));
}

#[test]
fn test_lru_cache_memory_limit() {
    // Each float is 4 bytes, so 10 floats = 40 bytes per embedding
    let cache = LruEmbeddingCache::new(1000, 100); // 100 byte limit
    
    let large_embedding = vec![0.0; 10]; // 40 bytes
    
    // Should fit 2 embeddings (80 bytes total)
    cache.insert("key1".to_string(), large_embedding.clone());
    cache.insert("key2".to_string(), large_embedding.clone());
    
    assert_eq!(cache.current_size(), 2);
    
    // Third embedding should trigger eviction
    cache.insert("key3".to_string(), large_embedding.clone());
    
    // Should still have room for 2 embeddings
    assert!(cache.current_size() <= 2);
}

#[test]
fn test_lru_cache_stats() {
    let cache = LruEmbeddingCache::new(10, 1000);
    
    // Initial stats
    let (hits, misses, total) = cache.stats();
    assert_eq!(hits, 0);
    assert_eq!(misses, 0);
    assert_eq!(total, 0);
    
    // Insert and access
    cache.insert("key1".to_string(), vec![1.0]);
    let _ = cache.get("key1"); // Hit
    let _ = cache.get("key2"); // Miss
    
    let (hits, misses, total) = cache.stats();
    assert_eq!(hits, 1);
    assert_eq!(misses, 1);
    assert_eq!(total, 2);
}

#[test]
fn test_lru_cache_concurrent_access() {
    use std::thread;
    use std::sync::Arc;
    
    let cache = Arc::new(LruEmbeddingCache::new(100, 10000));
    let mut handles = vec![];
    
    // Spawn multiple threads to access cache concurrently
    for i in 0..10 {
        let cache_clone = Arc::clone(&cache);
        let handle = thread::spawn(move || {
            let key = format!("key_{}", i);
            let embedding = vec![i as f32; 100];
            
            // Insert
            cache_clone.insert(key.clone(), embedding.clone());
            
            // Retrieve
            let retrieved = cache_clone.get(&key);
            assert!(retrieved.is_some());
        });
        handles.push(handle);
    }
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    // Cache should have entries from all threads
    assert!(cache.current_size() > 0);
}

#[test]
fn test_lru_cache_hit_rate() {
    let cache = LruEmbeddingCache::new(10, 1000);
    
    // Insert some data
    for i in 0..5 {
        cache.insert(format!("key_{}", i), vec![i as f32]);
    }
    
    // Access existing keys (hits)
    for i in 0..5 {
        let _ = cache.get(&format!("key_{}", i));
    }
    
    // Access non-existent keys (misses)
    for i in 5..10 {
        let _ = cache.get(&format!("key_{}", i));
    }
    
    let (hits, misses, total) = cache.stats();
    assert_eq!(hits, 5);
    assert_eq!(misses, 5);
    assert_eq!(total, 10);
    
    let hit_rate = cache.hit_rate();
    assert!((hit_rate - 0.5).abs() < 0.01); // Should be 50%
}

#[test]
fn test_lru_cache_memory_usage() {
    let cache = LruEmbeddingCache::new(10, 1000);
    
    let initial_memory = cache.memory_usage();
    assert_eq!(initial_memory, 0);
    
    // Add some data
    cache.insert("key1".to_string(), vec![1.0, 2.0, 3.0]); // 12 bytes
    
    let memory_after_insert = cache.memory_usage();
    assert!(memory_after_insert > initial_memory);
    assert!(memory_after_insert >= 12); // At least the size of the embedding
}

#[test]
fn test_lru_cache_eviction_policy() {
    let cache = LruEmbeddingCache::new(3, 1000);
    
    // Fill cache in order
    cache.insert("first".to_string(), vec![1.0]);
    cache.insert("second".to_string(), vec![2.0]);
    cache.insert("third".to_string(), vec![3.0]);
    
    // Access second to make it more recently used
    let _ = cache.get("second");
    
    // Access first to make it most recently used
    let _ = cache.get("first");
    
    // Now order should be: third (oldest), second, first (newest)
    
    // Insert new item - should evict "third"
    cache.insert("fourth".to_string(), vec![4.0]);
    
    assert!(cache.get("third").is_none());   // Evicted
    assert!(cache.get("second").is_some());  // Still there
    assert!(cache.get("first").is_some());   // Still there
    assert!(cache.get("fourth").is_some());  // New item
}

#[test]
fn test_lru_cache_large_embeddings() {
    let cache = LruEmbeddingCache::new(5, 10000); // 10KB limit
    
    // Large embeddings (1000 floats = 4KB each)
    let large_embedding = vec![1.0; 1000];
    
    cache.insert("large1".to_string(), large_embedding.clone());
    cache.insert("large2".to_string(), large_embedding.clone());
    
    // Should fit 2 large embeddings (8KB total)
    assert_eq!(cache.current_size(), 2);
    
    // Third should trigger memory-based eviction
    cache.insert("large3".to_string(), large_embedding.clone());
    
    // Should still be within memory limit
    assert!(cache.memory_usage() <= 10000);
}

#[test]
fn test_lru_cache_string_keys() {
    let cache = LruEmbeddingCache::new(10, 1000);
    
    let keys = vec![
        "simple_key",
        "key_with_underscores",
        "key-with-dashes",
        "KeyWithCamelCase",
        "key.with.dots",
        "key with spaces",
        "键_unicode_key",
        "key_123_numbers",
    ];
    
    // Insert with various key formats
    for (i, key) in keys.iter().enumerate() {
        cache.insert(key.to_string(), vec![i as f32]);
    }
    
    // All should be retrievable
    for (i, key) in keys.iter().enumerate() {
        let result = cache.get(key);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), vec![i as f32]);
    }
}

#[test]
fn test_lru_cache_edge_cases() {
    // Zero capacity cache
    let zero_cache = LruEmbeddingCache::new(0, 1000);
    zero_cache.insert("key".to_string(), vec![1.0]);
    assert_eq!(zero_cache.current_size(), 0);
    assert!(zero_cache.get("key").is_none());
    
    // Zero memory limit
    let zero_memory_cache = LruEmbeddingCache::new(10, 0);
    zero_memory_cache.insert("key".to_string(), vec![1.0]);
    assert_eq!(zero_memory_cache.current_size(), 0);
    
    // Empty embedding
    let cache = LruEmbeddingCache::new(10, 1000);
    cache.insert("empty".to_string(), vec![]);
    assert!(cache.get("empty").is_some());
    assert_eq!(cache.get("empty").unwrap(), vec![]);
}