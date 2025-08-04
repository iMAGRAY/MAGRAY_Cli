use memory::{FlushConfig, PerformanceMode};
use serde_json;
use std::env;

/// Комплексные unit тесты для flush configuration системы
/// Тестирует: performance modes, конфигурация интервалов, загрузка из env, сериализация

/// Тест создания конфигураций с различными performance modes
#[test]
fn test_performance_mode_configurations() {
    println!("🧪 Тестируем конфигурации для разных performance modes");
    
    // High Performance - быстрая работа, редкие flush
    let high_perf = FlushConfig::high_performance();
    assert_eq!(high_perf.performance_mode, PerformanceMode::HighPerformance);
    assert_eq!(high_perf.get_vector_storage_ms(), 5000);
    assert_eq!(high_perf.get_embedding_cache_ms(), 10000);
    assert_eq!(high_perf.get_lru_cache_ms(), 8000);
    assert_eq!(high_perf.get_promotion_indices_ms(), 3000);
    assert_eq!(high_perf.get_migration_db_ms(), 2000);
    assert_eq!(high_perf.get_compression_factor(), 15);
    assert!(high_perf.enable_compression);
    
    println!("  ✅ High Performance: vector={}ms, cache={}ms, compression={}", 
             high_perf.get_vector_storage_ms(), 
             high_perf.get_embedding_cache_ms(),
             high_perf.get_compression_factor());
    
    // High Reliability - надежность, частые flush
    let high_rel = FlushConfig::high_reliability();
    assert_eq!(high_rel.performance_mode, PerformanceMode::HighReliability);
    assert_eq!(high_rel.get_vector_storage_ms(), 500);
    assert_eq!(high_rel.get_embedding_cache_ms(), 1000);
    assert_eq!(high_rel.get_lru_cache_ms(), 800);
    assert_eq!(high_rel.get_promotion_indices_ms(), 300);
    assert_eq!(high_rel.get_migration_db_ms(), 200);
    assert_eq!(high_rel.get_compression_factor(), 19);
    assert!(high_rel.enable_compression);
    
    println!("  ✅ High Reliability: vector={}ms, cache={}ms, compression={}", 
             high_rel.get_vector_storage_ms(), 
             high_rel.get_embedding_cache_ms(),
             high_rel.get_compression_factor());
    
    // Balanced - баланс между производительностью и надежностью
    let balanced = FlushConfig::balanced();
    assert_eq!(balanced.performance_mode, PerformanceMode::Balanced);
    assert_eq!(balanced.get_vector_storage_ms(), 2000);
    assert_eq!(balanced.get_embedding_cache_ms(), 3000);
    assert_eq!(balanced.get_lru_cache_ms(), 2500);
    assert_eq!(balanced.get_promotion_indices_ms(), 1500);
    assert_eq!(balanced.get_migration_db_ms(), 1000);
    assert_eq!(balanced.get_compression_factor(), 17);
    assert!(balanced.enable_compression);
    
    println!("  ✅ Balanced: vector={}ms, cache={}ms, compression={}", 
             balanced.get_vector_storage_ms(), 
             balanced.get_embedding_cache_ms(),
             balanced.get_compression_factor());
    
    // Default должен быть Balanced
    let default = FlushConfig::default();
    assert_eq!(default.performance_mode, PerformanceMode::Balanced);
    assert!(default.enable_compression);
    assert_eq!(default.compression_factor, 19);
    
    println!("✅ Все performance modes работают корректно");
}

/// Тест custom конфигурации
#[test]
fn test_custom_configuration() {
    println!("🧪 Тестируем custom конфигурацию");
    
    let mut custom = FlushConfig::default();
    custom.performance_mode = PerformanceMode::Custom;
    custom.vector_storage_ms = Some(1500);
    custom.embedding_cache_ms = Some(2500);
    custom.lru_cache_ms = Some(1800);
    custom.promotion_indices_ms = Some(900);
    custom.migration_db_ms = Some(600);
    custom.enable_compression = false;
    custom.compression_factor = 10;
    
    // Проверяем что custom значения используются
    assert_eq!(custom.get_vector_storage_ms(), 1500);
    assert_eq!(custom.get_embedding_cache_ms(), 2500);
    assert_eq!(custom.get_lru_cache_ms(), 1800);
    assert_eq!(custom.get_promotion_indices_ms(), 900);
    assert_eq!(custom.get_migration_db_ms(), 600);
    assert_eq!(custom.get_compression_factor(), 0); // Disabled
    assert!(!custom.enable_compression);
    
    println!("  ✅ Custom значения: vector={}ms, compression disabled", 
             custom.get_vector_storage_ms());
    
    // Включаем compression
    custom.enable_compression = true;
    assert_eq!(custom.get_compression_factor(), 10); // Custom value
    
    println!("✅ Custom конфигурация работает корректно");
}

/// Тест fallback логики для None значений
#[test]
fn test_fallback_behavior() {
    println!("🧪 Тестируем fallback логику для None значений");
    
    // Создаем конфигурацию с None значениями для разных modes
    for mode in [PerformanceMode::HighPerformance, PerformanceMode::Balanced, PerformanceMode::HighReliability, PerformanceMode::Custom] {
        let config = FlushConfig {
            vector_storage_ms: None,
            embedding_cache_ms: None,
            lru_cache_ms: None,
            promotion_indices_ms: None,
            migration_db_ms: None,
            performance_mode: mode,
            enable_compression: true,
            compression_factor: 17,
        };
        
        // Все get_* методы должны возвращать валидные значения (fallback)
        assert!(config.get_vector_storage_ms() > 0);
        assert!(config.get_embedding_cache_ms() > 0);
        assert!(config.get_lru_cache_ms() > 0);
        assert!(config.get_promotion_indices_ms() > 0);
        assert!(config.get_migration_db_ms() > 0);
        
        println!("  ✅ Fallback для {:?}: vector={}ms, cache={}ms", 
                 mode, config.get_vector_storage_ms(), config.get_embedding_cache_ms());
    }
    
    println!("✅ Fallback логика работает корректно");
}

/// Тест загрузки конфигурации из переменных окружения
#[test]
fn test_env_configuration() {
    println!("🧪 Тестируем загрузку конфигурации из env переменных");
    
    // Полная очистка env перед тестом
    for var in ["MAGRAY_PERFORMANCE_MODE", "MAGRAY_VECTOR_FLUSH_MS", "MAGRAY_CACHE_FLUSH_MS", 
                "MAGRAY_LRU_FLUSH_MS", "MAGRAY_PROMOTION_FLUSH_MS", "MAGRAY_MIGRATION_FLUSH_MS",
                "MAGRAY_COMPRESSION", "MAGRAY_COMPRESSION_FACTOR"] {
        env::remove_var(var);
    }
    
    // Сохраняем оригинальные значения
    let original_vars = [
        "MAGRAY_PERFORMANCE_MODE",
        "MAGRAY_VECTOR_FLUSH_MS",
        "MAGRAY_CACHE_FLUSH_MS",
        "MAGRAY_LRU_FLUSH_MS",
        "MAGRAY_PROMOTION_FLUSH_MS",
        "MAGRAY_MIGRATION_FLUSH_MS",
        "MAGRAY_COMPRESSION",
        "MAGRAY_COMPRESSION_FACTOR",
    ].iter().map(|var| (*var, env::var(var).ok())).collect::<Vec<_>>();
    
    // Тест 1: High Performance mode
    env::set_var("MAGRAY_PERFORMANCE_MODE", "high_performance");
    env::set_var("MAGRAY_VECTOR_FLUSH_MS", "4000");
    env::set_var("MAGRAY_CACHE_FLUSH_MS", "8000");
    env::set_var("MAGRAY_COMPRESSION", "true");
    env::set_var("MAGRAY_COMPRESSION_FACTOR", "12");
    
    let config = FlushConfig::from_env();
    assert_eq!(config.performance_mode, PerformanceMode::HighPerformance);
    assert_eq!(config.get_vector_storage_ms(), 4000);
    assert_eq!(config.get_embedding_cache_ms(), 8000);
    assert!(config.enable_compression);
    assert_eq!(config.compression_factor, 12);
    
    println!("  ✅ Загрузка из env: mode={:?}, vector={}ms", 
             config.performance_mode, config.get_vector_storage_ms());
    
    // Тест 2: Aliases
    env::set_var("MAGRAY_PERFORMANCE_MODE", "fast");
    let config2 = FlushConfig::from_env();
    assert_eq!(config2.performance_mode, PerformanceMode::HighPerformance);
    
    env::set_var("MAGRAY_PERFORMANCE_MODE", "safe");
    let config3 = FlushConfig::from_env();
    assert_eq!(config3.performance_mode, PerformanceMode::HighReliability);
    
    // Тест 3: Invalid values fallback
    env::set_var("MAGRAY_PERFORMANCE_MODE", "invalid_mode");
    env::set_var("MAGRAY_VECTOR_FLUSH_MS", "not_a_number");
    let config4 = FlushConfig::from_env();
    assert_eq!(config4.performance_mode, PerformanceMode::Balanced); // Default fallback
    // vector_storage_ms должен быть None, поэтому fallback к режиму
    
    // Тест 4: Compression settings
    env::set_var("MAGRAY_COMPRESSION", "false");
    env::set_var("MAGRAY_COMPRESSION_FACTOR", "25"); // Out of range
    let config5 = FlushConfig::from_env();
    assert!(!config5.enable_compression);
    assert_eq!(config5.compression_factor, 19); // Clamped to max
    
    env::set_var("MAGRAY_COMPRESSION", "1");
    env::set_var("MAGRAY_COMPRESSION_FACTOR", "-5"); // Out of range
    let config6 = FlushConfig::from_env();
    assert!(config6.enable_compression);
    assert_eq!(config6.compression_factor, 1); // Clamped to min
    
    // Восстанавливаем оригинальные значения
    for (var, value) in original_vars {
        match value {
            Some(val) => env::set_var(var, val),
            None => env::remove_var(var),
        }
    }
    
    println!("✅ Загрузка из env переменных работает корректно");
}

/// Тест сохранения конфигурации в переменные окружения
#[test]
fn test_env_save() {
    println!("🧪 Тестируем сохранение конфигурации в env");
    
    // Очищаем env перед тестом
    for var in ["MAGRAY_PERFORMANCE_MODE", "MAGRAY_VECTOR_FLUSH_MS", "MAGRAY_CACHE_FLUSH_MS", 
                "MAGRAY_LRU_FLUSH_MS", "MAGRAY_PROMOTION_FLUSH_MS", "MAGRAY_MIGRATION_FLUSH_MS",
                "MAGRAY_COMPRESSION", "MAGRAY_COMPRESSION_FACTOR"] {
        env::remove_var(var);
    }
    
    // Создаем custom конфигурацию
    let mut config = FlushConfig::high_reliability();
    config.vector_storage_ms = Some(1234);
    config.embedding_cache_ms = Some(5678);
    config.enable_compression = false;
    config.compression_factor = 8;
    
    // Сохраняем в env
    config.to_env();
    
    // Проверяем что значения сохранились  
    let saved_mode = env::var("MAGRAY_PERFORMANCE_MODE").unwrap();
    println!("  📄 Saved mode: {}", saved_mode);
    assert_eq!(saved_mode, "high_reliability");
    assert_eq!(env::var("MAGRAY_VECTOR_FLUSH_MS").unwrap(), "1234");
    assert_eq!(env::var("MAGRAY_CACHE_FLUSH_MS").unwrap(), "5678");
    assert_eq!(env::var("MAGRAY_COMPRESSION").unwrap(), "false");
    assert_eq!(env::var("MAGRAY_COMPRESSION_FACTOR").unwrap(), "8");
    
    // Загружаем обратно и проверяем
    let loaded_config = FlushConfig::from_env();
    assert_eq!(loaded_config.performance_mode, PerformanceMode::HighReliability);
    assert_eq!(loaded_config.get_vector_storage_ms(), 1234);
    assert_eq!(loaded_config.get_embedding_cache_ms(), 5678);
    assert!(!loaded_config.enable_compression);
    assert_eq!(loaded_config.compression_factor, 8);
    
    println!("  ✅ Round-trip env save/load: mode={:?}, vector={}ms", 
             loaded_config.performance_mode, loaded_config.get_vector_storage_ms());
    
    println!("✅ Сохранение в env переменные работает корректно");
}

/// Тест сериализации и десериализации JSON
#[test]
fn test_json_serialization() {
    println!("🧪 Тестируем JSON сериализацию конфигурации");
    
    let config = FlushConfig::balanced();
    
    // Сериализуем в JSON
    let json = serde_json::to_string_pretty(&config).expect("Failed to serialize");
    println!("  📄 JSON representation:\n{}", json);
    
    // Проверяем что JSON содержит ожидаемые поля
    assert!(json.contains("performance_mode"));
    assert!(json.contains("enable_compression"));
    assert!(json.contains("compression_factor"));
    
    // Десериализуем обратно
    let deserialized: FlushConfig = serde_json::from_str(&json).expect("Failed to deserialize");
    
    // Проверяем что значения совпадают
    assert_eq!(deserialized.performance_mode, config.performance_mode);
    assert_eq!(deserialized.enable_compression, config.enable_compression);
    assert_eq!(deserialized.compression_factor, config.compression_factor);
    assert_eq!(deserialized.get_vector_storage_ms(), config.get_vector_storage_ms());
    assert_eq!(deserialized.get_embedding_cache_ms(), config.get_embedding_cache_ms());
    
    println!("  ✅ Round-trip JSON: mode={:?}, intervals match", 
             deserialized.performance_mode);
    
    println!("✅ JSON сериализация работает корректно");
}

/// Тест describe метода
#[test]
fn test_describe_method() {
    println!("🧪 Тестируем describe метод конфигурации");
    
    let config = FlushConfig::high_performance();
    let description = config.describe();
    
    println!("  📋 Description:\n{}", description);
    
    // Проверяем что описание содержит ключевые элементы
    assert!(description.contains("Performance Mode"));
    assert!(description.contains("Vector Storage"));
    assert!(description.contains("Embedding Cache"));
    assert!(description.contains("LRU Cache"));
    assert!(description.contains("Promotion"));
    assert!(description.contains("Migration"));
    assert!(description.contains("Compression"));
    assert!(description.contains("5000ms")); // High performance vector storage
    assert!(description.contains("factor: 15")); // High performance compression
    
    // Тест для disabled compression
    let mut no_compression = FlushConfig::balanced();
    no_compression.enable_compression = false;
    let desc2 = no_compression.describe();
    assert!(desc2.contains("disabled"));
    assert!(desc2.contains("factor: 0"));
    
    println!("✅ Describe method работает корректно");
}

/// Тест edge cases и валидации
#[test]
fn test_edge_cases() {
    println!("🧪 Тестируем edge cases и валидации");
    
    // Тест очень малых значений
    let mut config = FlushConfig::default();
    config.vector_storage_ms = Some(0);
    config.embedding_cache_ms = Some(1);
    assert_eq!(config.get_vector_storage_ms(), 0);
    assert_eq!(config.get_embedding_cache_ms(), 1);
    
    // Тест очень больших значений
    config.vector_storage_ms = Some(u64::MAX);
    config.embedding_cache_ms = Some(86400000); // 24 hours
    assert_eq!(config.get_vector_storage_ms(), u64::MAX);
    assert_eq!(config.get_embedding_cache_ms(), 86400000);
    
    // Тест compression factor clamping
    config.compression_factor = -10;
    config.enable_compression = true;
    config.performance_mode = PerformanceMode::Custom;
    assert_eq!(config.get_compression_factor(), -10); // Custom не clamp'ится в get_compression_factor()
    
    // Но from_env должен clamp
    env::set_var("MAGRAY_PERFORMANCE_MODE", "balanced"); // Reset mode
    env::set_var("MAGRAY_COMPRESSION", "true");
    env::set_var("MAGRAY_COMPRESSION_FACTOR", "-10");
    let env_config = FlushConfig::from_env();
    println!("  📊 Clamped factor for -10: {}", env_config.compression_factor);
    assert_eq!(env_config.compression_factor, 1); // Clamped to minimum
    
    env::set_var("MAGRAY_COMPRESSION_FACTOR", "25");
    let env_config2 = FlushConfig::from_env();
    println!("  📊 Clamped factor for 25: {}", env_config2.compression_factor);
    assert_eq!(env_config2.compression_factor, 19); // Clamped to maximum
    
    // Cleanup - полная очистка env
    for var in ["MAGRAY_PERFORMANCE_MODE", "MAGRAY_VECTOR_FLUSH_MS", "MAGRAY_CACHE_FLUSH_MS", 
                "MAGRAY_LRU_FLUSH_MS", "MAGRAY_PROMOTION_FLUSH_MS", "MAGRAY_MIGRATION_FLUSH_MS",
                "MAGRAY_COMPRESSION", "MAGRAY_COMPRESSION_FACTOR"] {
        env::remove_var(var);
    }
    
    println!("  ✅ Edge values: zero={}, max={}", 
             config.get_vector_storage_ms() == 0, 
             config.get_embedding_cache_ms() == 86400000);
    
    println!("✅ Edge cases обработаны корректно");
}

/// Тест производительности getters
#[test]
fn test_performance_getters() {
    println!("🧪 Тестируем производительность getter methods");
    
    let config = FlushConfig::balanced();
    
    let start = std::time::Instant::now();
    for _ in 0..100_000 {
        let _vector = config.get_vector_storage_ms();
        let _cache = config.get_embedding_cache_ms();
        let _lru = config.get_lru_cache_ms();
        let _promotion = config.get_promotion_indices_ms();
        let _migration = config.get_migration_db_ms();
        let _compression = config.get_compression_factor();
    }
    let elapsed = start.elapsed();
    
    println!("  ⚡ 600K getter calls за {:?}", elapsed);
    
    // Должно быть очень быстро (< 10ms для 600K вызовов)
    assert!(elapsed.as_millis() < 10);
    
    println!("✅ Производительность getters отличная");
}

/// Integration test всех performance modes
#[test]
fn test_all_modes_integration() {
    println!("🧪 Integration test всех performance modes");
    
    let modes = [
        (PerformanceMode::HighPerformance, FlushConfig::high_performance()),
        (PerformanceMode::Balanced, FlushConfig::balanced()),
        (PerformanceMode::HighReliability, FlushConfig::high_reliability()),
    ];
    
    for (expected_mode, config) in modes {
        // Проверяем логическую последовательность интервалов
        let vector_ms = config.get_vector_storage_ms();
        let cache_ms = config.get_embedding_cache_ms();
        let lru_ms = config.get_lru_cache_ms();
        let promotion_ms = config.get_promotion_indices_ms();
        let migration_ms = config.get_migration_db_ms();
        
        // Все интервалы должны быть положительными
        assert!(vector_ms > 0);
        assert!(cache_ms > 0);
        assert!(lru_ms > 0);
        assert!(promotion_ms > 0);
        assert!(migration_ms > 0);
        
        // High Performance должен иметь большие интервалы
        // High Reliability должен иметь маленькие интервалы
        match expected_mode {
            PerformanceMode::HighPerformance => {
                assert!(vector_ms >= 5000);
                assert!(cache_ms >= 10000);
            }
            PerformanceMode::HighReliability => {
                assert!(vector_ms <= 500);
                assert!(cache_ms <= 1000);
            }
            PerformanceMode::Balanced => {
                assert!(vector_ms >= 1000 && vector_ms <= 5000);
                assert!(cache_ms >= 1000 && cache_ms <= 10000);
            }
            _ => {}
        }
        
        // Compression factor должен быть в валидном диапазоне
        let comp_factor = config.get_compression_factor();
        assert!(comp_factor >= 0 && comp_factor <= 19);
        
        // Описание должно быть информативным
        let description = config.describe();
        assert!(description.len() > 50);
        assert!(description.contains(&format!("{:?}", expected_mode)));
        
        println!("  ✅ Mode {:?}: vector={}ms, cache={}ms, compression={}", 
                 expected_mode, vector_ms, cache_ms, comp_factor);
    }
    
    println!("✅ Все performance modes работают корректно");
}

/// Quick smoke test для всех основных функций
#[test]
fn test_flush_config_smoke() {
    // Test basic creation
    let _default = FlushConfig::default();
    let _balanced = FlushConfig::balanced();
    let _high_perf = FlushConfig::high_performance();
    let _high_rel = FlushConfig::high_reliability();
    
    // Test getters
    assert!(_balanced.get_vector_storage_ms() > 0);
    assert!(_balanced.get_compression_factor() >= 0);
    
    // Test describe
    let desc = _balanced.describe();
    assert!(!desc.is_empty());
    
    // Test serialization
    let json = serde_json::to_string(&_balanced).unwrap();
    let _deserialized: FlushConfig = serde_json::from_str(&json).unwrap();
    
    println!("✅ Все функции flush config работают");
}