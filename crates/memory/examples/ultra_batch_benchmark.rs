//! Ultra-optimized batch operations benchmark для достижения 1000+ QPS
//! 
//! Comprehensive performance testing:
//! - Batch insert operations throughput
//! - Concurrent search operations QPS  
//! - Cache hit rate optimization
//! - SIMD operations efficiency
//! - Sub-5ms latency validation
//!
//! @component: {"k":"T","id":"ultra_batch_benchmark","t":"1000+ QPS batch benchmark suite","m":{"cur":100,"tgt":100,"u":"%"},"f":["benchmark","1000qps","sub-5ms","simd","batch","performance"]}

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::{sleep, timeout};
use uuid::Uuid;
// use rayon::prelude::*; // Unused for now
use memory::{
    BatchOptimizedProcessor, BatchOptimizedConfig, BatchOptimizedStats,
    Layer, Record
};

/// Benchmark configuration для comprehensive testing
#[derive(Debug, Clone)]
struct BenchmarkConfig {
    /// Количество concurrent operations для throughput test
    pub concurrent_operations: usize,
    /// Размер batch для testing optimal batching
    pub batch_sizes: Vec<usize>,
    /// Количество test vectors для каждого benchmark
    pub test_vector_count: usize,
    /// Dimension vectors для testing
    pub vector_dimension: usize,
    /// Таймаут для каждого benchmark
    pub benchmark_timeout_s: u64,
    /// Количество warm-up iterations
    pub warmup_iterations: usize,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            concurrent_operations: 100,  // High concurrency для 1000+ QPS test
            batch_sizes: vec![32, 64, 128, 256, 512, 1024],
            test_vector_count: 10000,
            vector_dimension: 1024,  // Qwen3 dimension
            benchmark_timeout_s: 30,
            warmup_iterations: 1000,
        }
    }
}

/// Результаты benchmark testing
#[derive(Debug)]
struct BenchmarkResults {
    /// Throughput в QPS (queries per second)
    pub throughput_qps: f64,
    /// Средняя latency в milliseconds
    pub avg_latency_ms: f64,
    /// P95 latency в milliseconds
    pub p95_latency_ms: f64,
    /// P99 latency в milliseconds
    pub p99_latency_ms: f64,
    /// Cache hit rate (0.0 - 1.0)
    pub cache_hit_rate: f64,
    /// Количество SIMD operations
    pub simd_operations: u64,
    /// Success rate (0.0 - 1.0)
    pub success_rate: f64,
}

impl BenchmarkResults {
    pub fn meets_sla(&self) -> bool {
        self.throughput_qps >= 1000.0 &&  // Target: 1000+ QPS
        self.avg_latency_ms <= 5.0 &&     // Target: sub-5ms average
        self.p95_latency_ms <= 10.0 &&    // Target: sub-10ms P95
        self.success_rate >= 0.99          // Target: 99% success rate
    }
}

/// Ultra-optimized batch benchmark suite
pub struct UltraBatchBenchmark {
    processor: Arc<BatchOptimizedProcessor>,
    config: BenchmarkConfig,
}

impl UltraBatchBenchmark {
    /// Создать новый benchmark suite
    pub async fn new(config: BenchmarkConfig) -> Result<Self, Box<dyn std::error::Error>> {
        println!("🚀 Initializing Ultra Batch Benchmark Suite");
        
        // Конфигурация для maximum performance
        let processor_config = BatchOptimizedConfig {
            max_batch_size: 1024,        // Large batches для high throughput
            min_batch_size: 32,          // Reasonable minimum
            worker_threads: 16,          // High parallelism
            queue_capacity: 4096,        // Large queue для burst handling
            batch_timeout_us: 50,        // Ultra-low timeout для latency
            use_prefetching: true,       // Memory prefetching
            use_aligned_memory: true,    // Cache-aligned allocation
            adaptive_batching: true,     // Dynamic optimization
        };
        
        let processor = Arc::new(BatchOptimizedProcessor::new(processor_config)?);
        
        println!("✅ Ultra-optimized batch processor initialized");
        
        Ok(Self {
            processor,
            config,
        })
    }
    
    /// Запустить complete benchmark suite
    pub async fn run_comprehensive_benchmark(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n🧪 === ULTRA BATCH BENCHMARK SUITE ===");
        println!("Target: 1000+ QPS with sub-5ms latency");
        
        // Warm-up processor
        self.warmup_processor().await?;
        
        // Benchmark 1: Single batch insert throughput
        println!("\n📊 1. Batch Insert Throughput Test");
        let insert_results = self.benchmark_batch_insert().await?;
        self.print_results("Batch Insert", &insert_results);
        
        // Benchmark 2: Concurrent search operations
        println!("\n📊 2. Concurrent Search Operations Test");
        let search_results = self.benchmark_concurrent_search().await?;
        self.print_results("Concurrent Search", &search_results);
        
        // Benchmark 3: Mixed workload (insert + search)
        println!("\n📊 3. Mixed Workload Test");
        let mixed_results = self.benchmark_mixed_workload().await?;
        self.print_results("Mixed Workload", &mixed_results);
        
        // Benchmark 4: Batch size optimization
        println!("\n📊 4. Batch Size Optimization Test");
        self.benchmark_batch_sizes().await?;
        
        // Benchmark 5: Cache performance
        println!("\n📊 5. Cache Performance Test");
        let cache_results = self.benchmark_cache_performance().await?;
        self.print_results("Cache Performance", &cache_results);
        
        // Final SLA validation
        println!("\n🎯 === SLA VALIDATION ===");
        let meets_insert_sla = insert_results.meets_sla();
        let meets_search_sla = search_results.meets_sla();
        let meets_mixed_sla = mixed_results.meets_sla();
        
        println!("Insert SLA (1000+ QPS, sub-5ms): {}", if meets_insert_sla { "✅ PASS" } else { "❌ FAIL" });
        println!("Search SLA (1000+ QPS, sub-5ms): {}", if meets_search_sla { "✅ PASS" } else { "❌ FAIL" });
        println!("Mixed SLA (1000+ QPS, sub-5ms): {}", if meets_mixed_sla { "✅ PASS" } else { "❌ FAIL" });
        
        if meets_insert_sla && meets_search_sla && meets_mixed_sla {
            println!("\n🏆 ALL SLA TARGETS ACHIEVED! Ultra-optimized batch system ready for production.");
        } else {
            println!("\n⚠️  Some SLA targets not met. Additional optimization may be required.");
        }
        
        // Final processor stats
        self.print_processor_stats();
        
        Ok(())
    }
    
    /// Warm-up processor для consistent benchmarks
    async fn warmup_processor(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔥 Warming up processor with {} iterations...", self.config.warmup_iterations);
        
        let mut warmup_records = Vec::new();
        for i in 0..self.config.warmup_iterations {
            warmup_records.push(self.create_test_record(i, Layer::Interact));
        }
        
        let start = Instant::now();
        let _response = self.processor.insert_batch(warmup_records).await?;
        let warmup_time = start.elapsed();
        
        println!("✅ Warmup completed in {:.2}ms", warmup_time.as_millis() as f64);
        
        Ok(())
    }
    
    /// Benchmark batch insert operations
    async fn benchmark_batch_insert(&self) -> Result<BenchmarkResults, Box<dyn std::error::Error>> {
        let batch_size = 512;  // Optimal batch size
        let total_batches = self.config.test_vector_count / batch_size;
        
        println!("Testing {} batches of {} vectors each...", total_batches, batch_size);
        
        let mut latencies = Vec::new();
        let mut successful_operations = 0;
        
        let overall_start = Instant::now();
        
        for batch_idx in 0..total_batches {
            let mut batch_records = Vec::new();
            for i in 0..batch_size {
                let record_idx = batch_idx * batch_size + i;
                batch_records.push(self.create_test_record(record_idx, Layer::Interact));
            }
            
            let start = Instant::now();
            match timeout(
                Duration::from_secs(1), 
                self.processor.insert_batch(batch_records)
            ).await {
                Ok(Ok(_)) => {
                    let latency = start.elapsed();
                    latencies.push(latency.as_nanos() as f64 / 1_000_000.0); // Convert to ms
                    successful_operations += 1;
                }
                Ok(Err(e)) => {
                    eprintln!("Batch insert failed: {}", e);
                }
                Err(_) => {
                    eprintln!("Batch insert timeout");
                }
            }
            
            // Small delay to prevent overwhelming
            if batch_idx % 50 == 0 {
                sleep(Duration::from_millis(1)).await;
            }
        }
        
        let total_time = overall_start.elapsed();
        let stats = self.processor.get_stats();
        
        Ok(self.calculate_results(latencies, successful_operations, total_batches, total_time, &stats))
    }
    
    /// Benchmark concurrent search operations
    async fn benchmark_concurrent_search(&self) -> Result<BenchmarkResults, Box<dyn std::error::Error>> {
        // Сначала заполняем данными для поиска
        let setup_records = (0..1000).map(|i| self.create_test_record(i, Layer::Interact)).collect();
        self.processor.insert_batch(setup_records).await?;
        
        let concurrent_ops = self.config.concurrent_operations;
        let total_searches = concurrent_ops * 10;  // 10 searches per concurrent operation
        
        println!("Testing {} concurrent search operations ({} total searches)...", concurrent_ops, total_searches);
        
        let search_handles: Vec<_> = (0..concurrent_ops).map(|op_id| {
            let processor = self.processor.clone();
            tokio::spawn(async move {
                let mut latencies = Vec::new();
                let mut successful = 0;
                
                for i in 0..10 {
                    let query = Self::create_test_vector(op_id * 10 + i, 1024);
                    let start = Instant::now();
                    
                    match timeout(
                        Duration::from_millis(100),
                        processor.search(query, 10, Some(Layer::Interact))
                    ).await {
                        Ok(Ok(_results)) => {
                            let latency = start.elapsed().as_nanos() as f64 / 1_000_000.0;
                            latencies.push(latency);
                            successful += 1;
                        }
                        Ok(Err(e)) => {
                            eprintln!("Search failed in op {}: {}", op_id, e);
                        }
                        Err(_) => {
                            eprintln!("Search timeout in op {}", op_id);
                        }
                    }
                }
                
                (latencies, successful)
            })
        }).collect();
        
        let overall_start = Instant::now();
        
        // Collect results from all concurrent operations
        let mut all_latencies = Vec::new();
        let mut total_successful = 0;
        
        for handle in search_handles {
            if let Ok((latencies, successful)) = handle.await {
                all_latencies.extend(latencies);
                total_successful += successful;
            }
        }
        
        let total_time = overall_start.elapsed();
        let stats = self.processor.get_stats();
        
        Ok(self.calculate_results(all_latencies, total_successful, total_searches, total_time, &stats))
    }
    
    /// Benchmark mixed workload (insert + search)
    async fn benchmark_mixed_workload(&self) -> Result<BenchmarkResults, Box<dyn std::error::Error>> {
        let concurrent_ops = self.config.concurrent_operations / 2;
        
        println!("Testing mixed workload: {} inserters + {} searchers...", concurrent_ops, concurrent_ops);
        
        // Inserter tasks
        let insert_handles: Vec<_> = (0..concurrent_ops).map(|op_id| {
            let processor = self.processor.clone();
            tokio::spawn(async move {
                let mut latencies = Vec::new();
                let mut successful = 0;
                
                for batch_id in 0..5 {  // 5 batches per inserter
                    let batch_records = (0..100).map(|i| {
                        Self::create_test_record_static(op_id * 500 + batch_id * 100 + i, Layer::Interact)
                    }).collect();
                    
                    let start = Instant::now();
                    match timeout(
                        Duration::from_secs(1),
                        processor.insert_batch(batch_records)
                    ).await {
                        Ok(Ok(_)) => {
                            let latency = start.elapsed().as_nanos() as f64 / 1_000_000.0;
                            latencies.push(latency);
                            successful += 1;
                        }
                        Ok(Err(e)) => {
                            eprintln!("Mixed insert failed in op {}: {}", op_id, e);
                        }
                        Err(_) => {
                            eprintln!("Mixed insert timeout in op {}", op_id);
                        }
                    }
                    
                    sleep(Duration::from_millis(10)).await;
                }
                
                (latencies, successful)
            })
        }).collect();
        
        // Searcher tasks
        let search_handles: Vec<_> = (0..concurrent_ops).map(|op_id| {
            let processor = self.processor.clone();
            tokio::spawn(async move {
                let mut latencies = Vec::new();
                let mut successful = 0;
                
                // Wait a bit for some data to be inserted
                sleep(Duration::from_millis(100)).await;
                
                for i in 0..20 {  // 20 searches per searcher
                    let query = Self::create_test_vector(op_id * 20 + i, 1024);
                    let start = Instant::now();
                    
                    match timeout(
                        Duration::from_millis(50),
                        processor.search(query, 5, Some(Layer::Interact))
                    ).await {
                        Ok(Ok(_)) => {
                            let latency = start.elapsed().as_nanos() as f64 / 1_000_000.0;
                            latencies.push(latency);
                            successful += 1;
                        }
                        Ok(Err(_)) => {
                            // Silent fail for searches (may not find results initially)
                        }
                        Err(_) => {
                            eprintln!("Mixed search timeout in op {}", op_id);
                        }
                    }
                    
                    sleep(Duration::from_millis(5)).await;
                }
                
                (latencies, successful)
            })
        }).collect();
        
        let overall_start = Instant::now();
        
        // Collect results
        let mut all_latencies = Vec::new();
        let mut total_successful = 0;
        let total_operations = concurrent_ops * 5 + concurrent_ops * 20;  // 5 inserts + 20 searches per worker
        
        for handle in insert_handles.into_iter().chain(search_handles.into_iter()) {
            if let Ok((latencies, successful)) = handle.await {
                all_latencies.extend(latencies);
                total_successful += successful;
            }
        }
        
        let total_time = overall_start.elapsed();
        let stats = self.processor.get_stats();
        
        Ok(self.calculate_results(all_latencies, total_successful, total_operations, total_time, &stats))
    }
    
    /// Benchmark different batch sizes для optimization
    async fn benchmark_batch_sizes(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Testing optimal batch sizes: {:?}", self.config.batch_sizes);
        
        for &batch_size in &self.config.batch_sizes {
            println!("\n--- Testing batch size: {} ---", batch_size);
            
            let test_batches = 50;
            let mut latencies = Vec::new();
            
            let start = Instant::now();
            
            for i in 0..test_batches {
                let batch_records = (0..batch_size).map(|j| {
                    self.create_test_record(i * batch_size + j, Layer::Interact)
                }).collect();
                
                let batch_start = Instant::now();
                match self.processor.insert_batch(batch_records).await {
                    Ok(_) => {
                        let latency = batch_start.elapsed().as_nanos() as f64 / 1_000_000.0;
                        latencies.push(latency);
                    }
                    Err(e) => {
                        eprintln!("Batch size {} failed: {}", batch_size, e);
                    }
                }
            }
            
            let total_time = start.elapsed();
            let total_vectors = test_batches * batch_size;
            let throughput = total_vectors as f64 / total_time.as_secs_f64();
            
            if !latencies.is_empty() {
                let avg_latency = latencies.iter().sum::<f64>() / latencies.len() as f64;
                latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let p95_latency = latencies[(latencies.len() as f64 * 0.95) as usize];
                
                println!("  Throughput: {:.0} vectors/sec", throughput);
                println!("  Avg Latency: {:.2}ms", avg_latency);
                println!("  P95 Latency: {:.2}ms", p95_latency);
                
                if avg_latency <= 5.0 && throughput >= 1000.0 {
                    println!("  ✅ Meets SLA requirements");
                } else {
                    println!("  ⚠️  Does not meet SLA requirements");
                }
            }
        }
        
        Ok(())
    }
    
    /// Benchmark cache performance
    async fn benchmark_cache_performance(&self) -> Result<BenchmarkResults, Box<dyn std::error::Error>> {
        println!("Testing cache performance with repeated searches...");
        
        // Insert initial data
        let setup_records = (0..500).map(|i| self.create_test_record(i, Layer::Interact)).collect();
        self.processor.insert_batch(setup_records).await?;
        
        let search_count = 1000;
        let query_pool_size = 50;  // Small pool для cache hits
        let mut latencies = Vec::new();
        let mut successful = 0;
        
        let overall_start = Instant::now();
        
        for i in 0..search_count {
            let query_id = i % query_pool_size;  // Reuse queries для cache hits
            let query = Self::create_test_vector(query_id, 1024);
            
            let start = Instant::now();
            match self.processor.search(query, 5, Some(Layer::Interact)).await {
                Ok(_) => {
                    let latency = start.elapsed().as_nanos() as f64 / 1_000_000.0;
                    latencies.push(latency);
                    successful += 1;
                }
                Err(e) => {
                    eprintln!("Cache search failed: {}", e);
                }
            }
        }
        
        let total_time = overall_start.elapsed();
        let stats = self.processor.get_stats();
        
        Ok(self.calculate_results(latencies, successful, search_count, total_time, &stats))
    }
    
    /// Calculate benchmark results
    fn calculate_results(
        &self,
        mut latencies: Vec<f64>,
        successful_operations: usize,
        total_operations: usize,
        total_time: Duration,
        stats: &BatchOptimizedStats,
    ) -> BenchmarkResults {
        if latencies.is_empty() {
            return BenchmarkResults {
                throughput_qps: 0.0,
                avg_latency_ms: 0.0,
                p95_latency_ms: 0.0,
                p99_latency_ms: 0.0,
                cache_hit_rate: 0.0,
                simd_operations: 0,
                success_rate: 0.0,
            };
        }
        
        latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let throughput_qps = successful_operations as f64 / total_time.as_secs_f64();
        let avg_latency_ms = latencies.iter().sum::<f64>() / latencies.len() as f64;
        let p95_idx = ((latencies.len() as f64) * 0.95) as usize;
        let p99_idx = ((latencies.len() as f64) * 0.99) as usize;
        let p95_latency_ms = latencies[p95_idx.min(latencies.len() - 1)];
        let p99_latency_ms = latencies[p99_idx.min(latencies.len() - 1)];
        let cache_hit_rate = stats.cache_hit_rate();
        let simd_operations = stats.simd_operations.load(std::sync::atomic::Ordering::Relaxed);
        let success_rate = successful_operations as f64 / total_operations as f64;
        
        BenchmarkResults {
            throughput_qps,
            avg_latency_ms,
            p95_latency_ms,
            p99_latency_ms,
            cache_hit_rate,
            simd_operations,
            success_rate,
        }
    }
    
    /// Print benchmark results
    fn print_results(&self, test_name: &str, results: &BenchmarkResults) {
        println!("--- {} Results ---", test_name);
        println!("  Throughput: {:.1} QPS", results.throughput_qps);
        println!("  Avg Latency: {:.2}ms", results.avg_latency_ms);
        println!("  P95 Latency: {:.2}ms", results.p95_latency_ms);
        println!("  P99 Latency: {:.2}ms", results.p99_latency_ms);
        println!("  Cache Hit Rate: {:.1}%", results.cache_hit_rate * 100.0);
        println!("  SIMD Operations: {}", results.simd_operations);
        println!("  Success Rate: {:.1}%", results.success_rate * 100.0);
        
        if results.meets_sla() {
            println!("  🎯 ✅ MEETS SLA REQUIREMENTS");
        } else {
            println!("  🎯 ❌ Does not meet SLA requirements");
        }
    }
    
    /// Print final processor statistics
    fn print_processor_stats(&self) {
        println!("\n📈 === FINAL PROCESSOR STATISTICS ===");
        let stats = self.processor.get_stats();
        
        println!("Total Batches: {}", stats.total_batches_processed.load(std::sync::atomic::Ordering::Relaxed));
        println!("Total Vectors: {}", stats.total_vectors_processed.load(std::sync::atomic::Ordering::Relaxed));
        println!("Overall Throughput: {:.1} QPS", stats.throughput_qps());
        println!("Overall Avg Latency: {:.2}ms", stats.avg_latency_ms());
        println!("Cache Hit Rate: {:.1}%", stats.cache_hit_rate() * 100.0);
        println!("SIMD Operations: {}", stats.simd_operations.load(std::sync::atomic::Ordering::Relaxed));
        println!("Lock Contentions: {}", stats.lock_contentions.load(std::sync::atomic::Ordering::Relaxed));
        println!("Adaptive Adjustments: {}", stats.adaptive_batch_adjustments.load(std::sync::atomic::Ordering::Relaxed));
    }
    
    /// Create test record
    fn create_test_record(&self, id: usize, layer: Layer) -> Record {
        Record {
            id: Uuid::new_v4(),
            text: format!("Test record {}", id),
            embedding: Self::create_test_vector(id, self.config.vector_dimension),
            layer,
            kind: "benchmark".to_string(),
            tags: vec![],
            project: "benchmark".to_string(),
            session: "benchmark".to_string(),
            score: 0.8,
            ts: chrono::Utc::now(),
            last_access: chrono::Utc::now(),
            access_count: 0,
        }
    }
    
    /// Static method for creating test records in async contexts
    fn create_test_record_static(id: usize, layer: Layer) -> Record {
        Record {
            id: Uuid::new_v4(),
            text: format!("Test record {}", id),
            embedding: Self::create_test_vector(id, 1024),
            layer,
            kind: "benchmark".to_string(),
            tags: vec![],
            project: "benchmark".to_string(),
            session: "benchmark".to_string(),
            score: 0.8,
            ts: chrono::Utc::now(),
            last_access: chrono::Utc::now(),
            access_count: 0,
        }
    }
    
    /// Create test vector with deterministic values
    fn create_test_vector(seed: usize, dimension: usize) -> Vec<f32> {
        let mut vector = Vec::with_capacity(dimension);
        for i in 0..dimension {
            let value = ((seed + i) as f32 * 0.001) % 1.0;
            vector.push(value);
        }
        vector
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("🚀 Ultra Batch Operations Benchmark");
    println!("Target: 1000+ QPS with sub-5ms latency");
    
    let config = BenchmarkConfig {
        concurrent_operations: 200,  // High concurrency test
        test_vector_count: 20000,    // Large dataset
        ..Default::default()
    };
    
    let benchmark = UltraBatchBenchmark::new(config).await?;
    benchmark.run_comprehensive_benchmark().await?;
    
    println!("\n🏁 Benchmark complete!");
    
    Ok(())
}