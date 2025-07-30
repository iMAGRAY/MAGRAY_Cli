use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use memory::{VectorIndexV3, VectorIndexConfigV3};
use std::time::Duration;

/// Генерирует случайный эмбеддинг заданной размерности
fn random_embedding(dim: usize, seed: f32) -> Vec<f32> {
    (0..dim)
        .map(|i| ((i as f32 + seed) * 0.1).sin())
        .collect()
}

/// Генерирует набор эмбеддингов
fn generate_embeddings(count: usize, dim: usize) -> Vec<(String, Vec<f32>)> {
    (0..count)
        .map(|i| {
            let id = format!("doc_{}", i);
            let embedding = random_embedding(dim, i as f32);
            (id, embedding)
        })
        .collect()
}

fn benchmark_vector_index_v3(c: &mut Criterion) {
    let dimension = 1024;
    
    // Конфигурация индекса
    let config = VectorIndexConfigV3 {
        dimension,
        rebuild_threshold: 100,
        linear_search_threshold: 1000,
        ..Default::default()
    };
    
    // Тест 1: Добавление отдельных векторов
    {
        let mut group = c.benchmark_group("VectorIndexV3/add_single");
        group.measurement_time(Duration::from_secs(10));
        
        for size in [100, 500, 1000, 5000].iter() {
            group.bench_with_input(
                BenchmarkId::from_parameter(size),
                size,
                |b, &size| {
                    let index = VectorIndexV3::new(config.clone());
                    let embeddings = generate_embeddings(size, dimension);
                    let mut i = 0;
                    
                    b.iter(|| {
                        let (id, embedding) = &embeddings[i % size];
                        index.add(id.clone(), embedding.clone()).unwrap();
                        i += 1;
                    });
                },
            );
        }
        
        group.finish();
    }
    
    // Тест 2: Пакетное добавление
    {
        let mut group = c.benchmark_group("VectorIndexV3/add_batch");
        group.measurement_time(Duration::from_secs(10));
        
        for batch_size in [10, 50, 100, 500].iter() {
            group.bench_with_input(
                BenchmarkId::from_parameter(batch_size),
                batch_size,
                |b, &batch_size| {
                    b.iter(|| {
                        let index = VectorIndexV3::new(config.clone());
                        let batch = generate_embeddings(batch_size, dimension);
                        index.add_batch(black_box(batch)).unwrap();
                    });
                },
            );
        }
        
        group.finish();
    }
    
    // Тест 3: Поиск в индексах разного размера
    {
        let mut group = c.benchmark_group("VectorIndexV3/search");
        group.measurement_time(Duration::from_secs(10));
        
        for size in [100, 500, 1000, 5000, 10000].iter() {
            let index = VectorIndexV3::new(config.clone());
            let embeddings = generate_embeddings(*size, dimension);
            
            // Добавляем все эмбеддинги пакетом
            index.add_batch(embeddings).unwrap();
            
            let query_embedding = random_embedding(dimension, 0.5);
            
            group.bench_with_input(
                BenchmarkId::from_parameter(size),
                size,
                |b, _| {
                    b.iter(|| {
                        let results = index.search(black_box(&query_embedding), 10).unwrap();
                        black_box(results);
                    });
                },
            );
        }
        
        group.finish();
    }
    
    // Тест 4: Сравнение линейного поиска и HNSW
    {
        let mut group = c.benchmark_group("VectorIndexV3/search_comparison");
        group.measurement_time(Duration::from_secs(10));
        
        // Маленький датасет (линейный поиск)
        {
            let small_index = VectorIndexV3::new(VectorIndexConfigV3 {
                dimension,
                linear_search_threshold: 10000, // Форсируем линейный поиск
                ..Default::default()
            });
            
            let embeddings = generate_embeddings(500, dimension);
            small_index.add_batch(embeddings).unwrap();
            
            let query = random_embedding(dimension, 0.5);
            
            group.bench_function("linear_500", |b| {
                b.iter(|| {
                    let results = small_index.search(black_box(&query), 10).unwrap();
                    black_box(results);
                });
            });
        }
        
        // Большой датасет (HNSW)
        {
            let large_index = VectorIndexV3::new(VectorIndexConfigV3 {
                dimension,
                linear_search_threshold: 100, // Форсируем HNSW
                ..Default::default()
            });
            
            let embeddings = generate_embeddings(5000, dimension);
            large_index.add_batch(embeddings).unwrap();
            
            let query = random_embedding(dimension, 0.5);
            
            group.bench_function("hnsw_5000", |b| {
                b.iter(|| {
                    let results = large_index.search(black_box(&query), 10).unwrap();
                    black_box(results);
                });
            });
        }
        
        group.finish();
    }
    
    // Тест 5: Операции удаления
    {
        let mut group = c.benchmark_group("VectorIndexV3/remove");
        group.measurement_time(Duration::from_secs(5));
        
        for size in [1000, 5000].iter() {
            let index = VectorIndexV3::new(config.clone());
            let embeddings = generate_embeddings(*size, dimension);
            
            // Добавляем все эмбеддинги
            index.add_batch(embeddings).unwrap();
            
            let mut i = 0;
            
            group.bench_with_input(
                BenchmarkId::from_parameter(size),
                size,
                |b, &size| {
                    b.iter(|| {
                        let id = format!("doc_{}", i % size);
                        index.remove(black_box(&id));
                        i += 1;
                    });
                },
            );
        }
        
        group.finish();
    }
    
    // Тест 6: Перестройка индекса
    {
        let mut group = c.benchmark_group("VectorIndexV3/rebuild");
        group.measurement_time(Duration::from_secs(10));
        
        for size in [1000, 5000, 10000].iter() {
            group.bench_with_input(
                BenchmarkId::from_parameter(size),
                size,
                |b, &size| {
                    let index = VectorIndexV3::new(config.clone());
                    let embeddings = generate_embeddings(size, dimension);
                    
                    b.iter_batched(
                        || {
                            // Setup: добавляем все эмбеддинги
                            let idx = VectorIndexV3::new(config.clone());
                            for (id, emb) in &embeddings {
                                idx.add(id.clone(), emb.clone()).unwrap();
                            }
                            idx
                        },
                        |idx| {
                            // Benchmark: форсируем перестройку
                            idx.rebuild().unwrap();
                        },
                        criterion::BatchSize::SmallInput,
                    );
                },
            );
        }
        
        group.finish();
    }
}

// Дополнительный бенчмарк для проверки оптимизации памяти
fn benchmark_memory_optimization(c: &mut Criterion) {
    let mut group = c.benchmark_group("VectorIndexV3/memory");
    group.measurement_time(Duration::from_secs(5));
    
    let config = VectorIndexConfigV3 {
        dimension: 1024,
        ..Default::default()
    };
    
    group.bench_function("optimize_memory", |b| {
        let index = VectorIndexV3::new(config.clone());
        let embeddings = generate_embeddings(5000, 1024);
        index.add_batch(embeddings).unwrap();
        
        // Добавляем и удаляем для создания фрагментации
        for i in 0..500 {
            index.remove(&format!("doc_{}", i * 10));
        }
        
        b.iter(|| {
            index.optimize_memory().unwrap();
        });
    });
    
    group.finish();
}

criterion_group!(benches, benchmark_vector_index_v3, benchmark_memory_optimization);
criterion_main!(benches);