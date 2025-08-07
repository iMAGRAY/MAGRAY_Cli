#![no_main]

use libfuzzer_sys::fuzz_target;
use arbitrary::Arbitrary;
use serde_json;
use ai::config::EmbeddingConfig;
use memory::hnsw_index::HnswConfig;
use common::config_base::*;

#[derive(Debug, Arbitrary)]
struct ConfigFuzzInput {
    config_type: ConfigType,
    json_data: String,
    yaml_data: String,
    toml_data: String,
}

#[derive(Debug, Arbitrary)]
enum ConfigType {
    Embedding,
    Hnsw,
    Batch,
    Cache,
    Network,
    Gpu,
}

fuzz_target!(|input: ConfigFuzzInput| {
    // Test JSON parsing
    test_json_parsing(&input.config_type, &input.json_data);
    
    // Test YAML parsing (if enabled)
    test_yaml_parsing(&input.config_type, &input.yaml_data);
    
    // Test TOML parsing
    test_toml_parsing(&input.config_type, &input.toml_data);
    
    // Test configuration validation
    test_config_validation(&input.config_type, &input.json_data);
});

fn test_json_parsing(config_type: &ConfigType, json_data: &str) {
    match config_type {
        ConfigType::Embedding => {
            // Test EmbeddingConfig parsing
            let _ = serde_json::from_str::<EmbeddingConfig>(json_data);
            
            // Test with wrapper objects
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_data) {
                let _ = serde_json::from_value::<EmbeddingConfig>(value);
            }
        }
        ConfigType::Hnsw => {
            let _ = serde_json::from_str::<HnswConfig>(json_data);
        }
        ConfigType::Batch => {
            let _ = serde_json::from_str::<BatchConfigBase>(json_data);
        }
        ConfigType::Cache => {
            let _ = serde_json::from_str::<CacheConfigBase>(json_data);
        }
        ConfigType::Network => {
            let _ = serde_json::from_str::<NetworkConfigBase>(json_data);
        }
        ConfigType::Gpu => {
            let _ = serde_json::from_str::<GpuConfigBase>(json_data);
        }
    }
}

fn test_yaml_parsing(_config_type: &ConfigType, _yaml_data: &str) {
    // YAML parsing would go here if serde_yaml is available
    // For now, skip to avoid adding more dependencies to fuzz target
}

fn test_toml_parsing(config_type: &ConfigType, toml_data: &str) {
    // Test basic TOML structure parsing
    if let Ok(parsed) = toml_data.parse::<toml::Value>() {
        // Try to convert to JSON and then to our config types
        if let Ok(json_str) = serde_json::to_string(&parsed) {
            test_json_parsing(config_type, &json_str);
        }
    }
}

fn test_config_validation(config_type: &ConfigType, json_data: &str) {
    match config_type {
        ConfigType::Embedding => {
            if let Ok(config) = serde_json::from_str::<EmbeddingConfig>(json_data) {
                // Test config validation methods if they exist
                test_embedding_config_invariants(&config);
            }
        }
        ConfigType::Hnsw => {
            if let Ok(config) = serde_json::from_str::<HnswConfig>(json_data) {
                test_hnsw_config_invariants(&config);
            }
        }
        ConfigType::Batch => {
            if let Ok(config) = serde_json::from_str::<BatchConfigBase>(json_data) {
                test_batch_config_invariants(&config);
            }
        }
        ConfigType::Cache => {
            if let Ok(config) = serde_json::from_str::<CacheConfigBase>(json_data) {
                test_cache_config_invariants(&config);
            }
        }
        ConfigType::Network => {
            if let Ok(config) = serde_json::from_str::<NetworkConfigBase>(json_data) {
                test_network_config_invariants(&config);
            }
        }
        ConfigType::Gpu => {
            if let Ok(config) = serde_json::from_str::<GpuConfigBase>(json_data) {
                test_gpu_config_invariants(&config);
            }
        }
    }
}

fn test_embedding_config_invariants(config: &EmbeddingConfig) {
    // Test that config values are within reasonable bounds
    assert!(config.batch_size > 0, "Batch size must be positive");
    assert!(config.batch_size <= 10000, "Batch size must be reasonable");
    
    assert!(config.max_length > 0, "Max length must be positive");
    assert!(config.max_length <= 100000, "Max length must be reasonable");
    
    // Model name should not be empty
    assert!(!config.model_name.is_empty(), "Model name should not be empty");
    
    // Embedding dimension should be reasonable
    if let Some(embedding_dim) = config.embedding_dim {
        assert!(embedding_dim > 0, "Embedding dimension must be positive");
        assert!(embedding_dim <= 100000, "Embedding dimension must be reasonable");
    }
}

fn test_hnsw_config_invariants(config: &HnswConfig) {
    assert!(config.max_connections > 0, "Max connections must be positive");
    assert!(config.max_connections <= 1000, "Max connections must be reasonable");
    
    assert!(config.ef_construction > 0, "EF construction must be positive");
    assert!(config.ef_construction <= 10000, "EF construction must be reasonable");
    
    assert!(config.ef_search > 0, "EF search must be positive");
    assert!(config.ef_search <= 10000, "EF search must be reasonable");
    
    assert!(config.max_layers > 0, "Max layers must be positive");
    assert!(config.max_layers <= 100, "Max layers must be reasonable");
    
    assert!(config.dimension > 0, "Dimension must be positive");
    assert!(config.dimension <= 100000, "Dimension must be reasonable");
    
    assert!(config.max_elements > 0, "Max elements must be positive");
}

fn test_batch_config_invariants(config: &BatchConfigBase) {
    assert!(config.batch_size > 0, "Batch size must be positive");
    assert!(config.max_batch_size >= config.batch_size, "Max batch size must be >= batch size");
    
    assert!(config.batch_timeout_ms > 0, "Batch timeout must be positive");
    assert!(config.batch_timeout_ms <= 3600000, "Batch timeout must be reasonable (max 1 hour)");
    
    assert!(config.flush_interval_ms > 0, "Flush interval must be positive");
    assert!(config.flush_interval_ms <= 3600000, "Flush interval must be reasonable");
}

fn test_cache_config_invariants(config: &CacheConfigBase) {
    assert!(config.max_cache_size > 0, "Cache max size must be positive");
    
    assert!(config.cache_ttl_seconds > 0, "TTL must be positive");
    assert!(config.cache_ttl_seconds <= 86400 * 30, "TTL must be reasonable (max 30 days)");
    
    // Eviction policy should be valid
    assert!(["lru", "lfu", "fifo"].contains(&config.eviction_policy.as_str()), "Eviction policy should be valid");
}

fn test_network_config_invariants(config: &NetworkConfigBase) {
    assert!(config.max_connections > 0, "Max connections must be positive");
    assert!(config.max_connections <= 10000, "Max connections must be reasonable");
    
    assert!(config.keep_alive_seconds > 0, "Keep alive must be positive");
    assert!(config.keep_alive_seconds <= 3600, "Keep alive must be reasonable (max 1 hour)");
    
    assert!(config.dns_cache_ttl_seconds > 0, "DNS cache TTL must be positive");
    assert!(config.dns_cache_ttl_seconds <= 86400, "DNS cache TTL must be reasonable (max 1 day)");
}

fn test_gpu_config_invariants(config: &GpuConfigBase) {
    // Device ID should be reasonable
    assert!(config.device_id < 100, "Device ID should be reasonable");
    
    // Memory fraction should be between 0 and 1
    assert!(config.memory_fraction > 0.0, "Memory fraction must be positive");
    assert!(config.memory_fraction <= 1.0, "Memory fraction must be <= 1.0");
    
    // Batch size multiplier should be positive
    assert!(config.batch_size_multiplier > 0.0, "Batch size multiplier must be positive");
    assert!(config.batch_size_multiplier <= 10.0, "Batch size multiplier must be reasonable");
}

// Additional fuzz target for complex config compositions
#[derive(Debug, Arbitrary)]
struct ConfigCompositionInput {
    configs: Vec<ConfigEntry>,
}

#[derive(Debug, Arbitrary)]
enum ConfigEntry {
    Embedding(String), // JSON string
    Hnsw(String),
    Batch(String),
    Network(String),
}

fuzz_target!(|input: ConfigCompositionInput| {
    // Test loading multiple configs together
    let mut valid_configs = 0;
    let mut total_configs = 0;
    
    for config_entry in input.configs.into_iter().take(10) { // Limit for performance
        total_configs += 1;
        
        match config_entry {
            ConfigEntry::Embedding(json) => {
                if serde_json::from_str::<EmbeddingConfig>(&json).is_ok() {
                    valid_configs += 1;
                }
            }
            ConfigEntry::Hnsw(json) => {
                if serde_json::from_str::<HnswConfig>(&json).is_ok() {
                    valid_configs += 1;
                }
            }
            ConfigEntry::Batch(json) => {
                if serde_json::from_str::<BatchConfigBase>(&json).is_ok() {
                    valid_configs += 1;
                }
            }
            ConfigEntry::Network(json) => {
                if serde_json::from_str::<NetworkConfigBase>(&json).is_ok() {
                    valid_configs += 1;
                }
            }
        }
    }
    
    // Test config merging scenarios
    if total_configs > 1 {
        test_config_merging();
    }
});

fn test_config_merging() {
    // Test combining different config types
    // This would test actual config composition logic if implemented
    
    // For now, just ensure we don't panic on config interactions
    let default_embedding = EmbeddingConfig::default();
    let default_hnsw = HnswConfig::default();
    let default_batch = BatchConfigBase::default();
    
    // Ensure defaults are valid
    test_embedding_config_invariants(&default_embedding);
    test_hnsw_config_invariants(&default_hnsw);
    test_batch_config_invariants(&default_batch);
}