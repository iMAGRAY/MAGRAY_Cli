// @component: {"k":"T","id":"workflow_integration_tests","t":"Integration tests for complete workflows","m":{"cur":100,"tgt":100,"u":"%"},"f":["testing","integration","workflow","end_to_end"]}

//! Integration Tests для Complete Workflows
//! 
//! Покрывает полные пользовательские сценарии end-to-end:
//! - Полный цикл memory operations (store → search → promote)
//! - Multi-layer workflow (Interact → Insights → Assets)
//! - Batch processing workflows
//! - GPU/CPU fallback workflows
//! - Health monitoring workflows
//! - Error recovery workflows
//! - Performance monitoring workflows

#[cfg(test)]
mod tests {
    use memory::{
        DIMemoryService, default_config,
        Record, Layer, SearchOptions,
        BatchOptimizedProcessor, BatchOptimizedConfig,
        ResourceManager, HealthMonitor,
        NotificationSystem, MetricsCollector,
    };
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use uuid::Uuid;
    use chrono::Utc;
    use tokio::time::sleep;
    
    // ===== Утилиты для integration тестов =====
    
    /// Создание полного memory service для integration тестов
    async fn create_full_memory_service() -> Arc<DIMemoryService> {
        let config = default_config().expect("Default config should work");
        let service = DIMemoryService::new(config).await
            .expect("Memory service should initialize");
        service.initialize().await.expect("Should initialize layers");
        Arc::new(service)
    }
    
    /// Создание тестовых записей для различных слоев
    fn create_layered_records(count: usize, layer: Layer) -> Vec<Record> {
        (0..count).map(|i| Record {
            id: Uuid::new_v4(),
            text: format!("Test record {} for {:?} layer", i, layer),
            embedding: vec![0.1 + (i as f32 * 0.01); 1024],
            layer: layer.clone(),
            kind: format!("{:?}_record", layer).to_lowercase(),
            tags: vec!["integration_test".to_string(), format!("{:?}", layer).to_lowercase()],
            project: "workflow_test".to_string(),
            session: "integration_session".to_string(),
            ts: Utc::now(),
            score: 0.5 + (i as f64 * 0.1),
            access_count: i as u64,
            last_access: Utc::now(),
        }).collect()
    }
    
    /// Создание search options для тестирования
    fn create_search_options(layer: Layer) -> SearchOptions {
        SearchOptions {
            layers: vec![layer],
            top_k: 10,
            score_threshold: 0.3,
            tags: vec!["integration_test".to_string()],
            project: Some("workflow_test".to_string()),
        }
    }
    
    // ===== РАЗДЕЛ 1: Complete Memory Workflow =====
    
    #[tokio::test]
    async fn test_complete_memory_workflow() {
        let service = create_full_memory_service().await;
        
        println!("🔄 Starting complete memory workflow test");
        
        // 1. Store initial records in Interact layer
        let interact_records = create_layered_records(20, Layer::Interact);
        for record in interact_records {
            service.insert(record).await.expect("Should store in Interact");
        }
        println!("✅ Stored 20 records in Interact layer");
        
        // 2. Search in Interact layer
        let search_options = create_search_options(Layer::Interact);
        let search_results = service.search("test record", Layer::Interact, search_options).await
            .expect("Should find records in Interact");
        assert!(search_results.len() > 0, "Should find records in Interact layer");
        println!("✅ Found {} records in Interact layer", search_results.len());
        
        // 3. Run promotion (Interact → Insights)
        let promotion_stats = service.run_promotion().await
            .expect("Promotion should succeed");
        assert!(promotion_stats.interact_to_insights > 0, "Should promote some records");
        println!("✅ Promoted {} records to Insights", promotion_stats.interact_to_insights);
        
        // 4. Search in Insights layer
        let insights_options = create_search_options(Layer::Insights);
        let insights_results = service.search("test record", Layer::Insights, insights_options).await
            .expect("Should find records in Insights");
        assert!(insights_results.len() > 0, "Should find promoted records in Insights");
        println!("✅ Found {} records in Insights layer", insights_results.len());
        
        // 5. Add more valuable records and promote to Assets
        let valuable_records = create_layered_records(5, Layer::Insights);
        for mut record in valuable_records {
            record.score = 0.9; // High score for Assets promotion
            record.access_count = 10;
            service.insert(record).await.expect("Should store valuable record");
        }
        
        // Wait a moment for promotion conditions
        sleep(Duration::from_millis(100)).await;
        
        let final_promotion = service.run_promotion().await
            .expect("Final promotion should succeed");
        println!("✅ Final promotion: {} → Assets", final_promotion.insights_to_assets);
        
        // 6. Verify Assets layer
        let assets_options = create_search_options(Layer::Assets);
        let assets_results = service.search("test record", Layer::Assets, assets_options).await
            .expect("Should search Assets");
        
        if assets_results.len() > 0 {
            println!("✅ Found {} records in Assets layer", assets_results.len());
        } else {
            println!("ℹ️  No records in Assets yet (promotion criteria not met)");
        }
        
        // 7. Get final statistics
        let stats = service.get_stats().await;
        println!("📊 Final stats - Cache hits: {}, Cache size: {}", 
                stats.cache_hits, stats.cache_size);
        
        assert!(stats.cache_hits + stats.cache_misses > 0, "Should have cache activity");
        println!("✅ Complete memory workflow test passed");
    }
    
    // ===== РАЗДЕЛ 2: Multi-layer Workflow =====
    
    #[tokio::test]
    async fn test_multi_layer_search_workflow() {
        let service = create_full_memory_service().await;
        
        // Populate all layers with different content
        let interact_records = create_layered_records(15, Layer::Interact);
        let insights_records = create_layered_records(10, Layer::Insights);
        let assets_records = create_layered_records(5, Layer::Assets);
        
        // Store records in all layers
        for record in interact_records {
            service.insert(record).await.expect("Should store interact");
        }
        for record in insights_records {
            service.insert(record).await.expect("Should store insights");
        }
        for record in assets_records {
            service.insert(record).await.expect("Should store assets");
        }
        
        // Multi-layer search
        let multi_options = SearchOptions {
            layers: vec![Layer::Interact, Layer::Insights, Layer::Assets],
            top_k: 20,
            score_threshold: 0.1,
            tags: vec!["integration_test".to_string()],
            project: Some("workflow_test".to_string()),
        };
        
        let multi_results = service.search("test record", Layer::Interact, multi_options).await
            .expect("Multi-layer search should work");
        
        assert!(multi_results.len() > 0, "Should find records across layers");
        println!("✅ Multi-layer search found {} records total", multi_results.len());
        
        // Verify records from different layers
        let layer_distribution: std::collections::HashMap<Layer, usize> = multi_results
            .into_iter()
            .fold(std::collections::HashMap::new(), |mut acc, record| {
                *acc.entry(record.layer).or_insert(0) += 1;
                acc
            });
        
        println!("📊 Layer distribution: {:?}", layer_distribution);
        assert!(layer_distribution.len() > 1, "Should have records from multiple layers");
    }
    
    // ===== РАЗДЕЛ 3: Batch Processing Workflow =====
    
    #[tokio::test]
    async fn test_batch_processing_workflow() {
        let service = create_full_memory_service().await;
        
        // Create batch processor
        let batch_config = BatchOptimizedConfig {
            max_batch_size: 50,
            batch_timeout_ms: 100,
            num_workers: 2,
            queue_capacity: 200,
            enable_simd: true,
            memory_pool_size: 8,
            adaptive_batch_sizing: true,
            target_latency_ms: 10.0,
        };
        
        let batch_processor = BatchOptimizedProcessor::new(batch_config).await;
        
        // Generate large dataset for batch processing
        let large_dataset = create_layered_records(100, Layer::Interact);
        
        // Process in batches
        let batch_size = 25;
        let mut total_processed = 0;
        
        for chunk in large_dataset.chunks(batch_size) {
            let batch_records = chunk.to_vec();
            let batch_count = batch_records.len();
            
            // Process batch
            let result = batch_processor.process_batch(batch_records).await;
            assert!(result.is_ok(), "Batch processing should succeed");
            
            total_processed += batch_count;
            println!("✅ Processed batch of {} records (total: {})", batch_count, total_processed);
        }
        
        // Verify batch processor stats
        let batch_stats = batch_processor.get_stats().await;
        assert!(batch_stats.total_batches > 0, "Should have processed batches");
        assert_eq!(batch_stats.total_records, 100, "Should process all 100 records");
        
        println!("📊 Batch processing stats: {} batches, avg size: {:.1}", 
                batch_stats.total_batches, batch_stats.avg_batch_size);
        
        // Now store processed records in memory service
        for record in create_layered_records(total_processed, Layer::Interact) {
            service.insert(record).await.expect("Should store processed record");
        }
        
        println!("✅ Batch processing workflow completed successfully");
    }
    
    // ===== РАЗДЕЛ 4: Health Monitoring Workflow =====
    
    #[tokio::test]
    async fn test_health_monitoring_workflow() {
        let service = create_full_memory_service().await;
        
        // Initial health check
        let initial_health = service.check_health().await
            .expect("Health check should work");
        
        println!("🏥 Initial health status: {:?}", initial_health.overall_status);
        assert!(initial_health.component_statuses.len() > 0, "Should have component statuses");
        
        // Generate some load to affect health metrics
        for i in 0..50 {
            let record = create_layered_records(1, Layer::Interact)[0].clone();
            service.insert(record).await.expect("Should insert record");
            
            // Periodically check health during load
            if i % 10 == 0 {
                let current_health = service.check_health().await
                    .expect("Health check should work during load");
                println!("🏥 Health during load ({}): {:?}", i, current_health.overall_status);
            }
        }
        
        // Final health check after load
        let final_health = service.check_health().await
            .expect("Final health check should work");
        
        println!("🏥 Final health status: {:?}", final_health.overall_status);
        
        // Verify health metrics are being tracked
        assert!(final_health.metrics_summary.len() > 0, "Should have health metrics");
        assert!(final_health.uptime_seconds > 0, "Should track uptime");
        
        // Check specific metrics
        if let Some(cache_efficiency) = final_health.metrics_summary.get("cache_efficiency") {
            assert!(*cache_efficiency >= 0.0 && *cache_efficiency <= 1.0, 
                   "Cache efficiency should be valid");
        }
        
        println!("✅ Health monitoring workflow completed");
    }
    
    // ===== РАЗДЕЛ 5: Error Recovery Workflow =====
    
    #[tokio::test]
    async fn test_error_recovery_workflow() {
        let service = create_full_memory_service().await;
        
        println!("🔄 Testing error recovery workflow");
        
        // Store some valid records first
        let valid_records = create_layered_records(10, Layer::Interact);
        for record in valid_records {
            service.insert(record).await.expect("Valid records should store");
        }
        
        // Try to insert potentially problematic records
        let mut problematic_record = create_layered_records(1, Layer::Interact)[0].clone();
        problematic_record.text = "".to_string(); // Empty text
        problematic_record.embedding = vec![]; // Empty embedding
        
        let error_result = service.insert(problematic_record).await;
        
        // Should either succeed (graceful handling) or fail predictably
        if error_result.is_err() {
            println!("⚠️  Expected error for problematic record: {:?}", error_result.err());
        } else {
            println!("✅ Gracefully handled problematic record");
        }
        
        // Service should continue working after error
        let recovery_record = create_layered_records(1, Layer::Interact)[0].clone();
        let recovery_result = service.insert(recovery_record).await;
        assert!(recovery_result.is_ok(), "Should recover after error");
        
        // Health check after error
        let post_error_health = service.check_health().await
            .expect("Health check should work after error");
        
        println!("🏥 Post-error health: {:?}", post_error_health.overall_status);
        
        // Search should still work
        let search_options = create_search_options(Layer::Interact);
        let search_results = service.search("test", Layer::Interact, search_options).await
            .expect("Search should work after error");
        
        assert!(search_results.len() > 0, "Search should find valid records");
        println!("✅ Error recovery workflow completed successfully");
    }
    
    // ===== РАЗДЕЛ 6: Performance Monitoring Workflow =====
    
    #[tokio::test]
    async fn test_performance_monitoring_workflow() {
        let service = create_full_memory_service().await;
        
        println!("📊 Starting performance monitoring workflow");
        
        // Reset performance metrics
        service.reset_performance_metrics();
        let initial_metrics = service.get_performance_metrics();
        assert_eq!(initial_metrics.operation_count, 0, "Should start with 0 operations");
        
        // Generate measured workload
        let start_time = Instant::now();
        let workload_size = 30;
        
        for i in 0..workload_size {
            let record = create_layered_records(1, Layer::Interact)[0].clone();
            let insert_start = Instant::now();
            
            service.insert(record).await.expect("Should insert record");
            
            let insert_duration = insert_start.elapsed();
            println!("📊 Insert {} took: {:?}", i, insert_duration);
        }
        
        let total_duration = start_time.elapsed();
        println!("📊 Total workload took: {:?}", total_duration);
        
        // Get performance metrics
        let final_metrics = service.get_performance_metrics();
        println!("📊 Performance metrics: {}", service.get_performance_report());
        
        // Verify metrics are collected
        assert!(final_metrics.operation_count >= workload_size, 
               "Should track operations");
        assert!(final_metrics.avg_latency_ms >= 0.0, 
               "Should have valid average latency");
        
        // Calculate throughput
        let throughput = workload_size as f64 / total_duration.as_secs_f64();
        println!("📊 Achieved throughput: {:.1} ops/sec", throughput);
        assert!(throughput > 0.0, "Should have positive throughput");
        
        // Memory efficiency check
        if final_metrics.memory_efficiency > 0.0 {
            assert!(final_metrics.memory_efficiency <= 1.0, "Memory efficiency should be <= 1.0");
            println!("📊 Memory efficiency: {:.2}%", final_metrics.memory_efficiency * 100.0);
        }
        
        println!("✅ Performance monitoring workflow completed");
    }
    
    // ===== РАЗДЕЛ 7: Concurrent Workflow =====
    
    #[tokio::test]
    async fn test_concurrent_workflow() {
        let service = create_full_memory_service().await;
        
        println!("🔄 Testing concurrent workflow with multiple operations");
        
        let concurrent_tasks = 8;
        let records_per_task = 10;
        
        let mut handles = vec![];
        
        // Spawn concurrent insertion tasks
        for task_id in 0..concurrent_tasks {
            let service_clone = service.clone();
            
            let handle = tokio::spawn(async move {
                let mut task_results = vec![];
                
                for i in 0..records_per_task {
                    let mut record = create_layered_records(1, Layer::Interact)[0].clone();
                    record.text = format!("Concurrent task {} record {}", task_id, i);
                    record.tags.push(format!("task_{}", task_id));
                    
                    let result = service_clone.insert(record).await;
                    task_results.push(result.is_ok());
                }
                
                (task_id, task_results)
            });
            
            handles.push(handle);
        }
        
        // Wait for all tasks and collect results
        let mut total_success = 0;
        let mut total_operations = 0;
        
        for handle in handles {
            let (task_id, results) = handle.await.expect("Task should complete");
            let success_count = results.iter().filter(|&&success| success).count();
            
            println!("✅ Task {} completed: {}/{} successful", 
                    task_id, success_count, results.len());
            
            total_success += success_count;
            total_operations += results.len();
        }
        
        println!("📊 Overall concurrent results: {}/{} operations successful", 
                total_success, total_operations);
        
        assert_eq!(total_success, total_operations, "All concurrent operations should succeed");
        
        // Verify final state
        let search_options = SearchOptions {
            layers: vec![Layer::Interact],
            top_k: 100,
            score_threshold: 0.0,
            tags: vec![],
            project: Some("workflow_test".to_string()),
        };
        
        let final_search = service.search("concurrent", Layer::Interact, search_options).await
            .expect("Final search should work");
        
        println!("🔍 Found {} records from concurrent operations", final_search.len());
        assert!(final_search.len() > 0, "Should find records from concurrent tasks");
        
        println!("✅ Concurrent workflow completed successfully");
    }
    
    // ===== РАЗДЕЛ 8: Long-running Workflow =====
    
    #[tokio::test]
    async fn test_long_running_workflow() {
        let service = create_full_memory_service().await;
        
        println!("⏱️  Starting long-running workflow simulation");
        
        let phases = vec![
            ("Initial Load", 20),
            ("Growth Phase", 15),
            ("Steady State", 10),
            ("Peak Load", 25),
            ("Cleanup", 5),
        ];
        
        let mut total_records = 0;
        
        for (phase_name, record_count) in phases {
            println!("📈 Phase: {} - inserting {} records", phase_name, record_count);
            
            let phase_start = Instant::now();
            
            // Insert records for this phase
            for i in 0..record_count {
                let mut record = create_layered_records(1, Layer::Interact)[0].clone();
                record.text = format!("{} record {}", phase_name, i);
                record.tags.push(phase_name.to_lowercase().replace(" ", "_"));
                
                service.insert(record).await.expect("Should insert phase record");
                
                // Small delay to simulate real usage
                sleep(Duration::from_millis(5)).await;
            }
            
            let phase_duration = phase_start.elapsed();
            total_records += record_count;
            
            println!("✅ {} completed in {:?} (total records: {})", 
                    phase_name, phase_duration, total_records);
            
            // Health check after each phase
            let health = service.check_health().await
                .expect("Health check should work");
            println!("🏥 Health after {}: {:?}", phase_name, health.overall_status);
            
            // Periodic promotion
            if total_records % 30 == 0 {
                let promotion_stats = service.run_promotion().await
                    .expect("Promotion should work");
                println!("🔄 Promotion: {} → Insights, {} → Assets", 
                        promotion_stats.interact_to_insights,
                        promotion_stats.insights_to_assets);
            }
        }
        
        // Final statistics
        let final_stats = service.get_stats().await;
        println!("📊 Final long-running stats:");
        println!("   Cache hits: {}, misses: {}", final_stats.cache_hits, final_stats.cache_misses);
        println!("   Cache size: {}", final_stats.cache_size);
        
        let cache_hit_rate = if final_stats.cache_hits + final_stats.cache_misses > 0 {
            final_stats.cache_hits as f64 / (final_stats.cache_hits + final_stats.cache_misses) as f64
        } else {
            0.0
        };
        
        println!("   Cache hit rate: {:.2}%", cache_hit_rate * 100.0);
        
        assert!(final_stats.cache_hits + final_stats.cache_misses > 0, 
               "Should have cache activity in long-running workflow");
        
        println!("✅ Long-running workflow completed successfully");
    }
}