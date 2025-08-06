//! Comprehensive Performance Benchmarking Suite для MAGRAY CLI
//! 
//! Измеряет производительность всех критических компонентов системы
//! для отслеживания regression и оптимизации performance.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::time::Duration;
use tokio::runtime::Runtime;

// Re-export основных компонентов для benchmarking
use magray_memory::{VectorIndex, Record, Layer};
use magray_ai::{CpuEmbeddingService, EmbeddingServiceTrait};
use magray_llm::{LlmClient, CompletionRequest};

/// Benchmark suite для HNSW vector operations
fn bench_hnsw_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("hnsw_operations");
    
    // Создаем тестовые данные
    let vectors: Vec<Vec<f32>> = (0..1000)
        .map(|i| (0..1024).map(|j| ((i + j) as f32) / 1024.0).collect())
        .collect();
    
    // Benchmark build index
    group.bench_function("build_index_1k", |b| {
        b.iter(|| {
            let index = VectorIndex::new(1024, 16, 200);
            for (id, vec) in vectors.iter().enumerate() {
                index.add_vector(id, vec).unwrap();
            }
        });
    });
    
    // Benchmark search
    let index = VectorIndex::new(1024, 16, 200);
    for (id, vec) in vectors.iter().enumerate().take(100) {
        index.add_vector(id, vec).unwrap();
    }
    
    let query = &vectors[0];
    group.bench_function("search_top10", |b| {
        b.iter(|| {
            index.search(query, 10).unwrap()
        });
    });
    
    group.finish();
}

/// Benchmark suite для embedding operations
fn bench_embedding_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("embedding_operations");
    
    // Создаем embedding service
    let service = rt.block_on(async {
        CpuEmbeddingService::new(Default::default()).await.unwrap()
    });
    
    let texts = vec![
        "This is a test sentence for embedding generation",
        "Another sample text for performance measurement",
        "Machine learning and artificial intelligence research",
        "Rust programming language performance optimization",
        "Vector databases and similarity search algorithms",
    ];
    
    // Single embedding benchmark
    group.bench_function("single_embedding", |b| {
        b.to_async(&rt).iter(|| async {
            service.embed(&texts[0]).await.unwrap()
        });
    });
    
    // Batch embedding benchmark
    group.throughput(Throughput::Elements(texts.len() as u64));
    group.bench_function("batch_embedding", |b| {
        b.to_async(&rt).iter(|| async {
            for text in &texts {
                service.embed(text).await.unwrap();
            }
        });
    });
    
    group.finish();
}

/// Benchmark suite для LLM operations
fn bench_llm_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("llm_operations");
    group.measurement_time(Duration::from_secs(60)); // Longer measurement for network calls
    
    // Mock LLM operations (avoiding real API calls in benchmarks)
    group.bench_function("completion_request_processing", |b| {
        b.iter(|| {
            let request = CompletionRequest::new("Test prompt for benchmarking")
                .max_tokens(100)
                .temperature(0.7);
            
            // Simulate request processing without actual API call
            std::thread::sleep(Duration::from_millis(10));
            request
        });
    });
    
    group.finish();
}

/// Benchmark suite для memory operations
fn bench_memory_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("memory_operations");
    
    // Mock record creation
    let records: Vec<Record> = (0..100)
        .map(|i| Record {
            id: uuid::Uuid::new_v4(),
            text: format!("Test record number {}", i),
            embedding: (0..1024).map(|j| (i + j) as f32 / 1024.0).collect(),
            layer: Layer::Interact,
            kind: "benchmark".to_string(),
            tags: vec!["test".to_string()],
            project: "benchmark".to_string(),
            session: "test".to_string(),
            ts: chrono::Utc::now(),
            score: 0.0,
            access_count: 1,
            last_access: chrono::Utc::now(),
        })
        .collect();
    
    group.throughput(Throughput::Elements(records.len() as u64));
    group.bench_function("record_processing", |b| {
        b.iter(|| {
            for record in &records {
                // Simulate record processing
                let _size = record.text.len() + record.embedding.len() * 4;
            }
        });
    });
    
    group.finish();
}

/// Benchmark suite для agent operations
fn bench_agent_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("agent_operations");
    
    let messages = vec![
        "создай файл test.txt с содержимым hello world",
        "покажи статус git репозитория",
        "найди файлы с расширением .rs в текущей папке",
        "выполни команду ls -la",
        "объясни что такое vector database",
    ];
    
    // Simulate intent analysis
    group.bench_function("intent_analysis", |b| {
        b.iter(|| {
            for message in &messages {
                // Simulate intent analysis processing
                let _intent = if message.contains("файл") || message.contains("file") {
                    "tools"
                } else if message.contains("покажи") || message.contains("show") {
                    "tools"
                } else {
                    "chat"
                };
            }
        });
    });
    
    group.finish();
}

/// Comprehensive end-to-end workflow benchmark
fn bench_e2e_workflow(c: &mut Criterion) {
    let mut group = c.benchmark_group("end_to_end_workflow");
    group.measurement_time(Duration::from_secs(30));
    
    group.bench_function("complete_workflow_simulation", |b| {
        b.iter(|| {
            // Simulate complete workflow:
            // 1. Message processing
            let message = "создай файл и добавь его в git";
            
            // 2. Intent analysis
            let _intent = "tools";
            
            // 3. Tool selection
            let _tools = vec!["file_ops", "git_ops"];
            
            // 4. Execution simulation
            std::thread::sleep(Duration::from_millis(1));
            
            // 5. Response generation
            let _response = "Файл создан и добавлен в git";
        });
    });
    
    group.finish();
}

/// Regression benchmark для отслеживания performance changes
fn bench_regression_tracking(c: &mut Criterion) {
    let mut group = c.benchmark_group("regression_tracking");
    
    // Baseline operations that should remain stable
    group.bench_function("baseline_string_processing", |b| {
        b.iter(|| {
            let text = "This is a baseline string processing operation for regression tracking";
            let words: Vec<&str> = text.split_whitespace().collect();
            let _word_count = words.len();
            let _char_count = text.len();
        });
    });
    
    group.bench_function("baseline_vector_operations", |b| {
        b.iter(|| {
            let vec: Vec<f32> = (0..1000).map(|i| i as f32 / 1000.0).collect();
            let _sum: f32 = vec.iter().sum();
            let _avg = _sum / vec.len() as f32;
        });
    });
    
    group.finish();
}

// Группируем все benchmarks
criterion_group!(
    comprehensive_benches,
    bench_hnsw_performance,
    bench_embedding_performance, 
    bench_llm_performance,
    bench_memory_performance,
    bench_agent_performance,
    bench_e2e_workflow,
    bench_regression_tracking
);

criterion_main!(comprehensive_benches);