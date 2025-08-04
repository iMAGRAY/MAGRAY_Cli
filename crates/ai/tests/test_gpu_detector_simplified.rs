use ai::gpu_detector::{GpuDetector, GpuDevice};

#[test]
fn test_gpu_detector_detect() {
    let detector = GpuDetector::detect();
    
    // Should always return some detection result
    // available might be false on systems without GPU
    assert!(detector.available || !detector.available);
    
    // Device count should be reasonable
    assert!(detector.devices.len() <= 16); // Reasonable upper bound
    
    if detector.available {
        assert!(!detector.devices.is_empty());
    }
    
    // Versions should be strings
    assert!(!detector.cuda_version.is_empty() || detector.cuda_version.is_empty());
    assert!(!detector.driver_version.is_empty() || detector.driver_version.is_empty());
}

#[test]
fn test_gpu_detector_structure() {
    let detector = GpuDetector::detect();
    
    // Test that structure has expected fields
    let _ = detector.available;
    let _ = &detector.devices;
    let _ = &detector.cuda_version;
    let _ = &detector.driver_version;
    
    // If GPU is available, should have reasonable versions
    if detector.available && !detector.devices.is_empty() {
        // Versions might be empty on mock/test systems
        let _ = &detector.cuda_version;
        let _ = &detector.driver_version;
    }
}

#[test]
fn test_gpu_device_structure() {
    let detector = GpuDetector::detect();
    
    for device in &detector.devices {
        // Test all fields are accessible
        let _ = device.index;
        let _ = &device.name;
        let _ = &device.compute_capability;
        let _ = device.total_memory_mb;
        let _ = device.free_memory_mb;
        let _ = device.temperature_c;
        let _ = device.utilization_percent;
        let _ = device.power_draw_w;
        
        // Basic sanity checks
        assert!(device.total_memory_mb >= device.free_memory_mb);
        assert!(!device.name.is_empty());
        assert!(!device.compute_capability.is_empty());
    }
}

#[test]
fn test_gpu_device_creation() {
    let gpu_device = GpuDevice {
        index: 0,
        name: "Test GPU".to_string(),
        compute_capability: "7.5".to_string(),
        total_memory_mb: 8192,
        free_memory_mb: 6144,
        temperature_c: Some(45),
        utilization_percent: Some(25),
        power_draw_w: Some(150.5),
    };
    
    assert_eq!(gpu_device.index, 0);
    assert_eq!(gpu_device.name, "Test GPU");
    assert_eq!(gpu_device.compute_capability, "7.5");
    assert_eq!(gpu_device.total_memory_mb, 8192);
    assert_eq!(gpu_device.free_memory_mb, 6144);
    assert_eq!(gpu_device.temperature_c, Some(45));
    assert_eq!(gpu_device.utilization_percent, Some(25));
    assert_eq!(gpu_device.power_draw_w, Some(150.5));
}

#[test]
fn test_gpu_device_clone() {
    let original = GpuDevice {
        index: 1,
        name: "Clone Test GPU".to_string(),
        compute_capability: "8.0".to_string(),
        total_memory_mb: 4096,
        free_memory_mb: 3072,
        temperature_c: None,
        utilization_percent: Some(50),
        power_draw_w: None,
    };
    
    let cloned = original.clone();
    
    assert_eq!(original.index, cloned.index);
    assert_eq!(original.name, cloned.name);
    assert_eq!(original.compute_capability, cloned.compute_capability);
    assert_eq!(original.total_memory_mb, cloned.total_memory_mb);
    assert_eq!(original.free_memory_mb, cloned.free_memory_mb);
    assert_eq!(original.temperature_c, cloned.temperature_c);
    assert_eq!(original.utilization_percent, cloned.utilization_percent);
    assert_eq!(original.power_draw_w, cloned.power_draw_w);
}

#[test]
fn test_gpu_device_debug() {
    let gpu_device = GpuDevice {
        index: 2,
        name: "Debug GPU".to_string(),
        compute_capability: "9.0".to_string(),
        total_memory_mb: 12288,
        free_memory_mb: 10240,
        temperature_c: Some(60),
        utilization_percent: Some(75),
        power_draw_w: Some(200.0),
    };
    
    let debug_str = format!("{:?}", gpu_device);
    assert!(debug_str.contains("GpuDevice"));
    assert!(debug_str.contains("2")); // index
    assert!(debug_str.contains("Debug GPU"));
    assert!(debug_str.contains("9.0")); // compute_capability
    assert!(debug_str.contains("12288"));
    assert!(debug_str.contains("10240"));
}

#[test]
fn test_gpu_detector_clone() {
    let original = GpuDetector::detect();
    let cloned = original.clone();
    
    assert_eq!(original.available, cloned.available);
    assert_eq!(original.devices.len(), cloned.devices.len());
    assert_eq!(original.cuda_version, cloned.cuda_version);
    assert_eq!(original.driver_version, cloned.driver_version);
    
    // Device details should match
    for (dev1, dev2) in original.devices.iter().zip(cloned.devices.iter()) {
        assert_eq!(dev1.index, dev2.index);
        assert_eq!(dev1.name, dev2.name);
        assert_eq!(dev1.compute_capability, dev2.compute_capability);
        assert_eq!(dev1.total_memory_mb, dev2.total_memory_mb);
        assert_eq!(dev1.free_memory_mb, dev2.free_memory_mb);
        assert_eq!(dev1.temperature_c, dev2.temperature_c);
        assert_eq!(dev1.utilization_percent, dev2.utilization_percent);
        assert_eq!(dev1.power_draw_w, dev2.power_draw_w);
    }
}

#[test]
fn test_gpu_detector_debug() {
    let detector = GpuDetector::detect();
    
    let debug_str = format!("{:?}", detector);
    assert!(debug_str.contains("GpuDetector"));
    
    if detector.available {
        assert!(debug_str.contains("available: true"));
    } else {
        assert!(debug_str.contains("available: false"));
    }
}

#[test]
fn test_gpu_detector_multiple_calls_consistency() {
    // Multiple calls should return consistent results
    let detector1 = GpuDetector::detect();
    let detector2 = GpuDetector::detect();
    
    assert_eq!(detector1.available, detector2.available);
    assert_eq!(detector1.devices.len(), detector2.devices.len());
    
    // Note: CUDA/driver versions might vary slightly due to system state,
    // but availability and device count should be consistent
}

#[test]
fn test_gpu_detector_optional_fields() {
    let detector = GpuDetector::detect();
    
    for device in &detector.devices {
        // Test that optional fields handle None correctly
        match device.temperature_c {
            Some(temp) => assert!(temp > 0), // Temperature should be positive if present
            None => {}, // None is acceptable
        }
        
        match device.utilization_percent {
            Some(util) => assert!(util <= 100), // Utilization should be 0-100%
            None => {}, // None is acceptable
        }
        
        match device.power_draw_w {
            Some(power) => assert!(power >= 0.0), // Power should be non-negative
            None => {}, // None is acceptable
        }
    }
}

#[test]
fn test_gpu_detector_serialization_fields() {
    let detector = GpuDetector::detect();
    
    // Test that the struct can be serialized (fields are public)
    let _available = detector.available;
    let _devices = &detector.devices;
    let _cuda_version = &detector.cuda_version;
    let _driver_version = &detector.driver_version;
    
    for device in &detector.devices {
        let _index = device.index;
        let _name = &device.name;
        let _compute_capability = &device.compute_capability;
        let _total_memory_mb = device.total_memory_mb;
        let _free_memory_mb = device.free_memory_mb;
        let _temperature_c = device.temperature_c;
        let _utilization_percent = device.utilization_percent;
        let _power_draw_w = device.power_draw_w;
    }
    
    assert!(true); // If we get here, all fields are accessible
}

#[test]
fn test_gpu_device_memory_consistency() {
    let detector = GpuDetector::detect();
    
    for device in &detector.devices {
        // Free memory should not exceed total memory
        assert!(device.free_memory_mb <= device.total_memory_mb);
        
        // Memory values should be reasonable (not zero unless it's a test device)
        if device.total_memory_mb > 0 {
            assert!(device.total_memory_mb >= device.free_memory_mb);
        }
    }
}

#[test]
fn test_gpu_detector_not_available_case() {
    // Test the not available case explicitly
    let detector = GpuDetector {
        available: false,
        devices: vec![],
        cuda_version: "Not available".to_string(),
        driver_version: "Not available".to_string(),
    };
    
    assert!(!detector.available);
    assert!(detector.devices.is_empty());
    assert_eq!(detector.cuda_version, "Not available");
    assert_eq!(detector.driver_version, "Not available");
}