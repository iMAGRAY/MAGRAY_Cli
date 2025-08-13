//! Basic Integration Benchmarks
//!
//! Simple performance benchmarks for testing integration framework performance

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use integration_tests::{
    common::{PerformanceMetrics, TestFixture},
    TestEnvironment,
};
use std::time::Duration;
use tokio::runtime::Runtime;

/// Benchmark test environment setup performance
fn bench_test_environment_setup(c: &mut Criterion) {
    let rt = Runtime::new().expect("Test operation should succeed");

    c.bench_function("test_environment_setup", |b| {
        b.iter(|| {
            let env = rt.block_on(async {
                let env = TestEnvironment::setup().await.expect("Test operation should succeed");
                let result = env.temp_dir.clone();
                env.cleanup().await.expect("Test operation should succeed");
                result
            });
            black_box(env)
        });
    });
}

/// Benchmark test fixture creation performance
fn bench_test_fixture_creation(c: &mut Criterion) {
    let rt = Runtime::new().expect("Test operation should succeed");

    let sizes = vec![1, 5, 10, 20];
    let mut group = c.benchmark_group("test_fixture_creation");

    for size in sizes {
        group.bench_with_input(
            BenchmarkId::new("create_fixtures", size),
            &size,
            |b, &fixture_count| {
                b.iter(|| {
                    rt.block_on(async {
                        let mut fixtures = Vec::new();

                        for i in 0..fixture_count {
                            let fixture = TestFixture::new(&format!("benchmark_fixture_{}", i))
                                .await
                                .expect("Test operation should succeed");
                            fixtures.push(fixture);
                        }

                        // Cleanup
                        for fixture in fixtures {
                            fixture.cleanup().await.expect("Test operation should succeed");
                        }

                        black_box(fixture_count)
                    })
                });
            },
        );
    }

    group.finish();
}

/// Benchmark performance metrics collection
fn bench_performance_metrics(c: &mut Criterion) {
    c.bench_function("performance_metrics_collection", |b| {
        b.iter(|| {
            let mut metrics = PerformanceMetrics::new("benchmark_test");

            // Simulate performance data collection
            for i in 0..100 {
                metrics.record(&format!("measurement_{}", i), i as f64);
                metrics.increment(&format!("counter_{}", i % 10));
            }

            // Mark timing points
            for i in 0..10 {
                metrics.mark(&format!("mark_{}", i));
            }

            // Measure some durations
            for i in 0..5 {
                metrics.measure_since_mark(&format!("duration_{}", i), &format!("mark_{}", i));
            }

            black_box(&metrics.measurements.len() + metrics.counters.len())
        });
    });
}

/// Benchmark test data creation performance
fn bench_test_data_creation(c: &mut Criterion) {
    let rt = Runtime::new().expect("Test operation should succeed");

    let data_sizes = vec!["small", "medium", "large"];
    let mut group = c.benchmark_group("test_data_creation");

    for data_size in data_sizes {
        group.bench_with_input(
            BenchmarkId::new("create_test_data", data_size),
            &data_size,
            |b, &size| {
                b.iter(|| {
                    rt.block_on(async {
                        let mut fixture = TestFixture::new("data_creation_benchmark").await.expect("Test operation should succeed");
                        
                        let test_data = match size {
                            "small" => serde_json::json!({
                                "test": true,
                                "data": "small dataset"
                            }),
                            "medium" => serde_json::json!({
                                "test": true,
                                "data": vec!["item"; 100],
                                "metadata": {
                                    "size": "medium",
                                    "items": 100
                                }
                            }),
                            "large" => serde_json::json!({
                                "test": true,
                                "data": vec!["item"; 1000],
                                "large_object": {
                                    "nested": (0..1000).map(|i| serde_json::json!({"id": i, "value": format!("value_{}", i)})).collect::<Vec<_>>()
                                },
                                "metadata": {
                                    "size": "large",
                                    "items": 1000
                                }
                            }),
                            _ => serde_json::json!({"default": true})
                        };
                        
                        let _data_path = fixture.create_test_data("benchmark", test_data).await.expect("Test operation should succeed");
                        
                        fixture.cleanup().await.expect("Test operation should succeed");
                        black_box(size)
                    })
                });
            },
        );
    }

    group.finish();
}

/// Benchmark concurrent test fixture usage
fn bench_concurrent_fixture_usage(c: &mut Criterion) {
    let rt = Runtime::new().expect("Test operation should succeed");

    let concurrency_levels = vec![1, 2, 4, 8];
    let mut group = c.benchmark_group("concurrent_fixture_usage");

    for concurrent_count in concurrency_levels {
        group.bench_with_input(
            BenchmarkId::new("concurrent_fixtures", concurrent_count),
            &concurrent_count,
            |b, &count| {
                b.iter(|| {
                    rt.block_on(async {
                        let mut handles = Vec::new();

                        for i in 0..count {
                            let handle = tokio::spawn(async move {
                                let mut fixture =
                                    TestFixture::new(&format!("concurrent_benchmark_{}", i))
                                        .await
                                        .expect("Test operation should succeed");

                                // Simulate work
                                let test_data = serde_json::json!({
                                    "concurrent_test": true,
                                    "worker_id": i,
                                    "data": vec!["item"; 50]
                                });

                                let _data_path = fixture
                                    .create_test_data("concurrent", test_data)
                                    .await
                                    .expect("Test operation should succeed");

                                // Small delay to simulate real work
                                tokio::time::sleep(Duration::from_millis(1)).await;

                                fixture.cleanup().await.expect("Test operation should succeed");
                                i
                            });
                            handles.push(handle);
                        }

                        // Wait for all tasks to complete
                        let mut results = Vec::new();
                        for handle in handles {
                            results.push(handle.await.expect("Test operation should succeed"));
                        }

                        black_box(results.len())
                    })
                });
            },
        );
    }

    group.finish();
}

// Define benchmark groups
criterion_group! {
    name = basic_integration_benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(3))
        .sample_size(20);
    targets =
        bench_test_environment_setup,
        bench_test_fixture_creation,
        bench_performance_metrics,
        bench_test_data_creation,
        bench_concurrent_fixture_usage
}

criterion_main!(basic_integration_benches);
