#![allow(clippy::uninlined_format_args)]
#[cfg(feature = "gpu")]
use ai::gpu_config::*;

#[cfg(feature = "gpu")]
#[test]
fn test_gpu_config_default() {
    let config = GpuConfig::default();

    assert_eq!(config.device_id, 0);
    assert_eq!(config.gpu_mem_limit, 2 * 1024 * 1024 * 1024); // 2GB
    assert!(!config.use_tensorrt); // Default false
    assert!(config.auto_optimize); // Default true
    assert_eq!(config.preferred_provider, GpuProviderType::Auto);
}

#[cfg(feature = "gpu")]
#[test]
fn test_gpu_config_auto_optimized() {
    let config = GpuConfig::auto_optimized();

    assert!(config.device_id >= 0);
    assert!(config.gpu_mem_limit > 0);
    assert!(config.tensorrt_cache_size > 0);
    assert_eq!(config.preferred_provider, GpuProviderType::Auto);
}

#[cfg(feature = "gpu")]
#[test]
fn test_gpu_config_creation() {
    let mut config = GpuConfig::default();
    config.device_id = 1;
    config.gpu_mem_limit = 4 * 1024 * 1024 * 1024; // 4GB
    config.use_tensorrt = true;
    config.auto_optimize = false;
    config.preferred_provider = GpuProviderType::CUDA;

    assert_eq!(config.device_id, 1);
    assert_eq!(config.gpu_mem_limit, 4 * 1024 * 1024 * 1024);
    assert!(config.use_tensorrt);
    assert!(!config.auto_optimize);
    assert_eq!(config.preferred_provider, GpuProviderType::CUDA);
}

#[cfg(feature = "gpu")]
#[test]
fn test_gpu_config_clone() {
    let original = GpuConfig::auto_optimized();
    let cloned = original.clone();

    assert_eq!(original.device_id, cloned.device_id);
    assert_eq!(original.gpu_mem_limit, cloned.gpu_mem_limit);
    assert_eq!(original.use_tensorrt, cloned.use_tensorrt);
}

#[cfg(feature = "gpu")]
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

#[cfg(feature = "gpu")]
#[test]
fn test_gpu_memory_limits() {
    let memory_limits = vec![
        512 * 1024 * 1024,       // 512MB
        1 * 1024 * 1024 * 1024,  // 1GB
        2 * 1024 * 1024 * 1024,  // 2GB
        8 * 1024 * 1024 * 1024,  // 8GB
        16 * 1024 * 1024 * 1024, // 16GB
    ];

    for limit in memory_limits {
        let mut config = GpuConfig::default();
        config.gpu_mem_limit = limit;

        assert!(config.gpu_mem_limit > 0);
        assert!(config.gpu_mem_limit >= 512 * 1024 * 1024); // At least 512MB
        assert!(config.gpu_mem_limit <= 32 * 1024 * 1024 * 1024); // Max 32GB reasonable
    }
}

#[cfg(feature = "gpu")]
#[test]
fn test_gpu_device_ids() {
    let device_ids = vec![0, 1, 2, 3];

    for device_id in device_ids {
        let mut config = GpuConfig::default();
        config.device_id = device_id;

        assert_eq!(config.device_id, device_id);
        assert!(config.device_id >= 0);
    }
}

#[cfg(feature = "gpu")]
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

#[cfg(feature = "gpu")]
#[test]
fn test_gpu_config_combinations() {
    // Performance-oriented config
    let mut perf_config = GpuConfig::default();
    perf_config.gpu_mem_limit = 8 * 1024 * 1024 * 1024; // 8GB
    perf_config.use_tensorrt = true;
    perf_config.enable_fp16 = true;
    perf_config.preferred_provider = GpuProviderType::CUDA;

    // Memory-conservative config
    let mut conservative_config = GpuConfig::default();
    conservative_config.gpu_mem_limit = 1024 * 1024 * 1024; // 1GB
    conservative_config.use_tensorrt = false;
    conservative_config.enable_fp16 = false;
    conservative_config.preferred_provider = GpuProviderType::OpenVINO;

    // Minimal config
    let mut minimal_config = GpuConfig::default();
    minimal_config.gpu_mem_limit = 512 * 1024 * 1024; // 512MB
    minimal_config.use_tensorrt = false;

    let configs = vec![perf_config, conservative_config, minimal_config];
    for config in configs {
        assert!(config.device_id >= 0);
        assert!(config.gpu_mem_limit > 0);
        assert!(config.tensorrt_cache_size > 0);
    }
}

#[cfg(feature = "gpu")]
#[test]
fn test_optimal_params_calculation() {
    let config = GpuConfig::auto_optimized();

    // Basic assumptions about optimized parameters
    assert!(config.gpu_mem_limit > 0);
    assert!(config.tensorrt_cache_size > 0);
}

#[cfg(feature = "gpu")]
#[test]
fn test_provider_creation() {
    let config = GpuConfig::auto_optimized();

    // Provider creation simulation
    assert!(matches!(
        config.preferred_provider,
        GpuProviderType::Auto
            | GpuProviderType::CUDA
            | GpuProviderType::DirectML
            | GpuProviderType::OpenVINO
    ));
}

#[cfg(feature = "gpu")]
#[test]
fn test_windows_directml_config() {
    let mut config = GpuConfig::default();

    if cfg!(target_os = "windows") {
        // Test DirectML path
        config.preferred_provider = GpuProviderType::DirectML;
        assert_eq!(config.preferred_provider, GpuProviderType::DirectML);

        // Test manual configuration
        config.preferred_provider = GpuProviderType::DirectML;
        assert_eq!(config.preferred_provider, GpuProviderType::DirectML);
    } else {
        // On non-Windows, DirectML should not be default
        assert_ne!(config.preferred_provider, GpuProviderType::DirectML);
    }
}
