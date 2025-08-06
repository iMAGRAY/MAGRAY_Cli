//! Performance Integration Tests
//! 
//! Comprehensive тесты производительности для валидации SLA требований:
//! - Sub-5ms search performance requirement 
//! - 100+ concurrent operations support
//! - Memory efficiency под sustained load
//! - Auto-scaling behavior validation
//! - Production throughput benchmarks

use anyhow::Result;
use memory::{
    DIMemoryService,
    service_di::default_config,
    Record, Layer, SearchOptions,
    CacheConfigType,
};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::time::{sleep, Duration, timeout, Instant};
use uuid::Uuid;
use chrono::Utc;
use std::collections::HashMap;


/// Утилита для создания performance test service
async fn create_performance_test_service() -> Result<DIMemoryService> {
    let temp_dir = TempDir::new()?;
    let mut config = default_config()?;
    
    // Оптимизированная конфигурация для производительности
    config.db_path = temp_dir.path().join("performance_test.db");
    config.cache_path = temp_dir.path().join("performance_cache");
    config.cache_config = CacheConfigType::InMemory { max_size: 20000 }; // Больший cache
    config.health_enabled = true;
    
    std::fs::create_dir_all(&config.cache_path)?;
    
    DIMemoryService::new(config).await
}

/// Создание тестовой записи для performance tests
fn create_perf_test_record(id: usize, content: &str, layer: Layer) -> Record {
    Record {
        id: Uuid::new_v4(),
        text: format!("Performance test record {}: {}", id, content),
        embedding: vec![], // Будет сгенерирован автоматически
        layer,
        kind: "performance_test".to_string(),
        tags: vec!["performance".to_string(), format!("batch_{}", id / 100)],
        project: "performance_testing".to_string(),
        session: "perf_session".to_string(),
        score: 0.75 + (id % 100) as f32 / 400.0, // Варьируем score
        ts: Utc::now(),
        access_count: (id % 50) as u32,
        last_access: Utc::now(),
    }
}

/// ТЕСТ 1: Sub-5ms Search SLA Validation
/// 
/// Валидирует что search operations выполняются в < 5ms
#[tokio::test]
async fn test_sub_5ms_search_sla_validation() -> Result<()> {
    println!("⚡ Starting Sub-5ms Search SLA Validation");
    
    let service = create_performance_test_service().await?;
    
    // === ПОДГОТОВКА PERFORMANCE DATA ===
    
    // Загружаем достаточно данных для realistic performance testing
    println!("📊 Loading performance test data...");
    
    let test_data_templates = vec![
        "Vector database optimization techniques and HNSW algorithm implementations",
        "Machine learning embeddings for semantic search and natural language processing",
        "High-performance computing strategies for distributed vector search systems",
        "Real-time search algorithms with sub-millisecond latency requirements",
        "Memory-efficient data structures for large-scale vector similarity computation",
        "Production-scale indexing strategies for high-dimensional vector spaces",
        "Parallel processing architectures for concurrent vector search operations",
        "Cache optimization patterns for improving search performance metrics",
        "Load balancing techniques for distributed vector database deployments",
        "Performance monitoring and SLA compliance in production search systems",
    ];
    
    // Создаем 1000 записей для realistic load
    for i in 0..1000 {
        let template = &test_data_templates[i % test_data_templates.len()];
        let record = create_perf_test_record(i, template, Layer::Interact);
        service.insert(record).await?;
        
        if i % 200 == 0 {
            println!("   Loaded {} records", i + 1);
        }
    }
    
    println!("✅ Test data loaded: 1000 records");
    
    // === WARM-UP PHASE ===
    
    println!("🔥 Warming up system...");
    
    // Выполняем warm-up searches для cache population
    let warmup_queries = vec![
        "vector database optimization",
        "machine learning embeddings", 
        "high-performance computing",
        "real-time search algorithms",
        "memory-efficient data structures",
    ];
    
    for query in &warmup_queries {
        let _results = service.search(
            query,
            Layer::Interact,
            SearchOptions { top_k: 10, ..Default::default() }
        ).await?;
    }
    
    println!("✅ System warmed up");
    
    // === SUB-5MS SLA TESTING ===
    
    println!("⏱️ Testing sub-5ms SLA compliance...");
    
    let sla_test_queries = vec![
        "vector optimization performance",
        "embedding search algorithms",
        "distributed computing systems", 
        "parallel processing techniques",
        "cache optimization strategies",
        "production search latency",
        "real-time vector similarity",
        "high-dimensional indexing",
        "memory efficient structures",
        "load balancing search",
    ];
    
    let mut search_times = Vec::new();
    let mut sla_violations = 0;
    
    // Выполняем 100 search операций для статистически значимых результатов
    for i in 0..100 {
        let query = &sla_test_queries[i % sla_test_queries.len()];
        
        let search_start = Instant::now();
        
        let results = service.search(
            query,
            Layer::Interact,
            SearchOptions { top_k: 10, ..Default::default() }
        ).await?;
        
        let search_duration = search_start.elapsed();
        let search_time_ms = search_duration.as_micros() as f64 / 1000.0;
        
        search_times.push(search_time_ms);
        
        // Проверяем SLA
        if search_time_ms >= 5.0 {
            sla_violations += 1;
            println!("⚠️ SLA violation {}: {:.3}ms for query '{}'", sla_violations, search_time_ms, query);
        }
        
        assert!(!results.is_empty(), "Search should return results for: {}", query);
        
        // Небольшая пауза между requests
        if i % 10 == 0 {
            sleep(Duration::from_micros(100)).await;
        }
    }
    
    // === СТАТИСТИЧЕСКИЙ АНАЛИЗ ===
    
    let total_searches = search_times.len();
    let avg_search_time = search_times.iter().sum::<f64>() / total_searches as f64;
    let min_search_time = search_times.iter().fold(f64::INFINITY, |acc, &x| acc.min(x));
    let max_search_time = search_times.iter().fold(0.0, |acc, &x| acc.max(x));
    
    // Вычисляем percentiles
    let mut sorted_times = search_times.clone();
    sorted_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let p50 = sorted_times[total_searches / 2];
    let p95 = sorted_times[(total_searches * 95) / 100];
    let p99 = sorted_times[(total_searches * 99) / 100];
    
    println!("📊 Search Performance Analysis:");
    println!("   Total searches: {}", total_searches);
    println!("   Average time: {:.3}ms", avg_search_time);
    println!("   Min time: {:.3}ms", min_search_time);
    println!("   Max time: {:.3}ms", max_search_time);
    println!("   P50 (median): {:.3}ms", p50);
    println!("   P95: {:.3}ms", p95);
    println!("   P99: {:.3}ms", p99);
    println!("   SLA violations: {}/{} ({:.1}%)", sla_violations, total_searches, 
             (sla_violations as f64 / total_searches as f64) * 100.0);
    
    // === SLA VALIDATION ===
    
    // Основные SLA требования
    assert!(avg_search_time < 5.0, "Average search SLA violation: {:.3}ms >= 5ms", avg_search_time);
    assert!(p95 < 8.0, "P95 latency too high: {:.3}ms", p95); // Некоторая толерантность для P95
    assert!(p99 < 15.0, "P99 latency too high: {:.3}ms", p99); // Толерантность для tail latency
    
    // Максимум 5% SLA violations допустимо
    let violation_rate = (sla_violations as f64 / total_searches as f64) * 100.0;
    assert!(violation_rate <= 5.0, "Too many SLA violations: {:.1}%", violation_rate);
    
    println!("✅ Sub-5ms Search SLA Validation successful");
    println!("   Average: {:.3}ms < 5ms ✓", avg_search_time);
    println!("   P95: {:.3}ms < 8ms ✓", p95);
    println!("   Violation rate: {:.1}% < 5% ✓", violation_rate);
    
    Ok(())
}

/// ТЕСТ 2: Concurrent Operations Performance
/// 
/// Тестирует производительность при 100+ concurrent operations
#[tokio::test]
async fn test_concurrent_operations_performance() -> Result<()> {
    println!("👥 Starting Concurrent Operations Performance Test");
    
    let service = Arc::new(create_performance_test_service().await?);
    
    // === ПОДГОТОВКА CONCURRENT DATA ===
    
    // Предварительно загружаем данные для concurrent testing
    for i in 0..500 {
        let record = create_perf_test_record(
            i,
            "Concurrent performance test data for high-load scenarios",
            Layer::Interact
        );
        service.insert(record).await?;
    }
    
    println!("✅ Concurrent test data prepared: 500 records");
    
    // === CONCURRENT READ OPERATIONS ===
    
    println!("📖 Testing concurrent read performance...");
    
    let read_start = Instant::now();
    let mut read_handles = Vec::new();
    
    // 150 concurrent читающих операций
    for i in 0..150 {
        let service_clone = service.clone();
        let handle = tokio::spawn(async move {
            let query = format!("concurrent performance test {}", i % 20);
            let search_start = Instant::now();
            
            let result = service_clone.search(
                &query,
                Layer::Interact,
                SearchOptions { top_k: 5, ..Default::default() }
            ).await;
            
            let search_time = search_start.elapsed();
            (result, search_time)
        });
        
        read_handles.push(handle);
    }
    
    // Ждем завершения всех read operations
    let read_results = timeout(
        Duration::from_secs(30),
        futures::future::try_join_all(read_handles)
    ).await??;
    
    let read_duration = read_start.elapsed();
    
    // === АНАЛИЗ READ PERFORMANCE ===
    
    let successful_reads = read_results.iter().filter(|(result, _)| result.is_ok()).count();
    let read_search_times: Vec<f64> = read_results.iter()
        .filter_map(|(result, time)| {
            if result.is_ok() {
                Some(time.as_micros() as f64 / 1000.0)
            } else {
                None
            }
        })
        .collect();
    
    let avg_concurrent_read_time = read_search_times.iter().sum::<f64>() / read_search_times.len() as f64;
    let reads_per_second = successful_reads as f64 / read_duration.as_secs_f64();
    
    println!("📊 Concurrent Read Results:");
    println!("   Successful reads: {}/150", successful_reads);
    println!("   Total duration: {:.2}s", read_duration.as_secs_f64());
    println!("   Reads per second: {:.1}", reads_per_second);
    println!("   Average search time: {:.3}ms", avg_concurrent_read_time);
    
    // === CONCURRENT WRITE OPERATIONS ===
    
    println!("✍️ Testing concurrent write performance...");
    
    let write_start = Instant::now();
    let mut write_handles = Vec::new();
    
    // 100 concurrent записывающих операций
    for i in 0..100 {
        let service_clone = service.clone();
        let handle = tokio::spawn(async move {
            let record = create_perf_test_record(
                i + 1000, // Избегаем конфликтов ID
                "Concurrent write performance test data",
                Layer::Interact
            );
            
            let write_start = Instant::now();
            let result = service_clone.insert(record).await;
            let write_time = write_start.elapsed();
            
            (result, write_time)
        });
        
        write_handles.push(handle);
    }
    
    let write_results = timeout(
        Duration::from_secs(30),
        futures::future::try_join_all(write_handles)
    ).await??;
    
    let write_duration = write_start.elapsed();
    
    // === АНАЛИЗ WRITE PERFORMANCE ===
    
    let successful_writes = write_results.iter().filter(|(result, _)| result.is_ok()).count();
    let write_times: Vec<f64> = write_results.iter()
        .filter_map(|(result, time)| {
            if result.is_ok() {
                Some(time.as_micros() as f64 / 1000.0)
            } else {
                None
            }
        })
        .collect();
    
    let avg_concurrent_write_time = write_times.iter().sum::<f64>() / write_times.len() as f64;
    let writes_per_second = successful_writes as f64 / write_duration.as_secs_f64();
    
    println!("📊 Concurrent Write Results:");
    println!("   Successful writes: {}/100", successful_writes);
    println!("   Total duration: {:.2}s", write_duration.as_secs_f64());
    println!("   Writes per second: {:.1}", writes_per_second);
    println!("   Average write time: {:.3}ms", avg_concurrent_write_time);
    
    // === MIXED CONCURRENT OPERATIONS ===
    
    println!("🔄 Testing mixed concurrent operations...");
    
    let mixed_start = Instant::now();
    let mut mixed_handles = Vec::new();
    
    // 120 mixed operations (80% reads, 20% writes)
    for i in 0..120 {
        let service_clone = service.clone();
        let handle = if i % 5 == 0 {
            // 20% write operations
            tokio::spawn(async move {
                let record = create_perf_test_record(
                    i + 2000,
                    "Mixed concurrent operation write test",
                    Layer::Interact
                );
                service_clone.insert(record).await.map(|_| "write")
            })
        } else {
            // 80% read operations
            tokio::spawn(async move {
                let query = format!("mixed concurrent test {}", i % 30);
                service_clone.search(
                    &query,
                    Layer::Interact,
                    SearchOptions { top_k: 3, ..Default::default() }
                ).await.map(|_| "read")
            })
        };
        
        mixed_handles.push(handle);
    }
    
    let mixed_results = timeout(
        Duration::from_secs(30),
        futures::future::try_join_all(mixed_handles)
    ).await??;
    
    let mixed_duration = mixed_start.elapsed();
    
    // === АНАЛИЗ MIXED PERFORMANCE ===
    
    let successful_mixed = mixed_results.iter().filter(|result| result.is_ok()).count();
    let mixed_ops_per_second = successful_mixed as f64 / mixed_duration.as_secs_f64();
    
    println!("📊 Mixed Operations Results:");
    println!("   Successful operations: {}/120", successful_mixed);
    println!("   Total duration: {:.2}s", mixed_duration.as_secs_f64());
    println!("   Operations per second: {:.1}", mixed_ops_per_second);
    
    // === PERFORMANCE VALIDATION ===
    
    // Требования производительности для concurrent operations
    assert!(successful_reads >= 140, "Too many failed concurrent reads: {}/150", successful_reads);
    assert!(successful_writes >= 90, "Too many failed concurrent writes: {}/100", successful_writes);
    assert!(successful_mixed >= 110, "Too many failed mixed operations: {}/120", successful_mixed);
    
    assert!(reads_per_second >= 20.0, "Concurrent read throughput too low: {:.1} ops/sec", reads_per_second);
    assert!(writes_per_second >= 10.0, "Concurrent write throughput too low: {:.1} ops/sec", writes_per_second);
    assert!(mixed_ops_per_second >= 15.0, "Mixed operations throughput too low: {:.1} ops/sec", mixed_ops_per_second);
    
    // Search latency должна оставаться разумной даже при concurrency
    assert!(avg_concurrent_read_time < 10.0, "Concurrent read latency too high: {:.3}ms", avg_concurrent_read_time);
    assert!(avg_concurrent_write_time < 50.0, "Concurrent write latency too high: {:.3}ms", avg_concurrent_write_time);
    
    println!("✅ Concurrent Operations Performance Test successful");
    println!("   Read throughput: {:.1} ops/sec", reads_per_second);
    println!("   Write throughput: {:.1} ops/sec", writes_per_second);
    println!("   Mixed throughput: {:.1} ops/sec", mixed_ops_per_second);
    
    Ok(())
}

/// ТЕСТ 3: Memory Efficiency Under Sustained Load
/// 
/// Тестирует memory efficiency при длительной нагрузке
#[tokio::test]
async fn test_memory_efficiency_sustained_load() -> Result<()> {
    println!("💾 Starting Memory Efficiency Under Sustained Load Test");
    
    let service = Arc::new(create_performance_test_service().await?);
    
    // === BASELINE MEMORY MEASUREMENT ===
    
    let initial_stats = service.get_stats().await;
    println!("📊 Initial memory stats:");
    println!("   Cache hits: {}", initial_stats.cache_hits);
    println!("   Cache misses: {}", initial_stats.cache_misses);
    
    // === SUSTAINED LOAD SIMULATION ===
    
    println!("🔄 Applying sustained load for memory efficiency testing...");
    
    let load_duration = Duration::from_secs(30); // 30 секунд sustained load
    let load_start = Instant::now();
    
    let mut operation_count = 0;
    let mut memory_snapshots = Vec::new();
    
    while load_start.elapsed() < load_duration {
        // Цикл операций: 70% reads, 30% writes
        for i in 0..10 {
            if i < 7 {
                // Read operation
                let query = format!("sustained load test {}", operation_count % 50);
                let _results = service.search(
                    &query,
                    Layer::Interact,
                    SearchOptions { top_k: 5, ..Default::default() }
                ).await?;
            } else {
                // Write operation
                let record = create_perf_test_record(
                    operation_count + 5000,
                    "Sustained load memory efficiency test data",
                    Layer::Interact
                );
                service.insert(record).await?;
            }
            
            operation_count += 1;
        }
        
        // Периодически снимаем memory snapshots
        if operation_count % 100 == 0 {
            let stats = service.get_stats().await;
            memory_snapshots.push((operation_count, stats.cache_hits, stats.cache_misses));
            
            println!("   Operations: {}, Cache hit rate: {:.1}%", 
                     operation_count,
                     if stats.cache_hits + stats.cache_misses > 0 {
                         stats.cache_hits as f64 / (stats.cache_hits + stats.cache_misses) as f64 * 100.0
                     } else { 0.0 });
        }
        
        // Небольшая пауза для предотвращения overwhelming
        sleep(Duration::from_millis(1)).await;
    }
    
    let final_load_duration = load_start.elapsed();
    
    // === MEMORY EFFICIENCY ANALYSIS ===
    
    let final_stats = service.get_stats().await;
    let final_operations_per_second = operation_count as f64 / final_load_duration.as_secs_f64();
    
    println!("📊 Sustained Load Results:");
    println!("   Duration: {:.2}s", final_load_duration.as_secs_f64());
    println!("   Total operations: {}", operation_count);
    println!("   Operations per second: {:.1}", final_operations_per_second);
    println!("   Final cache hits: {}", final_stats.cache_hits);
    println!("   Final cache misses: {}", final_stats.cache_misses);
    
    // === CACHE EFFICIENCY ANALYSIS ===
    
    let total_cache_operations = final_stats.cache_hits + final_stats.cache_misses;
    let cache_hit_rate = if total_cache_operations > 0 {
        final_stats.cache_hits as f64 / total_cache_operations as f64 * 100.0
    } else {
        0.0
    };
    
    println!("💾 Memory Efficiency Metrics:");
    println!("   Cache hit rate: {:.1}%", cache_hit_rate);
    println!("   Total cache operations: {}", total_cache_operations);
    
    // === MEMORY STABILITY CHECK ===
    
    // Проверяем что cache hit rate улучшается со временем (memory warming)
    if memory_snapshots.len() >= 3 {
        let early_snapshot = &memory_snapshots[0];
        let late_snapshot = &memory_snapshots[memory_snapshots.len() - 1];
        
        let early_hit_rate = if early_snapshot.1 + early_snapshot.2 > 0 {
            early_snapshot.1 as f64 / (early_snapshot.1 + early_snapshot.2) as f64 * 100.0
        } else {
            0.0
        };
        
        let late_hit_rate = if late_snapshot.1 + late_snapshot.2 > 0 {
            late_snapshot.1 as f64 / (late_snapshot.1 + late_snapshot.2) as f64 * 100.0
        } else {
            0.0
        };
        
        println!("   Early hit rate: {:.1}%", early_hit_rate);
        println!("   Late hit rate: {:.1}%", late_hit_rate);
        
        // Cache должен warming up со временем
        if late_hit_rate > early_hit_rate {
            println!("✅ Cache warming detected: {:.1}% → {:.1}%", early_hit_rate, late_hit_rate);
        }
    }
    
    // === PERFORMANCE REQUIREMENTS ===
    
    assert!(final_operations_per_second >= 50.0, 
            "Sustained load throughput too low: {:.1} ops/sec", final_operations_per_second);
    
    assert!(cache_hit_rate >= 30.0, 
            "Cache efficiency too low: {:.1}%", cache_hit_rate);
    
    // Проверяем что система остается responsive
    let health_check = service.check_health().await?;
    assert!(health_check.overall_status == "healthy", 
            "System should remain healthy under sustained load");
    
    println!("✅ Memory Efficiency Under Sustained Load Test successful");
    println!("   Sustained throughput: {:.1} ops/sec", final_operations_per_second);
    println!("   Cache efficiency: {:.1}%", cache_hit_rate);
    
    Ok(())
}

/// ТЕСТ 4: Production Throughput Benchmarks
/// 
/// Comprehensive production-style benchmarks
#[tokio::test]
async fn test_production_throughput_benchmarks() -> Result<()> {
    println!("🏭 Starting Production Throughput Benchmarks");
    
    let service = Arc::new(create_performance_test_service().await?);
    
    // === PRODUCTION DATA SETUP ===
    
    println!("📊 Setting up production-like dataset...");
    
    // Создаем realistic production dataset
    let production_templates = vec![
        "Customer support interaction: user query about product features and pricing",
        "Technical documentation: API endpoint specifications and usage examples", 
        "Business intelligence: market analysis and competitive research findings",
        "System monitoring: performance metrics and operational health indicators",
        "User behavior analytics: engagement patterns and conversion tracking data",
        "Product development: feature specifications and technical requirements",
        "Security audit: vulnerability assessment and compliance verification", 
        "Data processing: ETL pipeline configuration and transformation logic",
        "Machine learning: model training data and performance evaluation metrics",
        "Infrastructure management: deployment strategies and scaling configurations",
    ];
    
    // Загружаем 2000 записей для production-scale testing
    for i in 0..2000 {
        let template = &production_templates[i % production_templates.len()];
        let record = create_perf_test_record(i, template, Layer::Interact);
        service.insert(record).await?;
        
        if i % 400 == 0 {
            println!("   Loaded {} production records", i + 1);
        }
    }
    
    println!("✅ Production dataset loaded: 2000 records");
    
    // === PRODUCTION WORKLOAD BENCHMARKS ===
    
    println!("🔄 Running production workload benchmarks...");
    
    let benchmark_start = Instant::now();
    
    // Benchmark 1: Search-heavy workload (80% searches, 20% inserts)
    let search_heavy_start = Instant::now();
    let mut search_heavy_handles = Vec::new();
    
    for i in 0..200 {
        let service_clone = service.clone();
        let handle = if i % 5 == 0 {
            // 20% insert operations
            tokio::spawn(async move {
                let record = create_perf_test_record(
                    i + 10000,
                    "Production search-heavy workload insert",
                    Layer::Interact
                );
                service_clone.insert(record).await.map(|_| "insert")
            })
        } else {
            // 80% search operations
            tokio::spawn(async move {
                let queries = vec![
                    "customer support interaction",
                    "technical documentation API",
                    "business intelligence analysis",
                    "system monitoring metrics",
                    "user behavior analytics",
                ];
                let query = &queries[i % queries.len()];
                
                service_clone.search(
                    query,
                    Layer::Interact,
                    SearchOptions { top_k: 10, ..Default::default() }
                ).await.map(|_| "search")
            })
        };
        
        search_heavy_handles.push(handle);
    }
    
    let search_heavy_results = timeout(
        Duration::from_secs(45),
        futures::future::try_join_all(search_heavy_handles)
    ).await??;
    
    let search_heavy_duration = search_heavy_start.elapsed();
    let search_heavy_success = search_heavy_results.iter().filter(|r| r.is_ok()).count();
    let search_heavy_throughput = search_heavy_success as f64 / search_heavy_duration.as_secs_f64();
    
    println!("📊 Search-Heavy Workload Results:");
    println!("   Operations: {}/200", search_heavy_success);
    println!("   Duration: {:.2}s", search_heavy_duration.as_secs_f64());
    println!("   Throughput: {:.1} ops/sec", search_heavy_throughput);
    
    // Benchmark 2: Balanced workload (50% searches, 50% inserts)
    let balanced_start = Instant::now();
    let mut balanced_handles = Vec::new();
    
    for i in 0..100 {
        let service_clone = service.clone();
        let handle = if i % 2 == 0 {
            // 50% search operations
            tokio::spawn(async move {
                service_clone.search(
                    "production balanced workload test",
                    Layer::Interact,
                    SearchOptions { top_k: 5, ..Default::default() }
                ).await.map(|_| "search")
            })
        } else {
            // 50% insert operations
            tokio::spawn(async move {
                let record = create_perf_test_record(
                    i + 20000,
                    "Production balanced workload insert",
                    Layer::Interact
                );
                service_clone.insert(record).await.map(|_| "insert")
            })
        };
        
        balanced_handles.push(handle);
    }
    
    let balanced_results = timeout(
        Duration::from_secs(30),
        futures::future::try_join_all(balanced_handles)
    ).await??;
    
    let balanced_duration = balanced_start.elapsed();
    let balanced_success = balanced_results.iter().filter(|r| r.is_ok()).count();
    let balanced_throughput = balanced_success as f64 / balanced_duration.as_secs_f64();
    
    println!("📊 Balanced Workload Results:");
    println!("   Operations: {}/100", balanced_success);
    println!("   Duration: {:.2}s", balanced_duration.as_secs_f64());
    println!("   Throughput: {:.1} ops/sec", balanced_throughput);
    
    // === LATENCY DISTRIBUTION ANALYSIS ===
    
    println!("⏱️ Analyzing latency distribution...");
    
    let mut latency_samples = Vec::new();
    
    for i in 0..50 {
        let query = format!("latency analysis test {}", i);
        let latency_start = Instant::now();
        
        let _results = service.search(
            &query,
            Layer::Interact,
            SearchOptions { top_k: 5, ..Default::default() }
        ).await?;
        
        let latency = latency_start.elapsed().as_micros() as f64 / 1000.0;
        latency_samples.push(latency);
    }
    
    latency_samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let latency_avg = latency_samples.iter().sum::<f64>() / latency_samples.len() as f64;
    let latency_p50 = latency_samples[latency_samples.len() / 2];
    let latency_p95 = latency_samples[(latency_samples.len() * 95) / 100];
    let latency_p99 = latency_samples[(latency_samples.len() * 99) / 100];
    
    println!("📊 Latency Distribution:");
    println!("   Average: {:.3}ms", latency_avg);
    println!("   P50: {:.3}ms", latency_p50);
    println!("   P95: {:.3}ms", latency_p95);
    println!("   P99: {:.3}ms", latency_p99);
    
    let total_benchmark_duration = benchmark_start.elapsed();
    
    // === PRODUCTION REQUIREMENTS VALIDATION ===
    
    // Production throughput requirements
    assert!(search_heavy_throughput >= 30.0, 
            "Search-heavy throughput too low: {:.1} ops/sec", search_heavy_throughput);
    
    assert!(balanced_throughput >= 20.0, 
            "Balanced throughput too low: {:.1} ops/sec", balanced_throughput);
    
    // Latency requirements for production
    assert!(latency_avg < 5.0, "Average latency SLA violation: {:.3}ms", latency_avg);
    assert!(latency_p95 < 10.0, "P95 latency too high: {:.3}ms", latency_p95);
    assert!(latency_p99 < 20.0, "P99 latency too high: {:.3}ms", latency_p99);
    
    // Success rate requirements
    assert!(search_heavy_success >= 190, "Too many search-heavy failures: {}/200", search_heavy_success);
    assert!(balanced_success >= 95, "Too many balanced workload failures: {}/100", balanced_success);
    
    println!("✅ Production Throughput Benchmarks successful");
    println!("   Total benchmark duration: {:.2}s", total_benchmark_duration.as_secs_f64());
    println!("   Search-heavy throughput: {:.1} ops/sec", search_heavy_throughput);
    println!("   Balanced throughput: {:.1} ops/sec", balanced_throughput);
    println!("   Average latency: {:.3}ms", latency_avg);
    println!("   P95 latency: {:.3}ms", latency_p95);
    
    Ok(())
}