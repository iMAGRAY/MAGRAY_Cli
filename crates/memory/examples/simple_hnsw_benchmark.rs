//! Simple HNSW Benchmark - Direct Performance Testing
//!
//! Минимальный benchmark для измерения baseline производительности HNSW индекса

use std::time::{Duration, Instant};
use rand::{thread_rng, Rng};

mod hnsw_simple {
    use hnsw_rs::hnsw::*;
    use hnsw_rs::prelude::*;
    use std::sync::Arc;
    use parking_lot::RwLock;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Instant;

    /// Простой HNSW wrapper для benchmarking
    pub struct SimpleHnsw {
        hnsw: Arc<RwLock<Option<Hnsw<'static, f32, DistCosine>>>>,
        id_to_point: Arc<RwLock<HashMap<String, usize>>>,
        point_to_id: Arc<RwLock<HashMap<usize, String>>>,
        next_point_id: AtomicU64,
        dimension: usize,
    }

    impl SimpleHnsw {
        pub fn new(dimension: usize, max_elements: usize) -> anyhow::Result<Self> {
            Ok(Self {
                hnsw: Arc::new(RwLock::new(None)),
                id_to_point: Arc::new(RwLock::new(HashMap::new())),
                point_to_id: Arc::new(RwLock::new(HashMap::new())),
                next_point_id: AtomicU64::new(0),
                dimension,
            })
        }

        fn ensure_initialized(&self, expected_size: usize) -> anyhow::Result<()> {
            let mut hnsw_guard = self.hnsw.write();
            
            if hnsw_guard.is_none() {
                let hnsw_instance: Hnsw<'static, f32, DistCosine> = Hnsw::new(
                    16,         // M - максимальные соединения
                    expected_size, // max_nb_connection
                    16,         // max_layer
                    200,        // ef_construction
                    DistCosine {},
                );
                *hnsw_guard = Some(hnsw_instance);
            }
            
            Ok(())
        }

        pub fn add(&self, id: String, vector: Vec<f32>) -> anyhow::Result<()> {
            if vector.len() != self.dimension {
                return Err(anyhow::anyhow!("Vector dimension mismatch"));
            }

            self.ensure_initialized(1000)?;
            
            let point_id = self.next_point_id.fetch_add(1, Ordering::Relaxed) as usize;
            
            // Add to HNSW
            {
                let mut hnsw_guard = self.hnsw.write();
                if let Some(ref mut hnsw) = hnsw_guard.as_mut() {
                    hnsw.insert_data(&vector, point_id);
                }
            }

            // Update mappings
            {
                let mut id_to_point = self.id_to_point.write();
                let mut point_to_id = self.point_to_id.write();
                
                id_to_point.insert(id.clone(), point_id);
                point_to_id.insert(point_id, id);
            }

            Ok(())
        }

        pub fn search(&self, query: &[f32], k: usize) -> anyhow::Result<Vec<(String, f32)>> {
            if query.len() != self.dimension {
                return Err(anyhow::anyhow!("Query dimension mismatch"));
            }

            let results = {
                let hnsw_guard = self.hnsw.read();
                if let Some(ref hnsw) = hnsw_guard.as_ref() {
                    hnsw.search(query, k, 100) // ef_search = 100
                } else {
                    return Err(anyhow::anyhow!("HNSW not initialized"));
                }
            };

            let mut string_results = Vec::new();
            let point_to_id = self.point_to_id.read();
            
            for neighbor in results.iter() {
                if let Some(string_id) = point_to_id.get(&neighbor.d_id) {
                    string_results.push((string_id.clone(), neighbor.distance));
                }
            }

            Ok(string_results)
        }

        pub fn len(&self) -> usize {
            self.id_to_point.read().len()
        }
    }
}

const VECTOR_DIM: usize = 1024;
const NUM_VECTORS: usize = 1000; // Smaller set для быстрого тестирования
const NUM_QUERIES: usize = 50;

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

#[cfg(target_arch = "x86_64")]
fn benchmark_simd() -> (f64, f64) {
    use std::arch::x86_64::*;
    
    let vectors = generate_vectors(100, VECTOR_DIM);
    let query = generate_vectors(1, VECTOR_DIM)[0].clone();
    
    // Scalar benchmark
    let scalar_start = Instant::now();
    for vector in &vectors {
        let mut dot = 0.0;
        let mut norm_a = 0.0;
        let mut norm_b = 0.0;
        
        for i in 0..VECTOR_DIM {
            dot += query[i] * vector[i];
            norm_a += query[i] * query[i];
            norm_b += vector[i] * vector[i];
        }
        
        let _distance = 1.0 - (dot / (norm_a.sqrt() * norm_b.sqrt()));
    }
    let scalar_time = scalar_start.elapsed().as_nanos() as f64;
    
    // SIMD benchmark (если доступно)
    let simd_start = Instant::now();
    if std::arch::is_x86_feature_detected!("avx2") {
        for vector in &vectors {
            unsafe {
                let mut dot_product = _mm256_setzero_ps();
                let mut norm_a = _mm256_setzero_ps();
                let mut norm_b = _mm256_setzero_ps();
                
                let chunks = VECTOR_DIM / 8;
                for i in 0..chunks {
                    let idx = i * 8;
                    let va = _mm256_loadu_ps(query.as_ptr().add(idx));
                    let vb = _mm256_loadu_ps(vector.as_ptr().add(idx));
                    
                    dot_product = _mm256_add_ps(dot_product, _mm256_mul_ps(va, vb));
                    norm_a = _mm256_add_ps(norm_a, _mm256_mul_ps(va, va));
                    norm_b = _mm256_add_ps(norm_b, _mm256_mul_ps(vb, vb));
                }
                
                // Horizontal sum
                let dot_sum = {
                    let hi = _mm256_extractf128_ps(dot_product, 1);
                    let lo = _mm256_castps256_ps128(dot_product);
                    let sum128 = _mm_add_ps(hi, lo);
                    
                    let hi64 = _mm_movehl_ps(sum128, sum128);
                    let sum64 = _mm_add_ps(sum128, hi64);
                    
                    let hi32 = _mm_shuffle_ps(sum64, sum64, 0x01);
                    let sum32 = _mm_add_ss(sum64, hi32);
                    
                    _mm_cvtss_f32(sum32)
                };
                
                let norm_a_sum = {
                    let hi = _mm256_extractf128_ps(norm_a, 1);
                    let lo = _mm256_castps256_ps128(norm_a);
                    let sum128 = _mm_add_ps(hi, lo);
                    
                    let hi64 = _mm_movehl_ps(sum128, sum128);
                    let sum64 = _mm_add_ps(sum128, hi64);
                    
                    let hi32 = _mm_shuffle_ps(sum64, sum64, 0x01);
                    let sum32 = _mm_add_ss(sum64, hi32);
                    
                    _mm_cvtss_f32(sum32)
                };
                
                let norm_b_sum = {
                    let hi = _mm256_extractf128_ps(norm_b, 1);
                    let lo = _mm256_castps256_ps128(norm_b);
                    let sum128 = _mm_add_ps(hi, lo);
                    
                    let hi64 = _mm_movehl_ps(sum128, sum128);
                    let sum64 = _mm_add_ps(sum128, hi64);
                    
                    let hi32 = _mm_shuffle_ps(sum64, sum64, 0x01);
                    let sum32 = _mm_add_ss(sum64, hi32);
                    
                    _mm_cvtss_f32(sum32)
                };
                
                let similarity = dot_sum / (norm_a_sum.sqrt() * norm_b_sum.sqrt());
                let _distance = 1.0 - similarity;
            }
        }
    } else {
        // Fallback to scalar if AVX2 not available
        for vector in &vectors {
            let mut dot = 0.0;
            let mut norm_a = 0.0;
            let mut norm_b = 0.0;
            
            for i in 0..VECTOR_DIM {
                dot += query[i] * vector[i];
                norm_a += query[i] * query[i];
                norm_b += vector[i] * vector[i];
            }
            
            let _distance = 1.0 - (dot / (norm_a.sqrt() * norm_b.sqrt()));
        }
    }
    let simd_time = simd_start.elapsed().as_nanos() as f64;
    
    (scalar_time / 1_000_000.0, simd_time / 1_000_000.0) // Convert to milliseconds
}

#[cfg(not(target_arch = "x86_64"))]
fn benchmark_simd() -> (f64, f64) {
    (0.0, 0.0) // No SIMD available
}

fn main() -> anyhow::Result<()> {
    println!("🚀 Simple HNSW Performance Benchmark");
    println!("====================================");
    
    // Generate test data
    println!("📊 Generating test data...");
    let vectors = generate_vectors(NUM_VECTORS, VECTOR_DIM);
    let queries = generate_vectors(NUM_QUERIES, VECTOR_DIM);
    println!("Generated {} vectors and {} queries", NUM_VECTORS, NUM_QUERIES);
    
    // Create HNSW index
    println!("🏗️ Building HNSW index...");
    let index = hnsw_simple::SimpleHnsw::new(VECTOR_DIM, NUM_VECTORS * 2)?;
    
    let build_start = Instant::now();
    for (i, vector) in vectors.iter().enumerate() {
        index.add(format!("vec_{}", i), vector.clone())?;
    }
    let build_time = build_start.elapsed();
    
    println!("✅ Index built in {:?} ({} vectors)", build_time, index.len());
    
    // Benchmark single queries
    println!("⚡ Benchmarking single queries...");
    let mut query_times = Vec::new();
    
    for query in &queries {
        let start = Instant::now();
        let _results = index.search(query, 10)?;
        let latency = start.elapsed();
        query_times.push(latency.as_nanos() as f64 / 1_000_000.0); // Convert to milliseconds
    }
    
    query_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let avg_latency = query_times.iter().sum::<f64>() / query_times.len() as f64;
    let p95_latency = query_times[(query_times.len() as f64 * 0.95) as usize];
    let p99_latency = query_times[(query_times.len() as f64 * 0.99) as usize];
    
    // SIMD benchmark
    println!("⚙️ Benchmarking SIMD performance...");
    let (scalar_time, simd_time) = benchmark_simd();
    let simd_speedup = if simd_time > 0.0 { scalar_time / simd_time } else { 1.0 };
    
    // Print results
    println!("\n🔥 PERFORMANCE RESULTS 🔥");
    println!("========================");
    
    println!("📊 Index Building:");
    println!("  Build Time:     {:?}", build_time);
    println!("  Vectors Added:  {}", index.len());
    
    println!("⚡ Single Query Performance:");
    println!("  Average:        {:.3}ms", avg_latency);
    println!("  P95:            {:.3}ms", p95_latency);
    println!("  P99:            {:.3}ms", p99_latency);
    
    if avg_latency <= 5.0 {
        println!("  Status:         ✅ SUB-5MS TARGET ACHIEVED!");
    } else {
        println!("  Status:         ❌ EXCEEDS 5ms TARGET by {:.1}ms", avg_latency - 5.0);
    }
    
    println!("⚙️ SIMD Performance:");
    println!("  Scalar Time:    {:.3}ms (per 100 vectors)", scalar_time);
    println!("  SIMD Time:      {:.3}ms (per 100 vectors)", simd_time);
    println!("  Speedup:        {:.1}x", simd_speedup);
    
    if simd_speedup >= 2.0 {
        println!("  Status:         ✅ EXCELLENT SIMD PERFORMANCE");
    } else if simd_speedup >= 1.5 {
        println!("  Status:         ⚠️ GOOD SIMD PERFORMANCE");
    } else {
        println!("  Status:         ❌ POOR SIMD - OPTIMIZATION NEEDED");
    }
    
    // Optimization recommendations
    println!("\n💡 OPTIMIZATION OPPORTUNITIES:");
    if avg_latency > 5.0 {
        println!("  🔧 Query Latency: Reduce ef_search, optimize distance calculations");
    }
    if simd_speedup < 2.0 {
        println!("  🔧 SIMD: Implement better horizontal sum, add prefetching");
    }
    if build_time.as_millis() > 1000 {
        println!("  🔧 Build Speed: Parallel insertion, batch optimization");
    }
    
    println!("\n🎯 Ready for advanced SIMD/GPU optimization!");
    
    Ok(())
}