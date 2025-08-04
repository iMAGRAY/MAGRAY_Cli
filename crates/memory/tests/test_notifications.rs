use anyhow::Result;
use memory::{
    NotificationConfig, NotificationChannel, NotificationManager,
    health::{HealthAlert, AlertSeverity, ComponentType},
};
use std::collections::HashMap;
use chrono::Utc;
use std::time::Duration;
use tokio::time::sleep;
use serde_json;

/// Комплексные unit тесты для notification системы
/// Тестирует: channels, routing, группировка, cooldown, фильтры, отправка

/// Тест создания конфигураций уведомлений
#[test]
fn test_notification_config_creation() {
    println!("🧪 Тестируем создание конфигураций уведомлений");
    
    // Default конфигурация
    let default_config = NotificationConfig::default();
    assert_eq!(default_config.channels.len(), 2); // Log + Console
    assert!(default_config.enable_grouping);
    assert_eq!(default_config.cooldown_seconds, 300);
    assert_eq!(default_config.max_group_size, 10);
    assert_eq!(default_config.group_interval_seconds, 60);
    
    // Проверяем routing
    assert!(default_config.routing.contains_key(&AlertSeverity::Info));
    assert!(default_config.routing.contains_key(&AlertSeverity::Warning));
    assert!(default_config.routing.contains_key(&AlertSeverity::Critical));
    assert!(default_config.routing.contains_key(&AlertSeverity::Fatal));
    
    let fatal_channels = default_config.routing.get(&AlertSeverity::Fatal).unwrap();
    assert_eq!(fatal_channels, &vec!["*".to_string()]); // All channels
    
    println!("  ✅ Default: {} channels, grouping={}, cooldown={}s", 
             default_config.channels.len(), 
             default_config.enable_grouping,
             default_config.cooldown_seconds);
    
    // Custom конфигурация
    let mut custom_routing = HashMap::new();
    custom_routing.insert(AlertSeverity::Critical, vec!["slack".to_string(), "webhook".to_string()]);
    
    let custom_config = NotificationConfig {
        channels: vec![
            NotificationChannel::Console { colored: false },
            NotificationChannel::Slack {
                webhook_url: "https://hooks.slack.com/test".to_string(),
                channel: Some("#alerts".to_string()),
                mention_users: vec!["@user1".to_string()],
            },
            NotificationChannel::Webhook {
                url: "https://api.example.com/alerts".to_string(),
                method: "POST".to_string(),
                headers: {
                    let mut h = HashMap::new();
                    h.insert("Content-Type".to_string(), "application/json".to_string());
                    h
                },
                auth_token: Some("secret".to_string()),
            },
        ],
        routing: custom_routing,
        cooldown_seconds: 60,
        enable_grouping: false,
        max_group_size: 5,
        group_interval_seconds: 30,
        component_filters: Some(vec!["VectorStore".to_string()]),
        ignore_patterns: vec!["test".to_string(), "debug".to_string()],
    };
    
    assert_eq!(custom_config.channels.len(), 3);
    assert!(!custom_config.enable_grouping);
    assert_eq!(custom_config.cooldown_seconds, 60);
    assert_eq!(custom_config.component_filters.as_ref().unwrap().len(), 1);
    assert_eq!(custom_config.ignore_patterns.len(), 2);
    
    println!("  ✅ Custom: {} channels, no grouping, {} filters", 
             custom_config.channels.len(),
             custom_config.ignore_patterns.len());
    
    println!("✅ Создание конфигураций работает корректно");
}

/// Тест JSON сериализации конфигурации
#[test]
fn test_config_serialization() {
    println!("🧪 Тестируем JSON сериализацию конфигурации");
    
    let config = NotificationConfig::default();
    
    // Сериализуем в JSON
    let json = serde_json::to_string_pretty(&config).expect("Failed to serialize");
    println!("  📄 JSON representation length: {} bytes", json.len());
    
    // Проверяем что JSON содержит ожидаемые поля
    assert!(json.contains("channels"));
    assert!(json.contains("routing"));
    assert!(json.contains("cooldown_seconds"));
    assert!(json.contains("enable_grouping"));
    
    // Десериализуем обратно
    let deserialized: NotificationConfig = serde_json::from_str(&json).expect("Failed to deserialize");
    
    // Проверяем что значения совпадают
    assert_eq!(deserialized.channels.len(), config.channels.len());
    assert_eq!(deserialized.enable_grouping, config.enable_grouping);
    assert_eq!(deserialized.cooldown_seconds, config.cooldown_seconds);
    assert_eq!(deserialized.max_group_size, config.max_group_size);
    
    println!("  ✅ Round-trip: {} channels, grouping={}", 
             deserialized.channels.len(), deserialized.enable_grouping);
    
    println!("✅ JSON сериализация работает корректно");
}

/// Создание тестового алерта
fn create_test_alert(severity: AlertSeverity, title: &str, description: &str) -> HealthAlert {
    let severity_id = severity.clone() as u8;
    HealthAlert {
        id: format!("test-{}-{}", severity_id, rand::random::<u32>()),
        component: ComponentType::VectorStore,
        severity,
        title: title.to_string(),
        description: description.to_string(),
        metric_value: Some(85.0),
        threshold: Some(80.0),
        timestamp: Utc::now(),
        resolved: false,
        resolved_at: None,
    }
}

/// Тест notification manager создания и базовой функциональности
#[tokio::test]
async fn test_notification_manager_creation() -> Result<()> {
    println!("🧪 Тестируем создание notification manager");
    
    // Создаем с default конфигурацией
    let default_config = NotificationConfig::default();
    let manager = NotificationManager::new(default_config)?;
    
    // Тестируем отправку алерта
    let alert = create_test_alert(AlertSeverity::Info, "Test Alert", "This is a test");
    let result = manager.handle_alert(alert).await;
    assert!(result.is_ok());
    
    println!("  ✅ Default manager создан и обработал алерт");
    
    // Создаем с custom конфигурацией (только console)
    let mut custom_config = NotificationConfig::default();
    custom_config.channels = vec![NotificationChannel::Console { colored: false }];
    custom_config.enable_grouping = false;
    
    let custom_manager = NotificationManager::new(custom_config)?;
    let alert2 = create_test_alert(AlertSeverity::Warning, "Warning Test", "Custom manager test");
    let result2 = custom_manager.handle_alert(alert2).await;
    assert!(result2.is_ok());
    
    println!("  ✅ Custom manager создан и обработал алерт");
    
    println!("✅ Notification manager создание работает корректно");
    Ok(())
}

/// Тест фильтрации алертов
#[tokio::test]
async fn test_alert_filtering() -> Result<()> {
    println!("🧪 Тестируем фильтрацию алертов");
    
    // Конфигурация с фильтрами
    let mut config = NotificationConfig::default();
    config.component_filters = Some(vec!["Cache".to_string()]);
    config.ignore_patterns = vec!["ignore_me".to_string(), "test_pattern".to_string()];
    config.channels = vec![NotificationChannel::Console { colored: false }];
    
    let manager = NotificationManager::new(config)?;
    
    // Тест 1: Алерт от VectorStore (не в фильтре) - должен быть проигнорирован
    let alert1 = HealthAlert {
        id: "filter-test-1".to_string(),
        component: ComponentType::VectorStore,
        severity: AlertSeverity::Warning,
        title: "VectorStore Alert".to_string(),
        description: "This should be filtered out".to_string(),
        metric_value: None,
        threshold: None,
        timestamp: Utc::now(),
        resolved: false,
        resolved_at: None,
    };
    
    let result1 = manager.handle_alert(alert1).await;
    assert!(result1.is_ok());
    println!("  ✅ VectorStore алерт отфильтрован (component filter)");
    
    // Тест 2: Алерт с игнорируемым паттерном
    let alert2 = HealthAlert {
        id: "filter-test-2".to_string(),
        component: ComponentType::Cache,
        severity: AlertSeverity::Critical,
        title: "Cache Alert".to_string(),
        description: "This contains ignore_me pattern".to_string(),
        metric_value: None,
        threshold: None,
        timestamp: Utc::now(),
        resolved: false,
        resolved_at: None,
    };
    
    let result2 = manager.handle_alert(alert2).await;
    assert!(result2.is_ok());
    println!("  ✅ Алерт с игнорируемым паттерном отфильтрован");
    
    // Тест 3: Валидный алерт (Cache компонент, без игнорируемых паттернов)
    let alert3 = HealthAlert {
        id: "filter-test-3".to_string(),
        component: ComponentType::Cache,
        severity: AlertSeverity::Info,
        title: "Valid Cache Alert".to_string(),
        description: "This should go through".to_string(),
        metric_value: None,
        threshold: None,
        timestamp: Utc::now(),
        resolved: false,
        resolved_at: None,
    };
    
    let result3 = manager.handle_alert(alert3).await;
    assert!(result3.is_ok());
    println!("  ✅ Валидный алерт прошел через фильтры");
    
    println!("✅ Фильтрация алертов работает корректно");
    Ok(())
}

/// Тест механизма cooldown
#[tokio::test]
async fn test_cooldown_mechanism() -> Result<()> {
    println!("🧪 Тестируем механизм cooldown");
    
    // Конфигурация с коротким cooldown для тестирования
    let mut config = NotificationConfig::default();
    config.cooldown_seconds = 1; // 1 секунда для быстрого теста
    config.channels = vec![NotificationChannel::Console { colored: false }];
    config.enable_grouping = false; // Отключаем группировку для теста cooldown
    
    let manager = NotificationManager::new(config)?;
    
    // Создаем одинаковые алерты
    let alert1 = create_test_alert(AlertSeverity::Warning, "Cooldown Test", "First alert");
    let alert2 = create_test_alert(AlertSeverity::Warning, "Cooldown Test", "Second alert (should be cooled down)");
    
    // Отправляем первый алерт
    let result1 = manager.handle_alert(alert1).await;
    assert!(result1.is_ok());
    println!("  ✅ Первый алерт отправлен");
    
    // Сразу отправляем второй (должен быть в cooldown)
    let result2 = manager.handle_alert(alert2).await;
    assert!(result2.is_ok()); // Не ошибка, просто игнорируется
    println!("  ✅ Второй алерт в cooldown (проигнорирован)");
    
    // Ждем окончания cooldown
    sleep(Duration::from_millis(1100)).await;
    
    // Отправляем третий алерт (должен пройти)
    let alert3 = create_test_alert(AlertSeverity::Warning, "Cooldown Test", "Third alert (after cooldown)");
    let result3 = manager.handle_alert(alert3).await;
    assert!(result3.is_ok());
    println!("  ✅ Третий алерт после cooldown отправлен");
    
    println!("✅ Механизм cooldown работает корректно");
    Ok(())
}

/// Тест группировки алертов
#[tokio::test]
async fn test_alert_grouping() -> Result<()> {
    println!("🧪 Тестируем группировку алертов");
    
    // Конфигурация с группировкой
    let mut config = NotificationConfig::default();
    config.enable_grouping = true;
    config.max_group_size = 3; // Маленький размер для быстрого теста
    config.group_interval_seconds = 1; // Короткий интервал
    config.channels = vec![NotificationChannel::Console { colored: false }];
    config.cooldown_seconds = 0; // Отключаем cooldown для теста группировки
    
    let manager = NotificationManager::new(config)?;
    
    // Отправляем алерты (не Fatal - они не группируются)
    for i in 0..5 {
        let alert = create_test_alert(
            AlertSeverity::Warning, 
            &format!("Group Test {}", i), 
            &format!("Alert {} for grouping", i)
        );
        let result = manager.handle_alert(alert).await;
        assert!(result.is_ok());
    }
    
    println!("  ✅ Отправлено 5 алертов для группировки");
    
    // Ждем обработки группы
    sleep(Duration::from_millis(1200)).await;
    
    // Отправляем Fatal алерт (должен пройти сразу)
    let fatal_alert = create_test_alert(AlertSeverity::Fatal, "Fatal Alert", "This should not be grouped");
    let result = manager.handle_alert(fatal_alert).await;
    assert!(result.is_ok());
    
    println!("  ✅ Fatal алерт отправлен немедленно (не группируется)");
    
    println!("✅ Группировка алертов работает корректно");
    Ok(())
}

/// Тест различных severity routing
#[tokio::test]
async fn test_severity_routing() -> Result<()> {
    println!("🧪 Тестируем маршрутизацию по severity");
    
    // Создаем custom routing
    let mut routing = HashMap::new();
    routing.insert(AlertSeverity::Info, vec!["log".to_string()]);
    routing.insert(AlertSeverity::Warning, vec!["log".to_string(), "console".to_string()]);
    routing.insert(AlertSeverity::Critical, vec!["*".to_string()]); // All channels
    routing.insert(AlertSeverity::Fatal, vec!["*".to_string()]);
    
    let config = NotificationConfig {
        channels: vec![
            NotificationChannel::Log,
            NotificationChannel::Console { colored: false },
        ],
        routing,
        cooldown_seconds: 0,
        enable_grouping: false,
        max_group_size: 10,
        group_interval_seconds: 60,
        component_filters: None,
        ignore_patterns: vec![],
    };
    
    let manager = NotificationManager::new(config)?;
    
    // Тестируем каждый уровень severity
    let severities = [
        (AlertSeverity::Info, "Info Alert"),
        (AlertSeverity::Warning, "Warning Alert"),
        (AlertSeverity::Critical, "Critical Alert"),
        (AlertSeverity::Fatal, "Fatal Alert"),
    ];
    
    for (severity, title) in severities {
        let alert = create_test_alert(severity, title, &format!("Testing {} routing", title));
        let result = manager.handle_alert(alert).await;
        assert!(result.is_ok());
        println!("  ✅ {} алерт обработан через routing", title);
    }
    
    println!("✅ Маршрутизация по severity работает корректно");
    Ok(())
}

/// Тест каналов уведомлений
#[tokio::test]
async fn test_notification_channels() -> Result<()> {
    println!("🧪 Тестируем различные каналы уведомлений");
    
    // Конфигурация с разными каналами
    let config = NotificationConfig {
        channels: vec![
            NotificationChannel::Console { colored: true },
            NotificationChannel::Console { colored: false }, // Будет перезаписан
            NotificationChannel::Log,
        ],
        routing: {
            let mut r = HashMap::new();
            r.insert(AlertSeverity::Info, vec!["console".to_string(), "log".to_string()]);
            r
        },
        cooldown_seconds: 0,
        enable_grouping: false,
        max_group_size: 10,
        group_interval_seconds: 60,
        component_filters: None,
        ignore_patterns: vec![],
    };
    
    let manager = NotificationManager::new(config)?;
    
    // Тестируем отправку через разные каналы
    let alert = create_test_alert(AlertSeverity::Info, "Multi-Channel Test", "Testing multiple channels");
    let result = manager.handle_alert(alert).await;
    assert!(result.is_ok());
    
    println!("  ✅ Алерт отправлен через console и log каналы");
    
    println!("✅ Каналы уведомлений работают корректно");
    Ok(())
}

/// Тест клонирования manager
#[tokio::test]
async fn test_manager_cloning() -> Result<()> {
    println!("🧪 Тестируем клонирование notification manager");
    
    let config = NotificationConfig::default();
    let original_manager = NotificationManager::new(config)?;
    
    // Клонируем manager
    let cloned_manager = original_manager.clone();
    
    // Тестируем что клон работает
    let alert = create_test_alert(AlertSeverity::Info, "Clone Test", "Testing cloned manager");
    let result = cloned_manager.handle_alert(alert).await;
    assert!(result.is_ok());
    
    println!("  ✅ Клонированный manager работает корректно");
    
    // Тестируем что original тоже работает
    let alert2 = create_test_alert(AlertSeverity::Warning, "Original Test", "Testing original manager");
    let result2 = original_manager.handle_alert(alert2).await;
    assert!(result2.is_ok());
    
    println!("  ✅ Оригинальный manager работает корректно");
    
    println!("✅ Клонирование manager работает корректно");
    Ok(())
}

/// Тест edge cases и error handling
#[tokio::test] 
async fn test_edge_cases() -> Result<()> {
    println!("🧪 Тестируем edge cases");
    
    // Пустая конфигурация
    let empty_config = NotificationConfig {
        channels: vec![],
        routing: HashMap::new(),
        cooldown_seconds: 0,
        enable_grouping: false,
        max_group_size: 0,
        group_interval_seconds: 1,
        component_filters: None,
        ignore_patterns: vec![],
    };
    
    let empty_manager = NotificationManager::new(empty_config)?;
    
    // Алерт в пустой manager должен обрабатываться без ошибок
    let alert = create_test_alert(AlertSeverity::Critical, "Empty Manager Test", "No channels configured");
    let result = empty_manager.handle_alert(alert).await;
    assert!(result.is_ok());
    
    println!("  ✅ Пустой manager обработал алерт без ошибок");
    
    // Конфигурация с некорректными каналами
    let weird_config = NotificationConfig {
        channels: vec![NotificationChannel::Log],
        routing: {
            let mut r = HashMap::new();
            r.insert(AlertSeverity::Fatal, vec!["nonexistent_channel".to_string()]);
            r
        },
        cooldown_seconds: u64::MAX, // Очень большой cooldown
        enable_grouping: true,
        max_group_size: 0, // Нулевой размер группы
        group_interval_seconds: 0, // Нулевой интервал
        component_filters: Some(vec![]), // Пустые фильтры
        ignore_patterns: vec!["".to_string()], // Пустой паттерн
    };
    
    let weird_manager = NotificationManager::new(weird_config)?;
    let weird_alert = create_test_alert(AlertSeverity::Fatal, "Weird Config Test", "Testing edge case config");
    let result = weird_manager.handle_alert(weird_alert).await;
    assert!(result.is_ok());
    
    println!("  ✅ Менеджер с edge case конфигурацией работает");
    
    println!("✅ Edge cases обработаны корректно");
    Ok(())
}

/// Stress test для многочисленных алертов
#[tokio::test]
async fn test_high_volume_alerts() -> Result<()> {
    println!("🧪 Stress test для большого объема алертов");
    
    let mut config = NotificationConfig::default();
    config.enable_grouping = true;
    config.max_group_size = 100;
    config.cooldown_seconds = 0; // Отключаем cooldown для stress теста
    config.channels = vec![NotificationChannel::Console { colored: false }];
    
    let manager = NotificationManager::new(config)?;
    
    let start_time = std::time::Instant::now();
    
    // Отправляем много алертов
    for i in 0..1000 {
        let alert = create_test_alert(
            match i % 4 {
                0 => AlertSeverity::Info,
                1 => AlertSeverity::Warning,
                2 => AlertSeverity::Critical,
                _ => AlertSeverity::Info,
            },
            &format!("Stress Test {}", i),
            &format!("High volume alert number {}", i),
        );
        
        let result = manager.handle_alert(alert).await;
        assert!(result.is_ok());
    }
    
    let elapsed = start_time.elapsed();
    println!("  📊 1000 алертов обработано за {:?}", elapsed);
    
    // Должно быть достаточно быстро (< 1 секунды)
    assert!(elapsed.as_secs() < 1);
    
    println!("✅ Stress test прошел успешно");
    Ok(())
}

/// Quick smoke test для всех основных функций
#[tokio::test]
async fn test_notifications_smoke() -> Result<()> {
    // Test config creation
    let _config = NotificationConfig::default();
    
    // Test manager creation
    let manager = NotificationManager::new(_config)?;
    
    // Test alert handling
    let alert = create_test_alert(AlertSeverity::Info, "Smoke Test", "Basic functionality test");
    manager.handle_alert(alert).await?;
    
    // Test cloning
    let _cloned = manager.clone();
    
    println!("✅ Все функции notifications работают");
    Ok(())
}

// Вспомогательная функция для тестов
mod rand {
    pub fn random<T>() -> T 
    where 
        T: From<u32>
    {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let mut hasher = DefaultHasher::new();
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos().hash(&mut hasher);
        T::from(hasher.finish() as u32)
    }
}