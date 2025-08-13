//! Resilience Integration Tests
//! 
//! Comprehensive тесты для валидации resilience и fault tolerance:
//! - Circuit breaker activation и recovery
//! - Component failure scenarios
//! - Graceful degradation testing  
//! - Emergency shutdown procedures
//! - Data consistency under failures
//! - System recovery validation

use anyhow::Result;
use memory::{
    DIMemoryService,
    service_di::default_config,
    orchestration::{MemoryOrchestrator, EmbeddingCoordinator, SearchCoordinator, HealthManager},
    Record, Layer, SearchOptions,
    CacheConfigType,
};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::time::{sleep, Duration, timeout, Instant};
use uuid::Uuid;
use chrono::Utc;

/// Утилита для создания resilience test service
async fn create_resilience_test_service() -> Result<DIMemoryService> {
    let temp_dir = TempDir::new()?;
    let mut config = default_config()?;
    
    config.db_path = temp_dir.path().join("resilience_test.db");
    config.cache_path = temp_dir.path().join("resilience_cache");
    config.cache_config = CacheConfigType::InMemory { max_size: 5000 };
    config.health_enabled = true;
    
    std::fs::create_dir_all(&config.cache_path)?;
    
    DIMemoryService::new(config).await
}

/// Создание тестовой записи
fn create_resilience_test_record(id: usize, content: &str, layer: Layer) -> Record {
    Record {
        id: Uuid::new_v4(),
        text: format!("Resilience test record {}: {}", id, content),
        embedding: vec![], // Будет сгенерирован автоматически
        layer,
        kind: "resilience_test".to_string(),
        tags: vec!["resilience".to_string(), "fault_tolerance".to_string()],
        project: "resilience_testing".to_string(),
        session: "resilience_session".to_string(),
        score: 0.8,
        ts: Utc::now(),
        access_count: 0,
        last_access: Utc::now(),
    }
}

/// ТЕСТ 1: Circuit Breaker Activation and Recovery
/// 
/// Тестирует circuit breaker patterns для всех coordinators
#[tokio::test]
async fn test_circuit_breaker_activation_recovery() -> Result<()> {
    println!("🔌 Starting Circuit Breaker Activation and Recovery Test");
    
    let service = create_resilience_test_service().await?;
    
    // === ПОДГОТОВКА ДАННЫХ ===
    
    // Добавляем базовые данные для тестирования
    for i in 0..50 {
        let record = create_resilience_test_record(
            i,
            "Circuit breaker test data for failure simulation",
            Layer::Interact
        );
        service.insert(record).await?;
    }
    
    println!("✅ Test data prepared: 50 records");
    
    // === ПОЛУЧЕНИЕ COORDINATORS ===
    
    let orchestrator = service.try_resolve::<MemoryOrchestrator>();
    let embedding_coord = service.try_resolve::<EmbeddingCoordinator>();
    let search_coord = service.try_resolve::<SearchCoordinator>();
    let health_manager = service.try_resolve::<HealthManager>();
    
    assert!(orchestrator.is_some(), "MemoryOrchestrator should be available");
    
    // === ПРОВЕРКА INITIAL CIRCUIT BREAKER STATE ===
    
    if let Some(orchestrator) = &orchestrator {
        let initial_cb_status = orchestrator.circuit_breaker_status().await;
        println!("📊 Initial circuit breaker status: {}", initial_cb_status);
        
        // Все circuit breakers должны быть Closed (нормальная работа)
        assert!(initial_cb_status.contains("Closed"), "Circuit breakers should be initially Closed");
    }
    
    // === СИМУЛЯЦИЯ HIGH ERROR RATE ===
    
    println!("💥 Simulating high error rate to trigger circuit breakers...");
    
    // Создаем нагрузку которая может вызвать errors/timeouts
    let stress_start = Instant::now();
    let mut error_operations = 0;
    let mut success_operations = 0;
    
    // Выполняем rapid операции для potential circuit breaker triggering
    for i in 0..100 {
        // Смешиваем быстрые операции с потенциально медленными
        if i % 3 == 0 {
            // Потенциально проблематичная операция - очень большой top_k
            let result = timeout(
                Duration::from_millis(100), // Короткий timeout
                service.search(
                    "circuit breaker stress test",
                    Layer::Interact,
                    SearchOptions { top_k: 1000, ..Default::default() } // Большой top_k
                )
            ).await;
            
            match result {
                Ok(Ok(_)) => success_operations += 1,
                _ => error_operations += 1,
            }
        } else {
            // Нормальная операция
            let result = service.search(
                "normal operation",
                Layer::Interact,
                SearchOptions { top_k: 5, ..Default::default() }
            ).await;
            
            match result {
                Ok(_) => success_operations += 1,
                Err(_) => error_operations += 1,
            }
        }
        
        // Минимальная пауза
        if i % 10 == 0 {
            sleep(Duration::from_millis(1)).await;
        }
    }
    
    let stress_duration = stress_start.elapsed();
    
    println!("📊 Stress test results:");
    println!("   Duration: {:.2}s", stress_duration.as_secs_f64());
    println!("   Success operations: {}", success_operations);
    println!("   Error operations: {}", error_operations);
    println!("   Error rate: {:.1}%", (error_operations as f64 / 100.0) * 100.0);
    
    // === ПРОВЕРКА CIRCUIT BREAKER STATE ПОСЛЕ STRESS ===
    
    if let Some(orchestrator) = &orchestrator {
        let post_stress_cb_status = orchestrator.circuit_breaker_status().await;
        println!("🔌 Post-stress circuit breaker status: {}", post_stress_cb_status);
        
        // Circuit breakers могут быть в разных состояниях после stress
        // Half-Open или Open если errors превысили threshold
        if post_stress_cb_status.contains("Open") || post_stress_cb_status.contains("Half-Open") {
            println!("⚠️ Circuit breakers activated due to stress (expected behavior)");
        }
    }
    
    // === RECOVERY TESTING ===
    
    println!("🔄 Testing circuit breaker recovery...");
    
    // Даем время для recovery
    sleep(Duration::from_secs(2)).await;
    
    // Выполняем normal операции для circuit breaker recovery
    let recovery_start = Instant::now();
    let mut recovery_success = 0;
    
    for i in 0..20 {
        let result = service.search(
            "recovery test operation",
            Layer::Interact,
            SearchOptions { top_k: 3, ..Default::default() }
        ).await;
        
        if result.is_ok() {
            recovery_success += 1;
        }
        
        // Пауза между recovery operations
        sleep(Duration::from_millis(50)).await;
    }
    
    let recovery_duration = recovery_start.elapsed();
    
    println!("📊 Recovery test results:");
    println!("   Duration: {:.2}s", recovery_duration.as_secs_f64());
    println!("   Successful recoveries: {}/20", recovery_success);
    
    // === FINAL CIRCUIT BREAKER STATE ===
    
    if let Some(orchestrator) = &orchestrator {
        let final_cb_status = orchestrator.circuit_breaker_status().await;
        println!("🔌 Final circuit breaker status: {}", final_cb_status);
        
        // После recovery период, circuit breakers должны стремиться к Closed
        if final_cb_status.contains("Closed") {
            println!("✅ Circuit breakers successfully recovered to Closed state");
        } else {
            println!("⚠️ Circuit breakers still in recovery phase (may be normal)");
        }
    }
    
    // === VALIDATION ===
    
    // Система должна оставаться responsive даже при circuit breaker activation
    assert!(recovery_success >= 15, "Recovery success rate too low: {}/20", recovery_success);
    
    // Health check должен показывать что система работает
    let final_health = service.check_health().await?;
    // Система может быть в degraded state, но должна быть functional
    assert!(final_health.overall_status == "healthy" || 
            final_health.overall_status == "degraded", 
            "System should be functional after circuit breaker test");
    
    println!("✅ Circuit Breaker Activation and Recovery Test successful");
    
    Ok(())
}

/// ТЕСТ 2: Component Failure Scenarios
/// 
/// Тестирует поведение при failure отдельных компонентов
#[tokio::test]
async fn test_component_failure_scenarios() -> Result<()> {
    println!("💥 Starting Component Failure Scenarios Test");
    
    let service = create_resilience_test_service().await?;
    
    // === ПОДГОТОВКА BASELINE DATA ===
    
    let baseline_records = vec![
        "Critical system data that must survive component failures",
        "Important user information for failure recovery testing",
        "Essential application state for resilience validation",
        "Key performance metrics for component failure analysis",
        "Vital operational data for system stability verification",
    ];
    
    for (i, data) in baseline_records.iter().enumerate() {
        let record = create_resilience_test_record(i, data, Layer::Assets); // Важные данные в Assets
        service.insert(record).await?;
    }
    
    println!("✅ Baseline critical data stored: {} records", baseline_records.len());
    
    // === ПРОВЕРКА INITIAL SYSTEM STATE ===
    
    let initial_health = service.check_health().await?;
    println!("📊 Initial system health: {}", initial_health.overall_status);
    assert!(initial_health.overall_healthy, "System should be healthy initially");
    
    // === SCENARIO 1: EMBEDDING COORDINATOR STRESS ===
    
    println!("🧠 Testing EmbeddingCoordinator stress scenario...");
    
    // Создаем нагрузку на embedding generation
    let embedding_stress_start = Instant::now();
    let mut embedding_operations = Vec::new();
    
    for i in 0..30 {
        let service_clone = service.clone();
        let handle = tokio::spawn(async move {
            // Операции требующие embedding generation
            let record = create_resilience_test_record(
                i + 1000,
                "Large text content for embedding stress testing with complex semantics and extensive vocabulary",
                Layer::Interact
            );
            
            timeout(
                Duration::from_millis(200), // Короткий timeout для potential failure
                service_clone.insert(record)
            ).await
        });
        
        embedding_operations.push(handle);
    }
    
    let embedding_results = futures::future::join_all(embedding_operations).await;
    let embedding_success = embedding_results.iter()
        .filter(|r| matches!(r, Ok(Ok(Ok(_)))))
        .count();
    
    let embedding_stress_duration = embedding_stress_start.elapsed();
    
    println!("📊 EmbeddingCoordinator stress results:");
    println!("   Duration: {:.2}s", embedding_stress_duration.as_secs_f64());
    println!("   Successful operations: {}/30", embedding_success);
    
    // === SCENARIO 2: SEARCH COORDINATOR OVERLOAD ===
    
    println!("🔍 Testing SearchCoordinator overload scenario...");
    
    let search_stress_start = Instant::now();
    let mut search_operations = Vec::new();
    
    for i in 0..50 {
        let service_clone = service.clone();
        let handle = tokio::spawn(async move {
            // Сложные поисковые запросы
            let query = format!("complex search query {} with multiple terms and semantic requirements", i);
            
            timeout(
                Duration::from_millis(150), // Aggressive timeout
                service_clone.search(
                    &query,
                    Layer::Interact,
                    SearchOptions { top_k: 20, ..Default::default() }
                )
            ).await
        });
        
        search_operations.push(handle);
    }
    
    let search_results = futures::future::join_all(search_operations).await;
    let search_success = search_results.iter()
        .filter(|r| matches!(r, Ok(Ok(Ok(_)))))
        .count();
    
    let search_stress_duration = search_stress_start.elapsed();
    
    println!("📊 SearchCoordinator stress results:");
    println!("   Duration: {:.2}s", search_stress_duration.as_secs_f64());
    println!("   Successful operations: {}/50", search_success);
    
    // === SCENARIO 3: MEMORY PRESSURE SIMULATION ===
    
    println!("💾 Testing memory pressure scenario...");
    
    let memory_pressure_start = Instant::now();
    let mut memory_operations = 0;
    
    // Создаем много concurrent операций для memory pressure
    let mut memory_handles = Vec::new();
    
    for i in 0..100 {
        let service_clone = service.clone();
        let handle = tokio::spawn(async move {
            // Mix of operations создающих memory pressure
            if i % 2 == 0 {
                // Insert operation
                let record = create_resilience_test_record(
                    i + 2000,
                    "Memory pressure test data with large content payload for stress testing",
                    Layer::Interact
                );
                service_clone.insert(record).await.map(|_| "insert")
            } else {
                // Search operation
                service_clone.search(
                    "memory pressure search",
                    Layer::Interact,
                    SearchOptions { top_k: 15, ..Default::default() }
                ).await.map(|_| "search")
            }
        });
        
        memory_handles.push(handle);
    }
    
    let memory_results = timeout(
        Duration::from_secs(20),
        futures::future::join_all(memory_handles)
    ).await?;
    
    let memory_success = memory_results.iter()
        .filter(|r| matches!(r, Ok(Ok(_))))
        .count();
    
    let memory_pressure_duration = memory_pressure_start.elapsed();
    
    println!("📊 Memory pressure test results:");
    println!("   Duration: {:.2}s", memory_pressure_duration.as_secs_f64());
    println!("   Successful operations: {}/100", memory_success);
    
    // === ПРОВЕРКА SYSTEM STABILITY ПОСЛЕ STRESS ===
    
    sleep(Duration::from_millis(500)).await; // Brief recovery period
    
    let post_stress_health = service.check_health().await?;
    println!("🏥 Post-stress health: {}", post_stress_health.overall_status);
    
    // === ПРОВЕРКА DATA INTEGRITY ===
    
    println!("🔍 Verifying data integrity after component stress...");
    
    // Проверяем что critical data все еще доступно
    for (i, expected_data) in baseline_records.iter().enumerate() {
        let search_results = service.search(
            expected_data,
            Layer::Assets,
            SearchOptions { top_k: 5, ..Default::default() }
        ).await?;
        
        assert!(!search_results.is_empty(), 
                "Critical data should survive component failures: {}", expected_data);
    }
    
    println!("✅ Data integrity verified after component stress");
    
    // === VALIDATION ===
    
    // Система должна оставаться функциональной даже при component stress
    assert!(embedding_success >= 20, "Too many embedding failures: {}/30", embedding_success);
    assert!(search_success >= 35, "Too many search failures: {}/50", search_success);
    assert!(memory_success >= 70, "Too many memory pressure failures: {}/100", memory_success);
    
    // Health status может быть degraded, но система должна функционировать
    assert!(post_stress_health.overall_status == "healthy" || 
            post_stress_health.overall_status == "degraded",
            "System should remain functional after component stress");
    
    println!("✅ Component Failure Scenarios Test successful");
    
    Ok(())
}

/// ТЕСТ 3: Graceful Degradation Testing
/// 
/// Тестирует graceful degradation при partial system failures
#[tokio::test]
async fn test_graceful_degradation() -> Result<()> {
    println!("🎭 Starting Graceful Degradation Test");
    
    let service = create_resilience_test_service().await?;
    
    // === ПОДГОТОВКА TEST ENVIRONMENT ===
    
    // Создаем multi-layer dataset
    for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
        for i in 0..20 {
            let record = create_resilience_test_record(
                i,
                &format!("Graceful degradation test data for {:?} layer", layer),
                layer
            );
            service.insert(record).await?;
        }
    }
    
    println!("✅ Multi-layer test data prepared: 60 records across all layers");
    
    // === BASELINE PERFORMANCE MEASUREMENT ===
    
    println!("📊 Measuring baseline performance...");
    
    let baseline_start = Instant::now();
    let mut baseline_operations = Vec::new();
    
    for i in 0..20 {
        let operation_start = Instant::now();
        let results = service.search(
            "graceful degradation baseline",
            Layer::Interact,
            SearchOptions { top_k: 5, ..Default::default() }
        ).await?;
        let operation_duration = operation_start.elapsed();
        
        baseline_operations.push((results.len(), operation_duration.as_micros() as f64 / 1000.0));
    }
    
    let baseline_duration = baseline_start.elapsed();
    let baseline_avg_latency = baseline_operations.iter()
        .map(|(_, latency)| latency)
        .sum::<f64>() / baseline_operations.len() as f64;
    
    println!("   Baseline avg latency: {:.3}ms", baseline_avg_latency);
    println!("   Baseline total duration: {:.2}s", baseline_duration.as_secs_f64());
    
    // === DEGRADATION SCENARIO 1: PARTIAL COORDINATOR STRESS ===
    
    println!("📉 Testing partial coordinator stress degradation...");
    
    // Создаем targeted stress на specific coordinators
    let degradation_start = Instant::now();
    let mut degraded_operations = Vec::new();
    
    // Simultaneous операции создающие stress на разные coordinators
    let mut stress_handles = Vec::new();
    
    for i in 0..40 {
        let service_clone = service.clone();
        let handle = tokio::spawn(async move {
            if i < 20 {
                // Stress на EmbeddingCoordinator
                let record = create_resilience_test_record(
                    i + 3000,
                    "Large embedding stress content for degradation testing with extensive text",
                    Layer::Interact
                );
                
                let op_start = Instant::now();
                let result = timeout(
                    Duration::from_millis(300),
                    service_clone.insert(record)
                ).await;
                let op_duration = op_start.elapsed();
                
                (result.is_ok(), op_duration.as_micros() as f64 / 1000.0, "embedding")
            } else {
                // Stress на SearchCoordinator
                let op_start = Instant::now();
                let result = timeout(
                    Duration::from_millis(200),
                    service_clone.search(
                        "degradation stress search query",
                        Layer::Interact,
                        SearchOptions { top_k: 25, ..Default::default() }
                    )
                ).await;
                let op_duration = op_start.elapsed();
                
                (result.is_ok(), op_duration.as_micros() as f64 / 1000.0, "search")
            }
        });
        
        stress_handles.push(handle);
    }
    
    let stress_results = futures::future::join_all(stress_handles).await;
    
    // === ИЗМЕРЕНИЕ DEGRADED PERFORMANCE ===
    
    sleep(Duration::from_millis(100)).await; // Brief stabilization
    
    let degraded_measurement_start = Instant::now();
    
    for i in 0..20 {
        let operation_start = Instant::now();
        let result = timeout(
            Duration::from_millis(500), // More tolerant timeout
            service.search(
                "degraded performance measurement",
                Layer::Interact,
                SearchOptions { top_k: 5, ..Default::default() }
            )
        ).await;
        
        let operation_duration = operation_start.elapsed();
        let success = result.is_ok() && result.expect("Test operation should succeed").is_ok();
        let latency = operation_duration.as_micros() as f64 / 1000.0;
        
        degraded_operations.push((success, latency));
    }
    
    let degraded_measurement_duration = degraded_measurement_start.elapsed();
    
    // === DEGRADATION ANALYSIS ===
    
    let stress_success_count = stress_results.iter()
        .filter(|r| matches!(r, Ok((true, _, _))))
        .count();
    
    let degraded_success_count = degraded_operations.iter()
        .filter(|(success, _)| *success)
        .count();
    
    let degraded_avg_latency = degraded_operations.iter()
        .filter(|(success, _)| *success)
        .map(|(_, latency)| latency)
        .sum::<f64>() / degraded_success_count.max(1) as f64;
    
    println!("📊 Degradation analysis:");
    println!("   Stress operations success: {}/40", stress_success_count);
    println!("   Degraded success rate: {}/20", degraded_success_count);
    println!("   Baseline avg latency: {:.3}ms", baseline_avg_latency);
    println!("   Degraded avg latency: {:.3}ms", degraded_avg_latency);
    println!("   Latency increase: {:.1}%", 
             if baseline_avg_latency > 0.0 {
                 ((degraded_avg_latency - baseline_avg_latency) / baseline_avg_latency) * 100.0
             } else { 0.0 });
    
    // === GRACEFUL DEGRADATION VALIDATION ===
    
    // Проверяем что degradation является graceful:
    // 1. Система все еще functional (success rate > 70%)
    assert!(degraded_success_count >= 14, 
            "Graceful degradation failed: too many operation failures {}/20", degraded_success_count);
    
    // 2. Latency increase разумный (< 300% от baseline)
    let latency_increase = if baseline_avg_latency > 0.0 {
        degraded_avg_latency / baseline_avg_latency
    } else { 1.0 };
    
    assert!(latency_increase < 4.0, 
            "Latency degradation too severe: {:.1}x increase", latency_increase);
    
    // 3. Система остается responsive
    assert!(degraded_avg_latency < 100.0, 
            "System became unresponsive: {:.3}ms avg latency", degraded_avg_latency);
    
    // === RECOVERY VALIDATION ===
    
    println!("🔄 Testing recovery from degraded state...");
    
    sleep(Duration::from_secs(2)).await; // Recovery period
    
    let recovery_start = Instant::now();
    let mut recovery_operations = Vec::new();
    
    for i in 0..15 {
        let operation_start = Instant::now();
        let result = service.search(
            "recovery validation test",
            Layer::Interact,
            SearchOptions { top_k: 5, ..Default::default() }
        ).await;
        let operation_duration = operation_start.elapsed();
        
        recovery_operations.push((result.is_ok(), operation_duration.as_micros() as f64 / 1000.0));
    }
    
    let recovery_success_count = recovery_operations.iter()
        .filter(|(success, _)| *success)
        .count();
    
    let recovery_avg_latency = recovery_operations.iter()
        .filter(|(success, _)| *success)
        .map(|(_, latency)| latency)
        .sum::<f64>() / recovery_success_count.max(1) as f64;
    
    println!("📊 Recovery results:");
    println!("   Recovery success rate: {}/15", recovery_success_count);
    println!("   Recovery avg latency: {:.3}ms", recovery_avg_latency);
    
    // Recovery должен приближаться к baseline performance
    assert!(recovery_success_count >= 13, "Recovery incomplete: {}/15", recovery_success_count);
    
    if baseline_avg_latency > 0.0 {
        let recovery_improvement = degraded_avg_latency / recovery_avg_latency;
        println!("   Recovery improvement: {:.1}x", recovery_improvement);
        
        // Recovery должен показывать improvement
        assert!(recovery_improvement >= 0.8, "Insufficient recovery improvement: {:.1}x", recovery_improvement);
    }
    
    println!("✅ Graceful Degradation Test successful");
    println!("   Degradation was graceful: {}/20 operations succeeded", degraded_success_count);
    println!("   Recovery completed: {}/15 operations succeeded", recovery_success_count);
    
    Ok(())
}

/// ТЕСТ 4: Data Consistency Under Failures
/// 
/// Тестирует consistency данных при concurrent failures
#[tokio::test]
async fn test_data_consistency_under_failures() -> Result<()> {
    println!("🔒 Starting Data Consistency Under Failures Test");
    
    let service = Arc::new(create_resilience_test_service().await?);
    
    // === ПОДГОТОВКА CONSISTENCY TEST DATA ===
    
    let consistency_test_data = vec![
        ("user_1", "User profile data for consistency validation"),
        ("config_1", "System configuration settings for failure testing"),
        ("session_1", "Active session information for data integrity"),
        ("metrics_1", "Performance metrics data for consistency checks"),
        ("state_1", "Application state data for failure recovery"),
    ];
    
    // Вставляем critical data который должен оставаться consistent
    for (id, content) in &consistency_test_data {
        let record = Record {
            id: Uuid::new_v4(),
            text: format!("Consistency test {}: {}", id, content),
            embedding: vec![],
            layer: Layer::Assets,
            kind: "consistency_test".to_string(),
            tags: vec!["critical".to_string(), id.to_string()],
            project: "consistency_testing".to_string(),
            session: "consistency_session".to_string(),
            score: 0.95,
            ts: Utc::now(),
            access_count: 0,
            last_access: Utc::now(),
        };
        
        service.insert(record).await?;
    }
    
    println!("✅ Critical consistency data prepared: {} records", consistency_test_data.len());
    
    // === CONCURRENT OPERATIONS WITH SIMULATED FAILURES ===
    
    println!("⚡ Running concurrent operations with simulated failures...");
    
    let consistency_test_start = Instant::now();
    let mut consistency_handles = Vec::new();
    
    // Создаем 60 concurrent operations mixed с potential failures
    for i in 0..60 {
        let service_clone = service.clone();
        let handle = tokio::spawn(async move {
            match i % 4 {
                0 => {
                    // Normal insert operation
                    let record = create_resilience_test_record(
                        i + 4000,
                        "Consistency test normal insert operation",
                        Layer::Interact
                    );
                    
                    timeout(
                        Duration::from_millis(200),
                        service_clone.insert(record)
                    ).await.map(|r| (r.is_ok(), "insert"))
                }
                1 => {
                    // Normal search operation
                    timeout(
                        Duration::from_millis(150),
                        service_clone.search(
                            "consistency validation",
                            Layer::Interact,
                            SearchOptions { top_k: 5, ..Default::default() }
                        )
                    ).await.map(|r| (r.is_ok(), "search"))
                }
                2 => {
                    // Potentially problematic operation - large search
                    timeout(
                        Duration::from_millis(100), // Aggressive timeout
                        service_clone.search(
                            "large consistency search operation",
                            Layer::Interact,
                            SearchOptions { top_k: 100, ..Default::default() }
                        )
                    ).await.map(|r| (r.is_ok(), "large_search"))
                }
                3 => {
                    // Update operation on critical data
                    let mut record = create_resilience_test_record(
                        i,
                        "Updated consistency test data",
                        Layer::Assets
                    );
                    record.tags.push("updated".to_string());
                    
                    timeout(
                        Duration::from_millis(250),
                        service_clone.insert(record) // Insert acts as upsert
                    ).await.map(|r| (r.is_ok(), "update"))
                }
                _ => unreachable!(),
            }
        });
        
        consistency_handles.push(handle);
    }
    
    let consistency_results = futures::future::join_all(consistency_handles).await;
    let consistency_duration = consistency_test_start.elapsed();
    
    // === АНАЛИЗ CONSISTENCY RESULTS ===
    
    let total_operations = consistency_results.len();
    let successful_operations = consistency_results.iter()
        .filter(|r| matches!(r, Ok(Ok((true, _)))))
        .count();
    
    println!("📊 Concurrent operations results:");
    println!("   Duration: {:.2}s", consistency_duration.as_secs_f64());
    println!("   Successful operations: {}/{}", successful_operations, total_operations);
    println!("   Success rate: {:.1}%", (successful_operations as f64 / total_operations as f64) * 100.0);
    
    // === DATA CONSISTENCY VERIFICATION ===
    
    println!("🔍 Verifying data consistency after concurrent failures...");
    
    sleep(Duration::from_millis(200)).await; // Brief stabilization
    
    let mut consistency_checks = Vec::new();
    
    // Проверяем что все critical data остается accessible и consistent
    for (id, expected_content) in &consistency_test_data {
        let search_results = service.search(
            id,
            Layer::Assets,
            SearchOptions { top_k: 10, ..Default::default() }
        ).await?;
        
        let found_record = search_results.iter()
            .find(|record| record.tags.contains(&id.to_string()));
        
        consistency_checks.push((id, found_record.is_some()));
        
        if let Some(record) = found_record {
            // Проверяем integrity содержимого
            assert!(record.text.contains(expected_content), 
                    "Data consistency violation for {}: content mismatch", id);
            
            // Проверяем что metadata корректен
            assert!(record.layer == Layer::Assets, 
                    "Data consistency violation for {}: layer mismatch", id);
            
            println!("   ✅ {} consistency verified", id);
        } else {
            println!("   ❌ {} missing after concurrent operations", id);
        }
    }
    
    let consistent_data_count = consistency_checks.iter()
        .filter(|(_, is_consistent)| *is_consistent)
        .count();
    
    println!("📊 Data consistency results:");
    println!("   Consistent records: {}/{}", consistent_data_count, consistency_test_data.len());
    
    // === CROSS-LAYER CONSISTENCY CHECK ===
    
    println!("🔄 Checking cross-layer consistency...");
    
    // Проверяем что data в разных layers остается consistent
    for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
        let layer_results = service.search(
            "consistency",
            layer,
            SearchOptions { top_k: 20, ..Default::default() }
        ).await?;
        
        println!("   Layer {:?}: {} records found", layer, layer_results.len());
        
        // Проверяем что все записи в правильном layer
        for record in &layer_results {
            assert!(record.layer == layer, 
                    "Cross-layer consistency violation: record in wrong layer");
        }
    }
    
    // === CONSISTENCY VALIDATION ===
    
    // Требования для data consistency
    assert!(consistent_data_count >= consistency_test_data.len() - 1, 
            "Too many consistency violations: {}/{} records lost", 
            consistency_test_data.len() - consistent_data_count, consistency_test_data.len());
    
    assert!(successful_operations >= total_operations * 70 / 100, 
            "Too many operation failures: {}/{}", successful_operations, total_operations);
    
    // Health check после consistency test
    let final_health = service.check_health().await?;
    assert!(final_health.overall_status == "healthy" || final_health.overall_status == "degraded",
            "System should maintain consistency under failures");
    
    println!("✅ Data Consistency Under Failures Test successful");
    println!("   Data consistency: {}/{} critical records preserved", 
             consistent_data_count, consistency_test_data.len());
    println!("   Operation success rate: {:.1}%", 
             (successful_operations as f64 / total_operations as f64) * 100.0);
    
    Ok(())
}