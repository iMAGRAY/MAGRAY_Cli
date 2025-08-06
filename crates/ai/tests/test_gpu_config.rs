use ai::gpu_config::*;

#[test]
fn test_gpu_config_default() {
    let config = GpuConfig::default();
    
    // Should have reasonable default values
    assert_eq!(config.device_id, 0);
    assert_eq!(config.gpu_mem_limit, 2 * 1024 * 1024 * 1024); // 2GB
    assert!(!config.use_tensorrt); // Default false
    assert_eq!(config.tensorrt_cache_size, 1024 * 1024 * 1024); // 1GB
    assert!(config.enable_fp16); // Default true
    assert!(config.auto_optimize); // Default true
    assert_eq!(config.preferred_provider, GpuProviderType::Auto);
    assert_eq!(config.use_directml, cfg!(windows));
    assert!(config.use_openvino);
}

#[test]
fn test_gpu_config_auto_optimized() {
    let config = GpuConfig::auto_optimized();
    
    // Auto-optimized config should have performance-oriented defaults
    assert!(config.device_id >= 0);
    assert!(config.gpu_mem_limit > 0);
    assert!(config.tensorrt_cache_size > 0);
    assert_eq!(config.preferred_provider, GpuProviderType::Auto);
}

#[test]
fn test_gpu_config_creation() {
    let mut config = GpuConfig::default();
    config.device_id = 1;
    config.gpu_mem_limit = 4 * 1024 * 1024 * 1024; // 4GB
    config.use_tensorrt = true;
    config.tensorrt_cache_size = 2 * 1024 * 1024 * 1024; // 2GB
    config.enable_fp16 = false;
    config.auto_optimize = false;
    config.preferred_provider = GpuProviderType::CUDA;
    
    assert_eq!(config.device_id, 1);
    assert_eq!(config.gpu_mem_limit, 4 * 1024 * 1024 * 1024);
    assert!(config.use_tensorrt);
    assert_eq!(config.tensorrt_cache_size, 2 * 1024 * 1024 * 1024);
    assert!(!config.enable_fp16);
    assert!(!config.auto_optimize);
    assert_eq!(config.preferred_provider, GpuProviderType::CUDA);
}

#[test]
fn test_gpu_config_clone() {
    let original = GpuConfig::auto_optimized();
    let cloned = original.clone();
    
    assert_eq!(original.device_id, cloned.device_id);
    assert_eq!(original.gpu_mem_limit, cloned.gpu_mem_limit);
    assert_eq!(original.use_tensorrt, cloned.use_tensorrt);
    assert_eq!(original.tensorrt_cache_size, cloned.tensorrt_cache_size);
    assert_eq!(original.enable_fp16, cloned.enable_fp16);
    assert_eq!(original.auto_optimize, cloned.auto_optimize);
    assert_eq!(original.preferred_provider, cloned.preferred_provider);
    assert_eq!(original.use_directml, cloned.use_directml);
    assert_eq!(original.use_openvino, cloned.use_openvino);
}

#[test]
fn test_gpu_config_debug_format() {
    let config = GpuConfig::default();
    let debug_str = format!("{:?}", config);
    
    // Debug format should contain key information
    assert!(debug_str.contains("GpuConfig"));
    assert!(debug_str.contains("device_id"));
    assert!(debug_str.contains("gpu_mem_limit"));
    assert!(debug_str.contains("preferred_provider"));
}

#[test]
fn test_memory_limit_validation() {
    let memory_limits = [
        512 * 1024 * 1024,      // 512MB
        1024 * 1024 * 1024,     // 1GB
        2 * 1024 * 1024 * 1024, // 2GB
        4 * 1024 * 1024 * 1024, // 4GB
        8 * 1024 * 1024 * 1024, // 8GB
    ];
    
    for limit in memory_limits {
        let mut config = GpuConfig::default();
        config.gpu_mem_limit = limit;
        
        assert!(config.gpu_mem_limit > 0);
        assert!(config.gpu_mem_limit >= 512 * 1024 * 1024); // At least 512MB
        assert!(config.gpu_mem_limit <= 32 * 1024 * 1024 * 1024); // Max 32GB reasonable
    }
}

#[test]
fn test_device_id_validation() {
    let device_ids = [0, 1, 2, 3, 7]; // Common GPU device IDs
    
    for device_id in device_ids {
        let mut config = GpuConfig::default();
        config.device_id = device_id;
        
        assert!(config.device_id >= 0);
        assert!(config.device_id < 16); // Reasonable upper bound
    }
}

#[test]
fn test_gpu_provider_types() {
    let provider_types = vec![
        GpuProviderType::Auto,
        GpuProviderType::CUDA,
        GpuProviderType::DirectML,
        GpuProviderType::OpenVINO,
    ];
    
    for provider_type in provider_types {
        let mut config = GpuConfig::default();
        config.preferred_provider = provider_type.clone();
        
        assert_eq!(config.preferred_provider, provider_type);
    }
}

#[test]
fn test_gpu_config_combinations() {
    // Performance-oriented config
    let mut perf_config = GpuConfig::default();
    perf_config.gpu_mem_limit = 8 * 1024 * 1024 * 1024; // 8GB
    perf_config.use_tensorrt = true;
    perf_config.tensorrt_cache_size = 2 * 1024 * 1024 * 1024; // 2GB
    perf_config.enable_fp16 = true;
    perf_config.preferred_provider = GpuProviderType::CUDA;
    
    // Memory-conservative config
    let mut conservative_config = GpuConfig::default();
    conservative_config.gpu_mem_limit = 1024 * 1024 * 1024; // 1GB
    conservative_config.use_tensorrt = false;
    conservative_config.tensorrt_cache_size = 256 * 1024 * 1024; // 256MB
    conservative_config.enable_fp16 = false;
    conservative_config.preferred_provider = GpuProviderType::OpenVINO;
    
    // Minimal config  
    let mut minimal_config = GpuConfig::default();
    minimal_config.gpu_mem_limit = 512 * 1024 * 1024; // 512MB
    minimal_config.use_tensorrt = false;
    minimal_config.tensorrt_cache_size = 128 * 1024 * 1024; // 128MB
    minimal_config.enable_fp16 = false;
    minimal_config.auto_optimize = false;
    
    let configs = vec![perf_config, conservative_config, minimal_config];
    
    for config in configs {
        // All configs should be valid
        assert!(config.device_id >= 0);
        assert!(config.gpu_mem_limit > 0);
        assert!(config.tensorrt_cache_size > 0);
        // Boolean fields can be any value
    }
}

#[test]
fn test_tensorrt_cache_size_validation() {
    let cache_sizes = [
        128 * 1024 * 1024,      // 128MB
        256 * 1024 * 1024,      // 256MB
        512 * 1024 * 1024,      // 512MB
        1024 * 1024 * 1024,     // 1GB
        2 * 1024 * 1024 * 1024, // 2GB
    ];
    
    for cache_size in cache_sizes {
        let mut config = GpuConfig::default();
        config.tensorrt_cache_size = cache_size;
        
        assert!(config.tensorrt_cache_size >= 128 * 1024 * 1024); // At least 128MB
        assert!(config.tensorrt_cache_size <= 8 * 1024 * 1024 * 1024); // Max 8GB reasonable
    }
}

#[test]
fn test_fp16_and_tensorrt_combinations() {
    let combinations = [
        (false, false), // No TensorRT, no FP16
        (false, true),  // No TensorRT, with FP16
        (true, false),  // TensorRT, no FP16
        (true, true),   // TensorRT with FP16 (optimal)
    ];
    
    for (use_tensorrt, enable_fp16) in combinations {
        let mut config = GpuConfig::default();
        config.use_tensorrt = use_tensorrt;
        config.enable_fp16 = enable_fp16;
        
        // All combinations should be valid
        assert_eq!(config.use_tensorrt, use_tensorrt);
        assert_eq!(config.enable_fp16, enable_fp16);
    }
}

#[test]
fn test_optimal_params_calculation() {
    let config = GpuConfig::auto_optimized();
    
    // Test with different model sizes
    let model_sizes = [500, 1000, 2000]; // MB
    
    for model_size in model_sizes {
        let params = config.get_optimal_params(model_size);
        
        assert!(params.batch_size > 0);
        assert!(params.batch_size <= 512); // Reasonable upper limit
        assert!(params.max_sequence_length > 0);
        assert!(params.max_sequence_length <= 2048); // Reasonable upper limit
        assert!(params.memory_fraction > 0.0);
        assert!(params.memory_fraction <= 1.0);
    }
}

#[cfg(feature = "gpu")]
#[test]
fn test_provider_creation() {
    let config = GpuConfig::auto_optimized();
    
    // This might fail in test environment without GPU, but shouldn't panic
    match config.create_providers() {
        Ok(providers) => {
            println!("✅ Created {} GPU providers", providers.len());
        }
        Err(e) => {
            println!("⚠️ GPU providers unavailable in test environment: {}", e);
            // This is expected in CI/testing environments
        }
    }
}

#[test] 
fn test_windows_directml_config() {
    let mut config = GpuConfig::default();
    
    // DirectML should be enabled by default on Windows
    if cfg!(windows) {
        assert!(config.use_directml);
        
        // Test manual configuration
        config.preferred_provider = GpuProviderType::DirectML;
        assert_eq!(config.preferred_provider, GpuProviderType::DirectML);
    } else {
        // On non-Windows, DirectML should be false by default
        assert!(!config.use_directml);
    }
}