use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use memory::di::{
    DIContainer, DIContainerBuilder, Lifetime,
};
use memory::service_di_facade::{MemoryServiceConfig, UnifiedMemoryConfigurator};
use memory::service_di_facade::service_config::BatchConfig;
use memory::di::core_traits::ServiceResolver;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::runtime::Runtime;

/// Performance benchmarks для DI container system
/// Измеряет: registration, resolution, factory execution, caching performance

/// Simple service для тестирования
struct LightweightService {
    id: u64,
    data: String,
}

impl LightweightService {
    fn new(id: u64) -> Self {
        Self {
            id,
            data: format!("service-{}", id),
        }
    }
}

/// Heavy service с много работы в конструкторе
struct HeavyService {
    id: u64,
    computed_data: Vec<u64>,
    expensive_result: String,
}

impl HeavyService {
    fn new(id: u64) -> Self {
        // Имитируем тяжелые вычисления
        let computed_data: Vec<u64> = (0..1000).map(|i| (i * id) % 1000).collect();
        let expensive_result = format!(
            "heavy-computation-{}-{}",
            id,
            computed_data.iter().sum::<u64>()
        );

        Self {
            id,
            computed_data,
            expensive_result,
        }
    }
}

/// Зависимый сервис для тестирования dependency injection
struct DependentService {
    #[allow(dead_code)]
    lightweight: Arc<LightweightService>,
    #[allow(dead_code)]
    heavy: Arc<HeavyService>,
    combined_id: String,
}

impl DependentService {
    fn new(lightweight: Arc<LightweightService>, heavy: Arc<HeavyService>) -> Self {
        Self {
            combined_id: format!("dep-{}-{}", lightweight.id, heavy.id),
            lightweight,
            heavy,
        }
    }
}

/// Создать тестовую конфигурацию
fn create_test_config() -> MemoryServiceConfig {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path().to_path_buf();

    MemoryServiceConfig {
        db_path: base_path.join("bench.db"),
        cache_path: base_path.join("cache"),
        promotion: memory::types::PromotionConfig::default(),
        cache_config: memory::CacheConfigType::default(),
        health_enabled: true,
        health_config: memory::health::HealthMonitorConfig::default(),
        batch_config: BatchConfig::default(),
    }
}

/// Benchmark: Регистрация множественных сервисов
fn bench_service_registration(c: &mut Criterion) {
    let mut group = c.benchmark_group("di_registration");

    for service_count in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::new("lightweight_services", service_count),
            service_count,
            |b, &count| {
                b.iter(|| {
                    let container = DIContainer::new();

                    for i in 0..count {
                        let i_copy = i;
                        container
                            .register(
                                move |_| Ok(LightweightService::new(i_copy)),
                                Lifetime::Singleton,
                            )
                            .unwrap();
                    }

                    black_box(container)
                });
            },
        );
    }

    for service_count in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("heavy_services", service_count),
            service_count,
            |b, &count| {
                b.iter(|| {
                    let container = DIContainer::new();

                    for i in 0..count {
                        let i_copy = i;
                        container
                            .register(move |_| Ok(HeavyService::new(i_copy)), Lifetime::Singleton)
                            .unwrap();
                    }

                    black_box(container)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Разрешение зависимостей (resolution)
fn bench_service_resolution(c: &mut Criterion) {
    let mut group = c.benchmark_group("di_resolution");

    // Подготавливаем контейнер с сервисами
    let container = DIContainer::new();

    // Регистрируем lightweight services
    for i in 0..100 {
        let i_copy = i;
        container
            .register(
                move |_| Ok(LightweightService::new(i_copy)),
                Lifetime::Singleton,
            )
            .unwrap();
    }

    // Регистрируем heavy services
    for i in 0..50 {
        let i_copy = i;
        container
            .register(
                move |_| Ok(HeavyService::new(i_copy + 1000)),
                Lifetime::Transient, // Transient чтобы тестировать factory execution
            )
            .unwrap();
    }

    // Benchmark singleton resolution (должен быть очень быстрый)
    group.bench_function("singleton_resolution", |b| {
        b.iter(|| {
            for i in 0..10 {
                let service: Arc<LightweightService> = container.resolve().unwrap();
                black_box(service);
            }
        });
    });

    // Benchmark transient resolution (включает factory execution)
    group.bench_function("transient_resolution", |b| {
        b.iter(|| {
            let service: Arc<HeavyService> = container.resolve().unwrap();
            black_box(service);
        });
    });

    group.finish();
}

/// Benchmark: Dependency injection chains
fn bench_dependency_chains(c: &mut Criterion) {
    let mut group = c.benchmark_group("di_dependency_chains");

    let container = DIContainer::new();

    // Регистрируем базовые сервисы
    container
        .register(|_| Ok(LightweightService::new(1)), Lifetime::Singleton)
        .unwrap();

    container
        .register(|_| Ok(HeavyService::new(2)), Lifetime::Singleton)
        .unwrap();

    // Регистрируем зависимый сервис
    container
        .register(
            |container| {
                let lightweight = container.resolve::<LightweightService>()?;
                let heavy = container.resolve::<HeavyService>()?;
                Ok(DependentService::new(lightweight, heavy))
            },
            Lifetime::Singleton,
        )
        .unwrap();

    group.bench_function("dependency_chain_resolution", |b| {
        b.iter(|| {
            let service: Arc<DependentService> = container.resolve().unwrap();
            black_box(service);
        });
    });

    // Benchmark повторного разрешения (должен использовать кэш)
    group.bench_function("cached_dependency_resolution", |b| {
        // Первый resolve для создания кэша
        let _: Arc<DependentService> = container.resolve().unwrap();

        b.iter(|| {
            for _ in 0..10 {
                let service: Arc<DependentService> = container.resolve().unwrap();
                black_box(service);
            }
        });
    });

    group.finish();
}

/// Benchmark: Memory DI configuration (real-world scenario)
fn bench_memory_di_configuration(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("memory_di_config");

    group.bench_function("minimal_configuration", |b| {
        b.to_async(&rt).iter(|| async {
            let config = create_test_config();
            let container = UnifiedMemoryConfigurator::configure_minimal(config)
                .await
                .unwrap();
            black_box(container)
        });
    });

    group.bench_function("cpu_only_configuration", |b| {
        b.to_async(&rt).iter(|| async {
            let config = create_test_config();
            let container = UnifiedMemoryConfigurator::configure_cpu_only(config)
                .await
                .unwrap();
            black_box(container)
        });
    });

    group.bench_function("full_configuration", |b| {
        b.to_async(&rt).iter(|| async {
            let config = create_test_config();
            let container = UnifiedMemoryConfigurator::configure_full(config).await.unwrap();
            black_box(container)
        });
    });

    group.finish();
}

/// Benchmark: Container overhead и statistics
fn bench_container_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("di_overhead");

    let container = DIContainer::new();

    // Регистрируем много сервисов для realistic overhead
    for i in 0..1000 {
        let i_copy = i;
        container
            .register(
                move |_| Ok(LightweightService::new(i_copy)),
                Lifetime::Singleton,
            )
            .unwrap();
    }

    group.bench_function("stats_calculation", |b| {
        b.iter(|| {
            let stats = container.stats();
            black_box(stats);
        });
    });

    group.bench_function("type_registration_check", |b| {
        b.iter(|| {
            for i in 0..100 {
                let is_registered = container.is_registered::<LightweightService>();
                black_box((i, is_registered));
            }
        });
    });

    group.bench_function("registered_types_listing", |b| {
        b.iter(|| {
            let types = container.registered_types();
            black_box(types);
        });
    });

    group.finish();
}

/// Benchmark: Concurrent access
fn bench_concurrent_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("di_concurrent");

    let container = Arc::new(DIContainer::new());

    // Регистрируем сервисы
    container
        .register(|_| Ok(LightweightService::new(42)), Lifetime::Singleton)
        .unwrap();

    container
        .register(|_| Ok(HeavyService::new(84)), Lifetime::Transient)
        .unwrap();

    group.bench_function("concurrent_singleton_resolution", |b| {
        b.iter(|| {
            let handles: Vec<_> = (0..10)
                .map(|_| {
                    let container_clone = Arc::clone(&container);
                    std::thread::spawn(move || {
                        let service: Arc<LightweightService> = container_clone.resolve().unwrap();
                        black_box(service);
                    })
                })
                .collect();

            for handle in handles {
                handle.join().unwrap();
            }
        });
    });

    group.bench_function("concurrent_transient_resolution", |b| {
        b.iter(|| {
            let handles: Vec<_> = (0..5)
                .map(|_| {
                    let container_clone = Arc::clone(&container);
                    std::thread::spawn(move || {
                        let service: Arc<HeavyService> = container_clone.resolve().unwrap();
                        black_box(service);
                    })
                })
                .collect();

            for handle in handles {
                handle.join().unwrap();
            }
        });
    });

    group.finish();
}

/// Benchmark: Builder pattern performance
fn bench_builder_pattern(c: &mut Criterion) {
    let mut group = c.benchmark_group("di_builder");

    group.bench_function("builder_chain_creation", |b| {
        b.iter(|| {
            let container = DIContainerBuilder::new()
                .register_singleton(|_| Ok(LightweightService::new(1)))
                .unwrap()
                .register_singleton(|_| Ok(HeavyService::new(2)))
                .unwrap()
                .register_transient(|container| {
                    let lightweight = container.resolve::<LightweightService>()?;
                    let heavy = container.resolve::<HeavyService>()?;
                    Ok(DependentService::new(lightweight, heavy))
                })
                .unwrap()
                .build()
                .unwrap();

            black_box(container);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_service_registration,
    bench_service_resolution,
    bench_dependency_chains,
    bench_memory_di_configuration,
    bench_container_overhead,
    bench_concurrent_access,
    bench_builder_pattern
);

criterion_main!(benches);
