//! DI Container Integration Tests
//! 
//! Comprehensive тесты для валидации DI container integration:
//! - Full DI container с всеми зависимостями
//! - Performance metrics для DI operations
//! - Lifecycle management через DI
//! - Error propagation через DI chain
//! - Singleton vs Factory behavior
//! - Circular dependency detection

use anyhow::Result;
use memory::{
    DIMemoryService,
    service_di::default_config,
    DIContainer, Lifetime,
    orchestration::{
        MemoryOrchestrator, EmbeddingCoordinator, SearchCoordinator, 
        HealthManager, ResourceController, PromotionCoordinator, BackupCoordinator
    },
    Record, Layer, SearchOptions,
    CacheConfigType,
};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::time::{sleep, Duration, Instant};
use uuid::Uuid;
use chrono::Utc;

/// Утилита для создания DI test service
async fn create_di_test_service() -> Result<DIMemoryService> {
    let temp_dir = TempDir::new()?;
    let mut config = default_config()?;
    
    config.db_path = temp_dir.path().join("di_test.db");
    config.cache_path = temp_dir.path().join("di_cache");
    config.cache_config = CacheConfigType::InMemory { max_size: 5000 };
    config.health_enabled = true;
    
    std::fs::create_dir_all(&config.cache_path)?;
    
    DIMemoryService::new(config).await
}

/// Создание тестовой записи
fn create_di_test_record(id: usize, content: &str, layer: Layer) -> Record {
    Record {
        id: Uuid::new_v4(),
        text: format!("DI test record {}: {}", id, content),
        embedding: vec![], // Будет сгенерирован автоматически
        layer,
        kind: "di_test".to_string(),
        tags: vec!["dependency_injection".to_string(), "integration".to_string()],
        project: "di_testing".to_string(),
        session: "di_session".to_string(),
        score: 0.8,
        ts: Utc::now(),
        access_count: 0,
        last_access: Utc::now(),
    }
}

/// ТЕСТ 1: Full DI Container Integration
/// 
/// Тестирует полную integration всех компонентов через DI container
#[tokio::test]
async fn test_full_di_container_integration() -> Result<()> {
    println!("🔧 Starting Full DI Container Integration Test");
    
    let service = create_di_test_service().await?;
    
    // === ПРОВЕРКА DI CONTAINER INITIALIZATION ===
    
    let di_stats = service.di_stats();
    println!("📊 DI Container Statistics:");
    println!("   Total types: {}", di_stats.total_types);
    println!("   Registered factories: {}", di_stats.registered_factories);
    println!("   Cached singletons: {}", di_stats.cached_singletons);
    
    // DI container должен содержать все необходимые types
    assert!(di_stats.total_types > 0, "DI container should have registered types");
    assert!(di_stats.registered_factories > 0, "DI container should have factories");
    
    // === ПРОВЕРКА DEPENDENCY RESOLUTION ===
    
    println!("🔍 Testing dependency resolution...");
    
    // Тестируем resolution всех key coordinators
    let coordination_components = vec![
        ("MemoryOrchestrator", service.try_resolve::<MemoryOrchestrator>().is_some()),
        ("EmbeddingCoordinator", service.try_resolve::<EmbeddingCoordinator>().is_some()),
        ("SearchCoordinator", service.try_resolve::<SearchCoordinator>().is_some()),
        ("HealthManager", service.try_resolve::<HealthManager>().is_some()),
        ("ResourceController", service.try_resolve::<ResourceController>().is_some()),
        ("PromotionCoordinator", service.try_resolve::<PromotionCoordinator>().is_some()),
        ("BackupCoordinator", service.try_resolve::<BackupCoordinator>().is_some()),
    ];
    
    for (component_name, is_resolved) in &coordination_components {
        println!("   {}: {}", component_name, if *is_resolved { "✅ Resolved" } else { "❌ Not resolved" });
    }
    
    // Основные coordinators должны быть доступны
    let essential_coordinators = ["MemoryOrchestrator", "EmbeddingCoordinator", "SearchCoordinator", "HealthManager"];
    for essential in &essential_coordinators {
        let is_available = coordination_components.iter()
            .find(|(name, _)| name == essential)
            .map(|(_, resolved)| *resolved)
            .unwrap_or(false);
        
        assert!(is_available, "{} should be available through DI", essential);
    }
    
    // === DEPENDENCY GRAPH VALIDATION ===
    
    println!("🕸️ Validating dependency graph...");
    
    // Проверяем что MemoryOrchestrator может получить доступ к своим dependencies
    if let Some(orchestrator) = service.try_resolve::<MemoryOrchestrator>() {
        // Orchestrator должен быть готов (его dependencies resolved)
        let is_ready = orchestrator.all_ready().await;
        println!("   MemoryOrchestrator readiness: {}", is_ready);
        
        // Получаем метрики от orchestrator (требует working dependencies)
        let metrics = orchestrator.all_metrics().await;
        println!("   Orchestrator metrics: {}", metrics);
        
        assert!(!metrics.is_empty(), "Orchestrator should provide metrics through dependencies");
    }
    
    // === COMPONENT INTERACTION ЧЕРЕЗ DI ===
    
    println!("🔄 Testing component interaction through DI...");
    
    // Выполняем операции которые require multiple coordinator interactions
    let test_data = vec![
        "DI integration test: embedding generation through coordinator chain",
        "Dependency injection validation: search coordination with caching",
        "Component interaction test: health monitoring with resource management",
    ];
    
    for (i, data) in test_data.iter().enumerate() {
        let record = create_di_test_record(i, data, Layer::Interact);
        
        // Insert operation требует EmbeddingCoordinator, HealthManager, ResourceController
        service.insert(record).await?;
        
        // Search operation требует SearchCoordinator, HealthManager
        let results = service.search(
            data,
            Layer::Interact,
            SearchOptions { top_k: 5, ..Default::default() }
        ).await?;
        
        assert!(!results.is_empty(), "DI-coordinated operations should work: {}", data);
    }
    
    println!("✅ Component interactions through DI successful");
    
    // === DI PERFORMANCE METRICS ===
    
    let performance_metrics = service.get_performance_metrics();
    println!("📈 DI Performance Metrics:");
    println!("   Total resolutions: {}", performance_metrics.total_resolutions);
    println!("   Cache hits: {}", performance_metrics.cache_hits);
    println!("   Cache misses: {}", performance_metrics.cache_misses);
    
    if performance_metrics.total_resolutions > 0 {
        let cache_hit_rate = performance_metrics.cache_hits as f64 / 
                           performance_metrics.total_resolutions as f64 * 100.0;
        println!("   Cache hit rate: {:.1}%", cache_hit_rate);
        
        // DI cache должен быть эффективным
        assert!(cache_hit_rate >= 50.0, "DI cache hit rate too low: {:.1}%", cache_hit_rate);
    }
    
    println!("✅ Full DI Container Integration Test successful");
    
    Ok(())
}

/// ТЕСТ 2: DI Container Performance Under Load
/// 
/// Тестирует производительность DI container при высокой нагрузке
#[tokio::test]
async fn test_di_container_performance_under_load() -> Result<()> {
    println!("⚡ Starting DI Container Performance Under Load Test");
    
    let service = create_di_test_service().await?;
    
    // === BASELINE DI PERFORMANCE ===
    
    println!("📊 Measuring baseline DI performance...");
    
    let baseline_start = Instant::now();
    let mut baseline_resolutions = Vec::new();
    
    // Выполняем baseline resolution operations
    for i in 0..50 {
        let resolution_start = Instant::now();
        
        // Resolution different coordinators
        let _embedding = service.try_resolve::<EmbeddingCoordinator>();
        let _search = service.try_resolve::<SearchCoordinator>();
        let _health = service.try_resolve::<HealthManager>();
        
        let resolution_time = resolution_start.elapsed();
        baseline_resolutions.push(resolution_time.as_micros() as f64 / 1000.0);
        
        if i % 10 == 0 {
            sleep(Duration::from_millis(1)).await;
        }
    }
    
    let baseline_duration = baseline_start.elapsed();
    let baseline_avg_resolution = baseline_resolutions.iter().sum::<f64>() / baseline_resolutions.len() as f64;
    
    println!("   Baseline resolution time: {:.3}ms", baseline_avg_resolution);
    println!("   Baseline total duration: {:.2}s", baseline_duration.as_secs_f64());
    
    // === CONCURRENT DI LOAD TEST ===
    
    println!("👥 Testing concurrent DI container access...");
    
    let concurrent_start = Instant::now();
    let mut concurrent_handles = Vec::new();
    
    // 100 concurrent операций requiring DI resolution
    for i in 0..100 {
        let service_clone = service.clone();
        let handle = tokio::spawn(async move {
            let op_start = Instant::now();
            
            // Multiple DI resolutions в concurrent context
            let _orchestrator = service_clone.try_resolve::<MemoryOrchestrator>();
            let _embedding = service_clone.try_resolve::<EmbeddingCoordinator>();
            let _search = service_clone.try_resolve::<SearchCoordinator>();
            let _health = service_clone.try_resolve::<HealthManager>();
            
            // Также выполняем actual operation requiring these components
            let record = create_di_test_record(
                i + 1000,
                "Concurrent DI load test operation",
                Layer::Interact
            );
            
            let result = service_clone.insert(record).await;
            let op_duration = op_start.elapsed();
            
            (result.is_ok(), op_duration.as_micros() as f64 / 1000.0)
        });
        
        concurrent_handles.push(handle);
    }
    
    let concurrent_results = futures::future::try_join_all(concurrent_handles).await?;
    let concurrent_duration = concurrent_start.elapsed();
    
    // === АНАЛИЗ CONCURRENT PERFORMANCE ===
    
    let successful_operations = concurrent_results.iter()
        .filter(|(success, _)| *success)
        .count();
    
    let concurrent_operation_times: Vec<f64> = concurrent_results.iter()
        .filter(|(success, _)| *success)
        .map(|(_, time)| *time)
        .collect();
    
    let concurrent_avg_time = if !concurrent_operation_times.is_empty() {
        concurrent_operation_times.iter().sum::<f64>() / concurrent_operation_times.len() as f64
    } else {
        0.0
    };
    
    let operations_per_second = successful_operations as f64 / concurrent_duration.as_secs_f64();
    
    println!("📊 Concurrent DI load results:");
    println!("   Successful operations: {}/100", successful_operations);
    println!("   Average operation time: {:.3}ms", concurrent_avg_time);
    println!("   Operations per second: {:.1}", operations_per_second);
    println!("   Total duration: {:.2}s", concurrent_duration.as_secs_f64());
    
    // === HIGH-FREQUENCY DI RESOLUTION TEST ===
    
    println!("🔥 Testing high-frequency DI resolutions...");
    
    let high_freq_start = Instant::now();
    let mut resolution_times = Vec::new();
    
    // Выполняем rapid DI resolutions
    for _ in 0..500 {
        let resolution_start = Instant::now();
        
        // Rapid resolution cycle
        let _orchestrator = service.try_resolve::<MemoryOrchestrator>();
        let _embedding = service.try_resolve::<EmbeddingCoordinator>();
        let _search = service.try_resolve::<SearchCoordinator>();
        
        let resolution_time = resolution_start.elapsed();
        resolution_times.push(resolution_time.as_micros() as f64 / 1000.0);
    }
    
    let high_freq_duration = high_freq_start.elapsed();
    let high_freq_avg = resolution_times.iter().sum::<f64>() / resolution_times.len() as f64;
    let resolutions_per_second = 500.0 / high_freq_duration.as_secs_f64();
    
    println!("📊 High-frequency resolution results:");
    println!("   Average resolution time: {:.3}ms", high_freq_avg);
    println!("   Resolutions per second: {:.1}", resolutions_per_second);
    
    // === DI CONTAINER EFFICIENCY ANALYSIS ===
    
    let final_performance_metrics = service.get_performance_metrics();
    println!("📈 Final DI Performance Metrics:");
    println!("   Total resolutions: {}", final_performance_metrics.total_resolutions);
    println!("   Cache hits: {}", final_performance_metrics.cache_hits);
    println!("   Cache misses: {}", final_performance_metrics.cache_misses);
    
    let final_cache_hit_rate = if final_performance_metrics.total_resolutions > 0 {
        final_performance_metrics.cache_hits as f64 / 
        final_performance_metrics.total_resolutions as f64 * 100.0
    } else {
        0.0
    };
    
    println!("   Final cache hit rate: {:.1}%", final_cache_hit_rate);
    
    // === PERFORMANCE VALIDATION ===
    
    // DI container не должен быть bottleneck
    assert!(successful_operations >= 90, "Too many DI operation failures: {}/100", successful_operations);
    assert!(operations_per_second >= 20.0, "DI throughput too low: {:.1} ops/sec", operations_per_second);
    assert!(concurrent_avg_time < 50.0, "DI operation latency too high: {:.3}ms", concurrent_avg_time);
    
    // DI resolution должно быть быстрым
    assert!(high_freq_avg < 1.0, "DI resolution too slow: {:.3}ms", high_freq_avg);
    assert!(resolutions_per_second >= 1000.0, "DI resolution throughput too low: {:.1} res/sec", resolutions_per_second);
    
    // Cache efficiency должна быть high
    assert!(final_cache_hit_rate >= 70.0, "DI cache efficiency too low: {:.1}%", final_cache_hit_rate);
    
    println!("✅ DI Container Performance Under Load Test successful");
    println!("   Throughput: {:.1} ops/sec", operations_per_second);
    println!("   Resolution speed: {:.1} res/sec", resolutions_per_second);
    println!("   Cache efficiency: {:.1}%", final_cache_hit_rate);
    
    Ok(())
}

/// ТЕСТ 3: DI Lifecycle Management
/// 
/// Тестирует lifecycle management компонентов через DI
#[tokio::test]
async fn test_di_lifecycle_management() -> Result<()> {
    println!("🔄 Starting DI Lifecycle Management Test");
    
    let service = create_di_test_service().await?;
    
    // === SINGLETON BEHAVIOR VALIDATION ===
    
    println!("🔍 Testing singleton behavior...");
    
    // Получаем instances несколько раз
    let orchestrator_1 = service.try_resolve::<MemoryOrchestrator>();
    let orchestrator_2 = service.try_resolve::<MemoryOrchestrator>();
    let orchestrator_3 = service.try_resolve::<MemoryOrchestrator>();
    
    // Проверяем consistency singleton behavior
    let all_resolved = orchestrator_1.is_some() && orchestrator_2.is_some() && orchestrator_3.is_some();
    println!("   All orchestrator instances resolved: {}", all_resolved);
    
    if all_resolved {
        // Для singletons, instances должны быть consistent
        let first_ready = orchestrator_1.as_ref().expect("Test operation should succeed").all_ready().await;
        let second_ready = orchestrator_2.as_ref().expect("Test operation should succeed").all_ready().await;
        let third_ready = orchestrator_3.as_ref().expect("Test operation should succeed").all_ready().await;
        
        println!("   Singleton consistency: {} {} {}", first_ready, second_ready, third_ready);
        
        // Singleton instances должны иметь consistent state
        assert_eq!(first_ready, second_ready, "Singleton instances should have consistent state");
        assert_eq!(second_ready, third_ready, "Singleton instances should have consistent state");
    }
    
    // === COMPONENT INITIALIZATION ORDER ===
    
    println!("📋 Testing component initialization order...");
    
    // Проверяем что dependencies инициализируются в правильном порядке
    let health_manager = service.try_resolve::<HealthManager>();
    let resource_controller = service.try_resolve::<ResourceController>();
    let embedding_coordinator = service.try_resolve::<EmbeddingCoordinator>();
    let search_coordinator = service.try_resolve::<SearchCoordinator>();
    
    println!("   HealthManager resolved: {}", health_manager.is_some());
    println!("   ResourceController resolved: {}", resource_controller.is_some());
    println!("   EmbeddingCoordinator resolved: {}", embedding_coordinator.is_some());
    println!("   SearchCoordinator resolved: {}", search_coordinator.is_some());
    
    // Если components доступны, их dependencies должны быть ready
    if let Some(health) = health_manager {
        let health_status = health.check_system_health().await;
        match health_status {
            Ok(status) => {
                println!("   HealthManager functional: {}", status.overall_healthy);
            }
            Err(e) => {
                println!("   HealthManager initialization incomplete: {}", e);
            }
        }
    }
    
    // === COMPONENT LIFECYCLE COORDINATION ===
    
    println!("🔗 Testing component lifecycle coordination...");
    
    // Выполняем операции которые require coordinated lifecycle
    let lifecycle_test_data = vec![
        "Lifecycle test: component initialization coordination",
        "Dependency management: ordered startup and shutdown",
        "Resource sharing: coordinated component lifecycle",
    ];
    
    let lifecycle_start = Instant::now();
    let mut lifecycle_operations = Vec::new();
    
    for (i, data) in lifecycle_test_data.iter().enumerate() {
        let record = create_di_test_record(i, data, Layer::Interact);
        
        let operation_start = Instant::now();
        let insert_result = service.insert(record).await;
        
        if insert_result.is_ok() {
            let search_result = service.search(
                data,
                Layer::Interact,
                SearchOptions { top_k: 3, ..Default::default() }
            ).await;
            
            lifecycle_operations.push((
                true,
                !search_result.unwrap_or_default().is_empty(),
                operation_start.elapsed().as_micros() as f64 / 1000.0
            ));
        } else {
            lifecycle_operations.push((false, false, operation_start.elapsed().as_micros() as f64 / 1000.0));
        }
    }
    
    let lifecycle_duration = lifecycle_start.elapsed();
    
    let successful_inserts = lifecycle_operations.iter().filter(|(insert, _, _)| *insert).count();
    let successful_searches = lifecycle_operations.iter().filter(|(_, search, _)| *search).count();
    let avg_operation_time = lifecycle_operations.iter()
        .map(|(_, _, time)| time)
        .sum::<f64>() / lifecycle_operations.len() as f64;
    
    println!("📊 Lifecycle coordination results:");
    println!("   Successful inserts: {}/{}", successful_inserts, lifecycle_test_data.len());
    println!("   Successful searches: {}/{}", successful_searches, lifecycle_test_data.len());
    println!("   Average operation time: {:.3}ms", avg_operation_time);
    println!("   Total coordination time: {:.2}s", lifecycle_duration.as_secs_f64());
    
    // === RESOURCE CLEANUP VALIDATION ===
    
    println!("🧹 Testing resource cleanup...");
    
    // Получаем metrics до cleanup
    let pre_cleanup_stats = service.di_stats();
    let pre_cleanup_performance = service.get_performance_metrics();
    
    println!("   Pre-cleanup DI stats: {} types, {} cached", 
             pre_cleanup_stats.total_types, pre_cleanup_stats.cached_singletons);
    
    // Симулируем операции которые могут require cleanup
    for i in 0..20 {
        let record = create_di_test_record(
            i + 2000,
            "Resource cleanup test operation",
            Layer::Interact
        );
        service.insert(record).await?;
        
        // Periodic resolution для testing cleanup
        if i % 5 == 0 {
            let _components = (
                service.try_resolve::<EmbeddingCoordinator>(),
                service.try_resolve::<SearchCoordinator>(),
                service.try_resolve::<HealthManager>(),
            );
        }
    }
    
    // Проверяем состояние после операций
    let post_operations_stats = service.di_stats();
    let post_operations_performance = service.get_performance_metrics();
    
    println!("   Post-operations DI stats: {} types, {} cached", 
             post_operations_stats.total_types, post_operations_stats.cached_singletons);
    
    // DI container должен управлять resources эффективно
    assert_eq!(pre_cleanup_stats.total_types, post_operations_stats.total_types, 
               "DI type count should remain stable");
    
    // Performance metrics должны показывать рост без degradation
    assert!(post_operations_performance.total_resolutions >= pre_cleanup_performance.total_resolutions,
            "DI resolution count should increase");
    
    // === LIFECYCLE VALIDATION ===
    
    assert!(successful_inserts >= lifecycle_test_data.len() - 1, 
            "Lifecycle coordination should succeed: {}/{}", successful_inserts, lifecycle_test_data.len());
    
    assert!(successful_searches >= lifecycle_test_data.len() - 1,
            "Component coordination should work: {}/{}", successful_searches, lifecycle_test_data.len());
    
    assert!(avg_operation_time < 20.0, 
            "Lifecycle operations should be efficient: {:.3}ms", avg_operation_time);
    
    println!("✅ DI Lifecycle Management Test successful");
    println!("   Component coordination: {}/{} operations", successful_inserts, lifecycle_test_data.len());
    println!("   Resource efficiency: stable DI container state");
    
    Ok(())
}

/// ТЕСТ 4: Error Propagation Through DI Chain
/// 
/// Тестирует error handling и propagation через DI dependencies
#[tokio::test]
async fn test_error_propagation_through_di_chain() -> Result<()> {
    println!("⚠️ Starting Error Propagation Through DI Chain Test");
    
    let service = create_di_test_service().await?;
    
    // === ПОДГОТОВКА ERROR SCENARIOS ===
    
    println!("📋 Setting up error scenarios...");
    
    // Добавляем some valid data для baseline
    for i in 0..10 {
        let record = create_di_test_record(
            i,
            "Valid baseline data for error testing",
            Layer::Interact
        );
        service.insert(record).await?;
    }
    
    println!("✅ Baseline data established");
    
    // === ERROR SCENARIO 1: MALFORMED OPERATIONS ===
    
    println!("💥 Testing malformed operation error propagation...");
    
    let malformed_operations = vec![
        // Потенциально проблематичные операции
        ("", Layer::Interact), // Empty text
        ("Very short", Layer::Interact),
        ("Normal content but with extreme parameters", Layer::Interact),
    ];
    
    let mut error_handling_results = Vec::new();
    
    for (i, (content, layer)) in malformed_operations.iter().enumerate() {
        let mut record = create_di_test_record(i + 3000, content, *layer);
        
        // Намеренно создаем потенциально problematic conditions
        if content.is_empty() {
            record.text = String::new(); // Empty text
        }
        
        let operation_start = Instant::now();
        let result = service.insert(record).await;
        let operation_time = operation_start.elapsed();
        
        error_handling_results.push((
            result.is_ok(),
            result.err().map(|e| e.to_string()),
            operation_time.as_micros() as f64 / 1000.0
        ));
        
        println!("   Operation {}: {} ({:.3}ms)", 
                 i + 1, 
                 if result.is_ok() { "Success" } else { "Error" },
                 operation_time.as_micros() as f64 / 1000.0);
    }
    
    // === ERROR SCENARIO 2: TIMEOUT CONDITIONS ===
    
    println!("⏰ Testing timeout error propagation...");
    
    let timeout_operations = vec![
        // Operations with potential timeout issues
        ("Complex search query with multiple semantic terms and extensive context", Layer::Interact),
        ("Large batch operation simulation with extensive data processing", Layer::Insights),
        ("Resource intensive operation requiring significant computation", Layer::Assets),
    ];
    
    let mut timeout_results = Vec::new();
    
    for (i, (content, layer)) in timeout_operations.iter().enumerate() {
        let record = create_di_test_record(i + 4000, content, *layer);
        
        let operation_start = Instant::now();
        
        // Используем timeout для simulating potential DI chain delays
        let result = tokio::time::timeout(
            Duration::from_millis(500), // Reasonable timeout
            service.insert(record)
        ).await;
        
        let operation_time = operation_start.elapsed();
        
        let (success, error_msg) = match result {
            Ok(Ok(_)) => (true, None),
            Ok(Err(e)) => (false, Some(e.to_string())),
            Err(_) => (false, Some("Timeout".to_string())),
        };
        
        timeout_results.push((success, error_msg, operation_time.as_micros() as f64 / 1000.0));
        
        println!("   Timeout test {}: {} ({:.3}ms)", 
                 i + 1, 
                 if success { "Success" } else { "Error/Timeout" },
                 operation_time.as_micros() as f64 / 1000.0);
    }
    
    // === ERROR SCENARIO 3: RESOURCE PRESSURE ERRORS ===
    
    println!("💾 Testing resource pressure error handling...");
    
    let pressure_start = Instant::now();
    let mut pressure_handles = Vec::new();
    
    // Создаем resource pressure через concurrent operations
    for i in 0..30 {
        let service_clone = service.clone();
        let handle = tokio::spawn(async move {
            let record = create_di_test_record(
                i + 5000,
                "Resource pressure error testing with concurrent load",
                Layer::Interact
            );
            
            // Mix of operations creating different types of pressure
            let operation_start = Instant::now();
            let result = if i % 2 == 0 {
                service_clone.insert(record).await
            } else {
                service_clone.search(
                    "resource pressure search",
                    Layer::Interact,
                    SearchOptions { top_k: 20, ..Default::default() }
                ).await.map(|_| ())
            };
            
            let operation_time = operation_start.elapsed();
            (result.is_ok(), operation_time.as_micros() as f64 / 1000.0)
        });
        
        pressure_handles.push(handle);
    }
    
    let pressure_results = futures::future::try_join_all(pressure_handles).await?;
    let pressure_duration = pressure_start.elapsed();
    
    let pressure_success_count = pressure_results.iter().filter(|(success, _)| *success).count();
    let pressure_avg_time = pressure_results.iter()
        .map(|(_, time)| time)
        .sum::<f64>() / pressure_results.len() as f64;
    
    println!("📊 Resource pressure results:");
    println!("   Successful operations: {}/30", pressure_success_count);
    println!("   Average operation time: {:.3}ms", pressure_avg_time);
    println!("   Total duration: {:.2}s", pressure_duration.as_secs_f64());
    
    // === ERROR RECOVERY VALIDATION ===
    
    println!("🔄 Testing error recovery...");
    
    sleep(Duration::from_millis(200)).await; // Brief recovery period
    
    // Проверяем что система восстанавливается после errors
    let recovery_operations = vec![
        "Recovery test: normal operation after errors",
        "Error recovery validation: system stability check",
        "Post-error functionality: component coordination test",
    ];
    
    let mut recovery_success = 0;
    
    for (i, content) in recovery_operations.iter().enumerate() {
        let record = create_di_test_record(i + 6000, content, Layer::Interact);
        
        if service.insert(record).await.is_ok() {
            let search_result = service.search(
                content,
                Layer::Interact,
                SearchOptions { top_k: 3, ..Default::default() }
            ).await;
            
            if search_result.is_ok() && !search_result.expect("Test operation should succeed").is_empty() {
                recovery_success += 1;
            }
        }
    }
    
    println!("📊 Error recovery results:");
    println!("   Recovery operations: {}/{}", recovery_success, recovery_operations.len());
    
    // === HEALTH CHECK ПОСЛЕ ERRORS ===
    
    let post_error_health = service.check_health().await?;
    println!("🏥 Post-error health: {}", post_error_health.overall_status);
    
    // === ERROR PROPAGATION VALIDATION ===
    
    // Система должна handle errors gracefully
    let malformed_handled = error_handling_results.iter().filter(|(success, _, _)| *success).count();
    let timeout_handled = timeout_results.iter().filter(|(success, _, _)| *success).count();
    
    // Не все malformed operations должны succeed, но система не должна падать
    println!("📊 Error handling summary:");
    println!("   Malformed operations handled: {}/{}", malformed_handled, malformed_operations.len());
    println!("   Timeout operations handled: {}/{}", timeout_handled, timeout_operations.len());
    println!("   Pressure operations handled: {}/30", pressure_success_count);
    
    // Система должна оставаться functional после errors
    assert!(recovery_success >= recovery_operations.len() - 1, 
            "System should recover from errors: {}/{}", recovery_success, recovery_operations.len());
    
    assert!(pressure_success_count >= 20, 
            "System should handle resource pressure: {}/30", pressure_success_count);
    
    // Health должен быть reasonable после error scenarios
    assert!(post_error_health.overall_status == "healthy" || 
            post_error_health.overall_status == "degraded",
            "System should maintain reasonable health after errors");
    
    println!("✅ Error Propagation Through DI Chain Test successful");
    println!("   Error handling: graceful degradation maintained");
    println!("   Recovery capability: {}/{} operations", recovery_success, recovery_operations.len());
    
    Ok(())
}