use memory::resource_manager::*;
use memory::types::Layer;

#[test]
fn test_resource_config_creation() {
    let config = ResourceConfig {
        max_memory_mb: 1024,
        max_records_per_layer: 10000,
        cache_size_mb: 256,
        flush_interval_seconds: 300,
        compaction_threshold: 0.8,
        enable_background_tasks: true,
    };
    
    assert_eq!(config.max_memory_mb, 1024);
    assert_eq!(config.max_records_per_layer, 10000);
    assert_eq!(config.cache_size_mb, 256);
    assert_eq!(config.flush_interval_seconds, 300);
    assert!((config.compaction_threshold - 0.8).abs() < 0.001);
    assert!(config.enable_background_tasks);
}

#[test]
fn test_resource_config_default() {
    let config = ResourceConfig::default();
    
    assert!(config.max_memory_mb > 0);
    assert!(config.max_records_per_layer > 0);
    assert!(config.cache_size_mb > 0);
    assert!(config.flush_interval_seconds > 0);
    assert!(config.compaction_threshold > 0.0);
    assert!(config.compaction_threshold <= 1.0);
}

#[test]
fn test_resource_config_validation() {
    let mut config = ResourceConfig::default();
    
    // Valid config should validate
    assert!(config.validate().is_ok());
    
    // Invalid memory
    config.max_memory_mb = 0;
    assert!(config.validate().is_err());
    
    // Reset and test invalid cache
    config = ResourceConfig::default();
    config.cache_size_mb = 0;
    assert!(config.validate().is_err());
    
    // Reset and test invalid threshold
    config = ResourceConfig::default();
    config.compaction_threshold = 1.5; // > 1.0
    assert!(config.validate().is_err());
    
    config.compaction_threshold = -0.1; // < 0.0
    assert!(config.validate().is_err());
}

#[test]
fn test_memory_usage_tracking() {
    let mut usage = MemoryUsage::new();
    
    assert_eq!(usage.total_bytes(), 0);
    assert_eq!(usage.get_layer_usage(&Layer::Interact), 0);
    assert_eq!(usage.get_layer_usage(&Layer::Insights), 0);
    assert_eq!(usage.get_layer_usage(&Layer::Assets), 0);
    
    // Add usage
    usage.add_usage(Layer::Interact, 1024);
    usage.add_usage(Layer::Insights, 2048);
    usage.add_usage(Layer::Assets, 4096);
    
    assert_eq!(usage.total_bytes(), 7168);
    assert_eq!(usage.get_layer_usage(&Layer::Interact), 1024);
    assert_eq!(usage.get_layer_usage(&Layer::Insights), 2048);
    assert_eq!(usage.get_layer_usage(&Layer::Assets), 4096);
}

#[test]
fn test_memory_usage_remove() {
    let mut usage = MemoryUsage::new();
    
    usage.add_usage(Layer::Interact, 2048);
    assert_eq!(usage.get_layer_usage(&Layer::Interact), 2048);
    
    usage.remove_usage(Layer::Interact, 1024);
    assert_eq!(usage.get_layer_usage(&Layer::Interact), 1024);
    
    // Remove more than available - should clamp to 0
    usage.remove_usage(Layer::Interact, 2048);
    assert_eq!(usage.get_layer_usage(&Layer::Interact), 0);
}

#[test]
fn test_memory_usage_percentage() {
    let mut usage = MemoryUsage::new();
    usage.add_usage(Layer::Interact, 512 * 1024 * 1024); // 512 MB
    
    let limit = 1024 * 1024 * 1024; // 1 GB
    let percentage = usage.percentage_of_limit(limit);
    
    assert!((percentage - 50.0).abs() < 0.1); // Should be ~50%
}

#[test]
fn test_memory_usage_clear_layer() {
    let mut usage = MemoryUsage::new();
    
    usage.add_usage(Layer::Interact, 1024);
    usage.add_usage(Layer::Insights, 2048);
    
    usage.clear_layer(Layer::Interact);
    
    assert_eq!(usage.get_layer_usage(&Layer::Interact), 0);
    assert_eq!(usage.get_layer_usage(&Layer::Insights), 2048);
    assert_eq!(usage.total_bytes(), 2048);
}

#[test]
fn test_adaptive_resource_manager_creation() {
    let config = ResourceConfig::default();
    let manager = AdaptiveResourceManager::new(config);
    
    assert!(manager.is_healthy());
    assert!(!manager.needs_cleanup());
}

#[test]
fn test_adaptive_resource_manager_usage_tracking() {
    let config = ResourceConfig::default();
    let mut manager = AdaptiveResourceManager::new(config);
    
    let initial_usage = manager.current_memory_usage();
    assert_eq!(initial_usage, 0);
    
    // Track some usage
    manager.track_allocation(Layer::Interact, 1024);
    manager.track_allocation(Layer::Insights, 2048);
    
    let updated_usage = manager.current_memory_usage();
    assert_eq!(updated_usage, 3072);
}

#[test]
fn test_adaptive_resource_manager_cleanup_threshold() {
    let mut config = ResourceConfig::default();
    config.max_memory_mb = 1; // 1 MB limit
    config.compaction_threshold = 0.5; // 50% threshold
    
    let mut manager = AdaptiveResourceManager::new(config);
    
    // Add memory usage below threshold
    manager.track_allocation(Layer::Interact, 256 * 1024); // 256 KB
    assert!(!manager.needs_cleanup());
    
    // Add more to exceed threshold
    manager.track_allocation(Layer::Insights, 512 * 1024); // 512 KB more = 768 KB total
    assert!(manager.needs_cleanup()); // Should exceed 50% of 1MB
}

#[test]
fn test_adaptive_resource_manager_health_check() {
    let mut config = ResourceConfig::default();
    config.max_memory_mb = 1; // 1 MB limit
    
    let mut manager = AdaptiveResourceManager::new(config);
    
    assert!(manager.is_healthy());
    
    // Add memory usage near limit
    manager.track_allocation(Layer::Interact, 900 * 1024); // 900 KB
    assert!(manager.is_healthy());
    
    // Exceed limit
    manager.track_allocation(Layer::Insights, 200 * 1024); // 200 KB more = 1.1 MB total
    assert!(!manager.is_healthy()); // Should be unhealthy
}

#[test]
fn test_adaptive_resource_manager_deallocation() {
    let config = ResourceConfig::default();
    let mut manager = AdaptiveResourceManager::new(config);
    
    manager.track_allocation(Layer::Interact, 1024);
    manager.track_allocation(Layer::Insights, 2048);
    
    assert_eq!(manager.current_memory_usage(), 3072);
    
    manager.track_deallocation(Layer::Interact, 512);
    assert_eq!(manager.current_memory_usage(), 2560);
    
    manager.track_deallocation(Layer::Insights, 2048);
    assert_eq!(manager.current_memory_usage(), 512);
}

#[test]
fn test_adaptive_resource_manager_cleanup_suggestions() {
    let mut config = ResourceConfig::default();
    config.max_memory_mb = 2; // 2 MB limit
    
    let mut manager = AdaptiveResourceManager::new(config);
    
    // Fill different layers with different amounts
    manager.track_allocation(Layer::Interact, 512 * 1024);  // 512 KB
    manager.track_allocation(Layer::Insights, 768 * 1024);  // 768 KB
    manager.track_allocation(Layer::Assets, 1024 * 1024);   // 1 MB
    
    let suggestions = manager.get_cleanup_suggestions();
    
    // Should suggest cleaning up the layer with most usage first (Assets)
    assert!(!suggestions.is_empty());
    
    // Verify suggestions make sense
    for suggestion in &suggestions {
        assert!(suggestion.estimated_savings > 0);
        assert!(!suggestion.layer_name.is_empty());
        assert!(!suggestion.description.is_empty());
    }
}

#[test]
fn test_cleanup_suggestion_priority() {
    let suggestion1 = CleanupSuggestion {
        layer_name: "Layer1".to_string(),
        estimated_savings: 1000,
        urgency: CleanupUrgency::High,
        description: "High priority cleanup".to_string(),
    };
    
    let suggestion2 = CleanupSuggestion {
        layer_name: "Layer2".to_string(),
        estimated_savings: 2000,
        urgency: CleanupUrgency::Medium,
        description: "Medium priority cleanup".to_string(),
    };
    
    let suggestion3 = CleanupSuggestion {
        layer_name: "Layer3".to_string(),
        estimated_savings: 500,
        urgency: CleanupUrgency::Low,
        description: "Low priority cleanup".to_string(),
    };
    
    let mut suggestions = vec![suggestion3, suggestion1, suggestion2];
    
    // Sort by priority (High > Medium > Low, then by savings)
    suggestions.sort_by(|a, b| {
        match (&a.urgency, &b.urgency) {
            (CleanupUrgency::High, CleanupUrgency::High) => b.estimated_savings.cmp(&a.estimated_savings),
            (CleanupUrgency::High, _) => std::cmp::Ordering::Less,
            (_, CleanupUrgency::High) => std::cmp::Ordering::Greater,
            (CleanupUrgency::Medium, CleanupUrgency::Medium) => b.estimated_savings.cmp(&a.estimated_savings),
            (CleanupUrgency::Medium, _) => std::cmp::Ordering::Less,
            (_, CleanupUrgency::Medium) => std::cmp::Ordering::Greater,
            (CleanupUrgency::Low, CleanupUrgency::Low) => b.estimated_savings.cmp(&a.estimated_savings),
        }
    });
    
    // First should be high priority
    assert!(matches!(suggestions[0].urgency, CleanupUrgency::High));
    // Second should be medium priority with higher savings
    assert!(matches!(suggestions[1].urgency, CleanupUrgency::Medium));
    // Third should be low priority
    assert!(matches!(suggestions[2].urgency, CleanupUrgency::Low));
}

#[test]
fn test_resource_manager_statistics() {
    let config = ResourceConfig::default();
    let mut manager = AdaptiveResourceManager::new(config);
    
    let initial_stats = manager.get_statistics();
    assert_eq!(initial_stats.total_memory_usage, 0);
    assert_eq!(initial_stats.layer_usage.len(), 3); // Three layers
    
    // Add some usage
    manager.track_allocation(Layer::Interact, 1024);
    manager.track_allocation(Layer::Insights, 2048);
    
    let updated_stats = manager.get_statistics();
    assert_eq!(updated_stats.total_memory_usage, 3072);
    assert_eq!(updated_stats.layer_usage[&Layer::Interact], 1024);
    assert_eq!(updated_stats.layer_usage[&Layer::Insights], 2048);
    assert_eq!(updated_stats.layer_usage[&Layer::Assets], 0);
}

#[test]
fn test_resource_manager_reset() {
    let config = ResourceConfig::default();
    let mut manager = AdaptiveResourceManager::new(config);
    
    // Add some usage
    manager.track_allocation(Layer::Interact, 1024);
    manager.track_allocation(Layer::Insights, 2048);
    
    assert_eq!(manager.current_memory_usage(), 3072);
    
    // Reset
    manager.reset();
    
    assert_eq!(manager.current_memory_usage(), 0);
    assert!(manager.is_healthy());
    assert!(!manager.needs_cleanup());
}

#[test]
fn test_memory_usage_overflow_protection() {
    let mut usage = MemoryUsage::new();
    
    // Add large amounts that might overflow
    usage.add_usage(Layer::Interact, u64::MAX / 2);
    usage.add_usage(Layer::Insights, u64::MAX / 2);
    
    // Should handle gracefully without overflow
    let total = usage.total_bytes();
    assert!(total > 0);
    
    // Remove usage
    usage.remove_usage(Layer::Interact, u64::MAX / 4);
    let new_total = usage.total_bytes();
    assert!(new_total < total);
}

#[test]
fn test_resource_config_clone() {
    let config = ResourceConfig {
        max_memory_mb: 512,
        max_records_per_layer: 5000,
        cache_size_mb: 128,
        flush_interval_seconds: 600,
        compaction_threshold: 0.7,
        enable_background_tasks: false,
    };
    
    let cloned = config.clone();
    
    assert_eq!(config.max_memory_mb, cloned.max_memory_mb);
    assert_eq!(config.max_records_per_layer, cloned.max_records_per_layer);
    assert_eq!(config.cache_size_mb, cloned.cache_size_mb);
    assert_eq!(config.flush_interval_seconds, cloned.flush_interval_seconds);
    assert!((config.compaction_threshold - cloned.compaction_threshold).abs() < 0.001);
    assert_eq!(config.enable_background_tasks, cloned.enable_background_tasks);
}

#[test]
fn test_cleanup_urgency_ordering() {
    assert!(CleanupUrgency::High > CleanupUrgency::Medium);
    assert!(CleanupUrgency::Medium > CleanupUrgency::Low);
    assert!(CleanupUrgency::High > CleanupUrgency::Low);
    
    assert_eq!(CleanupUrgency::High, CleanupUrgency::High);
    assert_eq!(CleanupUrgency::Medium, CleanupUrgency::Medium);
    assert_eq!(CleanupUrgency::Low, CleanupUrgency::Low);
}

#[test]
fn test_adaptive_resource_manager_concurrent_access() {
    use std::sync::Arc;
    use std::thread;
    
    let config = ResourceConfig::default();
    let manager = Arc::new(std::sync::Mutex::new(AdaptiveResourceManager::new(config)));
    let mut handles = vec![];
    
    // Spawn multiple threads to modify usage concurrently
    for i in 0..10 {
        let manager_clone = Arc::clone(&manager);
        let handle = thread::spawn(move || {
            let mut mg = manager_clone.lock().unwrap();
            mg.track_allocation(Layer::Interact, 100 * i);
            mg.track_allocation(Layer::Insights, 200 * i);
        });
        handles.push(handle);
    }
    
    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }
    
    // Verify final state
    let final_manager = manager.lock().unwrap();
    assert!(final_manager.current_memory_usage() > 0);
}