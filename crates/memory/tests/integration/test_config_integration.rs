//! Configuration Integration Tests
//! 
//! Comprehensive integration testing of configuration system including:
//! - Environment detection and automatic configuration selection
//! - File-based configuration loading and validation
//! - Configuration merging and priority handling
//! - Hot-reload scenarios and change detection
//! - Multi-environment configuration management
//! - Configuration security and validation

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::time::{timeout, Duration, sleep};

use crate::{
    di::{
        unified_config::{UnifiedDIConfiguration, ConfigurationBuilder},
        unified_container::UnifiedDIContainer,
        errors::{DIError, DIResult},
    },
    services::{
        unified_factory::{UnifiedServiceFactory, FactoryPreset},
        monitoring_service::MonitoringService,
    },
    tests::common::{
        test_fixtures::{TestContainerFactory, TestDataGenerator},
        mock_services::MockMonitoringService,
    },
};

#[tokio::test]
async fn test_environment_based_configuration_detection() -> DIResult<()> {
    // Сохраняем текущие переменные окружения
    let original_env = env::var("MAGRAY_ENVIRONMENT").ok();
    let original_log_level = env::var("MAGRAY_LOG_LEVEL").ok();
    
    // Тестируем разные окружения
    let test_environments = vec![
        ("production", "info", false),
        ("development", "debug", true),
        ("test", "trace", true),
        ("staging", "info", false),
    ];
    
    for (env_name, expected_log_level, expected_debug_mode) in test_environments {
        // Устанавливаем переменные окружения
        env::set_var("MAGRAY_ENVIRONMENT", env_name);
        env::set_var("MAGRAY_LOG_LEVEL", expected_log_level);
        
        // Загружаем конфигурацию из окружения
        let config = UnifiedDIConfiguration::from_environment()?;
        
        // Проверяем что конфигурация соответствует окружению
        assert_eq!(config.environment_type, env_name);
        assert_eq!(config.log_level, expected_log_level);
        assert_eq!(config.debug_mode, expected_debug_mode);
        
        // Создаем контейнер с этой конфигурацией
        let factory = UnifiedServiceFactory::from_environment()?;
        let container = factory.build_container(&config).await?;
        
        // Проверяем что контейнер работает с данной конфигурацией
        assert!(container.is_healthy().await);
        
        // Проверяем что сервисы могут быть резолвлены
        let monitoring = container.resolve::<MonitoringService>().await?;
        assert!(monitoring.is_healthy().await);
        
        container.shutdown().await?;
    }
    
    // Восстанавливаем исходные переменные окружения
    match original_env {
        Some(val) => env::set_var("MAGRAY_ENVIRONMENT", val),
        None => env::remove_var("MAGRAY_ENVIRONMENT"),
    }
    match original_log_level {
        Some(val) => env::set_var("MAGRAY_LOG_LEVEL", val),
        None => env::remove_var("MAGRAY_LOG_LEVEL"),
    }
    
    Ok(())
}

#[tokio::test]
async fn test_file_based_configuration_integration() -> DIResult<()> {
    // Создаем временную директорию для конфигурационных файлов
    let temp_dir = TempDir::new().expect("Test operation should succeed");
    let base_config_path = temp_dir.path().join("base_config.toml");
    let override_config_path = temp_dir.path().join("override_config.toml");
    
    // Создаем базовую конфигурацию
    let base_config_content = r#"
        [application]
        environment_type = "integration_test"
        max_services = 75
        timeout_seconds = 45
        enable_monitoring = true
        enable_caching = false
        debug_mode = false
        log_level = "info"
        
        [database]
        connection_string = "sqlite://base_test.db"
        max_connections = 20
        
        [cache]
        max_size_mb = 256
        ttl_seconds = 1800
        
        [performance]
        enable_advanced_features = true
        max_concurrent_operations = 50
    "#;
    
    // Создаем override конфигурацию
    let override_config_content = r#"
        [application]
        max_services = 150
        enable_caching = true
        debug_mode = true
        log_level = "debug"
        
        [database]
        max_connections = 50
        
        [cache]
        max_size_mb = 512
        
        [custom]
        custom_feature_enabled = true
        custom_timeout = 120
    "#;
    
    // Записываем файлы
    fs::write(&base_config_path, base_config_content).expect("Test operation should succeed");
    fs::write(&override_config_path, override_config_content).expect("Test operation should succeed");
    
    // Загружаем базовую конфигурацию
    let base_config = UnifiedDIConfiguration::from_file(&base_config_path).await?;
    
    // Проверяем базовые значения
    assert_eq!(base_config.max_services, 75);
    assert_eq!(base_config.timeout_seconds, 45);
    assert!(base_config.enable_monitoring);
    assert!(!base_config.enable_caching);
    assert_eq!(base_config.log_level, "info");
    
    // Загружаем override конфигурацию
    let override_config = UnifiedDIConfiguration::from_file(&override_config_path).await?;
    
    // Мержим конфигурации
    let merged_config = base_config.merge(&override_config)?;
    
    // Проверяем результат мержинга
    assert_eq!(merged_config.max_services, 150); // Из override
    assert_eq!(merged_config.timeout_seconds, 45); // Из base (не переопределено)
    assert!(merged_config.enable_monitoring); // Из base (не переопределено)
    assert!(merged_config.enable_caching); // Из override
    assert!(merged_config.debug_mode); // Из override
    assert_eq!(merged_config.log_level, "debug"); // Из override
    
    // Создаем контейнер с мерженной конфигурацией
    let factory = UnifiedServiceFactory::with_preset(FactoryPreset::Development)?;
    let container = factory.build_container(&merged_config).await?;
    
    // Проверяем что система работает с комбинированной конфигурацией
    assert!(container.is_healthy().await);
    
    let stats = container.get_statistics().await?;
    assert!(stats.registered_services > 0);
    
    container.shutdown().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_configuration_hot_reload_integration() -> DIResult<()> {
    let temp_dir = TempDir::new().expect("Test operation should succeed");
    let config_file_path = temp_dir.path().join("hot_reload_test.toml");
    
    // Создаем начальную конфигурацию
    let initial_config_content = r#"
        [application]
        max_services = 50
        timeout_seconds = 30
        enable_monitoring = true
        log_level = "info"
        
        [feature_flags]
        experimental_feature = false
        advanced_caching = false
    "#;
    
    fs::write(&config_file_path, initial_config_content).expect("Test operation should succeed");
    
    // Загружаем начальную конфигурацию и создаем контейнер
    let initial_config = UnifiedDIConfiguration::from_file(&config_file_path).await?;
    let factory = UnifiedServiceFactory::with_preset(FactoryPreset::Development)?;
    let container = factory.build_container(&initial_config).await?;
    
    // Проверяем начальное состояние
    assert_eq!(initial_config.max_services, 50);
    assert_eq!(initial_config.log_level, "info");
    assert!(container.is_healthy().await);
    
    // Симулируем работу системы
    let monitoring = container.resolve::<MonitoringService>().await?;
    monitoring.record_operation("initial_test", Duration::from_millis(10)).await;
    
    // Изменяем конфигурационный файл
    let updated_config_content = r#"
        [application]
        max_services = 100
        timeout_seconds = 60
        enable_monitoring = true
        log_level = "debug"
        
        [feature_flags]
        experimental_feature = true
        advanced_caching = true
        
        [new_section]
        new_feature = "enabled"
        new_timeout = 45
    "#;
    
    fs::write(&config_file_path, updated_config_content).expect("Test operation should succeed");
    
    // Обнаруживаем изменения
    let config_changed = initial_config.detect_changes(&config_file_path).await?;
    assert!(config_changed);
    
    // Перезагружаем конфигурацию
    let reloaded_config = UnifiedDIConfiguration::from_file(&config_file_path).await?;
    
    // Проверяем что изменения загрузились
    assert_eq!(reloaded_config.max_services, 100);
    assert_eq!(reloaded_config.timeout_seconds, 60);
    assert_eq!(reloaded_config.log_level, "debug");
    
    // В реальной системе здесь был бы hot reload контейнера
    // Пока что создаем новый контейнер с обновленной конфигурацией
    let new_container = factory.build_container(&reloaded_config).await?;
    assert!(new_container.is_healthy().await);
    
    // Проверяем что новая конфигурация применилась
    let new_stats = new_container.get_statistics().await?;
    assert!(new_stats.registered_services > 0);
    
    // Cleanup
    container.shutdown().await?;
    new_container.shutdown().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_multi_environment_configuration_precedence() -> DIResult<()> {
    let temp_dir = TempDir::new().expect("Test operation should succeed");
    
    // Создаем конфигурации для разных окружений
    let configs = vec![
        ("base.toml", r#"
            [application]
            max_services = 25
            timeout_seconds = 15
            enable_monitoring = false
            log_level = "warn"
            
            [database]
            connection_string = "sqlite://base.db"
        "#),
        ("development.toml", r#"
            [application]
            max_services = 50
            debug_mode = true
            log_level = "debug"
            
            [database]
            connection_string = "sqlite://dev.db"
        "#),
        ("production.toml", r#"
            [application]
            max_services = 200
            enable_monitoring = true
            log_level = "info"
            
            [database]
            connection_string = "postgresql://prod.db"
            
            [performance]
            enable_advanced_features = true
        "#),
        ("local.toml", r#"
            [application]
            log_level = "trace"
            
            [database]
            connection_string = "sqlite://local_override.db"
        "#),
    ];
    
    // Создаем файлы конфигураций
    let mut config_paths = HashMap::new();
    for (filename, content) in configs {
        let path = temp_dir.path().join(filename);
        fs::write(&path, content).expect("Test operation should succeed");
        config_paths.insert(filename, path);
    }
    
    // Тестируем приоритет загрузки конфигураций
    let base_config = UnifiedDIConfiguration::from_file(
        config_paths.get("base.toml").expect("Test operation should succeed")
    ).await?;
    
    let dev_config = UnifiedDIConfiguration::from_file(
        config_paths.get("development.toml").expect("Test operation should succeed")
    ).await?;
    
    let prod_config = UnifiedDIConfiguration::from_file(
        config_paths.get("production.toml").expect("Test operation should succeed")
    ).await?;
    
    let local_config = UnifiedDIConfiguration::from_file(
        config_paths.get("local.toml").expect("Test operation should succeed")
    ).await?;
    
    // Тестируем сценарий development окружения
    let dev_merged = base_config.merge(&dev_config)?;
    let dev_final = dev_merged.merge(&local_config)?; // Local переопределяет все
    
    assert_eq!(dev_final.max_services, 50); // Из dev
    assert_eq!(dev_final.timeout_seconds, 15); // Из base
    assert_eq!(dev_final.log_level, "trace"); // Из local (наивысший приоритет)
    assert!(!dev_final.enable_monitoring); // Из base (dev не переопределяет)
    
    // Тестируем сценарий production окружения
    let prod_merged = base_config.merge(&prod_config)?;
    let prod_final = prod_merged.merge(&local_config)?;
    
    assert_eq!(prod_final.max_services, 200); // Из prod
    assert_eq!(prod_final.timeout_seconds, 15); // Из base
    assert_eq!(prod_final.log_level, "trace"); // Из local
    assert!(prod_final.enable_monitoring); // Из prod
    
    // Создаем контейнеры с разными конфигурациями
    let dev_factory = UnifiedServiceFactory::with_preset(FactoryPreset::Development)?;
    let dev_container = dev_factory.build_container(&dev_final).await?;
    
    let prod_factory = UnifiedServiceFactory::with_preset(FactoryPreset::Production)?;
    let prod_container = prod_factory.build_container(&prod_final).await?;
    
    // Проверяем что оба контейнера работают
    assert!(dev_container.is_healthy().await);
    assert!(prod_container.is_healthy().await);
    
    // Проверяем различия в поведении
    let dev_stats = dev_container.get_statistics().await?;
    let prod_stats = prod_container.get_statistics().await?;
    
    // Production должен иметь больше зарегистрированных сервисов
    assert!(prod_stats.registered_services >= dev_stats.registered_services);
    
    // Cleanup
    dev_container.shutdown().await?;
    prod_container.shutdown().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_configuration_security_validation_integration() -> DIResult<()> {
    let temp_dir = TempDir::new().expect("Test operation should succeed");
    let secure_config_path = temp_dir.path().join("secure_config.toml");
    let insecure_config_path = temp_dir.path().join("insecure_config.toml");
    
    // Создаем безопасную конфигурацию
    let secure_config_content = r#"
        [application]
        max_services = 100
        timeout_seconds = 30
        log_level = "info"
        
        [database]
        connection_string = "sqlite://./data/secure.db"
        
        [logging]
        log_file_path = "./logs/application.log"
        
        [security]
        enable_encryption = true
        validate_inputs = true
    "#;
    
    // Создаем небезопасную конфигурацию
    let insecure_config_content = r#"
        [application]
        max_services = 10000
        timeout_seconds = 3600
        log_level = "trace"
        
        [database]
        connection_string = "sqlite://../../etc/passwd"
        
        [logging]
        log_file_path = "/etc/hosts"
        
        [security]
        enable_encryption = false
        validate_inputs = false
        
        [dangerous]
        allow_arbitrary_code = true
        disable_sandboxing = true
    "#;
    
    fs::write(&secure_config_path, secure_config_content).expect("Test operation should succeed");
    fs::write(&insecure_config_path, insecure_config_content).expect("Test operation should succeed");
    
    // Загружаем и валидируем безопасную конфигурацию
    let secure_config = UnifiedDIConfiguration::from_file(&secure_config_path).await?;
    let secure_validation = secure_config.validate_security_constraints()?;
    
    assert!(secure_validation.is_secure);
    assert!(secure_validation.security_issues.is_empty());
    
    // Создаем контейнер с безопасной конфигурацией
    let factory = UnifiedServiceFactory::with_preset(FactoryPreset::Production)?;
    let secure_container = factory.build_container(&secure_config).await?;
    assert!(secure_container.is_healthy().await);
    
    // Загружаем и валидируем небезопасную конфигурацию
    let insecure_config = UnifiedDIConfiguration::from_file(&insecure_config_path).await?;
    let insecure_validation = insecure_config.validate_security_constraints()?;
    
    assert!(!insecure_validation.is_secure);
    assert!(!insecure_validation.security_issues.is_empty());
    
    // Проверяем что обнаружены специфические проблемы безопасности
    let issue_descriptions: Vec<String> = insecure_validation.security_issues.iter()
        .map(|issue| issue.description.clone())
        .collect();
    
    assert!(issue_descriptions.iter().any(|desc| desc.contains("path traversal")));
    assert!(issue_descriptions.iter().any(|desc| desc.contains("system file")));
    
    // Попытка создания контейнера с небезопасной конфигурацией должна завершиться ошибкой
    // или предупреждением (в зависимости от политики безопасности)
    let insecure_result = factory.build_container(&insecure_config).await;
    
    // В production режиме небезопасные конфигурации должны отклоняться
    if factory.get_preset() == FactoryPreset::Production {
        assert!(insecure_result.is_err());
    }
    
    secure_container.shutdown().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_configuration_performance_impact() -> DIResult<()> {
    // Тестируем влияние различных конфигураций на производительность системы
    
    let configs = vec![
        ("minimal", UnifiedDIConfiguration::minimal_config()?),
        ("standard", UnifiedDIConfiguration::test_config()?),
        ("high_performance", {
            let mut config = UnifiedDIConfiguration::production_config()?;
            config.apply_performance_optimizations()?;
            config
        }),
        ("low_resource", {
            let mut config = UnifiedDIConfiguration::minimal_config()?;
            config.apply_low_resource_optimizations()?;
            config
        }),
    ];
    
    for (config_name, config) in configs {
        let start_time = std::time::Instant::now();
        
        // Создаем контейнер
        let factory = match config_name {
            "minimal" | "low_resource" => UnifiedServiceFactory::with_preset(FactoryPreset::Minimal)?,
            "high_performance" => UnifiedServiceFactory::with_preset(FactoryPreset::Production)?,
            _ => UnifiedServiceFactory::with_preset(FactoryPreset::Test)?,
        };
        
        let container = factory.build_container(&config).await?;
        let container_creation_time = start_time.elapsed();
        
        // Измеряем время резолва сервисов
        let resolve_start = std::time::Instant::now();
        
        let monitoring = container.resolve::<MonitoringService>().await?;
        let resolve_time = resolve_start.elapsed();
        
        // Измеряем производительность операций
        let operation_start = std::time::Instant::now();
        
        monitoring.record_operation("performance_test", Duration::from_millis(1)).await;
        let operation_time = operation_start.elapsed();
        
        // Измеряем время shutdown
        let shutdown_start = std::time::Instant::now();
        container.shutdown().await?;
        let shutdown_time = shutdown_start.elapsed();
        
        // Выводим метрики производительности
        println!("Performance metrics for '{}' configuration:", config_name);
        println!("  Container creation: {:?}", container_creation_time);
        println!("  Service resolution: {:?}", resolve_time);
        println!("  Operation execution: {:?}", operation_time);
        println!("  Container shutdown: {:?}", shutdown_time);
        
        // Устанавливаем разумные пороги производительности
        assert!(container_creation_time < Duration::from_secs(10));
        assert!(resolve_time < Duration::from_secs(1));
        assert!(operation_time < Duration::from_millis(100));
        assert!(shutdown_time < Duration::from_secs(5));
    }
    
    Ok(())
}

#[tokio::test]
async fn test_configuration_compatibility_across_versions() -> DIResult<()> {
    // Тестируем обратную совместимость конфигураций между версиями
    
    // Имитация старой версии конфигурации
    let legacy_config_v1 = r#"{
        "maxServices": 50,
        "timeoutSeconds": 30,
        "enableMonitoring": true,
        "logLevel": "info",
        "databaseConnectionString": "sqlite://legacy.db"
    }"#;
    
    // Имитация конфигурации версии 2
    let config_v2 = r#"
        [application]
        max_services = 75
        timeout_seconds = 45
        enable_monitoring = true
        log_level = "debug"
        
        [database]
        connection_string = "sqlite://v2.db"
        max_connections = 25
    "#;
    
    // Имитация текущей версии конфигурации
    let config_v3 = r#"
        [application]
        max_services = 100
        timeout_seconds = 60
        enable_monitoring = true
        enable_caching = true
        log_level = "info"
        
        [database]
        connection_string = "sqlite://v3.db"
        max_connections = 50
        
        [cache]
        max_size_mb = 256
        ttl_seconds = 3600
        
        [new_features]
        advanced_analytics = true
        ml_integration = false
    "#;
    
    // Загружаем конфигурации разных версий
    let legacy_config = UnifiedDIConfiguration::from_legacy_json(legacy_config_v1)?;
    
    let temp_dir = TempDir::new().expect("Test operation should succeed");
    
    let v2_path = temp_dir.path().join("config_v2.toml");
    fs::write(&v2_path, config_v2).expect("Test operation should succeed");
    let v2_config = UnifiedDIConfiguration::from_file(&v2_path).await?;
    
    let v3_path = temp_dir.path().join("config_v3.toml");
    fs::write(&v3_path, config_v3).expect("Test operation should succeed");
    let v3_config = UnifiedDIConfiguration::from_file(&v3_path).await?;
    
    // Проверяем что все конфигурации валидны
    assert!(legacy_config.validate()?.is_valid);
    assert!(v2_config.validate()?.is_valid);
    assert!(v3_config.validate()?.is_valid);
    
    // Проверяем миграцию между версиями
    let migrated_v1_to_v2 = legacy_config.migrate_to_current_version()?;
    let migrated_v2_to_v3 = v2_config.migrate_to_current_version()?;
    
    // Создаем контейнеры для всех версий конфигураций
    let factory = UnifiedServiceFactory::with_preset(FactoryPreset::Test)?;
    
    let legacy_container = factory.build_container(&legacy_config).await?;
    let v2_container = factory.build_container(&v2_config).await?;
    let v3_container = factory.build_container(&v3_config).await?;
    let migrated_container = factory.build_container(&migrated_v1_to_v2).await?;
    
    // Проверяем что все контейнеры работают
    assert!(legacy_container.is_healthy().await);
    assert!(v2_container.is_healthy().await);
    assert!(v3_container.is_healthy().await);
    assert!(migrated_container.is_healthy().await);
    
    // Проверяем что сервисы резолвятся во всех версиях
    let legacy_monitoring = legacy_container.resolve::<MonitoringService>().await?;
    let v2_monitoring = v2_container.resolve::<MonitoringService>().await?;
    let v3_monitoring = v3_container.resolve::<MonitoringService>().await?;
    let migrated_monitoring = migrated_container.resolve::<MonitoringService>().await?;
    
    assert!(legacy_monitoring.is_healthy().await);
    assert!(v2_monitoring.is_healthy().await);
    assert!(v3_monitoring.is_healthy().await);
    assert!(migrated_monitoring.is_healthy().await);
    
    // Cleanup
    legacy_container.shutdown().await?;
    v2_container.shutdown().await?;
    v3_container.shutdown().await?;
    migrated_container.shutdown().await?;
    
    Ok(())
}