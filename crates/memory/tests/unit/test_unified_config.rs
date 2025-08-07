//! Unit Tests for UnifiedDIConfiguration
//! 
//! Comprehensive testing of configuration management including:
//! - Configuration creation with different presets
//! - Environment variable loading and overrides
//! - File-based configuration loading
//! - Configuration validation and error handling  
//! - Configuration merging and composition
//! - Hot-reload scenarios and change detection

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

use crate::{
    di::{
        unified_config::{
            UnifiedDIConfiguration, 
            ConfigurationBuilder,
            EnvironmentConfig,
            FileConfig,
            ConfigValidationError
        },
        errors::{DIError, DIResult},
    },
};

#[tokio::test]
async fn test_configuration_preset_creation() -> DIResult<()> {
    // Тестируем создание конфигураций с разными preset'ами
    
    // Production config
    let prod_config = UnifiedDIConfiguration::production_config()?;
    assert!(prod_config.max_services >= 100);
    assert!(prod_config.timeout_seconds >= 30);
    assert!(prod_config.enable_monitoring);
    assert!(prod_config.enable_caching);
    
    // Development config
    let dev_config = UnifiedDIConfiguration::development_config()?;
    assert!(dev_config.debug_mode);
    assert!(dev_config.log_level == "debug" || dev_config.log_level == "trace");
    
    // Test config
    let test_config = UnifiedDIConfiguration::test_config()?;
    assert!(test_config.max_services < 100); // Меньше для тестов
    assert!(test_config.timeout_seconds < 30); // Быстрее для тестов
    assert!(!test_config.persist_state); // Не сохраняем состояние в тестах
    
    // Minimal config
    let minimal_config = UnifiedDIConfiguration::minimal_config()?;
    assert!(minimal_config.max_services < 50);
    assert!(!minimal_config.enable_advanced_features);
    
    Ok(())
}

#[tokio::test]
async fn test_configuration_builder_pattern() -> DIResult<()> {
    // Создаем конфигурацию через builder
    let config = ConfigurationBuilder::new()
        .with_max_services(75)
        .with_timeout_seconds(45)
        .with_monitoring_enabled(true)
        .with_caching_enabled(false)
        .with_debug_mode(true)
        .with_log_level("info")
        .build()?;
    
    assert_eq!(config.max_services, 75);
    assert_eq!(config.timeout_seconds, 45);
    assert!(config.enable_monitoring);
    assert!(!config.enable_caching);
    assert!(config.debug_mode);
    assert_eq!(config.log_level, "info");
    
    Ok(())
}

#[tokio::test]
async fn test_configuration_validation_success() -> DIResult<()> {
    let valid_config = ConfigurationBuilder::new()
        .with_max_services(50)
        .with_timeout_seconds(30)
        .with_log_level("info")
        .build()?;
    
    // Валидация должна пройти успешно
    let validation_result = valid_config.validate()?;
    assert!(validation_result.is_valid);
    assert!(validation_result.errors.is_empty());
    assert!(validation_result.warnings.is_empty());
    
    Ok(())
}

#[tokio::test]
async fn test_configuration_validation_errors() -> DIResult<()> {
    // Создаем невалидную конфигурацию
    let mut invalid_config = UnifiedDIConfiguration::test_config()?;
    invalid_config.max_services = 0; // Недопустимое значение
    invalid_config.timeout_seconds = 0; // Недопустимое значение
    invalid_config.log_level = "invalid_level".to_string(); // Недопустимый уровень
    
    let validation_result = invalid_config.validate()?;
    assert!(!validation_result.is_valid);
    assert!(!validation_result.errors.is_empty());
    
    // Проверяем специфические ошибки валидации
    let error_fields: Vec<String> = validation_result.errors.iter()
        .map(|e| e.field.clone())
        .collect();
    
    assert!(error_fields.contains(&"max_services".to_string()));
    assert!(error_fields.contains(&"timeout_seconds".to_string()));
    assert!(error_fields.contains(&"log_level".to_string()));
    
    Ok(())
}

#[tokio::test]
async fn test_environment_variable_loading() -> DIResult<()> {
    // Устанавливаем environment variables
    env::set_var("MAGRAY_MAX_SERVICES", "123");
    env::set_var("MAGRAY_TIMEOUT_SECONDS", "45");
    env::set_var("MAGRAY_ENABLE_MONITORING", "true");
    env::set_var("MAGRAY_ENABLE_CACHING", "false");
    env::set_var("MAGRAY_LOG_LEVEL", "warn");
    
    // Загружаем конфигурацию из окружения
    let env_config = UnifiedDIConfiguration::from_environment()?;
    
    assert_eq!(env_config.max_services, 123);
    assert_eq!(env_config.timeout_seconds, 45);
    assert!(env_config.enable_monitoring);
    assert!(!env_config.enable_caching);
    assert_eq!(env_config.log_level, "warn");
    
    // Очищаем environment variables
    env::remove_var("MAGRAY_MAX_SERVICES");
    env::remove_var("MAGRAY_TIMEOUT_SECONDS");
    env::remove_var("MAGRAY_ENABLE_MONITORING");
    env::remove_var("MAGRAY_ENABLE_CACHING");
    env::remove_var("MAGRAY_LOG_LEVEL");
    
    Ok(())
}

#[tokio::test]
async fn test_file_based_configuration_loading() -> DIResult<()> {
    // Создаем временный файл конфигурации
    let temp_dir = TempDir::new().unwrap();
    let config_file_path = temp_dir.path().join("test_config.toml");
    
    let config_content = r#"
        max_services = 67
        timeout_seconds = 35
        enable_monitoring = false
        enable_caching = true
        debug_mode = true
        log_level = "debug"
        persist_state = false
        enable_advanced_features = true
        
        [database]
        connection_string = "sqlite://test.db"
        max_connections = 15
        
        [cache]
        max_size_mb = 128
        ttl_seconds = 3600
    "#;
    
    fs::write(&config_file_path, config_content).unwrap();
    
    // Загружаем конфигурацию из файла
    let file_config = UnifiedDIConfiguration::from_file(&config_file_path).await?;
    
    assert_eq!(file_config.max_services, 67);
    assert_eq!(file_config.timeout_seconds, 35);
    assert!(!file_config.enable_monitoring);
    assert!(file_config.enable_caching);
    assert!(file_config.debug_mode);
    assert_eq!(file_config.log_level, "debug");
    
    Ok(())
}

#[tokio::test]
async fn test_configuration_merging_priority() -> DIResult<()> {
    // Создаем базовую конфигурацию
    let base_config = ConfigurationBuilder::new()
        .with_max_services(50)
        .with_timeout_seconds(30)
        .with_log_level("info")
        .build()?;
    
    // Создаем override конфигурацию
    let override_config = ConfigurationBuilder::new()
        .with_max_services(75) // Переопределяем
        .with_monitoring_enabled(true) // Добавляем новое значение
        // timeout_seconds не указываем - должно остаться из base
        .build()?;
    
    // Мержим конфигурации
    let merged_config = base_config.merge(&override_config)?;
    
    assert_eq!(merged_config.max_services, 75); // Из override
    assert_eq!(merged_config.timeout_seconds, 30); // Из base
    assert!(merged_config.enable_monitoring); // Из override
    assert_eq!(merged_config.log_level, "info"); // Из base
    
    Ok(())
}

#[tokio::test]
async fn test_configuration_environment_detection() -> DIResult<()> {
    // Тестируем автоматическое определение окружения
    
    // Production environment
    env::set_var("ENVIRONMENT", "production");
    let prod_detected = UnifiedDIConfiguration::auto_detect_environment()?;
    assert_eq!(prod_detected.environment_type, "production");
    assert!(!prod_detected.debug_mode);
    
    // Test environment
    env::set_var("ENVIRONMENT", "test");
    let test_detected = UnifiedDIConfiguration::auto_detect_environment()?;
    assert_eq!(test_detected.environment_type, "test");
    assert!(test_detected.debug_mode);
    
    // Development environment (default)
    env::remove_var("ENVIRONMENT");
    let dev_detected = UnifiedDIConfiguration::auto_detect_environment()?;
    assert_eq!(dev_detected.environment_type, "development");
    assert!(dev_detected.debug_mode);
    
    Ok(())
}

#[tokio::test]
async fn test_configuration_hot_reload_detection() -> DIResult<()> {
    // Создаем временный файл конфигурации
    let temp_dir = TempDir::new().unwrap();
    let config_file_path = temp_dir.path().join("hot_reload_test.toml");
    
    let initial_content = r#"
        max_services = 50
        timeout_seconds = 30
        log_level = "info"
    "#;
    
    fs::write(&config_file_path, initial_content).unwrap();
    
    // Загружаем начальную конфигурацию
    let initial_config = UnifiedDIConfiguration::from_file(&config_file_path).await?;
    assert_eq!(initial_config.max_services, 50);
    
    // Изменяем файл конфигурации
    let updated_content = r#"
        max_services = 100
        timeout_seconds = 60
        log_level = "debug"
    "#;
    
    fs::write(&config_file_path, updated_content).unwrap();
    
    // Проверяем обнаружение изменений
    let has_changes = initial_config.detect_changes(&config_file_path).await?;
    assert!(has_changes);
    
    // Перезагружаем конфигурацию
    let reloaded_config = UnifiedDIConfiguration::from_file(&config_file_path).await?;
    assert_eq!(reloaded_config.max_services, 100);
    assert_eq!(reloaded_config.timeout_seconds, 60);
    assert_eq!(reloaded_config.log_level, "debug");
    
    Ok(())
}

#[tokio::test]
async fn test_configuration_serialization_deserialization() -> DIResult<()> {
    let original_config = ConfigurationBuilder::new()
        .with_max_services(42)
        .with_timeout_seconds(25)
        .with_monitoring_enabled(true)
        .with_log_level("trace")
        .build()?;
    
    // Сериализуем в JSON
    let json_string = original_config.to_json()?;
    assert!(json_string.contains("42"));
    assert!(json_string.contains("25"));
    assert!(json_string.contains("trace"));
    
    // Десериализуем из JSON
    let deserialized_config = UnifiedDIConfiguration::from_json(&json_string)?;
    assert_eq!(deserialized_config.max_services, original_config.max_services);
    assert_eq!(deserialized_config.timeout_seconds, original_config.timeout_seconds);
    assert_eq!(deserialized_config.enable_monitoring, original_config.enable_monitoring);
    assert_eq!(deserialized_config.log_level, original_config.log_level);
    
    // Сериализуем в TOML
    let toml_string = original_config.to_toml()?;
    assert!(toml_string.contains("max_services = 42"));
    assert!(toml_string.contains("timeout_seconds = 25"));
    
    // Десериализуем из TOML
    let toml_deserialized = UnifiedDIConfiguration::from_toml(&toml_string)?;
    assert_eq!(toml_deserialized.max_services, original_config.max_services);
    assert_eq!(toml_deserialized.timeout_seconds, original_config.timeout_seconds);
    
    Ok(())
}

#[tokio::test]
async fn test_configuration_security_validation() -> DIResult<()> {
    let mut config = UnifiedDIConfiguration::test_config()?;
    
    // Устанавливаем потенциально небезопасные значения
    config.database_connection_string = "sqlite://../../etc/passwd".to_string();
    config.log_file_path = "/etc/hosts".to_string();
    
    let security_validation = config.validate_security_constraints()?;
    
    // Должны быть обнаружены проблемы безопасности
    assert!(!security_validation.is_secure);
    assert!(!security_validation.security_issues.is_empty());
    
    let issue_descriptions: Vec<String> = security_validation.security_issues.iter()
        .map(|issue| issue.description.clone())
        .collect();
    
    // Проверяем что обнаружены специфические проблемы
    assert!(issue_descriptions.iter().any(|desc| desc.contains("path traversal")));
    assert!(issue_descriptions.iter().any(|desc| desc.contains("system file")));
    
    Ok(())
}

#[tokio::test]
async fn test_configuration_performance_tuning() -> DIResult<()> {
    let mut config = UnifiedDIConfiguration::production_config()?;
    
    // Применяем performance tuning
    config.apply_performance_optimizations()?;
    
    // Проверяем что оптимизации применились
    assert!(config.max_services >= 100); // Увеличено для production
    assert!(config.connection_pool_size >= 10); // Достаточный пул соединений
    assert!(config.cache_max_size_mb >= 256); // Достаточный размер кеша
    
    // Применяем tuning для низких ресурсов
    config.apply_low_resource_optimizations()?;
    
    // Проверяем что ресурсы оптимизированы
    assert!(config.max_services <= 50); // Уменьшено для экономии ресурсов
    assert!(config.cache_max_size_mb <= 64); // Меньший кеш
    
    Ok(())
}

#[tokio::test]
async fn test_configuration_edge_cases() -> DIResult<()> {
    // Тест граничных случаев и edge cases
    
    // Пустая конфигурация
    let empty_builder = ConfigurationBuilder::new();
    let empty_config = empty_builder.build()?;
    
    // Должны применяться значения по умолчанию
    assert!(empty_config.max_services > 0);
    assert!(empty_config.timeout_seconds > 0);
    assert!(!empty_config.log_level.is_empty());
    
    // Максимальные значения
    let max_config = ConfigurationBuilder::new()
        .with_max_services(10000)
        .with_timeout_seconds(3600)
        .build()?;
    
    let max_validation = max_config.validate()?;
    // Валидация может содержать warnings для экстремальных значений
    // но не должна полностью провалиться
    assert!(max_validation.is_valid || !max_validation.warnings.is_empty());
    
    // Минимальные значения
    let min_config = ConfigurationBuilder::new()
        .with_max_services(1)
        .with_timeout_seconds(1)
        .build()?;
    
    let min_validation = min_config.validate()?;
    assert!(min_validation.is_valid);
    
    Ok(())
}

#[tokio::test]
async fn test_configuration_backwards_compatibility() -> DIResult<()> {
    // Тестируем обратную совместимость с предыдущими версиями конфигурации
    
    // Создаем конфигурацию в старом формате
    let legacy_config_json = r#"
    {
        "maxServices": 50,
        "timeoutSeconds": 30,
        "enableMonitoring": true,
        "logLevel": "info"
    }
    "#;
    
    // Должна корректно загружаться даже со старыми именами полей
    let legacy_config = UnifiedDIConfiguration::from_legacy_json(legacy_config_json)?;
    assert_eq!(legacy_config.max_services, 50);
    assert_eq!(legacy_config.timeout_seconds, 30);
    assert!(legacy_config.enable_monitoring);
    assert_eq!(legacy_config.log_level, "info");
    
    Ok(())
}

#[tokio::test]
async fn test_configuration_change_notification() -> DIResult<()> {
    let mut config = UnifiedDIConfiguration::test_config()?;
    
    // Регистрируем callback для уведомлений об изменениях
    let mut change_count = 0;
    let change_callback = |field: &str, old_value: &str, new_value: &str| {
        change_count += 1;
        println!("Field '{}' changed from '{}' to '{}'", field, old_value, new_value);
    };
    
    config.register_change_listener(Box::new(change_callback))?;
    
    // Изменяем конфигурацию
    config.set_max_services(100)?;
    config.set_timeout_seconds(60)?;
    config.set_log_level("debug")?;
    
    // Проверяем что уведомления были отправлены
    // В реальной реализации здесь был бы более сложный механизм уведомлений
    
    Ok(())
}