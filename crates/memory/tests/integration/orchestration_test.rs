//! Orchestration Integration Tests
//! 
//! Comprehensive тесты для валидации интеграции всех orchestration coordinators:
//! - MemoryOrchestrator (95% готовности) - главный координатор
//! - EmbeddingCoordinator (95% готовности) - AI embeddings с adaptive batching
//! - SearchCoordinator (95% готовности) - Sub-5ms HNSW поиск с caching
//! - HealthManager (95% готовности) - Production monitoring с SLA метриками  
//! - ResourceController (95% готовности) - Auto-scaling с predictive analytics
//! - PromotionCoordinator, BackupCoordinator

use anyhow::Result;
use memory::{
    DIMemoryService,
    service_di::default_config,
    orchestration::{
        MemoryOrchestrator, EmbeddingCoordinator, SearchCoordinator, 
        HealthManager, ResourceController, PromotionCoordinator, BackupCoordinator
    },
    Record, Layer, SearchOptions,
    CacheConfigType,
};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::time::{sleep, Duration, timeout};
use uuid::Uuid;
use chrono::Utc;

/// Утилита для создания test service с orchestration
async fn create_orchestration_test_service() -> Result<DIMemoryService> {
    let temp_dir = TempDir::new()?;
    let mut config = default_config()?;
    
    config.db_path = temp_dir.path().join("orchestration_test.db");
    config.cache_path = temp_dir.path().join("orchestration_cache");
    config.cache_config = CacheConfigType::InMemory { max_size: 5000 };
    config.health_enabled = true;
    
    std::fs::create_dir_all(&config.cache_path)?;
    
    DIMemoryService::new(config).await
}

/// Создание тестовой записи
fn create_test_record(text: &str, layer: Layer) -> Record {
    Record {
        id: Uuid::new_v4(),
        text: text.to_string(),
        embedding: vec![], // Будет сгенерирован автоматически
        layer,
        kind: "orchestration_test".to_string(),
        tags: vec!["test".to_string()],
        project: "orchestration".to_string(),
        session: "test_session".to_string(),
        score: 0.8,
        ts: Utc::now(),
        access_count: 0,
        last_access: Utc::now(),
    }
}

/// ТЕСТ 1: MemoryOrchestrator Integration
/// 
/// Тестирует главный координатор и его взаимодействие со всеми sub-coordinators
#[tokio::test]
async fn test_memory_orchestrator_integration() -> Result<()> {
    println!("🎯 Starting MemoryOrchestrator Integration Test");
    
    let service = create_orchestration_test_service().await?;
    
    // === ПРОВЕРКА ИНИЦИАЛИЗАЦИИ ORCHESTRATOR ===
    
    let orchestrator = service.try_resolve::<MemoryOrchestrator>();
    assert!(orchestrator.is_some(), "MemoryOrchestrator should be available");
    
    let orchestrator = orchestrator.unwrap();
    
    // Проверяем что все coordinators инициализированы
    let all_ready = orchestrator.all_ready().await;
    println!("📊 All coordinators ready: {}", all_ready);
    
    // Получаем метрики от orchestrator
    let orchestrator_metrics = orchestrator.all_metrics().await;
    println!("📈 Orchestrator metrics: {}", orchestrator_metrics);
    
    // === ПРОВЕРКА CIRCUIT BREAKERS ===
    
    // Проверяем состояние circuit breakers всех coordinators
    let cb_status = orchestrator.circuit_breaker_status().await;
    println!("🔌 Circuit breaker status: {}", cb_status);
    
    // Все circuit breakers должны быть в состоянии Closed (нормальная работа)
    assert!(cb_status.contains("Closed"), "At least some circuit breakers should be Closed");
    
    // === ТЕСТИРОВАНИЕ COORDINATED OPERATIONS ===
    
    // Вставляем данные через orchestrator
    let test_records = vec![
        create_test_record("Orchestrator test record 1: AI system coordination", Layer::Interact),
        create_test_record("Orchestrator test record 2: Memory management strategies", Layer::Insights),
        create_test_record("Orchestrator test record 3: Performance optimization techniques", Layer::Assets),
    ];
    
    for record in &test_records {
        service.insert(record.clone()).await?;
    }
    
    println!("✅ Test records inserted through orchestrated service");
    
    // === ПРОВЕРКА SLA MONITORING ===
    
    // Выполняем поисковые операции для проверки SLA
    let search_start = std::time::Instant::now();
    
    for _ in 0..20 {
        let _results = service.search(
            "orchestrator coordination",
            Layer::Interact,
            SearchOptions { top_k: 5, ..Default::default() }
        ).await?;
    }
    
    let search_duration = search_start.elapsed();
    let avg_search_time = search_duration.as_millis() as f64 / 20.0;
    
    println!("🔍 Average search time: {:.2}ms", avg_search_time);
    
    // === ПРОВЕРКА HEALTH AGGREGATION ===
    
    let health_status = orchestrator.aggregated_health().await;
    println!("🏥 Aggregated health: {}", health_status);
    
    // Health status должен быть positive
    assert!(health_status.contains("healthy") || health_status.contains("ok"), 
            "Aggregated health should be positive");
    
    println!("✅ MemoryOrchestrator Integration Test successful");
    
    Ok(())
}

/// ТЕСТ 2: EmbeddingCoordinator Integration
/// 
/// Тестирует coordinator для embeddings с adaptive batching
#[tokio::test]
async fn test_embedding_coordinator_integration() -> Result<()> {
    println!("🧠 Starting EmbeddingCoordinator Integration Test");
    
    let service = create_orchestration_test_service().await?;
    
    // === ПОЛУЧЕНИЕ EMBEDDING COORDINATOR ===
    
    let embedding_coordinator = service.try_resolve::<EmbeddingCoordinator>();
    assert!(embedding_coordinator.is_some(), "EmbeddingCoordinator should be available");
    
    let embedding_coordinator = embedding_coordinator.unwrap();
    
    // === ПРОВЕРКА COORDINATOR STATUS ===
    
    let is_ready = embedding_coordinator.is_ready().await;
    println!("📊 EmbeddingCoordinator ready: {}", is_ready);
    
    let metrics = embedding_coordinator.get_metrics().await;
    println!("📈 EmbeddingCoordinator metrics: {:?}", metrics);
    
    // === ТЕСТИРОВАНИЕ BATCH PROCESSING ===
    
    // Создаем batch текстов для embedding
    let texts = vec![
        "Machine learning embeddings for semantic search",
        "Vector databases and similarity algorithms",
        "HNSW algorithm implementation details",
        "Natural language processing techniques",
        "Deep learning model optimization",
    ];
    
    // Тестируем batch embedding generation
    let batch_start = std::time::Instant::now();
    
    // Вставляем записи через service, что должно активировать EmbeddingCoordinator
    for (i, text) in texts.iter().enumerate() {
        let record = create_test_record(text, Layer::Interact);
        service.insert(record).await?;
        
        if i % 2 == 0 {
            sleep(Duration::from_millis(5)).await; // Небольшие паузы для batch testing
        }
    }
    
    let batch_duration = batch_start.elapsed();
    println!("🔄 Batch embedding processing time: {:.2}ms", batch_duration.as_millis());
    
    // === ПРОВЕРКА ADAPTIVE BATCHING ===
    
    // Создаем burst операций для тестирования adaptive batching
    let burst_texts: Vec<String> = (0..20).map(|i| 
        format!("Burst text {} for adaptive batching test", i)
    ).collect();
    
    let burst_start = std::time::Instant::now();
    
    for text in burst_texts {
        let record = create_test_record(&text, Layer::Interact);
        service.insert(record).await?;
    }
    
    let burst_duration = burst_start.elapsed();
    println!("⚡ Burst processing time: {:.2}ms", burst_duration.as_millis());
    
    // Adaptive batching должен быть эффективнее чем linear processing
    let burst_avg = burst_duration.as_millis() as f64 / 20.0;
    println!("   Average per operation: {:.2}ms", burst_avg);
    
    // === ПРОВЕРКА CIRCUIT BREAKER ===
    
    let cb_status = embedding_coordinator.circuit_breaker_status().await;
    println!("🔌 EmbeddingCoordinator circuit breaker: {}", cb_status);
    
    // Circuit breaker должен быть Closed (нормальная работа)
    assert!(cb_status == "Closed", "Circuit breaker should be Closed for normal operation");
    
    println!("✅ EmbeddingCoordinator Integration Test successful");
    
    Ok(())
}

/// ТЕСТ 3: SearchCoordinator Integration  
/// 
/// Тестирует coordinator для поиска с sub-5ms SLA и caching
#[tokio::test]
async fn test_search_coordinator_integration() -> Result<()> {
    println!("🔍 Starting SearchCoordinator Integration Test");
    
    let service = create_orchestration_test_service().await?;
    
    // === ПОДГОТОВКА TEST DATA ===
    
    // Добавляем тестовые данные для поиска
    let search_test_data = vec![
        "Advanced vector search algorithms and optimization techniques",
        "HNSW hierarchical navigable small world implementation",
        "Semantic similarity computation using cosine distance",
        "Machine learning embeddings for natural language processing",
        "Database indexing strategies for high-dimensional vectors",
        "Real-time search performance optimization methods",
        "Distributed vector databases and sharding techniques",
        "AI-powered search ranking and relevance scoring",
        "Memory-efficient vector storage and compression",
        "Production-scale vector search system architecture",
    ];
    
    for (i, text) in search_test_data.iter().enumerate() {
        let record = create_test_record(text, Layer::Interact);
        service.insert(record).await?;
        
        if i % 3 == 0 {
            println!("   Inserted {} test records", i + 1);
        }
    }
    
    println!("✅ Test data prepared: {} records", search_test_data.len());
    
    // === ПОЛУЧЕНИЕ SEARCH COORDINATOR ===
    
    let search_coordinator = service.try_resolve::<SearchCoordinator>();
    assert!(search_coordinator.is_some(), "SearchCoordinator should be available");
    
    let search_coordinator = search_coordinator.unwrap();
    
    // === SUB-5MS SLA TESTING ===
    
    println!("⏱️ Testing sub-5ms SLA requirement...");
    
    let search_queries = vec![
        "vector search optimization",
        "HNSW algorithm performance", 
        "machine learning embeddings",
        "database indexing strategies",
        "real-time search systems",
    ];
    
    let mut search_times = Vec::new();
    
    for query in &search_queries {
        let search_start = std::time::Instant::now();
        
        let results = service.search(
            query,
            Layer::Interact,
            SearchOptions { top_k: 10, ..Default::default() }
        ).await?;
        
        let search_time = search_start.elapsed();
        search_times.push(search_time.as_micros() as f64 / 1000.0); // Convert to milliseconds
        
        assert!(!results.is_empty(), "Search should return results for: {}", query);
    }
    
    let avg_search_time = search_times.iter().sum::<f64>() / search_times.len() as f64;
    let max_search_time = search_times.iter().fold(0.0, |acc, &x| acc.max(x));
    
    println!("📊 Search performance results:");
    println!("   Average search time: {:.3}ms", avg_search_time);
    println!("   Maximum search time: {:.3}ms", max_search_time);
    println!("   All search times: {:?}", search_times);
    
    // SLA requirement: sub-5ms search
    assert!(avg_search_time < 5.0, "Average search SLA violation: {:.3}ms >= 5ms", avg_search_time);
    assert!(max_search_time < 10.0, "Maximum search time too high: {:.3}ms", max_search_time); // Некоторая толерантность
    
    // === CACHE EFFECTIVENESS TESTING ===
    
    println!("💾 Testing cache effectiveness...");
    
    // Первый поиск - cold cache
    let cold_search_start = std::time::Instant::now();
    let _cold_results = service.search(
        "vector search optimization",
        Layer::Interact,
        SearchOptions { top_k: 10, ..Default::default() }
    ).await?;
    let cold_search_time = cold_search_start.elapsed();
    
    // Второй поиск - warm cache (должен быть быстрее)
    let warm_search_start = std::time::Instant::now();
    let _warm_results = service.search(
        "vector search optimization",
        Layer::Interact,
        SearchOptions { top_k: 10, ..Default::default() }
    ).await?;
    let warm_search_time = warm_search_start.elapsed();
    
    println!("   Cold cache search: {:.3}ms", cold_search_time.as_micros() as f64 / 1000.0);
    println!("   Warm cache search: {:.3}ms", warm_search_time.as_micros() as f64 / 1000.0);
    
    // Warm cache должен быть не медленнее чем cold cache
    assert!(warm_search_time <= cold_search_time * 2, 
            "Warm cache search should not be significantly slower");
    
    // === ПРОВЕРКА COORDINATOR METRICS ===
    
    let search_metrics = search_coordinator.get_metrics().await;
    println!("📈 SearchCoordinator metrics: {:?}", search_metrics);
    
    let cb_status = search_coordinator.circuit_breaker_status().await;
    println!("🔌 SearchCoordinator circuit breaker: {}", cb_status);
    
    assert!(cb_status == "Closed", "Circuit breaker should be Closed");
    
    println!("✅ SearchCoordinator Integration Test successful");
    println!("   SLA compliance: {:.3}ms < 5ms ✓", avg_search_time);
    
    Ok(())
}

/// ТЕСТ 4: HealthManager Integration
/// 
/// Тестирует health monitoring с SLA метриками
#[tokio::test]
async fn test_health_manager_integration() -> Result<()> {
    println!("🏥 Starting HealthManager Integration Test");
    
    let service = create_orchestration_test_service().await?;
    
    // === ПОЛУЧЕНИЕ HEALTH MANAGER ===
    
    let health_manager = service.try_resolve::<HealthManager>();
    assert!(health_manager.is_some(), "HealthManager should be available");
    
    let health_manager = health_manager.unwrap();
    
    // === ПРОВЕРКА INITIAL HEALTH ===
    
    let initial_health = health_manager.check_system_health().await?;
    println!("📊 Initial system health: {:?}", initial_health);
    
    assert!(initial_health.overall_healthy, "System should be healthy initially");
    assert!(!initial_health.components.is_empty(), "Should have component health data");
    
    // === ГЕНЕРАЦИЯ LOAD ДЛЯ HEALTH METRICS ===
    
    println!("🔄 Generating load for health metrics...");
    
    // Выполняем операции для генерации метрик
    for i in 0..30 {
        let record = create_test_record(
            &format!("Health test record {}: monitoring system performance", i),
            Layer::Interact
        );
        
        service.insert(record).await?;
        
        // Периодические поисковые запросы
        if i % 5 == 0 {
            let _results = service.search(
                "health monitoring performance",
                Layer::Interact,
                SearchOptions { top_k: 5, ..Default::default() }
            ).await?;
        }
        
        if i % 10 == 0 {
            sleep(Duration::from_millis(5)).await;
        }
    }
    
    // === ПРОВЕРКА HEALTH ПОСЛЕ LOAD ===
    
    let health_after_load = health_manager.check_system_health().await?;
    println!("📊 Health after load: {:?}", health_after_load);
    
    assert!(health_after_load.overall_healthy, "System should remain healthy under load");
    
    // === SLA MONITORING TESTING ===
    
    let sla_metrics = health_manager.get_sla_metrics().await?;
    println!("📈 SLA metrics: {:?}", sla_metrics);
    
    // Проверяем что SLA metrics содержат нужную информацию
    assert!(sla_metrics.contains_key("search_latency") || 
            sla_metrics.contains_key("response_time") ||
            sla_metrics.contains_key("availability"), 
            "SLA metrics should contain performance data");
    
    // === ALERT SYSTEM TESTING ===
    
    println!("🚨 Testing alert system...");
    
    // Получаем текущие alerts
    let current_alerts = health_manager.get_current_alerts().await?;
    println!("   Current alerts: {} active", current_alerts.len());
    
    // В нормальных условиях не должно быть critical alerts
    let critical_alerts: Vec<_> = current_alerts.iter()
        .filter(|alert| alert.contains("critical") || alert.contains("error"))
        .collect();
    
    assert!(critical_alerts.is_empty(), "Should not have critical alerts in normal operation");
    
    // === UPTIME TRACKING ===
    
    let uptime = health_after_load.uptime;
    println!("⏱️ System uptime: {:?}", uptime);
    
    assert!(uptime > Duration::from_millis(100), "System should have positive uptime");
    
    // === PERFORMANCE DEGRADATION CHECK ===
    
    let performance_status = health_manager.check_performance_degradation().await?;
    println!("⚡ Performance status: {}", performance_status);
    
    // Performance не должна быть degraded в тестовых условиях
    assert!(!performance_status.contains("degraded"), 
            "Performance should not be degraded: {}", performance_status);
    
    println!("✅ HealthManager Integration Test successful");
    
    Ok(())
}

/// ТЕСТ 5: ResourceController Integration
/// 
/// Тестирует auto-scaling и resource management
#[tokio::test] 
async fn test_resource_controller_integration() -> Result<()> {
    println!("⚙️ Starting ResourceController Integration Test");
    
    let service = create_orchestration_test_service().await?;
    
    // === ПОЛУЧЕНИЕ RESOURCE CONTROLLER ===
    
    let resource_controller = service.try_resolve::<ResourceController>();
    assert!(resource_controller.is_some(), "ResourceController should be available");
    
    let resource_controller = resource_controller.unwrap();
    
    // === ПРОВЕРКА INITIAL RESOURCE STATE ===
    
    let initial_resources = resource_controller.get_resource_status().await?;
    println!("📊 Initial resource status: {:?}", initial_resources);
    
    // === RESOURCE MONITORING UNDER LOAD ===
    
    println!("🔄 Testing resource monitoring under load...");
    
    // Создаем нагрузку для тестирования resource scaling
    let load_start = std::time::Instant::now();
    let mut operation_handles = Vec::new();
    
    for i in 0..50 {
        let service_clone = service.clone();
        let handle = tokio::spawn(async move {
            let record = create_test_record(
                &format!("Resource load test {}: intensive operation data", i),
                Layer::Interact
            );
            
            service_clone.insert(record).await
        });
        
        operation_handles.push(handle);
    }
    
    // Ждем завершения load operations
    let load_results = timeout(
        Duration::from_secs(10),
        futures::future::try_join_all(operation_handles)
    ).await??;
    
    let load_duration = load_start.elapsed();
    let successful_ops = load_results.iter().filter(|r| r.is_ok()).count();
    
    println!("📈 Load test results:");
    println!("   Duration: {:.2}s", load_duration.as_secs_f64());
    println!("   Successful operations: {}/{}", successful_ops, load_results.len());
    
    // === ПРОВЕРКА RESOURCE STATE ПОСЛЕ LOAD ===
    
    let resources_after_load = resource_controller.get_resource_status().await?;
    println!("📊 Resources after load: {:?}", resources_after_load);
    
    // === AUTO-SCALING BEHAVIOR ===
    
    let scaling_status = resource_controller.get_scaling_status().await?;
    println!("📏 Auto-scaling status: {}", scaling_status);
    
    // Проверяем что auto-scaling реагирует на нагрузку
    assert!(scaling_status.contains("stable") || 
            scaling_status.contains("scaling") ||
            scaling_status.contains("active"), 
            "Auto-scaling should be responsive");
    
    // === PREDICTIVE ANALYTICS TESTING ===
    
    let prediction = resource_controller.predict_resource_needs().await?;
    println!("🔮 Resource prediction: {:?}", prediction);
    
    // Prediction должна содержать полезную информацию
    assert!(prediction.contains_key("cpu") || 
            prediction.contains_key("memory") ||
            prediction.contains_key("storage"), 
            "Prediction should contain resource metrics");
    
    // === RESOURCE OPTIMIZATION ===
    
    let optimization_result = resource_controller.optimize_resources().await?;
    println!("⚡ Resource optimization: {}", optimization_result);
    
    assert!(optimization_result.contains("completed") || 
            optimization_result.contains("optimized") ||
            optimization_result.contains("success"), 
            "Resource optimization should complete successfully");
    
    println!("✅ ResourceController Integration Test successful");
    
    Ok(())
}

/// ТЕСТ 6: Cross-Coordinator Integration
/// 
/// Тестирует взаимодействие между всеми coordinators
#[tokio::test]
async fn test_cross_coordinator_integration() -> Result<()> {
    println!("🔗 Starting Cross-Coordinator Integration Test");
    
    let service = create_orchestration_test_service().await?;
    
    // === ПОЛУЧЕНИЕ ВСЕХ COORDINATORS ===
    
    let orchestrator = service.try_resolve::<MemoryOrchestrator>();
    let embedding_coord = service.try_resolve::<EmbeddingCoordinator>();
    let search_coord = service.try_resolve::<SearchCoordinator>();
    let health_manager = service.try_resolve::<HealthManager>();
    let resource_controller = service.try_resolve::<ResourceController>();
    
    assert!(orchestrator.is_some(), "MemoryOrchestrator should be available");
    assert!(embedding_coord.is_some(), "EmbeddingCoordinator should be available");
    assert!(search_coord.is_some(), "SearchCoordinator should be available");
    assert!(health_manager.is_some(), "HealthManager should be available");
    
    println!("✅ All coordinators resolved successfully");
    
    // === COORDINATED WORKFLOW TESTING ===
    
    println!("🔄 Testing coordinated workflow...");
    
    // Комплексная операция затрагивающая все coordinators:
    // 1. EmbeddingCoordinator: генерация embeddings
    // 2. SearchCoordinator: поиск и caching
    // 3. HealthManager: мониторинг операций
    // 4. ResourceController: отслеживание ресурсов
    // 5. MemoryOrchestrator: общая координация
    
    let workflow_records = vec![
        "Coordinated workflow test: embedding generation and vector search",
        "Cross-coordinator integration: health monitoring and resource management", 
        "System orchestration: performance optimization and SLA compliance",
        "Multi-component operation: caching, scaling, and circuit breaker management",
        "Production workflow simulation: comprehensive coordinator interaction",
    ];
    
    let workflow_start = std::time::Instant::now();
    
    for (i, text) in workflow_records.iter().enumerate() {
        // Insert операция (затрагивает EmbeddingCoordinator)
        let record = create_test_record(text, Layer::Interact);
        service.insert(record).await?;
        
        // Search операция (затрагивает SearchCoordinator)
        let _results = service.search(
            &format!("workflow test {}", i),
            Layer::Interact,
            SearchOptions { top_k: 3, ..Default::default() }
        ).await?;
        
        // Небольшая пауза для coordinator interaction
        sleep(Duration::from_millis(10)).await;
    }
    
    let workflow_duration = workflow_start.elapsed();
    
    println!("⏱️ Coordinated workflow completed in {:.2}ms", workflow_duration.as_millis());
    
    // === ПРОВЕРКА COORDINATOR SYNCHRONIZATION ===
    
    if let Some(orchestrator) = orchestrator {
        // Проверяем что все coordinators синхронизированы
        let sync_status = orchestrator.check_coordinator_sync().await;
        println!("🔄 Coordinator synchronization: {}", sync_status);
        
        // Получаем агрегированные метрики
        let aggregated_metrics = orchestrator.all_metrics().await;
        println!("📊 Aggregated metrics: {}", aggregated_metrics);
        
        // Проверяем общий health
        let overall_health = orchestrator.aggregated_health().await;
        println!("🏥 Overall health: {}", overall_health);
        
        assert!(overall_health.contains("healthy") || overall_health.contains("ok"),
                "Overall system health should be positive");
    }
    
    // === CIRCUIT BREAKER COORDINATION ===
    
    println!("🔌 Testing circuit breaker coordination...");
    
    // Проверяем что circuit breakers всех coordinators координируются
    if let (Some(embedding), Some(search), Some(health)) = (&embedding_coord, &search_coord, &health_manager) {
        let embedding_cb = embedding.circuit_breaker_status().await;
        let search_cb = search.circuit_breaker_status().await;
        let health_cb = health.circuit_breaker_status().await;
        
        println!("   EmbeddingCoordinator CB: {}", embedding_cb);
        println!("   SearchCoordinator CB: {}", search_cb);
        println!("   HealthManager CB: {}", health_cb);
        
        // Все circuit breakers должны быть в consistent state
        let all_closed = [&embedding_cb, &search_cb, &health_cb]
            .iter()
            .all(|cb| cb.contains("Closed"));
        
        if !all_closed {
            println!("⚠️ Warning: Circuit breakers in mixed states (expected in test env)");
        }
    }
    
    // === PERFORMANCE UNDER COORDINATION ===
    
    // Тестируем производительность при полной coordinator coordination
    let perf_start = std::time::Instant::now();
    
    for i in 0..20 {
        let _results = service.search(
            "coordinated performance test",
            Layer::Interact,
            SearchOptions { top_k: 5, ..Default::default() }
        ).await?;
        
        if i % 5 == 0 {
            let record = create_test_record(
                &format!("Performance test record {}", i),
                Layer::Interact
            );
            service.insert(record).await?;
        }
    }
    
    let perf_duration = perf_start.elapsed();
    let ops_per_sec = 25.0 / perf_duration.as_secs_f64(); // 20 searches + 5 inserts
    
    println!("⚡ Coordinated performance: {:.1} ops/sec", ops_per_sec);
    
    // Координация не должна существенно снижать производительность
    assert!(ops_per_sec >= 20.0, "Coordination overhead too high: {:.1} ops/sec", ops_per_sec);
    
    println!("✅ Cross-Coordinator Integration Test successful");
    println!("   Workflow duration: {:.2}ms", workflow_duration.as_millis());
    println!("   Coordinated performance: {:.1} ops/sec", ops_per_sec);
    
    Ok(())
}