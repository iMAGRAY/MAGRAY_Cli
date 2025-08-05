use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use memory::{
    VectorStore,
    VectorIndexHnswRs, HnswRsConfig,
    Layer, Record,
};
use uuid::Uuid;
use tokio::runtime::Runtime;

// @component: {"k":"T","id":"perf_benchmarks","t":"Performance benchmarks для memory system","m":{"cur":0,"tgt":100,"u":"%"},"f":["benchmarks","performance"]}

/// Генерация случайных векторов для тестов
fn generate_random_vectors(count: usize, dimension: usize) -> Vec<Vec<f32>> {
    (0..count)
        .map(|_| {
            (0..dimension)
                .map(|_| fastrand::f32() * 2.0 - 1.0) // [-1, 1]
                .collect()
        })
        .collect()
}

/// Создание тестовых записей
fn create_test_records(vectors: Vec<Vec<f32>>, layer: Layer) -> Vec<Record> {
    vectors
        .into_iter()
        .map(|embedding| Record {
            id: Uuid::new_v4(),
            text: "test document".to_string(),
            embedding,
            layer,
            kind: "benchmark".to_string(),
            tags: vec!["test".to_string(), "benchmark".to_string()],
            project: "benchmark".to_string(),
            session: "benchmark_session".to_string(),
            score: 0.0,
            ts: chrono::Utc::now(),
            access_count: 0,
            last_access: chrono::Utc::now(),
        })
        .collect()
}

/// Benchmark: HNSW индекс - добавление векторов
fn bench_hnsw_insert(c: &mut Criterion) {
    let config = HnswRsConfig {
        dimension: 1024,
        max_connections: 24,
        ef_construction: 400,
        ef_search: 100,
        max_elements: 100_000,
        max_layers: 16,
        use_parallel: true,
    };
    
    let mut group = c.benchmark_group("hnsw_insert");
    
    for size in [100, 1000, 5000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        
        group.bench_with_input(BenchmarkId::new("sequential", size), size, |b, &size| {
            b.iter_with_setup(
                || {
                    let index = VectorIndexHnswRs::new(config.clone()).unwrap();
                    let vectors = generate_random_vectors(size, 768);
                    (index, vectors)
                },
                |(index, vectors)| {
                    for (i, vector) in vectors.into_iter().enumerate() {
                        black_box(index.add(format!("doc_{}", i), vector).unwrap());
                    }
                }
            );
        });
        
        group.bench_with_input(BenchmarkId::new("batch", size), size, |b, &size| {
            b.iter_with_setup(
                || {
                    let index = VectorIndexHnswRs::new(config.clone()).unwrap();
                    let vectors = generate_random_vectors(size, 768);
                    let batch: Vec<_> = vectors.into_iter()
                        .enumerate()
                        .map(|(i, v)| (format!("doc_{}", i), v))
                        .collect();
                    (index, batch)
                },
                |(index, batch)| {
                    black_box(index.add_batch(batch).unwrap());
                }
            );
        });
    }
    
    group.finish();
}

/// Benchmark: HNSW индекс - поиск векторов
fn bench_hnsw_search(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = HnswRsConfig {
        dimension: 1024,
        max_connections: 24,
        ef_construction: 400,
        ef_search: 100,
        max_elements: 100_000,
        max_layers: 16,
        use_parallel: true,
    };
    
    // Подготавливаем индекс с данными разных размеров
    let sizes = [1000, 10000, 50000];
    let mut prepared_indices = Vec::new();
    
    for &size in &sizes {
        let index = VectorIndexHnswRs::new(config.clone()).unwrap();
        let vectors = generate_random_vectors(size, 768);
        let batch: Vec<_> = vectors.into_iter()
            .enumerate()
            .map(|(i, v)| (format!("doc_{}", i), v))
            .collect();
        index.add_batch(batch).unwrap();
        prepared_indices.push((size, index));
    }
    
    let mut group = c.benchmark_group("hnsw_search");
    
    for (size, index) in prepared_indices {
        for k in [1, 10, 50, 100].iter() {
            group.bench_with_input(
                BenchmarkId::new(format!("{}k_vectors", size/1000), k), 
                k, 
                |b, &k| {
                    let query = generate_random_vectors(1, 768)[0].clone();
                    b.iter(|| {
                        black_box(index.search(&query, k).unwrap());
                    });
                }
            );
        }
    }
    
    group.finish();
}

/// Benchmark: VectorStore - комплексные операции
fn bench_vector_store(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("vector_store");
    
    for size in [100, 1000, 5000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        
        // Benchmark insert
        group.bench_with_input(BenchmarkId::new("insert", size), size, |b, &size| {
            b.to_async(&rt).iter(|| async {
                let temp_dir = tempfile::TempDir::new().unwrap();
                let store = VectorStore::new(temp_dir.path()).await.unwrap();
                let vectors = generate_random_vectors(size, 1024);
                let records = create_test_records(vectors, Layer::Interact);
                
                for record in records {
                    black_box(store.insert(&record).await.unwrap());
                }
            });
        });
        
        // Benchmark batch insert
        group.bench_with_input(BenchmarkId::new("batch_insert", size), size, |b, &size| {
            b.to_async(&rt).iter(|| async {
                let temp_dir = tempfile::TempDir::new().unwrap();
                let store = VectorStore::new(temp_dir.path()).await.unwrap();
                let vectors = generate_random_vectors(size, 1024);
                let records = create_test_records(vectors, Layer::Interact);
                
                let refs: Vec<&Record> = records.iter().collect();
                black_box(store.insert_batch(&refs).await.unwrap());
            });
        });
        
        // Benchmark search (with pre-populated data)
        group.bench_with_input(BenchmarkId::new("search", size), size, |b, &size| {
            // Pre-populate store once per benchmark group
            let temp_dir = tempfile::TempDir::new().unwrap();
            let store = rt.block_on(async {
                let store = VectorStore::new(temp_dir.path()).await.unwrap();
                let vectors = generate_random_vectors(size, 1024);
                let records = create_test_records(vectors, Layer::Interact);
                let refs: Vec<&Record> = records.iter().collect();
                store.insert_batch(&refs).await.unwrap();
                store
            });
            
            b.to_async(&rt).iter(|| async {
                let query = generate_random_vectors(1, 1024)[0].clone();
                black_box(store.search(&query, Layer::Interact, 10).await.unwrap());
            });
        });
    }
    
    group.finish();
}

/// Benchmark: Memory scaling под нагрузкой
fn bench_memory_scaling(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("memory_scaling_stress", |b| {
        b.to_async(&rt).iter(|| async {
            let temp_dir = tempfile::TempDir::new().unwrap();
            let store = VectorStore::new(temp_dir.path()).await.unwrap();
            
            // Симулируем быстрый рост данных
            for batch_size in [100, 500, 1000, 2000].iter() {
                let vectors = generate_random_vectors(*batch_size, 1024);
                let records = create_test_records(vectors, Layer::Interact);
                let refs: Vec<&Record> = records.iter().collect();
                
                black_box(store.insert_batch(&refs).await.unwrap());
                
                // Проверяем время search после каждого batch
                let query = generate_random_vectors(1, 1024)[0].clone();
                black_box(store.search(&query, Layer::Interact, 10).await.unwrap());
            }
        });
    });
}

/// Benchmark: Параллельный поиск (эксклюзивная фича hnsw_rs)
fn bench_parallel_search(c: &mut Criterion) {
    let config = HnswRsConfig {
        dimension: 1024,
        max_connections: 24,
        ef_construction: 400,
        ef_search: 100,
        max_elements: 100_000,
        max_layers: 16,
        use_parallel: true,
    };
    
    // Подготавливаем большой индекс
    let index = VectorIndexHnswRs::new(config).unwrap();
    let vectors = generate_random_vectors(10000, 768);
    let batch: Vec<_> = vectors.into_iter()
        .enumerate()
        .map(|(i, v)| (format!("doc_{}", i), v))
        .collect();
    index.add_batch(batch).unwrap();
    
    let mut group = c.benchmark_group("parallel_search");
    
    for query_count in [1, 4, 8, 16, 32].iter() {
        group.throughput(Throughput::Elements(*query_count as u64));
        
        group.bench_with_input(
            BenchmarkId::new("queries", query_count), 
            query_count, 
            |b, &query_count| {
                let queries = generate_random_vectors(query_count, 768);
                b.iter(|| {
                    black_box(index.parallel_search(&queries, 10).unwrap());
                });
            }
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_hnsw_insert,
    bench_hnsw_search, 
    bench_vector_store,
    bench_memory_scaling,
    bench_parallel_search
);

criterion_main!(benches);