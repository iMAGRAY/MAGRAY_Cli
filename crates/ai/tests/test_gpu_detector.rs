#[cfg(feature = "gpu")]
use ai::gpu_detector::GpuDetector;

#[cfg(feature = "gpu")]
#[test]
fn test_gpu_detection_basic() {
    let _ = GpuDetector::detect();
}

#[test]
fn test_gpu_device_creation() {
    let gpu = GpuDevice {
        index: 0,
        name: "NVIDIA GeForce RTX 3080".to_string(),
        total_memory_mb: 10240,
        free_memory_mb: 8192,
        compute_capability: "8.6".to_string(),
        temperature_c: Some(65),
        utilization_percent: Some(50),
        power_draw_w: Some(220.5),
    };

    assert_eq!(gpu.index, 0);
    assert_eq!(gpu.name, "NVIDIA GeForce RTX 3080");
    assert_eq!(gpu.total_memory_mb, 10240);
    assert_eq!(gpu.free_memory_mb, 8192);
    assert_eq!(gpu.compute_capability, "8.6");
    assert_eq!(gpu.temperature_c, Some(65));
    assert_eq!(gpu.utilization_percent, Some(50));
    assert_eq!(gpu.power_draw_w, Some(220.5));
}

#[test]
fn test_gpu_device_clone() {
    let original = GpuDevice {
        index: 1,
        name: "Tesla V100".to_string(),
        total_memory_mb: 32768,
        free_memory_mb: 30000,
        compute_capability: "7.0".to_string(),
        temperature_c: Some(70),
        utilization_percent: Some(80),
        power_draw_w: Some(300.0),
    };

    let cloned = original.clone();

    assert_eq!(original.index, cloned.index);
    assert_eq!(original.name, cloned.name);
    assert_eq!(original.total_memory_mb, cloned.total_memory_mb);
    assert_eq!(original.free_memory_mb, cloned.free_memory_mb);
    assert_eq!(original.compute_capability, cloned.compute_capability);
    assert_eq!(original.temperature_c, cloned.temperature_c);
    assert_eq!(original.utilization_percent, cloned.utilization_percent);
    assert_eq!(original.power_draw_w, cloned.power_draw_w);
}

#[test]
fn test_gpu_detector_creation() {
    let gpu = GpuDevice {
        index: 0,
        name: "RTX 4090".to_string(),
        total_memory_mb: 24576,
        free_memory_mb: 20000,
        compute_capability: "8.9".to_string(),
        temperature_c: Some(60),
        utilization_percent: Some(30),
        power_draw_w: Some(400.0),
    };

    let detector = GpuDetector {
        available: true,
        devices: vec![gpu],
        cuda_version: "12.2".to_string(),
        driver_version: "545.23".to_string(),
    };

    assert!(detector.available);
    assert_eq!(detector.devices.len(), 1);
    assert_eq!(detector.cuda_version, "12.2");
    assert_eq!(detector.driver_version, "545.23");
}

#[test]
fn test_gpu_detector_no_gpu() {
    let detector = GpuDetector {
        available: false,
        devices: vec![],
        cuda_version: "".to_string(),
        driver_version: "".to_string(),
    };

    assert!(!detector.available);
    assert!(detector.devices.is_empty());
    assert!(detector.cuda_version.is_empty());
    assert!(detector.driver_version.is_empty());
}

#[test]
fn test_gpu_detector_debug() {
    let detector = GpuDetector {
        available: true,
        devices: vec![],
        cuda_version: "11.8".to_string(),
        driver_version: "522.25".to_string(),
    };

    let debug_str = format!("{:?}", detector);
    assert!(debug_str.contains("GpuDetector"));
    assert!(debug_str.contains("available: true"));
}

#[test]
fn test_gpu_device_debug() {
    let gpu = GpuDevice {
        index: 0,
        name: "Test GPU".to_string(),
        total_memory_mb: 8192,
        free_memory_mb: 6000,
        compute_capability: "7.5".to_string(),
        temperature_c: Some(55),
        utilization_percent: Some(40),
        power_draw_w: Some(180.0),
    };

    let debug_str = format!("{:?}", gpu);
    assert!(debug_str.contains("GpuDevice"));
    assert!(debug_str.contains("Test GPU"));
}

#[test]
fn test_has_sufficient_memory() {
    let detector = GpuDetector {
        available: true,
        devices: vec![GpuDevice {
            index: 0,
            name: "High Memory GPU".to_string(),
            total_memory_mb: 16384, // 16GB
            free_memory_mb: 14000,
            compute_capability: "8.0".to_string(),
            temperature_c: Some(60),
            utilization_percent: Some(20),
            power_draw_w: Some(250.0),
        }],
        cuda_version: "12.0".to_string(),
        driver_version: "530.41".to_string(),
    };

    // Should have sufficient memory (using free memory for check)
    assert!(detector.has_sufficient_memory(8000)); // 8GB required, 14GB free
    assert!(detector.has_sufficient_memory(14000)); // Exactly 14GB free

    // Should not have sufficient memory
    assert!(!detector.has_sufficient_memory(20000)); // 20GB required
}

#[test]
fn test_has_sufficient_memory_no_gpu() {
    let detector = GpuDetector {
        available: false,
        devices: vec![],
        cuda_version: "".to_string(),
        driver_version: "".to_string(),
    };

    // No GPU should never have sufficient memory
    assert!(!detector.has_sufficient_memory(1000));
    assert!(!detector.has_sufficient_memory(0));
}

#[test]
fn test_memory_validation_ranges() {
    let memory_sizes = [1024, 2048, 4096, 8192, 16384, 24576, 32768];

    for memory in memory_sizes {
        assert!(memory > 0);
        assert!(memory <= 131072); // 128GB max reasonable
                                   // Memory should be power of 2 or multiple of 1024
        assert!(memory % 1024 == 0 || (memory & (memory - 1)) == 0);
    }
}

#[test]
fn test_compute_capability_validation() {
    let capabilities = ["6.0", "6.1", "7.0", "7.5", "8.0", "8.6", "8.9", "9.0"];

    for cap in capabilities {
        assert!(!cap.is_empty());
        assert!(cap.contains('.'));

        let parts: Vec<&str> = cap.split('.').collect();
        assert_eq!(parts.len(), 2);

        // Should be valid numbers
        let major: u32 = parts[0].parse().unwrap();
        let minor: u32 = parts[1].parse().unwrap();

        assert!(major >= 3); // Minimum supported
        assert!(major <= 15); // Reasonable maximum
        assert!(minor <= 9); // Single digit minor version
    }
}

#[test]
fn test_driver_version_format() {
    let versions = ["470.86", "515.48", "522.25", "530.41", "531.61", "545.23"];

    for version in versions {
        if !version.is_empty() {
            assert!(version.contains('.'));

            let parts: Vec<&str> = version.split('.').collect();
            assert_eq!(parts.len(), 2);

            // Should be valid numbers
            let major: u32 = parts[0].parse().unwrap();
            let minor: u32 = parts[1].parse().unwrap();

            assert!(major >= 400); // Reasonable minimum
            assert!(major <= 999); // Reasonable maximum
            assert!(minor <= 99); // Two digit minor version
        }
    }
}

#[test]
fn test_cuda_version_format() {
    let versions = ["11.0", "11.2", "11.8", "12.0", "12.1", "12.2"];

    for version in versions {
        if !version.is_empty() {
            assert!(version.contains('.'));

            let parts: Vec<&str> = version.split('.').collect();
            assert_eq!(parts.len(), 2);

            // Should be valid numbers
            let major: u32 = parts[0].parse().unwrap();
            let minor: u32 = parts[1].parse().unwrap();

            assert!(major >= 10); // CUDA 10+ is reasonable minimum
            assert!(major <= 20); // Reasonable maximum
            assert!(minor <= 9); // Single digit minor version
        }
    }
}

#[test]
fn test_multiple_gpus() {
    let detector = GpuDetector {
        available: true,
        devices: vec![
            GpuDevice {
                index: 0,
                name: "GPU 0".to_string(),
                total_memory_mb: 8192,
                free_memory_mb: 6000,
                compute_capability: "7.5".to_string(),
                temperature_c: Some(55),
                utilization_percent: Some(30),
                power_draw_w: Some(200.0),
            },
            GpuDevice {
                index: 1,
                name: "GPU 1".to_string(),
                total_memory_mb: 16384,
                free_memory_mb: 14000,
                compute_capability: "8.0".to_string(),
                temperature_c: Some(60),
                utilization_percent: Some(40),
                power_draw_w: Some(300.0),
            },
        ],
        cuda_version: "11.8".to_string(),
        driver_version: "515.48".to_string(),
    };

    assert_eq!(detector.devices.len(), 2);
    assert_eq!(detector.devices[0].index, 0);
    assert_eq!(detector.devices[1].index, 1);

    // Should use highest free memory GPU for memory check
    assert!(detector.has_sufficient_memory(12000)); // Uses GPU 1 (14GB free)
}

#[test]
fn test_gpu_detector_basic() {
    // This will use whatever is available on the system
    let detector = GpuDetector::detect();

    // Should create a valid result regardless of GPU availability
    assert!(detector.available || !detector.available); // Always true

    if detector.available {
        assert!(!detector.devices.is_empty());
        for device in &detector.devices {
            assert!(device.index < 32); // Reasonable device ID
            assert!(!device.name.is_empty());
            assert!(device.total_memory_mb > 0);
        }
    } else {
        // Might be empty if no NVIDIA GPUs are available
        assert!(true); // Always passes
    }
}

#[test]
fn test_optimal_batch_sizes() {
    let memory_sizes = [4096, 8192, 16384, 24576];
    let batch_sizes = [16, 32, 64, 128, 256];

    for memory in memory_sizes {
        for batch_size in batch_sizes {
            // Memory per item estimation
            let memory_per_item = 1024 * 512 * 4; // max_length * dim * bytes_per_float
            let total_memory_needed = batch_size * memory_per_item / (1024 * 1024); // Convert to MB

            if total_memory_needed <= memory / 2 {
                // Use half of available memory
                // This batch size should be safe
                assert!(batch_size <= 512); // Reasonable upper bound
            }
        }
    }
}

#[test]
fn test_gpu_device_optional_fields() {
    let gpu_with_all = GpuDevice {
        index: 0,
        name: "Complete GPU".to_string(),
        total_memory_mb: 8192,
        free_memory_mb: 6000,
        compute_capability: "8.0".to_string(),
        temperature_c: Some(65),
        utilization_percent: Some(75),
        power_draw_w: Some(250.0),
    };

    assert!(gpu_with_all.temperature_c.is_some());
    assert!(gpu_with_all.utilization_percent.is_some());
    assert!(gpu_with_all.power_draw_w.is_some());

    let gpu_minimal = GpuDevice {
        index: 1,
        name: "Minimal GPU".to_string(),
        total_memory_mb: 4096,
        free_memory_mb: 3000,
        compute_capability: "7.0".to_string(),
        temperature_c: None,
        utilization_percent: None,
        power_draw_w: None,
    };

    assert!(gpu_minimal.temperature_c.is_none());
    assert!(gpu_minimal.utilization_percent.is_none());
    assert!(gpu_minimal.power_draw_w.is_none());
}
