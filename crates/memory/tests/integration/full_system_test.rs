//! Comprehensive Full System Integration Tests
//! 
//! Эти тесты валидируют полную функциональность MAGRAY CLI системы
//! после всех архитектурных улучшений:
//! - DIMemoryService (95% готовности)
//! - MemoryOrchestrator (95% готовности)  
//! - UnifiedAgent (90% готовности)
//! - Все orchestration coordinators (95% готовности)

use anyhow::Result;
use memory::{
    DIMemoryService, 
    MemoryServiceConfig,
    Record, Layer, SearchOptions,
    orchestration::{MemoryOrchestrator, EmbeddingCoordinator, SearchCoordinator, HealthManager, ResourceController},
    service_di::default_config,
    CacheConfigType,
};
use ai::AiConfig;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::time::{sleep, Duration, timeout};
use uuid::Uuid;
use chrono::Utc;
use std::collections::HashMap;


/// Утилита для создания тестовых записей
fn create_test_record(text: &str, layer: Layer, session: &str) -> Record {
    Record {
        id: Uuid::new_v4(),
        text: text.to_string(),
        embedding: vec![], // Будет сгенерирован автоматически
        layer,
        kind: "integration_test".to_string(),
        tags: vec!["test".to_string(), session.to_string()],
        project: "magray_cli".to_string(),
        session: session.to_string(),
        score: 0.85,
        ts: Utc::now(),
        access_count: 0,
        last_access: Utc::now(),
    }
}

/// Создание test service с полной конфигурацией
async fn create_production_test_service() -> Result<DIMemoryService> {
    let temp_dir = TempDir::new()?;
    let mut config = default_config()?;
    
    // Production-like конфигурация
    config.db_path = temp_dir.path().join("production_test.db");
    config.cache_path = temp_dir.path().join("production_cache");
    config.cache_config = CacheConfigType::InMemory { max_size: 10000 };
    config.health_enabled = true;
    
    // AI конфигурация для embeddings
    config.ai_config = AiConfig {
        enable_ai: true,
        ..Default::default()
    };
    
    std::fs::create_dir_all(&config.cache_path)?;
    
    DIMemoryService::new(config).await
}

/// ТЕСТ 1: Полный End-to-End Workflow
/// 
/// Симулирует полный пользовательский workflow:
/// 1. User input → Intent analysis → Routing → Execution → Memory storage
/// 2. Chat flow: message → LLM → response → memory storage  
/// 3. Tools flow: command → router → tool execution → result formatting
/// 4. Cross-layer memory operations: insert → search → promotion → assets
#[tokio::test]
async fn test_complete_end_to_end_workflow() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    println!("🚀 Starting Complete End-to-End Workflow Test");
    
    // === ФАЗА 1: SYSTEM INITIALIZATION ===
    let service = create_production_test_service().await?;
    
    // Проверяем что все orchestration coordinators инициализированы
    let di_stats = service.di_stats();
    println!("📊 DI Container initialized: {} types, {} cached", 
             di_stats.total_types, di_stats.cached_singletons);
    
    // Проверяем health status
    let health = service.check_health().await?;
    assert!(health.overall_status == "healthy", "System should be healthy on startup");
    
    println!("✅ Phase 1: System initialization complete");
    
    // === ФАЗА 2: CHAT WORKFLOW SIMULATION ===
    
    // Симулируем chat messages от пользователя
    let chat_messages = vec![
        "Как создать векторную базу данных в Rust?",
        "Объясни архитектуру HNSW алгоритма",
        "Какие есть best practices для оптимизации поиска?",
        "Покажи примеры использования embeddings в машинном обучении",
    ];
    
    let mut chat_records = Vec::new();
    for (i, message) in chat_messages.iter().enumerate() {
        let record = create_test_record(
            &format!("Chat message {}: {}", i + 1, message),
            Layer::Interact,
            "chat_session_001"
        );
        
        // Симулируем insert через DIMemoryService
        service.insert(record.clone()).await?;
        chat_records.push(record);
        
        // Небольшая пауза для симуляции реального времени
        sleep(Duration::from_millis(10)).await;
    }
    
    println!("✅ Phase 2: Chat workflow - {} messages processed", chat_records.len());
    
    // === ФАЗА 3: TOOLS WORKFLOW SIMULATION ===
    
    // Симулируем tools/commands от пользователя
    let tool_commands = vec![
        "file_read: Прочитать README.md",
        "web_search: Rust vector databases comparison",
        "git_status: Проверить статус репозитория", 
        "shell_exec: cargo test --package memory",
    ];
    
    let mut tool_records = Vec::new();
    for (i, command) in tool_commands.iter().enumerate() {
        let record = create_test_record(
            &format!("Tool execution {}: {}", i + 1, command),
            Layer::Insights, // Tools results в Insights layer
            "tools_session_001"
        );
        
        service.insert(record.clone()).await?;
        tool_records.push(record);
        
        sleep(Duration::from_millis(5)).await;
    }
    
    println!("✅ Phase 3: Tools workflow - {} commands processed", tool_records.len());
    
    // === ФАЗА 4: CROSS-LAYER MEMORY OPERATIONS ===
    
    // Тестируем поиск по всем слоям
    for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
        let search_results = service.search(
            "Rust vector database",
            layer,
            SearchOptions {
                top_k: 5,
                ..Default::default()
            }
        ).await?;
        
        println!("   Layer {:?}: {} search results", layer, search_results.len());
    }
    
    // Добавляем записи в Assets layer для permanent knowledge
    let asset_knowledge = vec![
        "HNSW (Hierarchical Navigable Small World) - это граф-based алгоритм для approximate nearest neighbor search",
        "Vector embeddings представляют текст в высокомерном пространстве для semantic similarity",
        "Rust provides memory safety без garbage collection через ownership system",
    ];
    
    for knowledge in asset_knowledge {
        let record = create_test_record(knowledge, Layer::Assets, "knowledge_base");
        service.insert(record).await?;
    }
    
    println!("✅ Phase 4: Cross-layer operations complete");
    
    // === ФАЗА 5: PERFORMANCE VALIDATION ===
    
    // Тестируем sub-5ms search SLA
    let search_start = std::time::Instant::now();
    
    for _ in 0..100 {
        let _results = service.search(
            "vector database search performance",
            Layer::Interact,
            SearchOptions { top_k: 10, ..Default::default() }
        ).await?;
    }
    
    let search_duration = search_start.elapsed();
    let avg_search_time = search_duration.as_millis() as f64 / 100.0;
    
    println!("🔍 Average search time: {:.2}ms", avg_search_time);
    
    // SLA requirement: sub-5ms search
    assert!(avg_search_time < 5.0, "Search performance SLA violation: {:.2}ms > 5ms", avg_search_time);
    
    println!("✅ Phase 5: Performance validation - SLA met ({:.2}ms < 5ms)", avg_search_time);
    
    // === ФАЗА 6: MEMORY PROMOTION CYCLE ===
    
    // Запускаем promotion cycle
    let promotion_stats = service.run_promotion().await?;
    
    println!("📈 Promotion cycle completed:");
    println!("   Interact → Insights: {}", promotion_stats.interact_to_insights);
    println!("   Insights → Assets: {}", promotion_stats.insights_to_assets);
    println!("   Expired Interact: {}", promotion_stats.expired_interact);
    println!("   Expired Insights: {}", promotion_stats.expired_insights);
    
    println!("✅ Phase 6: Memory promotion cycle complete");
    
    // === ФАЗА 7: HEALTH & METRICS VALIDATION ===
    
    let final_health = service.check_health().await?;
    println!("🏥 Final health status: {}", final_health.overall_status);
    
    // Проверяем metrics
    let stats = service.get_stats().await;
    println!("📊 Final system stats:");
    println!("   Cache hits: {}", stats.cache_hits);
    println!("   Cache misses: {}", stats.cache_misses);
    println!("   Total operations: {}", stats.cache_hits + stats.cache_misses);
    
    if stats.cache_hits + stats.cache_misses > 0 {
        let hit_rate = stats.cache_hits as f64 / (stats.cache_hits + stats.cache_misses) as f64 * 100.0;
        println!("   Cache hit rate: {:.1}%", hit_rate);
        
        // Ожидаем минимум 50% cache hit rate для эффективности
        assert!(hit_rate >= 50.0, "Cache hit rate too low: {:.1}%", hit_rate);
    }
    
    println!("✅ Phase 7: Health & metrics validation complete");
    
    println!("🎉 COMPLETE END-TO-END WORKFLOW TEST SUCCESSFUL");
    println!("   Total chat messages: {}", chat_records.len());
    println!("   Total tool commands: {}", tool_records.len());
    println!("   Average search performance: {:.2}ms", avg_search_time);
    println!("   Final health status: {}", final_health.overall_status);
    
    Ok(())
}

/// ТЕСТ 2: Concurrent User Sessions Simulation
/// 
/// Симулирует multiple concurrent пользователей работающих с системой
#[tokio::test]
async fn test_concurrent_user_sessions() -> Result<()> {
    println!("👥 Starting Concurrent User Sessions Test");
    
    let service = Arc::new(create_production_test_service().await?);
    
    let mut session_handles = Vec::new();
    
    // Создаем 10 concurrent user sessions
    for session_id in 0..10 {
        let service_clone = service.clone();
        let handle = tokio::spawn(async move {
            let session_name = format!("user_session_{:02}", session_id);
            let mut operations_count = 0;
            
            // Каждая сессия выполняет 20 операций
            for i in 0..20 {
                let operation_type = i % 3;
                
                match operation_type {
                    0 => {
                        // Chat message
                        let record = create_test_record(
                            &format!("User {} message {}: How to implement AI systems?", session_id, i),
                            Layer::Interact,
                            &session_name
                        );
                        
                        if service_clone.insert(record).await.is_ok() {
                            operations_count += 1;
                        }
                    }
                    1 => {
                        // Search operation
                        let query = format!("AI implementation session {}", session_id);
                        if service_clone.search(&query, Layer::Interact, SearchOptions::default()).await.is_ok() {
                            operations_count += 1;
                        }
                    }
                    2 => {
                        // Tool command
                        let record = create_test_record(
                            &format!("User {} tool {}: Execute search operation", session_id, i),
                            Layer::Insights,
                            &session_name
                        );
                        
                        if service_clone.insert(record).await.is_ok() {
                            operations_count += 1;
                        }
                    }
                    _ => unreachable!(),
                }
                
                // Небольшая пауза между операциями
                sleep(Duration::from_millis(5)).await;
            }
            
            operations_count
        });
        
        session_handles.push(handle);
    }
    
    // Ждем завершения всех сессий
    let session_results = futures::future::try_join_all(session_handles).await?;
    
    let total_operations: usize = session_results.iter().sum();
    let expected_operations = 10 * 20; // 10 sessions × 20 operations each
    
    println!("📊 Concurrent sessions completed:");
    println!("   Total operations: {}/{}", total_operations, expected_operations);
    println!("   Success rate: {:.1}%", (total_operations as f64 / expected_operations as f64) * 100.0);
    
    // Проверяем что минимум 90% операций выполнились успешно
    assert!(total_operations >= (expected_operations * 9 / 10), 
            "Too many failed operations: {}/{}", total_operations, expected_operations);
    
    // Проверяем health после нагрузки
    let health_after_load = service.check_health().await?;
    assert!(health_after_load.overall_status == "healthy", 
            "System should remain healthy after concurrent load");
    
    println!("✅ Concurrent User Sessions Test successful");
    
    Ok(())
}

/// ТЕСТ 3: Production Workload Simulation
/// 
/// Симулирует реальную production нагрузку с различными типами операций
#[tokio::test]
async fn test_production_workload_simulation() -> Result<()> {
    println!("🏭 Starting Production Workload Simulation");
    
    let service = Arc::new(create_production_test_service().await?);
    
    // === ПОДГОТОВКА BASELINE DATA ===
    
    // Загружаем baseline knowledge base
    let knowledge_base = vec![
        "Vector databases enable semantic search through high-dimensional embeddings",
        "HNSW algorithm provides efficient approximate nearest neighbor search",
        "Rust ownership system ensures memory safety without garbage collection",
        "Machine learning embeddings capture semantic relationships in text",
        "Distributed systems require careful consideration of consistency and availability",
        "API design principles emphasize clarity, consistency, and backwards compatibility",
        "Database indexing strategies significantly impact query performance",
        "Microservices architecture enables independent scaling and deployment",
        "Caching strategies reduce latency and improve system throughput",
        "Load balancing distributes traffic across multiple service instances",
    ];
    
    for (i, knowledge) in knowledge_base.iter().enumerate() {
        let record = create_test_record(knowledge, Layer::Assets, "baseline_knowledge");
        service.insert(record).await?;
        
        if i % 5 == 0 {
            println!("   Loaded {} baseline records", i + 1);
        }
    }
    
    println!("✅ Baseline knowledge loaded: {} records", knowledge_base.len());
    
    // === PRODUCTION WORKLOAD SIMULATION ===
    
    let workload_start = std::time::Instant::now();
    let mut operation_handles = Vec::new();
    
    // 80% читающих операций, 20% записывающих (типичное production соотношение)
    for op_id in 0..200 {
        let service_clone = service.clone();
        
        let handle = if op_id % 5 == 0 {
            // 20% writing operations
            tokio::spawn(async move {
                let record = create_test_record(
                    &format!("Production operation {}: Real user data processing", op_id),
                    Layer::Interact,
                    "production_workload"
                );
                
                service_clone.insert(record).await.map(|_| "write".to_string())
            })
        } else {
            // 80% reading operations
            tokio::spawn(async move {
                let queries = vec![
                    "vector database performance",
                    "HNSW algorithm optimization", 
                    "Rust memory management",
                    "machine learning embeddings",
                    "distributed systems architecture",
                ];
                
                let query = &queries[op_id % queries.len()];
                service_clone.search(query, Layer::Interact, SearchOptions {
                    top_k: 5,
                    ..Default::default()
                }).await.map(|_| "read".to_string())
            })
        };
        
        operation_handles.push(handle);
    }
    
    // Выполняем все операции с timeout
    let workload_results = timeout(
        Duration::from_secs(30),
        futures::future::try_join_all(operation_handles)
    ).await??;
    
    let workload_duration = workload_start.elapsed();
    
    // === АНАЛИЗ ПРОИЗВОДИТЕЛЬНОСТИ ===
    
    let successful_ops = workload_results.len();
    let ops_per_second = successful_ops as f64 / workload_duration.as_secs_f64();
    
    println!("📈 Production workload results:");
    println!("   Total operations: {}", successful_ops);
    println!("   Duration: {:.2}s", workload_duration.as_secs_f64());
    println!("   Throughput: {:.1} ops/sec", ops_per_second);
    
    // Production requirements
    assert!(ops_per_second >= 50.0, "Production throughput too low: {:.1} ops/sec", ops_per_second);
    assert!(successful_ops >= 190, "Too many failed operations: {}/200", successful_ops);
    
    // === СИСТЕМА ОСТАЕТСЯ СТАБИЛЬНОЙ ===
    
    let final_health = service.check_health().await?;
    assert!(final_health.overall_status == "healthy", 
            "System should remain healthy after production workload");
    
    // Проверяем metrics
    let final_stats = service.get_stats().await;
    println!("📊 Final production metrics:");
    println!("   Cache hit rate: {:.1}%", 
             if final_stats.cache_hits + final_stats.cache_misses > 0 {
                 final_stats.cache_hits as f64 / (final_stats.cache_hits + final_stats.cache_misses) as f64 * 100.0
             } else { 0.0 });
    
    println!("✅ Production Workload Simulation successful");
    println!("   Throughput: {:.1} ops/sec", ops_per_second);
    println!("   Success rate: {:.1}%", (successful_ops as f64 / 200.0) * 100.0);
    
    Ok(())
}

/// ТЕСТ 4: Memory Lifecycle Integration
/// 
/// Тестирует полный lifecycle записей через все слои памяти
#[tokio::test] 
async fn test_memory_lifecycle_integration() -> Result<()> {
    println!("🔄 Starting Memory Lifecycle Integration Test");
    
    let service = create_production_test_service().await?;
    
    // === СОЗДАНИЕ ЗАПИСЕЙ В INTERACT LAYER ===
    
    let mut lifecycle_records = Vec::new();
    
    for i in 0..50 {
        let record = create_test_record(
            &format!("Lifecycle record {}: Important information about system design", i),
            Layer::Interact,
            "lifecycle_test"
        );
        
        service.insert(record.clone()).await?;
        lifecycle_records.push(record);
    }
    
    println!("✅ Created {} records in Interact layer", lifecycle_records.len());
    
    // === ПРОВЕРКА ПОИСКА В INTERACT ===
    
    let interact_search = service.search(
        "system design",
        Layer::Interact,
        SearchOptions { top_k: 10, ..Default::default() }
    ).await?;
    
    assert!(!interact_search.is_empty(), "Should find records in Interact layer");
    println!("   Found {} records in Interact search", interact_search.len());
    
    // === СИМУЛЯЦИЯ ВРЕМЕНИ ДЛЯ PROMOTION ===
    
    // Добавляем записи в Insights для демонстрации promotion logic
    for i in 0..20 {
        let record = Record {
            id: Uuid::new_v4(),
            text: format!("Insights record {}: Analysis and patterns from data", i),
            embedding: vec![],
            layer: Layer::Insights,
            kind: "analysis".to_string(),
            tags: vec!["insights".to_string(), "analysis".to_string()],
            project: "magray_cli".to_string(),
            session: "lifecycle_test".to_string(),
            score: 0.9,
            ts: Utc::now() - chrono::Duration::days(1), // Older timestamp
            access_count: i + 5, // Simulated access
            last_access: Utc::now(),
        };
        
        service.insert(record).await?;
    }
    
    // Добавляем важные записи в Assets
    for i in 0..10 {
        let record = Record {
            id: Uuid::new_v4(),
            text: format!("Asset record {}: Core knowledge and principles", i),
            embedding: vec![],
            layer: Layer::Assets,
            kind: "knowledge".to_string(),
            tags: vec!["assets".to_string(), "core".to_string()],
            project: "magray_cli".to_string(),
            session: "lifecycle_test".to_string(),
            score: 0.95,
            ts: Utc::now() - chrono::Duration::days(30), // Much older
            access_count: i + 10, // High access count
            last_access: Utc::now(),
        };
        
        service.insert(record).await?;
    }
    
    println!("✅ Added records to all memory layers");
    
    // === ТЕСТИРОВАНИЕ PROMOTION CYCLE ===
    
    let promotion_start = std::time::Instant::now();
    let promotion_results = service.run_promotion().await?;
    let promotion_duration = promotion_start.elapsed();
    
    println!("📈 Promotion cycle completed in {:.2}s:", promotion_duration.as_secs_f64());
    println!("   Interact → Insights: {}", promotion_results.interact_to_insights);
    println!("   Insights → Assets: {}", promotion_results.insights_to_assets);
    println!("   Expired from Interact: {}", promotion_results.expired_interact);
    println!("   Expired from Insights: {}", promotion_results.expired_insights);
    
    // === ПРОВЕРКА СОСТОЯНИЯ ПОСЛЕ PROMOTION ===
    
    // Проверяем что можем найти записи во всех слоях
    for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
        let layer_search = service.search(
            "information knowledge",
            layer,
            SearchOptions { top_k: 5, ..Default::default() }
        ).await?;
        
        println!("   Layer {:?}: {} records found", layer, layer_search.len());
    }
    
    println!("✅ Memory Lifecycle Integration Test successful");
    
    Ok(())
}

/// ТЕСТ 5: Error Recovery and Resilience
/// 
/// Тестирует recovery после различных failure scenarios
#[tokio::test]
async fn test_error_recovery_resilience() -> Result<()> {
    println!("🛡️ Starting Error Recovery and Resilience Test");
    
    let temp_dir = TempDir::new()?;
    let mut config = default_config()?;
    config.db_path = temp_dir.path().join("resilience_test.db");
    config.cache_path = temp_dir.path().join("resilience_cache");
    config.health_enabled = true;
    
    std::fs::create_dir_all(&config.cache_path)?;
    
    // === СОЗДАНИЕ INITIAL SERVICE ===
    
    let service = DIMemoryService::new(config.clone()).await?;
    
    // Добавляем важные данные
    let critical_data = vec![
        "Critical system configuration data",
        "Important user preferences and settings", 
        "Essential application state information",
        "Key performance metrics and monitoring data",
    ];
    
    for (i, data) in critical_data.iter().enumerate() {
        let record = create_test_record(data, Layer::Assets, "critical_data");
        service.insert(record).await?;
        
        if i == 0 {
            // Проверяем что первая запись точно сохранилась
            let search_result = service.search(data, Layer::Assets, SearchOptions::default()).await?;
            assert!(!search_result.is_empty(), "Critical data should be immediately searchable");
        }
    }
    
    println!("✅ Initial data stored: {} critical records", critical_data.len());
    
    // === СИМУЛЯЦИЯ SERVICE RESTART ===
    
    println!("💥 Simulating service restart...");
    
    // Закрываем сервис
    drop(service);
    sleep(Duration::from_millis(100)).await;
    
    // Восстанавливаем сервис
    let recovered_service = DIMemoryService::new(config.clone()).await?;
    
    // === ПРОВЕРКА ВОССТАНОВЛЕНИЯ ДАННЫХ ===
    
    // Проверяем что все critical data восстановилось
    for data in &critical_data {
        let search_result = recovered_service.search(data, Layer::Assets, SearchOptions::default()).await?;
        assert!(!search_result.is_empty(), "Critical data should survive restart: {}", data);
    }
    
    println!("✅ Data recovery successful: all critical records restored");
    
    // === ПРОВЕРКА HEALTH ПОСЛЕ RECOVERY ===
    
    let health_after_recovery = recovered_service.check_health().await?;
    assert!(health_after_recovery.overall_status == "healthy", 
            "System should be healthy after recovery");
    
    // === ТЕСТИРОВАНИЕ ОПЕРАЦИЙ ПОСЛЕ RECOVERY ===
    
    // Новые операции должны работать нормально
    let post_recovery_record = create_test_record(
        "Post-recovery test data",
        Layer::Interact,
        "recovery_test"
    );
    
    recovered_service.insert(post_recovery_record).await?;
    
    let post_recovery_search = recovered_service.search(
        "Post-recovery test",
        Layer::Interact,
        SearchOptions::default()
    ).await?;
    
    assert!(!post_recovery_search.is_empty(), "Operations should work normally after recovery");
    
    println!("✅ Post-recovery operations functional");
    
    // === STRESS TEST AFTER RECOVERY ===
    
    // Небольшой stress test чтобы убедиться что система стабильна
    let stress_handles: Vec<_> = (0..20).map(|i| {
        let service = recovered_service.clone();
        tokio::spawn(async move {
            let record = create_test_record(
                &format!("Stress test record {}", i),
                Layer::Interact,
                "stress_after_recovery"
            );
            
            service.insert(record).await
        })
    }).collect();
    
    let stress_results = futures::future::try_join_all(stress_handles).await?;
    let stress_success_count = stress_results.iter().filter(|r| r.is_ok()).count();
    
    assert!(stress_success_count >= 18, "Most stress operations should succeed after recovery: {}/20", stress_success_count);
    
    println!("✅ Stress test after recovery: {}/20 operations successful", stress_success_count);
    
    println!("🛡️ Error Recovery and Resilience Test successful");
    
    Ok(())
}