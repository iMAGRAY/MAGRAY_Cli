use ai::GpuConfig;

#[test]
fn test_gpu_config_default() {
    let config = GpuConfig::default();
    
    assert!(config.gpu_mem_limit > 0);
    assert_eq!(config.device_id, 0);
    assert!(config.tensorrt_cache_size > 0);
    // enable_fp16 and auto_optimize are booleans, just test they exist
    let _ = config.enable_fp16;
    let _ = config.auto_optimize;
    let _ = config.use_tensorrt;
}

#[test]
fn test_gpu_config_auto_optimized() {
    let config = GpuConfig::auto_optimized();
    
    // Auto-optimized should have reasonable defaults
    assert!(config.gpu_mem_limit > 0);
    assert_eq!(config.device_id, 0);
    assert!(config.enable_fp16); // Should use FP16 for better performance
    assert!(config.auto_optimize); // Should be auto-optimizing
    assert!(config.tensorrt_cache_size > 0);
}

#[test]
fn test_gpu_config_clone() {
    let original = GpuConfig {
        device_id: 1,
        gpu_mem_limit: 8192,
        use_tensorrt: true,
        tensorrt_cache_size: 2048,
        enable_fp16: true,
        auto_optimize: false,
    };
    
    let cloned = original.clone();
    
    assert_eq!(original.device_id, cloned.device_id);
    assert_eq!(original.gpu_mem_limit, cloned.gpu_mem_limit);
    assert_eq!(original.use_tensorrt, cloned.use_tensorrt);
    assert_eq!(original.tensorrt_cache_size, cloned.tensorrt_cache_size);
    assert_eq!(original.enable_fp16, cloned.enable_fp16);
    assert_eq!(original.auto_optimize, cloned.auto_optimize);
}

#[test]
fn test_gpu_config_debug() {
    let config = GpuConfig {
        device_id: 2,
        gpu_mem_limit: 4096,
        use_tensorrt: false,
        tensorrt_cache_size: 1024,
        enable_fp16: false,
        auto_optimize: true,
    };
    
    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("GpuConfig"));
    assert!(debug_str.contains("2")); // device_id
    assert!(debug_str.contains("4096"));
    assert!(debug_str.contains("false")); // use_tensorrt
    assert!(debug_str.contains("1024")); // tensorrt_cache_size
}

#[test]
fn test_gpu_config_get_optimal_params() {
    let config = GpuConfig::auto_optimized();
    
    // Test with different model sizes
    let small_model_params = config.get_optimal_params(500); // 500MB model
    let large_model_params = config.get_optimal_params(2000); // 2GB model
    
    // Small model should allow larger batch size
    assert!(small_model_params.batch_size > 0);
    assert!(small_model_params.max_sequence_length > 0);
    
    // Large model might have smaller batch size
    assert!(large_model_params.batch_size > 0);
    assert!(large_model_params.max_sequence_length > 0);
    
    // Both should respect FP16 setting
    assert_eq!(small_model_params.use_fp16, config.enable_fp16);
    assert_eq!(large_model_params.use_fp16, config.enable_fp16);
}

#[test]
fn test_gpu_config_create_providers() {
    let config = GpuConfig::auto_optimized();
    
    // This might fail on systems without GPU, but should not panic
    let result = config.create_providers();
    
    // Should return either Ok with providers or Err
    match result {
        Ok(providers) => {
            // If successful, might have providers (could be empty in test environment)
            // Just check that it's a Vec
            let _ = providers.len();
        }
        Err(_) => {
            // Expected on systems without proper GPU setup
            // This is fine for testing
        }
    }
}

#[test]
fn test_gpu_config_memory_limits() {
    let mut config = GpuConfig::default();
    
    // Test different memory limits
    config.gpu_mem_limit = 1024; // 1GB
    let params_1gb = config.get_optimal_params(500);
    
    config.gpu_mem_limit = 8192; // 8GB
    let params_8gb = config.get_optimal_params(500);
    
    // Higher memory should allow larger batch sizes or similar
    assert!(params_8gb.batch_size >= params_1gb.batch_size);
}

#[test]
fn test_gpu_config_device_id() {
    let mut config = GpuConfig::default();
    
    // Test different device IDs
    for device_id in 0..4 {
        config.device_id = device_id;
        assert_eq!(config.device_id, device_id);
        
        // Should still be able to get optimal params
        let params = config.get_optimal_params(500);
        assert!(params.batch_size > 0);
    }
}

#[test]
fn test_gpu_config_fp16_toggle() {
    let mut config = GpuConfig::default();
    
    // Test FP16 enabled
    config.enable_fp16 = true;
    let params_fp16 = config.get_optimal_params(500);
    assert!(params_fp16.use_fp16);
    
    // Test FP16 disabled
    config.enable_fp16 = false;
    let params_fp32 = config.get_optimal_params(500);
    // Note: params might still use FP16 based on other factors
    // Just test that we can get params
    assert!(params_fp32.batch_size > 0);
}

#[test]
fn test_gpu_config_edge_cases() {
    let mut config = GpuConfig::default();
    
    // Test very small memory limit
    config.gpu_mem_limit = 100; // 100MB
    let params_tiny = config.get_optimal_params(50);
    assert!(params_tiny.batch_size > 0);
    assert!(params_tiny.batch_size <= 64); // Should be reasonable
    
    // Test very large model
    config.gpu_mem_limit = 16384; // 16GB
    let params_huge_model = config.get_optimal_params(8000); // 8GB model
    assert!(params_huge_model.batch_size > 0);
    assert!(params_huge_model.batch_size <= 32); // Should be conservative
    
    // Test zero model size (edge case)
    let params_zero = config.get_optimal_params(0);
    assert!(params_zero.batch_size > 0); // Should still work
}

#[test]
fn test_gpu_config_consistency() {
    let config = GpuConfig::auto_optimized();
    
    // Multiple calls should return same results
    let params1 = config.get_optimal_params(1000);
    let params2 = config.get_optimal_params(1000);
    
    assert_eq!(params1.batch_size, params2.batch_size);
    assert_eq!(params1.max_sequence_length, params2.max_sequence_length);
    assert_eq!(params1.use_fp16, params2.use_fp16);
    assert_eq!(params1.memory_fraction, params2.memory_fraction);
}

#[test]
fn test_gpu_config_tensorrt_settings() {
    let mut config = GpuConfig::default();
    
    // Test with TensorRT enabled
    config.use_tensorrt = true;
    config.tensorrt_cache_size = 4096;
    assert!(config.use_tensorrt);
    assert_eq!(config.tensorrt_cache_size, 4096);
    
    // Test with TensorRT disabled
    config.use_tensorrt = false;
    config.tensorrt_cache_size = 0;
    assert!(!config.use_tensorrt);
    assert_eq!(config.tensorrt_cache_size, 0);
    
    // Should still be able to get optimal params regardless
    let params = config.get_optimal_params(500);
    assert!(params.batch_size > 0);
}

#[test]
fn test_gpu_config_auto_optimize_settings() {
    let mut config = GpuConfig::default();
    
    // Test with auto optimization enabled
    config.auto_optimize = true;
    assert!(config.auto_optimize);
    
    // Test with auto optimization disabled
    config.auto_optimize = false;
    assert!(!config.auto_optimize);
    
    // Should still be able to get optimal params regardless
    let params = config.get_optimal_params(500);
    assert!(params.batch_size > 0);
}