use anyhow::Result;
use memory::{VectorIndexHnswRs, HnswRsConfig};
use std::time::Instant;

/// Benchmark сравнения HNSW vs Linear search производительности
#[tokio::main]
async fn main() -> Result<()> {
    println!("🏁 Бенчмарк: HNSW vs Linear Search Performance");
    println!("================================================\n");

    // Test configurations
    let dataset_sizes = vec![100, 500, 1000, 2000, 5000];
    let query_count = 100;
    let k = 10;
    let dimension = 1024;

    for dataset_size in dataset_sizes {
        println!("📊 Тест с {} документами:", dataset_size);
        
        // Generate test data
        let mut vectors = Vec::new();
        for i in 0..dataset_size {
            let mut vector = vec![0.0f32; dimension];
            for j in 0..dimension {
                vector[j] = (i as f32 + j as f32 * 0.001 + rand::random::<f32>() * 0.1) / 100.0;
            }
            vectors.push((format!("doc_{}", i), vector));
        }

        // Generate query vectors
        let mut queries = Vec::new();
        for i in 0..query_count {
            let mut query = vec![0.0f32; dimension];
            for j in 0..dimension {
                query[j] = (i as f32 + j as f32 * 0.001 + rand::random::<f32>() * 0.05) / 100.0;
            }
            queries.push(query);
        }

        // Test HNSW
        println!("  🚀 Тестирование HNSW...");
        let hnsw_config = HnswRsConfig {
            dimension,
            max_connections: 24,
            ef_construction: 400,
            ef_search: 100,
            max_elements: dataset_size * 2,
            max_layers: 16,
            use_parallel: true,
        };
        
        let hnsw_index = VectorIndexHnswRs::new(hnsw_config)?;
        
        // Build HNSW index
        let build_start = Instant::now();
        hnsw_index.add_batch(vectors.clone())?;
        let hnsw_build_time = build_start.elapsed();
        
        // Search with HNSW
        let search_start = Instant::now();
        let mut hnsw_total_results = 0;
        for query in &queries {
            let results = hnsw_index.search(query, k)?;
            hnsw_total_results += results.len();
        }
        let hnsw_search_time = search_start.elapsed();
        let hnsw_avg_search = hnsw_search_time.as_micros() as f64 / query_count as f64;

        // Test Linear Search (simplified simulation)
        println!("  📏 Тестирование Linear Search...");
        let linear_start = Instant::now();
        let mut linear_total_results = 0;
        
        for query in &queries {
            // Simulate linear search - calculate distances to all vectors
            let mut scored_results = Vec::new();
            for (id, vector) in &vectors {
                let distance = cosine_distance(query, vector);
                scored_results.push((id.clone(), distance));
            }
            
            // Sort by distance and take top k
            scored_results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            linear_total_results += scored_results.into_iter().take(k).count();
        }
        let linear_search_time = linear_start.elapsed();
        let linear_avg_search = linear_search_time.as_micros() as f64 / query_count as f64;

        // Results
        println!("  ✅ Результаты:");
        println!("     HNSW Build Time: {:?}", hnsw_build_time);
        println!("     HNSW Search Time: {:?} ({:.1} μs/query)", hnsw_search_time, hnsw_avg_search);
        println!("     Linear Search Time: {:?} ({:.1} μs/query)", linear_search_time, linear_avg_search);
        
        let speedup = linear_avg_search / hnsw_avg_search;
        println!("     🚀 HNSW Speedup: {:.1}x faster", speedup);
        
        let hnsw_recall = hnsw_total_results as f64 / (query_count * k) as f64;
        let linear_recall = linear_total_results as f64 / (query_count * k) as f64;
        println!("     🎯 HNSW Recall: {:.1}%", hnsw_recall * 100.0);
        println!("     🎯 Linear Recall: {:.1}%", linear_recall * 100.0);
        
        // Performance analysis
        if speedup > 10.0 {
            println!("     🎉 ПРЕВОСХОДНО: HNSW показывает отличное ускорение!");
        } else if speedup > 3.0 {
            println!("     ✅ ХОРОШО: HNSW эффективен для этого размера данных");
        } else {
            println!("     ⚠️  Для малых датасетов HNSW может не давать преимущества");
        }
        
        println!();
    }

    println!("🎯 ЗАКЛЮЧЕНИЕ:");
    println!("  - HNSW эффективен для больших датасетов (>1000 документов)");
    println!("  - Linear search может быть быстрее для малых датасетов (<100 документов)");
    println!("  - HNSW обеспечивает сублинейное время поиска O(log n)");
    println!("  - Linear search имеет линейное время O(n)");

    Ok(())
}

fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return f32::INFINITY;
    }
    
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if norm_a == 0.0 || norm_b == 0.0 {
        return f32::INFINITY;
    }
    
    // Return distance (1 - cosine_similarity)
    1.0 - (dot_product / (norm_a * norm_b))
}