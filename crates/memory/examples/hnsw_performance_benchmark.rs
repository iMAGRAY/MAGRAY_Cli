//! Comprehensive HNSW Performance Benchmark - BASELINE MEASUREMENTS
//!
//! Измеряет текущую производительность HNSW индекса для установления baseline метрик:
//! - Single query latency (target: <5ms)  
//! - Batch query latency (target: <15ms for 10 queries)
//! - Memory usage (target: <2GB for 100K vectors)
//! - QPS throughput (target: 1000+ QPS)
//! - Index building time (target: <30s for 100K vectors)
//!
//! Использует реалистичные 1024D embeddings и выявляет узкие места

use memory::{
    hnsw_index::{VectorIndex, HnswConfig},
    simd_optimized::{cosine_distance_auto, cosine_distance_scalar},
    batch_optimized::{BatchOptimizedProcessor, BatchOptimizedConfig},
};
use std::time::{Duration, Instant};
use std::sync::Arc;
use rand::{thread_rng, Rng};
use tokio::runtime::Runtime;
use tracing::{info, warn, Level};

/// Test vector dimension - реалистичный размер embeddings
const VECTOR_DIM: usize = 1024;
/// Количество векторов для baseline testing
const NUM_VECTORS: usize = 10_000;  // Начнем с 10K для baseline
/// Количество test queries
const NUM_QUERIES: usize = 100;
/// Batch size для batch testing
const BATCH_SIZE: usize = 10;
/// Target latency для single query (ms)
const TARGET_SINGLE_LATENCY_MS: f64 = 5.0;
/// Target latency для batch queries (ms) 
const TARGET_BATCH_LATENCY_MS: f64 = 15.0;

/// Performance metrics structure
#[derive(Debug)]
struct PerformanceMetrics {
    // Index building metrics
    index_build_time_ms: f64,
    memory_usage_mb: f64,
    
    // Single query metrics
    single_query_avg_ms: f64,
    single_query_p95_ms: f64,
    single_query_p99_ms: f64,
    
    // Batch query metrics  
    batch_query_avg_ms: f64,
    batch_throughput_qps: f64,
    
    // SIMD performance comparison
    simd_speedup_factor: f64,
    
    // Quality metrics
    recall_at_10: f64,
    accuracy_score: f64,
}

impl PerformanceMetrics {
    fn print_summary(&self) {
        println!("\n🔥 HNSW PERFORMANCE BENCHMARK RESULTS 🔥");
        println!("=========================================");
        
        // Index building results
        println!("📊 INDEX BUILDING:");
        println!("  Build Time:    {:.2}ms", self.index_build_time_ms);
        println!("  Memory Usage:  {:.1}MB", self.memory_usage_mb);
        
        // Single query results
        println!("⚡ SINGLE QUERY LATENCY:");
        println!("  Average:       {:.3}ms (target: <{:.1}ms)", self.single_query_avg_ms, TARGET_SINGLE_LATENCY_MS);
        println!("  P95:           {:.3}ms", self.single_query_p95_ms);
        println!("  P99:           {:.3}ms", self.single_query_p99_ms);
        if self.single_query_avg_ms <= TARGET_SINGLE_LATENCY_MS {
            println!("  Status:        ✅ TARGET ACHIEVED");
        } else {
            println!("  Status:        ❌ EXCEEDS TARGET by {:.1}ms", self.single_query_avg_ms - TARGET_SINGLE_LATENCY_MS);
        }
        
        // Batch query results
        println!("🚀 BATCH QUERY PERFORMANCE:");
        println!("  Batch Latency: {:.3}ms (target: <{:.1}ms)", self.batch_query_avg_ms, TARGET_BATCH_LATENCY_MS);
        println!("  Throughput:    {:.1} QPS", self.batch_throughput_qps);
        if self.batch_query_avg_ms <= TARGET_BATCH_LATENCY_MS {
            println!("  Status:        ✅ TARGET ACHIEVED");
        } else {
            println!("  Status:        ❌ EXCEEDS TARGET by {:.1}ms", self.batch_query_avg_ms - TARGET_BATCH_LATENCY_MS);
        }
        
        // SIMD performance
        println!("⚙️ SIMD OPTIMIZATION:");
        println!("  SIMD Speedup:  {:.1}x", self.simd_speedup_factor);
        if self.simd_speedup_factor >= 2.0 {
            println!("  Status:        ✅ EXCELLENT SIMD PERFORMANCE");
        } else if self.simd_speedup_factor >= 1.5 {
            println!("  Status:        ⚠️ GOOD SIMD PERFORMANCE");
        } else {
            println!("  Status:        ❌ POOR SIMD PERFORMANCE - OPTIMIZATION NEEDED");
        }
        
        // Quality metrics
        println!("🎯 QUALITY METRICS:");
        println!("  Recall@10:     {:.3}", self.recall_at_10);
        println!("  Accuracy:      {:.3}", self.accuracy_score);
        
        // Overall assessment
        println!("📈 OVERALL ASSESSMENT:");
        let targets_met = (self.single_query_avg_ms <= TARGET_SINGLE_LATENCY_MS) as i32 + 
                         (self.batch_query_avg_ms <= TARGET_BATCH_LATENCY_MS) as i32 +
                         (self.simd_speedup_factor >= 2.0) as i32;
        match targets_met {
            3 => println!("  Status:        🏆 ALL TARGETS MET - EXCELLENT PERFORMANCE"),
            2 => println!("  Status:        ✅ MOST TARGETS MET - GOOD PERFORMANCE"),  
            1 => println!("  Status:        ⚠️ SOME TARGETS MET - NEEDS OPTIMIZATION"),
            _ => println!("  Status:        ❌ OPTIMIZATION REQUIRED - PERFORMANCE BELOW TARGETS"),
        }
        
        println!("=========================================\n");
    }
}

/// Generate realistic embedding vectors
fn generate_vectors(count: usize, dim: usize) -> Vec<Vec<f32>> {
    let mut rng = thread_rng();
    (0..count)
        .map(|_| {
            (0..dim)
                .map(|_| rng.gen_range(-1.0..1.0))
                .collect()
        })
        .collect()
}

/// Measure memory usage (approximation)
fn estimate_memory_usage(index: &VectorIndex) -> f64 {
    // Простая оценка based on количества векторов и размерности
    let num_vectors = index.len() as f64;
    let vector_size = VECTOR_DIM as f64 * 4.0; // f32 = 4 bytes
    let overhead_factor = 2.5; // HNSW overhead factor
    
    (num_vectors * vector_size * overhead_factor) / 1_000_000.0 // Convert to MB
}

/// Benchmark single query performance
fn benchmark_single_queries(index: &VectorIndex, queries: &[Vec<f32>], k: usize) -> (f64, f64, f64) {
    let mut latencies = Vec::with_capacity(queries.len());
    
    for query in queries {
        let start = Instant::now();
        let _results = index.search(query, k).expect("Search failed");
        let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
        latencies.push(latency_ms);
    }
    
    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let avg = latencies.iter().sum::<f64>() / latencies.len() as f64;
    let p95_idx = (latencies.len() as f64 * 0.95) as usize;
    let p99_idx = (latencies.len() as f64 * 0.99) as usize;
    
    let p95 = latencies[p95_idx.min(latencies.len() - 1)];
    let p99 = latencies[p99_idx.min(latencies.len() - 1)];
    
    (avg, p95, p99)
}

/// Benchmark batch query performance  
fn benchmark_batch_queries(index: &VectorIndex, queries: &[Vec<f32>], k: usize) -> (f64, f64) {
    let batches: Vec<&[Vec<f32>]> = queries.chunks(BATCH_SIZE).collect();
    let mut batch_latencies = Vec::with_capacity(batches.len());
    let mut total_queries = 0;
    
    let overall_start = Instant::now();
    
    for batch in batches {
        let batch_start = Instant::now();
        let _results = index.parallel_search(batch, k).expect("Batch search failed");
        let batch_latency_ms = batch_start.elapsed().as_secs_f64() * 1000.0;
        batch_latencies.push(batch_latency_ms);
        total_queries += batch.len();
    }
    
    let total_time_s = overall_start.elapsed().as_secs_f64();
    let avg_batch_latency = batch_latencies.iter().sum::<f64>() / batch_latencies.len() as f64;
    let throughput_qps = total_queries as f64 / total_time_s;
    
    (avg_batch_latency, throughput_qps)
}

/// Benchmark SIMD performance vs scalar
fn benchmark_simd_performance(vectors: &[Vec<f32>], queries: &[Vec<f32>]) -> f64 {
    let sample_size = 1000.min(vectors.len());
    let query_sample_size = 10.min(queries.len());
    
    // Benchmark scalar performance
    let scalar_start = Instant::now();
    for query in queries.iter().take(query_sample_size) {
        for vector in vectors.iter().take(sample_size) {
            let _distance = cosine_distance_scalar(query, vector);
        }
    }
    let scalar_time = scalar_start.elapsed();
    
    // Benchmark SIMD performance  
    let simd_start = Instant::now();
    for query in queries.iter().take(query_sample_size) {
        for vector in vectors.iter().take(sample_size) {
            let _distance = cosine_distance_auto(query, vector);
        }
    }
    let simd_time = simd_start.elapsed();
    
    // Calculate speedup factor
    if simd_time.as_nanos() > 0 {
        scalar_time.as_nanos() as f64 / simd_time.as_nanos() as f64
    } else {
        1.0
    }
}

/// Calculate recall@k quality metric
fn calculate_recall_at_k(index: &VectorIndex, test_vectors: &[Vec<f32>], test_queries: &[Vec<f32>], k: usize) -> f64 {
    let mut total_recall = 0.0;
    let sample_size = 20.min(test_queries.len()); // Sample for speed
    
    for query in test_queries.iter().take(sample_size) {
        // Get HNSW results
        let hnsw_results = index.search(query, k).unwrap_or_default();
        
        // Calculate ground truth (brute force top-k) - expensive but accurate
        let mut ground_truth = Vec::new();
        for (idx, vector) in test_vectors.iter().enumerate() {
            let distance = cosine_distance_scalar(query, vector);
            ground_truth.push((idx, distance));
        }
        ground_truth.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        let gt_ids: std::collections::HashSet<_> = ground_truth.iter()
            .take(k)
            .map(|(idx, _)| idx.to_string())
            .collect();
        
        // Calculate recall
        let hnsw_ids: std::collections::HashSet<_> = hnsw_results.iter()
            .map(|(id, _)| id.clone())
            .collect();
        
        let intersection_size = gt_ids.intersection(&hnsw_ids).count();
        let recall = intersection_size as f64 / k as f64;
        total_recall += recall;
    }
    
    total_recall / sample_size as f64
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();
    
    info!("🚀 Starting comprehensive HNSW performance benchmark");
    info!("Configuration: {}D vectors, {} total vectors, {} queries", 
          VECTOR_DIM, NUM_VECTORS, NUM_QUERIES);
    
    // Generate test data
    info!("📊 Generating test vectors...");
    let start_gen = Instant::now();
    let vectors = generate_vectors(NUM_VECTORS, VECTOR_DIM);
    let queries = generate_vectors(NUM_QUERIES, VECTOR_DIM);
    let gen_time = start_gen.elapsed();
    info!("Generated {} vectors + {} queries in {:?}", NUM_VECTORS, NUM_QUERIES, gen_time);
    
    // Create optimized HNSW config
    let config = HnswConfig {
        dimension: VECTOR_DIM,
        max_elements: NUM_VECTORS * 2, // Leave room for growth
        max_connections: 16,
        ef_construction: 200,
        ef_search: 100,
        max_layers: 16,
        use_parallel: true,
    };
    
    // Create index
    info!("🏗️ Building HNSW index...");
    let index = VectorIndex::new(config)?;
    
    // Measure index building time
    let build_start = Instant::now();
    let batch_data: Vec<_> = vectors.into_iter()
        .enumerate()
        .map(|(i, vector)| (format!("vec_{}", i), vector))
        .collect();
    
    index.add_batch(batch_data)?;
    let build_time_ms = build_start.elapsed().as_secs_f64() * 1000.0;
    
    info!("✅ Index built in {:.2}ms", build_time_ms);
    
    // Measure memory usage
    let memory_usage_mb = estimate_memory_usage(&index);
    info!("📈 Estimated memory usage: {:.1}MB", memory_usage_mb);
    
    // Benchmark single queries
    info!("⚡ Benchmarking single queries...");
    let (single_avg, single_p95, single_p99) = benchmark_single_queries(&index, &queries, 10);
    
    // Benchmark batch queries
    info!("🚀 Benchmarking batch queries...");
    let (batch_avg, batch_qps) = benchmark_batch_queries(&index, &queries, 10);
    
    // Benchmark SIMD performance
    info!("⚙️ Benchmarking SIMD performance...");
    let test_vectors: Vec<_> = (0..100).map(|i| format!("test_{}", i)).collect();
    let test_vectors_data: Vec<_> = (0..100).map(|_| generate_vectors(1, VECTOR_DIM).into_iter().next().unwrap()).collect();
    let simd_speedup = benchmark_simd_performance(&test_vectors_data, &queries);
    
    // Calculate quality metrics  
    info!("🎯 Calculating quality metrics...");
    let recall_at_10 = calculate_recall_at_k(&index, &test_vectors_data, &queries, 10);
    let accuracy_score = recall_at_10; // Simplified accuracy metric
    
    // Compile results
    let metrics = PerformanceMetrics {
        index_build_time_ms: build_time_ms,
        memory_usage_mb,
        single_query_avg_ms: single_avg,
        single_query_p95_ms: single_p95,
        single_query_p99_ms: single_p99,
        batch_query_avg_ms: batch_avg,
        batch_throughput_qps: batch_qps,
        simd_speedup_factor: simd_speedup,
        recall_at_10,
        accuracy_score,
    };
    
    // Print comprehensive results
    metrics.print_summary();
    
    // Generate recommendations based on results
    println!("💡 OPTIMIZATION RECOMMENDATIONS:");
    
    if metrics.single_query_avg_ms > TARGET_SINGLE_LATENCY_MS {
        println!("  🔧 SINGLE QUERY OPTIMIZATION:");
        println!("    - Consider reducing ef_search parameter");
        println!("    - Implement aggressive SIMD optimization");
        println!("    - Add hot vector caching");
        println!("    - Optimize horizontal sum implementation");
    }
    
    if metrics.batch_query_avg_ms > TARGET_BATCH_LATENCY_MS {
        println!("  🔧 BATCH QUERY OPTIMIZATION:");
        println!("    - Implement lock-free batch processing");
        println!("    - Add cache-conscious memory layout");
        println!("    - Optimize parallel search orchestration");
        println!("    - Implement adaptive batching");
    }
    
    if metrics.simd_speedup_factor < 2.0 {
        println!("  🔧 SIMD OPTIMIZATION:");
        println!("    - Implement AVX-512 support");
        println!("    - Add memory alignment optimizations");
        println!("    - Optimize horizontal sum functions");
        println!("    - Add prefetching hints");
    }
    
    if metrics.batch_throughput_qps < 1000.0 {
        println!("  🔧 THROUGHPUT OPTIMIZATION:");
        println!("    - Implement GPU acceleration");
        println!("    - Add memory mapping for large indices");
        println!("    - Optimize worker thread coordination");
        println!("    - Add vector quantization");
    }
    
    println!("\n🎯 Ready for ultra-optimization phase!");
    
    Ok(())
}